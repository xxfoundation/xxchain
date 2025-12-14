//! Custody functionality for xx-team-custody pallet.

use crate::{
    Config, BalanceOf, Pallet, Error, Event,
    TeamAccounts, CustodyAccounts, TotalCustody
};
use sp_runtime::Perbill;
use core::convert::TryFrom;
use codec::{Encode, Decode, HasCompact};
use frame_system::pallet_prelude::BlockNumberFor;

use frame_support::traits::{
    Currency, Get, ExistenceRequirement::AllowDeath,
    fungible::Inspect, ReservableCurrency,
};
use sp_runtime::traits::{
    Zero, Saturating, AtLeast32BitUnsigned,
    Convert, SaturatedConversion
};
use frame_support::dispatch::DispatchResult;
use scale_info::TypeInfo;

/// Custody Info
#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug, TypeInfo, frame_support::pallet_prelude::MaxEncodedLen)]
pub struct CustodyInfo<AccountId, Balance: HasCompact + frame_support::pallet_prelude::MaxEncodedLen> {
    /// Allocation
    #[codec(compact)]
    pub allocation: Balance,
    /// Current vested amount (increases as payouts are given)
    #[codec(compact)]
    pub vested: Balance,
    /// Custody account
    pub custody: AccountId,
    /// Reserve account
    pub reserve: AccountId,
}

impl<AccountId, Balance> CustodyInfo<AccountId, Balance> where
    Balance: AtLeast32BitUnsigned + Saturating + Copy + HasCompact + frame_support::pallet_prelude::MaxEncodedLen,
{
    /// Increase vested amount in self
    fn increase_vested(&mut self, amount: Balance) {
        self.vested += amount;
    }
}

/// Get payout frequency
pub fn payout_frequency<T: Config>() -> BlockNumberFor<T> {
    T::PayoutFrequency::get()
}

/// Get the custody duration
pub fn custody_duration<T: Config>() -> BlockNumberFor<T> {
    T::CustodyDuration::get()
}

/// Get the governance custody duration
pub fn governance_custody_duration<T: Config>() -> BlockNumberFor<T> {
    T::GovernanceCustodyDuration::get()
}

/// Check if custody is done
pub fn is_custody_done<T: Config>(block: BlockNumberFor<T>) -> bool {
    block > custody_duration::<T>()
}

/// Check if governance custody is done
pub fn is_governance_custody_done<T: Config>(block: BlockNumberFor<T>) -> bool {
    block > governance_custody_duration::<T>()
}

/// Initialize custody for a given account and allocation
pub fn initialize_custody<T: Config>(who: &T::AccountId, amount: BalanceOf<T>) {
    // 1. Split allocation: 5.55555556% -> reserve, 94.4444444% -> custody
    // NOTE: allocation here is 90% of actual team member allocation, so the reserve
    // has 5.55555556% * 0.9 = 5% of total funds
    let reserve_ratio = Perbill::from_rational(55_555_556u32, 1_000_000_000u32);
    let reserve_balance = reserve_ratio * (amount);
    let custody_balance = amount - reserve_balance;

    // 2. Create custody account
    // Generate custody account for team member using
    // the anonymous account function from proxy pallet
    let custody_account =
        <pallet_proxy::Pallet<T>>::pure_account(
            who,
            &T::CustodyProxy::get(),
            0,
            None
        );

    // Deposit amount in custody account
    // The imbalance must be dropped to update TotalIssuance
    let imbalance = <T as Config>::Currency::deposit_creating(
        &custody_account,
        custody_balance
    );
    drop(imbalance);

    // 3. Create reserve account
    // Generate reserve account for team member using
    // the anonymous account function from proxy pallet
    let reserve_account =
        <pallet_proxy::Pallet<T>>::pure_account(
            who,
            &T::CustodyProxy::get(),
            1,
            None
        );

    // Deposit amount in reserve account
    // The imbalance must be dropped to update TotalIssuance
    let imbalance = <T as Config>::Currency::deposit_creating(
        &reserve_account,
        reserve_balance
    );
    drop(imbalance);

    // 4. Create custody info
    let custody_info = CustodyInfo {
        allocation: amount,
        vested: Zero::zero(),
        custody: custody_account.clone(),
        reserve: reserve_account,
    };

    // 5. Store custody info and custody account
    TeamAccounts::<T>::insert(who, custody_info);
    CustodyAccounts::<T>::insert(&custody_account, ());

    // 6. Update total amount under custody
    TotalCustody::<T>::mutate(|n| *n += custody_balance);
    TotalCustody::<T>::mutate(|n| *n += reserve_balance);
}

