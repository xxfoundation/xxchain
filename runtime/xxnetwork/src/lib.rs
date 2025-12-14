// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

extern crate alloc;
use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		tokens::nonfungibles_v2::Inspect, AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64,
		Contains, EitherOf, EitherOfDiverse, EqualPrivilegeOnly, InstanceFilter,
		KeyOwnerProofSystem,
	},
	weights::{constants::RocksDbWeight, ConstantMultiplier, Weight},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureWithSuccess};
pub use node_primitives::{AccountId, Signature};
use node_primitives::{Balance, BlockNumber, Hash, Index, Moment};
use pallet_revive::evm::runtime::EthExtra;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{Perbill, RuntimeDebug};
use sp_staking::currency_to_vote::U128CurrencyToVote;
/// Nonce type alias for compatibility with pallet-revive macros.
pub type Nonce = Index;
use frame_election_provider_support::{onchain, SequentialPhragmen};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::{
	disabling::UpToLimitDisablingStrategy, historical as pallet_session_historical,
};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
pub use pallet_transaction_payment::{FungibleAdapter, Multiplier, TargetedFeeAdjustment};
use runtime_common::constants::currency::{deposit, UNITS};
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		self, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, IdentityLookup,
		NumberFor, OpaqueKeys, SaturatedConversion, StaticLookup,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Runtime common stuff
use runtime_common::{
	constants::{fee::*, time::*},
	impls::*,
	*,
};
use sp_runtime::generic::Era;

use sp_io::hashing::sha2_256;

// Migrations
use migrations::{bridge_adjust::BridgeAdjust, cmix_id_migration::CmixIdMigration};

mod weights;

/// Generated voter bag information.
mod voter_bags;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is \
						built with `SKIP_WASM_BUILD` flag and it is only usable for \
						production chains. Please rebuild with the flag disabled.",
	)
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("xxnetwork"),
	impl_name: alloc::borrow::Cow::Borrowed("xxlabs-xxnetwork"),
	authoring_version: 0,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 207, // Bumped for SDK upgrade
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2, // Bumped for transaction format changes
	system_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 55;
}

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(call: &RuntimeCall) -> bool {
		// These modules are all allowed to be called by transactions
		match call {
			// Disable uniques pallet, since Nfts one should be used instead
			RuntimeCall::Uniques(_) => false,

			// System pallets
			RuntimeCall::System(_) | RuntimeCall::Scheduler(_) | RuntimeCall::Preimage(_) |
			// Block production and Balances
			RuntimeCall::Babe(_) | RuntimeCall::Balances(_) | RuntimeCall::Timestamp(_) |
			// Consensus support
			RuntimeCall::Staking(_) | RuntimeCall::ElectionProviderMultiPhase(_) |
			RuntimeCall::Session(_) | RuntimeCall::Grandpa(_) | RuntimeCall::ImOnline(_) | RuntimeCall::VoterList(_) |
			// Governance
			RuntimeCall::Democracy(_) | RuntimeCall::Council(_) | RuntimeCall::TechnicalCommittee(_) |
			RuntimeCall::Elections(_) | RuntimeCall::TechnicalMembership(_) | RuntimeCall::Treasury(_) |
			// Claims
			RuntimeCall::Claims(_) |
			// Misc
			RuntimeCall::Vesting(_) | RuntimeCall::Utility(_) | RuntimeCall::Identity(_) |
			RuntimeCall::Proxy(_) | RuntimeCall::Bounties(_) | RuntimeCall::ChildBounties(_) | RuntimeCall::Tips(_) |
			RuntimeCall::Multisig(_) | RuntimeCall::Recovery(_) | RuntimeCall::Assets(_) | RuntimeCall::Nfts(_) |
			// XX Network
			RuntimeCall::XXCmix(_) | RuntimeCall::XXEconomics(_) |
			RuntimeCall::XXPublic(_) | RuntimeCall::XXStakingExtension(_) |
			RuntimeCall::XXCustody(_) |
			// ChainBridge and Swap
			RuntimeCall::ChainBridge(_) | RuntimeCall::Swap(_) |
			// Smart Contracts
			RuntimeCall::Revive(_)
			=> true,
		}
	}
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = BaseFilter;
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeTask = RuntimeTask;
	type Nonce = Index;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type ExtensionsWeightInfo = weights::frame_system_extensions::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
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
				RuntimeCall::Assets(..)
					| RuntimeCall::Uniques(..)
					| RuntimeCall::Nfts(..)
					| RuntimeCall::Balances(..)
					| RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..)
					| RuntimeCall::Council(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::Elections(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::Preimage(..)
					| RuntimeCall::Bounties(_)
					| RuntimeCall::ChildBounties(_)
			),
			ProxyType::Staking => matches!(c, RuntimeCall::Staking(..)),
			ProxyType::Voting => matches!(
				c,
				RuntimeCall::Democracy(
					pallet_democracy::Call::vote { .. }
						| pallet_democracy::Call::remove_vote { .. }
				) | RuntimeCall::Elections(
					pallet_elections_phragmen::Call::vote { .. }
						| pallet_elections_phragmen::Call::remove_voter { .. }
				)
			),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = System;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

// Epoch duration is 8 hours in prod, 4 minutes in dev (fast-runtime)
const EPOCH_DURATION_IN_BLOCKS: BlockNumber = prod_or_fast!(8 * HOURS, 4 * MINUTES);

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
	type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}

