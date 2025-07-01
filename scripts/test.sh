#!/bin/bash

# OPC UA Client - Docker Testing Commands
# 
# Usage:
#   ./scripts/test.sh <command>
#
# Commands:
#   basic       - Run basic Docker build test
#   comprehensive - Run comprehensive test suite
#   alpine      - Test Alpine-based minimal build
#   ubuntu      - Test Ubuntu-based comprehensive build
#   local       - Run local build tests (no Docker)
#   cleanup     - Clean up Docker images and containers
#   help        - Show this help

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

show_help() {
    echo "🐳 OPC UA Client - Docker Testing Commands"
    echo "=========================================="
    echo ""
    echo "Usage: ./scripts/test.sh <command>"
    echo ""
    echo "Commands:"
    echo "  basic         - Run basic Docker build test"
    echo "  simple        - Run simple cross-compilation test (no cross tool)"
    echo "  comprehensive - Run comprehensive test suite with all scenarios"
    echo "  alpine        - Test Alpine-based minimal build (static binary)"
    echo "  ubuntu        - Test Ubuntu-based comprehensive build"
    echo "  local         - Run local build tests (no Docker)"
    echo "  cleanup       - Clean up Docker images and containers"
    echo "  help          - Show this help"
    echo ""
    echo "Examples:"
    echo "  ./scripts/test.sh basic"
    echo "  ./scripts/test.sh comprehensive"
    echo "  ./scripts/test.sh cleanup"
}

run_simple_test() {
    echo "🔧 Running simple cross-compilation test..."
    cd "$PROJECT_ROOT"
    docker build -f docker/Dockerfile.simple --target test-simple .
}

run_basic_test() {
    echo "🔨 Running basic Docker build test..."
    "$SCRIPT_DIR/test-docker.sh"
}

run_comprehensive_test() {
    echo "🧪 Running comprehensive test suite..."
    "$SCRIPT_DIR/test-comprehensive.sh"
}

run_alpine_test() {
    echo "🏔️ Running Alpine minimal build test..."
    cd "$PROJECT_ROOT"
    docker build -f docker/Dockerfile.alpine .
}

run_ubuntu_test() {
    echo "🐧 Running Ubuntu comprehensive build test..."
    cd "$PROJECT_ROOT"
    docker build -f docker/Dockerfile.ubuntu .
}

run_local_test() {
    echo "🏠 Running local build tests..."
    if [ -f "$SCRIPT_DIR/test-builds.sh" ]; then
        "$SCRIPT_DIR/test-builds.sh"
    else
        echo "⚠️  Local test script not found, running cargo build..."
        cd "$PROJECT_ROOT"
        cargo build --release
        cargo test
    fi
}

run_cleanup() {
    echo "🧹 Cleaning up Docker resources..."
    cd "$PROJECT_ROOT"
    
    echo "Stopping containers..."
    docker ps -q --filter "label=com.docker.compose.project=jk-opc-client" | xargs -r docker stop
    
    echo "Removing containers..."
    docker ps -aq --filter "label=com.docker.compose.project=jk-opc-client" | xargs -r docker rm
    
    echo "Removing images..."
    docker images --filter "reference=jk-opc-client*" -q | xargs -r docker rmi
    
    echo "Cleaning up build cache..."
    docker volume ls -q --filter "name=jk-opc-client" | xargs -r docker volume rm
    
    # Also clean up any test-related containers/images
    docker images --filter "reference=*test*" -q | grep -E "(opcua|jk-opc)" | xargs -r docker rmi 2>/dev/null || true
    
    echo "✅ Cleanup complete"
}

# Main command dispatcher
case "${1:-help}" in
    "basic")
        run_basic_test
        ;;
    "simple")
        run_simple_test
        ;;
    "comprehensive")
        run_comprehensive_test
        ;;
    "alpine")
        run_alpine_test
        ;;
    "ubuntu")
        run_ubuntu_test
        ;;
    "local")
        run_local_test
        ;;
    "cleanup")
        run_cleanup
        ;;
    "help"|"--help"|"-h")
        show_help
        ;;
    *)
        echo "❌ Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac
