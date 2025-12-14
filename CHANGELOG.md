# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

### Runtime Version Mapping

| Runtime spec_version | Tags (this repo)                | Summary |
| -------------------- | ------------------------------- | ------- |
| 207                  | (unreleased)                    | Polkadot SDK `polkadot-stable2509-2` migration; see "[Unreleased] - Polkadot SDK Upgrade". |
| 206                  | v0.2.5-1, v0.2.5-2, v0.2.6      | Last legacy runtime on the `xx-labs/substrate` fork; baseline described in "[206] - Legacy Fork Baseline (xx-network-v0.2.6)". |
| 205                  | v0.2.5                          | Legacy fork runtime prior to 206; same cMix/custody staking architecture as 206 (see 206 notes and `XXCHANGES.md`). |
| 204                  | v0.2.4                          | Legacy fork runtime; incremental update within the 200-series on the xx-labs fork. |
| 203                  | v0.2.3, v0.2.3-1                | Legacy fork runtime; incremental update within the 200-series on the xx-labs fork. |
| 202                  | v0.2.2                          | Legacy fork runtime; incremental update within the 200-series on the xx-labs fork. |
| 201                  | v0.2.1                          | Legacy fork runtime; incremental update within the 200-series on the xx-labs fork. |
| 200                  | v0.2.0                          | First 2xx-series runtime on the legacy xx-labs fork. |
| 100                  | v0.1.1, v0.1.2                  | 1xx-series runtime on the legacy xx-labs fork; earliest runtime tracked in this repository. |

## [Unreleased] - Polkadot SDK Upgrade

This section documents **runtime 207** for the `xxnetwork` runtime (`spec_version = 207` in `runtime/xxnetwork/src/lib.rs`).

### Overview

Upgrade from forked `xx-labs/substrate` (v0.2.6, runtime spec_version 206) to upstream Polkadot SDK `polkadot-stable2509-2`.

### Breaking Changes

#### Custody Staking Discontinued

**⚠️ IMPORTANT**: Custody staking functionality has been removed in this upgrade.

The legacy xx-labs fork of `pallet-staking` included a `custody` field in the `Exposure` struct, which tracked stake from team custody accounts. This stake backed validators but did not earn rewards.

**What Changed:**
- The `custody` field is **permanently dropped** from all `ErasStakers` entries during migration
- Custody-specific reward adjustments in pallet-staking are removed
- The `xx-team-custody` pallet continues to function for vesting and account management

**Impact:**
- Custody accounts can no longer back validators with special reward treatment
- Any remaining custody stake backing validators will now earn rewards like normal stake
- This is an intentional simplification to use upstream SDK without modifications

#### Currency to Fungible Traits Migration

The Polkadot SDK has deprecated the `Currency` trait in favor of `fungible` traits. This affects how balances, holds, freezes, and transfers work.

**Key Changes:**

1. **`deposit_creating` behavior**: Returns a `PositiveImbalance` that increases `TotalIssuance` when dropped. However, `pallet_balances` genesis config **overwrites** `TotalIssuance` with `put()`, not adds to it.

2. **Staking uses holds instead of locks**: `pallet-staking` now uses `set_on_hold` (fungible holds) instead of freezes/locks. This affects `usable_balance`/`reducible_balance` calculations.

3. **Error message changes**: `LiquidityRestrictions` replaced with `InsufficientBalance` in some scenarios when funds are held/bonded.

4. **`ExistentialDeposit` must be > 0**: The new SDK requires a non-zero existential deposit.

#### Removed Canary Runtime and Betanet Rewards

- Removed the `runtime/canary` runtime; all networks now use the unified `xxnetwork` runtime selected via `--chain`.
- Removed the `xx-betanet-rewards` pallet and all associated runtime wiring and weight files from the runtime.
- Updated the `claims` pallet to use a `NoOpClaimsRewardHandler`, so Betanet-specific rewards are no longer minted during claims.
- Removed remaining dependencies on the legacy `xx-labs/substrate` fork; the runtime and node now depend directly on upstream `polkadot-sdk`.

### New Features

#### Smart Contracts Support (pallet-revive)

Added `pallet-revive` (pallet index 44) enabling smart contract functionality for the xx network:

**Supported Contract Types:**
- **Solidity contracts**: Compiled to PolkaVM/RISC-V bytecode via `revive` compiler
- **ink! contracts**: Native Substrate smart contracts using ink! framework