/// Set the governance proxy of given custody account
fn set_custody_governance_proxy<T: Config>(custody: &T::AccountId, proxy: T::AccountId) -> DispatchResult {
    // 1. Remove any proxies
    remove_custody_proxies::<T>(custody);
    // 2. Set new proxy
    <pallet_proxy::Pallet<T>>::add_proxy_delegate(
        custody,
        proxy,
        T::CustodyProxy::get(),
        Zero::zero()
    )
}

/// Remove any proxies of given custody account, refunding team member's deposit if existing
/// NOTE: Any proxies are guaranteed to be only of Governance type
fn remove_custody_proxies<T: Config>(custody: &T::AccountId) {
    // Can't call remove_proxies directly, so need to replicate code here
    let (_, old_deposit) = <pallet_proxy::Proxies<T>>::take(custody);
    <T as pallet_proxy::Config>::Currency::unreserve(custody, old_deposit.clone());
}

/// Compute payout
fn compute_payout<T: Config>(allocation: BalanceOf<T>) -> BalanceOf<T> {
    let payout_ratio = Perbill::from_rational(
        payout_frequency::<T>(),
        custody_duration::<T>()
    );
    payout_ratio * allocation
}

/// Attempt a payout to the given team member account
pub fn try_payout<T: Config>(who: T::AccountId) -> DispatchResult {
    // 1. Get the block number from the FRAME System module.
    let block = <frame_system::Pallet<T>>::block_number();

    // 2. Get custody info for team member
    // (can't fail because team member existing is checked before)
    let info = TeamAccounts::<T>::get(&who).unwrap();

    // 3. If custody is over, payout full remaining amount
    if is_custody_done::<T>(block) {
        // 3.1. If any leftover custody amount is still bonded, force unstake
        let custody = info.custody.clone();
        if <pallet_staking::Bonded<T>>::contains_key(&custody) {
            <pallet_staking::Pallet<T>>::force_unstake(
                frame_system::RawOrigin::Root.into(),
                custody.clone(),
                Zero::zero(),
            )?;
        }
        // 3.2. Remove any proxies on the custody account
        remove_custody_proxies::<T>(&custody);
        // 3.3. Payout remaining amount
        do_payout::<T>(who.clone(), Zero::zero(), info, false)?;
        // 3.4. Emit custody done event
        Pallet::<T>::deposit_event(Event::CustodyDone { who });
        return Ok(())
    }

    // 4. Compute payout according to block
    let payout = compute_payout::<T>(info.allocation);
    let chunks = block / payout_frequency::<T>();
    let chunks = T::BlockNumberToBalance::convert(chunks);
    let amount = payout * chunks - info.vested;

    if !amount.is_zero() {
        // 4.1. Do payout
        // NOTE: function does nothing if custody account has only the existential deposit
        // transferable balance and reserve funds are exhausted
        do_payout::<T>(who.clone(), amount, info, true)?;
    } else {
        // 4.2. Payout not available
        Err(Error::<T>::PayoutNotAvailable)?
    }
    Ok(())
}

