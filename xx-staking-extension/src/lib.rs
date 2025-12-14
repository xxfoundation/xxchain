//! # xx Staking Extension Pallet
//!
//! This pallet extends the upstream `pallet-staking` with additional functionality
//! required by the xx network, specifically:
//!
//! - **Point Deductions**: Track negative points for validators who underperform
//!   in the cMix protocol. The upstream pallet-staking only has `reward_by_ids`
//!   to add points, but xx network needs `deduct_by_ids` to subtract points.
//!
//! - **cMix ID Storage**: Store the cMix network ID associated with each staking
//!   account, which was previously embedded in the StakingLedger struct in the
//!   xx-labs fork of pallet-staking.
//!
//! ## How Point Deductions Work
//!
//! Instead of modifying the upstream ErasRewardPoints storage directly (which would
//! require a fork), we maintain a separate ErasDeductionPoints storage. The net
//! points for any validator in an era is:
//!
//!   `net_points = reward_points - deduction_points`
//!
//! The reward payout logic should use `get_net_points()` instead of reading
//! ErasRewardPoints directly.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Get, weights::Weight};
use frame_system::{ensure_signed, pallet_prelude::*};
use scale_info::TypeInfo;
use sp_staking::EraIndex;

pub use pallet::*;

/// cMix network ID type (32 bytes)
pub type CmixId = [u8; 32];

/// Trait for staking extension operations that can be used by other pallets
pub trait StakingExtension<AccountId> {
	/// Deduct points from validators for the current era
	fn deduct_by_ids(validators_points: impl IntoIterator<Item = (AccountId, u32)>);
}

/// Era deduction points structure, mirrors EraRewardPoints from pallet-staking
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug)]
pub struct EraDeductionPoints<AccountId: Ord + MaxEncodedLen> {
	/// Total number of deduction points for the era
	pub total: u32,
	/// Individual deduction points per validator
	pub individual: BoundedBTreeMap<AccountId, u32, ConstU32<1000>>,
}

