#!/bin/bash

# Docker-based build testing script
# This script uses Docker to test cross-compilation in isolated environments

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🐳 Docker Build Testing for OPC UA Client"
echo "=========================================="

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed or not in PATH"
    echo "Please install Docker and try again"
    exit 1
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    echo "❌ Docker daemon is not running"
    echo "Please start Docker and try again"
    exit 1
fi

cd "$PROJECT_ROOT"

echo "📁 Working directory: $(pwd)"
echo ""

# Function to run Docker build with error handling
run_docker_test() {
    local test_name="$1"
    local dockerfile="$2"
    local target="$3"
    
    echo "🔨 Running $test_name..."
    
    if docker build -f "$dockerfile" --target "$target" --progress=plain . 2>&1 | tee "docker-test-$target.log"; then
        echo "✅ $test_name completed successfully"
        return 0
    else
        echo "❌ $test_name failed"
        echo "📋 Check docker-test-$target.log for details"
        return 1
    fi
}

# Array to track test results
FAILED_TESTS=()
SUCCESSFUL_TESTS=()

echo "🧪 Testing Linux cross-compilation builds..."
if run_docker_test "Linux Cross-Compilation Test" "docker/Dockerfile.test" "test-linux"; then
    SUCCESSFUL_TESTS+=("Linux Cross-Compilation")
else
    FAILED_TESTS+=("Linux Cross-Compilation")
fi

echo ""
echo "🧪 Testing build verification script..."
if run_docker_test "Build Script Verification" "docker/Dockerfile.test" "verify"; then
    SUCCESSFUL_TESTS+=("Build Script Verification")
else
    FAILED_TESTS+=("Build Script Verification")
fi

echo ""
echo "📊 Docker Test Summary:"
echo "======================"

if [ ${#SUCCESSFUL_TESTS[@]} -gt 0 ]; then
    echo "✅ Successful tests:"
    for test in "${SUCCESSFUL_TESTS[@]}"; do
        echo "   - $test"
    done
fi

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo "❌ Failed tests:"
    for test in "${FAILED_TESTS[@]}"; do
        echo "   - $test"
    done
    echo ""
    echo "⚠️  Some tests failed. Check the log files for details:"
    ls -la docker-test-*.log 2>/dev/null || echo "   No log files found"
    exit 1
else
    echo ""
    echo "🎉 All Docker tests passed! Cross-compilation setup is working correctly."
fi

# Clean up log files on success
echo ""
echo "🧹 Cleaning up..."
rm -f docker-test-*.log
echo "✅ Cleanup complete"