/// Do a payout
fn do_payout<T: Config>(
    who: T::AccountId,
    amount: BalanceOf<T>,
    mut info: CustodyInfo<T::AccountId, BalanceOf<T>>,
    keep_alive: bool,
) -> DispatchResult {
    // 1. Get custody transferable balance
    // Use Inspect trait here to get transferable balance of Custody account, and use keep
    // alive to limit transfers down to existential deposit until end of the custody period
    let custody = info.custody.clone();
    let preservation = if keep_alive {
        frame_support::traits::tokens::Preservation::Preserve
    } else {
        frame_support::traits::tokens::Preservation::Expendable
    };
    let custody_transferable_balance =
        <T as Config>::Currency::reducible_balance(&custody, preservation, frame_support::traits::tokens::Fortitude::Polite);
    // T::Currency and T::Inspect are both implemented by Balances pallet, so the
    // balance type is the same. However, explicit conversion is needed here.
    let custody_balance = <BalanceOf<T>>::try_from(
            custody_transferable_balance.saturated_into::<u128>()
        ).ok().unwrap_or(Zero::zero());

    // 2. Get reserve balance
    // Reserve account is never used in any Reservable or Lockable Currency operations
    // However, if funds are taken from the Reserve, we could have a payout that
    // leaves dust in the account. This will lead to loss of custody funds, meaning the
    // team member never fully vests.
    // In order to prevent this, use the Inspect trait here, and use keep alive to
    // limit transfers down to existential deposit until end of the custody period
    let reserve = info.reserve.clone();
    let reserve_transferable_balance =
        <T as Config>::Currency::reducible_balance(&reserve, preservation, frame_support::traits::tokens::Fortitude::Polite);
    // T::Currency and T::Inspect are both implemented by Balances pallet, so the
    // balance type is the same. However, explicit conversion is needed here.
    let reserve_balance = <BalanceOf<T>>::try_from(
            reserve_transferable_balance.saturated_into::<u128>()
        ).ok().unwrap_or(Zero::zero());

    // 3. Calculate amounts to withdraw from custody and reserve
    // If custody period is done, transfer full amount from both accounts
    // in order to not leave any inaccessible funds around
    let (withdraw_custody, withdraw_reserve) = if keep_alive {
        let from_custody = amount.min(custody_balance);
        let from_reserve = amount - from_custody;
        (from_custody, from_reserve.min(reserve_balance))
    } else {
        (custody_balance, reserve_balance)
    };
    let withdraw = withdraw_custody + withdraw_reserve;

    // 4. Make transfer from custody, if possible
    if !withdraw_custody.is_zero() {
        // Transfer from custody to team member account
        <T as Config>::Currency::transfer(
            &custody,
            &who,
            withdraw_custody,
            AllowDeath
        )?;
        // Emit TeamPayoutCustody event
        Pallet::<T>::deposit_event(Event::PayoutFromCustody { who: who.clone(), amount: withdraw_custody });
    }

    // 5. Make transfer from reserve, if possible
    if !withdraw_reserve.is_zero() {
        // Transfer from reserve to team member account
        <T as Config>::Currency::transfer(
            &reserve,
            &who,
            withdraw_reserve,
            AllowDeath
        )?;
        // Emit TeamPayoutReserve event
        Pallet::<T>::deposit_event(Event::PayoutFromReserve { who: who.clone(), amount: withdraw_reserve });
    }

    // 6. Error if no payout is possible (only before custody ends)
    if keep_alive && withdraw.is_zero() {
        Err(Error::<T>::PayoutFailedInsufficientFunds)?
    }

    // 7. Update custody info
    // Increase vested amount
    info.increase_vested(withdraw);

    // Adjust total custody
    let custody_deduct = if keep_alive {
        // If its not the last payout, then use the total withdraw
        withdraw
    } else {
        // At the last payout, make sure to square total custody correctly based
        // on difference between vested and allocation
        if info.allocation < info.vested {
            // If more was paid out than was allocated, make sure to deduct the difference
            let diff = info.vested - info.allocation;
            withdraw.saturating_sub(diff)
        } else if info.allocation > info.vested {
            // If not enough was paid out compared to allocation, make sure to add the difference
            let diff = info.allocation - info.vested;
            withdraw.saturating_add(diff)
        } else {
            // Total paid equals allocation, all good
            withdraw
        }
    };

    // Note: The logic above has been tested and results in exact amounts being subtracted
    // from TotalCustody. Still, use saturating sub here just to be safe and not have panics
    TotalCustody::<T>::mutate(|n| *n = n.clone().saturating_sub(custody_deduct));

    // Update or delete info in storage
    if keep_alive {
        // Insert updated custody info
        TeamAccounts::<T>::insert(&who, info);
    } else {
        // Delete team member custody info
        TeamAccounts::<T>::remove(&who);
        // Delete custody account
        CustodyAccounts::<T>::remove(&info.custody);
    }

    Ok(())
}

