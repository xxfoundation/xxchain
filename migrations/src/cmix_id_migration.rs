//! Migration from xx-labs fork format (v12xx) to SDK format (v12)
//!
//! ## Problem
//!
//! The forked `xx-labs/substrate` pallet-staking had two struct modifications:
//!
//! ### 1. StakingLedger - embedded cmix_id
//!
//! ```ignore
//! // v12xx (xx-labs fork) - has extra cmix_id field
//! pub struct StakingLedger<AccountId, Balance> {
//!     pub stash: AccountId,
//!     pub total: Balance,
//!     pub active: Balance,
//!     pub unlocking: BoundedVec<UnlockChunk<Balance>, MaxUnlockingChunks>,
//!     pub legacy_claimed_rewards: BoundedVec<EraIndex, ...>,
//!     pub cmix_id: Option<[u8; 32]>,  // <-- Added by xx-labs fork
//! }
//! ```
//!
//! ### 2. Exposure - embedded custody balance
//!
//! ```ignore
//! // v12xx (xx-labs fork) - has extra custody field
//! pub struct Exposure<AccountId, Balance> {
//!     pub total: Balance,
//!     pub own: Balance,
//!     pub others: Vec<IndividualExposure<AccountId, Balance>>,
//!     pub custody: Balance,  // <-- Added by xx-labs fork
//! }
//! ```
//!
//! The upstream SDK pallet-staking does NOT have these fields.
//!
//! ## Solution
//!
//! This migration performs v12xx → v12 conversion:
//! 1. Reads each ledger in old v12xx format, extracts cmix_id to CmixIds storage
//! 2. Rewrites the ledger in standard v12 format (without cmix_id)
//! 3. Reads each ErasStakers entry, rewrites without custody field
//!
//! ## IMPORTANT: Custody Staking Discontinued
//!
//! The `custody` field in `Exposure` was used to track stake from custody accounts
//! (team tokens that back validators but don't receive rewards). This feature is
//! **NO LONGER SUPPORTED** after this migration:
//!
//! - The `custody` balance is **permanently dropped** from all ErasStakers entries
//! - Custody-specific reward adjustments in pallet-staking are removed
//! - The `xx-team-custody` pallet continues to track custody accounts separately
//!   for other purposes (vesting, account management) but NOT for staking rewards
//!
//! After this migration, the standard SDK migrations v12→v13→v14→v15→v16 can run.

extern crate alloc;
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

use codec::{Decode, Encode};
use frame_support::{
    storage::storage_prefix,
    traits::{Get, OnRuntimeUpgrade},
    weights::Weight,
    BoundedVec,
};

use xx_staking_extension::CmixId;

/// UnlockChunk structure (same in both old and new formats)
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct UnlockChunk<Balance> {
    #[codec(compact)]
    pub value: Balance,
    #[codec(compact)]
    pub era: u32, // EraIndex
}

/// Old StakingLedger structure from the xx-labs fork (v12xx)
/// This MUST match the exact encoding of the old storage format
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct OldStakingLedger<AccountId, Balance, MaxUnlockingChunks: Get<u32>> {
    pub stash: AccountId,
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub active: Balance,
    pub unlocking: BoundedVec<UnlockChunk<Balance>, MaxUnlockingChunks>,
    pub legacy_claimed_rewards: BoundedVec<u32, frame_support::traits::ConstU32<84>>,
    pub cmix_id: Option<CmixId>,
}

/// New StakingLedger structure matching SDK v12 format (no cmix_id)
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct NewStakingLedger<AccountId, Balance, MaxUnlockingChunks: Get<u32>> {
    pub stash: AccountId,
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub active: Balance,
    pub unlocking: BoundedVec<UnlockChunk<Balance>, MaxUnlockingChunks>,
    pub legacy_claimed_rewards: BoundedVec<u32, frame_support::traits::ConstU32<84>>,
    // NO cmix_id field - this is the standard SDK format
}