parameter_types! {
	pub const SessionKeyDeposit: Balance = 100 * UNITS;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
	type DisablingStrategy = UpToLimitDisablingStrategy;
	type Currency = Balances;
	type KeyDeposit = SessionKeyDeposit;
}

impl pallet_session::historical::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::DefaultExposureOf<Runtime>;
}

parameter_types! {
	pub const RewardsPoolId: PalletId = PalletId(*b"xx/rwrds");
	pub const EraDuration: BlockNumber = SessionsPerEra::get() * EPOCH_DURATION_IN_BLOCKS;
	pub const TestnetId: PalletId = PalletId(*b"xx/tstnt");
	pub const SaleId: PalletId = PalletId(*b"xx//sale");
}

type EnsureTechnicalUnanimity = EitherOfDiverse<
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	EnsureRoot<AccountId>,
>;

type EnsureTwoThirdsCouncil = EitherOfDiverse<
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	EnsureRoot<AccountId>,
>;

impl xx_economics::Config for Runtime {
	type Currency = Balances;

	type PublicAccountsHandler = XXPublic;

	// Rewards Pool config
	type RewardsPoolId = RewardsPoolId;
	// rewards remainder are taken from pool (until empty) and passed to Treasury
	type RewardRemainder = Treasury;

	// Inflation config
	type EraDuration = EraDuration;

	// Admin is technical committee unanimity
	type AdminOrigin = EnsureTechnicalUnanimity;

	type WeightInfo = weights::xx_economics::WeightInfo<Runtime>;
}

impl xx_staking_extension::Config for Runtime {}

impl xx_cmix::Config for Runtime {
	// CMIX Variables can be changed by 2/3 Council
	type CmixVariablesOrigin = EnsureTwoThirdsCouncil;
	// Admin is 2/3 technical committee
	type AdminOrigin = EnsureTwoThirdsTechnical;
	// Weight information for extrinsics in this pallet.
	type WeightInfo = weights::xx_cmix::WeightInfo<Runtime>;
}

impl xx_public::Config for Runtime {
	type VestingSchedule = Vesting;
	type TestnetId = TestnetId;
	type SaleId = SaleId;
	// Admin is technical committee unanimity
	type AdminOrigin = EnsureTechnicalUnanimity;
	// Weight information for extrinsics in this pallet.
	type WeightInfo = weights::xx_public::WeightInfo<Runtime>;
}

parameter_types! {
	/// Custody payout frequency
	pub const CustodyPayoutFrequency: BlockNumber = 14 * DAYS;
	/// Custody duration (total time team tokens are under custody)
	pub const CustodyDuration: BlockNumber = 2 * 365 * DAYS;
	/// Governance custody duration (time during which custodians control governance)
	pub const GovernanceCustodyDuration: BlockNumber = 180 * DAYS;
	/// Custody proxy type - use Voting for governance
	pub const CustodyProxy: ProxyType = ProxyType::Voting;
}

