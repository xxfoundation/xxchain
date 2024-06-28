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
    parameter_types,
    traits::{
        Currency, FindAuthor, Get, Imbalance, OnFinalize, OnInitialize, OnUnbalanced,
        OneSessionHandler, GenesisBuild, ConstU32,
    },
    weights::constants::RocksDbWeight,
};
use frame_system::EnsureRoot;
use pallet_staking::{ConvertCurve, Exposure, ExposureOf, StashOf, StakerStatus};
use sp_core::H256;
use sp_io;
use sp_runtime::{
    curve::PiecewiseLinear,
    testing::{Header, TestXt, UintAuthorityId},
    traits::{IdentityLookup, Zero},
    Perbill,
};
use sp_staking::{EraIndex, SessionIndex};
use std::{cell::RefCell, collections::HashSet};

pub(crate) const INIT_TIMESTAMP: u64 = 30_000;
pub(crate) const BLOCK_TIME: u64 = 1000;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
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

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Authorship: pallet_authorship::{Pallet, Storage},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Historical: pallet_session::historical::{Pallet},
        XXCmix: xx_cmix::{Pallet, Call, Storage, Event<T>, Config<T>},
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

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type Index = AccountIndex;
    type BlockNumber = BlockNumber;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}
impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
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
    type ValidatorIdOf = StashOf<Test>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
    type FullIdentification = Exposure<AccountId, Balance>;
    type FullIdentificationOf = ExposureOf<Test>;
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

pub struct CustodyHandlerMock;

impl pallet_staking::CustodyHandler<AccountId, Balance> for CustodyHandlerMock {
    fn is_custody_account(_account: &AccountId) -> bool {
        false
    }
    fn total_custody() -> Balance {
        Balance::zero() // This isn't used by the staking pallet
    }
}

thread_local! {
    static CUSTODY_ACCOUNTS: RefCell<HashSet<AccountId>> = RefCell::new(Default::default());
    static XX_BLOCK_POINTS: RefCell<u32> = RefCell::new(20); // default block reward is 20. This is to stop existing tests from breaking
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
    type MaxWinners = ConstU32<100>;
	type VotersBound = ConstU32<{ u32::MAX }>;
	type TargetsBound = ConstU32<{ u32::MAX }>;
}

impl pallet_staking::Config for Test {
    type MaxNominations = ConstU32<16>;
    type Currency = Balances;
    type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type RewardRemainder = RewardRemainderMock;
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type BondingDuration = BondingDuration;
    type SessionInterface = Self;
    type EraPayout = ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type WeightInfo = ();
    type CmixHandler = xx_cmix::Module<Test>; // connect up the staking and xx pallets
    type CustodyHandler = CustodyHandlerMock;
    type GenesisElectionProvider = Self::ElectionProvider;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = ConstU32<84>;
	type OnStakerSlash = ();
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
}

impl xx_cmix::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CmixVariablesOrigin = EnsureRoot<AccountId>;
    type AdminOrigin = EnsureRoot<AccountId>;
    type WeightInfo = weights::SubstrateWeight<Self>;
}

pub type Extrinsic = TestXt<RuntimeCall, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    RuntimeCall: From<LocalCall>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = Extrinsic;
}

pub struct ExtBuilder {
    initialize_first_session: bool,
    admin_permission: BlockNumber,
    scheduling_account: Option<AccountId>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            initialize_first_session: true,
            admin_permission: 0,
            scheduling_account: None,
        }
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
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                // controllers
                (10, 100),
                (20, 10),
                // stashes
                (11, 1000),
                (21, 1000),
            ],
        }.assimilate_storage(&mut storage);

        let stakers = vec![
            // (stash, ctrl, stake, status)
            // these two will be elected in the default test where we elect 2.
            (11, 10, 1000,
                StakerStatus::<H256, AccountId>::Validator(Some(H256::repeat_byte(11u8)))),
            (21, 20, 1000,
                StakerStatus::<H256, AccountId>::Validator(Some(H256::repeat_byte(21u8)))),
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
        }.assimilate_storage(&mut storage);

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
		}.assimilate_storage(&mut storage);

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
    start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
    assert_eq!(active_era(), era_index);
    // One way or another, current_era must have changed before the active era, so they must match
    // at this point.
    assert_eq!(current_era(), active_era());
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
        .filter_map(|e| {
            if let RuntimeEvent::XXCmix(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}
