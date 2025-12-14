// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Added as part the code review and testing
// by ChainSafe Systems Aug 2021

use crate as xx_cmix;
use crate::*;

use frame_election_provider_support::{onchain, SequentialPhragmen};
use frame_support::{
	derive_impl, parameter_types,
	traits::{
		ConstU32, Currency, FindAuthor, Get, Imbalance, OnFinalize, OnInitialize, OnUnbalanced,
		OneSessionHandler,
	},
};
use frame_system::EnsureRoot;
use pallet_staking::{ConvertCurve, Exposure, StakerStatus};
use sp_io;
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::UintAuthorityId,
	traits::{IdentityLookup, Zero},
	BuildStorage, Perbill,
};
use sp_staking::{currency_to_vote::SaturatingCurrencyToVote, EraIndex, SessionIndex};
use std::cell::RefCell;

pub(crate) const INIT_TIMESTAMP: u64 = 30_000;
pub(crate) const BLOCK_TIME: u64 = 1000;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Authorship: pallet_authorship,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		Staking: pallet_staking,
		Session: pallet_session,
		Historical: pallet_session::historical,
		XXStakingExtension: xx_staking_extension,
		XXCmix: xx_cmix,
	}
);

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<AccountId> for Author11 {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some(11)
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaxLocks: u32 = 1024;
	pub static SessionsPerEra: SessionIndex = 3;
	pub static ExistentialDeposit: Balance = 1;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}
sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
impl pallet_session::Config for Test {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = sp_runtime::traits::ConvertInto;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
	type DisablingStrategy = pallet_session::disabling::UpToLimitDisablingStrategy;
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pallet_session::historical::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::DefaultExposureOf<Test>;
}
impl pallet_authorship::Config for Test {
	type FindAuthor = Author11;
	type EventHandler = pallet_staking::Pallet<Test>;
}
parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}
pallet_staking_reward_curve::build! {
	const I_NPOS: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}
parameter_types! {
	pub const BondingDuration: EraIndex = 3;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &I_NPOS;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(75);
}

thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct RewardRemainderMock;

type NegativeImbalanceOf<T> = <<T as pallet_staking::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

impl OnUnbalanced<NegativeImbalanceOf<Test>> for RewardRemainderMock {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Test>) {
		REWARD_REMAINDER_UNBALANCED.with(|v| {
			*v.borrow_mut() += amount.peek();
		});
		drop(amount);
	}
}

thread_local! {
	static XX_BLOCK_POINTS: RefCell<u32> = RefCell::new(20); // default block reward is 20. This is to stop existing tests from breaking
}

parameter_types! {
	pub ElectionBoundsOnChain: frame_election_provider_support::bounds::ElectionBounds =
		frame_election_provider_support::bounds::ElectionBoundsBuilder::default()
			.voters_count(10_000.into())
			.targets_count(1_500.into())
			.build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type Sort = ();
	type MaxBackersPerWinner = ConstU32<1000>;
	type MaxWinnersPerPage = ConstU32<100>;
	type Bounds = ElectionBoundsOnChain;
}

parameter_types! {
	pub static MaxControllersInDeprecationBatch: u32 = 5900;
}

impl pallet_staking::Config for Test {
	type Currency = Balances;
	type OldCurrency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = Self;
	type EraPayout = ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxExposurePageSize = ConstU32<64>;
	type MaxValidatorSet = ConstU32<300>;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type WeightInfo = ();
	type GenesisElectionProvider = Self::ElectionProvider;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type HistoryDepth = ConstU32<84>;
	type EventListeners = ();
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type Filter = ();
}

impl xx_staking_extension::Config for Test {}

impl xx_cmix::Config for Test {
	type CmixVariablesOrigin = EnsureRoot<AccountId>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::SubstrateWeight<Self>;
}

pub struct ExtBuilder {
	initialize_first_session: bool,
	admin_permission: BlockNumber,
	scheduling_account: Option<AccountId>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { initialize_first_session: true, admin_permission: 0, scheduling_account: None }
	}
}

impl ExtBuilder {
	pub fn with_admin_permission(mut self, admin_permission: BlockNumber) -> Self {
		self.admin_permission = admin_permission;
		self
	}

	pub fn with_scheduling_account(mut self, scheduling_account: AccountId) -> Self {
		self.scheduling_account = Some(scheduling_account);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				// stashes (stash = controller now, need extra for existential deposit)
				(11, 10000),
				(21, 10000),
			],
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let stakers = vec![
			// (stash, ctrl, stake, status)
			// these two will be elected in the default test where we elect 2.
			(11, 11, 1000, StakerStatus::<AccountId>::Validator),
			(21, 21, 1000, StakerStatus::<AccountId>::Validator),
		];
		let _ = pallet_staking::GenesisConfig::<Test> {
			stakers: stakers.clone(),
			validator_count: 2,
			minimum_validator_count: 0,
			invulnerables: vec![],
			slash_reward_fraction: Perbill::from_percent(10),
			min_nominator_bond: ExistentialDeposit::get(),
			min_validator_bond: ExistentialDeposit::get(),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = xx_cmix::GenesisConfig::<Test> {
			admin_permission: self.admin_permission,
			scheduling_account: self.scheduling_account,
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = pallet_session::GenesisConfig::<Test> {
			keys: stakers
				.into_iter()
				.map(|(id, ..)| (id, id, SessionKeys { other: id.into() }))
				.collect(),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);
		if self.initialize_first_session {
			// We consider all test to start after timestamp is initialized This must be ensured by
			// having `timestamp::on_initialize` called before `staking::on_initialize`. Also, if
			// session length is 1, then it is already triggered.
			ext.execute_with(|| {
				System::set_block_number(1);
				Session::on_initialize(1);
				Staking::on_initialize(1);
				XXCmix::on_initialize(1);
				Timestamp::set_timestamp(INIT_TIMESTAMP);
			});
		}

		ext
	}
	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		let mut ext = self.build();
		ext.execute_with(test);
		ext.execute_with(post_conditions);
	}
}

fn post_conditions() {}

// staking pallet still controls the eras
pub(crate) fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
}

pub(crate) fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
}

/// Progress to the given block, triggering session and era changes as we progress.
///
/// This will finalize the previous block, initialize up to the given block, essentially simulating
/// a block import/propose process where we first initialize the block, then execute some stuff (not
/// in the function), and then finalize the block.
pub(crate) fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		Staking::on_initialize(b);
		XXCmix::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

/// Progresses from the current block number (whatever that may be) to the `P * session_index + 1`.
pub(crate) fn start_session(session_index: SessionIndex) {
	let end: u64 = if Offset::get().is_zero() {
		(session_index as u64) * Period::get()
	} else {
		Offset::get() + (session_index.saturating_sub(1) as u64) * Period::get()
	};
	run_to_block(end);
	// session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

/// Progress until the given era.
pub(crate) fn start_active_era(era_index: EraIndex) {
	start_session(era_index * <SessionsPerEra as Get<u32>>::get());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era, so they must match
	// at this point.
	assert_eq!(current_era(), active_era());
	// In the new SDK, we need to explicitly call end_era to trigger cmix variable updates
	// This simulates what the forked substrate's staking pallet would call at era boundaries
	<XXCmix as xx_cmix::CmixHandler>::end_era();
}

#[macro_export]
macro_rules! assert_session_era {
	($session:expr, $era:expr) => {
		assert_eq!(
			Session::current_index(),
			$session,
			"wrong session {} != {}",
			Session::current_index(),
			$session,
		);
		assert_eq!(
			Staking::current_era().unwrap(),
			$era,
			"wrong current era {} != {}",
			Staking::current_era().unwrap(),
			$era,
		);
	};
}

pub(crate) fn xx_cmix_events() -> Vec<xx_cmix::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| if let RuntimeEvent::XXCmix(inner) = e { Some(inner) } else { None })
		.collect()
}
