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

//! Weights for pallet_transaction_payment.
//!
//! Based on benchmarks from polkadot-sdk stable2509-2.
//! These weights are conservative estimates derived from the SDK reference implementations.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for pallet_transaction_payment.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_transaction_payment::WeightInfo for WeightInfo<T> {
	/// Weight for charging transaction payment.
	/// Storage: 2 reads and 2 writes to `System::Account`.
	fn charge_transaction_payment() -> Weight {
		// Proof Size summary:
		//   Measured:  `101`
		//   Estimated: `6196`
		// Minimum execution time: 44_317_000 picoseconds.
		Weight::from_parts(45_035_000, 6196)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}
