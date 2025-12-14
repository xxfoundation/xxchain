// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use super::*;
use mock::*;

use frame_support::{
	assert_noop, assert_ok,
	traits::{
		tokens::{fungible::Inspect, Preservation},
		Currency, StorePreimage,
	},
};
use pallet_democracy::{AccountVote, BoundedCallOf, Conviction, Vote};
use pallet_proxy::ProxyDefinition;
use sp_runtime::{DispatchError, ModuleError};
use sp_staking::StakingAccount;
use std::convert::TryInto;

fn reserve_ratio() -> Perbill {
	Perbill::from_rational(55_555_556u32, 1_000_000_000u32)
}

////////////
// Payout //
////////////

#[test]
fn payout_fails_if_payee_is_not_team_member() {
	let caller = 1;
	let payee = 2;
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXCustody::payout(RuntimeOrigin::signed(caller), payee),
			Error::<Test>::InvalidTeamMember
		);
	});
}

#[test]
fn payout_fails_if_called_before_payout_available() {
	let caller = 1;
	let payee = 2;
	let allocation = 1000;
	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		assert_noop!(
			XXCustody::payout(RuntimeOrigin::signed(caller), payee),
			Error::<Test>::PayoutNotAvailable
		);
	});
}

#[test]
fn payout_call_after_first_payout_frequency() {
	let caller = 1;
	let payee = 2;
	let allocation = 1000;
	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		run_to_block(PayoutFrequency::get());
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(caller), payee),);
		// expected payout fraction based on time elapsed
		let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
		let expected_payout = fraction * allocation;
		// balance is tranferred to account
		assert_eq!(Balances::usable_balance(payee), expected_payout);
		// same amount is deducted from TotalCustody
		assert_eq!(XXCustody::total_custody(), allocation - expected_payout,);

		assert_eq!(
			xx_team_custody_events(),
			vec![Event::PayoutFromCustody { who: payee, amount: expected_payout }]
		);
	});
}

#[test]
fn payout_cannot_collect_payout_twice() {
	let caller = 1;
	let payee = 2;
	let allocation = 1000;
	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		run_to_block(PayoutFrequency::get());
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(caller), payee),);
		// second call fails
		assert_noop!(
			XXCustody::payout(RuntimeOrigin::signed(caller), payee),
			Error::<Test>::PayoutNotAvailable
		);
	});
}

#[test]
fn payout_accumulates_with_multiple_frequencies() {
	let caller = 1;
	let payee = 2;
	let allocation = 1000;
	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		run_to_block(PayoutFrequency::get() * 2);
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(caller), payee),);
		// expected payout fraction based on time elapsed (2 periods)
		let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
		let expected_payout = fraction * allocation * 2;
		// balance is tranferred to account
		assert_eq!(Balances::usable_balance(payee), expected_payout);
		assert_eq!(
			xx_team_custody_events(),
			vec![Event::PayoutFromCustody { who: payee, amount: expected_payout }]
		);
	});
}

#[test]
fn payout_call_after_staking_custody_coins() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;
	// In SDK 2509, usable_balance excludes the existential deposit (1)
	// So we need to leave at least 2 to have 1 usable after bonding
	let bond_amount = custody_allocation - 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_eq!(Balances::usable_balance(info.custody), custody_allocation);
			assert_eq!(Balances::usable_balance(info.reserve), reserve_allocation);

			// the custodian bonds all but one coin setting their own accout as controller
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				bond_amount                       // amount to bond
			));

			// compute expected payouts from each account
			let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
			let expected_payout = fraction * allocation;

			// In SDK 2509, staking uses Holds which reduce free_balance
			// Get custody free balance before payout (after bond, free = custody_allocation - bond_amount)
			let custody_free_before = Balances::free_balance(info.custody);

			// Use reducible_balance to match the payout logic (Preserve mode during custody)
			use frame_support::traits::tokens::Fortitude;
			let custody_reducible = <Balances as Inspect<_>>::reducible_balance(
				&info.custody,
				Preservation::Preserve,
				Fortitude::Polite,
			);
			let payout_from_custody = custody_reducible.min(expected_payout);
			let payout_from_reserve = expected_payout - payout_from_custody;

			// actually do the payout
			run_to_block(PayoutFrequency::get());
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			// balance is tranferred to account
			assert_eq!(Balances::free_balance(payee), expected_payout);

			// expected amounts are deducted from custody and reserve accounts
			// In SDK 2509, free_balance is reduced by holds (staking), so use the before value
			assert_eq!(
				Balances::free_balance(info.custody),
				custody_free_before - payout_from_custody
			);
			assert_eq!(
				Balances::free_balance(info.reserve),
				reserve_allocation - payout_from_reserve
			);
			// total custody is decreased
			assert_eq!(
				XXCustody::total_custody(),
				allocation - payout_from_custody - payout_from_reserve,
			);

			assert_eq!(
				xx_team_custody_events(),
				vec![
					Event::PayoutFromCustody { who: payee, amount: payout_from_custody },
					Event::PayoutFromReserve { who: payee, amount: payout_from_reserve }
				]
			);
		});
}

