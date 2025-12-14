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

//! Test accounts.

use sp_keyring::{Sr25519Keyring, Ed25519Keyring};
use node_primitives::{AccountId, Balance, Index};
use xxnetwork_runtime::{CheckedExtrinsic, UncheckedExtrinsic, SessionKeys, SignedExtra};
use sp_runtime::generic::{self, ExtrinsicFormat};
use sp_runtime::generic::Era;
use codec::Encode;

/// Alice's account id.
pub fn alice() -> AccountId {
	Sr25519Keyring::Alice.to_account_id()
}

/// Bob's account id.
pub fn bob() -> AccountId {
	Sr25519Keyring::Bob.to_account_id()
}

/// Charlie's account id.
pub fn charlie() -> AccountId {
	Sr25519Keyring::Charlie.to_account_id()
}

/// Dave's account id.
pub fn dave() -> AccountId {
	Sr25519Keyring::Dave.to_account_id()
}

/// Eve's account id.
pub fn eve() -> AccountId {
	Sr25519Keyring::Eve.to_account_id()
}

/// Ferdie's account id.
pub fn ferdie() -> AccountId {
	Sr25519Keyring::Ferdie.to_account_id()
}

/// Convert keyrings into `SessionKeys`.
pub fn to_session_keys(
	ed25519_keyring: &Ed25519Keyring,
	sr25519_keyring: &Sr25519Keyring,
) -> SessionKeys {
	SessionKeys {
		grandpa: ed25519_keyring.to_owned().public().into(),
		babe: sr25519_keyring.to_owned().public().into(),
		im_online: sr25519_keyring.to_owned().public().into(),
		authority_discovery: sr25519_keyring.to_owned().public().into(),
	}
}

/// Returns transaction extra.
pub fn signed_extra(nonce: Index, extra_fee: Balance) -> SignedExtra {
	(
		frame_system::CheckNonZeroSender::new(),
		frame_system::CheckSpecVersion::new(),
		frame_system::CheckTxVersion::new(),
		frame_system::CheckGenesis::new(),
		frame_system::CheckEra::from(Era::mortal(256, 0)),
		frame_system::CheckNonce::from(nonce),
		frame_system::CheckWeight::new(),
		pallet_transaction_payment::ChargeTransactionPayment::from(extra_fee),
		claims::PrevalidateAttests::new(),
	)
}

/// Sign given `CheckedExtrinsic`.
///
/// In the new SDK, CheckedExtrinsic uses `format: ExtrinsicFormat` instead of `signed: Option<...>`.
pub fn sign(xt: CheckedExtrinsic, spec_version: u32, tx_version: u32, genesis_hash: [u8; 32]) -> UncheckedExtrinsic {
	match xt.format {
		ExtrinsicFormat::Signed(signed, extra) => {
			let payload = (xt.function.clone(), extra.clone(), spec_version, tx_version, genesis_hash, genesis_hash);
			let key = Sr25519Keyring::from_account_id(&signed).unwrap();
			let signature = payload.using_encoded(|b| {
				if b.len() > 256 {
					key.sign(&sp_io::hashing::blake2_256(b))
				} else {
					key.sign(b)
				}
			}).into();
			// Use generic::UncheckedExtrinsic and convert to the pallet-revive wrapper
			generic::UncheckedExtrinsic::new_signed(
				xt.function,
				sp_runtime::MultiAddress::Id(signed),
				signature,
				extra,
			).into()
		}
		ExtrinsicFormat::Bare => {
			// Bare/unsigned extrinsic
			generic::UncheckedExtrinsic::new_bare(xt.function).into()
		}
		ExtrinsicFormat::General(_extension_version, extra) => {
			// General transaction (unsigned with extension)
			generic::UncheckedExtrinsic::new_transaction(xt.function, extra).into()
		}
	}
}
