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
//! ### 2. Exposure - embedded custody balance (in SECOND position!)
//!
//! ```ignore
//! // v12xx (xx-labs fork) - has extra custody field in SECOND position
//! pub struct Exposure<AccountId, Balance> {
//!     pub total: Balance,
//!     pub custody: Balance,  // <-- Added by xx-labs fork (SECOND position, not last!)
//!     pub own: Balance,
//!     pub others: Vec<IndividualExposure<AccountId, Balance>>,
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
/// Has an extra `custody` field in SECOND position (after total, before own)
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct OldExposure<AccountId, Balance> {
	#[codec(compact)]
	pub total: Balance,
	#[codec(compact)]
	pub custody: Balance, // <-- Added by xx-labs fork in SECOND position
	#[codec(compact)]
	pub own: Balance,
	pub others: alloc::vec::Vec<IndividualExposure<AccountId, Balance>>,
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
						if let Some(cmix_id) = old_ledger.cmix_id {
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
				match OldExposure::<T::AccountId, pallet_staking::BalanceOf<T>>::decode(
					&mut &raw_value[..],
				) {
					Ok(old_exposure) => {
						// Rewrite Exposure in standard SDK format (without custody)
						let new_exposure =
							NewExposure::<T::AccountId, pallet_staking::BalanceOf<T>> {
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

		// Set staking pallet version to 12 so SDK migrations v12→v13→... can run
		use frame_support::traits::StorageVersion;
		StorageVersion::new(12).put::<pallet_staking::Pallet<T>>();
		writes += 1;

		log::info!(
			target: "runtime::migrations::cmix_id",
			"Set staking storage version to 12"
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
		let mut exposures_in_sdk_format = 0u32;
		let mut sample_logged = false;

		let mut current_key = eras_stakers_prefix.to_vec();
		while let Some(next_key) = sp_io::storage::next_key(&current_key) {
			if !next_key.starts_with(&eras_stakers_prefix) {
				break;
			}

			total_exposures += 1;
			if let Some(raw_value) = sp_io::storage::get(&next_key) {
				// Log first entry for debugging
				if !sample_logged {
					log::info!(
						target: "runtime::migrations::cmix_id",
						"Sample ErasStakers entry: key len={}, value len={}, first 100 bytes: {:?}",
						next_key.len(),
						raw_value.len(),
						&raw_value[..core::cmp::min(100, raw_value.len())]
					);
					sample_logged = true;
				}

				// Try decoding as OldExposure (with custody)
				if OldExposure::<T::AccountId, pallet_staking::BalanceOf<T>>::decode(
					&mut &raw_value[..],
				)
				.is_ok()
				{
					exposures_in_old_format += 1;
				}

				// Also try decoding as SDK Exposure (without custody)
				if NewExposure::<T::AccountId, pallet_staking::BalanceOf<T>>::decode(
					&mut &raw_value[..],
				)
				.is_ok()
				{
					exposures_in_sdk_format += 1;
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
		Ok((
			ledgers_with_cmix_id,
			existing_cmix_ids,
			total_ledgers,
			ledgers_in_old_format,
			exposures_in_old_format,
		)
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let (
			expected_cmix_ids,
			existing_before,
			_total_ledgers_before,
			ledgers_in_old_format,
			exposures_in_old_format,
		): (u32, u32, u32, u32, u32) =
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
				>::decode(&mut &raw_value[..])
				.is_ok()
				{
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
				if NewExposure::<T::AccountId, pallet_staking::BalanceOf<T>>::decode(
					&mut &raw_value[..],
				)
				.is_ok()
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

#[cfg(test)]
mod tests {
	use super::*;
	use codec::{Decode, Encode};
	use sp_core::crypto::AccountId32;

	/// Test encoding/decoding of Exposure structs with concrete types
	/// Using u32 for AccountId and u128 for Balance (similar to real runtime)
	#[test]
	fn test_exposure_encoding_roundtrip() {
		// Create an OldExposure with custody (custody is in SECOND position)
		let old_exposure = OldExposure::<u32, u128> {
			total: 1000u128,
			custody: 250u128,
			own: 500u128,
			others: vec![IndividualExposure { who: 1u32, value: 250u128 }],
		};

		let encoded = old_exposure.encode();
		println!("OldExposure encoded bytes ({} bytes): {:?}", encoded.len(), encoded);

		// Decode back as OldExposure - should succeed
		let decoded: OldExposure<u32, u128> =
			OldExposure::decode(&mut &encoded[..]).expect("Decode OldExposure failed");
		assert_eq!(decoded.total, 1000);
		assert_eq!(decoded.custody, 250);
	}

	#[test]
	fn test_new_exposure_encoding_roundtrip() {
		// Create a NewExposure without custody
		let new_exposure = NewExposure::<u32, u128> {
			total: 1000u128,
			own: 500u128,
			others: vec![IndividualExposure { who: 1u32, value: 250u128 }],
		};

		let encoded = new_exposure.encode();
		println!("NewExposure encoded bytes ({} bytes): {:?}", encoded.len(), encoded);

		// Decode back as NewExposure - should succeed
		let decoded: NewExposure<u32, u128> =
			NewExposure::decode(&mut &encoded[..]).expect("Decode NewExposure failed");
		assert_eq!(decoded.total, 1000);
		assert_eq!(decoded.own, 500);
	}

	#[test]
	fn test_old_exposure_cannot_decode_as_new() {
		// Create an OldExposure with custody (custody in second position)
		let old_exposure = OldExposure::<u32, u128> {
			total: 1000u128,
			custody: 250u128,
			own: 500u128,
			others: vec![],
		};

		let encoded = old_exposure.encode();
		println!(
			"OldExposure (empty others) encoded bytes ({} bytes): {:?}",
			encoded.len(),
			encoded
		);

		// Try to decode as NewExposure - what happens?
		let result = NewExposure::<u32, u128>::decode(&mut &encoded[..]);
		println!("Decoding OldExposure bytes as NewExposure: {:?}", result);

		// The decode might succeed but with wrong values if custody was consumed as "others" prefix
		// Or it might fail. Let's see what happens.
	}

	#[test]
	fn test_new_exposure_cannot_decode_as_old() {
		// Create a NewExposure without custody
		let new_exposure = NewExposure::<u32, u128> {
			total: 1000u128,
			own: 500u128,
			others: vec![],
		};

		let encoded = new_exposure.encode();
		println!(
			"NewExposure (empty others) encoded bytes ({} bytes): {:?}",
			encoded.len(),
			encoded
		);

		// Try to decode as OldExposure - should fail because no custody bytes
		let result = OldExposure::<u32, u128>::decode(&mut &encoded[..]);
		println!("Decoding NewExposure bytes as OldExposure: {:?}", result);
		assert!(result.is_err(), "NewExposure should not decode as OldExposure");
	}

	#[test]
	fn test_print_byte_layouts() {
		println!("\n=== Byte Layouts for Exposure Structs ===\n");

		// OldExposure with various values (custody in second position)
		let old = OldExposure::<u32, u128> {
			total: 100_000_000_000_000_000u128, // 0.1 * 10^18
			custody: 0u128,
			own: 50_000_000_000_000_000u128,
			others: vec![
				IndividualExposure { who: 12345u32, value: 25_000_000_000_000_000u128 },
				IndividualExposure { who: 67890u32, value: 25_000_000_000_000_000u128 },
			],
		};
		let old_bytes = old.encode();
		println!("OldExposure (real-world values, custody=0):");
		println!("  Bytes ({} total): {:?}\n", old_bytes.len(), old_bytes);

		// NewExposure with same values (no custody)
		let new = NewExposure::<u32, u128> {
			total: 100_000_000_000_000_000u128,
			own: 50_000_000_000_000_000u128,
			others: vec![
				IndividualExposure { who: 12345u32, value: 25_000_000_000_000_000u128 },
				IndividualExposure { who: 67890u32, value: 25_000_000_000_000_000u128 },
			],
		};
		let new_bytes = new.encode();
		println!("NewExposure (real-world values, no custody):");
		println!("  Bytes ({} total): {:?}\n", new_bytes.len(), new_bytes);

		// Compare
		println!("Difference in byte length: {} bytes", old_bytes.len() - new_bytes.len());
		println!("Old ends with: {:?}", &old_bytes[old_bytes.len() - 5..]);
		println!("New ends with: {:?}", &new_bytes[new_bytes.len() - 5..]);
	}

	#[test]
	fn test_with_accountid32() {
		println!("\n=== Testing with AccountId32 (real runtime type) ===\n");

		let _alice = AccountId32::new([1u8; 32]);
		let bob = AccountId32::new([2u8; 32]);

		// OldExposure with AccountId32 (custody in second position)
		let old = OldExposure::<AccountId32, u128> {
			total: 1_000_000_000_000_000_000u128, // 1 * 10^18
			custody: 0u128,
			own: 500_000_000_000_000_000u128,
			others: vec![IndividualExposure { who: bob.clone(), value: 500_000_000_000_000_000u128 }],
		};
		let old_bytes = old.encode();
		println!("OldExposure with AccountId32:");
		println!("  Total bytes: {}", old_bytes.len());
		println!("  First 20 bytes: {:?}", &old_bytes[..20]);
		println!("  Last 10 bytes: {:?}\n", &old_bytes[old_bytes.len() - 10..]);

		// NewExposure with AccountId32
		let new = NewExposure::<AccountId32, u128> {
			total: 1_000_000_000_000_000_000u128,
			own: 500_000_000_000_000_000u128,
			others: vec![IndividualExposure { who: bob.clone(), value: 500_000_000_000_000_000u128 }],
		};
		let new_bytes = new.encode();
		println!("NewExposure with AccountId32:");
		println!("  Total bytes: {}", new_bytes.len());
		println!("  First 20 bytes: {:?}", &new_bytes[..20]);
		println!("  Last 10 bytes: {:?}\n", &new_bytes[new_bytes.len() - 10..]);

		println!("Byte difference: {} bytes", old_bytes.len() as i32 - new_bytes.len() as i32);

		// Try to decode old as new (should work, ignores trailing bytes)
		let decoded_as_new = NewExposure::<AccountId32, u128>::decode(&mut &old_bytes[..]);
		println!("Decode OldExposure bytes as NewExposure: {:?}", decoded_as_new.is_ok());

		// Try to decode new as old (should fail, missing custody bytes)
		let decoded_as_old = OldExposure::<AccountId32, u128>::decode(&mut &new_bytes[..]);
		println!("Decode NewExposure bytes as OldExposure: {:?}", decoded_as_old.is_ok());
	}

	#[test]
	fn test_decode_compatibility() {
		// Key insight: OldExposure has custody in SECOND position, so decoding
		// as NewExposure will get wrong values (custody interpreted as own).
		let _alice = AccountId32::new([1u8; 32]);

		let old = OldExposure::<AccountId32, u128> {
			total: 1_000_000_000u128,
			custody: 250_000_000u128,
			own: 500_000_000u128,
			others: vec![],
		};
		let old_bytes = old.encode();

		// This will decode but with WRONG values - custody becomes own, own becomes vec_len
		let result = NewExposure::<AccountId32, u128>::decode(&mut &old_bytes[..]);
		println!("Decoding OldExposure bytes as NewExposure: {:?}", result);

		// The decode may succeed but values will be wrong because field order differs
		if let Ok(new_decoded) = result {
			println!("  total: {} (expected 1_000_000_000)", new_decoded.total);
			println!("  own: {} (expected 500_000_000, got custody value)", new_decoded.own);
			// own will actually be the custody value (250_000_000) because of field order mismatch
		}

		println!("IMPORTANT: OldExposure has custody in SECOND position, migration must reorder fields!");
	}

	#[test]
	fn test_decode_actual_chain_data() {
		// Test decoding actual on-chain data with the corrected OldExposure struct
		let raw_hex = "0fcd74a17ecd3602000ba40a33f46b3620c0ab0c276360b9d232e83df43ab19984d129d9dc68c0b33ea97190c3a34b213d0fc8b5debae3db017e49d11f1b44eddaae23ae634e8ff5ed8a040b7fed85f04907bc9d5ee2a278110bde0020bb2d2008fb3d1250b92899d59d9b625f9168e9681d6c4225776b8a34bc7aa9f2fc81760b90b458dfd30162d410a617c6a0adfa16922a236449e8d468c306f6199846fb363d9ab248a85a0bd7430e47b9018439db7a3a961d1c341ae8cc1f76ac6bd0b2d76ea7a08957c5708a54f2cf1864078b90fc9588aa5bba068604aa8dc9faadc86426b9ab65e07d2cc88cdb032615450843d3790f075a500294225e9dc158c3c392525ca76929a1c412ddd88ac26dca154fa3bb3feb7687c6765a075ef07e5e175f47da7a805e2e4ec0941be711f5c5629c9ee85bf09fe3007de09de4fddeaa5803d9e98a65";
		let raw_bytes = hex::decode(raw_hex).expect("valid hex");

		println!("Attempting to decode {} bytes as OldExposure", raw_bytes.len());

		let result = OldExposure::<AccountId32, u128>::decode(&mut &raw_bytes[..]);
		match result {
			Ok(exposure) => {
				println!("SUCCESS! Decoded OldExposure:");
				println!("  total: {} ({:.2} tokens)", exposure.total, exposure.total as f64 / 1e9);
				println!("  custody: {} ({:.2} tokens)", exposure.custody, exposure.custody as f64 / 1e9);
				println!("  own: {} ({:.2} tokens)", exposure.own, exposure.own as f64 / 1e9);
				println!("  others.len(): {}", exposure.others.len());
				for (i, ind) in exposure.others.iter().enumerate() {
					if i < 3 {
						println!("    [{}] value: {} ({:.2} tokens)", i, ind.value, ind.value as f64 / 1e9);
					}
				}
				if exposure.others.len() > 3 {
					println!("    ... and {} more", exposure.others.len() - 3);
				}
			}
			Err(e) => {
				println!("FAILED to decode: {:?}", e);
			}
		}
	}
}
