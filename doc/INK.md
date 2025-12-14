# INK.md

# ink! Smart Contracts on xx Network

This guide explains how to develop and deploy ink! smart contracts on the xx network using `cargo-contract`.

**Last Updated**: 2025-12-14

## Table of Contents

1. [Overview](#1-overview)
2. [Prerequisites](#2-prerequisites)
3. [Creating a Contract](#3-creating-a-contract)
4. [Building Contracts](#4-building-contracts)
5. [Deploying Contracts](#5-deploying-contracts)
6. [Interacting with Contracts](#6-interacting-with-contracts)
7. [Address Resolution](#7-address-resolution)
8. [Example: Flipper Contract](#8-example-flipper-contract)
9. [Testing](#9-testing)
10. [References](#10-references)

---

## 1. Overview

ink! is a Rust-based embedded domain-specific language (eDSL) for writing smart contracts on Polkadot SDK chains. On the xx network, ink! contracts are compiled to RISC-V bytecode and executed on PolkaVM via `pallet-revive`.

### Key Features

- **Rust Safety**: Leverage Rust's type system and memory safety
- **Familiar Tooling**: Use cargo, rustfmt, clippy, and standard Rust tooling
- **Substrate Integration**: Native integration with Substrate's storage and events
- **PolkaVM Execution**: High-performance RISC-V execution environment

### Version Requirements

| Component | Version | Notes |
|-----------|---------|-------|
| **cargo-contract** | v6.0.0+ | Required for pallet-revive compatibility |
| **ink!** | v6.0.0+ | RISC-V/PolkaVM target support |
| **Rust** | stable | With `rust-src` component |

---

## 2. Prerequisites

### Install Rust

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add rust-src component (required for contract compilation)
rustup component add rust-src
```

### Install cargo-contract

```bash
# Install the latest cargo-contract
cargo install --force --locked cargo-contract

# Verify installation
cargo contract --version
```

### Optional: Docker Environment

For reproducible builds, use the official ink! Docker image:

```bash
# Pull the latest stable image
docker pull useink/ci

# Run commands in container
docker run --rm -it -v "$(pwd):/sources" useink/ci cargo contract build
```

---

## 3. Creating a Contract

### Generate New Contract

```bash
# Create a new contract project
cargo contract new my_contract
cd my_contract

# Project structure:
# my_contract/
# ├── Cargo.toml      # Rust package manifest
# ├── .gitignore
# └── lib.rs          # Contract source code
```

### Contract Template

A new contract includes a basic flipper-style template:

```rust
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod my_contract {
    #[ink(storage)]
    pub struct MyContract {
        value: bool,
    }

    impl MyContract {
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value }
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(false)
        }

        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }
}
```

---

## 4. Building Contracts

### Standard Build

```bash
# Build in release mode (recommended for deployment)
cargo contract build --release

# Build output location:
# target/ink/
# ├── my_contract.contract  # Complete bundle (code + metadata)
# ├── my_contract.polkavm   # Compiled bytecode only
# └── my_contract.json      # Metadata only
```

### Build Options

| Option | Description |
|--------|-------------|
| `--release` | Optimize for size and performance |
| `--features` | Enable additional features |
| `--verifiable` | Deterministic builds for verification |

### Verifiable Builds

For production deployments, use verifiable builds:

```bash
cargo contract build --release --verifiable
```

This ensures reproducible builds that can be independently verified.

---

## 5. Deploying Contracts

### Option 1: cargo-contract CLI

**Upload and Instantiate in One Step:**

```bash
# Deploy contract with constructor arguments
cargo contract instantiate \
    --suri //Alice \
    --url ws://localhost:9944 \
    --constructor new \
    --args true \
    --execute
```

**Upload Code First, Then Instantiate:**

```bash
# Step 1: Upload contract code
cargo contract upload \
    --suri //Alice \
    --url ws://localhost:9944 \
    --execute

# Step 2: Instantiate from uploaded code
cargo contract instantiate \
    --suri //Alice \
    --url ws://localhost:9944 \
    --constructor new \
    --args true \
    --code-hash 0x... \
    --execute
```

### Option 2: Polkadot.js Apps

1. Navigate to **Developer → Contracts** in Polkadot.js Apps
2. Click **"Upload & deploy code"**
3. Select your account (wallet)
4. Upload the `.contract` file
5. Configure constructor parameters
6. Submit and sign the transaction

### Option 3: Programmatic Deployment

Use the `Revive::instantiate` extrinsic:

```rust
Revive::instantiate(
    origin,               // Deployer account
    value,                // Initial balance (XX tokens)
    gas_limit,            // Maximum gas
    storage_deposit_limit, // Maximum storage deposit
    code,                 // .polkavm bytecode
    data,                 // SCALE-encoded constructor + args
    salt                  // Unique deployment salt
)
```

---

## 6. Interacting with Contracts

### Using cargo-contract CLI

**Call a Message (State-Changing):**

```bash
cargo contract call \
    --suri //Alice \
    --url ws://localhost:9944 \
    --contract 5GrwvaEF... \
    --message flip \
    --execute
```

**Dry-Run a Call (Read-Only):**

```bash
cargo contract call \
    --suri //Alice \
    --url ws://localhost:9944 \
    --contract 5GrwvaEF... \
    --message get \
    --dry-run
```

### Using Polkadot.js Apps

1. Navigate to **Developer → Contracts**
2. Click on an existing contract or add by address
3. Use the contract interface to call messages
4. Read messages execute immediately
5. Write messages require transaction signing

### Programmatic Calls

Use the `Revive::call` extrinsic:

```rust
Revive::call(
    origin,               // Caller account
    dest,                 // Contract address
    value,                // XX tokens to send
    gas_limit,            // Maximum gas
    storage_deposit_limit, // Maximum additional deposit
    input_data            // SCALE-encoded message + args
)
```

---

## 7. Address Resolution

ink! contracts on xx network use the `AccountId32Mapper` for address mapping between Substrate and Ethereum-style addresses.

### Resolve H160 to AccountId32

```bash
# Get the Substrate AccountId for an H160 contract address
cargo contract account \
    --address 0x1234567890abcdef1234567890abcdef12345678
```

### Address Format Comparison

| Type | Format | Length |
|------|--------|--------|
| Substrate | SS58 encoded | 32 bytes |
| Ethereum | Hex with 0x | 20 bytes |

### Example Mapping

```
Substrate AccountId32:
  5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

Ethereum H160 (first 20 bytes):
  0xd43593c715fdd31c61141abd04a99fd6822c8558
```

---

## 8. Example: Flipper Contract

A complete example of a simple state-toggling contract:

### Contract Code (`lib.rs`)

```rust
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod flipper {
    /// Stores a single boolean value that can be flipped.
    #[ink(storage)]
    pub struct Flipper {
        value: bool,
    }

    impl Flipper {
        /// Creates a new flipper contract with the given initial value.
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value }
        }

        /// Creates a new flipper contract with default value (false).
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(false)
        }

        /// Flips the current value from true to false or vice versa.
        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

        /// Returns the current value of the flipper.
        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn default_works() {
            let flipper = Flipper::default();
            assert!(!flipper.get());
        }

        #[ink::test]
        fn it_works() {
            let mut flipper = Flipper::new(false);
            assert!(!flipper.get());
            flipper.flip();
            assert!(flipper.get());
        }
    }
}
```

### Cargo.toml

```toml
[package]
name = "flipper"
version = "0.1.0"
edition = "2021"

[dependencies]
ink = { version = "6.0", default-features = false }

[dev-dependencies]
ink_e2e = { version = "6.0" }

[lib]
path = "lib.rs"

[features]
default = ["std"]
std = ["ink/std"]
ink-as-dependency = []
e2e-tests = []
```

### Build and Deploy

```bash
# Build the contract
cargo contract build --release

# Deploy to local node
cargo contract instantiate \
    --suri //Alice \
    --url ws://localhost:9944 \
    --constructor default \
    --execute

# Interact with the contract
cargo contract call \
    --suri //Alice \
    --url ws://localhost:9944 \
    --contract <CONTRACT_ADDRESS> \
    --message flip \
    --execute
```

---

## 9. Testing

### Unit Tests

Run unit tests with:

```bash
cargo test
```

### End-to-End Tests

ink! supports e2e tests that run against a real node:

```bash
# Run e2e tests (requires running node)
cargo test --features e2e-tests
```

### Example E2E Test

```rust
#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {
    use super::*;
    use ink_e2e::ContractsBackend;

    type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[ink_e2e::test]
    async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // Deploy contract
        let mut constructor = FlipperRef::new(false);
        let contract = client
            .instantiate("flipper", &ink_e2e::alice(), &mut constructor)
            .submit()
            .await
            .expect("instantiate failed")
            .call_builder::<Flipper>();

        // Call flip
        let _flip_res = client
            .call(&ink_e2e::alice(), &contract.flip())
            .submit()
            .await;

        // Verify state changed
        let get_res = client
            .call(&ink_e2e::alice(), &contract.get())
            .dry_run()
            .await?;
        assert!(get_res.return_value());

        Ok(())
    }
}
```

---

## 10. References

### Official Documentation

- [ink! Documentation](https://use.ink/)
- [cargo-contract GitHub](https://github.com/use-ink/cargo-contract)
- [ink! Examples](https://github.com/use-ink/ink-examples)
- [pallet-revive Documentation](https://paritytech.github.io/polkadot-sdk/master/pallet_revive/index.html)

### xx Network Configuration

- Runtime: `runtime/xxnetwork/src/lib.rs` (lines 1286-1327)
- Chain ID: 55
- Pallet Index: 44
- Substrate RPC: `ws://localhost:9944`

### Tutorials

- [ink! Workshop](https://docs.substrate.io/tutorials/smart-contracts/)
- [Polkadot SDK and ink!](https://use.ink/docs/v6/background/polkadot-sdk/)

### Tools

- [Contracts UI](https://contracts-ui.substrate.io/) - Web interface for contract interaction
- [Polkadot.js Apps](https://polkadot.js.org/apps/) - Full Substrate interface
