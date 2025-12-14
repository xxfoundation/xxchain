// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
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

//! Genesis Configuration.

use crate::keyring::*;
use sp_keyring::{Ed25519Keyring, Sr25519Keyring};
use xxnetwork_runtime::{
	RuntimeGenesisConfig, BalancesConfig, SessionConfig, StakingConfig,
	GrandpaConfig, SwapConfig,
	AccountId, StakerStatus, BabeConfig, BABE_GENESIS_EPOCH_CONFIG,
};
use runtime_common::constants::currency::UNITS;
use sp_runtime::Perbill;

/// Create genesis runtime configuration for tests.
///
/// Note: The `code` parameter is deprecated and ignored. WASM code is now
/// set separately via `TestExternalities::new_with_code`.
pub fn config(_code: Option<&[u8]>) -> RuntimeGenesisConfig {
	config_endowed(Default::default())
}

/// Create genesis runtime configuration for tests with some extra
/// endowed accounts.
pub fn config_endowed(
	extra_endowed: Vec<AccountId>,
) -> RuntimeGenesisConfig {
	let mut endowed = vec![
		(alice(), 111 * UNITS),
		(bob(), 100 * UNITS),
		(charlie(), 100_000_000 * UNITS),
		(dave(), 111 * UNITS),
		(eve(), 101 * UNITS),
		(ferdie(), 100 * UNITS),
	];

	endowed.extend(
		extra_endowed.into_iter().map(|endowed| (endowed, 100*UNITS))
	);

	RuntimeGenesisConfig {
		// SystemConfig no longer has a code field - code is set via TestExternalities::new_with_code
		system: Default::default(),
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
		},
		balances: BalancesConfig {
			balances: endowed,
			..Default::default()
		},
		staking: StakingConfig {
			stakers: vec![
				// Note: StakerStatus::Validator no longer takes cMix ID argument in standard SDK
				// cMix IDs would need to be set via xx_staking_extension if needed
				(dave(), alice(), 111 * UNITS, StakerStatus::Validator),
				(eve(), bob(), 100 * UNITS, StakerStatus::Validator),
				(ferdie(), charlie(), 100 * UNITS, StakerStatus::Validator)
			],
			validator_count: 3,
			minimum_validator_count: 0,
			slash_reward_fraction: Perbill::from_percent(10),
			invulnerables: vec![alice(), bob(), charlie()],
			.. Default::default()
		},
		session: SessionConfig {
			keys: vec![
				(alice(), dave(), to_session_keys(
					&Ed25519Keyring::Alice,
					&Sr25519Keyring::Alice,
				)),
				(bob(), eve(), to_session_keys(
					&Ed25519Keyring::Bob,
					&Sr25519Keyring::Bob,
				)),
				(charlie(), ferdie(), to_session_keys(
					&Ed25519Keyring::Charlie,
					&Sr25519Keyring::Charlie,
				)),
			],
			..Default::default()
		},
		grandpa: GrandpaConfig {
			authorities: vec![],
			..Default::default()
		},
		im_online: Default::default(),
		authority_discovery: Default::default(),
		democracy: Default::default(),
		council: Default::default(),
		technical_committee: Default::default(),
		elections: Default::default(),
		technical_membership: Default::default(),
		treasury: Default::default(),
		claims: Default::default(),
		vesting: Default::default(),
		swap: SwapConfig {
			threshold: 1,
			..Default::default()
		},
		xx_cmix: Default::default(),
		xx_economics: Default::default(),
		xx_custody: Default::default(),
		// xx_betanet_rewards pallet was retired (index 33)
		xx_public: Default::default(),
		assets: Default::default(),
		revive: Default::default(),
	}
}
