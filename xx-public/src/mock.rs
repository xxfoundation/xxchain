use crate as xx_public;
use crate::*;

use frame_support::{
    parameter_types,
    ord_parameter_types,
    traits::{GenesisBuild, ConstU32, WithdrawReasons},
    weights::constants::RocksDbWeight,
};
use frame_system::EnsureSignedBy;
use sp_runtime::{
    testing::{Header, TestXt, H256},
    traits::{IdentityLookup, ConvertInto},
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
        Vesting: pallet_vesting::{Pallet, Call, Storage, Config<T>, Event<T>},
        XXPublic: xx_public::{Pallet, Call, Storage, Config<T>, Event},
    }
);


parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxLocks: u32 = 1024;
    pub static ExistentialDeposit: Balance = 1;
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
    type MaxConsumers = ConstU32<16>;
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
    pub const MinVestedTransfer: u64 = 0;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = ();
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    const MAX_VESTING_SCHEDULES: u32 = 2;
}

parameter_types! {
    pub const TestnetId: PalletId = PalletId(*b"xx/tstnt");
	pub const SaleId: PalletId = PalletId(*b"xx//sale");
}

ord_parameter_types! {
    pub const AdminAccount: AccountId = 99;
}

pub type TestAdminOrigin = EnsureSignedBy<AdminAccount, AccountId>;

impl xx_public::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type VestingSchedule = Vesting;
    type TestnetId = TestnetId;
    type SaleId = SaleId;
    // Admin is technical committee unanimity
    type AdminOrigin = TestAdminOrigin;
    type WeightInfo = ();
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
    testnet_balance: BalanceOf<Test>,
    sale_balance: BalanceOf<Test>,
    vesting: bool,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            testnet_balance: Default::default(),
            sale_balance: Default::default(),
            vesting: false,
        }
    }
}

impl ExtBuilder {

    pub fn with_testnet_balance(mut self, testnet_balance: BalanceOf<Test>) -> Self {
        self.testnet_balance = testnet_balance;
        self
    }

    pub fn with_sale_balance(mut self, sale_balance: BalanceOf<Test>) -> Self {
        self.sale_balance = sale_balance;
        self
    }

    pub fn with_vesting(mut self) -> Self {
        self.vesting = true;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        sp_tracing::try_init_simple();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                // Give managers some coins
                (42, 100),
                (43, 100),
                (12, 100),
                (13, 100),
            ],
        }.assimilate_storage(&mut storage).unwrap();

        if self.vesting {
            pallet_vesting::GenesisConfig::<Test> {
                vesting: vec![
                    (12, 0, 100, 0),
                    (13, 0, 100, 0),
                ]
            }.assimilate_storage(&mut storage).unwrap();
        }

        xx_public::GenesisConfig::<Test> {
            testnet_manager: Some(42u64),
            sale_manager: Some(43u64),
            testnet_balance: self.testnet_balance,
            sale_balance: self.sale_balance,
        }.assimilate_storage(&mut storage).unwrap();

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

pub(crate) fn xx_public_events() -> Vec<xx_public::Event> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::XXPublic(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}
