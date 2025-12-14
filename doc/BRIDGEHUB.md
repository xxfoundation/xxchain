# xxnetwork Bridge Hub Integration Guide

This document provides detailed technical guidance for integrating xxnetwork with Polkadot Bridge Hub, enabling trustless cross-chain communication via native XCM.

**Last Updated**: 2025-12-14
**Status**: Phase 1 Implementation

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Prerequisites](#3-prerequisites)
4. [Pallet Integration](#4-pallet-integration)
5. [Runtime Configuration](#5-runtime-configuration)
6. [Relayer Setup](#6-relayer-setup)
7. [Testing Strategy](#7-testing-strategy)
8. [OpenGov Proposal Guide](#8-opengov-proposal-guide)
9. [Operational Considerations](#9-operational-considerations)
10. [References](#10-references)

---

## 1. Overview

### What This Document Covers

- Technical integration of xxnetwork with Polkadot Bridge Hub
- xxnetwork as a GRANDPA-finalized solochain connecting to Bridge Hub
- Bidirectional trustless messaging via native XCM

### Why Bridge Hub for xxnetwork

| Benefit | Description |
|---------|-------------|
| **Native XCM** | First-class Polkadot citizen status |
| **Trustless** | No external intermediaries, GRANDPA light client verification |
| **SDK Compatible** | Works with polkadot-stable2509-2 immediately |
| **Proven Path** | Kusama bridge provides reference ([Referendum #545](https://polkadot.subsquare.io/referenda/545)) |

### Current Status

| Component | Status |
|-----------|--------|
| Bridge pallet dependencies | Added to Cargo.toml |
| XCM stack dependencies | Pending |
| Runtime configuration | Pending |
| construct_runtime! | Pending (indices 45-48) |
| Weight files | Pending |
| OpenGov proposal | Pending |

---

## 2. Architecture

### High-Level Architecture

```
              +------------------------------------------+
              |              xxnetwork                   |
              |        (Sovereign Solochain)             |
              |                                          |
              |  +------------------------------------+  |
              |  | pallet-bridge-grandpa (BH headers) |  |
              |  | pallet-bridge-messages             |  |
              |  | pallet-xcm                         |  |
              |  | pallet-message-queue               |  |
              |  +------------------------------------+  |
              |                                          |
              |        BABE + GRANDPA (6s blocks)        |
              +------------------------------------------+
                                   |
                           GRANDPA Proofs
                           (via Relayers)
                                   |
                                   v
                          +----------------+
                          |  Bridge Hub    |
                          | (System Chain) |
                          |                |
                          | xxnetwork      |
                          | GRANDPA Client |
                          +-------+--------+
                                  |
                             Native XCM
                                  |
                    +-------------+-------------+
                    |             |             |
                    v             v             v
              +----------+  +----------+  +----------+
              | Polkadot |  |  Asset   |  |  Other   |
              |Parachains|  |   Hub    |  |Parachains|
              +----------+  +----------+  +----------+
```

### Message Flow: xxnetwork -> Polkadot Parachain

1. **User initiates**: XCM transfer on xxnetwork
2. **Message queued**: Stored in outbound message lane
3. **Finality**: GRANDPA finalizes the block
4. **Relayer picks up**: Constructs GRANDPA justification proof
5. **Relayer submits**: To Bridge Hub's `pallet-bridge-grandpa`
6. **Bridge Hub verifies**: xxnetwork header via light client
7. **Message dispatched**: Via native XCM to destination parachain

### Message Flow: Polkadot Parachain -> xxnetwork

1. **Parachain initiates**: XCM to xxnetwork via Bridge Hub
2. **Bridge Hub routes**: Message to xxnetwork lane
3. **Relayer picks up**: Constructs Bridge Hub proof
4. **Relayer submits**: To xxnetwork's `pallet-bridge-messages`
5. **xxnetwork verifies**: Bridge Hub header via light client
6. **Message executed**: Via XCM executor on xxnetwork

---

## 3. Prerequisites

### SDK Version Requirements

| Requirement | Value |
|-------------|-------|
| Polkadot SDK | `polkadot-stable2509-2` (v1.20.2) |
| Bridge pallets | Same version (workspace dependencies) |
| XCM version | v4 (latest) |

### Chain Requirements

| Requirement | xxnetwork Status |
|-------------|------------------|
| GRANDPA finality | Yes (native) |
| Block time | 6 seconds |
| Stable validator set | Yes |
| Public RPC endpoints | Yes (`wss://rpc.xx.network`) |

---

## 4. Pallet Integration

### 4.1 Workspace Dependencies

Add to `/Cargo.toml` workspace dependencies:

```toml
# XCM Stack
pallet-xcm = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
pallet-message-queue = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
staging-xcm = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
staging-xcm-builder = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
staging-xcm-executor = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }

# Additional bridge primitives
bp-polkadot-core = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
pallet-xcm-bridge-hub = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
bp-xcm-bridge-hub = { git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2509-2", default-features = false }
```

### 4.2 Runtime Dependencies

Add to `runtime/xxnetwork/Cargo.toml`:

```toml
[dependencies]
# XCM stack
pallet-xcm = { workspace = true }
pallet-message-queue = { workspace = true }
staging-xcm = { workspace = true }
staging-xcm-builder = { workspace = true }
staging-xcm-executor = { workspace = true }
bp-polkadot-core = { workspace = true }
pallet-xcm-bridge-hub = { workspace = true }
bp-xcm-bridge-hub = { workspace = true }
```

Update `std` feature:

```toml
std = [
    # ... existing ...
    "pallet-xcm/std",
    "pallet-message-queue/std",
    "staging-xcm/std",
    "staging-xcm-builder/std",
    "staging-xcm-executor/std",
    "bp-polkadot-core/std",
    "pallet-xcm-bridge-hub/std",
    "bp-xcm-bridge-hub/std",
]
```

### 4.3 Pallet Indices

| Index | Pallet | Purpose |
|-------|--------|---------|
| 45 | `MessageQueue` | Message queue processing |
| 46 | `PolkadotXcm` | Core XCM pallet |
| 47 | `BridgePolkadotGrandpa` | Bridge Hub GRANDPA light client |
| 48 | `BridgePolkadotMessages` | Message passing |
| 49 | Reserved | XcmBridgeHub (future) |

---

## 5. Runtime Configuration

### 5.1 XCM Config Module

Create `runtime/xxnetwork/src/xcm_config.rs`:

```rust
use super::*;
use frame_support::{
    parameter_types,
    traits::{Everything, Nothing},
};
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowTopLevelPaidExecutionFrom,
    FixedWeightBounds, FungibleAdapter, IsConcrete,
    ParentIsPreset, SignedAccountId32AsNative,
    SovereignSignedViaLocation, TakeWeightCredit,
    UsingComponents, WithComputedOrigin,
};
use xcm_executor::XcmExecutor;

parameter_types! {
    pub const XxnetworkLocation: Location = Location::here();
    pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Polkadot);
    pub UniversalLocation: InteriorLocation =
        [GlobalConsensus(RelayNetwork::get().unwrap())].into();
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

/// Type for specifying how a `Location` can be converted into an `AccountId`.
pub type LocationToAccountId = (
    ParentIsPreset<AccountId>,
    AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Asset transactor for native XX token
pub type LocalAssetTransactor = FungibleAdapter<
    Balances,
    IsConcrete<XxnetworkLocation>,
    LocationToAccountId,
    AccountId,
    (),
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = UsingComponents<IdentityFee<Balance>, XxnetworkLocation, AccountId, Balances, ()>;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    // ... additional config
}
```

### 5.2 Bridge Chain Types

Define in `lib.rs`:

```rust
/// Bridge Hub chain definition
pub struct PolkadotBridgeHubChain;

impl bp_runtime::Chain for PolkadotBridgeHubChain {
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hasher = sp_runtime::traits::BlakeTwo256;
    type Header = sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>;
    type AccountId = AccountId;
    type Balance = Balance;
    type Nonce = u32;
    type Signature = Signature;

    fn max_extrinsic_size() -> u32 { 4 * 1024 * 1024 }
    fn max_extrinsic_weight() -> Weight {
        Weight::from_parts(2_000_000_000_000, u64::MAX)
    }
}

impl bp_header_chain::ChainWithGrandpa for PolkadotBridgeHubChain {
    const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = "BridgePolkadotGrandpa";
    const MAX_AUTHORITIES_COUNT: u32 = 100_000;
    const REASONABLE_HEADERS_IN_JUSTIFICATION_ANCESTRY: u32 = 8;
    const MAX_MANDATORY_HEADER_SIZE: u32 = 256 * 1024;
    const AVERAGE_HEADER_SIZE: u32 = 512;
}
```

### 5.3 construct_runtime! Addition

```rust
construct_runtime!(
    pub enum Runtime
    {
        // ... existing pallets ...

        // XCM and Bridge Hub Integration (indices 45-48)
        MessageQueue: pallet_message_queue = 45,
        PolkadotXcm: pallet_xcm = 46,
        BridgePolkadotGrandpa: pallet_bridge_grandpa::<Instance1> = 47,
        BridgePolkadotMessages: pallet_bridge_messages::<Instance1> = 48,
    }
);
```

### 5.4 BaseFilter (Disabled Initially)

```rust
impl Contains<RuntimeCall> for BaseFilter {
    fn contains(call: &RuntimeCall) -> bool {
        match call {
            // ... existing entries ...

            // Bridge calls disabled until OpenGov approval
            RuntimeCall::PolkadotXcm(_) => false,
            RuntimeCall::BridgePolkadotGrandpa(_) => false,
            RuntimeCall::BridgePolkadotMessages(_) => false,

            // ... rest of match
        }
    }
}
```

---

## 6. Relayer Setup

### 6.1 Relayer Architecture

Bridge relayers are separate offchain processes that:
- Monitor xxnetwork for finalized blocks
- Construct GRANDPA justification proofs
- Submit proofs to Bridge Hub
- Monitor Bridge Hub for xxnetwork-bound messages
- Deliver messages back to xxnetwork

### 6.2 Relayer Binary

Use `substrate-relay` from [parity-bridges-common](https://github.com/paritytech/parity-bridges-common):

```bash
# Clone bridges repo
git clone https://github.com/paritytech/parity-bridges-common
cd parity-bridges-common

# Build relay binary
cargo build --release -p substrate-relay
```

### 6.3 Running Relayers

```bash
# GRANDPA finality relay (xxnetwork -> Bridge Hub)
./substrate-relay relay-headers xxnetwork-to-bridge-hub \
    --source-host rpc.xx.network \
    --source-port 443 \
    --target-host bridge-hub-polkadot-rpc.polkadot.io \
    --target-port 443 \
    --target-signer //RelayerKey

# Messages relay (bidirectional)
./substrate-relay relay-messages xxnetwork-bridge-hub \
    --source-host rpc.xx.network \
    --target-host bridge-hub-polkadot-rpc.polkadot.io \
    --lane 00000001 \
    --source-signer //RelayerKey \
    --target-signer //RelayerKey
```

### 6.4 Relayer Economics

| Cost | Estimate |
|------|----------|
| Server (VPS) | $50-200/month |
| Transaction fees (xxnetwork) | XX token gas |
| Transaction fees (Bridge Hub) | DOT gas |
| Recommended setup | 2+ relayers for redundancy |

---

## 7. Testing Strategy

### 7.1 Local Development

```bash
# Build with try-runtime
cargo build --release --features try-runtime

# Run dev node
./target/release/xxnetwork-chain --dev
```

### 7.2 Chopsticks Testing

Fork mainnet state for testing:

```bash
npx @acala-network/chopsticks \
    --config chopsticks/xxnetwork.yml \
    --port 8000
```

### 7.3 Testnet Integration

Test against Paseo (Polkadot testnet) Bridge Hub:
1. Deploy xxnetwork testnet with bridge pallets enabled
2. Configure relayers for testnet
3. Test bidirectional message flow

### 7.4 try-runtime Validation

```bash
try-runtime-cli \
    --runtime ./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm \
    on-runtime-upgrade \
    live \
    --uri wss://rpc.xx.network
```

---

## 8. OpenGov Proposal Guide

### 8.1 Overview

Adding xxnetwork to Polkadot Bridge Hub requires a Polkadot OpenGov referendum to:
1. Initialize xxnetwork GRANDPA light client on Bridge Hub
2. Configure XCM routing for xxnetwork

### 8.2 Precedent

**Reference**: [Polkadot Referendum #545](https://polkadot.subsquare.io/referenda/545) - Initialize Kusama bridge

The Kusama↔Polkadot bridge was established via:
- Referendum #545 (Polkadot): Initialize Kusama GRANDPA light client
- Referendum #354 (Kusama): Initialize Polkadot GRANDPA light client

### 8.3 Required Parameters

Collect before submitting proposal:

| Parameter | Description | How to Obtain |
|-----------|-------------|---------------|
| Initial Header | Recent finalized xxnetwork header | `chain_getFinalizedHead` RPC |
| Authority List | Current GRANDPA authorities | `grandpa_roundState` RPC |
| Set ID | Current authority set ID | `grandpa_roundState` RPC |

### 8.4 Proposal Template

```markdown
## Title
Initialize xxnetwork GRANDPA Light Client on Polkadot Bridge Hub

## Summary
Enable trustless communication between xxnetwork and Polkadot ecosystem
by initializing an on-chain xxnetwork GRANDPA light client on Bridge Hub.

## Background

### About xxnetwork
- Privacy-focused blockchain using cMix protocol
- Substrate-based with BABE/GRANDPA consensus (6-second blocks)
- Operating since mainnet launch
- Native token: XX

### Why Bridge Hub?
- Native XCM access to all Polkadot parachains
- Trustless bridging via GRANDPA finality verification
- Same architecture as proven Kusama bridge

## Technical Details

### Call
```
bridgeXxnetworkGrandpa.initialize({
    initial_header: <header>,
    authority_list: <authorities>,
    set_id: <set_id>,
    operating_mode: "Normal"
})
```

### Security
- xxnetwork uses standard GRANDPA finality
- Light client tracks authority set changes automatically
- Same security model as Kusama bridge

## Benefits
1. Native XCM connectivity for xxnetwork users
2. Trustless cross-chain asset transfers
3. Access to Polkadot DeFi ecosystem
4. Expands Polkadot ecosystem reach

## Timeline
- Technical testing: Complete
- Community discussion: [link]
- Post-approval: Relayer deployment within 1 week
```

### 8.5 Proposal Timeline

| Week | Activity |
|------|----------|
| 1-2 | Technical preparation, testnet validation |
| 3-4 | Draft proposal, community discussion (Polkadot Forum) |
| 5-6 | Polkassembly pre-proposal discussion |
| 7 | Fellowship review (if whitelisted track) |
| 8+ | Referendum voting period |

### 8.6 Track Selection

| Track | Requirements | Decision Time |
|-------|--------------|---------------|
| Root | Highest security | 28 days |
| Whitelisted Caller | Fellowship approval | 14 days |

**Recommendation**: Whitelisted Caller track with Fellowship review (similar to Kusama bridge).

### 8.7 Parameter Collection Script

```bash
#!/bin/bash
# Collect OpenGov proposal parameters

echo "Fetching finalized header..."
HEADER=$(curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' \
  https://rpc.xx.network | jq -r '.result')
echo "Finalized: $HEADER"

echo "Fetching header details..."
curl -s -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[\"$HEADER\"],\"id\":1}" \
  https://rpc.xx.network | jq '.result'

echo "Fetching GRANDPA authorities..."
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"grandpa_roundState","params":[],"id":1}' \
  https://rpc.xx.network | jq '.result'
```

---

## 9. Operational Considerations

### 9.1 Monitoring

Monitor these metrics:
- **Header lag**: Time since last xxnetwork header on Bridge Hub
- **Message queue depth**: Pending messages in lanes
- **Relayer health**: Uptime, transaction success rate
- **Finality delay**: Time between block production and finalization

### 9.2 Incident Response

| Scenario | Response |
|----------|----------|
| Relayer down | Backup relayer takes over, no message loss |
| Fork on xxnetwork | Governance intervention to re-initialize light client |
| Bridge Hub congestion | Messages queue, delivered when space available |

### 9.3 Upgrades

When upgrading xxnetwork runtime:
1. Coordinate with relayer operators
2. Ensure Bridge Hub light client handles authority set changes
3. Test on testnet before mainnet
4. No special action needed for standard upgrades

---

## 10. References

### Official Documentation

- [Bridge Hub Documentation](https://docs.polkadot.com/polkadot-protocol/architecture/system-chains/bridge-hub/)
- [pallet-bridge-grandpa](https://docs.rs/pallet-bridge-grandpa/latest/pallet_bridge_grandpa/)
- [Polkadot SDK Bridges Overview](https://github.com/paritytech/polkadot-sdk/blob/master/bridges/docs/high-level-overview.md)
- [XCM Documentation](https://docs.polkadot.com/develop/interoperability/)

### Precedent Proposals

- [Polkadot Referendum #545](https://polkadot.subsquare.io/referenda/545) - Kusama bridge initialization
- [Kusama Referendum #354](https://kusama.subsquare.io/referenda/354) - Polkadot bridge initialization

### Related xxnetwork Documentation

- [POLKADOT_INTEGRATION.md](./POLKADOT_INTEGRATION.md) - Integration strategy overview
- [PLAN.md](./PLAN.md) - SDK upgrade status
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Runtime architecture

### Repositories

- [Parity Bridges Common](https://github.com/paritytech/parity-bridges-common)
- [Polkadot SDK](https://github.com/paritytech/polkadot-sdk)
- [xxnetwork Chain](https://github.com/xx-labs/xxchain)
