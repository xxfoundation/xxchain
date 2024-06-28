use crate as xx_betanet_rewards;
use crate::*;

use frame_support::{
    parameter_types,
    ord_parameter_types,
    traits::{GenesisBuild, OnInitialize, Imbalance, WithdrawReasons},
    weights::constants::RocksDbWeight,
};
use frame_system::EnsureSignedBy;
use sp_runtime::{
    testing::{Header, TestXt, H256},
    traits::{IdentityLookup, ConvertInto},
};
use sp_io::hashing::keccak_256;
use libsecp256k1::{SecretKey, PublicKey, Message, sign};
use claims::{EthereumAddress, EcdsaSignature, to_ascii_hex};
use std::cell::RefCell;
use std::mem;

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
        Claims: claims::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
        XXBetanetRewards: xx_betanet_rewards::{Pallet, Call, Storage, Config<T>, Event<T>},
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
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types!{
    pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}

ord_parameter_types!{
    pub const Six: AccountId = 6;
}

impl claims::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type VestingSchedule = Vesting;
    type Prefix = Prefix;
    type MoveClaimOrigin = EnsureSignedBy<Six, AccountId>;
    type RewardHandler = XXBetanetRewards;
    type WeightInfo = claims::weights::TestWeightInfo;
}

pub struct MockCustodyHandler;

impl pallet_staking::CustodyHandler<AccountId, Balance> for MockCustodyHandler {
    fn is_custody_account(_: &AccountId) -> bool { false }
    fn total_custody() -> Balance { Balance::zero() }
}

parameter_types! {
	pub const BetanetStakingRewardsBlock: BlockNumber = 10;
	pub const OneMonthVest: BlockNumber = 432_000;
	pub const ThreeMonthVest: BlockNumber = 3*OneMonthVest::get();
	pub const SixMonthVest: BlockNumber = 6*OneMonthVest::get();
	pub const NineMonthVest: BlockNumber = 9*OneMonthVest::get();
	pub const OneYearVest: BlockNumber = 5_259_600;
	pub const Decimals: Balance = 1_000_000_000;
	pub const RewardBalance: Balance = 1_000_000 * Decimals::get();
	pub const RewardAmount: Balance = 10_000 * Decimals::get();
	pub const ClaimBalance: Balance = 25_000 * Decimals::get();
	pub const ClaimRewardAmount: Balance = 5_000 * Decimals::get();
	// ClaimBalance / OneYearVest
	pub const ClaimVestingPerBlock: Balance = 4_753_213;
	// ClaimBalance + ClaimRewardAmount / OneYearVest
	pub const ClaimVestingWithRewardsPerBlock: Balance = 5_703_856;
	// Rewards Pool balance
	pub const RewardsPoolBalance: Balance = 100_000_000 * Decimals::get();
}

thread_local! {
    static REWARD_DEDUCTIONS: RefCell<Balance> = RefCell::new(Default::default());
}

pub struct RewardMock;

impl OnUnbalanced<PositiveImbalanceOf<Test>> for RewardMock {
    fn on_nonzero_unbalanced(amount: PositiveImbalanceOf<Test>) {
        REWARD_DEDUCTIONS.with(|v| {
            *v.borrow_mut() += amount.peek();
            mem::forget(amount)
        });
    }
}

impl RewardMock {
    pub fn total() -> BalanceOf<Test> {
        REWARD_DEDUCTIONS.with(|v| {
            *v.borrow()
        })
    }
}

impl xx_betanet_rewards::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type EnactmentBlock = BetanetStakingRewardsBlock;
    type Reward = RewardMock;
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

pub(crate) fn public(secret: &SecretKey) -> PublicKey {
    PublicKey::from_secret_key(secret)
}

pub(crate) fn eth(secret: &SecretKey) -> EthereumAddress {
    let mut res = EthereumAddress::default();
    res.0.copy_from_slice(&keccak_256(&public(secret).serialize()[1..65])[12..]);
    res
}

pub(crate) fn ethereum_signable_message<T: claims::Config> (what: &[u8], extra: &[u8]) -> Vec<u8> {
    let prefix = T::Prefix::get();
    let mut l = prefix.len() + what.len() + extra.len();
    let mut rev = Vec::new();
    while l > 0 {
        rev.push(b'0' + (l % 10) as u8);
        l /= 10;
    }
    let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
    v.extend(rev.into_iter().rev());
    v.extend_from_slice(&prefix[..]);
    v.extend_from_slice(what);
    v.extend_from_slice(extra);
    v
}

pub(crate) fn sig<T: Config>(secret: &SecretKey, what: &[u8], extra: &[u8]) -> EcdsaSignature {
    let msg = keccak_256(&ethereum_signable_message::<Test>(&to_ascii_hex(what)[..], extra));
    let (sig, recovery_id) = sign(&Message::parse(&msg), secret);
    let mut r = [0u8; 65];
    r[0..64].copy_from_slice(&sig.serialize()[..]);
    r[64] = recovery_id.serialize();
    EcdsaSignature(r)
}

pub(crate) fn alice() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}

pub(crate) fn bob() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Bob")).unwrap()
}

pub(crate) fn charlie() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Charlie")).unwrap()
}

pub(crate) fn dave() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Dave")).unwrap()
}

pub(crate) fn eve() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Eve")).unwrap()
}

pub(crate) fn frank() -> SecretKey {
    SecretKey::parse(&keccak_256(b"Frank")).unwrap()
}