/// IndividualExposure structure (same in both old and new formats)
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct IndividualExposure<AccountId, Balance> {
    pub who: AccountId,
    #[codec(compact)]
    pub value: Balance,
}

/// Old Exposure structure from the xx-labs fork (v12xx)
/// Has an extra `custody` field at the end
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct OldExposure<AccountId, Balance> {
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub own: Balance,
    pub others: alloc::vec::Vec<IndividualExposure<AccountId, Balance>>,
    #[codec(compact)]
    pub custody: Balance, // <-- Added by xx-labs fork, must be removed
}

/// New Exposure structure matching SDK v12 format (no custody)
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct NewExposure<AccountId, Balance> {
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub own: Balance,
    pub others: alloc::vec::Vec<IndividualExposure<AccountId, Balance>>,
    // NO custody field - this is the standard SDK format
}

/// Migration from v12xx (xx-labs fork) to v12 (SDK standard)
///
/// This migration:
/// 1. Extracts cmix_id from old ledgers and stores in xx_staking_extension::CmixIds
/// 2. Rewrites each ledger in standard SDK v12 format (without cmix_id)
/// 3. Rewrites each ErasStakers entry in standard SDK format (without custody)
///
/// After this migration completes, the SDK's standard migrations v12→v13→v14→v15→v16 can run.
pub struct CmixIdMigration<T>(core::marker::PhantomData<T>);

