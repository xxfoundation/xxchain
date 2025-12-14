//! # XX Team Custody Pallet
//!
//! This pallet manages team token custody with time-locked vesting and payout schedules.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub use weights::WeightInfo;

pub mod custody;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use alloc::vec::Vec;
use frame_support::traits::{Currency, Get, fungible::Inspect};
use sp_runtime::traits::Convert;

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_proxy::Config + pallet_staking::Config {
        /// The currency mechanism.
        type Currency: Currency<Self::AccountId> + Inspect<Self::AccountId>;

        /// The payout frequency of vested coins under custody.
        #[pallet::constant]
        type PayoutFrequency: Get<BlockNumberFor<Self>>;

        /// The custody duration.
        #[pallet::constant]
        type CustodyDuration: Get<BlockNumberFor<Self>>;

        /// The governance custody duration.
        #[pallet::constant]
        type GovernanceCustodyDuration: Get<BlockNumberFor<Self>>;

        /// The getter for the proxy type to use for custody accounts
        type CustodyProxy: Get<<Self as pallet_proxy::Config>::ProxyType>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

        /// The admin origin for the pallet (Tech Committee unanimity).
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    /// Keep track of team members' accounts custody info
    #[pallet::storage]
    #[pallet::getter(fn team_accounts)]
    pub type TeamAccounts<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        custody::CustodyInfo<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Keep track of custody accounts
    #[pallet::storage]
    #[pallet::getter(fn custody_accounts)]
    pub type CustodyAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

    /// Keep track of custodians
    #[pallet::storage]
    #[pallet::getter(fn custodians)]
    pub type Custodians<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

    /// Total amount under custody
    #[pallet::storage]
    #[pallet::getter(fn total_custody)]
    pub type TotalCustody<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Team payout was given from custody
        PayoutFromCustody { who: T::AccountId, amount: BalanceOf<T> },
        /// Team payout was given from reserve
        PayoutFromReserve { who: T::AccountId, amount: BalanceOf<T> },
        /// Custody finished for the given team account
        CustodyDone { who: T::AccountId },
        /// Custodian added
        CustodianAdded { who: T::AccountId },
        /// Custodian removed
        CustodianRemoved { who: T::AccountId },
        /// Team member updated
        TeamMemberUpdated { old: T::AccountId, new: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid team member account
        InvalidTeamMember,
        /// Invalid custody account
        InvalidCustodyAccount,
        /// Must be custodian to call this function
        MustBeCustodian,
        /// Payout not available yet
        PayoutNotAvailable,
        /// Payout failed due to insufficient custody + reserve funds
        PayoutFailedInsufficientFunds,
        /// Custody period ended, custodian can't call this function anymore
        CustodyPeriodEnded,
        /// Governance custody ongoing, team member can't call this function yet
        GovernanceCustodyActive,
        /// Governance custody period ended, custodian can't call this function anymore
        GovernanceCustodyPeriodEnded,
        /// This team member account already exists
        TeamMemberExists,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub custodians: Vec<(T::AccountId, ())>,
        pub team_allocations: Vec<(T::AccountId, BalanceOf<T>)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for (custodian, _) in &self.custodians {
                Custodians::<T>::insert(custodian, ());
            }
            for (who, balance) in &self.team_allocations {
                custody::initialize_custody::<T>(who, *balance);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Payout the amount already vested to the given team member account
        ///
        /// Anyone can call this function since it is deterministic
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::payout())]
        pub fn payout(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(Self::is_team_member(&who), Error::<T>::InvalidTeamMember);
            custody::try_payout::<T>(who)?;
            Ok(())
        }

        /// Bond the given amount from the given custody account, with the specified controller
        ///
        /// During the Custody period, the function is callable by Custodians only. After the
        /// Custody ends, the function is not callable anymore.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::custody_bond())]
        pub fn custody_bond(
            origin: OriginFor<T>,
            custody: T::AccountId,
            controller: T::AccountId,
            #[pallet::compact] value: pallet_staking::BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_custodian(&who), Error::<T>::MustBeCustodian);
            ensure!(Self::is_custody(&custody), Error::<T>::InvalidCustodyAccount);
            custody::try_custody_bond::<T>(custody, controller, value)?;
            Ok(())
        }

        /// Bond extra amount from the given custody account
        ///
        /// During the Custody period, the function is callable by Custodians only. After the
        /// Custody ends, the function is not callable anymore.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::custody_bond_extra())]
        pub fn custody_bond_extra(
            origin: OriginFor<T>,
            custody: T::AccountId,
            #[pallet::compact] value: pallet_staking::BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_custodian(&who), Error::<T>::MustBeCustodian);
            ensure!(Self::is_custody(&custody), Error::<T>::InvalidCustodyAccount);
            custody::try_custody_bond_extra::<T>(custody, value)?;
            Ok(())
        }

        /// Set the controller of a given custody account
        ///
        /// During the Custody period, the function is callable by Custodians only. After the
        /// Custody ends, the function is not callable anymore.
        ///
        /// NOTE: Controller is deprecated in SDK 2509+. This function is now a no-op.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::custody_set_controller())]
        pub fn custody_set_controller(
            origin: OriginFor<T>,
            custody: T::AccountId,
            controller: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_custodian(&who), Error::<T>::MustBeCustodian);
            ensure!(Self::is_custody(&custody), Error::<T>::InvalidCustodyAccount);
            custody::try_custody_set_controller::<T>(custody, controller)?;
            Ok(())
        }

        /// Set the governance proxy of a given custody account
        ///
        /// Only one proxy account is allowed per custody account, so this function
        /// removes any proxies first, and then adds the new proxy
        ///
        /// During the Governance Custody period, the function is callable by Custodians only.
        /// After the Governance Custody ends, the function is not callable anymore.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::custody_set_proxy())]
        pub fn custody_set_proxy(
            origin: OriginFor<T>,
            custody: T::AccountId,
            proxy: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_custodian(&who), Error::<T>::MustBeCustodian);
            ensure!(Self::is_custody(&custody), Error::<T>::InvalidCustodyAccount);
            custody::try_custody_set_proxy::<T>(custody, proxy)?;
            Ok(())
        }

        /// Allow the team member to set a governance proxy of their own custody account
        ///
        /// During the Governance Custody period, the function is not callable.
        /// After the Governance Custody ends, the function is callable by team members only.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::team_custody_set_proxy())]
        pub fn team_custody_set_proxy(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_team_member(&who), Error::<T>::InvalidTeamMember);
            custody::try_team_custody_set_proxy::<T>(who, proxy)?;
            Ok(())
        }

        /// Add a custodian account
        ///
        /// The dispatch origin must be AdminOrigin.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::add_custodian())]
        pub fn add_custodian(origin: OriginFor<T>, custodian: T::AccountId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Custodians::<T>::insert(&custodian, ());
            Self::deposit_event(Event::CustodianAdded { who: custodian });
            Ok(())
        }

        /// Remove a custodian account
        ///
        /// The dispatch origin must be AdminOrigin.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as Config>::WeightInfo::remove_custodian())]
        pub fn remove_custodian(origin: OriginFor<T>, custodian: T::AccountId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Custodians::<T>::remove(&custodian);
            Self::deposit_event(Event::CustodianRemoved { who: custodian });
            Ok(())
        }

        /// Replace an existing team member account with a new account
        ///
        /// The dispatch origin must be AdminOrigin.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as Config>::WeightInfo::replace_team_member())]
        pub fn replace_team_member(
            origin: OriginFor<T>,
            who: T::AccountId,
            new: T::AccountId,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            ensure!(Self::is_team_member(&who), Error::<T>::InvalidTeamMember);
            ensure!(!Self::is_team_member(&new), Error::<T>::TeamMemberExists);
            custody::update_team_member::<T>(who.clone(), new.clone());
            Self::deposit_event(Event::TeamMemberUpdated { old: who, new });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if given account is a team member
        pub fn is_team_member(who: &T::AccountId) -> bool {
            TeamAccounts::<T>::contains_key(who)
        }

        /// Check if given account is a custody account
        pub fn is_custody(who: &T::AccountId) -> bool {
            CustodyAccounts::<T>::contains_key(who)
        }

        /// Check if given account is a custodian
        pub fn is_custodian(who: &T::AccountId) -> bool {
            Custodians::<T>::contains_key(who)
        }

        /// Check if origin is admin
        fn ensure_admin(o: T::RuntimeOrigin) -> DispatchResult {
            <T as Config>::AdminOrigin::try_origin(o)
                .map(|_| ())
                .or_else(|o| frame_system::ensure_root(o))?;
            Ok(())
        }
    }
}

// Re-export for backwards compatibility
pub use pallet::{Config, Pallet, Event, Error, GenesisConfig};
pub use pallet::{TeamAccounts, CustodyAccounts, Custodians, TotalCustody};
