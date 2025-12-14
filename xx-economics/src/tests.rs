// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use super::*;
use crate::inflation::{IdealInterestPoint, InflationFixedParams};
use mock::*;
use pallet_balances::PositiveImbalance;
use pallet_staking::EraPayout;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{traits::BadOrigin, Perbill};

// set_inflation_params

#[test]
fn set_inflation_params_called_by_non_admin_fails() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXEconomics::set_inflation_params(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
fn set_inflation_params_called_by_admin() {
	ExtBuilder::default().build_and_execute(|| {
		let test_params =
			InflationFixedParams { min_inflation: Perbill::from_float(0.1), ..Default::default() };
		assert_ok!(XXEconomics::set_inflation_params(
			RuntimeOrigin::signed(AdminAccount::get()),
			test_params.clone()
		));
		assert_eq!(XXEconomics::inflation_params(), test_params);
		assert_eq!(xx_economics_events(), vec![Event::InflationParamsChanged]);
	});
}

// set_interest_points

#[test]
fn set_interest_points_called_by_non_admin_fails() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXEconomics::set_interest_points(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
fn set_interest_points_called_by_admin() {
	ExtBuilder::default().build_and_execute(|| {
		let test_points =
			vec![IdealInterestPoint::<BlockNumber> { block: 99, ..Default::default() }];
		assert_ok!(XXEconomics::set_interest_points(
			RuntimeOrigin::signed(AdminAccount::get()),
			test_points.clone()
		));
		assert_eq!(XXEconomics::interest_points(), test_points);
		assert_eq!(xx_economics_events(), vec![Event::InterestPointsChanged]);
	});
}

// set_liquidity_rewards_stake

#[test]
fn set_liquidity_rewards_stake_called_by_non_admin_fails() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXEconomics::set_liquidity_rewards_stake(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
fn set_liquidity_rewards_stake_called_by_admin() {
	ExtBuilder::default().build_and_execute(|| {
		let test_stake = 555;
		assert_ok!(XXEconomics::set_liquidity_rewards_stake(
			RuntimeOrigin::signed(AdminAccount::get()),
			test_stake
		));
		assert_eq!(XXEconomics::ideal_stake_rewards(), test_stake);
		assert_eq!(xx_economics_events(), vec![Event::IdealLiquidityStakeChanged]);
	});
}

// set_liquidity_rewards_balance

#[test]
fn set_liquidity_rewards_balance_called_by_non_admin_fails() {
	ExtBuilder::default().build_and_execute(|| {
		assert_noop!(
			XXEconomics::set_liquidity_rewards_balance(
				RuntimeOrigin::signed(1),
				Default::default()
			),
			BadOrigin
		);
	})
}

#[test]
fn set_liquidity_rewards_balance_called_by_admin() {
	ExtBuilder::default().build_and_execute(|| {
		let test_balance = 123;
		assert_ok!(XXEconomics::set_liquidity_rewards_balance(
			RuntimeOrigin::signed(AdminAccount::get()),
			test_balance
		));
		assert_eq!(XXEconomics::liquidity_rewards(), test_balance);
		assert_eq!(xx_economics_events(), vec![Event::LiquidityRewardsBalanceChanged]);
	});
}

// rewards

#[test]
fn takes_rewards_from_pool_when_possible() {
	let initial_rewards_balance = 10000;
	let issuance = 1000;
	ExtBuilder::default().with_rewards_balance(initial_rewards_balance).build_and_execute(|| {
		let initial_issuance = Balances::total_issuance();

		// 1000 coins were issued as validator rewards
		let imbalance = PositiveImbalance::<Test>::new(issuance);
		XXEconomics::on_nonzero_unbalanced(imbalance);

		// these should all be deducted form rewards balance
		assert_eq!(XXEconomics::rewards_balance(), initial_rewards_balance - issuance);
		// no further issuance
		assert_eq!(Balances::total_issuance(), initial_issuance);

		assert_eq!(xx_economics_events(), vec![Event::RewardFromPool(issuance)])
	});
}

#[test]
fn will_issue_when_pool_depleted() {
	let initial_rewards_balance = 0;
	let issuance = 1000;
	ExtBuilder::default().with_rewards_balance(initial_rewards_balance).build_and_execute(|| {
		let initial_issuance = Balances::total_issuance();

		// 1000 coins were issued as validator rewards
		let imbalance = PositiveImbalance::<Test>::new(issuance);
		XXEconomics::on_nonzero_unbalanced(imbalance);

		// some taken from rewards balance
		assert_eq!(XXEconomics::rewards_balance(), 0);
		// remainder issued
		assert_eq!(Balances::total_issuance(), initial_issuance + issuance);

		assert_eq!(xx_economics_events(), vec![Event::RewardMinted(issuance)])
	});
}

#[test]
fn will_split_pool_and_issuance_when_required() {
	let initial_rewards_balance = 300;
	let issuance = 1000;
	ExtBuilder::default().with_rewards_balance(initial_rewards_balance).build_and_execute(|| {
		let initial_issuance = Balances::total_issuance();

		// 1000 coins were issued as validator rewards
		let imbalance = PositiveImbalance::<Test>::new(issuance);
		XXEconomics::on_nonzero_unbalanced(imbalance);

		// some taken from rewards balance
		assert_eq!(XXEconomics::rewards_balance(), 0);
		// remainder issued
		assert_eq!(
			Balances::total_issuance(),
			initial_issuance + (issuance - initial_rewards_balance)
		);

		assert_eq!(
			xx_economics_events(),
			vec![
				Event::RewardFromPool(initial_rewards_balance),
				Event::RewardMinted(issuance - initial_rewards_balance)
			]
		)
	});
}

#[test]
fn noop_on_zero_imbalance() {
	let initial_rewards_balance = 100;
	let issuance = 0;
	ExtBuilder::default().with_rewards_balance(initial_rewards_balance).build_and_execute(|| {
		let initial_issuance = Balances::total_issuance();

		// 1000 coins were issued as validator rewards
		let imbalance = PositiveImbalance::<Test>::new(issuance);
		XXEconomics::on_nonzero_unbalanced(imbalance);

		// no change
		assert_eq!(XXEconomics::rewards_balance(), initial_rewards_balance);
		assert_eq!(Balances::total_issuance(), initial_issuance + issuance);
		// no events
		assert_eq!(xx_economics_events(), vec![])
	});
}

// reward remainders

#[test]
fn reward_remainders_will_split_pool_and_issuance_when_required() {
	let initial_rewards_balance = 300;
	let reward_remainder = 1000;
	ExtBuilder::default().with_rewards_balance(initial_rewards_balance).build_and_execute(|| {
		let initial_issuance = Balances::total_issuance();

		// 1000 tokens have alreay been minted, and the issuance updated
		let imbalance = Balances::issue(reward_remainder);

		// Send imbalance to treasury account
		rewards::RewardRemainderAdapter::<Test>::on_nonzero_unbalanced(imbalance);

		// some taken from rewards balance, emptying it
		assert_eq!(XXEconomics::rewards_balance(), 0);
		// remainder issued
		assert_eq!(
			Balances::total_issuance(),
			initial_issuance + (reward_remainder - initial_rewards_balance)
		);
		// all ends up in treasury
		assert_eq!(Balances::total_balance(MOCK_TREASURY), reward_remainder);

		assert_eq!(
			xx_economics_events(),
			vec![
				Event::RewardFromPool(initial_rewards_balance),
				Event::RewardMinted(reward_remainder - initial_rewards_balance)
			]
		);
	});
}

// era payout

fn test_interest_curve() -> Vec<IdealInterestPoint<BlockNumber>> {
	vec![
		IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 2u32) },
		IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 4u32) },
	]
}