#[test]
fn payout_call_after_staking_all_custody_coins() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;
	let bond_amount = custody_allocation;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_eq!(Balances::usable_balance(info.custody), custody_allocation);
			assert_eq!(Balances::usable_balance(info.reserve), reserve_allocation);

			// the custodian bonds all custody coins
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				bond_amount                       // amount to bond
			));

			let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
			let expected_payout = fraction * allocation;

			// actually do the payout
			run_to_block(PayoutFrequency::get());
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			// balance is tranferred to account
			assert_eq!(Balances::usable_balance(payee), expected_payout);

			// expected amounts are deducted from reserve only, custody remains the same (0)
			assert_eq!(Balances::usable_balance(info.custody), 0);
			assert_eq!(
				Balances::usable_balance(info.reserve),
				reserve_allocation - expected_payout
			);

			assert_eq!(
				xx_team_custody_events(),
				vec![Event::PayoutFromReserve { who: payee, amount: expected_payout }]
			);
		});
}

#[test]
fn payout_call_after_staking_all_custody_insufficient_reserve_funds_for_full_payout() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;
	let bond_amount = custody_allocation;

	let init_reserve_balance = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			// set the reserve a fixed initial balance
			Balances::make_free_balance_be(&info.reserve, init_reserve_balance);

			// the custodian bonds all custody coins
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				bond_amount                       // amount to bond
			));

			// actually do the payout
			run_to_block(PayoutFrequency::get());
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			// Balance receives all of reserve minus existential amount
			let expected_payout = init_reserve_balance - ExistentialDeposit::get();
			assert_eq!(Balances::usable_balance(payee), expected_payout);

			// expected amounts are deducted from reserve only, custody remains the same (0)
			assert_eq!(Balances::usable_balance(info.custody), 0);
			assert_eq!(
				Balances::usable_balance(info.reserve),
				init_reserve_balance - expected_payout
			);

			assert_eq!(
				xx_team_custody_events(),
				vec![Event::PayoutFromReserve { who: payee, amount: expected_payout }]
			);
		});
}

#[test]
fn payout_call_after_staking_all_custody_reserve_depleted() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;
	let bond_amount = custody_allocation;

	let init_reserve_balance = ExistentialDeposit::get();

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			// set the reserve a fixed initial balance
			Balances::make_free_balance_be(&info.reserve, init_reserve_balance);

			// the custodian bonds all custody coins
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				bond_amount                       // amount to bond
			));

			// Payout fails to do anything
			run_to_block(PayoutFrequency::get());
			assert_noop!(
				XXCustody::payout(RuntimeOrigin::signed(custodian), payee),
				Error::<Test>::PayoutFailedInsufficientFunds,
			);
		});
}

#[test]
fn payout_call_after_custody_period_ended() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			run_to_block(CustodyDuration::get() + 1);
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			assert_custody_ended(&[(payee, allocation, 0, info.custody, info.reserve)]);

			assert_eq!(
				xx_team_custody_events(),
				vec![
					Event::PayoutFromCustody { who: payee, amount: custody_allocation },
					Event::PayoutFromReserve { who: payee, amount: reserve_allocation },
					Event::CustodyDone { who: payee },
				]
			);
		});
}

#[test]
fn payout_call_after_custody_period_ended_with_bonded() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			// the custodian bonds all custody coins
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				custody_allocation                // amount to bond
			));
			// In SDK 2509, stash is the controller, so bonded returns the stash itself
			assert_eq!(Staking::bonded(&info.custody), Some(info.custody));

			run_to_block(CustodyDuration::get() + 1);
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			assert_custody_ended(&[(payee, allocation, 0, info.custody, info.reserve)]);

			assert_eq!(
				xx_team_custody_events(),
				vec![
					Event::PayoutFromCustody { who: payee, amount: custody_allocation },
					Event::PayoutFromReserve { who: payee, amount: reserve_allocation },
					Event::CustodyDone { who: payee },
				]
			);
		});
}

