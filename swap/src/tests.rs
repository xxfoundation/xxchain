// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

#![cfg(test)]

use super::{mock::*, *};

use super::mock::RuntimeEvent;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{BuildStorage, DispatchError};

///////////////////////////////////////////
//             transfer_native          ///
///////////////////////////////////////////

const TEST_DESTINATION_CHAIN: chainbridge::ChainId = 0xff;
const TEST_RECIPIENT_ADDR: &[u8] = &[0; 32]; // mock address on destination chain

#[test]
fn transfer_native_non_whitelisted_destination_chain_fails() {
	new_test_ext(&[]).execute_with(|| {
		let amount: u64 = 1000;
		assert_noop!(
			Swap::transfer_native(
				RuntimeOrigin::signed(ACCOUNT_A),
				amount,
				TEST_RECIPIENT_ADDR.to_vec(),
				TEST_DESTINATION_CHAIN,
			),
			Error::<Test>::DestinationNotWhitelisted
		);
	})
}

#[test]
fn transfer_native_insufficient_balance_for_fee_and_amount_fails() {
	let amount: u64 = 1000;
	// preload account_a with one less than the amount plus swap fee
	new_test_ext(&[(ACCOUNT_A, amount + SWAP_FEE - 1)]).execute_with(|| {
		assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), TEST_DESTINATION_CHAIN));
		assert_noop!(
			Swap::transfer_native(
				RuntimeOrigin::signed(ACCOUNT_A),
				amount,
				TEST_RECIPIENT_ADDR.to_vec(),
				TEST_DESTINATION_CHAIN,
			),
			Error::<Test>::InsufficientBalance
		);
	})
}

#[test]
fn transfer_native_successful() {
	let amount: u64 = 1000;
	let existential_deposit: u64 = 1; // Must match ExistentialDeposit in mock
	new_test_ext(&[(ACCOUNT_A, amount + SWAP_FEE), (FEE_DESTINATION, existential_deposit)])
		.execute_with(|| {
			assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), TEST_DESTINATION_CHAIN));
			assert_ok!(Swap::transfer_native(
				RuntimeOrigin::signed(ACCOUNT_A),
				amount,
				TEST_RECIPIENT_ADDR.to_vec(),
				TEST_DESTINATION_CHAIN,
			));
			// fee transferred to fee destination
			assert_eq!(Balances::free_balance(FEE_DESTINATION), existential_deposit + SWAP_FEE);
			// amount transferred to bridge account
			assert_eq!(Balances::free_balance(Bridge::account_id()), amount);
			// bridge emits FungibleTransfer event
			expect_event(bridge::Event::FungibleTransfer(
				TEST_DESTINATION_CHAIN,
				1,                    // nonce
				NativeTokenId::get(), // resource ID
				amount.into(),
				TEST_RECIPIENT_ADDR.to_vec(),
			));
		})
}

///////////////////////////////////////////
//                transfer              ///
///////////////////////////////////////////

