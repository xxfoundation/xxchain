# EVM.md

# Solidity Smart Contracts on xx Network

This guide explains how to deploy Solidity smart contracts on the xx network using `pallet-revive`.

**Last Updated**: 2025-12-14

## Table of Contents

1. [Overview](#1-overview)
2. [Chain Configuration](#2-chain-configuration)
3. [Compilation Pipeline](#3-compilation-pipeline)
4. [Development Tools](#4-development-tools)
5. [Deploying Contracts](#5-deploying-contracts)
6. [Interacting with Contracts](#6-interacting-with-contracts)
7. [ReviveApi Methods](#7-reviveapi-methods)
8. [Address Mapping](#8-address-mapping)
9. [Security Considerations](#9-security-considerations)
10. [References](#10-references)

---

## 1. Overview

The xx network supports Solidity smart contracts through `pallet-revive`, which executes contracts on PolkaVM (a RISC-V based virtual machine). Unlike traditional EVM chains, Solidity code is compiled to RISC-V bytecode using the **Revive compiler**, enabling high-performance execution within the Polkadot SDK framework.

### Key Features

- **Ethereum Compatibility**: Use familiar Solidity tooling and RPC methods
- **PolkaVM Execution**: Optimized RISC-V VM for smart contract execution
- **Native Integration**: Contracts integrate with Substrate's weight-based fee system
- **Sandboxed Security**: Contracts cannot access runtime extrinsics directly

---

## 2. Chain Configuration

| Parameter | Value | Description |
|-----------|-------|-------------|
| **Chain ID** | 55 | Matches xx network SS58 prefix |
| **Native Decimals** | 9 | XX token uses 9 decimals |
| **ETH Decimals** | 18 | Standard Ethereum decimal places |
| **Native-to-ETH Ratio** | 10^9 | Conversion factor between systems |
| **Runtime Memory** | 128 MB | Maximum memory per contract execution |
| **PVF Memory** | 512 MB | Maximum memory for contract compilation |
| **Pallet Index** | 44 | Runtime pallet position |

### MetaMask Configuration

To connect MetaMask to xx network:

| Setting | Value |
|---------|-------|
| Network Name | xx Network |
| Chain ID | 55 |
| Currency Symbol | XX |
| RPC URL | `http://localhost:8545` (local) or your node's ETH-RPC endpoint |

---

## 3. Compilation Pipeline

Solidity contracts are compiled through the following pipeline:

```
Solidity Source
      │
      ▼
┌─────────────┐
│    Solc     │  (Ethereum Solidity Compiler)
└─────────────┘
      │
      ▼
   YUL IR
      │
      ▼
┌─────────────┐
│   resolc    │  (Revive Compiler - YUL to LLVM)
└─────────────┘
      │
      ▼
   LLVM IR
      │
      ▼
┌─────────────┐
│    LLVM     │  (Optimization & RISC-V codegen)
└─────────────┘
      │
      ▼
  RISC-V ELF
      │
      ▼
┌─────────────┐
│ PVM Linker  │  (Creates final blob with metadata)
└─────────────┘
      │
      ▼
 PolkaVM Blob  (Deployable bytecode)
```

### Installing the Revive Compiler

You need both `resolc` (Revive compiler) and `solc` (Ethereum Solidity compiler):

```bash
# Install solc (Ethereum Solidity compiler)
# macOS
brew install solidity

# Linux (Ubuntu/Debian)
sudo add-apt-repository ppa:ethereum/ethereum
sudo apt update
sudo apt install solc

# Install resolc (Revive compiler)
cargo install --locked revive-solc

# Verify installation
resolc --version
solc --version
```

### Compiling a Contract

```bash
# Compile Solidity to PolkaVM bytecode
resolc --bin -O3 MyContract.sol -o output/

# The output directory will contain:
# - MyContract.polkavm (deployable bytecode)
```

---

## 4. Development Tools

### Polkadot Remix IDE

The easiest way to develop and deploy Solidity contracts is using **Polkadot Remix**:

**URL**: https://remix.polkadot.io

1. Write or import your Solidity contract
2. Compile using the built-in Revive compiler
3. Connect MetaMask (configured for xx network)
4. Deploy using "Injected Provider - MetaMask"

### Hardhat / Foundry

Standard EVM development tools work with xx network by switching the RPC endpoint:

**Hardhat Configuration** (`hardhat.config.js`):
```javascript
module.exports = {
  solidity: "0.8.20",
  networks: {
    xxnetwork: {
      url: "http://localhost:8545",  // Your ETH-RPC endpoint
      chainId: 55,
      accounts: [process.env.PRIVATE_KEY]
    }
  }
};
```

**Note**: You'll need to use the Revive compiler instead of standard Solc for deployment. Pre-compile contracts with `resolc` and deploy the resulting bytecode.

---

## 5. Deploying Contracts

### Via Remix IDE

1. Open https://remix.polkadot.io
2. Create or upload your `.sol` file
3. Compile with Solidity Compiler plugin
4. Switch to "Deploy & Run Transactions"
5. Select "Injected Provider - MetaMask"
6. Ensure MetaMask is connected to xx network (Chain ID 55)
7. Click "Deploy" and confirm the transaction

### Via Substrate Extrinsics

Deploy programmatically using `Revive::instantiate`:

```rust
// Extrinsic signature
Revive::instantiate(
    origin,               // Caller account
    value,                // XX tokens to send to contract
    gas_limit,            // Maximum gas for deployment
    storage_deposit_limit, // Maximum storage deposit
    code,                 // PolkaVM bytecode
    data,                 // Constructor arguments (ABI-encoded)
    salt                  // Unique salt for address derivation
)
```

### Via Polkadot.js Apps

1. Navigate to Developer → Extrinsics
2. Select `revive` → `instantiate`
3. Fill in parameters:
   - **value**: Initial balance for contract
   - **gasLimit**: e.g., `{ refTime: 1000000000, proofSize: 1000000 }`
   - **code**: Upload your `.polkavm` file
   - **data**: Constructor arguments (hex-encoded)
   - **salt**: Unique bytes for address generation

---

## 6. Interacting with Contracts

### Calling Contract Methods

```rust
// Extrinsic signature
Revive::call(
    origin,               // Caller account
    dest,                 // Contract address
    value,                // XX tokens to send
    gas_limit,            // Maximum gas for call
    storage_deposit_limit, // Maximum additional storage deposit
    input_data            // Method selector + arguments (ABI-encoded)
)
```

### Dry-Run Calls (Gas Estimation)

Use the `call` RPC method to simulate execution without submitting a transaction:

```javascript
// Using ethers.js
const result = await provider.call({
  to: contractAddress,
  data: encodedFunctionCall
});
```

---

## 7. ReviveApi Methods

The xx network exposes ETH-RPC compatible methods via the `ReviveApi` runtime API:

| Method | Description |
|--------|-------------|
| `eth_transact` | Execute Ethereum-style transactions (signed RLP-encoded) |
| `call` | Dry-run contract calls for gas estimation |
| `instantiate` | Deploy contracts with value and constructor arguments |
| `upload_code` | Upload contract code for later instantiation |
| `balance` | Query EVM-compatible account balance |
| `nonce` | Query account nonce |
| `gas_price` | Query current gas price |
| `get_storage` | Inspect contract storage |
| `get_storage_var_key` | Get storage variable key |
| `trace_block` | Full transaction tracing for debugging |
| `trace_tx` | Trace specific transaction |
| `trace_call` | Trace a call |
| `address` | Address helper methods |
| `block_author` | Query block author |
| `code` | Retrieve contract bytecode |

---

## 8. Address Mapping

The xx network uses `AccountId32Mapper` for bidirectional mapping between Substrate and Ethereum addresses:

### Substrate → Ethereum

```
AccountId32 (32 bytes) → H160 (20 bytes)
                         ↑
           First 20 bytes of AccountId32
```

### Ethereum → Substrate

When interacting with contracts using Ethereum-style addresses, the system automatically maps them to full Substrate AccountId32 addresses.

### Example

```
Substrate: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
           (32-byte AccountId32)

Ethereum:  0xd43593c715fdd31c61141abd04a99fd6822c8558
           (First 20 bytes as H160)
```

---

## 9. Security Considerations

### Sandboxed Execution

Smart contracts on xx network are fully sandboxed:

- **No runtime access**: Contracts cannot call runtime extrinsics directly
- **Limited interactions**: Can only interact with other contracts, use precompiles, and transfer tokens
- **Storage deposits**: Required for contract storage to prevent state bloat

### Storage Deposits

| Type | Cost |
|------|------|
| Per byte | `deposit(0, 1)` |
| Per item | `deposit(1, 0)` |
| Code hash lockup | 30% of deposit |

### Gas/Weight Conversion

The `NativeToEthRatio` of 10^9 converts between:
- **Substrate weight units**: Picoseconds of execution time
- **Ethereum gas units**: Abstract gas measurement

### Best Practices

1. **Test thoroughly**: Use local dev network before mainnet
2. **Audit contracts**: Security audits are recommended for production contracts
3. **Monitor storage**: Be aware of storage deposit requirements
4. **Handle failures**: Implement proper error handling and fallbacks

---

## 10. References

### Official Documentation

- [pallet-revive Documentation](https://paritytech.github.io/polkadot-sdk/master/pallet_revive/index.html)
- [PolkaVM Design](https://docs.polkadot.com/polkadot-protocol/smart-contract-basics/polkavm-design/)
- [Revive Compiler GitHub](https://github.com/paritytech/revive)
- [Polkadot Remix IDE](https://remix.polkadot.io)
- [Smart Contracts on Polkadot](https://wiki.polkadot.com/learn/learn-smart-contracts/)

### xx Network Configuration

- Runtime: `runtime/xxnetwork/src/lib.rs` (lines 1286-1327)
- EthExtraImpl: `runtime/xxnetwork/src/lib.rs` (lines 1531-1556)
- Chain ID: 55
- Pallet Index: 44
