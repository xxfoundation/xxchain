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

//! Test utilities

use crate as xx_team_custody;
use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_election_provider_support::{onchain, SequentialPhragmen};
use frame_support::{
    parameter_types,
    traits::{
        Currency, FindAuthor, Imbalance, OnFinalize, OnInitialize, OnUnbalanced,
        OneSessionHandler, InstanceFilter, LockIdentifier, EqualPrivilegeOnly, ConstU32, ConstU128
    },
    weights::{Weight, constants::RocksDbWeight},
    RuntimeDebug,
};
use frame_system::{EnsureRoot, EnsureSigned};
use pallet_staking::{ConvertCurve, Exposure, ExposureOf, StashOf};
use sp_core::H256;
pub use sp_runtime::{
    curve::PiecewiseLinear,
    testing::{Header, TestXt, UintAuthorityId},
    traits::{IdentityLookup, Zero, BlakeTwo256, ConvertInto},
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
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
        Authorship: pallet_authorship::{Pallet, Storage},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Historical: pallet_session::historical::{Pallet},
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
        Elections: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>},
        XXCustody: xx_team_custody::{Pallet, Call, Storage, Config<T>, Event<T>},
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
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = AccountIndex;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
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
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}
sp_runtime::impl_opaque_keys! {
    pub struct SessionKeys {
        pub other: OtherSessionHandler,
    }
}
impl pallet_session::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = StashOf<Test>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
    type SessionHandler = (OtherSessionHandler,);
    type Keys = SessionKeys;
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

pub struct CmixHandlerMock;

impl pallet_staking::CmixHandler for CmixHandlerMock {
    fn get_block_points() -> u32 {
        XX_BLOCK_POINTS.with(|x| {
            *x.borrow()
        })
    }
    fn end_era() {} // do nothing
}

thread_local! {
    static CUSTODY_ACCOUNTS: RefCell<HashSet<AccountId>> = RefCell::new(Default::default());
    static XX_BLOCK_POINTS: RefCell<u32> = RefCell::new(20); // default block reward is 20. This is to stop existing tests from breaking
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<u64>;
	type BaseDeposit = ConstU128<0>;
	type ByteDeposit = ConstU128<0>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Weight::from_parts(1_000_000_000_000_000, 0);
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<Self::AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = ();
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
    type CmixHandler = CmixHandlerMock;
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

parameter_types! {
    pub const ProxyDepositBase: u64 = 1;
    pub const ProxyDepositFactor: u64 = 1;
    pub const MaxProxies: u16 = 4;
    pub const MaxPending: u32 = 2;
    pub const AnnouncementDepositBase: u64 = 1;
    pub const AnnouncementDepositFactor: u64 = 1;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode,
    RuntimeDebug, MaxEncodedLen, scale_info::TypeInfo,
)]
pub enum ProxyType {
    Any,
    NonTransfer,
    Governance,
    Staking,
    Voting,
}
impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}
impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..)
			),
            ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..) |
				RuntimeCall::Elections(..)
			),
            ProxyType::Staking => matches!(c, RuntimeCall::Staking(..)),
            ProxyType::Voting => matches!(
				c,
				RuntimeCall::Democracy(pallet_democracy::Call::vote { .. } | pallet_democracy::Call::remove_vote { .. }) |
				RuntimeCall::Elections(pallet_elections_phragmen::Call::vote { .. } | pallet_elections_phragmen::Call::remove_voter { .. })
			),
        }
    }
}