impl<T> OnRuntimeUpgrade for CmixIdMigration<T>
where
    T: frame_system::Config + pallet_staking::Config + xx_staking_extension::Config,
{
    fn on_runtime_upgrade() -> Weight {
        let mut reads = 0u64;
        let mut writes = 0u64;
        let mut cmix_ids_migrated = 0u32;
        let mut ledgers_rewritten = 0u32;
        let mut failed_decode_count = 0u32;

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Starting v12xx → v12 migration: extracting cmix_id and rewriting ledgers"
        );

        // Construct the storage prefix for Staking::Ledger
        // Storage key format: Twox128("Staking") ++ Twox128("Ledger") ++ Blake2_128Concat(controller)
        let ledger_prefix = storage_prefix(b"Staking", b"Ledger");

        // Iterate over all raw storage entries with this prefix using sp_io directly
        let mut current_key = ledger_prefix.to_vec();

        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            // Check if the key still has our prefix
            if !next_key.starts_with(&ledger_prefix) {
                break;
            }

            reads += 1;

            // Get the raw value for this key
            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                // Try to decode the old format with cmix_id
                match OldStakingLedger::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                    <T as pallet_staking::Config>::MaxUnlockingChunks,
                >::decode(&mut &raw_value[..])
                {
                    Ok(old_ledger) => {
                        // Step 1: Extract cmix_id if present
                        if let Some(cmix_id) = old_ledger.cmix_id.clone() {
                            xx_staking_extension::CmixIds::<T>::insert(&old_ledger.stash, cmix_id);
                            writes += 1;
                            cmix_ids_migrated += 1;

                            log::debug!(
                                target: "runtime::migrations::cmix_id",
                                "Extracted cmix_id for stash {:?}",
                                old_ledger.stash
                            );
                        }

                        // Step 2: Rewrite ledger in standard v12 format (without cmix_id)
                        let new_ledger = NewStakingLedger::<
                            T::AccountId,
                            pallet_staking::BalanceOf<T>,
                            <T as pallet_staking::Config>::MaxUnlockingChunks,
                        > {
                            stash: old_ledger.stash,
                            total: old_ledger.total,
                            active: old_ledger.active,
                            unlocking: old_ledger.unlocking,
                            legacy_claimed_rewards: old_ledger.legacy_claimed_rewards,
                        };

                        // Write the new format back to storage
                        sp_io::storage::set(&next_key, &new_ledger.encode());
                        writes += 1;
                        ledgers_rewritten += 1;

                        log::debug!(
                            target: "runtime::migrations::cmix_id",
                            "Rewritten ledger in v12 format for key {:?}",
                            next_key
                        );
                    }
                    Err(e) => {
                        // This could happen if:
                        // 1. The storage format has already been migrated to new format
                        // 2. The OldStakingLedger struct doesn't match the actual encoding
                        failed_decode_count += 1;
                        log::warn!(
                            target: "runtime::migrations::cmix_id",
                            "Failed to decode old ledger format: {:?}. This may be expected if migration already ran.",
                            e
                        );
                    }
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Ledger migration: {} cmix_ids extracted, {} ledgers rewritten, {} failed decodes",
            cmix_ids_migrated, ledgers_rewritten, failed_decode_count
        );

        // ========================================
        // Part 2: Migrate ErasStakers (remove custody field from Exposure)
        // ========================================
        let mut exposures_rewritten = 0u32;
        let mut exposure_failed_decode_count = 0u32;

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Starting ErasStakers migration: removing custody field from Exposure"
        );

        // Construct the storage prefix for Staking::ErasStakers
        // Storage key format: Twox128("Staking") ++ Twox128("ErasStakers") ++ keys
        let eras_stakers_prefix = storage_prefix(b"Staking", b"ErasStakers");
        let mut current_key = eras_stakers_prefix.to_vec();

        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            if !next_key.starts_with(&eras_stakers_prefix) {
                break;
            }

            reads += 1;

            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                // Try to decode the old Exposure format with custody
                match OldExposure::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                >::decode(&mut &raw_value[..])
                {
                    Ok(old_exposure) => {
                        // Rewrite Exposure in standard SDK format (without custody)
                        let new_exposure = NewExposure::<
                            T::AccountId,
                            pallet_staking::BalanceOf<T>,
                        > {
                            total: old_exposure.total,
                            own: old_exposure.own,
                            others: old_exposure.others,
                        };

                        sp_io::storage::set(&next_key, &new_exposure.encode());
                        writes += 1;
                        exposures_rewritten += 1;
                    }
                    Err(_) => {
                        // This could happen if:
                        // 1. The entry is already in new format
                        // 2. The struct doesn't match the actual encoding
                        exposure_failed_decode_count += 1;
                    }
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "ErasStakers migration: {} exposures rewritten, {} failed decodes",
            exposures_rewritten, exposure_failed_decode_count
        );

        log::info!(
            target: "runtime::migrations::cmix_id",
            "v12xx → v12 migration complete: total {} reads, {} writes",
            reads, writes
        );

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        log::info!(
            target: "runtime::migrations::cmix_id",
            "Pre-upgrade check for v12xx → v12 migration"
        );

        // Count existing CmixIds before migration (should be empty or from previous runs)
        let existing_cmix_ids = xx_staking_extension::CmixIds::<T>::iter().count() as u32;

        // Count how many ledgers have cmix_id set by reading old format
        let ledger_prefix = storage_prefix(b"Staking", b"Ledger");
        let mut ledgers_with_cmix_id = 0u32;
        let mut total_ledgers = 0u32;
        let mut ledgers_in_old_format = 0u32;

        let mut current_key = ledger_prefix.to_vec();
        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            if !next_key.starts_with(&ledger_prefix) {
                break;
            }

            total_ledgers += 1;
            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                if let Ok(old_ledger) = OldStakingLedger::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                    <T as pallet_staking::Config>::MaxUnlockingChunks,
                >::decode(&mut &raw_value[..])
                {
                    ledgers_in_old_format += 1;
                    if old_ledger.cmix_id.is_some() {
                        ledgers_with_cmix_id += 1;
                    }
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Pre-upgrade: {} total ledgers, {} in old format, {} with cmix_id, {} existing CmixIds",
            total_ledgers, ledgers_in_old_format, ledgers_with_cmix_id, existing_cmix_ids
        );

        // Count ErasStakers entries in old format (with custody)
        let eras_stakers_prefix = storage_prefix(b"Staking", b"ErasStakers");
        let mut total_exposures = 0u32;
        let mut exposures_in_old_format = 0u32;

        let mut current_key = eras_stakers_prefix.to_vec();
        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            if !next_key.starts_with(&eras_stakers_prefix) {
                break;
            }

            total_exposures += 1;
            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                if OldExposure::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                >::decode(&mut &raw_value[..]).is_ok()
                {
                    exposures_in_old_format += 1;
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Pre-upgrade: {} total ErasStakers entries, {} in old format (with custody)",
            total_exposures, exposures_in_old_format
        );

        // Encode state for post_upgrade verification
        // (ledgers_with_cmix_id, existing_cmix_ids, total_ledgers, ledgers_in_old_format, exposures_in_old_format)
        Ok((ledgers_with_cmix_id, existing_cmix_ids, total_ledgers, ledgers_in_old_format, exposures_in_old_format).encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        use frame_support::ensure;

        let (expected_cmix_ids, existing_before, _total_ledgers_before, ledgers_in_old_format, exposures_in_old_format): (u32, u32, u32, u32, u32) =
            Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");

        // Check 1: Verify cmix_ids were extracted
        let actual_cmix_ids = xx_staking_extension::CmixIds::<T>::iter().count() as u32;
        let expected_total_cmix_ids = expected_cmix_ids + existing_before;

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Post-upgrade: CmixIds check - expected {} ({} migrated + {} existing), found {}",
            expected_total_cmix_ids, expected_cmix_ids, existing_before, actual_cmix_ids
        );

        ensure!(
            actual_cmix_ids >= expected_total_cmix_ids,
            "CmixIds count mismatch after migration"
        );

        // Check 2: Verify ledgers are now in new format (can be decoded by NewStakingLedger)
        let ledger_prefix = storage_prefix(b"Staking", b"Ledger");
        let mut ledgers_in_new_format = 0u32;
        let mut ledgers_still_old = 0u32;

        let mut current_key = ledger_prefix.to_vec();
        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            if !next_key.starts_with(&ledger_prefix) {
                break;
            }

            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                // Try to decode as new format
                if NewStakingLedger::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                    <T as pallet_staking::Config>::MaxUnlockingChunks,
                >::decode(&mut &raw_value[..]).is_ok() {
                    ledgers_in_new_format += 1;
                } else {
                    ledgers_still_old += 1;
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Post-upgrade: Ledger format check - {} in new format, {} still in old format (of {} that were in old format before)",
            ledgers_in_new_format, ledgers_still_old, ledgers_in_old_format
        );

        // All ledgers that were in old format should now be in new format
        ensure!(
            ledgers_still_old == 0 || ledgers_in_new_format >= ledgers_in_old_format,
            "Some ledgers were not rewritten to new format"
        );

        // Check 3: Verify ErasStakers entries are now in new format (without custody)
        let eras_stakers_prefix = storage_prefix(b"Staking", b"ErasStakers");
        let mut exposures_in_new_format = 0u32;
        let mut exposures_still_old = 0u32;

        let mut current_key = eras_stakers_prefix.to_vec();
        while let Some(next_key) = sp_io::storage::next_key(&current_key) {
            if !next_key.starts_with(&eras_stakers_prefix) {
                break;
            }

            if let Some(raw_value) = sp_io::storage::get(&next_key) {
                // Try to decode as new format
                if NewExposure::<
                    T::AccountId,
                    pallet_staking::BalanceOf<T>,
                >::decode(&mut &raw_value[..]).is_ok()
                {
                    exposures_in_new_format += 1;
                } else {
                    exposures_still_old += 1;
                }
            }

            current_key = next_key;
        }

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Post-upgrade: ErasStakers check - {} in new format, {} still in old format (of {} in old format before)",
            exposures_in_new_format, exposures_still_old, exposures_in_old_format
        );

        ensure!(
            exposures_still_old == 0 || exposures_in_new_format >= exposures_in_old_format,
            "Some ErasStakers entries were not rewritten to new format"
        );

        log::info!(
            target: "runtime::migrations::cmix_id",
            "Post-upgrade: v12xx → v12 migration verification passed"
        );

        Ok(())
    }
}
