// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use super::*;

use frame_benchmarking::{benchmarks, account, impl_benchmark_test_suite, BenchmarkError};
use frame_system::RawOrigin;
use frame_support::dispatch::UnfilteredDispatchable;
use sp_runtime::traits::Bounded;

const SEED: u32 = 0;

const TEST_DESTINATION_CHAIN: chainbridge::ChainId = 0xff;
const TEST_RECIPIENT_ADDR: &[u8] = &[0; 32]; // mock address on destination chain

fn account_from_index<T: Config>(index: u32) -> T::AccountId {
	account("x", index, SEED)
}

benchmarks!{

	transfer_native {
		// worst case should result in a call into the chainbridge pallet
		// transfer_fungible
		// - chain whitelisted
		// - account has balance for fee and transfer

		<chainbridge::Module<T>>::whitelist_chain(RawOrigin::Root.into(), TEST_DESTINATION_CHAIN)
			.expect("Could not whitelist chain");

		let payer = account_from_index::<T>(5);
		let initial_balance = <<T as Config>::Currency as Currency<T::AccountId>>::Balance::max_value();
		T::Currency::make_free_balance_be(&payer, initial_balance);

		let amount = T::Currency::minimum_balance() * 100u32.into();

	}: _(RawOrigin::Signed(payer), amount, TEST_RECIPIENT_ADDR.to_vec(), TEST_DESTINATION_CHAIN)


 	transfer {
		let amount = T::Currency::minimum_balance() * 10u32.into();
		let dest = account_from_index::<T>(1);
		let origin = T::BridgeOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let call = Call::<T>::transfer {
			to: dest,
			amount,
		};

	}: { call.dispatch_bypass_filter(origin)? }


	set_swap_fee {
		let new_fee = T::Currency::minimum_balance() * 10u32.into();
	}: _(RawOrigin::Root, new_fee)


	set_fee_destination {
	}: _(RawOrigin::Root, account_from_index::<T>(1))
}


impl_benchmark_test_suite!(
  Swap,
  crate::mock::new_test_ext(&[]),
  crate::mock::Test,
);
