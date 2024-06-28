#![cfg(test)]

use super::*;

use frame_support::{parameter_types, PalletId};
use frame_system::{self as system};
use sp_core::hashing::blake2_128;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

pub use crate::{self as swap, Config};
pub use chainbridge as bridge;

pub use pallet_balances as balances;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const MaxLocks: u32 = 100;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

const PALLET_ID: PalletId = PalletId(*b"cb/bridg");

parameter_types! {
    pub const TestChainId: u8 = 5;
    pub const ProposalLifetime: u64 = 100;
    pub const ChainbridgePalletId: PalletId = PALLET_ID;
}

impl bridge::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Proposal = RuntimeCall;
    type ChainId = TestChainId;
    type ProposalLifetime = ProposalLifetime;
    type PalletId = ChainbridgePalletId;
}

parameter_types! {
    pub NativeTokenId: bridge::ResourceId = bridge::derive_resource_id(1, &blake2_128(b"DAV"));
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type BridgeOrigin = bridge::EnsureBridge<Test>;
    type AdminOrigin = bridge::EnsureBridge<Test>;
    type Currency = Balances;
    type NativeTokenId = NativeTokenId;
    type WeightInfo = weights::SubstrateWeight<Self>;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, RuntimeCall, ()>;

pub type AccountId = <Test as frame_system::Config>::AccountId;
pub type Balance = <Test as balances::Config>::Balance;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{Pallet, Call, Event<T>},
        Balances: balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Bridge: bridge::{Pallet, Call, Storage, Event<T>},
        Swap: swap::{Pallet, Call, Event<T>}
    }
);

pub const FEE_DESTINATION: u64 = 0x1;
pub const ACCOUNT_A: u64 = 0x2;
pub const SWAP_FEE: u64 = 100;

pub const RELAYER_A: u64 = 0x3;
pub const RELAYER_B: u64 = 0x4;
pub const RELAYER_C: u64 = 0x5;

pub const RELAYER_THRESHOLD: u32 = 2;


pub fn new_test_ext(initial_balances: &[(AccountId, Balance)]) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    swap::GenesisConfig::<Test> {
        chains: vec![],
        relayers: vec![],
        resources: vec![],
        threshold: 2,
        balance: 0,
        swap_fee: SWAP_FEE,
        fee_destination: Some(FEE_DESTINATION),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    balances::GenesisConfig::<Test> {
        balances: initial_balances.to_vec(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn last_event() -> RuntimeEvent {
    system::Pallet::<Test>::events()
        .pop()
        .map(|e| e.event)
        .expect("Event expected")
}

pub fn expect_event<E: Into<RuntimeEvent>>(e: E) {
    assert_eq!(last_event(), e.into());
}


// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
    let mut actual: Vec<RuntimeEvent> = system::Pallet::<Test>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        println!("{:?}", next);
        assert_eq!(next, evt.into(), "Events don't match");
    }
}
