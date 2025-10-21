#!/bin/bash
set -e

echo "=== Environment Info ==="
rustc --version
cargo --version

echo "=== Current Directory ==="
pwd
ls -la

echo "=== Checking Source Files ==="
find src/ -name "*.rs" | head -10

echo "=== Attempting Build ==="
# First try a simple build check
cargo check --release || {
    echo "=== Cargo check failed, showing errors ==="
    # Try to build with full error output
    cargo build --release 2>&1 | head -100
    exit 1
}

echo "=== Cargo check passed, building release ==="
cargo build --release

echo "=== Build Results ==="
if [ -f "target/release/verinest" ]; then
    echo "✅ Binary built successfully at target/release/verinest"
    ls -la target/release/verinest
    echo "=== Testing binary ==="
    ./target/release/verinest --version 2>/dev/null || echo "Binary test failed but file exists"
    
    # Copy to expected location
    mkdir -p bin
    cp target/release/verinest bin/
    chmod +x bin/verinest
    echo "✅ Binary copied to bin/verinest"
else
    echo "❌ Binary not found! Checking what was built:"
    find target/ -type f -executable 2>/dev/null
    echo "=== Full target directory ==="
    ls -la target/ 2>/dev/null || echo "No target directory"
    exit 1
fi

echo "=== Build completed successfully ==="