impl xx_team_custody::Config for Runtime {
	type Currency = Balances;
	type PayoutFrequency = CustodyPayoutFrequency;
	type CustodyDuration = CustodyDuration;
	type GovernanceCustodyDuration = GovernanceCustodyDuration;
	type CustodyProxy = CustodyProxy;
	type BlockNumberToBalance = sp_runtime::traits::ConvertInto;
	// Admin is technical committee unanimity
	type AdminOrigin = EnsureTechnicalUnanimity;
	// Weight information for extrinsics in this pallet.
	type WeightInfo = weights::xx_team_custody::WeightInfo<Runtime>;
}

parameter_types! {
	pub ElectionBoundsOnChain: frame_election_provider_support::bounds::ElectionBounds =
		frame_election_provider_support::bounds::ElectionBoundsBuilder::default()
			.voters_count(MaxOnChainElectingVoters::get().into())
			.targets_count((MaxOnChainElectableTargets::get() as u32).into())
			.build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = weights::frame_election_provider_support::WeightInfo<Runtime>;
	type MaxBackersPerWinner = ConstU32<1024>;
	type MaxWinnersPerPage = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds = ElectionBoundsOnChain;
	type Sort = ();
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

parameter_types! {
	pub const MaxExposurePageSize: u32 = 256;
	pub const MaxControllersInDeprecationBatch: u32 = 100;
}

/// Type alias for staking negative imbalance (slashes, remainders) - NegativeImbalanceOf = Credit
pub type StakingNegativeImbalance = frame_support::traits::fungible::Credit<AccountId, Balances>;

/// Type alias for staking positive imbalance (rewards) - PositiveImbalanceOf = Debt
pub type StakingPositiveImbalance = frame_support::traits::fungible::Debt<AccountId, Balances>;

/// Staking reward handler - handles validator/nominator rewards
/// In staking, rewards are Debt (tokens owed to validators that were already minted)
/// We settle part of this debt against the rewards pool, the remainder increases issuance
pub struct StakingRewardHandler;

impl frame_support::traits::OnUnbalanced<StakingPositiveImbalance> for StakingRewardHandler {
	fn on_nonzero_unbalanced(reward: StakingPositiveImbalance) {
		use frame_support::traits::{
			fungible::{Balanced, Inspect},
			tokens::imbalance::Imbalance,
		};
		use sp_runtime::traits::Zero;

		// Get current rewards pool balance
		let rewards_account = XXEconomics::rewards_account_id();
		let pool_balance = <Balances as Inspect<_>>::balance(&rewards_account);

		let reward_amount = reward.peek();

		// Calculate how much to withdraw from pool vs mint
		let withdraw_amount = reward_amount.min(pool_balance);
		let mint_amount = reward_amount.saturating_sub(withdraw_amount);

		// Split the debt: part to settle from pool, part to mint
		let (pool_debt, mint_debt) = reward.split(withdraw_amount);

		// Settle the pool portion - this withdraws from rewards account to offset the debt
		if !withdraw_amount.is_zero() {
			use frame_support::traits::tokens::Preservation;
			let _ = <Balances as Balanced<_>>::settle(
				&rewards_account,
				pool_debt,
				Preservation::Expendable,
			)
			.expect("pool_balance was checked; qed");
			XXEconomics::deposit_event(xx_economics::Event::RewardFromPool(withdraw_amount));
		} else {
			drop(pool_debt);
		}

		// The remaining debt (mint_debt) will increase total issuance when dropped
		if !mint_amount.is_zero() {
			XXEconomics::deposit_event(xx_economics::Event::RewardMinted(mint_amount));
		}
		drop(mint_debt);
	}
}

/// Staking slash handler - sends slashed funds to Treasury
/// In staking, slashes are Credit (tokens taken from validators to distribute)
pub struct StakingSlashHandler;

impl frame_support::traits::OnUnbalanced<StakingNegativeImbalance> for StakingSlashHandler {
	fn on_nonzero_unbalanced(slash: StakingNegativeImbalance) {
		use frame_support::traits::fungible::Balanced;

		// Send slashed funds to Treasury
		let treasury_account = Treasury::account_id();
		let _ = <Balances as Balanced<_>>::resolve(&treasury_account, slash);
	}
}

/// Staking reward remainder handler - handles leftover rewards (sends to Treasury)
/// In staking, remainders are Credit (tokens that weren't distributed as rewards)
pub struct StakingRemainderHandler;

impl frame_support::traits::OnUnbalanced<StakingNegativeImbalance> for StakingRemainderHandler {
	fn on_nonzero_unbalanced(remainder: StakingNegativeImbalance) {
		use frame_support::traits::fungible::Balanced;

		// Send remainder to Treasury
		let treasury_account = Treasury::account_id();
		let _ = <Balances as Balanced<_>>::resolve(&treasury_account, remainder);
	}
}

/// No-op reward handler for claims pallet
/// Betanet rewards program has ended - claims no longer trigger reward tracking
pub struct NoOpClaimsRewardHandler;

impl claims::RewardHandler<AccountId, Balance> for NoOpClaimsRewardHandler {
	fn add_claimed(_dest: AccountId, _claim: Balance, _reward: Balance) {
		// Betanet rewards program discontinued - no action taken
	}
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type OldCurrency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RewardRemainder = StakingRemainderHandler;
	type RuntimeEvent = RuntimeEvent;
	type Slash = StakingSlashHandler;
	type Reward = StakingRewardHandler;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type SessionInterface = Self;
	type EraPayout = XXEconomics; // era payout is calculated according to inflation parameters
	type NextNewSession = Session;
	type MaxExposurePageSize = MaxExposurePageSize;
	type MaxValidatorSet = ConstU32<1000>;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	// Use Bags List Pallet
	type VoterList = VoterList;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxUnlockingChunks = ConstU32<32>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type HistoryDepth = ConstU32<84>;
	type EventListeners = ();
	type Filter = ();
	type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 32;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type MaxBackersPerWinner = ConstU32<1024>;
	type Solution = NposSolution16;
	type MaxVotesPerVoter = <
		<Self as pallet_election_provider_multi_phase::Config>::DataProvider
		as
		frame_election_provider_support::ElectionDataProvider
	>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

parameter_types! {
	pub ElectionBoundsMultiPhase: frame_election_provider_support::bounds::ElectionBounds =
		frame_election_provider_support::bounds::ElectionBoundsBuilder::default()
			.voters_count(MaxElectingVoters::get().into())
			.targets_count((MaxElectableTargets::get() as u32).into())
			.build();
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type UnsignedPhase = UnsignedPhase;
	type SignedPhase = SignedPhase;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedMaxWeight = MinerMaxWeight;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase =
		GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = SignedMaxRefunds;
	type SignedDepositWeight = ();
	type SlashHandler = ();
	type RewardHandler = XXEconomics;
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureTwoThirdsCouncil;
	type MaxWinners = MaxActiveValidators;
	type MaxBackersPerWinner = ConstU32<1024>;
	type ElectionBounds = ElectionBoundsMultiPhase;
	type BenchmarkingConfig = BenchmarkConfig;
	type WeightInfo = weights::pallet_election_provider_multi_phase::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

impl pallet_bags_list::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
	type MaxAutoRebagPerBlock = ConstU32<0>;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 7 * DAYS;
	pub const VotingPeriod: BlockNumber = 7 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub const InstantAllowed: bool = true;
	pub const EnactmentPeriod: BlockNumber = 7 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
}

type EnsureTwoThirdsTechnical = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>,
>;

type EnsureRootOrSixtyCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EitherOfDiverse<
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
		EnsureRoot<AccountId>,
	>;
	/// A 60% super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EnsureRootOrSixtyCouncil;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = EitherOfDiverse<
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
		EnsureRoot<AccountId>,
	>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = EnsureTwoThirdsTechnical;
	type InstantOrigin = EnsureTechnicalUnanimity;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = EnsureTwoThirdsCouncil;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureTechnicalUnanimity;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = weights::pallet_democracy::WeightInfo<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type SubmitOrigin = EnsureSigned<AccountId>;
}

parameter_types! {
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * BlockWeights::get().max_block;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

parameter_types! {
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 10;
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ElectionsPhragmenPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type MaxCandidates = MaxCandidates;
	type WeightInfo = weights::pallet_elections_phragmen::WeightInfo<Runtime>;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

type EnsureRootOrHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrHalfCouncil;
	type RemoveOrigin = EnsureRootOrHalfCouncil;
	type SwapOrigin = EnsureRootOrHalfCouncil;
	type ResetOrigin = EnsureRootOrHalfCouncil;
	type PrimeOrigin = EnsureRootOrHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = weights::pallet_membership::WeightInfo<Runtime>;
}

parameter_types! {
	pub const RootSpendOriginMaxAmount: Balance = Balance::MAX;
	pub const CouncilSpendOriginMaxAmount: Balance = Balance::MAX;
}

parameter_types! {
	pub const PayoutPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
	type Currency = Balances;
	type RejectOrigin = EnsureRootOrHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type PalletId = TreasuryPalletId;
	type BurnDestination = ();
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = Bounties;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EitherOf<
		EnsureRootWithSuccess<AccountId, RootSpendOriginMaxAmount>,
		EnsureWithSuccess<
			pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
			AccountId,
			CouncilSpendOriginMaxAmount,
		>,
	>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = frame_support::traits::tokens::pay::PayFromAccount<Balances, TreasuryAccount>;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = TreasuryBenchmarkPaymaster;
	type BalanceConverter = frame_support::traits::tokens::UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = TreasuryBenchmarkHelper;
}

/// Benchmark helper for Treasury pallet.
/// Needed because AssetKind = () doesn't implement FromEntropy.
#[cfg(feature = "runtime-benchmarks")]
pub struct TreasuryBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_treasury::ArgumentsFactory<(), AccountId> for TreasuryBenchmarkHelper {
	fn create_asset_kind(_seed: u32) -> () {
		()
	}
	fn create_beneficiary(seed: [u8; 32]) -> AccountId {
		AccountId::from(seed)
	}
}

/// Custom Paymaster for Treasury benchmarks.
/// PayFromAccount only funds the treasury account, but benchmarks use small amounts (100 units)
/// below the existential deposit. This wrapper also funds the beneficiary so they can receive
/// payments below ED.
#[cfg(feature = "runtime-benchmarks")]
pub struct TreasuryBenchmarkPaymaster;

#[cfg(feature = "runtime-benchmarks")]
impl frame_support::traits::tokens::Pay for TreasuryBenchmarkPaymaster {
	type Balance = Balance;
	type Beneficiary = AccountId;
	type AssetKind = ();
	type Id = ();
	type Error = sp_runtime::DispatchError;

	fn pay(
		who: &Self::Beneficiary,
		_asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		use frame_support::traits::fungible::Mutate;
		<Balances as Mutate<_>>::transfer(
			&TreasuryAccount::get(),
			who,
			amount,
			frame_support::traits::tokens::Preservation::Expendable,
		)?;
		Ok(())
	}

	fn check_payment(_id: Self::Id) -> frame_support::traits::tokens::PaymentStatus {
		frame_support::traits::tokens::PaymentStatus::Success
	}

	fn ensure_successful(
		who: &Self::Beneficiary,
		_asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) {
		use frame_support::traits::fungible::Mutate;
		// Fund the treasury account
		let _ = <Balances as Mutate<_>>::mint_into(&TreasuryAccount::get(), amount);
		// Also fund the beneficiary with existential deposit so they can receive small amounts
		let ed =
			<Balances as frame_support::traits::fungible::Inspect<AccountId>>::minimum_balance();
		let _ = <Balances as Mutate<_>>::mint_into(who, ed);
	}

	fn ensure_concluded(_id: Self::Id) {}
}

impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type RuntimeEvent = RuntimeEvent;
	type MaximumReasonLength = MaximumReasonLength;
	type OnSlash = Treasury;
	type WeightInfo = weights::pallet_bounties::WeightInfo<Runtime>;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = weights::pallet_child_bounties::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxTipAmount: Balance = 10_000 * UNITS;
}

impl pallet_tips::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaximumReasonLength = MaximumReasonLength;
	type DataDepositPerByte = DataDepositPerByte;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type Tippers = Elections;
	type MaxTipAmount = MaxTipAmount;
	type OnSlash = Treasury;
	type WeightInfo = weights::pallet_tips::WeightInfo<Runtime>;
}

