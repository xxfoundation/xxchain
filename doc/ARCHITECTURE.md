# ARCHITECTURE.md

This document provides a comprehensive overview of the xx network blockchain architecture for developers working on the Polkadot SDK upgrade and parachain migration.

## Table of Contents

1. [Repository Overview](#1-repository-overview)
2. [Directory Structure](#2-directory-structure)
3. [Runtime Architecture](#3-runtime-architecture)
4. [Custom xx Network Pallets](#4-custom-xx-network-pallets)
5. [Bridge and Claims Pallets](#5-bridge-and-claims-pallets)
6. [Custom Trait Integration](#6-custom-trait-integration)
7. [Consensus and Block Production](#7-consensus-and-block-production)
8. [Dependencies and Versioning](#8-dependencies-and-versioning)

---

## 1. Repository Overview

### Project Description

The xx network is a Substrate-based blockchain implementing the cMix privacy protocol. It features custom staking integration for validator performance tracking, team token custody with time-locked vesting, and cross-chain bridge functionality.

### Key Specifications

| Property | Value |
|----------|-------|
| Polkadot SDK | `polkadot-stable2509-2` (v1.20.2) |
| Base Version | November 2024 |
| SS58 Address Prefix | 55 |
| Consensus | BABE + GRANDPA |
| Block Time | 6 seconds |
| Runtime Version | 206 (spec_version) |

### Workspace Structure

The project is a Cargo workspace with 22 member crates:

```
Cargo.toml (workspace root)
├── runtime/xxnetwork     # Single runtime (mainnet/testnet via --chain)
├── runtime/common        # Shared runtime code
├── cli                   # Node binary
├── executor              # WASM executor
├── primitives            # Core types
├── rpc                   # RPC endpoints
├── inspect               # Debug tool
├── migrations            # Runtime migrations
├── testing               # Test utilities
├── utils/generate-bags   # Voter bag generator
└── Custom Pallets:
    ├── xx-cmix
    ├── xx-economics
    ├── xx-team-custody
    ├── xx-betanet-rewards
    ├── xx-public
    ├── chainbridge
    ├── claims
    └── swap
```

---

## 2. Directory Structure

### 2.1 Runtime Directories

#### `runtime/xxnetwork/`
**Purpose**: Production mainnet runtime (v0.2.6)

| File | Description |
|------|-------------|
| `src/lib.rs` | Main runtime configuration, `construct_runtime!`, pallet configs |
| `src/weights/` | Auto-generated benchmark weights for all pallets |
| `src/voter_bags.rs` | Voter bagging list for election provider |
| `Cargo.toml` | Runtime dependencies |
| `build.rs` | WASM build script |

Key runtime constants (from `lib.rs`):
- `spec_version`: 206
- `SS58Prefix`: 55
- Pallet indices: 0-42 allocated

#### `runtime/common/`
**Purpose**: Shared constants, types, and implementations

| File | Description |
|------|-------------|
| `src/lib.rs` | Shared parameters, block limits, fee configuration |
| `src/constants.rs` | Time, currency, and fee constants |
| `src/impls.rs` | `DealWithFees` implementation (80% treasury, 20% author) |

Key constants:
```rust
// Time
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;  // Production: 8 hours
pub const SESSIONS_PER_ERA: u32 = 3;

// Currency
pub const UNITS: Balance = 1_000_000_000;
pub const CENTS: Balance = UNITS / 100;
```

### 2.2 Node Infrastructure

#### `cli/`
**Purpose**: Node binary and CLI (`xxnetwork-cli` package)

| File | Description |
|------|-------------|
| `bin/main.rs` | Entry point for `xxnetwork-chain` binary |
| `src/service.rs` | Node service initialization (BABE, GRANDPA, networking) |
| `src/command.rs` | CLI command handlers |
| `src/chain_spec.rs` | Genesis configurations |
| `src/cli.rs` | Argument parsing |
| `res/xxnetwork.json` | Mainnet genesis spec |

Features:
- `runtime-benchmarks`: Enable benchmarking
- `try-runtime`: Enable try-runtime testing

Chain selection via `--chain` option: `xxnetwork`, `dev`, or custom JSON spec file.

#### `executor/`
**Purpose**: WASM executor wrapper

Provides the `ExecutorDispatch` for both runtimes with native execution support.

#### `primitives/`
**Purpose**: Core types shared across node and runtime

```rust
pub type BlockNumber = u32;
pub type Balance = u128;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Moment = u64;
pub type Index = u32;  // Note: deprecated, becomes Nonce in modern SDK
pub type Hash = sp_core::H256;
pub type Signature = MultiSignature;
```

#### `rpc/`
**Purpose**: RPC API endpoints

Standard Substrate RPC methods:
- System (account nonce)
- TransactionPayment (fee estimation)
- Babe (block production info)
- Grandpa (finality)
- SyncState (synchronization)

**Note**: No custom RPC methods defined for xx network pallets.

#### `inspect/`
**Purpose**: Blockchain debugging and inspection tool

CLI-based utility for examining blocks and runtime state.

### 2.3 Support Directories

#### `migrations/`
**Purpose**: Runtime upgrade migrations

| File | Description |
|------|-------------|
| `src/lib.rs` | Migration exports |
| `src/bridge_adjust.rs` | ChainBridge state migration example |

Uses `OnRuntimeUpgrade` trait with try-runtime hooks for safe migrations.

#### `testing/`
**Purpose**: Test infrastructure

- Mock runtime for unit tests
- Test client setup
- Database fixtures
- Integration test helpers

#### `utils/generate-bags/`
**Purpose**: Generates voter bag thresholds for `pallet-bags-list`

Used for efficient validator election with weighted voter lists.

#### `scripts/`
**Purpose**: Build and development scripts

| Script | Description |
|--------|-------------|
| `benchmark.sh` | Runs benchmarks for all pallets, generates weights |
| `benchmark_bin.sh` | Benchmarks compiled binary |

#### `doc/`
**Purpose**: Documentation

- `rust-setup.md`: Development environment setup
- `ChainSafe xxchain Code Review.pdf`: Security audit report

---

## 3. Runtime Architecture

### 3.1 Pallet Indices

The runtime assigns fixed indices to pallets for storage compatibility:

```rust
construct_runtime!(
    pub enum Runtime {
        // System (0-4)
        System: frame_system = 0,
        Scheduler: pallet_scheduler = 1,
        Babe: pallet_babe = 2,
        Timestamp: pallet_timestamp = 3,
        Balances: pallet_balances = 4,

        // Consensus (5-12)
        Authorship: pallet_authorship = 5,
        Staking: pallet_staking = 6,
        ElectionProviderMultiPhase = 7,
        Offences = 8,
        Session = 9,
        Grandpa = 10,
        ImOnline = 11,
        AuthorityDiscovery = 12,

        // Governance (13-18)
        Democracy = 13,
        Council = 14,
        TechnicalCommittee = 15,
        Elections = 16,
        TechnicalMembership = 17,
        Treasury = 18,

        // Custom xx network (19-34)
        Claims = 19,
        Vesting = 20,
        Utility = 21,
        Identity = 22,
        Proxy = 23,
        TransactionPayment = 24,
        Historical = 25,
        Bounties = 26,
        Tips = 27,
        ChainBridge = 28,
        Swap = 29,
        XXCmix = 30,
        XXEconomics = 31,
        XXCustody = 32,
        XXBetanetRewards = 33,
        XXPublic = 34,

        // Additional (35-42)
        Multisig = 35,
        Recovery = 36,
        Assets = 37,
        Uniques = 38,
        Preimage = 39,
        ChildBounties = 40,
        VoterList = 41,
        Nfts = 42,
    }
);
```

### 3.2 Runtime Configuration Flow

```
                    ┌─────────────────────┐
                    │   frame_system      │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ pallet_staking│◄───│    xx-cmix      │    │  xx-economics   │
│  (forked)     │    │  (CmixHandler)  │    │                 │
└───────┬───────┘    └─────────────────┘    └────────┬────────┘
        │                                            │
        │ CustodyHandler                             │
        ▼                                            ▼
┌───────────────┐                          ┌─────────────────┐
│xx-team-custody│                          │    xx-public    │
└───────────────┘                          │(PublicAccounts) │
                                           └─────────────────┘
```

### 3.3 Weight System

**Current**: 1D weights (single `Weight` value)
**Target**: 2D weights (`Weight { ref_time, proof_size }`)

All custom pallets use `decl_module!` macro with:
```rust
#[weight = <T as Config>::WeightInfo::function_name()]
```

Must be converted to:
```rust
#[pallet::weight(T::WeightInfo::function_name())]
#[pallet::call_index(N)]
```

---

## 4. Custom xx Network Pallets

### 4.1 xx-cmix (`/xx-cmix/src/lib.rs`)

**Purpose**: cMix privacy protocol integration with blockchain

**Storage**:
```rust
CmixHashes: cmix::SoftwareHashes<T::Hash>      // Software version hashes
AdminPermission: T::BlockNumber                 // Admin permission deadline
SchedulingAccount: Option<T::AccountId>         // Scheduling server account
CmixAddressSpace: u8                           // Address space bits
NextCmixVariables: Option<cmix::Variables>     // Pending variable updates
CmixVariables: cmix::Variables                 // Current cMix parameters
```

**Key Types** (`cmix.rs`):
```rust
pub struct Variables {
    pub performance: Performance,      // Points for rounds
    pub timeouts: Timeouts,           // Round timeouts
    pub scheduling: Scheduling,        // Pool and team size
    pub registration: UserRegistration, // User limits
}

pub struct SoftwareHashes<Hash> {
    pub server: Option<Vec<Hash>>,
    pub fatbin: Option<Vec<Hash>>,
    pub libpow: Option<Vec<Hash>>,
    // ... gateway, scheduling, wrapper, udb, notifications
}
```

**Integration with pallet-staking**:
- Implements `CmixHandler` trait (see Section 6)
- Scheduling server submits performance points via `submit_cmix_points`
- Points affect validator rewards at era end

### 4.2 xx-economics (`/xx-economics/src/lib.rs`)

**Purpose**: Token inflation and rewards management

**Storage**:
```rust
InflationParams: inflation::InflationFixedParams  // min_inflation, ideal_stake, falloff
InterestPoints: Vec<IdealInterestPoint>           // Block-indexed interest rates
IdealLiquidityStake: Balance                      // Target liquidity
LiquidityRewards: Balance                         // Current rewards balance
```

**Key Functions**:
- `compute_inflation()`: Calculates era inflation based on staking ratio
- `era_payout()`: Distributes rewards considering custody amounts

**Dependencies**:
- `CustodyHandler` from xx-team-custody
- `PublicAccountsHandler` from xx-public

### 4.3 xx-team-custody (`/xx-team-custody/src/lib.rs`)

**Purpose**: Team token custody with time-locked vesting

**Storage**:
```rust
TeamAccounts: Map<AccountId, CustodyInfo>  // Team member custody info
CustodyAccounts: Map<AccountId, ()>        // Custody account registry
Custodians: Map<AccountId, ()>             // Authorized custodians
TotalCustody: Balance                      // Total locked amount
```

**CustodyInfo Structure**:
```rust
pub struct CustodyInfo<AccountId, Balance> {
    pub allocation: Balance,    // Total allocation
    pub vested: Balance,        // Amount vested so far
    pub custody: AccountId,     // Proxy custody account
    pub reserve: AccountId,     // Reserve account (5%)
}
```

**Key Mechanisms**:
- Uses `pallet_proxy::pure_account()` for custody account derivation
- Two-phase vesting: Custody period → Governance custody period
- Payout frequency-based releases

**Integration**:
- Implements `CustodyHandler` trait for pallet-staking
- Uses `pallet-proxy` for account management

### 4.4 xx-betanet-rewards (`/xx-betanet-rewards/src/lib.rs`)

**Purpose**: BetaNet staking rewards distribution

**Storage**:
```rust
Accounts: Map<AccountId, UserInfo>  // Participant info
Approved: bool                       // Reward approval status
```

**Reward Options**:
```rust
pub enum RewardOption {
    NoVesting,        // 2% rewards
    Vesting1Month,    // 12% rewards
    Vesting3Month,    // 45% rewards
    Vesting6Month,    // 100% rewards (default)
    Vesting9Month,    // 120% rewards + 20% extra
}
```

**Integration**:
- Implements `RewardHandler` for claims pallet
- Processes rewards at `EnactmentBlock` (30 days)

### 4.5 xx-public (`/xx-public/src/lib.rs`)

**Purpose**: Public account management for token distribution

**Storage**:
```rust
TestnetManager: Option<AccountId>  // Testnet distribution manager
SaleManager: Option<AccountId>     // Sale distribution manager
```

**Integration**:
- Implements `PublicAccountsHandler` trait
- Used by xx-economics for public account identification

---

## 5. Bridge and Claims Pallets

### 5.1 chainbridge (`/chainbridge/src/lib.rs`)

**Purpose**: ChainSafe cross-chain bridge implementation

**Storage**:
```rust
ChainNonces: Map<ChainId, DepositNonce>           // Per-chain nonces
RelayerThreshold: u32                              // Required votes
Relayers: Map<AccountId, bool>                     // Relayer registry
Votes: DoubleMap<ChainId, (Nonce, Proposal), ProposalVotes>
Resources: Map<ResourceId, Vec<u8>>               // Resource handlers
```

**Key Types**:
```rust
pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

pub struct ProposalVotes<AccountId, BlockNumber> {
    pub votes_for: Vec<AccountId>,
    pub votes_against: Vec<AccountId>,
    pub status: ProposalStatus,
    pub expiry: BlockNumber,
}
```

**Custom Origin**:
```rust
pub struct EnsureBridge<T>;  // Ensures origin is bridge pallet account
```

### 5.2 swap (`/swap/src/lib.rs`)

**Purpose**: Token swap functionality via ChainBridge

**Storage**:
```rust
SwapFee: Balance                    // Fee for swaps
FeeDestination: Option<AccountId>   // Fee recipient
```

**Integration**:
- Depends on `chainbridge::Config`
- Uses `BridgeOrigin` for bridge-initiated operations
- Calls `chainbridge::transfer_fungible()`

### 5.3 claims (`/claims/src/lib.rs`)

**Purpose**: Ethereum-based token claims

**Storage**:
```rust
Claims: Map<EthereumAddress, Balance>              // Claimable amounts
Total: Balance                                      // Total unclaimed
Vesting: Map<EthereumAddress, Vec<VestingSchedule>> // Vesting schedules
Signing: Map<EthereumAddress, StatementKind>       // Required statements
Preclaims: Map<AccountId, EthereumAddress>         // Pre-linked accounts
Rewards: Map<EthereumAddress, Balance>             // BetaNet rewards
```

**Key Types**:
```rust
pub struct EthereumAddress(pub [u8; 20]);
pub struct EcdsaSignature(pub [u8; 65]);

pub enum StatementKind {
    Regular,  // Standard distribution
    Saft,     // SAFT holder
}
```

**Integration**:
- Uses `secp256k1` for Ethereum signature verification
- Calls `RewardHandler::add_claimed()` (xx-betanet-rewards)
- Integrates with `pallet-vesting`

---

## 6. Custom Trait Integration

### 6.1 Traits in Forked pallet-staking

The xx-labs Substrate fork modifies `pallet-staking` to include custom traits:

```rust
// In forked pallet-staking Config
pub trait Config: frame_system::Config {
    // ... standard config ...

    /// Handler for cMix-related operations
    type CmixHandler: CmixHandler;

    /// Handler for custody account identification
    type CustodyHandler: CustodyHandler<Self::AccountId, BalanceOf<Self>>;
}
```

### 6.2 CmixHandler Trait

**Location**: Defined in forked pallet-staking, implemented by xx-cmix

```rust
pub trait CmixHandler {
    /// Returns block production points from cMix variables
    fn get_block_points() -> u32;

    /// Called at era end to update cMix state
    fn end_era();
}
```

**Implementation** (`xx-cmix/src/lib.rs`):
```rust
impl<T: Config> pallet_staking::CmixHandler for Module<T> {
    fn get_block_points() -> u32 {
        Self::cmix_variables().performance.points.success
    }

    fn end_era() {
        if let Some(vars) = <NextCmixVariables<T>>::take() {
            <CmixVariables<T>>::put(vars);
        }
    }
}
```

**Usage in Staking**:
- `get_block_points()` called during reward calculation
- `end_era()` called in `on_finalize` at era boundaries

### 6.3 CustodyHandler Trait

**Location**: Defined in forked pallet-staking, implemented by xx-team-custody

```rust
pub trait CustodyHandler<AccountId, Balance> {
    /// Check if account is under custody
    fn is_custody_account(who: &AccountId) -> bool;

    /// Get total tokens under custody
    fn total_custody() -> Balance;
}
```

**Implementation** (`xx-team-custody/src/lib.rs`):
```rust
impl<T: Config> pallet_staking::CustodyHandler<T::AccountId, BalanceOf<T>>
    for Module<T>
{
    fn is_custody_account(who: &T::AccountId) -> bool {
        <CustodyAccounts<T>>::contains_key(who)
    }

    fn total_custody() -> BalanceOf<T> {
        <TotalCustody<T>>::get()
    }
}
```

**Usage in Staking**:
- `is_custody_account()` used during slashing decisions
- `total_custody()` used in inflation calculations

### 6.4 Other Custom Traits

**PublicAccountsHandler** (xx-public → xx-economics):
```rust
pub trait PublicAccountsHandler<AccountId> {
    fn accounts() -> Vec<AccountId>;
}
```

**RewardHandler** (xx-betanet-rewards → claims):
```rust
pub trait RewardHandler<AccountId, Balance> {
    fn add_claimed(who: &AccountId, amount: Balance);
}
```

---

## 7. Consensus and Block Production

### 7.1 Current Configuration

| Parameter | Value |
|-----------|-------|
| Block Time | 6 seconds |
| Slot Duration | 6000 ms |
| Epoch Duration | 4 hours (dev) / 8 hours (prod) |
| Sessions Per Era | 3 |
| Bonding Duration | 28 eras |
| Primary Probability | (1, 4) |

### 7.2 Consensus Pallets

- **BABE**: Block production (primary/secondary slots)
- **GRANDPA**: Block finalization
- **ImOnline**: Validator heartbeats
- **AuthorityDiscovery**: Peer discovery

### 7.3 Validator Selection

Uses `pallet-staking` with:
- NPoS (Nominated Proof of Stake)
- `pallet-election-provider-multi-phase` for elections
- `pallet-bags-list` for efficient voter management

---

## 8. Dependencies and Versioning

### 8.1 Substrate Dependencies

All Substrate crates from: `https://github.com/xx-labs/substrate`
Branch: `xx-network-v0.2.6`

**Key Categories**:

| Category | Examples |
|----------|----------|
| Primitives (sp-*) | sp-core, sp-runtime, sp-std, sp-io |
| Frame (frame-*) | frame-support, frame-system, frame-executive |
| Pallets (pallet-*) | pallet-staking, pallet-balances, pallet-democracy |
| Client (sc-*) | sc-service, sc-cli, sc-consensus-babe |

### 8.2 Third-Party Dependencies

```toml
parity-scale-codec = "3.0.0"
scale-info = "2.1.1"
libsecp256k1 = "0.7"  # For Ethereum signatures in claims
```

### 8.3 Build Configuration

```toml
[profile.production]
inherits = "release"
lto = "fat"
codegen-units = 1

[profile.dev.package]
# Crypto libraries optimized even in dev
blake2 = { opt-level = 3 }
curve25519-dalek = { opt-level = 3 }
# ... etc
```

### 8.4 Feature Flags

| Feature | Purpose |
|---------|---------|
| `std` | Standard library (default) |
| `runtime-benchmarks` | Enable benchmarking |
| `try-runtime` | Enable migration testing |
| `fast-runtime` | Accelerated block times for testing |

---

## Appendix: Key File Locations

### Runtime
- Main config: `runtime/xxnetwork/src/lib.rs`
- Shared config: `runtime/common/src/lib.rs`
- Weights: `runtime/xxnetwork/src/weights/`

### Custom Pallets
- xx-cmix: `xx-cmix/src/lib.rs`, `xx-cmix/src/cmix.rs`
- xx-economics: `xx-economics/src/lib.rs`, `xx-economics/src/inflation.rs`
- xx-team-custody: `xx-team-custody/src/lib.rs`, `xx-team-custody/src/custody.rs`
- xx-betanet-rewards: `xx-betanet-rewards/src/lib.rs`
- xx-public: `xx-public/src/lib.rs`
- chainbridge: `chainbridge/src/lib.rs`
- claims: `claims/src/lib.rs`
- swap: `swap/src/lib.rs`

### Node
- Service: `cli/src/service.rs`
- Chain spec: `cli/src/chain_spec.rs`
- Entry point: `cli/bin/main.rs`

### Scripts
- Benchmarking: `scripts/benchmark.sh`
