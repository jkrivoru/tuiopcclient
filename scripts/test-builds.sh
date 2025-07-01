#!/bin/bash

# Build verification script for all target platforms
# This script helps test cross-compilation locally before pushing

set -e

echo "🔨 Testing cross-compilation for all release targets"

# Install required targets
echo "📦 Installing required Rust targets..."
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Install cross if not available
if ! command -v cross &> /dev/null; then
    echo "📦 Installing cross for cross-compilation..."
    # Try git version first, fallback to stable if it fails
    if ! cargo install cross --git https://github.com/cross-rs/cross; then
        echo "⚠️  Git version failed, trying stable version..."
        cargo install cross
    fi
fi

echo ""
echo "🧪 Testing builds for each target..."

# Define targets that can be built on current platform
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    TARGETS=(
        "x86_64-unknown-linux-gnu"
        "x86_64-unknown-linux-musl"
        "aarch64-unknown-linux-gnu"
    )
elif [[ "$OSTYPE" == "darwin"* ]]; then
    TARGETS=(
        "x86_64-apple-darwin"
        "aarch64-apple-darwin"
    )
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    TARGETS=(
        "x86_64-pc-windows-msvc"
        "i686-pc-windows-msvc"
    )
else
    echo "⚠️  Unknown OS type: $OSTYPE"
    TARGETS=("x86_64-unknown-linux-gnu")
fi

FAILED_TARGETS=()
SUCCESSFUL_TARGETS=()

for target in "${TARGETS[@]}"; do
    echo "🔨 Building for $target..."
    
    if [[ "$target" == *"linux"* ]] && [[ "$OSTYPE" != "linux-gnu"* ]]; then
        # Use cross for Linux targets on non-Linux hosts
        if cross build --release --target "$target"; then
            echo "✅ $target build successful"
            SUCCESSFUL_TARGETS+=("$target")
        else
            echo "❌ $target build failed"
            FAILED_TARGETS+=("$target")
        fi
    else
        # Use regular cargo for native targets
        if cargo build --release --target "$target"; then
            echo "✅ $target build successful"
            SUCCESSFUL_TARGETS+=("$target")
        else
            echo "❌ $target build failed"
            FAILED_TARGETS+=("$target")
        fi
    fi
    echo ""
done

echo "📊 Build Summary:"
echo "=================="

if [ ${#SUCCESSFUL_TARGETS[@]} -gt 0 ]; then
    echo "✅ Successful builds:"
    for target in "${SUCCESSFUL_TARGETS[@]}"; do
        echo "   - $target"
    done
fi

if [ ${#FAILED_TARGETS[@]} -gt 0 ]; then
    echo "❌ Failed builds:"
    for target in "${FAILED_TARGETS[@]}"; do
        echo "   - $target"
    done
    echo ""
    echo "⚠️  Some builds failed. Check the output above for details."
    exit 1
else
    echo ""
    echo "🎉 All builds successful! Ready for release."
fi