impl pallet_proxy::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = ();
    type MaxPending = MaxPending;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 5;
	pub const VotingPeriod: BlockNumber = 5;
	pub const FastTrackVotingPeriod: BlockNumber = 2;
	pub const InstantAllowed: bool = true;
	pub const MinimumDeposit: Balance = 100;
	pub const EnactmentPeriod: BlockNumber = 5;
	pub const CooloffPeriod: BlockNumber = 5;
	pub const PreimageByteDeposit: Balance = 1;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod;
    type MinimumDeposit = MinimumDeposit;
    type ExternalOrigin = EnsureRoot<Self::AccountId>;
    type ExternalMajorityOrigin = EnsureRoot<Self::AccountId>;
    type ExternalDefaultOrigin = EnsureRoot<Self::AccountId>;
    type FastTrackOrigin = EnsureRoot<Self::AccountId>;
    type InstantOrigin = EnsureRoot<Self::AccountId>;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    type CancellationOrigin = EnsureRoot<Self::AccountId>;
    type BlacklistOrigin = EnsureRoot<Self::AccountId>;
    type CancelProposalOrigin = EnsureRoot<Self::AccountId>;
    type VetoOrigin = EnsureSigned<Self::AccountId>;
    type CooloffPeriod = CooloffPeriod;
    type Slash = ();
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = ();
    type MaxProposals = MaxProposals;
    type Preimages = ();
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type SubmitOrigin = EnsureSigned<Self::AccountId>;
}

parameter_types! {
    pub const PayoutFrequency: BlockNumber = 3;
    pub const CustodyDuration: BlockNumber = 100;
    pub const GovernanceCustodyDuration: BlockNumber = 45;
    pub const CustodyProxy: ProxyType = ProxyType::Voting;
}

parameter_types! {
	pub const CandidacyBond: Balance = 100;
	pub const VotingBondBase: Balance = 1;
	pub const VotingBondFactor: Balance = 1;
	pub const TermDuration: BlockNumber = 10;
	pub const DesiredMembers: u32 = 9;
	pub const DesiredRunnersUp: u32 = 10;
    pub const MaxVoters: u32 = 10 * 1000;
    pub const MaxVotesPerVoter: u32 = 16;
	pub const MaxCandidates: u32 = 1000;
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

impl pallet_elections_phragmen::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = ElectionsPhragmenPalletId;
    type Currency = Balances;
    type ChangeMembers = ();
    type InitializeMembers = ();
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = ();
    type KickedMember = ();
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type MaxVoters = MaxVoters;
    type MaxVotesPerVoter = MaxVotesPerVoter;
	type MaxCandidates = MaxCandidates;
    type WeightInfo = ();
}

impl xx_team_custody::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PayoutFrequency = PayoutFrequency;
    type CustodyDuration = CustodyDuration;
    type GovernanceCustodyDuration = GovernanceCustodyDuration;
    type CustodyProxy = CustodyProxy;
    type BlockNumberToBalance = ConvertInto;
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
    team_allocations: Vec<(AccountId, Balance)>,
    initial_balances: Vec<(AccountId, Balance)>,
    custodians: Vec<AccountId>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            initialize_first_session: true,
            team_allocations: Vec::new(),
            initial_balances: Vec::new(),
            custodians: Vec::new(),
        }
    }
}

impl ExtBuilder {

    pub fn with_team_allocations(mut self, team_allocations: &[(AccountId, Balance)]) -> Self {
        self.team_allocations = team_allocations.to_vec();
        self
    }

    pub fn with_custodians(mut self, custodians: &[AccountId]) -> Self {
        self.custodians = custodians.to_vec();
        self
    }

    pub fn with_initial_balances(mut self, initial_balances: &[(AccountId, Balance)]) -> Self {
        self.initial_balances = initial_balances.to_vec();
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        sp_tracing::try_init_simple();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = xx_team_custody::GenesisConfig::<Test> {
            team_allocations: self.team_allocations,
            custodians: self.custodians.into_iter().map(|e| (e, ())).collect(),
        }
        .assimilate_storage(&mut storage);

        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: self.initial_balances
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
                XXCustody::on_initialize(1);
                Timestamp::set_timestamp(INIT_TIMESTAMP);
            });
        }

        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce()) {
        let mut ext = self.build();
        ext.execute_with(test);
        ext.execute_with(post_conditions);
    }
}

fn post_conditions() {}

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
        XXCustody::on_initialize(b);
        Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
        if b != n {
            Staking::on_finalize(System::block_number());
        }
    }
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

pub(crate) fn xx_team_custody_events() -> Vec<xx_team_custody::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::XXCustody(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn proxy_events() -> Vec<pallet_proxy::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::Proxy(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}
