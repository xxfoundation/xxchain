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

//! Substrate chain configurations.

use sc_chain_spec::{ChainSpecExtension, ChainType};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};

use frame_support::PalletId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use runtime_common::constants::currency::UNITS;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{AccountIdConversion, IdentifyAccount, Verify},
	Perbill,
};
pub use xxnetwork_runtime as xxnetwork;

pub use node_primitives::{AccountId, Balance, Block, Hash, Signature};

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// The `ChainSpec` parameterized for the `xxnetwork` runtime.
pub type XXNetworkChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Genesis config for `xxnetwork` mainnet - loads from JSON
pub fn xxnetwork_config() -> Result<XXNetworkChainSpec, String> {
	XXNetworkChainSpec::from_json_bytes(&include_bytes!("../res/xxnetwork.json")[..])
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn xxnetwork_session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> xxnetwork::SessionKeys {
	xxnetwork::SessionKeys { grandpa, babe, im_online, authority_discovery }
}

/// `xxnetwork` development config (single validator Alice)
/// Uses JSON genesis config builder pattern
pub fn xxnetwork_development_config() -> XXNetworkChainSpec {
	let wasm_binary = xxnetwork::wasm_binary_unwrap();

	XXNetworkChainSpec::builder(wasm_binary, Extensions::default())
		.with_name("xxnetwork Development")
		.with_id("xxnetwork-dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(development_genesis_config_patch())
		.build()
}

/// Generate the genesis config patch for development network
fn development_genesis_config_patch() -> serde_json::Value {
	use serde_json::json;

	let initial_authorities = vec![authority_keys_from_seed("Alice")];

	let endowed_accounts: Vec<AccountId> = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
	];

	// Pallet accounts derived from PalletId (needed for benchmarks)
	let treasury_account: AccountId = PalletId(*b"xx/trsry").into_account_truncating();
	let bridge_account: AccountId = PalletId(*b"cb/bridg").into_account_truncating();

	const ENDOWMENT: Balance = 10_000_000 * UNITS;
	const STASH: Balance = ENDOWMENT / 1000;
	// Large balance for distribution benchmarks (100 distributions * 25 units each * safety margin)
	const DISTRIBUTION_BALANCE: Balance = 1_000_000 * UNITS;

	// Build balances including pallet accounts for benchmarks
	let mut balances: Vec<(AccountId, Balance)> =
		endowed_accounts.iter().map(|x| (x.clone(), ENDOWMENT)).collect();
	// Fund treasury account for pallet_treasury benchmarks
	balances.push((treasury_account.clone(), ENDOWMENT));
	// Fund bridge account for swap benchmarks
	balances.push((bridge_account.clone(), ENDOWMENT));
	// Note: xx_public pallet accounts are funded via xxPublic genesis config (testnetBalance, saleBalance)

	json!({
		"balances": {
			"balances": balances
		},
		"session": {
			"keys": initial_authorities.iter().map(|x| {
				(
					x.0.clone(),
					x.0.clone(),
					xxnetwork_session_keys(
						x.2.clone(),
						x.3.clone(),
						x.4.clone(),
						x.5.clone(),
					)
				)
			}).collect::<Vec<_>>()
		},
		"staking": {
			"validatorCount": initial_authorities.len() as u32 * 2,
			"minimumValidatorCount": initial_authorities.len() as u32,
			"invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			"slashRewardFraction": Perbill::from_percent(10),
			"stakers": initial_authorities.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, "Validator"))
				.collect::<Vec<_>>()
		},
		"elections": {
			"members": endowed_accounts.iter()
				.take(1)
				.map(|member| (member.clone(), STASH))
				.collect::<Vec<_>>()
		},
		"technicalCommittee": {
			"members": endowed_accounts.iter()
				.take(3)
				.cloned()
				.collect::<Vec<_>>()
		},
		"babe": {
			"epochConfig": xxnetwork::BABE_GENESIS_EPOCH_CONFIG
		},
		"swap": {
			"feeDestination": get_account_id_from_seed::<sr25519::Public>("Alice"),
			"swapFee": 0u32
		},
		"xxCmix": {
			"adminPermission": u32::MAX,
			"schedulingAccount": get_account_id_from_seed::<sr25519::Public>("Alice")
		},
		"xxEconomics": {
			"liquidityRewards": 0u128,
			"balance": 0u128
		},
		"xxCustody": {
			"custodians": vec![
				(get_account_id_from_seed::<sr25519::Public>("Alice"), ()),
				(get_account_id_from_seed::<sr25519::Public>("Bob"), ()),
				(get_account_id_from_seed::<sr25519::Public>("Charlie"), ()),
			],
			"teamAllocations": vec![
				(get_account_id_from_seed::<sr25519::Public>("Dave"), ENDOWMENT),
				(get_account_id_from_seed::<sr25519::Public>("Eve"), ENDOWMENT),
				(get_account_id_from_seed::<sr25519::Public>("Ferdie"), ENDOWMENT),
			]
		},
		"xxPublic": {
			"testnetManager": get_account_id_from_seed::<sr25519::Public>("Alice"),
			"testnetBalance": DISTRIBUTION_BALANCE,
			"saleManager": get_account_id_from_seed::<sr25519::Public>("Alice"),
			"saleBalance": DISTRIBUTION_BALANCE
		}
	})
}
