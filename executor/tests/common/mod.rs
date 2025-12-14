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

// NOTE: This module is only compiled when executor-tests feature is enabled.
#![cfg(feature = "executor-tests")]

use codec::{Decode, Encode};
use frame_support::Hashable;
use frame_system::offchain::AppCrypto;
use sc_executor::{error::Result, WasmExecutor};
use sp_consensus_babe::{
	digests::{PreDigest, SecondaryPlainPreDigest},
	Slot, BABE_ENGINE_ID,
};
use sp_core::{
	crypto::KeyTypeId,
	sr25519::Signature,
	traits::{CallContext, CodeExecutor, RuntimeCode},
};
use sp_runtime::{
	traits::{BlakeTwo256, Header as HeaderT},
	ApplyExtrinsicResult, Digest, DigestItem, MultiSignature, MultiSigner,
};
use sp_state_machine::TestExternalities as CoreTestExternalities;

use node_primitives::{AccountId, Balance, BlockNumber, Hash};
use runtime_common::constants::currency::*;
use xxnetwork_runtime::{
	Block, BuildStorage, CheckedExtrinsic, Header, Runtime, SignedExtra, UncheckedExtrinsic,
};
// Note: Nonce is not exported from node_primitives, use u32 directly
use sp_externalities::Externalities;
use sp_io;
use sp_keyring::Sr25519Keyring;
use sp_runtime::generic::{self, ExtrinsicFormat};

pub const TEST_KEY_TYPE_ID: KeyTypeId = KeyTypeId(*b"test");

pub mod sr25519 {
	mod app_sr25519 {
		use super::super::TEST_KEY_TYPE_ID;
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, TEST_KEY_TYPE_ID);
	}

	pub type AuthorityId = app_sr25519::Public;
}

pub struct TestAuthorityId;
impl AppCrypto<MultiSigner, MultiSignature> for TestAuthorityId {
	type RuntimeAppPublic = sr25519::AuthorityId;
	type GenericSignature = Signature;
	type GenericPublic = sp_core::sr25519::Public;
}

// Keyring helper functions (previously from node-testing)

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

/// Returns transaction extra.
pub fn signed_extra(nonce: u32, extra_fee: Balance) -> SignedExtra {
	(
		frame_system::CheckNonZeroSender::new(),
		frame_system::CheckSpecVersion::new(),
		frame_system::CheckTxVersion::new(),
		frame_system::CheckGenesis::new(),
		frame_system::CheckEra::from(sp_runtime::generic::Era::mortal(256, 0)),
		frame_system::CheckNonce::from(nonce),
		frame_system::CheckWeight::new(),
		pallet_transaction_payment::ChargeTransactionPayment::from(extra_fee),
		claims::PrevalidateAttests::new(),
	)
}

/// The wasm runtime code.
///
/// `compact` since it is after post-processing with wasm-gc which performs tree-shaking thus
/// making the binary slimmer. There is a convention to use compact version of the runtime
/// as canonical. This is why `native_executor_instance` also uses the compact version of the
/// runtime.
pub fn compact_code_unwrap() -> &'static [u8] {
	xxnetwork_runtime::WASM_BINARY.expect(
		"Development wasm binary is not available. \
									  Testing is only supported with the flag disabled.",
	)
}

pub const GENESIS_HASH: [u8; 32] = [69u8; 32];

pub const SPEC_VERSION: u32 = xxnetwork_runtime::VERSION.spec_version;

pub const TRANSACTION_VERSION: u32 = xxnetwork_runtime::VERSION.transaction_version;

pub type TestExternalities<H> = CoreTestExternalities<H>;