/// Check if the custody period is active, return error if not
fn check_custody<T: Config>() -> DispatchResult {
    // Get the block number from the FRAME System module.
    let block = <frame_system::Pallet<T>>::block_number();
    // If custody should be active but is done, return error
    if is_custody_done::<T>(block) {
        Err(Error::<T>::CustodyPeriodEnded)?
    }
    Ok(())
}

/// Check if the governance custody period is active/done, return appropriate error
fn check_governance_custody<T: Config>(active: bool) -> DispatchResult {
    // Get the block number from the FRAME System module.
    let block = <frame_system::Pallet<T>>::block_number();
    if active {
        // If governance custody should be active but is done, return error
        if is_governance_custody_done::<T>(block) {
            Err(Error::<T>::GovernanceCustodyPeriodEnded)?
        }
    } else {
        // If governance custody should be done but is active, return error
        if !is_governance_custody_done::<T>(block) {
            Err(Error::<T>::GovernanceCustodyActive)?
        }
    }
    Ok(())
}

/// Attempt to bond funds from a custody account
/// NOTE: In SDK 2509+, controller is deprecated - stash is the controller
pub fn try_custody_bond<T: Config>(
    custody: T::AccountId,
    _controller: T::AccountId,
    value: pallet_staking::BalanceOf<T>,
) -> DispatchResult {
    // 1. Return error if custody done
    check_custody::<T>()?;

    // 2. Call bond function (controller deprecated - stash is controller now)
    <pallet_staking::Pallet<T>>::bond(
        T::RuntimeOrigin::from(Some(custody).into()),
        value.into(),
        pallet_staking::RewardDestination::Stash,
    )
}

/// Attempt to bond extra funds from a custody account
pub fn try_custody_bond_extra<T: Config>(
    custody: T::AccountId,
    value: pallet_staking::BalanceOf<T>,
) -> DispatchResult {
    // 1. Return error if custody done
    check_custody::<T>()?;

    // 2. Call bond extra function
    <pallet_staking::Pallet<T>>::bond_extra(
        T::RuntimeOrigin::from(Some(custody).into()),
        value.into()
    )
}

/// Attempt to set the staking controller of a custody account
/// NOTE: In SDK 2509+, controller is deprecated - stash is the controller.
/// This function is now a no-op for backwards compatibility.
pub fn try_custody_set_controller<T: Config>(
    _custody: T::AccountId,
    _controller: T::AccountId,
) -> DispatchResult {
    // Controller is deprecated in SDK 2509+ - stash is now the controller
    // This function is kept for API compatibility but does nothing
    Ok(())
}

/// Attempt to set a governance proxy of a given custody account
pub fn try_custody_set_proxy<T: Config>(
    custody: T::AccountId,
    proxy: T::AccountId,
) -> DispatchResult {
    // 1. Return error if governance custody done
    check_governance_custody::<T>(true)?;

    // 2. Set new governance proxy (removes any previous existing ones)
    set_custody_governance_proxy::<T>(&custody, proxy)
}

/// Attempt to set a governance proxy of a team member's own custody account
pub fn try_team_custody_set_proxy<T: Config>(who: T::AccountId, proxy: T::AccountId) -> DispatchResult {
    // 1. Return error if governance custody is not done
    check_governance_custody::<T>(false)?;

    // 2. Get team member custody account
    // (can't fail because team member existing is checked before)
    let info = TeamAccounts::<T>::get(&who).unwrap();

    // 3. Set new governance proxy (removes any previous existing ones)
    set_custody_governance_proxy::<T>(&info.custody, proxy)
}

/// Update a team member account
pub fn update_team_member<T: Config>(who: T::AccountId, new: T::AccountId) {
    // 1. Take info from team accounts
    let info = TeamAccounts::<T>::take(&who).unwrap();
    // 2. Insert info in new account
    TeamAccounts::<T>::insert(&new, info);
}
