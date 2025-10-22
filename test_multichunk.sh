#!/bin/bash

# Test multi-chunk file support

set -e

echo "Testing multi-chunk file support..."

# Clean up any previous test
rm -rf .fai test_small.txt test_large.bin serve.log

# Initialize FAI repository
cargo run -- init

# Test 1: Small file (should be stored as single object)
echo "Small file content" > test_small.txt
echo "Testing small file storage..."
SMALL_HASH=$(cargo run -- add test_small.txt 2>&1 | grep -o '[a-f0-9]\{64\}' | head -n1)
echo "Small file hash: $SMALL_HASH"

# Test 2: Large file (should be chunked)
echo "Creating large test file (2MB)..."
dd if=/dev/zero of=test_large.bin bs=1024 count=2048 2>/dev/null
echo "Testing large file storage..."
LARGE_HASH=$(cargo run -- add test_large.bin 2>&1 | grep -o '[a-f0-9]\{64\}' | head -n1)
echo "Large file hash: $LARGE_HASH"

# Verify storage structure
echo ""
echo "Storage structure:"
echo "Small file objects:"
find .fai/objects -name "*" -type f | head -5
echo ""
echo "Large file objects:"
find .fai/objects -name "*" -type f | wc -l
echo "Total objects found"

# Test retrieval
echo ""
echo "Testing retrieval..."
cargo run -- serve > serve.log 2>&1 &
SERVE_PID=$!
sleep 2

# Test fetching small file
cargo run -- fetch $(grep "Local peer ID:" serve.log | awk '{print $4}' | head -n1) $SMALL_HASH
echo "Small file fetched successfully"

# Test fetching large file
cargo run -- fetch $(grep "Local peer ID:" serve.log | awk '{print $4}' | head -n1) $LARGE_HASH
echo "Large file fetched successfully"

# Cleanup
kill $SERVE_PID 2>/dev/null || true
rm -rf .fai test_small.txt test_large.bin serve.log

echo "Multi-chunk test completed successfully!"
