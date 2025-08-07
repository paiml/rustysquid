#!/bin/bash

# Build script for router-cache proxy
# Uses Docker to cross-compile for ARM64

set -e

echo "🔨 Building router-cache for ARM64..."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is required but not installed"
    echo "   Please install Docker or use cargo-cross"
    exit 1
fi

# Use messense/rust-musl-cross for cross-compilation
docker run --rm -it \
    -v "$(pwd)":/home/rust/src \
    messense/rust-musl-cross:aarch64-musl \
    cargo build --release

# Binary will be in target/aarch64-unknown-linux-musl/release/router-cache
BINARY_PATH="target/aarch64-unknown-linux-musl/release/router-cache"

if [ -f "$BINARY_PATH" ]; then
    echo "✅ Build successful!"
    echo "📦 Binary size: $(du -h $BINARY_PATH | cut -f1)"
    echo "📍 Binary location: $BINARY_PATH"
    
    # Strip the binary for smaller size
    docker run --rm -it \
        -v "$(pwd)":/home/rust/src \
        messense/rust-musl-cross:aarch64-musl \
        aarch64-linux-musl-strip /home/rust/src/$BINARY_PATH
    
    echo "📦 Stripped size: $(du -h $BINARY_PATH | cut -f1)"
else
    echo "❌ Build failed - binary not found"
    exit 1
fi