#!/bin/bash

# Soroban Development Environment Verification Script
# This script verifies that all required tools are properly installed

echo "🔍 Verifying Soroban Development Environment..."
echo "================================================"

# Check Rust installation
echo "📦 Checking Rust installation..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo "✅ Rust installed: $RUST_VERSION"
else
    echo "❌ Rust not found. Please install Rust first."
    exit 1
fi

# Check Rust target for Soroban
echo "🎯 Checking Rust targets..."
if rustup target list --installed | grep -q "wasm32v1-none"; then
    echo "✅ wasm32v1-none target installed"
else
    echo "❌ wasm32v1-none target not found. Installing..."
    rustup target add wasm32v1-none
    echo "✅ wasm32v1-none target installed"
fi

# Check Soroban CLI installation
echo "🛠️  Checking Soroban CLI..."
if command -v soroban &> /dev/null; then
    SOROBAN_VERSION=$(soroban --version)
    echo "✅ Soroban CLI installed: $SOROBAN_VERSION"
else
    echo "❌ Soroban CLI not found. Please install it first."
    exit 1
fi

# Check Cargo
echo "📦 Checking Cargo..."
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    echo "✅ Cargo installed: $CARGO_VERSION"
else
    echo "❌ Cargo not found. Please install Cargo first."
    exit 1
fi

# Test contract build
echo "🔨 Testing contract build..."
if cargo test; then
    echo "✅ Contract tests pass"
else
    echo "❌ Contract tests failed"
    exit 1
fi

# Test contract compilation
echo "🏗️  Testing contract compilation..."
if soroban contract build; then
    echo "✅ Contract builds successfully"
else
    echo "❌ Contract build failed"
    exit 1
fi

# Check if WASM file was generated
if [ -f "target/wasm32v1-none/release/whspr_contract.wasm" ]; then
    echo "✅ WASM file generated: target/wasm32v1-none/release/whspr_contract.wasm"
    WASM_SIZE=$(du -h target/wasm32v1-none/release/whspr_contract.wasm | cut -f1)
    echo "📊 WASM file size: $WASM_SIZE"
else
    echo "❌ WASM file not found"
    exit 1
fi

echo ""
echo "🎉 All checks passed! Your Soroban development environment is ready."
echo "================================================"
echo ""
echo "📋 Quick Commands:"
echo "  cargo test              - Run contract tests"
echo "  soroban contract build  - Build contract for deployment"
echo "  soroban --help          - Show Soroban CLI help"
echo ""
echo "🚀 Happy coding!"
