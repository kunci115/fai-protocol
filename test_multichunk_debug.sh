#!/bin/bash

# Debug test for multi-chunk file support

set -e

echo "=== DEBUGGING MULTI-CHUNK FUNCTIONALITY ==="

# Clean up any previous test
rm -rf .fai test_large.bin serve.log

# Initialize FAI repository
cargo run -- init

# Create a larger test file (3MB to ensure multiple chunks)
echo "Creating large test file (3MB)..."
dd if=/dev/zero of=test_large.bin bs=1024 count=3072 2>/dev/null
ORIGINAL_SIZE=$(wc -c < test_large.bin)
echo "Original file size: $ORIGINAL_SIZE bytes"

# Add file and capture detailed output
echo ""
echo "=== ADDING FILE TO STORAGE ==="
ADD_OUTPUT=$(cargo run -- add test_large.bin 2>&1)
echo "$ADD_OUTPUT"

# Extract manifest hash (last hash in output for chunked files)
LARGE_HASH=$(echo "$ADD_OUTPUT" | grep -o '[a-f0-9]\{64\}' | tail -n1)
echo "Large file hash: $LARGE_HASH"
echo "Hash type: MANIFEST (should reconstruct complete file)"

# Check storage structure
echo ""
echo "=== CHECKING STORAGE STRUCTURE ==="
echo "Objects in storage:"
find .fai/objects -name "*" -type f -exec wc -c {} \; | sort -n

# Check if we can find manifest files
echo ""
echo "=== LOOKING FOR MANIFEST FILES ==="
find .fai/objects -name "*" -type f -exec file {} \; | grep -i json

# Test direct retrieval from storage
echo ""
echo "=== TESTING DIRECT STORAGE RETRIEVAL ==="
cargo run --bin fai -- -h 2>/dev/null || echo "No direct storage test available"

# Test with server
echo ""
echo "=== TESTING WITH SERVER ==="
cargo run -- serve > serve.log 2>&1 &
SERVE_PID=$!
sleep 3

# Get server peer ID
SERVER_PEER_ID=$(grep "Local peer ID:" serve.log | awk '{print $4}' | head -n1)
echo "Server peer ID: $SERVER_PEER_ID"

# Test fetch
echo "Fetching large file..."
cargo run -- fetch "$SERVER_PEER_ID" "$LARGE_HASH"

# Check fetched file size
if [ -f "fetched_${LARGE_HASH}.dat" ]; then
    FETCHED_SIZE=$(wc -c < "fetched_${LARGE_HASH}.dat")
    echo "Fetched file size: $FETCHED_SIZE bytes"
    
    if [ "$FETCHED_SIZE" -eq "$ORIGINAL_SIZE" ]; then
        echo "✓ SUCCESS: File sizes match!"
    else
        echo "✗ ERROR: File size mismatch!"
        echo "  Original: $ORIGINAL_SIZE bytes"
        echo "  Fetched:  $FETCHED_SIZE bytes"
    fi
else
    echo "✗ ERROR: Fetched file not found"
fi

# Cleanup
kill $SERVE_PID 2>/dev/null || true
echo ""
echo "=== SERVER LOG ==="
cat serve.log

echo ""
echo "=== DEBUG COMPLETE ==="