#[test]
fn payout_call_after_custody_period_ended_with_custodian_set_proxy() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian),
				info.custody,
				proxy,
			));

			assert_proxy(info.custody, proxy);

			run_to_block(CustodyDuration::get() + 1);
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			assert_custody_ended(&[(payee, allocation, 0, info.custody, info.reserve)]);

			assert_eq!(
				xx_team_custody_events(),
				vec![
					Event::PayoutFromCustody { who: payee, amount: custody_allocation },
					Event::PayoutFromReserve { who: payee, amount: reserve_allocation },
					Event::CustodyDone { who: payee },
				]
			);
		});
}

#[test]
fn payout_call_after_custody_period_ended_with_team_set_proxy() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;
	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;
	let payee_initial_balance = 3; // needs to be able to pay the proxy bond fee

	ExtBuilder::default()
		.with_team_allocations(&[(payee, allocation)])
		.with_custodians(&[custodian])
		.with_initial_balances(&[(payee, payee_initial_balance)])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			// need to wait until team governance period
			run_to_block(GovernanceCustodyDuration::get() + 1);

			assert_ok!(XXCustody::team_custody_set_proxy(RuntimeOrigin::signed(payee), proxy,));

			assert_proxy(info.custody, proxy);

			run_to_block(CustodyDuration::get() + 1);
			assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));

			assert_custody_ended(&[(
				payee,
				allocation,
				payee_initial_balance,
				info.custody,
				info.reserve,
			)]);

			assert_eq!(
				xx_team_custody_events(),
				vec![
					Event::PayoutFromCustody { who: payee, amount: custody_allocation },
					Event::PayoutFromReserve { who: payee, amount: reserve_allocation },
					Event::CustodyDone { who: payee },
				]
			);
		});
}

#[test]
fn payout_fails_if_team_member_deleted() {
	let caller = 1;
	let payee = 2;
	let payee_replacement = 3;
	let allocation = 1000;
	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		run_to_block(PayoutFrequency::get());

		// member has vested balance to claim
		// they are then replaced
		assert_ok!(XXCustody::replace_team_member(RuntimeOrigin::root(), payee, payee_replacement));

		// can no longer payout to old team member account
		assert_noop!(
			XXCustody::payout(RuntimeOrigin::signed(caller), payee),
			Error::<Test>::InvalidTeamMember
		);
	});
}

#[test]
fn payout_pays_out_additional_contributions_to_custody_account() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let additional = 10000;

	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();

		// transfer additional to custody account
		assert_eq!(Balances::usable_balance(info.custody), custody_allocation);
		Balances::make_free_balance_be(&info.custody, custody_allocation + additional);
		assert_eq!(Balances::usable_balance(info.custody), custody_allocation + additional);

		// payout one payout frequency. Should behave the same as normal
		run_to_block(PayoutFrequency::get());
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee),);
		// expected payout fraction based on time elapsed
		let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
		let expected_payout = fraction * allocation;
		// balance is tranferred to account
		assert_eq!(Balances::usable_balance(payee), expected_payout);
		// same amount is deducted from TotalCustody
		assert_eq!(XXCustody::total_custody(), allocation - expected_payout,);

		// payout at the end of the custody period
		run_to_block(CustodyDuration::get() + 1);

		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));
		// the custody account has been totally emptied
		assert_eq!(Balances::usable_balance(info.custody), 0);
		// All funds withdrawn to payee account
		assert_eq!(Balances::usable_balance(payee), allocation + additional);

		assert_custody_ended(&[(payee, allocation, additional, info.custody, info.reserve)]);
	});
}

#[test]
fn payout_pays_out_additional_contributions_to_reserve_account() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let additional = 10000;

	let reserve_allocation = reserve_ratio() * allocation;

	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();

		// transfer additional to reserve account
		assert_eq!(Balances::usable_balance(info.reserve), reserve_allocation);
		Balances::make_free_balance_be(&info.reserve, reserve_allocation + additional);
		assert_eq!(Balances::usable_balance(info.reserve), reserve_allocation + additional);

		// payout one payout frequency. Should behave the same as normal
		run_to_block(PayoutFrequency::get());
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee),);
		// expected payout fraction based on time elapsed
		let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
		let expected_payout = fraction * allocation;
		// balance is tranferred to account
		assert_eq!(Balances::usable_balance(payee), expected_payout);
		// same amount is deducted from TotalCustody
		assert_eq!(XXCustody::total_custody(), allocation - expected_payout,);

		// payout at the end of the custody period
		run_to_block(CustodyDuration::get() + 1);

		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));
		// the custody account has been totally emptied
		assert_eq!(Balances::usable_balance(info.custody), 0);
		// All funds withdrawn to payee account
		assert_eq!(Balances::usable_balance(payee), allocation + additional);

		assert_custody_ended(&[(payee, allocation, additional, info.custody, info.reserve)]);
	});
}

