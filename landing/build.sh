#!/bin/bash
# Build script for loctree landing page
# Requires Rust stable (1.88+, currently 1.91)

set -e

echo "=== Setting up Rust stable ==="
rustup default stable
rustup update stable
rustup target add wasm32-unknown-unknown

echo "=== Rust version ==="
rustc --version

echo "=== Building with Trunk ==="
trunk build --release

echo "=== Build complete ==="
ls -la dist/

echo ""
echo "API endpoints:"
echo "  /api/agent/index.txt  - plain text prompt"
echo "  /api/agent/index.json - JSON format"
echo ""
echo "To serve: simple-http-server dist -p 8080 --index"
