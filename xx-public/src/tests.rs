
use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{
    traits::BadOrigin,
};

// set_testnet_manager_account

#[test]
fn set_testnet_manager_account_called_by_non_admin_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::set_testnet_manager_account(RuntimeOrigin::signed(1), 45),
				BadOrigin
			);
        })
}

#[test]
fn set_testnet_manager_account_called_by_admin_works() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::set_testnet_manager_account(RuntimeOrigin::signed(AdminAccount::get()), 45)
			);
            assert_eq!(
                XXPublic::testnet_manager().unwrap(),
                45
            );
            assert_eq!(
                xx_public_events(),
                vec![super::Event::TestnetManagerUpdated]
            );
        })
}

// set_sale_manager_account

#[test]
fn set_sale_manager_account_called_by_non_admin_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::set_sale_manager_account(RuntimeOrigin::signed(1), 45),
				BadOrigin
			);
        })
}

#[test]
fn set_sale_manager_account_called_by_admin_works() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::set_sale_manager_account(RuntimeOrigin::signed(AdminAccount::get()), 46)
			);
            assert_eq!(
                XXPublic::sale_manager().unwrap(),
                46
            );
            assert_eq!(
                xx_public_events(),
                vec![super::Event::SaleManagerUpdated]
            );
        })
}

// testnet_distribute

#[test]
fn testnet_distribute_called_by_non_manager_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::testnet_distribute(RuntimeOrigin::signed(1), Default::default()),
				Error::<Test>::MustBeTestnetManager
			);
        })
}

#[test]
fn testnet_distribute_called_by_sale_manager_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::testnet_distribute(RuntimeOrigin::signed(43), Default::default()),
				Error::<Test>::MustBeTestnetManager
			);
        })
}

#[test]
fn testnet_distribute_called_by_manager_works() {
    ExtBuilder::default()
        .with_testnet_balance(1000)
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::testnet_distribute(
				    RuntimeOrigin::signed(42),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 100,
				            schedules: None
				        }
				    ],
                )
			);
            // Check coins distributed
            assert_eq!(Balances::total_balance(&10), 100);
        })
}

// sale_distribute

#[test]
fn sale_distribute_called_by_non_manager_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::sale_distribute(RuntimeOrigin::signed(1), Default::default()),
				Error::<Test>::MustBeSaleManager
			);
        })
}

#[test]
fn sale_distribute_called_by_testnet_manager_fails() {
    ExtBuilder::default()
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::sale_distribute(RuntimeOrigin::signed(42), Default::default()),
				Error::<Test>::MustBeSaleManager
			);
        })
}

#[test]
fn sale_distribute_called_by_manager_works() {
    ExtBuilder::default()
        .with_sale_balance(1000)
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::sale_distribute(
				    RuntimeOrigin::signed(43),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 100,
				            schedules: None
				        }
				    ],
                )
			);
            // Check coins distributed
            assert_eq!(Balances::total_balance(&10), 100);
        })
}

// distribution tests

#[test]
fn distributions_fail_when_not_enough_funds() {
    ExtBuilder::default()
        .with_testnet_balance(1000)
        .with_sale_balance(1000)
        .build_and_execute(|| {
            assert_noop!(
				XXPublic::testnet_distribute(
				    RuntimeOrigin::signed(42),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 1001,
				            schedules: None
				        }
				    ],
                ),
                Error::<Test>::NotEnoughFunds
			);
            assert_noop!(
				XXPublic::sale_distribute(
				    RuntimeOrigin::signed(43),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 1001,
				            schedules: None
				        }
				    ],
                ),
                Error::<Test>::NotEnoughFunds
			);
        })
}


#[test]
fn testnet_distribution_with_vesting() {
    ExtBuilder::default()
        .with_testnet_balance(1000)
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::testnet_distribute(
				    RuntimeOrigin::signed(42),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 200,
				            schedules: Some(vec![(100, 1, 0)])
				        },
				        TransferData::<AccountId, Balance, u64> {
				            destination: 11,
				            amount: 200,
				            schedules: Some(vec![(100, 100, 100)])
				        },
				    ],
                )
			);
            // Check coins distributed
            assert_eq!(Balances::total_balance(&10), 200);
            assert_eq!(Balances::total_balance(&11), 200);
            // Check vesting schedules added
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(99));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(100));

            // Advance to block 50
            run_to_block(50);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(50));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(100));

            // Advance to block 101
            run_to_block(101);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(0));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(0));
        })
}

#[test]
fn sale_distribution_with_vesting() {
    ExtBuilder::default()
        .with_sale_balance(1000)
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::sale_distribute(
				    RuntimeOrigin::signed(43),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 10,
				            amount: 200,
				            schedules: Some(vec![(100, 1, 0)])
				        },
				        TransferData::<AccountId, Balance, u64> {
				            destination: 11,
				            amount: 200,
				            schedules: Some(vec![(100, 100, 100)])
				        },
				    ],
                )
			);
            // Check coins distributed
            assert_eq!(Balances::total_balance(&10), 200);
            assert_eq!(Balances::total_balance(&11), 200);
            // Check vesting schedules added
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(99));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(100));

            // Advance to block 50
            run_to_block(50);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(50));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(100));

            // Advance to block 101
            run_to_block(101);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&10), Some(0));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&11), Some(0));
        })
}

#[test]
fn distribute_to_account_with_existing_vesting() {
    ExtBuilder::default()
        .with_sale_balance(1000)
        .with_vesting()
        .build_and_execute(|| {
            assert_ok!(
				XXPublic::sale_distribute(
				    RuntimeOrigin::signed(43),
				    vec![
				        TransferData::<AccountId, Balance, u64> {
				            destination: 12,
				            amount: 200,
				            schedules: Some(vec![(100, 1, 0), (100, 100, 100)])
				        },
				        TransferData::<AccountId, Balance, u64> {
				            destination: 13,
				            amount: 200,
				            schedules: Some(vec![(100, 100, 100)])
				        },
				    ],
                )
			);
            // Check coins distributed
            assert_eq!(Balances::total_balance(&12), 300);
            assert_eq!(Balances::total_balance(&13), 300);
            // Check only first vesting schedule added to account 12
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&12), Some(198));
            // Check vesting schedule added to account 13
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&13), Some(199));

            // Advance to block 50
            run_to_block(50);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&12), Some(100));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&13), Some(150));

            // Advance to block 101
            run_to_block(101);

            // Check vesting schedules updated
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&12), Some(0));
            assert_eq!(<Test as Config>::VestingSchedule::vesting_balance(&13), Some(0));
        })
}
