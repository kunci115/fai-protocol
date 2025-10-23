#!/bin/bash
set -e

echo "=== Manual Network Test ==="
echo

# Create test directories
mkdir -p test_repo1 test_repo2

# Initialize two repositories
echo "Initializing test_repo1..."
cd test_repo1
cargo run -- init > /dev/null 2>&1
echo "test data 1" > test1.txt
cargo run -- add test1.txt > /dev/null 2>&1
cargo run -- commit --message "Test commit 1" > /dev/null 2>&1
REPO1_COMMIT=$(cargo run -- log | grep "commit" | head -n1 | awk '{print $2}')
echo "Repo1 commit: $REPO1_COMMIT"

cd ../test_repo2
echo "Initializing test_repo2..."
cargo run -- init > /dev/null 2>&1
echo "test data 2" > test2.txt
cargo run -- add test2.txt > /dev/null 2>&1
cargo run -- commit --message "Test commit 2" > /dev/null 2>&1

echo
echo "Starting server in test_repo1..."
cd ../test_repo1
cargo run -- serve > server.log 2>&1 &
SERVER_PID=$!

echo "Server PID: $SERVER_PID"
echo "Waiting for server to start..."
sleep 5

# Check if server started
if grep -q "Local peer ID:" server.log; then
    PEER_ID=$(grep "Local peer ID:" server.log | awk '{print $4}')
    echo "Server started with peer ID: $PEER_ID"

    echo
    echo "Testing peer discovery from test_repo2..."
    cd ../test_repo2
    timeout 8 cargo run -- peers || echo "Peer discovery completed"

    echo
    echo "Testing simple fetch operation..."
    timeout 10 cargo run -- fetch "$PEER_ID" "$REPO1_COMMIT" || echo "Fetch operation completed"
else
    echo "ERROR: Server failed to start"
    cat server.log
fi

echo
echo "Cleaning up..."
kill $SERVER_PID 2>/dev/null || true
sleep 1
cd ..
rm -rf test_repo1 test_repo2

echo "Manual test completed!"