#[test]
fn can_payout_if_custody_account_deducted() {
	let custodian = 1;
	let payee = 2;
	let allocation = 1000;
	let deduction = 100;

	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default().with_team_allocations(&[(payee, allocation)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();

		// deduct from custody account
		// This could be caused by a proxy call if the
		// permissions are not set correctly
		assert_eq!(Balances::usable_balance(info.custody), custody_allocation);
		let _ = Balances::slash(&info.custody, deduction);
		assert_eq!(Balances::usable_balance(info.custody), custody_allocation - deduction);

		// payout one payout frequency. Should behave the same as normal
		run_to_block(PayoutFrequency::get());
		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee),);
		// expected payout fraction based on time elapsed
		let fraction = Perbill::from_rational(PayoutFrequency::get(), CustodyDuration::get());
		let expected_payout = fraction * allocation;
		// balance is tranferred to account
		assert_eq!(Balances::usable_balance(payee), expected_payout);
		// same amount is deducted from TotalCustody
		assert_eq!(XXCustody::total_custody(), allocation - expected_payout,);

		// payout at the end of the custody period
		run_to_block(CustodyDuration::get() + 1);

		assert_ok!(XXCustody::payout(RuntimeOrigin::signed(custodian), payee));
		// the custody account has been totally emptied
		assert_eq!(Balances::usable_balance(info.custody), 0);
		// All funds withdrawn to payee account
		assert_eq!(Balances::usable_balance(payee), allocation - deduction);

		assert_custody_ended(&[(payee, allocation - deduction, 0, info.custody, info.reserve)]);
	});
}

//////////////////////////////////////////
// custody_bond and custody_bond_extra  //
//////////////////////////////////////////

#[test]
fn custody_bond_call_from_non_custodian_fails() {
	let not_custodian = 1;
	let payee = 2;

	ExtBuilder::default().with_team_allocations(&[(payee, 10)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();
		assert_noop!(
			XXCustody::custody_bond(
				RuntimeOrigin::signed(not_custodian), // called by custodian
				info.custody,                         // team member's custody account
				not_custodian,                        // controller of the bond
				1                                     // amount to bond
			),
			Error::<Test>::MustBeCustodian
		);
		assert_noop!(
			XXCustody::custody_bond_extra(
				RuntimeOrigin::signed(not_custodian), // called by custodian
				not_custodian,                        // controller of the bond
				1                                     // amount to bond
			),
			Error::<Test>::MustBeCustodian
		);
	});
}

#[test]
fn custody_bond_call_for_non_custody_account_fails() {
	let custodian = 1;
	let payee = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_noop!(
				XXCustody::custody_bond(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.reserve,                     // team member's custody account
					custodian,                        // controller of the bond
					1                                 // amount to bond
				),
				Error::<Test>::InvalidCustodyAccount
			);
			assert_noop!(
				XXCustody::custody_bond_extra(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.reserve,                     // controller of the bond
					1                                 // amount to bond
				),
				Error::<Test>::InvalidCustodyAccount
			);
		});
}

#[test]
fn custody_bond_can_bond_during_custody_period() {
	let custodian = 1;
	let payee = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				1                                 // amount to bond
			));

			// In SDK 2509, stash is the controller, so bonded returns the stash itself
			assert_eq!(Staking::bonded(&info.custody), Some(info.custody));
			assert_eq!(Staking::ledger(StakingAccount::Stash(info.custody)).unwrap().total, 1);

			// add 1 extra
			assert_ok!(XXCustody::custody_bond_extra(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // custody account
				1                                 // amount to bond
			));
			assert_eq!(Staking::ledger(StakingAccount::Stash(info.custody)).unwrap().total, 2);
		});
}

