#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod cmix;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use alloc::vec::Vec;

use frame_support::{
	dispatch::{DispatchClass, DispatchResult, Pays},
	ensure,
	traits::{EnsureOrigin, RewardsReporter},
};

use frame_system::{ensure_root, ensure_signed};
pub use weights::WeightInfo;
use xx_staking_extension::StakingExtension;

/// Trait for cMix protocol integration with staking
/// This was previously part of the forked pallet-staking, now defined locally
pub trait CmixHandler {
	/// Get block points for current era
	fn get_block_points() -> u32;
	/// Called at the end of each era
	fn end_era();
}

/// Default implementation for chains that don't use cMix
pub struct DefaultCmixHandler;
impl CmixHandler for DefaultCmixHandler {
	fn get_block_points() -> u32 {
		20 // default block points
	}
	fn end_era() {}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_staking::Config + xx_staking_extension::Config
	{
		/// The Event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The origin that is allowed to modify cmix variables.
		type CmixVariablesOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The admin origin for the pallet (Tech Committee unanimity).
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Cmix hashes updated
		CmixHashesUpdated,
		/// Admin permission updated
		AdminPermissionUpdated(BlockNumberFor<T>),
		/// Scheduling server account updated
		SchedulingAccountUpdated,
		/// Cmix variables updated
		CmixVariablesUpdated,
		/// Cmix address space size updated
		CmixAddressSpaceUpdated,
		/// Cmix points data submitted to chain
		CmixPointsAdded,
		/// Cmix points deduction data submitted to chain
		CmixPointsDeducted,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// AdminOrigin is not allowed to modify cmix hashes
		AdminPermissionExpired,
		/// Must be scheduling server account to call this function
		MustBeScheduling,
	}

	/// Cmix software hashes
	#[pallet::storage]
	#[pallet::getter(fn cmix_hashes)]
	pub type CmixHashes<T: Config> = StorageValue<_, cmix::SoftwareHashes<T::Hash>, ValueQuery>;

	/// Highest block number that AdminOrigin is allowed to change cmix hashes
	#[pallet::storage]
	#[pallet::getter(fn admin_permission)]
	pub type AdminPermission<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Scheduling server account
	#[pallet::storage]
	#[pallet::getter(fn scheduling_account)]
	pub type SchedulingAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Cmix user ephemeral reception IDs address space size in bits
	#[pallet::storage]
	#[pallet::getter(fn cmix_address_space)]
	pub type CmixAddressSpace<T> = StorageValue<_, u8, ValueQuery>;

	/// Next cmix variables
	#[pallet::storage]
	#[pallet::getter(fn next_cmix_variables)]
	pub type NextCmixVariables<T> = StorageValue<_, cmix::Variables, OptionQuery>;

	/// Current cmix variables
	#[pallet::storage]
	#[pallet::getter(fn cmix_variables)]
	pub type CmixVariables<T> = StorageValue<_, cmix::Variables, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub cmix_hashes: cmix::SoftwareHashes<T::Hash>,
		pub admin_permission: BlockNumberFor<T>,
		pub scheduling_account: Option<T::AccountId>,
		pub cmix_address_space: u8,
		pub cmix_variables: cmix::Variables,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			CmixHashes::<T>::put(&self.cmix_hashes);
			AdminPermission::<T>::put(&self.admin_permission);
			CmixAddressSpace::<T>::put(self.cmix_address_space);
			CmixVariables::<T>::put(&self.cmix_variables);