**Configuration:**
- **Chain ID**: 55 (matches xx network SS58 prefix)
- **Decimal ratio**: 10^9 (XX uses 9 decimals, ETH compatibility uses 18)
- **Runtime memory**: 128 MB per contract execution
- **Storage deposits**: Per-byte and per-item deposits for contract storage

**Security:**
Smart contracts in pallet-revive are sandboxed and cannot call runtime extrinsics directly. They can only:
- Interact with other deployed contracts
- Use precompiles (none configured initially)
- Transfer tokens via the contract's internal balance

**Runtime API (ReviveApi):**
Full ETH-RPC adapter support is implemented via `ReviveApi`:
- `eth_transact`: Execute Ethereum-style transactions (signed RLP-encoded txs)
- `call`: Dry-run contract calls for gas estimation
- `instantiate`: Deploy contracts with value and constructor arguments
- `upload_code`: Upload contract code for later instantiation
- `balance`, `nonce`, `gas_price`: EVM-compatible account/gas queries
- `get_storage`, `get_storage_var_key`: Contract storage inspection
- `trace_block`, `trace_tx`, `trace_call`: Full transaction tracing for debugging
- `address`, `block_author`, `code`: Additional helper methods

**Ethereum Transaction Extensions:**
Custom `EthExtraImpl` enables Ethereum transactions to use the full xx network transaction extension pipeline, including claims attestation prevalidation.

**Address Mapping:**
- `AccountId32Mapper` provides bidirectional mapping between Substrate AccountId32 and Ethereum H160 addresses
- Ethereum addresses are derived from the first 20 bytes of the AccountId32

**Usage:**
```rust
// Deploy a contract
Revive::instantiate(origin, value, gas_limit, storage_deposit_limit, code, data, salt)

// Call a contract
Revive::call(origin, dest, value, gas_limit, storage_deposit_limit, input_data)

// Upload contract code
Revive::upload_code(origin, code, storage_deposit_limit)
```

### Pallet Test Mock Updates

#### chainbridge (`chainbridge/src/mock.rs`)
- Added `dev_accounts: None` to `pallet_balances::GenesisConfig`
- Updated to use `#[derive_impl]` macro for config preludes

#### swap (`swap/src/lib.rs`, `swap/src/mock.rs`, `swap/src/tests.rs`)
- **Fixed `GenesisConfig` default**: Implemented manual `Default` with `threshold: 1` instead of `DefaultNoBound` which set threshold to 0, causing `InvalidThreshold` error
- Set `ExistentialDeposit = 1` (must be > 0 in new SDK)
- Updated tests to initialize accounts with existential deposit
- Added `dev_accounts: None` to balances genesis config

#### claims (`claims/src/mock.rs`)
- Updated to use `#[derive_impl]` macros
- Added `dev_accounts: None` to balances genesis config

#### xx-betanet-rewards (`xx-betanet-rewards/src/mock.rs`)
- Updated to use `#[derive_impl]` macros
- Added `dev_accounts: None` to balances genesis config
- Removed `CustodyHandler` trait (no longer exists in upstream pallet-staking)

#### xx-economics (`xx-economics/src/mock.rs`)
- Updated to use `#[derive_impl]` macros
- Added new staking config parameters for new SDK

#### xx-public (`xx-public/src/mock.rs`)
- Updated to use `#[derive_impl]` macros
- Added `dev_accounts: None` to balances genesis config

