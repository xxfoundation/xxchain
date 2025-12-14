# TESTING.md

# xx Network Runtime 207 Upgrade Testing Guide

This document provides step-by-step instructions for testing the runtime 207 upgrade against forked mainnet state, and verifying that all xx network functionality is preserved before deploying to production.

**Last Updated**: 2025-12-14

**Runtime Version**: 207 (Polkadot SDK `polkadot-stable2509-2`)

## Table of Contents

1. [Quick Start: Runtime 207 Testing](#1-quick-start-runtime-207-testing)
2. [Detailed Testing Workflow](#2-detailed-testing-workflow)
   - [2.1 Prerequisites](#21-prerequisites)
   - [2.2 Step 1: Build the Runtime](#22-step-1-build-the-runtime)
   - [2.3 Step 2: Fork Mainnet State](#23-step-2-fork-mainnet-state)
   - [2.4 Step 3: Apply Runtime Upgrade](#24-step-3-apply-runtime-upgrade)
   - [2.5 Step 4: Validate Migrations](#25-step-4-validate-migrations)
   - [2.6 Step 5: Test Core Functionality](#26-step-5-test-core-functionality)
3. [Core Functionality Verification](#3-core-functionality-verification)
   - [3.1 Block Production & Finality](#31-block-production--finality)
   - [3.2 Staking Operations](#32-staking-operations)
   - [3.3 XX-Specific Pallet Verification](#33-xx-specific-pallet-verification)
   - [3.4 Balances & Transfers](#34-balances--transfers)
   - [3.5 Governance](#35-governance)
4. [Automated Verification Scripts](#4-automated-verification-scripts)
5. [Pre-Upgrade Checklist](#5-pre-upgrade-checklist)
6. [Troubleshooting](#6-troubleshooting)
7. [References](#7-references)

---

## 1. Quick Start: Runtime 207 Testing

**Goal**: Fork mainnet state, apply the new 207 runtime, verify migrations succeed, and confirm core functionality works.

```bash
# 1. Build the runtime with try-runtime feature
make build-try-runtime

# 2. Install Chopsticks (if not already installed)
npm install -g @acala-network/chopsticks

# 3. Run migration validation (checks migrations without interactive mode)
chopsticks try-runtime \
    --config=chopsticks.yml \
    --runtime=./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm \
    --checks=all

# 4. If migrations pass, start interactive forked chain with new runtime
chopsticks \
    --config=chopsticks.yml \
    --wasm-override=./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm

# 5. Connect Polkadot.js Apps to ws://127.0.0.1:9944 and verify functionality
```

---

## 2. Detailed Testing Workflow

### 2.1 Prerequisites

#### Required Software

```bash
# Node.js 18+ (for Chopsticks)
node --version  # Should be v18.0.0 or higher

# Install Chopsticks
npm install -g @acala-network/chopsticks

# Verify Chopsticks
chopsticks --version
```

#### Required Builds

| Build | Command | Purpose |
|-------|---------|---------|
| try-runtime WASM | `make build-try-runtime` | Migration validation |
| Release binary | `make build-release` | Local dev testing |
| Production binary | `make build-prod` | Final verification |

### 2.2 Step 1: Build the Runtime

```bash
# Build with try-runtime feature (required for migration hooks)
make build-try-runtime

# Verify the WASM was built
ls -la ./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm

# Expected output:
# -rw-r--r--  1 user  group  XXXXX  date xxnetwork_runtime.wasm
# Size should be approximately 2-4 MB
```

The WASM file at `./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm` contains the new runtime 207 code that will replace the on-chain runtime.

### 2.3 Step 2: Fork Mainnet State

Chopsticks creates a local chain with **real mainnet state** but allows modifications for testing.

#### Option A: Fork at Latest Block

```bash
# Fork current mainnet state (interactive mode)
chopsticks --config=chopsticks.yml

# Expected output:
# [timestamp] INFO: Connecting to wss://rpc.xx.network
# [timestamp] INFO: Forking from block #XXXXXXX
# [timestamp] INFO: Listening on ws://127.0.0.1:9944
```

#### Option B: Fork at Specific Block (Reproducible Testing)

```bash
# First, get a recent finalized block number
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getFinalizedHead"}' \
    https://rpc.xx.network | jq -r '.result'

# Fork at that specific block for reproducible testing
chopsticks --config=chopsticks.yml --block=15000000
```

#### Verify Fork is Working

```bash
# In a new terminal, check the forked chain
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
    http://127.0.0.1:9944 | jq

# Expected output:
# {
#   "jsonrpc": "2.0",
#   "result": {
#     "peers": 0,
#     "isSyncing": false,
#     "shouldHavePeers": false
#   },
#   "id": 1
# }
```

### 2.4 Step 3: Apply Runtime Upgrade

There are two ways to test the runtime upgrade:

#### Method 1: Migration Validation Only (Recommended First Step)

This runs the migration pre/post checks without starting an interactive chain:

```bash
chopsticks try-runtime \
    --config=chopsticks.yml \
    --runtime=./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm \
    --checks=all

# Expected output (success):
# [timestamp] INFO: Running pre-upgrade hooks...
# [timestamp] INFO: CmixIdMigration: Found XXX ledgers with cmix_id
# [timestamp] INFO: CmixIdMigration: Found XXX exposures to migrate
# [timestamp] INFO: BridgeAdjust: Bridge balance = XXXXX, Rewards = XXXXX
# [timestamp] INFO: Executing on_runtime_upgrade...
# [timestamp] INFO: Running post-upgrade hooks...
# [timestamp] INFO: CmixIdMigration: Migrated XXX cmix_ids successfully
# [timestamp] INFO: BridgeAdjust: Transfer complete
# [timestamp] INFO: ✅ All checks passed
```

#### Method 2: Interactive Testing with New Runtime

After migrations pass, start the forked chain with the new runtime for interactive testing:

```bash
chopsticks \
    --config=chopsticks.yml \
    --wasm-override=./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm

# This replaces the on-chain runtime (206) with the new runtime (207)
# while preserving all mainnet state
```

#### Verify Runtime Version Changed

```bash
# Check runtime version
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' \
    http://127.0.0.1:9944 | jq '.result.specVersion'

# Expected output:
# 207
```

### 2.5 Step 4: Validate Migrations

The runtime 207 upgrade includes these migrations:

| Migration | Purpose | Verification |
|-----------|---------|--------------|
| **CmixIdMigration** | Extract cmix_id from StakingLedger to xx-staking-extension | Query `xxStakingExtension.cmixIds` |
| **BridgeAdjust** | Transfer 40M from bridge to rewards pool | Check `xxEconomics.liquidityRewards` |
| **Staking v13-v16** | SDK staking storage updates | Query `staking.ledger` decodes correctly |

#### Verify CmixIdMigration

```bash
# Query a known validator's cmix_id after migration
# Replace VALIDATOR_ADDRESS with an actual validator stash address

curl -s -H "Content-Type: application/json" \
    -d '{
        "id":1,
        "jsonrpc":"2.0",
        "method": "state_call",
        "params": ["Metadata_metadata", ""]
    }' \
    http://127.0.0.1:9944 | jq

# Using Polkadot.js Apps Developer > Chain State:
# xxStakingExtension > cmixIds(AccountId): should return CmixId for validators
```

#### Verify BridgeAdjust

```bash
# Check liquidity rewards balance
# Use Polkadot.js Apps Developer > Chain State:
# xxEconomics > liquidityRewards(): should be ~10,000,000 XX (10M)
```

### 2.6 Step 5: Test Core Functionality

With the forked chain running with the new runtime, test core operations.

#### Produce New Blocks

```bash
# Advance one block
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "dev_newBlock", "params": [{}]}' \
    http://127.0.0.1:9944 | jq

# Expected output:
# { "jsonrpc": "2.0", "result": "0x...", "id": 1 }
# (returns the new block hash)

# Advance 10 blocks
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "dev_newBlock", "params": [{"count": 10}]}' \
    http://127.0.0.1:9944 | jq
```

#### Submit Test Transaction

With `mock-signature-host: true`, you can submit transactions with any account:

```javascript
// In Polkadot.js Apps > Developer > Extrinsics
// 1. Select any account (signatures are mocked)
// 2. Submit: balances.transferKeepAlive(dest, amount)
// 3. Verify transaction succeeds
```

---

## 3. Core Functionality Verification

After applying the runtime upgrade, systematically verify each component.

### 3.1 Block Production & Finality

| Test | Command/Query | Expected Result |
|------|---------------|-----------------|
| New blocks produce | `dev_newBlock` | Block hash returned |
| Block number increments | `chain_getHeader` | Number increases |
| Block has valid parent | `chain_getHeader` | `parentHash` matches previous |
| Transactions included | Submit extrinsic | Included in block |

**Verification Script:**

```bash
#!/bin/bash
# scripts/verify-blocks.sh

RPC="http://127.0.0.1:9944"

echo "=== Block Production Test ==="

# Get initial block
BEFORE=$(curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' \
    $RPC | jq -r '.result.number' | xargs printf "%d")

echo "Current block: $BEFORE"

# Produce new block
curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "dev_newBlock", "params": [{}]}' \
    $RPC > /dev/null

# Get new block
AFTER=$(curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' \
    $RPC | jq -r '.result.number' | xargs printf "%d")

echo "New block: $AFTER"

if [ $AFTER -gt $BEFORE ]; then
    echo "✅ Block production: PASSED"
else
    echo "❌ Block production: FAILED"
    exit 1
fi
```

### 3.2 Staking Operations

Critical staking functionality that must be verified:

| Test | Storage/Extrinsic | Expected |
|------|-------------------|----------|
| Active era exists | `staking.activeEra` | Returns `{index, start}` |
| Validators listed | `staking.validators(addr)` | Returns `{commission, blocked}` |
| Ledgers decode | `staking.ledger(ctrl)` | Returns valid ledger |
| Era reward points | `staking.erasRewardPoints(era)` | Returns `{total, individual}` |
| Nominations work | `staking.nominators(addr)` | Returns nomination targets |

**Query Active Era:**

```bash
curl -s -H "Content-Type: application/json" \
    -d '{
        "id":1,
        "jsonrpc":"2.0",
        "method": "state_getStorage",
        "params": ["0x5f3e4907f716ac89b6347d15ececedca487df464e44a534ba6b0cbb32407b587"]
    }' \
    http://127.0.0.1:9944 | jq '.result'

# Returns encoded active era info (decode with Polkadot.js)
```

**Polkadot.js Verification:**

```javascript
// Developer > Chain State
// staking > activeEra(): Option<ActiveEraInfo>
// Expected: { index: XXX, start: Some(timestamp) }

// staking > currentEra(): Option<EraIndex>
// Expected: XXX (same as activeEra.index)

// staking > erasTotalStake(eraIndex): Balance
// Expected: Large balance (total staked in era)
```

### 3.3 XX-Specific Pallet Verification

These are the **critical** xx network-specific functions that must work after upgrade:

#### 3.3.1 xx-staking-extension

| Test | Query | Expected |
|------|-------|----------|
| CmixIds migrated | `xxStakingExtension.cmixIds(validatorStash)` | Returns 32-byte CmixId for validators that had one |
| Deduction points | `xxStakingExtension.erasDeductionPoints(era)` | Returns points map (may be empty) |

**Verification:**

```javascript
// Developer > Chain State > xxStakingExtension

// 1. Query a known validator stash that should have a cmix_id
// xxStakingExtension.cmixIds(AccountId32): Option<[u8; 32]>

// 2. If validator had cmix_id before upgrade, it should now appear here
// If returns None for a validator that had cmix_id, migration FAILED
```

#### 3.3.2 xx-cmix

| Test | Query | Expected |
|------|-------|----------|
| cMix hashes exist | `xxCmix.cmixHashes()` | Returns hash values |
| Scheduling account | `xxCmix.schedulingAccount()` | Returns AccountId |
| Variables set | `xxCmix.cmixVariables()` | Returns CmixVariables struct |
| Address space | `xxCmix.cmixAddressSpace()` | Returns u8 value |

**Verification:**

```javascript
// Developer > Chain State > xxCmix

// xxCmix.cmixHashes(): CmixHashes
// Expected: { cmix: 0x..., gateway: 0x..., scheduling: 0x... }

// xxCmix.schedulingAccount(): Option<AccountId>
// Expected: Some(AccountId) - the scheduling server account

// xxCmix.cmixVariables(): CmixVariables
// Expected: { performance: {...}, timeouts: {...}, ... }
```

#### 3.3.3 xx-economics

| Test | Query | Expected |
|------|-------|----------|
| Inflation params | `xxEconomics.inflationParams()` | Returns params struct |
| Interest points | `xxEconomics.interestPoints()` | Returns BTreeMap |
| Liquidity rewards | `xxEconomics.liquidityRewards()` | ~10M XX after BridgeAdjust |
| Ideal stake | `xxEconomics.idealLiquidityStake()` | Returns target balance |

**Verification:**

```javascript
// Developer > Chain State > xxEconomics

// xxEconomics.liquidityRewards(): Balance
// Expected: ~10,000,000,000,000,000 (10M XX with 9 decimals)
// This confirms BridgeAdjust migration succeeded

// xxEconomics.inflationParams(): InflationFixedParams
// Expected: { minInflation: X, idealStake: Y, falloff: Z }
```

#### 3.3.4 xx-team-custody

| Test | Query | Expected |
|------|-------|----------|
| Team accounts | `xxCustody.teamAccounts(addr)` | Returns CustodyInfo |
| Custodians | `xxCustody.custodians()` | Returns Vec<AccountId> |
| Total custody | `xxCustody.totalCustody()` | Returns Balance |

**Verification:**

```javascript
// Developer > Chain State > xxCustody

// xxCustody.totalCustody(): Balance
// Expected: Large balance (total under custody)

// xxCustody.custodians(): Vec<AccountId>
// Expected: List of custodian accounts
```

### 3.4 Balances & Transfers

| Test | Method | Expected |
|------|--------|----------|
| Query balance | `system.account(addr)` | Returns AccountInfo with balance |
| Transfer works | `balances.transferKeepAlive` | Success, balance changes |
| ED enforced | Transfer entire balance | Keeps existential deposit |
| Total issuance | `balances.totalIssuance()` | Same as before upgrade |

**Verify Total Issuance Unchanged:**

```javascript
// Developer > Chain State > balances

// balances.totalIssuance(): Balance
// Compare to mainnet value - should be identical
// (migrations should not change total issuance)
```

### 3.5 Governance

| Test | Query | Expected |
|------|-------|----------|
| Democracy state | `democracy.publicProps()` | Returns proposals array |
| Referenda | `democracy.referendumCount()` | Returns count |
| Council | `council.members()` | Returns member list |
| Tech committee | `technicalCommittee.members()` | Returns member list |

---

## 4. Automated Verification Scripts

### Full Verification Script

Create `scripts/verify-upgrade.sh`:

```bash
#!/bin/bash
# Comprehensive runtime 207 upgrade verification

set -e

RPC="${1:-http://127.0.0.1:9944}"
EXPECTED_SPEC_VERSION=207

echo "============================================"
echo "xx Network Runtime 207 Upgrade Verification"
echo "RPC Endpoint: $RPC"
echo "============================================"
echo ""

# Helper function
query_rpc() {
    curl -s -H "Content-Type: application/json" \
        -d "$1" \
        $RPC
}

# 1. Check Runtime Version
echo "1. Checking Runtime Version..."
SPEC_VERSION=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' | jq -r '.result.specVersion')

if [ "$SPEC_VERSION" = "$EXPECTED_SPEC_VERSION" ]; then
    echo "   ✅ Runtime version: $SPEC_VERSION"
else
    echo "   ❌ Runtime version: $SPEC_VERSION (expected $EXPECTED_SPEC_VERSION)"
    exit 1
fi

# 2. Check System Health
echo "2. Checking System Health..."
HEALTH=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "system_health"}')
IS_SYNCING=$(echo $HEALTH | jq -r '.result.isSyncing')

if [ "$IS_SYNCING" = "false" ]; then
    echo "   ✅ Node is synced"
else
    echo "   ⚠️  Node is syncing"
fi

# 3. Check Block Production
echo "3. Checking Block Production..."
BEFORE_BLOCK=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' | jq -r '.result.number' | xargs printf "%d")
query_rpc '{"id":1, "jsonrpc":"2.0", "method": "dev_newBlock", "params": [{}]}' > /dev/null 2>&1 || true
AFTER_BLOCK=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' | jq -r '.result.number' | xargs printf "%d")

if [ "$AFTER_BLOCK" -gt "$BEFORE_BLOCK" ]; then
    echo "   ✅ Block production works ($BEFORE_BLOCK -> $AFTER_BLOCK)"
else
    echo "   ⚠️  Block production unchanged (may be in non-dev mode)"
fi

# 4. Check Active Era (Staking)
echo "4. Checking Staking State..."
# Use state_call to get metadata and check staking is queryable
CHAIN_NAME=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' | jq -r '.result')
echo "   ✅ Chain: $CHAIN_NAME"

# 5. Check Total Issuance
echo "5. Checking Balances..."
# Storage key for balances.totalIssuance
TOTAL_ISSUANCE=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "state_getStorage", "params": ["0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"]}' | jq -r '.result')
if [ "$TOTAL_ISSUANCE" != "null" ] && [ -n "$TOTAL_ISSUANCE" ]; then
    echo "   ✅ Total issuance exists: ${TOTAL_ISSUANCE:0:20}..."
else
    echo "   ❌ Total issuance not found"
    exit 1
fi

# 6. Summary
echo ""
echo "============================================"
echo "Verification Complete"
echo "============================================"
echo ""
echo "Next steps:"
echo "1. Open Polkadot.js Apps at: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944"
echo "2. Verify xx-specific pallets in Developer > Chain State"
echo "3. Test extrinsic submission (signatures are mocked)"
echo "4. Check migration results for CmixIdMigration and BridgeAdjust"
```

### Run Verification

```bash
chmod +x scripts/verify-upgrade.sh
./scripts/verify-upgrade.sh http://127.0.0.1:9944
```

---

## 5. Pre-Upgrade Checklist

Complete all items before proposing the upgrade on mainnet.

### Phase 1: Build Verification

| # | Check | Command | Status |
|---|-------|---------|--------|
| 1.1 | [ ] All 162 pallet tests pass | `make test-pallets` | |
| 1.2 | [ ] Production build compiles | `make build-prod` | |
| 1.3 | [ ] try-runtime build compiles | `make build-try-runtime` | |
| 1.4 | [ ] WASM size is reasonable (<5MB) | `ls -la target/release/wbuild/*/xxnetwork_runtime.wasm` | |

### Phase 2: Migration Validation

| # | Check | Command | Status |
|---|-------|---------|--------|
| 2.1 | [ ] try-runtime passes with `--checks=all` | `chopsticks try-runtime --config=chopsticks.yml --runtime=<wasm> --checks=all` | |
| 2.2 | [ ] CmixIdMigration pre_upgrade logs count | Check logs | |
| 2.3 | [ ] CmixIdMigration post_upgrade confirms migration | Check logs | |
| 2.4 | [ ] BridgeAdjust completes successfully | Check logs | |
| 2.5 | [ ] No errors in migration output | Review full output | |

### Phase 3: Functional Verification (on Chopsticks fork)

| # | Check | Method | Status |
|---|-------|--------|--------|
| 3.1 | [ ] Runtime version is 207 | `state_getRuntimeVersion` | |
| 3.2 | [ ] Blocks can be produced | `dev_newBlock` | |
| 3.3 | [ ] Staking state is intact | Query `staking.activeEra` | |
| 3.4 | [ ] CmixIds storage populated | Query `xxStakingExtension.cmixIds` | |
| 3.5 | [ ] Liquidity rewards ~10M | Query `xxEconomics.liquidityRewards` | |
| 3.6 | [ ] cMix state intact | Query `xxCmix.*` | |
| 3.7 | [ ] Custody state intact | Query `xxCustody.*` | |
| 3.8 | [ ] Total issuance unchanged | Query `balances.totalIssuance` | |
| 3.9 | [ ] Transfers work | Submit extrinsic | |
| 3.10 | [ ] Governance state intact | Query `democracy.*`, `council.*` | |

### Phase 4: Documentation

| # | Check | Status |
|---|-------|--------|
| 4.1 | [ ] CHANGELOG.md updated for 207 | |
| 4.2 | [ ] Breaking changes documented | |
| 4.3 | [ ] Migration notes complete | |

---

## 6. Troubleshooting

### Chopsticks Won't Connect

```
Error: Failed to connect to wss://rpc.xx.network
```

**Solutions:**
1. Check internet connectivity
2. Try alternative RPC: `--endpoint=wss://rpc2.xx.network`
3. Check if RPC endpoint requires authentication

### Migration Fails with Decode Error

```
Error: Failed to decode storage: xxStakingExtension.CmixIds
```

**Solutions:**
1. Ensure WASM was built with correct features: `make build-try-runtime`
2. Check that you're using the correct WASM path
3. Verify the migration order in runtime (CmixIdMigration must run first)

### Runtime Version Not Updated

```
spec_version shows 206 instead of 207
```

**Solutions:**
1. Ensure you're using `--wasm-override` not just `--config`
2. Verify WASM file path is correct
3. Check that WASM file was rebuilt after code changes

### Block Production Fails

```
Error: Unable to produce block
```

**Solutions:**
1. This may indicate a migration failed - check logs
2. Try with `--runtime-log-level=4` for debug output
3. Run `try-runtime` first to validate migrations

### "Pre-upgrade hook failed"

```
Error: CmixIdMigration: pre_upgrade failed
```

**Solutions:**
1. Check that mainnet storage format matches expected legacy format
2. Review migration code for compatibility
3. May need to update legacy type definitions in migration

---

## 7. References

### Project Documentation

- [CHANGELOG.md](./CHANGELOG.md) - Detailed changes in runtime 207
- [ARCHITECTURE.md](./doc/ARCHITECTURE.md) - System architecture
- [chopsticks.yml](./chopsticks.yml) - Chopsticks configuration

### External Tools

- [Chopsticks GitHub](https://github.com/AcalaNetwork/chopsticks)
- [Polkadot.js Apps](https://polkadot.js.org/apps/)
- [try-runtime CLI](https://paritytech.github.io/try-runtime-cli/)

### xx Network

- **Mainnet RPC**: `wss://rpc.xx.network`
- **Block Explorer**: https://explorer.xx.network

---

## Appendix A: Key Storage Queries

Quick reference for important storage queries in Polkadot.js Apps (Developer > Chain State):

| Pallet | Storage | Description |
|--------|---------|-------------|
| `system` | `account(AccountId)` | Account info with balance |
| `balances` | `totalIssuance()` | Total token supply |
| `staking` | `activeEra()` | Current active era |
| `staking` | `currentEra()` | Current era index |
| `staking` | `ledger(AccountId)` | Staking ledger for controller |
| `staking` | `validators(AccountId)` | Validator preferences |
| `staking` | `nominators(AccountId)` | Nominator targets |
| `xxStakingExtension` | `cmixIds(AccountId)` | CmixId for stash |
| `xxStakingExtension` | `erasDeductionPoints(EraIndex)` | Deduction points |
| `xxCmix` | `cmixHashes()` | Software hashes |
| `xxCmix` | `schedulingAccount()` | Scheduling server |
| `xxCmix` | `cmixVariables()` | Network variables |
| `xxEconomics` | `liquidityRewards()` | Rewards pool balance |
| `xxEconomics` | `inflationParams()` | Inflation parameters |
| `xxCustody` | `totalCustody()` | Total under custody |
| `xxCustody` | `custodians()` | Custodian accounts |
| `democracy` | `publicProps()` | Active proposals |
| `council` | `members()` | Council members |

---

## Appendix B: Expected Migration Logs

When running `chopsticks try-runtime`, expect logs similar to:

```
[INFO] Connecting to wss://rpc.xx.network
[INFO] Forked at block #XXXXXXX (hash: 0x...)
[INFO] Running pre-upgrade hooks...

[INFO] CmixIdMigration::pre_upgrade
[INFO]   Ledgers with cmix_id: 150
[INFO]   ErasStakers entries to migrate: 450
[INFO]   Pre-upgrade state saved

[INFO] BridgeAdjust::pre_upgrade
[INFO]   ChainBridge balance: 40,000,000 XX
[INFO]   Current rewards pool: 5,000,000 XX
[INFO]   Pre-upgrade state saved

[INFO] Executing on_runtime_upgrade...
[INFO]   CmixIdMigration: Migrating ledgers...
[INFO]   CmixIdMigration: Migrating exposures...
[INFO]   BridgeAdjust: Transferring 40M to rewards pool...
[INFO]   pallet_staking v13: Running...
[INFO]   pallet_staking v14: Running...
[INFO]   pallet_staking v15: Running...
[INFO]   pallet_staking v16: Running...

[INFO] Running post-upgrade hooks...

[INFO] CmixIdMigration::post_upgrade
[INFO]   CmixIds storage entries: 150 ✓
[INFO]   Ledgers decode correctly ✓
[INFO]   Exposures decode correctly ✓

[INFO] BridgeAdjust::post_upgrade
[INFO]   Rewards pool: 45,000,000 XX ✓
[INFO]   Liquidity rewards: 10,000,000 XX ✓

[INFO] ✅ All migrations completed successfully
[INFO] ✅ All post-upgrade checks passed
```

If you see errors instead of checkmarks, the specific migration that failed will be indicated in the logs.
