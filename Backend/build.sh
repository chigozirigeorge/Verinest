#!/bin/bash
set -e

echo "=== Building verinest ==="
cargo build --release
echo "=== Build completed ==="