#[test]
fn custody_bond_fails_after_custody_period() {
	let custodian = 1;
	let payee = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			run_to_block(CustodyDuration::get() + 1);
			assert_noop!(
				XXCustody::custody_bond(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.custody,                     // team member's custody account
					custodian,                        // controller of the bond
					1                                 // amount to bond
				),
				Error::<Test>::CustodyPeriodEnded
			);
			assert_noop!(
				XXCustody::custody_bond_extra(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.custody,                     // controller of the bond
					1                                 // amount to bond
				),
				Error::<Test>::CustodyPeriodEnded
			);
		});
}

/////////////////////////////
// custody_set_controller  //
/////////////////////////////

#[test]
fn custody_set_controller_call_from_non_custodian_fails() {
	let not_custodian = 1;
	let payee = 2;

	ExtBuilder::default().with_team_allocations(&[(payee, 10)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();
		assert_noop!(
			XXCustody::custody_set_controller(
				RuntimeOrigin::signed(not_custodian), // called by custodian
				info.custody,                         // team member's custody account
				not_custodian,                        // controller of the bond
			),
			Error::<Test>::MustBeCustodian
		);
	});
}

#[test]
fn custody_set_controller_call_for_non_custody_account_fails() {
	let custodian = 1;
	let payee = 2;

	ExtBuilder::default().with_custodians(&[custodian]).build_and_execute(|| {
		assert_noop!(
			XXCustody::custody_set_controller(
				RuntimeOrigin::signed(custodian), // called by custodian
				payee,                            // team member's custody account
				custodian,                        // controller of the bond
			),
			Error::<Test>::InvalidCustodyAccount
		);
	});
}

#[test]
fn custody_set_controller_is_noop_in_sdk2509() {
	// NOTE: In SDK 2509+, controller is deprecated (stash is the controller).
	// This function is now a no-op for backwards compatibility.
	let custodian = 1;
	let payee = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			run_to_block(CustodyDuration::get() + 1);
			// Function is now a no-op, so it succeeds even after custody period
			assert_ok!(XXCustody::custody_set_controller(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
			));
		});
}

////////////////////////
// custody_set_proxy  //
////////////////////////

#[test]
fn custody_set_proxy_call_from_non_custodian_fails() {
	let not_custodian = 1;
	let payee = 2;
	let proxy = 3;

	ExtBuilder::default().with_team_allocations(&[(payee, 10)]).build_and_execute(|| {
		let info = XXCustody::team_accounts(payee).unwrap();
		assert_noop!(
			XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(not_custodian), // called by custodian
				info.custody,                         // team member's custody account
				proxy,                                // proxy
			),
			Error::<Test>::MustBeCustodian
		);
	});
}

#[test]
fn custody_set_proxy_call_for_non_custody_account_fails() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;

	ExtBuilder::default().with_custodians(&[custodian]).build_and_execute(|| {
		assert_noop!(
			XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				payee,                            // team member's custody account
				proxy,                            // proxy
			),
			Error::<Test>::InvalidCustodyAccount
		);
	});
}

#[test]
fn custody_set_proxy_can_set_proxy() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy,                            // proxy
			));

			assert_proxy(info.custody, proxy);
		});
}

#[test]
fn custody_set_proxy_fails_if_custody_fully_staked() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();

			// fully stake the custody balance
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				10                                // amount to bond
			));

			// In SDK 2509, this returns InsufficientBalance instead of LiquidityRestrictions
			assert_noop!(
				XXCustody::custody_set_proxy(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.custody,                     // team member's custody account
					proxy,                            // proxy
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
		});
}

#[test]
fn custody_set_proxy_can_set_then_redefine_proxy() {
	let custodian = 1;
	let payee = 2;
	let proxy_1 = 3;
	let proxy_2 = 4;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy_1,                          // proxy
			));

			assert_proxy(info.custody, proxy_1);

			// set proxy to another account
			// This removes old proxy and sets new one
			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy_2,                          // proxy
			));
			assert_proxy(info.custody, proxy_2);
		});
}

#[test]
fn custody_set_proxy_fails_after_governance_period() {
	let custodian = 1;
	let payee = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(payee, 10)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(payee).unwrap();
			run_to_block(GovernanceCustodyDuration::get() + 1);
			assert_noop!(
				XXCustody::custody_set_proxy(
					RuntimeOrigin::signed(custodian), // called by custodian
					info.custody,                     // team member's custody account
					proxy,                            // proxy
				),
				Error::<Test>::GovernanceCustodyPeriodEnded
			);
		});
}