impl<AccountId: Ord + MaxEncodedLen> Default for EraDeductionPoints<AccountId> {
	fn default() -> Self {
		Self { total: 0, individual: BoundedBTreeMap::new() }
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_staking::Config
	{
	}

	/// Deduction points for each era.
	///
	/// This mirrors the ErasRewardPoints storage from pallet-staking, but tracks
	/// negative points that should be subtracted from rewards.
	#[pallet::storage]
	#[pallet::getter(fn eras_deduction_points)]
	pub type ErasDeductionPoints<T: Config> =
		StorageMap<_, Twox64Concat, EraIndex, EraDeductionPoints<T::AccountId>, ValueQuery>;

	/// cMix ID associated with each stash account.
	///
	/// This was previously part of the StakingLedger struct in the xx-labs fork
	/// of pallet-staking. Now stored separately to avoid forking upstream.
	#[pallet::storage]
	#[pallet::getter(fn cmix_id)]
	pub type CmixIds<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, CmixId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Points were deducted from validators.
		PointsDeducted { era: EraIndex, total_deducted: u32 },
		/// cMix ID was set for an account.
		CmixIdSet { stash: T::AccountId, cmix_id: CmixId },
		/// cMix ID was cleared for an account.
		CmixIdCleared { stash: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The validator count exceeds the maximum allowed.
		TooManyValidators,
		/// The account is not bonded (no staking ledger).
		NotBonded,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the cMix ID for a bonded stash account.
		///
		/// The origin must be the stash account that has an active staking ledger.
		/// This replaces the cmix_id parameter that was previously part of the
		/// forked pallet-staking's `bond` function.
		///
		/// # Arguments
		/// * `cmix_id` - The 32-byte cMix network identifier
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn set_cmix_id(origin: OriginFor<T>, cmix_id: CmixId) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			// Ensure the account is bonded (has a ledger)
			ensure!(pallet_staking::Ledger::<T>::contains_key(&stash), Error::<T>::NotBonded);

			// Set the cmix_id
			CmixIds::<T>::insert(&stash, cmix_id);
			Self::deposit_event(Event::CmixIdSet { stash, cmix_id });

			Ok(())
		}

		/// Clear the cMix ID for a bonded stash account.
		///
		/// The origin must be the stash account.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn clear_cmix_id(origin: OriginFor<T>) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			// Ensure the account is bonded (has a ledger)
			ensure!(pallet_staking::Ledger::<T>::contains_key(&stash), Error::<T>::NotBonded);

			// Clear the cmix_id
			CmixIds::<T>::remove(&stash);
			Self::deposit_event(Event::CmixIdCleared { stash });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Deduct points from validators for the current era.
		///
		/// This is the counterpart to `pallet_staking::Pallet::reward_by_ids`.
		/// Points deducted here will be subtracted from the validator's share
		/// of era rewards.
		pub fn deduct_by_ids(validators_points: impl IntoIterator<Item = (T::AccountId, u32)>) {
			let active_era = match pallet_staking::ActiveEra::<T>::get() {
				Some(era) => era.index,
				None => return,
			};

			ErasDeductionPoints::<T>::mutate(active_era, |era_deductions| {
				let mut total_deducted = 0u32;
				for (validator, points) in validators_points {
					total_deducted = total_deducted.saturating_add(points);
					era_deductions.total = era_deductions.total.saturating_add(points);

					let current_points =
						era_deductions.individual.get(&validator).copied().unwrap_or(0);
					let _ = era_deductions
						.individual
						.try_insert(validator, current_points.saturating_add(points));
				}

				if total_deducted > 0 {
					Self::deposit_event(Event::PointsDeducted { era: active_era, total_deducted });
				}
			});
		}

		/// Get the net points (rewards - deductions) for a validator in an era.
		pub fn get_net_points(era: EraIndex, validator: &T::AccountId) -> u32 {
			let rewards = pallet_staking::ErasRewardPoints::<T>::get(era);
			let deductions = ErasDeductionPoints::<T>::get(era);

			let reward_points = rewards.individual.get(validator).copied().unwrap_or(0);
			let deduction_points = deductions.individual.get(validator).copied().unwrap_or(0);

			reward_points.saturating_sub(deduction_points)
		}

		/// Get the total net points for an era.
		pub fn get_total_net_points(era: EraIndex) -> u32 {
			let rewards = pallet_staking::ErasRewardPoints::<T>::get(era);
			let deductions = ErasDeductionPoints::<T>::get(era);

			rewards.total.saturating_sub(deductions.total)
		}

		/// Set cMix ID for a stash account (internal helper).
		/// For external calls, use the `set_cmix_id` extrinsic.
		pub fn do_set_cmix_id(stash: &T::AccountId, cmix_id: CmixId) {
			CmixIds::<T>::insert(stash, cmix_id);
			Self::deposit_event(Event::CmixIdSet { stash: stash.clone(), cmix_id });
		}

		/// Clear cMix ID for a stash account (internal helper).
		/// For external calls, use the `clear_cmix_id` extrinsic.
		pub fn do_clear_cmix_id(stash: &T::AccountId) {
			CmixIds::<T>::remove(stash);
			Self::deposit_event(Event::CmixIdCleared { stash: stash.clone() });
		}

		/// Get cMix ID for a stash account.
		pub fn get_cmix_id(stash: &T::AccountId) -> Option<CmixId> {
			CmixIds::<T>::get(stash)
		}

		/// Check if an account has a cMix ID set.
		pub fn has_cmix_id(stash: &T::AccountId) -> bool {
			CmixIds::<T>::contains_key(stash)
		}
	}

	/// Hooks to clean up old era deduction points
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			// Clean up old deduction points based on HistoryDepth
			// This matches pallet-staking's cleanup of ErasRewardPoints
			if let Some(active_era) = pallet_staking::ActiveEra::<T>::get() {
				let history_depth = T::HistoryDepth::get();
				if active_era.index > history_depth {
					let oldest_to_keep = active_era.index.saturating_sub(history_depth);
					// Remove eras older than history depth
					for era in 0..oldest_to_keep {
						if ErasDeductionPoints::<T>::contains_key(era) {
							ErasDeductionPoints::<T>::remove(era);
						}
					}
				}
			}
			Weight::zero()
		}
	}
}

/// Implement the StakingExtension trait for Pallet
impl<T: Config> StakingExtension<T::AccountId> for Pallet<T> {
	fn deduct_by_ids(validators_points: impl IntoIterator<Item = (T::AccountId, u32)>) {
		Self::deduct_by_ids(validators_points)
	}
}

#[cfg(test)]
mod tests {
	// Tests would go here
}
