// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use super::*;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

// all methods in XXEconomics are basic admin get/set of variables
// so the benchmarks are trivial

benchmarks! {
	 set_inflation_params {
	 }: _(RawOrigin::Root, Default::default())

	 set_interest_points {
	 }: _(RawOrigin::Root, Default::default())

	 set_liquidity_rewards_stake {
	 }: _(RawOrigin::Root, Default::default())

	set_liquidity_rewards_balance {
	 }: _(RawOrigin::Root, Default::default())
}

impl_benchmark_test_suite!(
	XXEconomics,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Test,
);
