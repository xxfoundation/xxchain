// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use crate::cmix::{Scheduling, SoftwareHashes};

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

type SoftwareHash = <mock::Test as frame_system::Config>::Hash;

///////////////////////////////
//      set_cmix_hashes      //
///////////////////////////////

#[test]
fn set_cmix_hashes_can_call_with_admin_during_permission_period() {
	ExtBuilder::default().with_admin_permission(10).build_and_execute(|| {
		run_to_block(10); // admin period is inclusive
		let new_hashes =
			SoftwareHashes { server: SoftwareHash::repeat_byte(0x55), ..Default::default() };
		assert_ok!(XXCmix::set_cmix_hashes(RuntimeOrigin::root(), new_hashes.clone()));

		assert_eq!(XXCmix::cmix_hashes(), new_hashes,);
		assert_eq!(*xx_cmix_events().last().unwrap(), Event::CmixHashesUpdated);
	});
}

#[test]
fn set_cmix_hashes_fails_outside_permission_period() {
	ExtBuilder::default().with_admin_permission(10).build_and_execute(|| {
		run_to_block(11);
		assert_noop!(
			XXCmix::set_cmix_hashes(RuntimeOrigin::root(), Default::default()),
			Error::<Test>::AdminPermissionExpired,
		);
	});
}

////////////////////////////////////
//    set_scheduling_account      //
////////////////////////////////////

#[test]
fn set_scheduling_account_can_call_with_admin_during_permission_period() {
	let new_scheduling_account = 1;
	ExtBuilder::default().with_admin_permission(10).build_and_execute(|| {
		run_to_block(10);
		assert_ok!(XXCmix::set_scheduling_account(RuntimeOrigin::root(), new_scheduling_account));
		assert_eq!(XXCmix::scheduling_account().unwrap(), new_scheduling_account);
		assert_eq!(*xx_cmix_events().last().unwrap(), Event::SchedulingAccountUpdated);
	});
}

////////////////////////////////////
//    set_next_cmix_variables      //
////////////////////////////////////

#[test]
fn set_next_cmix_variables_can_call_with_cmix_vars_origin() {
	let new_variables = cmix::Variables {
		scheduling: Scheduling { team_size: 4, ..Default::default() },
		..Default::default()
	};
	ExtBuilder::default().build_and_execute(|| {
		start_active_era(1);

		let init_cmix_vars = XXCmix::cmix_variables();
		assert_ok!(XXCmix::set_next_cmix_variables(RuntimeOrigin::root(), new_variables.clone()));
		assert_eq!(XXCmix::next_cmix_variables(), Some(new_variables.clone()),);
		// actual variables have not been set yet
		// Not set until next era
		assert_eq!(XXCmix::cmix_variables(), init_cmix_vars);

		// In the next era they are set
		start_active_era(2);

		assert_eq!(XXCmix::cmix_variables(), new_variables);
		assert_eq!(*xx_cmix_events().last().unwrap(), Event::CmixVariablesUpdated);
	});
}

////////////////////////////////////////////////////////
//    submit_cmix_points / submit_cmix_deductions     //
////////////////////////////////////////////////////////

#[test]
fn cmix_points_fails_if_not_scheduling() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXCmix::submit_cmix_points(RuntimeOrigin::signed(1), Vec::new()),
			Error::<Test>::MustBeScheduling,
		);
		assert_noop!(
			XXCmix::submit_cmix_deductions(RuntimeOrigin::signed(1), Vec::new()),
			Error::<Test>::MustBeScheduling,
		);
	});
}

#[test]
fn cmix_points_adds_remove_points_in_staking_pallet() {
	let scheduling = 1;
	let a = 2;
	let first_addition = 10;
	let second_addition = 33;

	ExtBuilder::default().with_scheduling_account(scheduling).build_and_execute(|| {
		start_active_era(1);

		assert_ok!(XXCmix::submit_cmix_points(
			RuntimeOrigin::signed(scheduling),
			vec![(a, first_addition)]
		),);
		assert_ok!(XXCmix::submit_cmix_points(
			RuntimeOrigin::signed(scheduling),
			vec![(a, second_addition)]
		),);
		// Note: In the new SDK, validators don't start with an implicit +1 point
		assert_eq!(
			Staking::eras_reward_points(active_era()).individual.get(&a),
			Some(&(first_addition + second_addition))
		);

		// now deduct. In the new SDK, deductions are stored separately
		// instead of modifying ErasRewardPoints directly
		assert_ok!(XXCmix::submit_cmix_deductions(
			RuntimeOrigin::signed(scheduling),
			vec![(a, 99)]
		),);
		// Check that deductions were recorded in xx-staking-extension
		let deductions = XXStakingExtension::eras_deduction_points(active_era());
		let deducted_points = deductions.individual.get(&a).copied().unwrap_or(0);
		// Net points = reward_points - deduction_points
		// The deduction of 99 should be recorded. Since we only have 43 points,
		// the net would be max(1, 43 - 99) = 1 when used in rewards calculation.
		// But the deduction storage just records the deduction request.
		assert_eq!(deducted_points, 99);
		// Original reward points unchanged in new architecture
		assert_eq!(
			Staking::eras_reward_points(active_era()).individual.get(&a),
			Some(&(first_addition + second_addition))
		);

		assert_eq!(
			xx_cmix_events(),
			vec![Event::CmixPointsAdded, Event::CmixPointsAdded, Event::CmixPointsDeducted]
		);
	});
}

///////////////////////////////////
//    set_cmix_address_space     //
///////////////////////////////////

#[test]
fn set_cmix_address_space_fails_if_not_scheduling() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXCmix::set_cmix_address_space(RuntimeOrigin::signed(1), 0x77),
			Error::<Test>::MustBeScheduling,
		);
	});
}

#[test]
fn set_cmix_address_space_sets_storage() {
	let scheduling = 1;
	let new_address_space = 0x77;
	ExtBuilder::default().with_scheduling_account(scheduling).build_and_execute(|| {
		assert_ok!(XXCmix::set_cmix_address_space(RuntimeOrigin::signed(1), new_address_space));
		assert_eq!(XXCmix::cmix_address_space(), new_address_space);
	});
}
