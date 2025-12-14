# xx network Blockchain

A Substrate-based blockchain implementing the cMix privacy protocol. Built on Polkadot SDK `polkadot-stable2509-2`.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Building](#building)
4. [Running](#running)
5. [Testing](#testing)
6. [Benchmarking](#benchmarking)
7. [Code Quality](#code-quality)
8. [Documentation](#documentation)
9. [License](#license)

---

## Overview

The xx network blockchain features:

- **cMix Integration**: Privacy-preserving messaging protocol with on-chain validator performance tracking
- **Custom Staking**: Extended staking with cMix ID management and point deductions
- **Team Custody**: Time-locked token vesting for team allocations
- **Cross-Chain Bridge**: ChainBridge integration for token transfers
- **Smart Contracts**: Support for Solidity (via pallet-revive) and ink! contracts

| Property | Value |
|----------|-------|
| Polkadot SDK | `polkadot-stable2509-2` |
| SS58 Prefix | 55 |
| Consensus | BABE + GRANDPA |
| Block Time | 6 seconds |
| Token | XX (9 decimals) |

---

## Prerequisites

### System Requirements

- **OS**: Ubuntu 20.04+ (recommended), macOS, or other Unix-based systems
- **RAM**: 8GB minimum, 16GB recommended
- **Disk**: 100GB+ SSD for full node

### Development Dependencies

Complete the [Rust setup instructions](./doc/rust-setup.md), which covers:

- Rust toolchain via `rustup`
- WASM target: `wasm32-unknown-unknown`
- System dependencies (cmake, openssl, clang, protobuf)

**Quick setup (Ubuntu/Debian):**

```bash
# System dependencies
sudo apt update
sudo apt install -y cmake pkg-config libssl-dev git build-essential clang libclang-dev curl protobuf-compiler

# Rust
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
rustup default stable
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

**Quick setup (macOS):**

```bash
brew update
brew install openssl cmake protobuf

curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
rustup default stable
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

---

## Building

The Makefile provides all build commands:

### Production Builds

```bash
# Production-ready binary (LTO, single codegen unit)
make build-prod

# Standard release binary
make build-release

# Build all workspace packages
make build
```

### Development Builds

```bash
# Build with fast-runtime feature (accelerated block times for testing)
make build-dev

# Build with runtime-benchmarks feature
make build-bench

# Build with try-runtime feature (for migration testing)
make build-try-runtime

# Build with both fast-runtime and try-runtime
make build-dev-try-runtime
```

### Reproducible Runtime Builds (srtool)

```bash
# Build WASM runtime with srtool for deterministic output
make build-xxnetwork-runtime
```

### Cross-Compilation (macOS to Linux)

```bash
# Install cross-compile toolchain
rustup target add x86_64-unknown-linux-gnu
brew tap SergioBenitez/osxct
brew install x86_64-unknown-linux-gnu

# Add target to build commands
cargo build --release --target=x86_64-unknown-linux-gnu
```

### Build Output

Binaries are located in:
- `./target/release/xxnetwork-chain` (release builds)
- `./target/production/xxnetwork-chain` (production builds)
- `./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm` (WASM runtime)

---

## Running

### Development Node

```bash
# Single-validator development chain
./target/release/xxnetwork-chain --dev

# With detailed logging
./target/release/xxnetwork-chain --dev -lruntime=debug

# Purge development chain data
./target/release/xxnetwork-chain purge-chain --dev
```

### Mainnet Node

```bash
# Run as full node
./target/release/xxnetwork-chain --chain xxnetwork

# Run as validator
./target/release/xxnetwork-chain \
    --chain xxnetwork \
    --validator \
    --name "MyValidator" \
    --base-path /data/xxchain
```

### Common Options

| Option | Description |
|--------|-------------|
| `--chain` | Chain specification: `xxnetwork`, `dev`, or path to JSON |
| `--validator` | Run as validator node |
| `--rpc-port` | RPC server port (default: 9944) |
| `--port` | P2P network port (default: 30333) |
| `--base-path` | Data directory path |
| `--name` | Node name for telemetry |
| `--pruning` | State pruning mode: `archive` or number of blocks |

---

## Testing

### Unit Tests

```bash
# Run all workspace tests
make all-tests

# Run tests for custom pallets only
make test-pallets

# Run tests for a specific pallet
cargo test -p xx-cmix
cargo test -p xx-economics
cargo test -p xx-team-custody
cargo test -p xx-public
cargo test -p xx-staking-extension
cargo test -p chainbridge
cargo test -p swap
cargo test -p claims

# Run a specific test with output
cargo test -p xx-cmix test_set_cmix_hashes -- --nocapture
```

### Test Coverage

| Pallet | Tests |
|--------|-------|
| chainbridge | 16 |
| claims | 27 |
| swap | 13 |
| xx-cmix | 12 |
| xx-economics | 23 |
| xx-public | 16 |
| xx-team-custody | 46 |
| **Total** | **162** |

### Runtime Upgrade Testing

For comprehensive testing including state forking, try-runtime validation, and testnet deployment, see [TESTING.md](./TESTING.md).

**Quick try-runtime test:**

```bash
# Build with try-runtime
make build-try-runtime

# Test against live state (requires Chopsticks)
npm install -g @acala-network/chopsticks
./scripts/chopsticks.sh try-runtime \
    --config=chopsticks.yml \
    --runtime=./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm
```

---

## Benchmarking

Generate weights for all pallets:

```bash
# Build benchmarking binary
make build-bench

# Run all benchmarks (takes several hours)
./scripts/benchmark.sh
```

Weight files are output to `./runtime/xxnetwork/src/weights/`.

The benchmark script tests 34 pallets including all custom xx network pallets.

---

## Code Quality

```bash
# Format code (requires nightly)
make fmt

# Check formatting without changes
make fmt-check

# Run clippy linter
make lint

# Run clippy with auto-fix
make lint-fix
```

---

## Documentation

### Architecture & Development

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./doc/ARCHITECTURE.md) | System architecture, pallet structure, runtime configuration |
| [TESTING.md](./TESTING.md) | Testnet deployment, verification checklists, testing procedures |
| [CHANGELOG.md](./CHANGELOG.md) | Version history and migration notes |

### Smart Contracts

| Document | Description |
|----------|-------------|
| [EVM.md](./doc/EVM.md) | Deploying Solidity contracts via pallet-revive |
| [INK.md](./doc/INK.md) | Deploying ink! Rust smart contracts |

### Bridge Integration

| Document | Description |
|----------|-------------|
| [BRIDGEHUB.md](./doc/BRIDGEHUB.md) | Polkadot Bridge Hub integration guide |

### Security

| Document | Description |
|----------|-------------|
| [ChainSafe Code Review](./doc/ChainSafe%20xxchain%20Code%20Review.pdf) | Security audit of staking modifications and custom pallets |

### Setup

| Document | Description |
|----------|-------------|
| [rust-setup.md](./doc/rust-setup.md) | Development environment setup |

---

## Project Structure

```
xxchain/
├── cli/                    # Node binary (xxnetwork-chain)
├── runtime/
│   ├── xxnetwork/          # Main runtime
│   └── common/             # Shared constants and implementations
├── xx-cmix/                # cMix protocol integration
├── xx-economics/           # Inflation and rewards
├── xx-team-custody/        # Team token vesting
├── xx-public/              # Public accounts registry
├── xx-staking-extension/   # Staking extensions for cMix
├── chainbridge/            # Cross-chain bridge
├── swap/                   # Token swap via bridge
├── claims/                 # Token claims with ETH signatures
├── migrations/             # Runtime storage migrations
├── primitives/             # Core types
├── rpc/                    # Custom RPC endpoints
├── executor/               # WASM executor
├── scripts/                # Build and benchmark scripts
└── doc/                    # Documentation
```

---

## License

[GPLv3](./LICENSE)