impl<LocalCall> frame_system::offchain::CreateTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = SignedExtra;

	fn create_transaction(call: RuntimeCall, extension: SignedExtra) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_transaction(call, extension).into()
	}
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<UncheckedExtrinsic> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let tx_ext: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			claims::PrevalidateAttests::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, tx_ext)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let transaction =
			generic::UncheckedExtrinsic::new_signed(call, address, signature, tx_ext).into();
		Some(transaction)
	}
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_bare(call).into()
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
	pub const MaxNominators: u32 = 256;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const UsernameDeposit: Balance = 0;
	pub const PendingUsernameExpiration: BlockNumber = 7 * DAYS;
	pub const UsernameGracePeriod: BlockNumber = 30 * DAYS;
	pub const MaxSuffixLength: u32 = 7;
	pub const MaxUsernameLength: u32 = 32;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type UsernameDeposit = UsernameDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = pallet_identity::legacy::IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type RegistrarOrigin = EnsureRootOrHalfCouncil;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as sp_runtime::traits::Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<AccountId>;
	type PendingUsernameExpiration = PendingUsernameExpiration;
	type UsernameGracePeriod = UsernameGracePeriod;
	type MaxSuffixLength = MaxSuffixLength;
	type MaxUsernameLength = MaxUsernameLength;
	type WeightInfo = weights::pallet_identity::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay xx coins to the xx network account:";
}