			// Set scheduling account
			if let Some(acct) = &self.scheduling_account {
				SchedulingAccount::<T>::put(acct);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set cmix software hashes
		///
		/// The dispatch origin must be AdminOrigin.
		/// Furthermore, this call is only allowed if current block is lower than `AdminPermission`.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_cmix_hashes())]
		pub fn set_cmix_hashes(
			origin: OriginFor<T>,
			hashes: cmix::SoftwareHashes<T::Hash>,
		) -> DispatchResult {
			Self::ensure_admin(origin)?;
			Self::ensure_admin_allowed_cmix_hashes()?;
			CmixHashes::<T>::put(hashes);
			Self::deposit_event(Event::CmixHashesUpdated);
			Ok(())
		}

		/// Set scheduling server account
		///
		/// The dispatch origin must be AdminOrigin.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::set_scheduling_account())]
		pub fn set_scheduling_account(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			Self::ensure_admin(origin)?;
			SchedulingAccount::<T>::put(who);
			Self::deposit_event(Event::SchedulingAccountUpdated);
			Ok(())
		}

		/// Set next cmix variables
		///
		/// The dispatch origin must be `CmixVariablesOrigin`.
		/// The new variables will be stored in `NextCmixVariables`.
		/// Then, at the beginning of the next era, `NextCmixVariables` is emptied and the value
		/// is written to `CmixVariables`.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_next_cmix_variables())]
		pub fn set_next_cmix_variables(
			origin: OriginFor<T>,
			variables: cmix::Variables,
		) -> DispatchResult {
			Self::ensure_cmix_variables(origin)?;
			NextCmixVariables::<T>::put(variables);
			Ok(())
		}

		/// Submit cmix performance points
		///
		/// `data` is a vector of tuples of (account, points)
		/// The dispatch origin must be `SchedulingAccount`
		#[pallet::call_index(3)]
		#[pallet::weight((
            <T as Config>::WeightInfo::submit_cmix_points(data.len() as u32),
            DispatchClass::Operational,
            Pays::No
        ))]
		pub fn submit_cmix_points(
			origin: OriginFor<T>,
			data: Vec<(T::AccountId, u32)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_scheduling(who), Error::<T>::MustBeScheduling);
			Self::reward_cmix_points(data);
			Self::deposit_event(Event::CmixPointsAdded);
			Ok(())
		}

		/// Submit cmix performance points deductions
		///
		/// `data` is a vector of tuples of (account, points)
		/// The dispatch origin must be `SchedulingAccount`
		#[pallet::call_index(4)]
		#[pallet::weight((
            <T as Config>::WeightInfo::submit_cmix_deductions(data.len() as u32),
            DispatchClass::Operational,
            Pays::No
        ))]
		pub fn submit_cmix_deductions(
			origin: OriginFor<T>,
			data: Vec<(T::AccountId, u32)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_scheduling(who), Error::<T>::MustBeScheduling);
			Self::deduct_cmix_points(data);
			Self::deposit_event(Event::CmixPointsDeducted);
			Ok(())
		}

		/// Set cmix address space size
		///
		/// The dispatch origin must be `SchedulingAccount`
		#[pallet::call_index(5)]
		#[pallet::weight((
            <T as Config>::WeightInfo::set_cmix_address_space(),
            DispatchClass::Operational,
            Pays::No
        ))]
		pub fn set_cmix_address_space(origin: OriginFor<T>, size: u8) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_scheduling(who), Error::<T>::MustBeScheduling);
			CmixAddressSpace::<T>::put(size);
			Self::deposit_event(Event::CmixAddressSpaceUpdated);
			Ok(())
		}

		/// Set admin permission
		///
		/// `permission` is the block number up to which the AdminOrigin
		/// will be allowed to call the `set_cmix_hashes` function.
		/// It is expected that `permission` will be modified by Democracy
		/// in 6-month periods.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_admin_permission())]
		pub fn set_admin_permission(
			origin: OriginFor<T>,
			permission: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			AdminPermission::<T>::put(permission);
			Self::deposit_event(Event::AdminPermissionUpdated(permission));
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Check if origin is admin
	fn ensure_admin(o: T::RuntimeOrigin) -> DispatchResult {
		<T as Config>::AdminOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(())
	}

	/// Checks if admin is allowed to modify cmix hashes
	fn ensure_admin_allowed_cmix_hashes() -> DispatchResult {
		let block = <frame_system::Pallet<T>>::block_number();
		let permission = AdminPermission::<T>::get();
		ensure!(permission >= block, Error::<T>::AdminPermissionExpired);
		Ok(())
	}

	/// Check if given account is scheduling server
	fn is_scheduling(who: T::AccountId) -> bool {
		if let Some(sched) = SchedulingAccount::<T>::get() {
			who == sched
		} else {
			false
		}
	}

	/// Check if origin is cmix variables
	fn ensure_cmix_variables(o: T::RuntimeOrigin) -> DispatchResult {
		T::CmixVariablesOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(())
	}

	/// Add cmix points to staking era rewards
	pub fn reward_cmix_points(data: Vec<(T::AccountId, u32)>) {
		<pallet_staking::Pallet<T> as RewardsReporter<_>>::reward_by_ids(data)
	}

	/// Deduct cmix points from staking era rewards
	/// Uses xx-staking-extension wrapper pallet to track deductions separately
	/// from the upstream pallet-staking's ErasRewardPoints.
	pub fn deduct_cmix_points(data: Vec<(T::AccountId, u32)>) {
		<xx_staking_extension::Pallet<T> as StakingExtension<T::AccountId>>::deduct_by_ids(data)
	}
}

/// Implement CmixHandler trait
impl<T: Config> CmixHandler for Pallet<T> {
	fn get_block_points() -> u32 {
		CmixVariables::<T>::get().get_block_points()
	}

	fn end_era() {
		// Update cmix variables if next ones are set
		if let Some(next) = NextCmixVariables::<T>::take() {
			CmixVariables::<T>::put(next);
			Self::deposit_event(Event::CmixVariablesUpdated);
		}
	}
}

// Type alias for backwards compatibility
pub type Module<T> = Pallet<T>;
