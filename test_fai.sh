#!/bin/bash

# FAI Protocol Comprehensive Test Script
# Tests all major functionality of the FAI Protocol system

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_section() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

# Function to check if command succeeded
check_success() {
    if [ $? -eq 0 ]; then
        print_success "$1"
        return 0
    else
        print_error "$1"
        exit 1
    fi
}

# Function to run command and check result
run_command() {
    local cmd="$1"
    local description="$2"
    print_info "Running: $cmd"
    eval "$cmd"
    if [ $? -eq 0 ]; then
        print_success "$description"
    else
        print_error "$description"
        exit 1
    fi
}

# Get the path to the fai binary
FAI_BIN="./target/debug/fai"
if [ ! -f "$FAI_BIN" ]; then
    print_error "FAI binary not found at $FAI_BIN"
    print_info "Please run 'cargo build' first"
    exit 1
fi

print_section "FAI Protocol Comprehensive Test"

# Cleanup function
cleanup() {
    print_info "Cleaning up previous test files..."
    rm -rf .fai
    rm -f test*.txt large_test_file.dat
}

# Cleanup any existing test files
cleanup

print_section "1. Testing Init"

run_command "$FAI_BIN init" "Initialize FAI repository"

# Verify .fai directory structure
if [ -d ".fai" ]; then
    print_success ".fai directory created"
else
    print_error ".fai directory not created"
    exit 1
fi

if [ -d ".fai/objects" ]; then
    print_success ".fai/objects directory created"
else
    print_error ".fai/objects directory not created"
    exit 1
fi

if [ -f ".fai/db.sqlite" ]; then
    print_success ".fai/db.sqlite database created"
else
    print_error ".fai/db.sqlite database not created"
    exit 1
fi

if [ -f ".fai/HEAD" ]; then
    print_success ".fai/HEAD file created"
else
    print_error ".fai/HEAD file not created"
    exit 1
fi

print_section "2. Testing Add (single file)"

# Create test file
echo "First test file" > test1.txt
print_info "Created test1.txt"

run_command "$FAI_BIN add test1.txt" "Add test1.txt to staging area"

# Check status
echo -e "\n${BLUE}Checking status:${NC}"
$FAI_BIN status

# Verify file is staged
if $FAI_BIN status | grep -q "test1.txt"; then
    print_success "test1.txt found in status"
else
    print_error "test1.txt not found in status"
    exit 1
fi

print_section "3. Testing Add (multiple files)"

# Create more test files
echo "Second test file" > test2.txt
echo "Third test file" > test3.txt
print_info "Created test2.txt and test3.txt"

run_command "$FAI_BIN add test2.txt" "Add test2.txt to staging area"
run_command "$FAI_BIN add test3.txt" "Add test3.txt to staging area"

# Check status
echo -e "\n${BLUE}Checking status for multiple files:${NC}"
$FAI_BIN status

# Verify all files are staged
if $FAI_BIN status | grep -q "test1.txt" && $FAI_BIN status | grep -q "test2.txt" && $FAI_BIN status | grep -q "test3.txt"; then
    print_success "All test files found in status"
else
    print_error "Not all test files found in status"
    exit 1
fi

print_section "4. Testing Commit"

run_command "$FAI_BIN commit -m \"First commit\"" "Create first commit"

# Check log
echo -e "\n${BLUE}Checking commit log:${NC}"
$FAI_BIN log

# Verify commit appears in log
if $FAI_BIN log | grep -q "First commit"; then
    print_success "Commit found in log"
else
    print_error "Commit not found in log"
    exit 1
fi

# Verify staging area is cleared
if $FAI_BIN status | grep -q "No changes staged"; then
    print_success "Staging area cleared after commit"
else
    print_error "Staging area not cleared after commit"
    exit 1
fi

print_section "5. Testing Multiple Commits"

# Create and add more files
echo "Fourth test file" > test4.txt
run_command "$FAI_BIN add test4.txt" "Add test4.txt"
run_command "$FAI_BIN commit -m \"Second commit\"" "Create second commit"

echo "Fifth test file" > test5.txt
run_command "$FAI_BIN add test5.txt" "Add test5.txt"
run_command "$FAI_BIN commit -m \"Third commit\"" "Create third commit"

# Check log for multiple commits
echo -e "\n${BLUE}Checking commit log for multiple commits:${NC}"
$FAI_BIN log

# Count commits
commit_count=$($FAI_BIN log | grep -c "commit ")
if [ "$commit_count" -eq 3 ]; then
    print_success "Found 3 commits in log"
else
    print_error "Expected 3 commits, found $commit_count"
    exit 1
fi

print_section "6. Testing Error Handling"

# Test adding non-existent file
print_info "Testing add non-existent file..."
if $FAI_BIN add nonexistent_file.txt 2>/dev/null; then
    print_error "Adding non-existent file should have failed"
    exit 1
else
    print_success "Correctly rejected non-existent file"
fi

# Test committing with nothing staged
print_info "Testing commit with nothing staged..."
if $FAI_BIN commit -m "Empty commit" 2>/dev/null; then
    print_error "Empty commit should have failed"
    exit 1
else
    print_success "Correctly rejected empty commit"
fi

print_section "7. Testing Large File"

print_info "Creating 10MB test file..."
dd if=/dev/zero of=large_test_file.dat bs=1M count=10 2>/dev/null

run_command "$FAI_BIN add large_test_file.dat" "Add large test file"
run_command "$FAI_BIN commit -m \"Add large test file\"" "Commit large test file"

# Verify large file was stored
if [ -f ".fai/objects" ] && [ $(find .fai/objects -type f -size +1M | wc -l) -gt 0 ]; then
    print_success "Large file stored correctly"
else
    print_error "Large file not stored correctly"
    exit 1
fi

print_section "8. Final Verification"

# Final log check
echo -e "\n${BLUE}Final commit log:${NC}"
$FAI_BIN log

# Count final commits
final_commit_count=$($FAI_BIN log | grep -c "commit ")
print_info "Total commits: $final_commit_count"

# Verify all expected commits are present
if $FAI_BIN log | grep -q "First commit" && $FAI_BIN log | grep -q "Second commit" && $FAI_BIN log | grep -q "Third commit" && $FAI_BIN log | grep -q "Add large test file"; then
    print_success "All expected commits found in log"
else
    print_error "Not all expected commits found in log"
    exit 1
fi

print_section "Test Summary"
print_success "All tests passed! ✓"
print_info "FAI Protocol is working correctly"

# Show final repository state
echo -e "\n${BLUE}Final repository status:${NC}"
$FAI_BIN status

echo -e "\n${YELLOW}Test completed successfully!${NC}"