impl claims::Config for Runtime {
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	/// Tech committee unanimity can move a claim
	type MoveClaimOrigin = EnsureTwoThirdsTechnical;
	type RewardHandler = NoOpClaimsRewardHandler;
	type WeightInfo = weights::claims::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ChainId: u8 = 0;
	pub const ProposalLifetime: BlockNumber = HOURS;
	pub const BridgePalletId: PalletId = PalletId(*b"cb/bridg");
	pub TokenID: chainbridge::ResourceId = chainbridge::derive_resource_id(0, &sha2_256(b"xx coin"));
}

// Chain Bridge Pallet
impl chainbridge::Config for Runtime {
	type PalletId = BridgePalletId;
	type AdminOrigin = EnsureTwoThirdsTechnical;
	type Proposal = RuntimeCall;
	type ChainId = ChainId;
	type ProposalLifetime = ProposalLifetime;
}

// Swap pallet
impl swap::Config for Runtime {
	type BridgeOrigin = chainbridge::EnsureBridge<Runtime>;
	type Currency = Balances;
	type NativeTokenId = TokenID;
	type AdminOrigin = EnsureTwoThirdsTechnical;
	type WeightInfo = weights::swap::WeightInfo<Runtime>;
}

// =============================================================================
// Smart Contracts - pallet-revive
// =============================================================================

