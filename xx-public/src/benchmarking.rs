use super::*;
use crate::Module as XXPublic;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, account};
use frame_system::RawOrigin;
use frame_support::traits::OriginTrait;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::Zero;

const SEED: u32 = 0;

fn account_from_index<T: Config>(index: u32) -> T::AccountId {
    account("x", index, SEED)
}

fn set_testnet_manager<T: Config>(account: T::AccountId) {
    XXPublic::<T>::set_testnet_manager_account(T::RuntimeOrigin::root(), account).ok();
}

fn set_sale_manager<T: Config>(account: T::AccountId) {
    XXPublic::<T>::set_sale_manager_account(T::RuntimeOrigin::root(), account).ok();
}

const MAX_DISTRIBUTIONS: u32 = 100;
const EXPECTED_SCHEDULES: u32 = 4;

benchmarks!{
    set_testnet_manager_account {

    }: _(RawOrigin::Root, account_from_index::<T>(42))

    set_sale_manager_account {

    }: _(RawOrigin::Root, account_from_index::<T>(43))

    // Real case all distributions might have up to 4 vesting schedules
    testnet_distribute {
        let n in 1 .. MAX_DISTRIBUTIONS;
        let amount = <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::minimum_balance() * 25u32.into();
        let vest = <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::minimum_balance() * 1u32.into();
        let block = BlockNumberFor::<T>::zero();
        let mut scheds = Vec::<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>::new();
        for _ in 1 .. EXPECTED_SCHEDULES {
            scheds.push((vest.clone(), vest.clone(), block.clone()))
        }

        let manager =  account_from_index::<T>(42);
        set_testnet_manager::<T>(manager.clone());

        let mut distribution = Vec::<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>::new();
        for i in 0 .. n {
            let data = TransferData::<T::AccountId, BalanceOf<T>, BlockNumberFor<T>> {
                destination: account_from_index::<T>(i),
                amount: amount.clone(),
                schedules: Some(scheds.clone()),
            };
            distribution.push(data)
        }

    }: _(RawOrigin::Signed(manager), distribution)

    sale_distribute {
        let n in 1 .. MAX_DISTRIBUTIONS;
        let amount = <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::minimum_balance() * 25u32.into();
        let vest = <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::minimum_balance() * 1u32.into();
        let block = BlockNumberFor::<T>::zero();
        let mut scheds = Vec::<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>::new();
        for _ in 1 .. EXPECTED_SCHEDULES {
            scheds.push((vest.clone(), vest.clone(), block.clone()))
        }

        let manager =  account_from_index::<T>(42);
        set_sale_manager::<T>(manager.clone());

        let mut distribution = Vec::<TransferData<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>>::new();
        for i in 0 .. n {
            let data = TransferData::<T::AccountId, BalanceOf<T>, BlockNumberFor<T>> {
                destination: account_from_index::<T>(i),
                amount: amount.clone(),
                schedules: Some(scheds.clone()),
            };
            distribution.push(data)
        }

    }: _(RawOrigin::Signed(manager), distribution)
}

impl_benchmark_test_suite!(
  XXPublic,
  crate::mock::ExtBuilder::default()
  	.build(),
  crate::mock::Test,
);