#[test]
fn get_era_payout_at_block_lower_than_first_point() {
	ExtBuilder::default()
		.with_interest_points(vec![
			IdealInterestPoint { block: 5, interest: Perbill::from_rational(1u32, 4u32) },
			IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 2u32) },
		])
		.build_and_execute(|| {
			assert_eq!(XXEconomics::era_payout(0, 0, 0), (0, 0));
		});
}

#[test]
fn get_era_payout_at_block_smaller_than_half_session() {
	ExtBuilder::default().with_interest_points(test_interest_curve()).build_and_execute(|| {
		assert_eq!(XXEconomics::era_payout(0, 0, 0), (0, 0));
	});
}

#[test]
fn get_era_payout_in_first_segment() {
	ExtBuilder::default().with_interest_points(test_interest_curve()).build_and_execute(|| {
		run_to_block(5);
		assert_eq!(XXEconomics::era_payout(0, 0, 0), (0, 0));
	});
}

#[test]
fn get_era_payout_after_final_point() {
	ExtBuilder::default().with_interest_points(test_interest_curve()).build_and_execute(|| {
		run_to_block(11);
		assert_eq!(XXEconomics::era_payout(0, 0, 0), (0, 0));
	});
}

#[test]
fn get_era_payout_with_increasing_interest() {
	ExtBuilder::default()
		.with_interest_points(vec![
			IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 4u32) },
			IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 2u32) },
		])
		.build_and_execute(|| {
			run_to_block(5);
			// With stake_ratio = 5000/11000 ≈ 45.45% (below ideal_stake = 50%),
			// inflation_ratio < 1, so validator_payout < max_payout
			assert_eq!(XXEconomics::era_payout(5000, 11000, MILLISECONDS_PER_YEAR), (1900, 162));
		});
}