#[test]
fn transfer_fails_for_non_bridge_origin() {
	new_test_ext(&[]).execute_with(|| {
		let amount: u64 = 1000;
		assert_noop!(
			Swap::transfer(RuntimeOrigin::signed(ACCOUNT_A), ACCOUNT_A, amount,),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn transfer_successful_with_bridge_origin() {
	let amount: u64 = 1000;
	let existential_deposit: u64 = 1; // Must match ExistentialDeposit in mock
	new_test_ext(&[
		(Bridge::account_id(), amount + existential_deposit),
		(ACCOUNT_A, existential_deposit),
	])
	.execute_with(|| {
		assert_ok!(Swap::transfer(RuntimeOrigin::signed(Bridge::account_id()), ACCOUNT_A, amount,));
		// amount transferred from bridge account to account A
		assert_eq!(Balances::free_balance(&Bridge::account_id()), existential_deposit);
		assert_eq!(Balances::free_balance(&ACCOUNT_A), existential_deposit + amount);
	})
}

///////////////////////////////////////////
//              set_swap_fee            ///
///////////////////////////////////////////

#[test]
fn set_swap_fee_fails_for_non_admin_origin() {
	let fee: u64 = 5;
	new_test_ext(&[]).execute_with(|| {
		assert_noop!(
			Swap::set_swap_fee(RuntimeOrigin::signed(ACCOUNT_A), fee,),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn set_swap_fee_successful_with_admin_origin() {
	let fee: u64 = 5;
	new_test_ext(&[]).execute_with(|| {
		assert_ok!(Swap::set_swap_fee(
			RuntimeOrigin::root(), // in the mock the admin origin is root
			fee,
		));
		// amount transferred from bridge account to account A
		assert_eq!(Swap::swap_fee(), fee);
		// fee change event emitted
		expect_event(swap::Event::FeeChanged(fee));
	})
}

///////////////////////////////////////////
//           set_fee_destination        ///
///////////////////////////////////////////

#[test]
fn set_fee_destination_fails_for_non_admin_origin() {
	let dest: u64 = ACCOUNT_A;
	new_test_ext(&[]).execute_with(|| {
		assert_noop!(
			Swap::set_fee_destination(RuntimeOrigin::signed(ACCOUNT_A), dest,),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn set_fee_destination_successful_with_admin_origin() {
	let dest: u64 = ACCOUNT_A;
	new_test_ext(&[]).execute_with(|| {
		assert_ok!(Swap::set_fee_destination(
			RuntimeOrigin::root(), // in the mock the admin origin is root
			dest,
		));
		// amount transferred from bridge account to account A
		assert_eq!(Swap::fee_destination().unwrap(), dest);
		// fee change event emitted
		expect_event(swap::Event::FeeDestinationChanged(dest));
	})
}

////////////////////////////////////////////////////
//       integration with chaibridge pallet      ///
////////////////////////////////////////////////////

/// This test creates a proposal on the chainbridge pallet for the execution
/// of the `transfer` extrinsic in the swap pallet. Relayers vote to approve the proposal and
/// it should execute correctly leaving a record in the events log.
#[test]
fn sucessful_transfer_proposal() {
	let amount = 1000;
	let account_a_initial = 99;
	let bridge_initial = 3 * amount;
	new_test_ext(&[(Bridge::account_id(), bridge_initial), (ACCOUNT_A, account_a_initial)])
		.execute_with(|| {
			let prop_id = 1; // proposal ID / nonce
			let r_id = <Test as swap::Config>::NativeTokenId::get(); // resource ID

			// The proposal for execution that a relayer will submit. Calling transfer of amount to account A.
			let proposal =
				super::mock::RuntimeCall::Swap(swap::Call::transfer { to: ACCOUNT_A, amount });

			assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), RELAYER_THRESHOLD,));
			assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_A));
			assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_B));
			assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_C));
			assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), TEST_DESTINATION_CHAIN));
			assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), r_id, b"xx coin".to_vec()));

			// Create proposal (& vote)
			assert_ok!(Bridge::acknowledge_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				prop_id,
				TEST_DESTINATION_CHAIN,
				r_id,
				Box::new(proposal.clone())
			));
			let prop = Bridge::votes(TEST_DESTINATION_CHAIN, (prop_id, proposal.clone())).unwrap();
			let expected = bridge::ProposalVotes {
				votes_for: vec![RELAYER_A],
				votes_against: vec![],
				status: bridge::ProposalStatus::Initiated,
				expiry: ProposalLifetime::get() + 1,
			};
			assert_eq!(prop, expected);

			// Second relayer votes against
			assert_ok!(Bridge::reject_proposal(
				RuntimeOrigin::signed(RELAYER_B),
				prop_id,
				TEST_DESTINATION_CHAIN,
				r_id,
				Box::new(proposal.clone())
			));
			let prop = Bridge::votes(TEST_DESTINATION_CHAIN, (prop_id, proposal.clone())).unwrap();
			let expected = bridge::ProposalVotes {
				votes_for: vec![RELAYER_A],
				votes_against: vec![RELAYER_B],
				status: bridge::ProposalStatus::Initiated,
				expiry: ProposalLifetime::get() + 1,
			};
			assert_eq!(prop, expected);

			// Third relayer votes in favour
			assert_ok!(Bridge::acknowledge_proposal(
				RuntimeOrigin::signed(RELAYER_C),
				prop_id,
				TEST_DESTINATION_CHAIN,
				r_id,
				Box::new(proposal.clone())
			));
			let prop = Bridge::votes(TEST_DESTINATION_CHAIN, (prop_id, proposal.clone())).unwrap();
			let expected = bridge::ProposalVotes {
				votes_for: vec![RELAYER_A, RELAYER_C],
				votes_against: vec![RELAYER_B],
				status: bridge::ProposalStatus::Approved,
				expiry: ProposalLifetime::get() + 1,
			};
			assert_eq!(prop, expected);

			// bridge account was deducted and account A credited
			assert_eq!(Balances::free_balance(ACCOUNT_A), account_a_initial + amount);
			assert_eq!(Balances::free_balance(Bridge::account_id()), bridge_initial - amount);

			assert_events(vec![
				RuntimeEvent::Bridge(bridge::Event::VoteFor(
					TEST_DESTINATION_CHAIN,
					prop_id,
					RELAYER_A,
				)),
				RuntimeEvent::Bridge(bridge::Event::VoteAgainst(
					TEST_DESTINATION_CHAIN,
					prop_id,
					RELAYER_B,
				)),
				RuntimeEvent::Bridge(bridge::Event::VoteFor(
					TEST_DESTINATION_CHAIN,
					prop_id,
					RELAYER_C,
				)),
				RuntimeEvent::Bridge(bridge::Event::ProposalApproved(
					TEST_DESTINATION_CHAIN,
					prop_id,
				)),
				RuntimeEvent::Balances(balances::Event::Transfer {
					from: Bridge::account_id(),
					to: ACCOUNT_A,
					amount,
				}),
				RuntimeEvent::Bridge(bridge::Event::ProposalSucceeded(
					TEST_DESTINATION_CHAIN,
					prop_id,
				)),
			]);
		})
}

///////////////////////////////////////
//       config initialization      ///
///////////////////////////////////////

#[test]
fn config_sets_expected_storage_items() {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let initial_bridge_balance = 100;
	let resource_id = <Test as swap::Config>::NativeTokenId::get();
	let resource_name = b"xx coin".to_vec();

	// define the gensis config for swap
	swap::GenesisConfig::<Test> {
		chains: vec![TEST_DESTINATION_CHAIN],
		relayers: vec![RELAYER_A, RELAYER_B],
		resources: vec![(resource_id, resource_name)],
		threshold: 2,
		balance: initial_bridge_balance,
		swap_fee: SWAP_FEE,
		fee_destination: Some(FEE_DESTINATION),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		assert_eq!(Swap::swap_fee(), SWAP_FEE);
		assert_eq!(Swap::fee_destination().unwrap(), FEE_DESTINATION);
		assert_eq!(Balances::free_balance(Bridge::account_id()), initial_bridge_balance);
		assert!(Bridge::chain_whitelisted(TEST_DESTINATION_CHAIN));
		assert!(Bridge::is_relayer(&RELAYER_A));
		assert!(Bridge::is_relayer(&RELAYER_B));
		assert!(Bridge::resource_exists(resource_id));
		assert_eq!(Bridge::relayer_threshold(), 2);
	});
}
