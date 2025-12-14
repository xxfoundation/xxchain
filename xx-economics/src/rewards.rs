use super::{Config, Pallet, pallet::Event, BalanceOf, PositiveImbalanceOf, NegativeImbalanceOf};
use frame_support::traits::{
    Currency, OnUnbalanced, Imbalance, Get,
    WithdrawReasons, ExistenceRequirement::AllowDeath
};
use sp_runtime::traits::{Zero, AccountIdConversion};

/// Implement Rewards Pool sub module functions
impl<T: Config> Pallet<T> {
    /// Get the AccountId for the Rewards pool
    pub fn rewards_account_id() -> T::AccountId {
        T::RewardsPoolId::get().into_account_truncating()
    }

    /// Get current balance of Rewards pool account
    pub fn rewards_balance() -> BalanceOf<T> {
        <T as Config>::Currency::free_balance(&Self::rewards_account_id())
    }

    /// Withdraw from the Rewards pool and Emit event
    fn withdraw(amount: PositiveImbalanceOf<T>) {
        let numeric_amount = amount.peek();
        if numeric_amount.is_zero() { return }
        let _ = <T as Config>::Currency::settle(
            &Self::rewards_account_id(),
            amount,
            WithdrawReasons::TRANSFER,
            AllowDeath,
        );
        Self::deposit_event(Event::RewardFromPool(numeric_amount));
    }

    /// Emit event with amount of coins minted
    fn mint_event(amount: BalanceOf<T>) {
        if amount.is_zero() { return }
        Self::deposit_event(Event::RewardMinted(amount));
    }
}

/// Implement OnUnbalanced trait for PositiveImbalance, to handle validator rewards
impl<T: Config> OnUnbalanced<PositiveImbalanceOf<T>> for Pallet<T> {
    fn on_nonzero_unbalanced(amount: PositiveImbalanceOf<T>) {
        // Get current rewards account balance
        let balance = Self::rewards_balance();

        // Split imbalance into withdraw and mint parts
        let (withdraw, mint) = amount.split(balance);

        // Withdraw funds from pool (only if != 0)
        Self::withdraw(withdraw);

        // Create mint event (only if != 0)
        Self::mint_event(mint.peek());

        // The mint imbalance will square up total issuance when dropped after leaving function
    }
}

/// Use an adapter to implement OnUnbalanced trait for NegativeImbalance
/// to handle rewards remainder
pub struct RewardRemainderAdapter<T>(core::marker::PhantomData<T>);
impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for RewardRemainderAdapter<T> {
    fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
        // Get current rewards account balance
        let balance = Pallet::<T>::rewards_balance();

        // Peek amount
        let numeric_amount = amount.peek();

        // Produce events and withdraw correct amount from the pool
        let withdraw = numeric_amount.min(balance);
        let mint = numeric_amount - withdraw;

        // Create burn imbalance (no-op if zero)
        let imbalance = <T as Config>::Currency::burn(withdraw);

        // Withdraw funds from pool (only if != 0)
        Pallet::<T>::withdraw(imbalance);

        // Create mint event (only if != 0)
        Pallet::<T>::mint_event(mint);

        // Pass the imbalance to the reward remainder handler to actually deposit funds
        <T as Config>::RewardRemainder::on_unbalanced(amount);
    }
}
