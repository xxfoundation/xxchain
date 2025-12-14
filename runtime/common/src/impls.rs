// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
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

//! Some configurable implementations as associated type for the substrate runtime.

use frame_support::traits::{
	fungible::{Balanced, Credit},
	tokens::imbalance::Imbalance,
	OnUnbalanced,
};

/// Type alias for fungible credit used in fee handling
pub type FungibleCredit<R> =
	Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>;

/// Split fees between treasury and block author using fungible traits
///
/// - Fees: 80% to treasury, 20% to author
/// - Tips: 100% to author
pub struct DealWithFees<R>(core::marker::PhantomData<R>);

impl<R> OnUnbalanced<FungibleCredit<R>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_authorship::Config + pallet_treasury::Config,
	<R as frame_system::Config>::AccountId: From<node_primitives::AccountId>,
	<R as frame_system::Config>::AccountId: Into<node_primitives::AccountId>,
{
	fn on_unbalanceds(mut fees_then_tips: impl Iterator<Item = FungibleCredit<R>>) {
		if let Some(fees) = fees_then_tips.next() {
			// Split fees: 80% to treasury, 20% to author
			let (treasury_part, mut author_part) = fees.ration(80, 20);

			// Tips go 100% to author
			if let Some(tips) = fees_then_tips.next() {
				author_part.subsume(tips);
			}

			// Resolve treasury portion to treasury account
			let treasury_account = pallet_treasury::Pallet::<R>::account_id();
			let _ = <pallet_balances::Pallet<R> as Balanced<_>>::resolve(
				&treasury_account,
				treasury_part,
			);

			// Resolve author portion to block author
			if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
				let _ = <pallet_balances::Pallet<R> as Balanced<_>>::resolve(&author, author_part);
			}
			// If no author, author_part is dropped (reducing total issuance)
		}
	}
}
