// Copyright 2017-2020 Parity Technologies (UK) Ltd.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn claim() -> Weight;
    fn mint_claim() -> Weight;
    fn claim_attest() -> Weight;
    fn attest() -> Weight;
    fn move_claim() -> Weight;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn claim() -> Weight { Weight::zero() }
    fn mint_claim() -> Weight { Weight::zero() }
    fn claim_attest() -> Weight { Weight::zero() }
    fn attest() -> Weight { Weight::zero() }
    fn move_claim() -> Weight { Weight::zero() }
}

/// Weight functions for claims.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn claim() -> Weight {
       Weight::from_parts(466_905_000, 1000)
            .saturating_add(T::DbWeight::get().reads(7 as u64))
            .saturating_add(T::DbWeight::get().writes(7 as u64))
    }
    fn mint_claim() -> Weight {
       Weight::from_parts(19_003_000, 1000)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(4 as u64))
    }
    fn claim_attest() -> Weight {
       Weight::from_parts(471_915_000, 1000)
            .saturating_add(T::DbWeight::get().reads(7 as u64))
            .saturating_add(T::DbWeight::get().writes(7 as u64))
    }
    fn attest() -> Weight {
       Weight::from_parts(156_649_000, 1000)
            .saturating_add(T::DbWeight::get().reads(8 as u64))
            .saturating_add(T::DbWeight::get().writes(8 as u64))
    }
    fn move_claim() -> Weight {
       Weight::from_parts(39_612_000, 1000)
            .saturating_add(T::DbWeight::get().reads(4 as u64))
            .saturating_add(T::DbWeight::get().writes(7 as u64))
    }
}
