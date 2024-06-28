use sp_std::convert::TryFrom;
use frame_support::{
    weights::Weight,
    traits::{Currency, OnRuntimeUpgrade, Get, ExistenceRequirement},
    sp_runtime::traits::Zero,
};
#[cfg(feature = "try-runtime")]
use frame_support::{pallet_prelude::*, inherent::Vec, sp_runtime::traits::SaturatedConversion};

use chainbridge::{Module, Config};
use xx_economics::{Module as Economics, Config as EconomicsConfig};

type BalanceOf<T> =
<<T as EconomicsConfig>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const BALANCE_TRANSFER: u128 = 40_000_000_000_000_000;
const BALANCE_LIQUIDITY_REWARDS: u128 = 10_000_000_000_000_000;

pub struct BridgeAdjust<T: Config>(sp_std::marker::PhantomData<T>);
impl<T: Config + EconomicsConfig> OnRuntimeUpgrade for BridgeAdjust<T> {
    fn on_runtime_upgrade() -> Weight {
        let reads = 2;
        let writes = 3;
        
        // Get bridge account
        let bridge_account = Module::<T>::account_id();

        // Get rewards pool account
        let rewards_account = Economics::<T>::rewards_account_id();

        // Transfer 40M from bridge to rewards
        let transfer_value = BalanceOf::<T>::try_from(BALANCE_TRANSFER).ok().unwrap_or(Zero::zero());
        <T as EconomicsConfig>::Currency::transfer(
            &bridge_account,
            &rewards_account,
            transfer_value,
            ExistenceRequirement::KeepAlive,
        ).unwrap();

        // Set liquidity rewards balance to 10M
        let amount = BalanceOf::<T>::try_from(BALANCE_LIQUIDITY_REWARDS).ok().unwrap_or(Zero::zero());
        Economics::<T>::set_liquidity_rewards_balance(frame_system::RawOrigin::Root.into(), amount).unwrap();

        log::info!(
            target: "runtime::migrations::bridge_adjust",
            "Bridge balance transfer completed, rewards balance updated, liquidity rewards balance set",
        );

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        // Get bridge account
        let bridge_account = Module::<T>::account_id();

        // Get rewards pool account
        let rewards_account = Economics::<T>::rewards_account_id();

        // Get current bridge balance
        let bridge_balance = T::Currency::free_balance(&bridge_account);

        // Get current rewards balance
        let rewards_balance = T::Currency::free_balance(&rewards_account);

        // Get total stakeable
        let total_issuance = T::Currency::total_issuance();
        let total_stakeable = Economics::<T>::compute_total_stakeable(total_issuance);

        log::info!(
            target: "runtime::migrations::bridge_adjust",
            "Pre upgrade: saved bridge balance {}, rewards balance {}, total stakeable {}",
            bridge_balance.saturated_into::<u128>(),
            rewards_balance.saturated_into::<u128>(),
            total_stakeable.saturated_into::<u128>(),
        );

        // Return values
        Ok((bridge_balance.saturated_into::<u128>(), rewards_balance.saturated_into::<u128>(), total_stakeable.saturated_into::<u128>()).encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        // Read and convert values
        let (before_bridge_balance_u128, before_rewards_balance_u128, before_total_stakeable_u128): (u128, u128, u128) =
			Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
        let before_bridge_balance = BalanceOf::<T>::try_from(before_bridge_balance_u128).ok().unwrap_or(Zero::zero());
        let before_rewards_balance = BalanceOf::<T>::try_from(before_rewards_balance_u128).ok().unwrap_or(Zero::zero());
        let before_total_stakeable = BalanceOf::<T>::try_from(before_total_stakeable_u128).ok().unwrap_or(Zero::zero());

        // Get bridge account
        let bridge_account = Module::<T>::account_id();

        // Get rewards pool account
        let rewards_account = Economics::<T>::rewards_account_id();

        // Get current bridge balance
        let bridge_balance = T::Currency::free_balance(&bridge_account);

        // Get current rewards balance
        let rewards_balance = T::Currency::free_balance(&rewards_account);

        // Get total stakeable
        let total_issuance = T::Currency::total_issuance();
        let total_stakeable = Economics::<T>::compute_total_stakeable(total_issuance);

        log::info!(
            target: "runtime::migrations::bridge_adjust",
            "Post upgrade: bridge balance {}, rewards balance {}, total stakeable {}",
            bridge_balance.saturated_into::<u128>(),
            rewards_balance.saturated_into::<u128>(),
            total_stakeable.saturated_into::<u128>(),
        );

        // Check bridge balance is decreased by 40M
        let transfer_value = BalanceOf::<T>::try_from(BALANCE_TRANSFER).ok().unwrap_or(Zero::zero());
        assert!(before_bridge_balance - transfer_value == bridge_balance, "Bridge balance mismatch after bridge transfer");

        // Check rewards balance is increased by 40M
        assert!(before_rewards_balance + transfer_value == rewards_balance, "Rewards balance mismatch after bridge transfer");

        // Check that total stakeable is unchanged 
        assert!(before_total_stakeable == total_stakeable, "Total stakeable is not the same after bridge transfer");

        // Check that liquidity rewards balance is set to 10M
        let liquidity_rewards_balance = Economics::<T>::liquidity_rewards();
        let amount = BalanceOf::<T>::try_from(BALANCE_LIQUIDITY_REWARDS).ok().unwrap_or(Zero::zero());
        assert!(liquidity_rewards_balance == amount, "Liquidity rewards balance mismatch after bridge transfer");
        
        log::info!(
            target: "runtime::migrations::bridge_adjust",
            "Post upgrade: checks completed, adjustment success",
        );
        Ok(())
    }
}