#### xx-cmix (`xx-cmix/src/mock.rs`, `xx-cmix/src/tests.rs`)
- Added explicit `<XXCmix as CmixHandler>::end_era()` call in `start_active_era` to trigger cmix variable updates at era boundaries
- Fixed points test: removed expected `+1` (new SDK doesn't add implicit point to validators)
- Updated deduction test: check `ErasDeductionPoints` storage instead of expecting `ErasRewardPoints` modification (deductions are now stored separately in xx-staking-extension)

#### xx-team-custody (`xx-team-custody/src/mock.rs`, `xx-team-custody/src/tests.rs`)

**Critical fix - Genesis config order:**
```rust
// IMPORTANT: pallet_balances must be assimilated BEFORE xx_team_custody!
// pallet_balances genesis OVERWRITES TotalIssuance with the sum of its balances.
// xx_team_custody uses deposit_creating which ADDS to TotalIssuance via Drop.
// If balances runs after custody, it will overwrite TotalIssuance to 0,
// causing Arithmetic(Underflow) errors during transfers.
```

**Other fixes:**
- Set `MaximumSchedulerWeight` with both `ref_time` AND `proof_size`: `Weight::from_parts(1_000_000_000_000_000, u64::MAX)`
- Set `type Preimages = Preimage` in Democracy config (was `()`)
- Updated error assertions: `LiquidityRestrictions` → `InsufficientBalance`
- Fixed `payout_call_after_staking_custody_coins` test to capture actual `usable_balance` after bonding rather than computing expected values

### Migration Notes

#### For Runtime Developers

1. **Genesis config order matters**: If your pallet uses `deposit_creating` in genesis build, ensure `pallet_balances` genesis is assimilated **before** your pallet.

2. **Test assertions may need updates**:
   - Check for `LiquidityRestrictions` → `InsufficientBalance` changes
   - Verify `usable_balance` behavior with staking holds

3. **Scheduler weight config**: Ensure `MaximumWeight` includes sufficient `proof_size`, not just `ref_time`.

4. **Staking behavior**: The new SDK uses holds for staking. `usable_balance` after bonding may be different than `free - bonded`.

#### For Node Operators

Storage migrations will be required for live chain upgrades. See `PLAN.md` for try-runtime testing procedures.

### Storage Migrations Required

#### pallet-staking (v12 → v16)

The upstream `pallet-staking` has progressed through several storage versions:
- **v14**: Paged staker exposures (`ErasStakersPaged`), `ClaimedRewards`
- **v15**: `DisablingStrategy`, `OffendingValidators` → `DisabledValidators`
- **v16**: `DisabledValidators` format change for offence severity

These migrations should run automatically via the pallet's built-in migration logic when storage versions are properly tracked.

#### CMIX ID API Changes

The forked `pallet-staking`'s `bond` function previously accepted an optional `cmix_id` parameter. With the upstream pallet-staking, users must now call separate extrinsics in `xx-staking-extension`:

```rust
// NEW API: xx-staking-extension pallet
XXStakingExtension::set_cmix_id(origin, cmix_id: [u8; 32])
XXStakingExtension::clear_cmix_id(origin)
```

**Workflow change:**
- **OLD**: `Staking::bond(controller, value, payee, cmix_id)`
- **NEW**: `Staking::bond(value, payee)` then `XXStakingExtension::set_cmix_id(cmix_id)`

The `set_cmix_id` extrinsic requires the caller to already be bonded (have a staking ledger).

#### CMIX ID Transfer Behavior Change

**Breaking Change**: The old forked runtime had a `transfer_cmix_id` extrinsic that atomically transferred a CMIX ID from one stash to another with election-time guards. This extrinsic **no longer exists** in the new `xx-staking-extension` pallet.

**Old behavior (`transfer_cmix_id`):**
```rust
// Atomic operation that:
// 1. Verified both source and target are bonded
// 2. Checked election timing (blocked during election)
// 3. Moved cmix_id from source to target in one transaction
Staking::transfer_cmix_id(origin, target_stash)
```

**New behavior (clear + set):**
```rust
// Two-step process:
// 1. Source clears their cmix_id
XXStakingExtension::clear_cmix_id(origin)

// 2. Target sets the cmix_id (can be any valid cmix_id)
XXStakingExtension::set_cmix_id(origin, cmix_id)
```

**Implications:**
- There is no atomic transfer - the CMIX ID is "free" between clear and set
- No election-time guard exists (can be added later if needed)
- Users must coordinate the clear/set operations off-chain
- If atomic transfer is required, a new `transfer_cmix_id` extrinsic can be added to `xx-staking-extension` post-upgrade

**Rationale:** The new design keeps `xx-staking-extension` simpler and more aligned with upstream Substrate patterns. Atomic transfer can be added as a feature enhancement if validator operations require it.

#### cMix ID Migration (Custom)

The forked `pallet-staking` embedded `cmix_id` in `StakingLedger`. The new architecture stores this in `xx-staking-extension::CmixIds`:

```rust
// OLD: Embedded in StakingLedger (forked pallet-staking)
pub struct StakingLedger<AccountId, Balance> {
    pub stash: AccountId,
    pub total: Balance,
    pub active: Balance,
    pub unlocking: Vec<UnlockChunk<Balance>>,
    pub claimed_rewards: Vec<EraIndex>,
    pub cmix_id: Option<[u8; 32]>,  // <-- Was here
}

// NEW: Separate storage in xx-staking-extension
#[pallet::storage]
pub type CmixIds<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, CmixId, OptionQuery>;
```

**Migration implementation (COMPLETED):**

The migration is implemented in `migrations/src/cmix_id_migration.rs` and now performs a full **layout rewrite** from the legacy v12xx staking types to the SDK v12 layout:

1. Defines legacy `OldStakingLedger` / `OldExposure` types with embedded `cmix_id` and custody fields, and new `NewStakingLedger` / `NewExposure` types that match upstream `pallet-staking` v12.
2. Iterates `Staking::Ledger`, moves any `cmix_id` into `xx-staking-extension::CmixIds`, and re-encodes every ledger using the new `NewStakingLedger` type.
3. Iterates `Staking::ErasStakers`, drops the custody field, and re-encodes exposures using the new `NewExposure` type.
4. Includes try-runtime hooks:
   - `pre_upgrade`: counts ledgers/exposures and records how many entries contain `cmix_id`.
   - `post_upgrade`: decodes all ledgers and exposures using the new types and verifies that `CmixIds` has the expected number of entries.

The migration is wired into the runtime at `runtime/xxnetwork/src/lib.rs` as the first step in the staking migration chain:
```rust
pub type Migrations = (
    CmixIdMigration<Runtime>,  // MUST run before pallet-staking's own migrations
    pallet_staking::migrations::v13::MigrateToV13<Runtime>,
    pallet_staking::migrations::v14::MigrateToV14<Runtime>,
    pallet_staking::migrations::v15::MigrateV14ToV15<Runtime>,
    pallet_staking::migrations::v16::MigrateV15ToV16<Runtime>,
    BridgeAdjust<Runtime>,
);
```

#### BridgeAdjust (Custom economic migration)

`BridgeAdjust<Runtime>` is a follow-up custom migration implemented in `migrations/src/bridge_adjust.rs` that:

- Moves a fixed amount of XX from the ChainBridge account into the staking rewards pool and liquidity rewards pool.
- Preserves the total stakeable supply (checked in `pre_upgrade` / `post_upgrade`).
- Is wired to run after all pallet-staking storage migrations have completed.

#### try-runtime Testing

The preferred approach for pre-upgrade validation is now to use **Chopsticks** to fork the live chain and execute the upgraded runtime WASM, instead of relying on the Rust `try-runtime-cli` (which has dependency conflicts with `polkadot-stable2509-2`).

This repository includes:

- `chopsticks.yml` – configuration for a forked xx network.
- `scripts/chopsticks.sh` – a small wrapper around the `chopsticks` binary that filters noisy `@polkadot` version warnings.

Typical workflow:

```bash
# 1. Build runtime with try-runtime feature enabled (produces WASM)
make build-try-runtime

# 2. Use chopsticks (via the wrapper) to run the upgraded runtime against forked state
./scripts/chopsticks.sh <chopsticks arguments>    # see chopsticks.yml for example config
```

If you prefer Rust-based tooling, you can still build `try-runtime-cli` from the matching `polkadot-sdk` tag and point it at the same WASM, but Chopsticks is the recommended path for this upgrade.

### References

- [FRAME: Move pallets over to use `fungible` traits · Issue #226](https://github.com/paritytech/polkadot-sdk/issues/226)
- [Polkadot v1.11.0 Release Notes](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-v1.11.0)
- [frame_tokens documentation](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_tokens/index.html)

### Remaining Deprecation Warnings

The following deprecation warnings are expected and can be addressed in future updates:

#### `RuntimeEvent::_w` Deprecation
All pallets show `RuntimeEvent::_w` deprecation warnings. This is internal to FRAME v2 and will be resolved automatically when the SDK team updates the macro. No action required.

### Completed Deprecation Migrations

#### ✅ `SignedExtension` to `TransactionExtension` (claims pallet)
- Updated `claims::PrevalidateAttests` to implement `TransactionExtension` trait
- Updated test code to use new `validate` signature with `TxBaseImplication`
- Re-added `PrevalidateAttests<Runtime>` to `SignedExtra` tuple in runtime

#### ✅ `CurrencyAdapter` to `FungibleAdapter` (runtime)
- Updated `DealWithFees` in `runtime/common/src/impls.rs` to use `fungible::Credit` instead of `NegativeImbalance`
- Changed fee splitting to use `Balanced::resolve` for depositing to treasury and author accounts
- Updated the `xxnetwork` runtime to use `FungibleAdapter<Balances, DealWithFees<Runtime>>` (the legacy canary runtime has been removed in this branch)

---

## [206] - Legacy Fork Baseline (xx-network-v0.2.6)

### Overview

Runtime **spec_version 206** was the last production runtime built directly on the legacy `xx-labs/substrate` fork (branch `xx-network-v0.2.6`, based on `polkadot-v0.9.41`).

From the perspective of this repository, runtime 206 serves as the **behavioral baseline** that runtime 207 migrates from. The changes below summarize how 206 differed from upstream Substrate (not from 207); see `XXCHANGES.md` for a detailed audit.

### Staking and cMix Integration

- Embedded **cMix IDs** directly into `pallet-staking`:
  - Added `cmix_id: Option<T::Hash>` to `StakingLedger`.
  - Modified `bond()` to accept and validate a `cmix_id` parameter.
  - Added extrinsics `set_cmix_id` and `transfer_cmix_id` with election-time guards.
- Introduced a **custody account system** inside staking:
  - Added `custody: Balance` field to `Exposure`.
  - Added a `CustodyHandler` trait for identifying custody accounts and their total stake.
  - Adjusted reward calculations to include custody stake in denominators while burning the custody share.
- Added **minimum validator commission** logic via `min_validator_commission` in staking genesis config.
- Integrated a `CmixHandler` trait so `pallet-staking` could query cMix block points and apply **deduction-based rewards** at era boundaries.

### Other Runtime-Level Modifications

- Simplified staking reward destinations by always paying rewards to the **stash** (removed `RewardDestination` / `Payee`).
- Added `ElectionActive` flag and related logic to prevent `transfer_cmix_id` during active elections.
- Introduced custom staking migrations (e.g., `MigrateFromV7dot5ToV10`) to roll together multiple upstream changes and xx-specific fields.

### Vesting and Governance

- Fixed **vesting genesis locks** to accumulate multiple schedules per account before setting the on-chain lock.
- Updated elections-phragmen to use **free balance** rather than total balance when computing voting stake.

### Infrastructure and Miscellaneous

- Carried a series of infrastructure changes (wasm-builder, wasmtime pin, call index stability) and cherry-picked upstream fixes for networking, contracts, benchmarks, and GRANDPA, as cataloged in `XXCHANGES.md`.

> Note: Earlier xx network runtimes prior to spec_version 206 lived entirely in the `xx-labs/substrate` repository. This changelog does not attempt a per-runtime breakdown before 206; instead, `XXCHANGES.md` serves as the authoritative historical record for those versions.

#### ✅ try-runtime CLI Integration
- Fixed `migrations` crate to properly compile with `try-runtime` feature
- Added `sp-runtime` dependency for `TryRuntimeError` type
- `make build-try-runtime` now completes successfully

### Test Results

All 162 pallet tests pass:

| Pallet | Tests |
|--------|-------|
| chainbridge | 16 |
| claims | 27 |
| swap | 13 |
| xx-betanet-rewards | 9 |
| xx-cmix | 12 |
| xx-economics | 23 |
| xx-public | 16 |
| xx-team-custody | 46 |
| **Total** | **162** |

### Follow-up Tasks

1. **Run try-runtime against live chain**: Test migrations against live chain state with:
   ```bash
   cargo build --release --features try-runtime
   ./target/release/xxnetwork-chain try-runtime \
       --runtime ./target/release/wbuild/xxnetwork-runtime/xxnetwork_runtime.wasm \
       on-runtime-upgrade \
       live --uri wss://rpc.xx.network:443
   ```

2. **Run benchmarks**: Generate actual weights for all pallets (replaces placeholder values):
   ```bash
   ./scripts/benchmark.sh
   ```
   Note: Full benchmark suite takes several hours. The script has been updated for the new SDK format.

3. **Deploy to testnet**: Test full upgrade cycle on testnet before mainnet deployment.

### Completed Implementation Tasks

- [x] Canary runtime removal (deprecated, chain selection now uses `--chain`)
- [x] Staking reward/slash handlers wired (RewardRemainder, Slash, Reward)
- [x] cmix_id migration fully implemented with try-runtime hooks
- [x] Benchmarking infrastructure updated for SDK polkadot-stable2509-2
- [x] All 162 pallet tests passing
- [x] CMIX ID behavior change documented
