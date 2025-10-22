#!/bin/bash

# P2P File Transfer Test Script for FAI Protocol
# This script tests the complete P2P file transfer functionality

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_FILE="test_file.txt"
TEST_CONTENT="Hello P2P World!"
SERVE_LOG="serve.log"
FETCH_LOG="fetch.log"
TIMEOUT=30

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    
    # Kill background processes
    if [ ! -z "$SERVE_PID" ] && kill -0 $SERVE_PID 2>/dev/null; then
        echo "Stopping serve process..."
        kill $SERVE_PID 2>/dev/null || true
        wait $SERVE_PID 2>/dev/null || true
    fi
    
    # Clean up test files and directories
    rm -f "$TEST_FILE"
    rm -f fetched_*.dat
    rm -f "$SERVE_LOG" "$FETCH_LOG"
    rm -rf .fai
    
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up cleanup on exit
trap cleanup EXIT

# Helper functions
print_step() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Start of test
echo -e "${GREEN}Starting FAI P2P Transfer Test${NC}"

# Step 1: Clean any previous test data
print_step "Step 1: Cleaning previous test data"
cleanup

# Step 2: Initialize FAI repository
print_step "Step 2: Initializing FAI repository"
if cargo run -- init 2>&1 | tee init.log; then
    print_success "FAI repository initialized"
    rm -f init.log
else
    print_error "Failed to initialize FAI repository"
    if [ -f init.log ]; then
        echo "Error output:"
        cat init.log
        rm -f init.log
    fi
    exit 1
fi

# Step 3: Create test file with known content
print_step "Step 3: Creating test file"
echo "$TEST_CONTENT" > "$TEST_FILE"
print_success "Created test file: $TEST_FILE"

# Step 4: Add file to FAI and capture the hash
print_step "Step 4: Adding file to FAI"
ADD_OUTPUT=$(cargo run -- add "$TEST_FILE" 2>&1)
if [ $? -ne 0 ]; then
    print_error "Failed to add file to FAI"
    echo "$ADD_OUTPUT"
    exit 1
fi

# Extract hash from output
FILE_HASH=$(echo "$ADD_OUTPUT" | grep -o '([a-f0-9]\{8\})' | sed 's/[()]*//g' | head -n1)
if [ -z "$FILE_HASH" ]; then
    # Try alternative format if the first pattern doesn't match
    FILE_HASH=$(echo "$ADD_OUTPUT" | grep -o '[a-f0-9]\{8\}' | head -n1)
fi
if [ -z "$FILE_HASH" ]; then
    print_error "Could not extract file hash from output"
    echo "$ADD_OUTPUT"
    exit 1
fi

print_success "Test file hash: $FILE_HASH"

# Step 5: Start FAI server in background
print_step "Step 5: Starting FAI server"
cargo run -- serve > "$SERVE_LOG" 2>&1 &
SERVE_PID=$!

# Wait for server to start
sleep 2

# Check if server is still running
if ! kill -0 $SERVE_PID 2>/dev/null; then
    print_error "Server failed to start"
    cat "$SERVE_LOG"
    exit 1
fi

# Extract peer ID from server log
SERVER_PEER_ID=$(grep "Local peer ID:" "$SERVE_LOG" | head -n1 | awk '{print $4}')
if [ -z "$SERVER_PEER_ID" ]; then
    print_error "Could not extract server peer ID"
    cat "$SERVE_LOG"
    exit 1
fi

print_success "Server peer ID: $SERVER_PEER_ID"

# Wait a bit more for server to be fully ready
sleep 2

# Step 6: Test P2P transfer with fetch
print_step "Step 6: Testing P2P transfer"
echo "Fetching chunk $FILE_HASH from peer $SERVER_PEER_ID..."
echo ""
echo "=== SERVER STATUS CHECK ==="
if ! kill -0 $SERVE_PID 2>/dev/null; then
    print_error "Server process died before fetch started"
    cat "$SERVE_LOG"
    exit 1
fi
echo "Server process is running (PID: $SERVE_PID)"
echo "Server log size: $(wc -l < "$SERVE_LOG") lines"
echo "Last few lines of server log:"
tail -5 "$SERVE_LOG" | sed 's/^/  /'
echo ""

# Start fetch in background
echo "=== STARTING FETCH ==="
echo "Running: cargo run -- fetch $SERVER_PEER_ID $FILE_HASH"
cargo run -- fetch "$SERVER_PEER_ID" "$FILE_HASH" > "$FETCH_LOG" 2>&1 &
FETCH_PID=$!
echo "Fetch started (PID: $FETCH_PID)"

# Show fetch command output as it happens
tail -f "$FETCH_LOG" &
TAIL_PID=$!

