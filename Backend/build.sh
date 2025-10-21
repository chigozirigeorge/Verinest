#!/bin/bash
set -e

echo "=== Building verinest ==="
cargo build --release

echo "=== Copying binary to /bin directory ==="
# Create bin directory if it doesn't exist
mkdir -p bin

# Copy the binary to where Railway expects it
cp target/release/verinest bin/verinest

# Make sure it's executable
chmod +x bin/verinest

echo "=== Verification ==="
echo "Contents of bin/:"
ls -la bin/

echo "=== Build completed ==="