#[test]
fn custody_set_proxy_not_allowed_to_transfer_funds() {
	let custodian = 1;
	let team = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(team, 1000)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team).unwrap();

			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy,                            // proxy
			));
			assert_proxy(info.custody, proxy);

			// The Voting ProxyType allows only calls to democracy and elections vote/remove_vote

			// Try to do a transfer with custody proxy
			// NOTE: proxy call always succeeds
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(proxy),
				info.custody,
				None,
				Box::new(mock::RuntimeCall::Balances(pallet_balances::Call::transfer_all {
					dest: proxy,
					keep_alive: false
				}))
			));

			// We have to look at proxy events to see that the execution of all calls failed
			assert_eq!(
				proxy_events(),
				vec![
					pallet_proxy::Event::ProxyAdded {
						delegator: 7793787273482591193,
						delegatee: proxy,
						proxy_type: ProxyType::Voting,
						delay: 0
					},
					// Proxy calls without enough permissions fail with the error System::CallFiltered
					pallet_proxy::Event::ProxyExecuted {
						result: Err(DispatchError::Module(ModuleError {
							index: 0,
							error: [5, 0, 0, 0],
							message: None
						})),
					},
				]
			);
		});
}

fn aye(who: AccountId) -> AccountVote<Balance> {
	AccountVote::Standard {
		vote: Vote { aye: true, conviction: Conviction::None },
		balance: Balances::free_balance(&who),
	}
}

fn set_balance_proposal(value: u128) -> BoundedCallOf<Test> {
	// In SDK 2509, set_balance was replaced with force_set_balance
	let inner = pallet_balances::Call::force_set_balance { who: 42, new_free: value };
	let outer = RuntimeCall::Balances(inner);
	Preimage::bound(outer).unwrap()
}

#[test]
fn custody_set_proxy_only_allowed_to_vote() {
	let custodian = 1;
	let team = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(team, 1000)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team).unwrap();

			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy,                            // proxy
			));
			assert_proxy(info.custody, proxy);

			// The Voting ProxyType allows only calls to democracy and elections vote/remove_vote

			// Try to run for council (could lead to slashing)
			// NOTE: proxy call always succeeds
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(proxy),
				info.custody,
				None,
				Box::new(mock::RuntimeCall::Elections(
					pallet_elections_phragmen::Call::submit_candidacy { candidate_count: 0 }
				))
			));

			// Try to propose a referendum (could lead to slashing)
			// NOTE: proxy call always succeeds
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(proxy),
				info.custody,
				None,
				Box::new(mock::RuntimeCall::Democracy(pallet_democracy::Call::propose {
					proposal: set_balance_proposal(1100),
					value: 110,
				}))
			));

			// Try to vote for a council candidate
			// NOTE: proxy call always succeeds
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(proxy),
				info.custody,
				None,
				Box::new(mock::RuntimeCall::Elections(pallet_elections_phragmen::Call::vote {
					votes: vec![team],
					value: 5
				}))
			));

			// Try to vote for an active referendum
			// NOTE: proxy call always succeeds
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(proxy),
				info.custody,
				None,
				Box::new(mock::RuntimeCall::Democracy(pallet_democracy::Call::vote {
					ref_index: 0,
					vote: aye(proxy)
				}))
			));

			// We have to look at proxy events to see that the execution of all calls failed
			assert_eq!(
				proxy_events(),
				vec![
					pallet_proxy::Event::ProxyAdded {
						delegator: 7793787273482591193,
						delegatee: proxy,
						proxy_type: ProxyType::Voting,
						delay: 0
					},
					// Elections::submit_candidacy not allowed
					pallet_proxy::Event::ProxyExecuted {
						result: Err(DispatchError::Module(ModuleError {
							index: 0,
							error: [5, 0, 0, 0],
							message: None
						})),
					},
					// Democracy::propose not allowed
					pallet_proxy::Event::ProxyExecuted {
						result: Err(DispatchError::Module(ModuleError {
							index: 0,
							error: [5, 0, 0, 0],
							message: None
						})),
					},
					// Elections::vote is allowed, but fails with UnableToVote (no candidates)
					// Elections has module index 11 in mock, and UnableToVote is error number 0
					pallet_proxy::Event::ProxyExecuted {
						result: Err(DispatchError::Module(ModuleError {
							index: 11,
							error: [0, 0, 0, 0],
							message: None
						})),
					},
					// Democracy::vote is allowed, but fails with ReferendumInvalid (no referendums)
					// Democracy has module index 10 in mock, and ReferendumInvalid is error number 9
					pallet_proxy::Event::ProxyExecuted {
						result: Err(DispatchError::Module(ModuleError {
							index: 10,
							error: [9, 0, 0, 0],
							message: None
						})),
					},
				]
			);
		});
}

