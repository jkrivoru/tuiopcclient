#!/bin/bash

# Comprehensive Docker-based testing using Docker Compose
# This script runs multiple test scenarios in parallel

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🐳 Comprehensive Docker Testing Suite"
echo "====================================="

# Check if Docker and Docker Compose are available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed or not in PATH"
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null 2>&1; then
    echo "❌ Docker Compose is not available"
    echo "Please install docker-compose or use Docker with compose plugin"
    exit 1
fi

cd "$PROJECT_ROOT"

# Determine which compose command to use
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
else
    COMPOSE_CMD="docker compose"
fi

echo "📁 Working directory: $(pwd)"
echo "🔧 Using compose command: $COMPOSE_CMD"
echo ""

# Function to run a test service
run_test_service() {
    local service_name="$1"
    local description="$2"
    
    echo "🧪 Running $description..."
    echo "   Service: $service_name"
    
    if $COMPOSE_CMD -f docker/docker-compose.test.yml build "$service_name" && \
       $COMPOSE_CMD -f docker/docker-compose.test.yml run --rm "$service_name"; then
        echo "✅ $description completed successfully"
        return 0
    else
        echo "❌ $description failed"
        return 1
    fi
}

# Array to track test results
FAILED_TESTS=()
SUCCESSFUL_TESTS=()

echo "🏗️  Building base images..."
$COMPOSE_CMD -f docker/docker-compose.test.yml build --parallel

echo ""
echo "🧪 Running test suite..."
echo ""

# Test 1: Linux cross-compilation
if run_test_service "test-linux" "Linux Cross-Compilation Test"; then
    SUCCESSFUL_TESTS+=("Linux Cross-Compilation")
else
    FAILED_TESTS+=("Linux Cross-Compilation")
fi

echo ""

# Test 2: Build script verification
if run_test_service "test-verify" "Build Script Verification"; then
    SUCCESSFUL_TESTS+=("Build Script Verification")
else
    FAILED_TESTS+=("Build Script Verification")
fi

echo ""

# Test 3: Alpine minimal build
if run_test_service "test-alpine" "Alpine Minimal Build Test"; then
    SUCCESSFUL_TESTS+=("Alpine Minimal Build")
else
    FAILED_TESTS+=("Alpine Minimal Build")
fi

echo ""

# Test 4: Ubuntu comprehensive build
if run_test_service "test-ubuntu" "Ubuntu Comprehensive Build Test"; then
    SUCCESSFUL_TESTS+=("Ubuntu Comprehensive Build")
else
    FAILED_TESTS+=("Ubuntu Comprehensive Build")
fi

echo ""

# Test 5: Specific Rust version
if run_test_service "test-rust-stable" "Rust Stable Version Test"; then
    SUCCESSFUL_TESTS+=("Rust Stable Version")
else
    FAILED_TESTS+=("Rust Stable Version")
fi

echo ""
echo "🧹 Cleaning up..."
$COMPOSE_CMD -f docker/docker-compose.test.yml down --volumes --remove-orphans

echo ""
echo "📊 Comprehensive Test Summary:"
echo "=============================="

if [ ${#SUCCESSFUL_TESTS[@]} -gt 0 ]; then
    echo "✅ Successful tests (${#SUCCESSFUL_TESTS[@]}):"
    for test in "${SUCCESSFUL_TESTS[@]}"; do
        echo "   - $test"
    done
fi

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo "❌ Failed tests (${#FAILED_TESTS[@]}):"
    for test in "${FAILED_TESTS[@]}"; do
        echo "   - $test"
    done
    echo ""
    echo "⚠️  Some tests failed. Review the output above for details."
    exit 1
else
    echo ""
    echo "🎉 All tests passed! Your cross-compilation setup is working perfectly."
    echo "🚀 Ready for production releases!"
fi
