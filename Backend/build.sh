#!/bin/bash
set -e

echo "=== Building verinest ==="
cargo build --release

echo "=== Finding binary ==="
# Find the actual binary location
find target/ -name "verinest" -type f 2>/dev/null || echo "Binary not found with find"

# List all binaries in release
find target/release/ -maxdepth 1 -type f -executable 2>/dev/null || echo "No executables in release"

echo "=== Build completed ==="