/////////////////////////////
// team_custody_set_proxy  //
/////////////////////////////

#[test]
fn team_custody_set_proxy_call_for_non_team_member_fails() {
	let not_team_member = 1;
	let proxy = 2;

	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXCustody::team_custody_set_proxy(RuntimeOrigin::signed(not_team_member), proxy,),
			Error::<Test>::InvalidTeamMember
		);
	});
}

#[test]
fn team_custody_set_proxy_call_before_governance_period_end_fails() {
	let team_member = 1;
	let proxy = 2;

	ExtBuilder::default().with_team_allocations(&[(team_member, 10)]).build_and_execute(|| {
		run_to_block(GovernanceCustodyDuration::get() - 1);
		assert_noop!(
			XXCustody::team_custody_set_proxy(RuntimeOrigin::signed(team_member), proxy,),
			Error::<Test>::GovernanceCustodyActive
		);
	});
}

#[test]
fn team_custody_set_proxy_call_after_governance_period() {
	let team_member = 1;
	let proxy = 2;

	ExtBuilder::default()
		.with_team_allocations(&[(team_member, 10)])
		.with_initial_balances(&[(team_member, 5)])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team_member).unwrap();

			run_to_block(GovernanceCustodyDuration::get() + 1);

			assert_ok!(XXCustody::team_custody_set_proxy(
				RuntimeOrigin::signed(team_member),
				proxy,
			),);

			// correct setting of proxy account
			assert_proxy(info.custody, proxy);
		});
}

#[test]
fn team_custody_set_proxy_call_after_governance_period_custody_fully_staked() {
	let team_member = 1;
	let custodian = 2;
	let proxy = 3;

	let allocation = 1000;
	let reserve_allocation = reserve_ratio() * allocation;
	let custody_allocation = allocation - reserve_allocation;

	ExtBuilder::default()
		.with_team_allocations(&[(team_member, allocation)])
		.with_initial_balances(&[(team_member, 5)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team_member).unwrap();

			// fully stake the custody balance
			assert_ok!(XXCustody::custody_bond(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				custodian,                        // controller of the bond
				custody_allocation                // amount to bond
			));

			run_to_block(GovernanceCustodyDuration::get() + 1);

			// In SDK 2509, this returns InsufficientBalance instead of LiquidityRestrictions
			assert_noop!(
				XXCustody::team_custody_set_proxy(RuntimeOrigin::signed(team_member), proxy,),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
		});
}

#[test]
fn team_custody_set_proxy_call_after_governance_period_with_existing_custodian_proxy() {
	let team_member = 1;
	let custodian = 2;
	let proxy = 3;

	ExtBuilder::default()
		.with_team_allocations(&[(team_member, 10)])
		.with_initial_balances(&[(team_member, 5)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team_member).unwrap();

			// custody proxy set before custody governance period ends

			assert_ok!(XXCustody::custody_set_proxy(
				RuntimeOrigin::signed(custodian), // called by custodian
				info.custody,                     // team member's custody account
				proxy,                            // proxy
			));

			assert_proxy(info.custody, proxy);

			run_to_block(GovernanceCustodyDuration::get() + 1);

			assert_ok!(XXCustody::team_custody_set_proxy(
				RuntimeOrigin::signed(team_member),
				proxy,
			),);

			// correct setting of new proxy account and removal of old one
			assert_proxy(info.custody, proxy)
		});
}

#[test]
fn team_custody_set_proxy_call_after_governance_period_set_twice() {
	let team_member = 1;
	let custodian = 2;
	let proxy_1 = 3;
	let proxy_2 = 4;

	ExtBuilder::default()
		.with_team_allocations(&[(team_member, 10)])
		.with_initial_balances(&[(team_member, 5)])
		.with_custodians(&[custodian])
		.build_and_execute(|| {
			let info = XXCustody::team_accounts(team_member).unwrap();

			run_to_block(GovernanceCustodyDuration::get() + 1);

			assert_ok!(XXCustody::team_custody_set_proxy(
				RuntimeOrigin::signed(team_member),
				proxy_1,
			),);

			// correct setting of new proxy account and removal of old one
			assert_proxy(info.custody, proxy_1);

			assert_ok!(XXCustody::team_custody_set_proxy(
				RuntimeOrigin::signed(team_member),
				proxy_2,
			),);

			// correct setting of new proxy account and removal of old one
			assert_proxy(info.custody, proxy_2);
		});
}