fn test_unordered_interest_curve() -> Vec<IdealInterestPoint<BlockNumber>> {
	vec![
		IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 4u32) },
		IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 2u32) },
	]
}

fn test_interest_curve_two() -> Vec<IdealInterestPoint<BlockNumber>> {
	vec![
		IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 2u32) },
		IdealInterestPoint { block: 5, interest: Perbill::from_rational(1u32, 3u32) },
		IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 4u32) },
	]
}

fn test_unordered_interest_curve_two() -> Vec<IdealInterestPoint<BlockNumber>> {
	vec![
		IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 4u32) },
		IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 2u32) },
		IdealInterestPoint { block: 5, interest: Perbill::from_rational(1u32, 3u32) },
	]
}

#[test]
fn confirm_unordered_points_get_sorted_at_genesis() {
	ExtBuilder::default().with_interest_points(test_unordered_interest_curve()).build_and_execute(
		|| {
			assert_eq!(XXEconomics::interest_points(), test_interest_curve());
		},
	);
}

#[test]
fn confirm_unordered_points_get_sorted_when_set_by_admin() {
	ExtBuilder::default().with_interest_points(test_interest_curve()).build_and_execute(|| {
		assert_ok!(XXEconomics::set_interest_points(
			RuntimeOrigin::signed(AdminAccount::get()),
			test_unordered_interest_curve_two()
		));
		assert_eq!(XXEconomics::interest_points(), test_interest_curve_two());
		assert_eq!(xx_economics_events(), vec![Event::InterestPointsChanged]);
	});
}

// Test payout doesn't count unstakeable coins in issuance
#[test]
fn confirm_payout_stakeable_calculation() {
	let rewards_balance = 1000;
	ExtBuilder::default()
		.with_rewards_balance(rewards_balance)
		.with_liquidity_balance(rewards_balance)
		.with_public_accounts()
		.with_interest_points(vec![
			IdealInterestPoint { block: 0, interest: Perbill::from_rational(1u32, 4u32) },
			IdealInterestPoint { block: 10, interest: Perbill::from_rational(1u32, 2u32) },
		])
		// Total unstakeable should be
		// rewards -> 1000
		// liquidity -> 1000
		// public -> 2000
		// total = 4000
		// So if issuance is 14000, total stakeable should be 10000, so staking ratio is 50%
		.build_and_execute(|| {
			run_to_block(5);
			assert_eq!(XXEconomics::era_payout(5000, 14000, MILLISECONDS_PER_YEAR), (1875, 0));
		});
}