pub struct ExtBuilder {
    with_claims: bool,
    with_vesting: bool,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            with_claims: false,
            with_vesting: false,
        }
    }
}

impl ExtBuilder {

    pub fn with_claims(mut self) -> Self {
        self.with_claims = true;
        self
    }

    pub fn with_vesting(mut self) -> Self {
        self.with_vesting = true;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        sp_tracing::try_init_simple();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                // 5 reward options + default
                (10, RewardBalance::get()),
                (20, RewardBalance::get()),
                (30, RewardBalance::get()),
                (40, RewardBalance::get()),
                (50, RewardBalance::get()),
                (60, RewardBalance::get()),
            ],
        }.assimilate_storage(&mut storage).unwrap();

        if self.with_vesting {
            // add vesting of 1 year to reward accounts
            pallet_vesting::GenesisConfig::<Test> {
                vesting: vec![
                    (10, 0, OneYearVest::get(), 0),
                    (20, 0, OneYearVest::get(), 0),
                    (30, 0, OneYearVest::get(), 0),
                    (40, 0, OneYearVest::get(), 0),
                    (50, 0, OneYearVest::get(), 0),
                    (60, 0, OneYearVest::get(), 0),
                ]
            }.assimilate_storage(&mut storage).unwrap();
        }

        if self.with_claims {
            claims::GenesisConfig::<Test> {
                claims: vec![
                    (eth(&alice()), ClaimBalance::get(), Some(ClaimRewardAmount::get()), None, None),
                    (eth(&bob()), ClaimBalance::get(), Some(ClaimRewardAmount::get()), None, None),
                    (eth(&charlie()), ClaimBalance::get(), Some(ClaimRewardAmount::get()), None, None),
                    (eth(&dave()), ClaimBalance::get(), Some(ClaimRewardAmount::get()), None, None),
                    (eth(&eve()), ClaimBalance::get(), None, None, None),
                    (eth(&frank()), ClaimBalance::get(), None, None, None),
                ],
                vesting: vec![
                    (eth(&bob()), vec![(ClaimBalance::get(), ClaimVestingPerBlock::get(), 0)]),
                    (eth(&dave()), vec![(ClaimBalance::get(), ClaimVestingPerBlock::get(), 0)]),
                ],
            }.assimilate_storage(&mut storage).unwrap();
        }

        xx_betanet_rewards::GenesisConfig::<Test> {
            accounts: vec![
                (10, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
                (20, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
                (30, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
                (40, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
                (50, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
                (60, UserInfo::<Balance> {
                    principal: RewardBalance::get(),
                    reward: RewardAmount::get(),
                    option: RewardOption::Vesting6Month,
                }),
            ],
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
        XXBetanetRewards::on_initialize(b);
    }
}

pub(crate) fn xx_betanet_rewards_events() -> Vec<xx_betanet_rewards::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::XXBetanetRewards(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn confirm_reward_result(who: &AccountId, option: RewardOption, claim: bool) -> Balance {
    // Compute reward
    let reward = if claim {
        (option.extra_rewards() * ClaimRewardAmount::get()) + (option.rewards() * ClaimRewardAmount::get())
    } else {
        (option.extra_rewards() * RewardAmount::get()) + (option.rewards() * RewardAmount::get())
    };

    // Check balance
    let expected_balance = if claim {
        ClaimBalance::get()
    } else {
        RewardBalance::get()
    };
    assert_eq!(
        Balances::free_balance(who),
        expected_balance + reward,
    );

    // Check the sum of vesting schedules lock at least the required amount
    match option {
        RewardOption::NoVesting => (),
        _ => {
            let min_locked = (option.principal_lock() * expected_balance) + reward;
            let locked = Vesting::vesting(who).unwrap().iter().fold(0u128, |acc, x| {
                acc + x.locked_at::<ConvertInto>(BetanetStakingRewardsBlock::get())
            });
            assert!(locked >= min_locked);
        }
    }

    // Return reward
    reward
}

pub(crate) fn confirm_claim_rewards_added(who: &AccountId) {
    assert_eq!(
        <Accounts<Test>>::get(who),
        UserInfo {
            principal: ClaimBalance::get(),
            reward: ClaimRewardAmount::get(),
            option: RewardOption::Vesting6Month,
        }
    );
}


pub(crate) fn confirm_leftover_claim_rewards_added(who: &EthereumAddress, vesting: bool) -> Balance {
    // Check reward was added to claim value
    assert_eq!(
        <claims::Claims<Test>>::get(who).unwrap(),
        ClaimBalance::get()+ClaimRewardAmount::get(),
    );

    // Check claim vesting schedule was changed to include reward amount
    if vesting {
        let option = RewardOption::default();
        let min_locked = (option.principal_lock() * ClaimBalance::get()) + ClaimRewardAmount::get();
        let locked = <claims::Vesting<Test>>::get(who).unwrap().iter().fold(0u128, |acc, x| {
            let vs = pallet_vesting::VestingInfo::new(x.0, x.1, x.2);
            acc + vs.locked_at::<ConvertInto>(BetanetStakingRewardsBlock::get())
        });
        assert!(locked >= min_locked);
    }
    // Leftover claims always get default option
    ClaimRewardAmount::get()
}

pub(crate) fn confirm_leftover_claim_without_rewards(who: &EthereumAddress) {
    // Check claim value didn't change
    assert_eq!(
        <claims::Claims<Test>>::get(who).unwrap(),
        ClaimBalance::get(),
    );
}