parameter_types! {
	/// Storage deposit per byte for contracts
	pub const ContractDepositPerByte: Balance = deposit(0, 1);
	/// Storage deposit per storage item
	pub const ContractDepositPerItem: Balance = deposit(1, 0);
	/// Default limit for storage deposits during contract deployment
	pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	/// Percentage of deposit locked for code hash protection
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

// NOTE: pallet-revive uses Precompiles for custom functionality, not a CallFilter.
// Smart contracts are sandboxed and cannot call runtime extrinsics directly.
// They can only interact with other contracts and use precompiles.

impl pallet_revive::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Time = Timestamp;
	type Currency = Balances;
	type WeightPrice = pallet_transaction_payment::Pallet<Runtime>;
	type WeightInfo = pallet_revive::weights::SubstrateWeight<Runtime>;
	type Precompiles = ();
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type DepositPerByte = ContractDepositPerByte;
	type DepositPerItem = ContractDepositPerItem;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type UploadOrigin = EnsureSigned<AccountId>;
	type InstantiateOrigin = EnsureSigned<AccountId>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type ChainId = ConstU64<55>;
	type NativeToEthRatio = ConstU32<1_000_000_000>;
	type AllowEVMBytecode = ConstBool<true>;
	type EthGasEncoder = ();
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type BlockNumberProvider = System;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
	type BlockNumberProvider = System;
	type WeightInfo = weights::pallet_recovery::WeightInfo<Runtime>;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u64;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type Holder = ();
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_uniques::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type WeightInfo = weights::pallet_uniques::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub Features: pallet_nfts::PalletFeatures = pallet_nfts::PalletFeatures::all_enabled();
}

impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type ApprovalsLimit = ApprovalsLimit;
	type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
	type MaxTips = MaxTips;
	type MaxDeadlineDuration = MaxDeadlineDuration;
	type MaxAttributesPerCall = MaxAttributesPerCall;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as traits::Verify>::Signer;
	type BlockNumberProvider = System;
	type WeightInfo = weights::pallet_nfts::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

construct_runtime!(
	pub enum Runtime
	{
		// System pallets
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>} = 0,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 1,

		// Block production, balances and transaction fees
		Babe: pallet_babe::{Pallet, Call, Storage, Config<T>, ValidateUnsigned} = 2,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 24,

		// Consensus support: Staking, Block Finality, Sessions and Eras
		Authorship: pallet_authorship::{Pallet, Storage} = 5,
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>, HoldReason} = 6,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 7,
		VoterList: pallet_bags_list::{Pallet, Call, Storage, Event<T>} = 41,
		Offences: pallet_offences::{Pallet, Storage, Event} = 8,
		Historical: pallet_session_historical::{Pallet, Event<T>} = 25,
		Session: pallet_session::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason} = 9,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config<T>, Event, ValidateUnsigned} = 10,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 11,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config<T>} = 12,

		// Governance
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 13,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 14,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 15,
		Elections: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 16,
		TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 17,
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config<T>, Event<T>} = 18,

		// Token Claims
		Claims: claims::{Pallet, Call, Storage, Event<T>, Config<T>, ValidateUnsigned} = 19,

		// Vesting
		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 20,
		// Utilities (for example batch call)
		Utility: pallet_utility::{Pallet, Call, Event} = 21,
		// Identity registry
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 22,
		// Proxy (allows setting up proxy accounts)
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 23,
		// Bounties
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 26,
		// Tips
		Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 27,
		// Chain Bridge
		ChainBridge: chainbridge::{Pallet, Call, Storage, Event<T>} = 28,
		// Swap pallet
		Swap: swap::{Pallet, Call, Storage, Config<T>, Event<T>} = 29,
		// xx network pallets
		XXStakingExtension: xx_staking_extension::{Pallet, Call, Storage, Event<T>} = 43,
		XXCmix: xx_cmix::{Pallet, Call, Storage, Config<T>, Event<T>} = 30,
		XXEconomics: xx_economics::{Pallet, Call, Storage, Config<T>, Event<T>} = 31,
		XXCustody: xx_team_custody::{Pallet, Call, Storage, Config<T>, Event<T>} = 32,
		// NOTE: Pallet index 33 (XXBetanetRewards) is RETIRED - do not reuse
		XXPublic: xx_public::{Pallet, Call, Storage, Config<T>, Event<T>} = 34,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 35,
		Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 36,
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>} = 37,
		Uniques: pallet_uniques::{Pallet, Call, Storage, Event<T>} = 38,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 39,
		ChildBounties: pallet_child_bounties = 40,
		Nfts: pallet_nfts = 42,

		// Smart Contracts
		Revive: pallet_revive = 44,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The TransactionExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
///
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	claims::PrevalidateAttests<Runtime>,
);

/// Default extensions applied to Ethereum transactions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EthExtraImpl;

impl EthExtra for EthExtraImpl {
	type Config = Runtime;
	type Extension = SignedExtra;

	fn get_eth_extension(nonce: Index, tip: Balance) -> Self::Extension {
		(
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(Era::Immortal),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			claims::PrevalidateAttests::<Runtime>::new(),
		)
	}
}

