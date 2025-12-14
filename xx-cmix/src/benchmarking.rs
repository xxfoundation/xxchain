// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use super::*;
use crate::Module as XXCmix;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::OriginTrait;
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use sp_runtime::traits::Bounded;

const SEED: u32 = 0;

const MAX_POINTS: u32 = 99;
const MAX_DEDUCTIONS: u32 = 99;

fn account_from_index<T: Config>(index: u32) -> T::AccountId {
	account("x", index, SEED)
}

fn set_scheduler<T: Config>(account: T::AccountId) {
	XXCmix::<T>::set_scheduling_account(T::RuntimeOrigin::root(), account).ok();
}

// sets the admin permission way into the future so we can call certain extrinsics
fn set_admin<T: Config>() {
	let block = BlockNumberFor::<T>::max_value();
	XXCmix::<T>::set_admin_permission(T::RuntimeOrigin::root(), block).ok();
}

benchmarks! {

	set_cmix_hashes {
		set_admin::<T>();
	}: _(RawOrigin::Root, cmix::SoftwareHashes::default())


	set_scheduling_account {
		let scheduler = account_from_index::<T>(99);
	}: _(RawOrigin::Root, scheduler)


	set_next_cmix_variables {

	}: _(RawOrigin::Root, cmix::Variables::default())


	submit_cmix_points {
		let n in 1 .. MAX_POINTS;

		let scheduler = account_from_index::<T>(0);
		set_scheduler::<T>(scheduler.clone());

		let mut points = Vec::<(T::AccountId, u32)>::new();
		for i in 0 .. n {
			points.push((account_from_index::<T>(i), 10_u32))
		}

	}: _(RawOrigin::Signed(scheduler), points)


	submit_cmix_deductions {
		let n in 1 .. MAX_DEDUCTIONS;

		let scheduler = account_from_index::<T>(0);
		set_scheduler::<T>(scheduler.clone());

		let mut deductions = Vec::<(T::AccountId, u32)>::new();
		for i in 0 .. n {
			deductions.push((account_from_index::<T>(i), 10_u32))
		}

	}: _(RawOrigin::Signed(scheduler), deductions)


	set_cmix_address_space {
		let scheduler = account_from_index::<T>(0);
		set_scheduler::<T>(scheduler.clone());
	}: _(RawOrigin::Signed(scheduler), 0x66)


	set_admin_permission {

	}: _(RawOrigin::Root, BlockNumberFor::<T>::max_value())

}

impl_benchmark_test_suite!(XXCmix, crate::mock::ExtBuilder::default().build(), crate::mock::Test,);