//////////////////////
// admin functions  //
//////////////////////

fn admin_origin() -> mock::RuntimeOrigin {
	RuntimeOrigin::root()
}

// add/remove custodian

#[test]
fn admin_add_remove_custodian() {
	let new_custodian = 1;
	ExtBuilder::default().build_and_execute(|| {
		assert_ok!(XXCustody::add_custodian(admin_origin(), new_custodian));
		assert!(<Custodians<Test>>::contains_key(new_custodian),);
		assert_ok!(XXCustody::remove_custodian(admin_origin(), new_custodian));
		assert!(!<Custodians<Test>>::contains_key(new_custodian),);

		assert_eq!(
			xx_team_custody_events(),
			vec![
				Event::CustodianAdded { who: new_custodian },
				Event::CustodianRemoved { who: new_custodian },
			]
		);
	});
}

// replace team member

#[test]
fn replace_team_member_cannot_replace_non_team() {
	let not_team_member = 1;
	let new_team_account = 2;
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXCustody::replace_team_member(admin_origin(), not_team_member, new_team_account),
			Error::<Test>::InvalidTeamMember
		);
	});
}

#[test]
fn replace_team_member_cannot_replace_with_existing_team_member() {
	let team_member_1 = 1;
	let team_member_2 = 2;
	ExtBuilder::default()
		.with_team_allocations(&[(team_member_1, 0), (team_member_2, 0)])
		.build_and_execute(|| {
			assert_noop!(
				XXCustody::replace_team_member(admin_origin(), team_member_1, team_member_2),
				Error::<Test>::TeamMemberExists
			);
		});
}

#[test]
fn replace_team_member() {
	let team_member = 1;
	let replacement = 2;
	ExtBuilder::default().with_team_allocations(&[(team_member, 0)]).build_and_execute(|| {
		let original_config = XXCustody::team_accounts(team_member).unwrap();

		assert!(<TeamAccounts<Test>>::contains_key(team_member),);
		assert!(!<TeamAccounts<Test>>::contains_key(replacement),);

		assert_ok!(XXCustody::replace_team_member(admin_origin(), team_member, replacement),);

		assert!(!<TeamAccounts<Test>>::contains_key(team_member),);
		assert!(<TeamAccounts<Test>>::contains_key(replacement),);
		assert_eq!(original_config, XXCustody::team_accounts(replacement).unwrap());

		assert_eq!(
			xx_team_custody_events(),
			vec![Event::TeamMemberUpdated { old: team_member, new: replacement },]
		);
	});
}

// helpers

fn assert_proxy(account: AccountId, delegate: AccountId) {
	assert_eq!(
		Proxy::proxies(account),
		(
			vec![ProxyDefinition { delegate, proxy_type: ProxyType::Voting, delay: 0 }]
				.try_into()
				.unwrap(),
			2
		)
	);
}

// accepts a slice of (account, allocation, initial_balance, account, account)
fn assert_custody_ended(allocations: &[(AccountId, Balance, Balance, AccountId, AccountId)]) {
	for (team_account, allocation, initial_balance, custody, reserve) in allocations {
		// any bonding is removed
		assert_eq!(Staking::bonded(custody), None);

		// governance proxy was removed
		assert_eq!(Proxy::proxies(*custody), (vec![].try_into().unwrap(), 0));

		// all funds transferred to payee account
		assert_eq!(Balances::usable_balance(team_account), initial_balance + allocation);
		// All custody and reserve accounts are killed
		// custody account depleted and reaped
		assert_eq!(<Balances as Currency<_>>::total_balance(custody), 0);
		assert!(is_reaped(custody));
		// reserve account depleted and reaped
		assert_eq!(<Balances as Currency<_>>::total_balance(reserve), 0);
		assert!(is_reaped(reserve));
	}

	// total custody amount is zero
	assert_eq!(XXCustody::total_custody(), 0);

	// team accounts storage is empty
	assert_eq!(<TeamAccounts<Test>>::iter().count(), 0);
	// custody accouts storage is empty
	assert_eq!(<CustodyAccounts<Test>>::iter().count(), 0);
}

fn is_reaped(account: &AccountId) -> bool {
	System::account(account) == Default::default()
}
