// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Weights for frame_system extensions.
//!
//! Based on benchmarks from polkadot-sdk stable2509-2.
//! These weights are conservative estimates derived from the SDK reference implementations.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for frame_system extensions.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> frame_system::ExtensionsWeightInfo for WeightInfo<T> {
	/// Weight for `CheckGenesis` extension.
	fn check_genesis() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 3_500_000 picoseconds.
		Weight::from_parts(3_500_000, 0)
	}

	/// Weight for `CheckMortality` extension with mortal transaction.
	fn check_mortality_mortal_transaction() -> Weight {
		// Proof Size summary:
		//   Measured:  `92`
		//   Estimated: `3557`
		// Minimum execution time: 6_500_000 picoseconds.
		Weight::from_parts(6_500_000, 3557)
	}

	/// Weight for `CheckMortality` extension with immortal transaction.
	fn check_mortality_immortal_transaction() -> Weight {
		// Proof Size summary:
		//   Measured:  `92`
		//   Estimated: `3557`
		// Minimum execution time: 6_400_000 picoseconds.
		Weight::from_parts(6_400_000, 3557)
	}

	/// Weight for `CheckNonZeroSender` extension.
	fn check_non_zero_sender() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 572_000 picoseconds.
		Weight::from_parts(572_000, 0)
	}

	/// Weight for `CheckNonce` extension.
	fn check_nonce() -> Weight {
		// Proof Size summary:
		//   Measured:  `101`
		//   Estimated: `3593`
		// Minimum execution time: 7_200_000 picoseconds.
		Weight::from_parts(7_200_000, 3593)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}

	/// Weight for `CheckSpecVersion` extension.
	fn check_spec_version() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 483_000 picoseconds.
		Weight::from_parts(483_000, 0)
	}

	/// Weight for `CheckTxVersion` extension.
	fn check_tx_version() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 443_000 picoseconds.
		Weight::from_parts(443_000, 0)
	}

	/// Weight for `CheckWeight` extension.
	fn check_weight() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 4_100_000 picoseconds.
		Weight::from_parts(4_100_000, 0)
	}

	/// Weight for weight reclaim.
	fn weight_reclaim() -> Weight {
		// Proof Size summary:
		//   Measured:  `0`
		//   Estimated: `0`
		// Minimum execution time: 2_400_000 picoseconds.
		Weight::from_parts(2_400_000, 0)
	}
}