/// Unchecked extrinsic type as expected by this runtime.
/// Uses pallet_revive's UncheckedExtrinsic for Ethereum transaction support.
pub type UncheckedExtrinsic =
	pallet_revive::evm::runtime::UncheckedExtrinsic<Address, Signature, EthExtraImpl>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

// Migration chain for SDK upgrade from xx-labs fork to polkadot-stable2509-2
//
// The old xx-labs fork was at storage version "v12xx" (v12 with cmix_id field in StakingLedger).
// The SDK pallet-staking is at storage version 16.
//
// Migration order:
// 1. CmixIdMigration: v12xx → v12 (extract cmix_id to xx-staking-extension, rewrite ledgers)
// 2. SDK migrations: v12 → v13 → v14 → v15 → v16
// 3. BridgeAdjust: custom balance adjustment (independent of staking)
pub type Migrations = (
	// Step 1: Custom migration from xx-labs fork format to SDK v12 format
	// MUST run FIRST before any SDK staking migrations
	// - Extracts cmix_id from old StakingLedger format
	// - Stores cmix_id in xx-staking-extension::CmixIds
	// - Rewrites ledgers in standard SDK v12 format (without cmix_id)
	CmixIdMigration<Runtime>,
	// Step 2: SDK staking migrations v12 → v16
	// These use VersionedMigration wrappers that check storage versions
	pallet_staking::migrations::v13::MigrateToV13<Runtime>,
	pallet_staking::migrations::v14::MigrateToV14<Runtime>,
	pallet_staking::migrations::v15::MigrateV14ToV15<Runtime>,
	pallet_staking::migrations::v16::MigrateV15ToV16<Runtime>,
	// Step 3: Custom bridge balance adjustment
	BridgeAdjust<Runtime>,
);

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[pallet_assets, Assets]
		[pallet_bags_list, VoterList]
		[pallet_balances, Balances]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_collective, Council]
		[pallet_democracy, Democracy]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[frame_election_provider_support, EPSBench::<Runtime>]
		[pallet_elections_phragmen, Elections]
		[pallet_identity, Identity]
		[pallet_im_online, ImOnline]
		[pallet_membership, TechnicalMembership]
		[pallet_multisig, Multisig]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_preimage, Preimage]
		[pallet_proxy, Proxy]
		[pallet_recovery, Recovery]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_staking, Staking]
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_tips, Tips]
		[pallet_treasury, Treasury]
		[pallet_uniques, Uniques]
		[pallet_nfts, Nfts]
		[pallet_revive, Revive]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		// xx network
		[claims, Claims]
		[swap, Swap]
		[xx_cmix, XXCmix]
		[xx_public, XXPublic]
		[xx_economics, XXEconomics]
		[xx_team_custody, XXCustody]
	);
}

pallet_revive::impl_runtime_apis_plus_revive! {
	Runtime,
	Executive,
	EthExtraImpl,

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> alloc::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_nfts_runtime_api::NftsApi<Block, AccountId, u32, u32> for Runtime {
		fn owner(collection: u32, item: u32) -> Option<AccountId> {
			<Nfts as Inspect<AccountId>>::owner(&collection, &item)
		}

		fn collection_owner(collection: u32) -> Option<AccountId> {
			<Nfts as Inspect<AccountId>>::collection_owner(&collection)
		}

		fn attribute(
			collection: u32,
			item: u32,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::attribute(&collection, &item, &key)
		}

		fn custom_attribute(
			account: AccountId,
			collection: u32,
			item: u32,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::custom_attribute(
				&account,
				&collection,
				&item,
				&key,
			)
		}

		fn system_attribute(
			collection: u32,
			item: Option<u32>,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::system_attribute(&collection, item.as_ref(), &key)
		}

		fn collection_attribute(collection: u32, key: Vec<u8>) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::collection_attribute(&collection, &key)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);
			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use frame_benchmarking::{baseline, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let treasury_key = frame_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			frame_support::genesis_builder_helper::build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			alloc::vec![]
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_system::offchain::CreateSignedTransaction;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}
}
