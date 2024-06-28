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

use crate as xx_economics;
use crate::*;

use frame_support::{
    parameter_types,
    ord_parameter_types,
    traits::{
        OnUnbalanced,
    },
    weights::constants::RocksDbWeight,
};
use frame_system::{EnsureSignedBy};
use sp_runtime::{
    testing::{Header, TestXt, H256},
    traits::IdentityLookup,
    Perbill,
};


/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        XXEconomics: xx_economics::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);


parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxLocks: u32 = 1024;
    pub static ExistentialDeposit: Balance = 1;
    pub static SlashDeferDuration: sp_staking::EraIndex = 0;
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
parameter_types! {
    pub const UncleGenerations: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}

pub struct MockCustodyHandler;

impl pallet_staking::CustodyHandler<AccountId, Balance> for MockCustodyHandler {
    fn is_custody_account(_: &AccountId) -> bool { false }
    fn total_custody() -> Balance {
        1000u128
    }
}

pub const MOCK_TREASURY: &AccountId = &1337;
pub const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

// allows funds to be deposited in a mock treasury account
pub struct MockTreasury<Test>(sp_std::marker::PhantomData<Test>);
impl OnUnbalanced<NegativeImbalanceOf<Test>> for MockTreasury<Test> {
    fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Test>) {
        // add balance to mock treasury account
        <Test as Config>::Currency::resolve_creating(&MOCK_TREASURY, amount);
    }
}

parameter_types! {
    pub const RewardsPoolId: PalletId = PalletId(*b"xx/rwrds");
    pub const EraDuration: BlockNumber = 10; // 10 blocks per era
}

ord_parameter_types! {
    pub const AdminAccount: AccountId = 99;
}

pub struct MockPublicAccountsHandler;

impl xx_public::PublicAccountsHandler<AccountId> for MockPublicAccountsHandler {
    fn accounts() -> Vec<AccountId> {
        return vec![42, 43]
    }
}

pub type TestAdminOrigin = EnsureSignedBy<AdminAccount, AccountId>;

impl xx_economics::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type CustodyHandler = MockCustodyHandler;
    type PublicAccountsHandler = MockPublicAccountsHandler;
    type RewardsPoolId = RewardsPoolId;
    type RewardRemainder = MockTreasury<Test>;
    type EraDuration = EraDuration;
    type AdminOrigin = TestAdminOrigin;
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
    rewards_balance: BalanceOf<Test>,
    liquidity_balance: BalanceOf<Test>,
    interest_points: Vec<inflation::IdealInterestPoint<BlockNumber>>,
    with_public: bool,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            rewards_balance: Default::default(),
            liquidity_balance: Default::default(),
            interest_points: Default::default(),
            with_public: false,
        }
    }
}

impl ExtBuilder {

    pub fn with_rewards_balance(mut self, rewards_balance: BalanceOf<Test>) -> Self {
        self.rewards_balance = rewards_balance;
        self
    }

    pub fn with_liquidity_balance(mut self, liquidity_balance: BalanceOf<Test>) -> Self {
        self.liquidity_balance = liquidity_balance;
        self
    }

    pub fn with_interest_points(mut self, points: Vec<inflation::IdealInterestPoint<BlockNumber>>) -> Self {
        self.interest_points = points;
        self
    }

    pub fn with_public_accounts(mut self) -> Self {
        self.with_public = true;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        sp_tracing::try_init_simple();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        xx_economics::GenesisConfig::<Test> {
            balance: self.rewards_balance,
            liquidity_rewards: self.liquidity_balance,
            interest_points: self.interest_points,
            ..Default::default()
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        if self.with_public {
            pallet_balances::GenesisConfig::<Test> {
                balances: vec![
                    (42, 1000),
                    (43, 1000),
                ]
            }.assimilate_storage(&mut storage).unwrap();
        }


        let ext = sp_io::TestExternalities::from(storage);
        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();

        ext.execute_with(|| {
            System::set_block_number(1);
        });

        ext.execute_with(test);
    }
}

pub(crate) fn run_to_block(n: BlockNumber) {
    for b in (System::block_number() + 1)..=n {
        System::set_block_number(b);
    }
}

pub(crate) fn xx_economics_events() -> Vec<xx_economics::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::XXEconomics(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}