# Monitor progress
echo "=== MONITORING FETCH PROGRESS ==="
for i in $(seq 1 $TIMEOUT); do
    if ! kill -0 $FETCH_PID 2>/dev/null; then
        echo ""
        echo "=== FETCH COMPLETED ==="
        wait $FETCH_PID
        FETCH_EXIT_CODE=$?
        echo "Fetch exit code: $FETCH_EXIT_CODE"
        echo "Fetch ran for $i seconds"
        break
    fi
    
    # Show progress every 5 seconds
    if [ $((i % 5)) -eq 0 ]; then
        echo "Progress: $i/$TIMEOUT seconds elapsed"
        echo "  Server log lines: $(wc -l < "$SERVE_LOG")"
        echo "  Fetch log lines: $(wc -l < "$FETCH_LOG")"
        
        # Show any recent interesting log entries
        if grep -q "DEBUG\|ERROR\|Found peer\|Requesting\|Received" "$FETCH_LOG"; then
            echo "  Recent fetch activity:"
            tail -3 "$FETCH_LOG" | grep -E "DEBUG|ERROR|Found|Requesting|Received" | sed 's/^/    /'
        fi
    fi
    
    sleep 1
done

# Stop tailing
kill $TAIL_PID 2>/dev/null || true

# Check if fetch is still running (timeout)
if kill -0 $FETCH_PID 2>/dev/null; then
    print_error "Fetch timed out after $TIMEOUT seconds"
    kill $FETCH_PID 2>/dev/null || true
    wait $FETCH_PID 2>/dev/null || true
    FETCH_EXIT_CODE=124  # timeout exit code
    echo ""
    echo "=== Server Log ==="
    cat "$SERVE_LOG"
    echo ""
    echo "=== Fetch Log ==="
    cat "$FETCH_LOG"
    echo ""
    exit 1
fi

# Step 7: Verify the transfer
print_step "Step 7: Verifying file transfer"

# Check if fetch was successful
if [ $FETCH_EXIT_CODE -ne 0 ]; then
    print_error "Fetch command failed with exit code $FETCH_EXIT_CODE"
    echo -e "\n${RED}=== FETCH LOG ===${NC}"
    cat "$FETCH_LOG"
    echo -e "\n${RED}=== SERVER LOG ===${NC}"
    cat "$SERVE_LOG"
    exit 1
fi

# Always show logs for debugging
echo -e "\n${YELLOW}=== FETCH LOG ===${NC}"
cat "$FETCH_LOG"
echo -e "\n${YELLOW}=== SERVER LOG ===${NC}"
cat "$SERVE_LOG"

# Check if server received chunk requests
if grep -q "Received chunk request" "$SERVE_LOG"; then
    echo -e "\n${GREEN}✓ Server received chunk request${NC}"
else
    echo -e "\n${RED}✗ Server never received chunk request${NC}"
fi

if grep -q "Sent chunk" "$SERVE_LOG"; then
    echo -e "\n${GREEN}✓ Server sent chunk response${NC}"
else
    echo -e "\n${RED}✗ Server never sent chunk response${NC}"
fi

# Check if fetched file exists
FETCHED_FILE="fetched_${FILE_HASH:0:8}.dat"
if [ ! -f "$FETCHED_FILE" ]; then
    print_error "Fetched file not found: $FETCHED_FILE"
    cat "$FETCH_LOG"
    exit 1
fi

# Compare content
if cmp -s "$TEST_FILE" "$FETCHED_FILE"; then
    print_success "File content matches original"
    MATCH=true
else
    print_error "File content does not match original"
    echo "Original content: '$TEST_CONTENT'"
    echo "Fetched content: '$(cat "$FETCHED_FILE")'"
    MATCH=false
fi

# Step 8: Print final results
print_step "Step 8: Test Results"
if [ "$MATCH" = true ]; then
    print_success "P2P File Transfer Test PASSED"
    echo -e "\n${GREEN}Original file: $TEST_FILE${NC}"
    echo -e "${GREEN}Fetched file: $FETCHED_FILE${NC}"
    echo -e "${GREEN}Content: '$TEST_CONTENT'${NC}"
    echo -e "${GREEN}Hash: $FILE_HASH${NC}"
else
    print_error "P2P File Transfer Test FAILED"
    echo -e "\n${RED}Check logs for details:${NC}"
    echo -e "${RED}Server log: $SERVE_LOG${NC}"
    echo -e "${RED}Fetch log: $FETCH_LOG${NC}"
    exit 1
fi

print_step "Step 9: Final cleanup"

echo -e "\n${GREEN}Test completed successfully!${NC}"