/// Sign given `CheckedExtrinsic`.
///
/// In the new SDK, CheckedExtrinsic uses `format: ExtrinsicFormat` instead of `signed: Option<...>`.
pub fn sign(xt: CheckedExtrinsic) -> UncheckedExtrinsic {
	match xt.format {
		ExtrinsicFormat::Signed(signed, extra) => {
			let payload = (
				xt.function.clone(),
				extra.clone(),
				SPEC_VERSION,
				TRANSACTION_VERSION,
				GENESIS_HASH,
				GENESIS_HASH,
			);
			let key = Sr25519Keyring::from_account_id(&signed).unwrap();
			let signature = payload
				.using_encoded(|b| {
					if b.len() > 256 {
						key.sign(&sp_io::hashing::blake2_256(b))
					} else {
						key.sign(b)
					}
				})
				.into();
			// Use generic::UncheckedExtrinsic and convert to the pallet-revive wrapper
			generic::UncheckedExtrinsic::new_signed(
				xt.function,
				sp_runtime::MultiAddress::Id(signed),
				signature,
				extra,
			)
			.into()
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

pub fn default_transfer_call() -> pallet_balances::Call<Runtime> {
	pallet_balances::Call::transfer_allow_death::<Runtime> { dest: bob().into(), value: 69 * UNITS }
}

pub fn from_block_number(n: u32) -> Header {
	Header::new(n, Default::default(), Default::default(), [69; 32].into(), Default::default())
}

/// Type alias for the WasmExecutor used in tests.
pub type TestExecutor = WasmExecutor<sp_io::SubstrateHostFunctions>;

pub fn executor() -> TestExecutor {
	WasmExecutor::builder().build()
}

/// Execute a runtime call using the WasmExecutor.
/// Note: The `use_native` parameter is ignored since WasmExecutor only runs wasm.
pub fn executor_call(
	t: &mut TestExternalities<BlakeTwo256>,
	method: &str,
	data: &[u8],
	_use_native: bool,
) -> Result<Vec<u8>> {
	let mut t = t.ext();

	let code = t.storage(sp_core::storage::well_known_keys::CODE).unwrap();
	let heap_pages = t.storage(sp_core::storage::well_known_keys::HEAP_PAGES);
	let runtime_code = RuntimeCode {
		code_fetcher: &sp_core::traits::WrappedRuntimeCode(code.as_slice().into()),
		hash: sp_core::blake2_256(&code).to_vec(),
		heap_pages: heap_pages.and_then(|hp| Decode::decode(&mut &hp[..]).ok()),
	};
	sp_tracing::try_init_simple();
	executor().call(&mut t, &runtime_code, method, data, CallContext::Onchain).0
}

pub fn new_test_ext(code: &[u8]) -> TestExternalities<BlakeTwo256> {
	TestExternalities::new_with_code(
		code,
		node_testing::genesis::config(Some(code)).build_storage().unwrap(),
	)
}

/// Construct a fake block.
///
/// `extrinsics` must be a list of valid extrinsics, i.e. none of the extrinsics for example
/// can report `ExhaustResources`. Otherwise, this function panics.
pub fn construct_block(
	env: &mut TestExternalities<BlakeTwo256>,
	number: BlockNumber,
	parent_hash: Hash,
	extrinsics: Vec<CheckedExtrinsic>,
	babe_slot: Slot,
) -> (Vec<u8>, Hash) {
	use sp_trie::{LayoutV1 as Layout, TrieConfiguration};

	// sign extrinsics.
	let extrinsics = extrinsics.into_iter().map(sign).collect::<Vec<_>>();

	// calculate the header fields that we can.
	let extrinsics_root =
		Layout::<BlakeTwo256>::ordered_trie_root(extrinsics.iter().map(Encode::encode))
			.to_fixed_bytes()
			.into();

	let header = Header {
		parent_hash,
		number,
		extrinsics_root,
		state_root: Default::default(),
		digest: Digest {
			logs: vec![DigestItem::PreRuntime(
				BABE_ENGINE_ID,
				PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
					slot: babe_slot,
					authority_index: 42,
				})
				.encode(),
			)],
		},
	};

	// execute the block to get the real header.
	executor_call(env, "Core_initialize_block", &header.encode(), true).unwrap();

	for extrinsic in extrinsics.iter() {
		// Try to apply the `extrinsic`. It should be valid, in the sense that it passes
		// all pre-inclusion checks.
		let r = executor_call(env, "BlockBuilder_apply_extrinsic", &extrinsic.encode(), true)
			.expect("application of an extrinsic failed");

		match ApplyExtrinsicResult::decode(&mut &r[..])
			.expect("apply result deserialization failed")
		{
			Ok(_) => {}
			Err(e) => panic!("Applying extrinsic failed: {:?}", e),
		}
	}

	let header = Header::decode(
		&mut &executor_call(env, "BlockBuilder_finalize_block", &[0u8; 0], true).unwrap()[..],
	)
	.unwrap();

	let hash = header.blake2_256();
	(Block { header, extrinsics }.encode(), hash.into())
}
