# WHSpr Soroban Contract

A Soroban smart contract for the WHSpr Stellar application. This contract provides the core blockchain functionality for the WHSpr platform.

## 🚀 Quick Start

### Prerequisites

- Rust toolchain (stable channel)
- Soroban CLI
- wasm32v1-none target

See [INSTALLATION.md](./INSTALLATION.md) for detailed setup instructions.

#### Install Rust Toolchain

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart terminal or source the profile
source ~/.cargo/env

# Verify installation
rustc --version
```

#### Install Soroban CLI

```bash
# Install Soroban CLI using Cargo
cargo install soroban-cli

# Verify installation
soroban --version
```

#### Add Required Rust Targets

```bash
# Add the WASM target for Soroban
rustup target add wasm32v1-none
rustup target add wasm32-unknown-unknown
```

### Verify Installation

```bash
# Run the verification script
./verify-setup.sh
```

### Build and Test

```bash
# Run tests
cargo test

# Build the contract
soroban contract build
```

## 📁 Project Structure

```
contract/
├── src/
│   ├── lib.rs          # Main contract code
│   └── test.rs         # Contract tests
├── Cargo.toml          # Rust project configuration
├── verify-setup.sh     # Environment verification script
├── INSTALLATION.md     # Platform-specific setup guide
└── README.md          # This file
```

## 🛠️ Development

### Contract Functions

The current contract includes a simple `hello` function that demonstrates basic Soroban contract functionality:

```rust
pub fn hello(env: Env, to: String) -> Vec<String>
```

### Adding New Functions

1. Add your function to the `Contract` impl block in `src/lib.rs`
2. Add corresponding tests in `src/test.rs`
3. Run `cargo test` to verify your changes
4. Build with `soroban contract build`

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_function_name

# Run tests with output
cargo test -- --nocapture
```

## 🏗️ Building

### Development Build

```bash
cargo build
```

### Production Build

```bash
soroban contract build
```

This creates an optimized WASM file at `target/wasm32v1-none/release/whspr_contract.wasm`.

## 📦 Deployment

### Testnet Deployment

```bash
# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32v1-none/release/whspr_contract.wasm \
  --source-account <your-account> \
  --network testnet
```

### Mainnet Deployment

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

```bash
# Deploy to mainnet (be careful!)
soroban contract deploy \
  --wasm target/wasm32v1-none/release/whspr_contract.wasm \
  --source-account <your-account> \
  --network mainnet
```

## 🔧 Configuration

### Environment Variables

- `SOROBAN_NETWORK`: Network to use (testnet/mainnet)
- `SOROBAN_SOURCE_ACCOUNT`: Default source account

### Cargo.toml

The contract uses Soroban SDK version 23.0.2. Update the version in `Cargo.toml` as needed.

## 📚 Resources

- [Soroban Documentation](https://developers.stellar.org/docs/build/smart-contracts/overview)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
- [Stellar Developer Portal](https://developers.stellar.org/)
- [Rust Book](https://doc.rust-lang.org/book/)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run `cargo test` to ensure all tests pass
6. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🆘 Support

- [Stellar Developer Discord](https://discord.gg/stellar)
- [GitHub Issues](https://github.com/your-repo/issues)
- [Documentation](https://developers.stellar.org/docs/build/smart-contracts/overview)
