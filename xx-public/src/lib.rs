//! # XX Public Pallet
//!
//! Manages public/system accounts for testnet and sale distributions.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, HasCompact};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get, VestingSchedule},
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{AccountIdConversion, Zero};

pub type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<
	<T as frame_system::Config>::AccountId,
>>::Currency;
pub type BalanceOf<T> =
	<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Transfer data contains information about a single transfer
#[derive(PartialEq, Eq, Clone, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct TransferData<AccountId, Balance: HasCompact, Block> {
	/// Destination account
	pub destination: AccountId,
	/// Amount to transfer
	#[codec(compact)]
	pub amount: Balance,
	/// Vesting schedules info
	pub schedules: Option<Vec<(Balance, Balance, Block)>>,
}

pub trait PublicAccountsHandler<AccountId> {
	fn accounts() -> Vec<AccountId>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// The Vesting mechanism.
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;

		/// An ID used to derive the Testnet account
		#[pallet::constant]
		type TestnetId: Get<PalletId>;

		/// An ID used to derive the Sale account
		#[pallet::constant]
		type SaleId: Get<PalletId>;

		/// The admin origin for the pallet
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn testnet_manager)]
	pub type TestnetManager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn sale_manager)]
	pub type SaleManager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub testnet_manager: Option<T::AccountId>,
		pub testnet_balance: BalanceOf<T>,
		pub sale_manager: Option<T::AccountId>,
		pub sale_balance: BalanceOf<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Set managers
			if let Some(manager) = &self.testnet_manager {
				<TestnetManager<T>>::put(manager);
			}
			if let Some(manager) = &self.sale_manager {
				<SaleManager<T>>::put(manager);
			}
			// Create Testnet account and set balance from genesis
			let testnet_account_id = <Pallet<T>>::testnet_account_id();
			let _ =
				<CurrencyOf<T>>::make_free_balance_be(&testnet_account_id, self.testnet_balance);
			// Create Sale account and set the balance from genesis
			let sale_account_id = <Pallet<T>>::sale_account_id();
			let _ = <CurrencyOf<T>>::make_free_balance_be(&sale_account_id, self.sale_balance);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Testnet Manager updated
		TestnetManagerUpdated,
		/// Sale Manager updated
		SaleManagerUpdated,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Must be the testnet manager to call this function
		MustBeTestnetManager,
		/// Must be the sale manager to call this function
		MustBeSaleManager,
		/// Not enough funds to do distribution
		NotEnoughFunds,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the Testnet manager account
		///
		/// The dispatch origin must be AdminOrigin.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_testnet_manager_account())]
		pub fn set_testnet_manager_account(
			origin: OriginFor<T>,
			who: T::AccountId,
		) -> DispatchResult {
			Self::ensure_admin(origin)?;
			<TestnetManager<T>>::put(&who);
			Self::deposit_event(Event::TestnetManagerUpdated);
			Ok(())
		}

		/// Set the Sale manager account
		///
		/// The dispatch origin must be AdminOrigin.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::set_sale_manager_account())]
		pub fn set_sale_manager_account(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			Self::ensure_admin(origin)?;
			<SaleManager<T>>::put(&who);
			Self::deposit_event(Event::SaleManagerUpdated);
			Ok(())
		}

		/// Do a testnet distribution
		///
		/// `data` is a vector of TransferData
		/// The dispatch origin must be `TestnetManager`
		#[pallet::call_index(2)]
		#[pallet::weight((
            <T as Config>::WeightInfo::testnet_distribute(data.len() as u32),
            DispatchClass::Operational,
            Pays::No
        ))]
		pub fn testnet_distribute(
			origin: OriginFor<T>,
			data: Vec<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_testnet_manager(who), Error::<T>::MustBeTestnetManager);
			Self::do_testnet_distribution(data)
		}

		/// Do a sale distribution
		///
		/// `data` is a vector of TransferData
		/// The dispatch origin must be `SaleManager`
		#[pallet::call_index(3)]
		#[pallet::weight((
            <T as Config>::WeightInfo::sale_distribute(data.len() as u32),
            DispatchClass::Operational,
            Pays::No
        ))]
		pub fn sale_distribute(
			origin: OriginFor<T>,
			data: Vec<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_sale_manager(who), Error::<T>::MustBeSaleManager);
			Self::do_sale_distribution(data)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the Testnet AccountId
		pub fn testnet_account_id() -> T::AccountId {
			T::TestnetId::get().into_account_truncating()
		}

		/// Get the Sale AccountId
		pub fn sale_account_id() -> T::AccountId {
			T::SaleId::get().into_account_truncating()
		}

		/// Check if origin is admin
		fn ensure_admin(o: T::RuntimeOrigin) -> DispatchResult {
			<T as Config>::AdminOrigin::try_origin(o)
				.map(|_| ())
				.or_else(frame_system::ensure_root)?;
			Ok(())
		}

		/// Check if given account is the testnet manager
		fn is_testnet_manager(who: T::AccountId) -> bool {
			if let Some(manager) = <TestnetManager<T>>::get() {
				who == manager
			} else {
				false
			}
		}

		/// Check if given account is the sale manager
		fn is_sale_manager(who: T::AccountId) -> bool {
			if let Some(manager) = <SaleManager<T>>::get() {
				who == manager
			} else {
				false
			}
		}

		/// Do a testnet distribution
		fn do_testnet_distribution(
			data: Vec<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>,
		) -> DispatchResult {
			Self::do_distribution(Self::testnet_account_id(), data)
		}

		/// Do a sale distribution
		fn do_sale_distribution(
			data: Vec<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>,
		) -> DispatchResult {
			Self::do_distribution(Self::sale_account_id(), data)
		}

		/// Do a distribution
		fn do_distribution(
			account: T::AccountId,
			data: Vec<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>,
		) -> DispatchResult {
			// Exit early if not enough funds to do distribution
			let available = <CurrencyOf<T>>::free_balance(&account);
			let total = data.iter().fold(Zero::zero(), |acc, x| acc + x.amount);
			ensure!(available >= total, Error::<T>::NotEnoughFunds);
			// Do distribution
			data.iter().try_for_each(|d| -> DispatchResult {
				<CurrencyOf<T>>::transfer(&account, &d.destination, d.amount, AllowDeath)?;
				if let Some(vs) = &d.schedules {
					vs.iter().for_each(|v| {
						// This can fail if we try to add more vesting schedules
						// than the Vesting pallet limit.
						// The caller is responsible for ensuring the limit is respected.
						// This function can only be called by the privileged manager accounts,
						// so this is fine.
						// Regardless, in the case that too many schedules are used,
						// by ignoring the return value, we ensure the function never
						// fails, but the extra vesting schedules are ignored.
						let _ =
							T::VestingSchedule::add_vesting_schedule(&d.destination, v.0, v.1, v.2);
					});
				}
				Ok(())
			})
		}
	}
}

// Implement PublicAccountsHandler
impl<T: Config> PublicAccountsHandler<T::AccountId> for Pallet<T> {
	fn accounts() -> Vec<T::AccountId> {
		let testnet_account = Self::testnet_account_id();
		let sale_account = Self::sale_account_id();
		alloc::vec![testnet_account, sale_account]
	}
}
