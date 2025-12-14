// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use frame_support::traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get};
use frame_support::{dispatch::DispatchResult, ensure};
use frame_system::{ensure_root, ensure_signed};
use sp_core::U256;
use sp_runtime::traits::SaturatedConversion;
pub use weights::WeightInfo;

pub use pallet::*;

pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

type ResourceId = chainbridge::ResourceId;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + chainbridge::Config {
        /// The Event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// The currency mechanism.
        type Currency: Currency<Self::AccountId>;

        /// Native token ID
        type NativeTokenId: Get<ResourceId>;

        /// Origin used to change fee and destination
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Swap service fee was changed
        FeeChanged(BalanceOf<T>),
        /// Swap fee destination was changed
        FeeDestinationChanged(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        DestinationNotWhitelisted,
        InsufficientBalance,
    }

    /// Swap service fee charged when moving native tokens out of the chain
    #[pallet::storage]
    #[pallet::getter(fn swap_fee)]
    pub type SwapFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Account to which the fee is paid to
    #[pallet::storage]
    #[pallet::getter(fn fee_destination)]
    pub type FeeDestination<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub swap_fee: BalanceOf<T>,
        pub chains: Vec<u8>,
        pub relayers: Vec<T::AccountId>,
        pub resources: Vec<(ResourceId, Vec<u8>)>,
        pub threshold: u32,
        pub balance: BalanceOf<T>,
        pub fee_destination: Option<T::AccountId>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                swap_fee: Default::default(),
                chains: Default::default(),
                relayers: Default::default(),
                resources: Default::default(),
                threshold: 1, // Must be > 0 for chainbridge
                balance: Default::default(),
                fee_destination: None,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Set swap fee
            SwapFee::<T>::put(&self.swap_fee);

            // Initialize chains, relayers and resources
            // Uses expect to panic in the case the values cannot be set. This is reasonable as in that
            // case the chain is invalid and should not progress any further.
            Pallet::<T>::initialize(&self.chains, &self.relayers, &self.resources, &self.threshold)
                .expect("Could not set config on Chainbridge pallet");

            // Create chainbridge account and set the balance from genesis
            let account_id = chainbridge::Pallet::<T>::account_id();
            T::Currency::make_free_balance_be(&account_id, self.balance);

            // Set fee destination
            if let Some(dest) = &self.fee_destination {
                FeeDestination::<T>::put(dest);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfers an amount of the native token to some recipient on a (whitelisted) destination chain.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::transfer_native())]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: chainbridge::ChainId,
        ) -> DispatchResult {
            let source = ensure_signed(origin)?;

            // Ensure destination chain is whitelisted
            ensure!(
                chainbridge::Pallet::<T>::chain_whitelisted(dest_id),
                Error::<T>::DestinationNotWhitelisted
            );

            // Ensure account has enough balance to pay for both fee and transfer
            let fee = SwapFee::<T>::get();
            let balance = T::Currency::free_balance(&source);
            ensure!(balance >= amount + fee, Error::<T>::InsufficientBalance);

            // Transfer fee to configured destination (if destination exists)
            if let Some(dest) = FeeDestination::<T>::get() {
                T::Currency::transfer(&source, &dest, fee, AllowDeath)?;
            }

            // Transfer amount to bridge
            let bridge_id = chainbridge::Pallet::<T>::account_id();
            T::Currency::transfer(&source, &bridge_id, amount, AllowDeath)?;

            let resource_id = T::NativeTokenId::get();
            chainbridge::Pallet::<T>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from(amount.saturated_into::<u128>()),
            )?;
            Ok(())
        }

        /// Executes a currency transfer from the bridge account
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            T::Currency::transfer(&source, &to, amount, AllowDeath)?;
            Ok(())
        }

        /// Set swap fee
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::set_swap_fee())]
        pub fn set_swap_fee(
            origin: OriginFor<T>,
            #[pallet::compact] fee: BalanceOf<T>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            SwapFee::<T>::put(fee);
            Self::deposit_event(Event::FeeChanged(fee));
            Ok(())
        }

        /// Set fee destination
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::set_fee_destination())]
        pub fn set_fee_destination(origin: OriginFor<T>, dest: T::AccountId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            FeeDestination::<T>::put(dest.clone());
            Self::deposit_event(Event::FeeDestinationChanged(dest));
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Initialize bridge configurations from genesis
    fn initialize(
        chains: &[u8],
        relayers: &[T::AccountId],
        resources: &[(ResourceId, Vec<u8>)],
        threshold: &u32,
    ) -> DispatchResult {
        for c in chains {
            chainbridge::Pallet::<T>::whitelist(*c)?;
        }

        for rs in relayers {
            chainbridge::Pallet::<T>::register_relayer(rs.clone())?;
        }

        for (re, m) in resources.iter() {
            chainbridge::Pallet::<T>::register_resource(*re, m.clone())?;
        }

        chainbridge::Pallet::<T>::set_relayer_threshold(*threshold)
    }

    fn ensure_admin(o: T::RuntimeOrigin) -> DispatchResult {
        <T as Config>::AdminOrigin::try_origin(o)
            .map(|_| ())
            .or_else(ensure_root)?;
        Ok(())
    }
}

// Type alias for backwards compatibility
pub type Module<T> = Pallet<T>;
