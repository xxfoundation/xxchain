#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod rewards;
pub mod inflation;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use alloc::vec::Vec;

use frame_support::traits::{Currency, Get, EnsureOrigin, OnUnbalanced};
use frame_support::{dispatch::DispatchResult, PalletId};
pub use weights::WeightInfo;
use frame_system::ensure_root;

pub use pallet::*;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::PositiveImbalance;

pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The Event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency mechanism.
        type Currency: Currency<Self::AccountId>;

        /// Handler to retrieve public accounts
        type PublicAccountsHandler: xx_public::PublicAccountsHandler<Self::AccountId>;

        //---------------- REWARDS POOL ----------------//

        /// The RewardsPool sub component id, used to derive its account ID.
        type RewardsPoolId: Get<PalletId>;

        /// The reward remainder handler (Treasury).
        type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

        //----------------   INFLATION  ----------------//

        /// Era duration needed for ideal inflation computation.
        type EraDuration: Get<BlockNumberFor<Self>>;

        /// The admin origin for the pallet (Tech Committee unanimity).
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        //---------------- REWARDS POOL ----------------//

        /// Rewards were given from the pool
        RewardFromPool(BalanceOf<T>),
        /// Rewards were minted
        RewardMinted(BalanceOf<T>),

        //----------------  INFLATION   ----------------//

        /// Inflation fixed parameters were changed
        InflationParamsChanged,
        /// Ideal interest points were changed
        InterestPointsChanged,
        /// Ideal liquidity rewards stake was changed
        IdealLiquidityStakeChanged,
        /// Liquidity rewards balance was changed
        LiquidityRewardsBalanceChanged,
    }

    /// Inflation fixed parameters: minimum inflation, ideal stake and curve falloff
    #[pallet::storage]
    #[pallet::getter(fn inflation_params)]
    pub type InflationParams<T> =
        StorageValue<_, inflation::InflationFixedParams, ValueQuery>;

    /// List of ideal interest points, defined as a tuple of block number and ideal interest
    #[pallet::storage]
    #[pallet::getter(fn interest_points)]
    pub type InterestPoints<T: Config> =
        StorageValue<_, Vec<inflation::IdealInterestPoint<BlockNumberFor<T>>>, ValueQuery>;

    /// Ideal liquidity rewards staked amount
    #[pallet::storage]
    #[pallet::getter(fn ideal_stake_rewards)]
    pub type IdealLiquidityStake<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Liquidity rewards balance
    #[pallet::storage]
    #[pallet::getter(fn liquidity_rewards)]
    pub type LiquidityRewards<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub inflation_params: inflation::InflationFixedParams,
        pub interest_points: Vec<inflation::IdealInterestPoint<BlockNumberFor<T>>>,
        pub ideal_liquidity_stake: BalanceOf<T>,
        pub liquidity_rewards: BalanceOf<T>,
        pub balance: BalanceOf<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Set inflation params
            InflationParams::<T>::put(&self.inflation_params);

            // Sort points when building from genesis
            let mut points = self.interest_points.clone();
            points.sort_by(|a, b| a.block.cmp(&b.block));
            InterestPoints::<T>::put(points);

            // Set ideal liquidity stake
            IdealLiquidityStake::<T>::put(&self.ideal_liquidity_stake);

            // Set liquidity rewards
            LiquidityRewards::<T>::put(&self.liquidity_rewards);

            // Create Rewards pool account and set the balance from genesis
            let account_id = Pallet::<T>::rewards_account_id();
            let _ = <T as Config>::Currency::make_free_balance_be(&account_id, self.balance);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set inflation fixed parameters
        ///
        /// The dispatch origin must be AdminOrigin.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::set_inflation_params())]
        pub fn set_inflation_params(
            origin: OriginFor<T>,
            params: inflation::InflationFixedParams,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            InflationParams::<T>::put(params);
            Self::deposit_event(Event::InflationParamsChanged);
            Ok(())
        }

        /// Set ideal interest points
        ///
        /// Overwrites the full list of points. Doesn't check if points are ordered per block.
        /// It's up to the caller to ensure the ordering, otherwise leads to unexpected behavior.
        ///
        /// The dispatch origin must be AdminOrigin.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::set_interest_points())]
        pub fn set_interest_points(
            origin: OriginFor<T>,
            points: Vec<inflation::IdealInterestPoint<BlockNumberFor<T>>>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            // Insert sorted vector of points
            let mut sorted_points = points.clone();
            sorted_points.sort_by(|a, b| a.block.cmp(&b.block));
            InterestPoints::<T>::put(sorted_points);
            Self::deposit_event(Event::InterestPointsChanged);
            Ok(())
        }

        /// Set ideal liquidity rewards stake amount
        ///
        /// The dispatch origin must be AdminOrigin.
        /// This can be used to adjust the ideal liquidity reward stake
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::set_liquidity_rewards_stake())]
        pub fn set_liquidity_rewards_stake(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            IdealLiquidityStake::<T>::put(amount);
            Self::deposit_event(Event::IdealLiquidityStakeChanged);
            Ok(())
        }

        /// Set balance of liquidity rewards
        ///
        /// The dispatch origin must be AdminOrigin.
        /// This should only be used to make corrections to liquidity rewards balance
        /// according to data from ETH chain
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::set_liquidity_rewards_balance())]
        pub fn set_liquidity_rewards_balance(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            LiquidityRewards::<T>::put(amount);
            Self::deposit_event(Event::LiquidityRewardsBalanceChanged);
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Check if origin is admin
    fn ensure_admin(o: T::RuntimeOrigin) -> DispatchResult {
        <T as Config>::AdminOrigin::try_origin(o)
            .map(|_| ())
            .or_else(ensure_root)?;
        Ok(())
    }
}

// Type alias for backwards compatibility
pub type Module<T> = Pallet<T>;
