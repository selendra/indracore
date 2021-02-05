// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.
//! Autogenerated weights for pallet_elections_phragmen
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-12-08, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("indracore-dev"), DB CACHE: 128

// Executed Command:
// target/release/indracore
// benchmark
// --chain=indracore-dev
// --steps=50
// --repeat=20
// --pallet=pallet_elections_phragmen
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --header=./file_header.txt
// --output=./runtime/indracore/src/weights/

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_elections_phragmen.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_elections_phragmen::WeightInfo for WeightInfo<T> {
    fn vote(v: u32) -> Weight {
        (88_644_000 as Weight)
            // Standard Error: 7_000
            .saturating_add((130_000 as Weight).saturating_mul(v as Weight))
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn vote_update(v: u32) -> Weight {
        (54_456_000 as Weight)
            // Standard Error: 3_000
            .saturating_add((133_000 as Weight).saturating_mul(v as Weight))
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn remove_voter() -> Weight {
        (71_138_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn report_defunct_voter_correct(c: u32, v: u32) -> Weight {
        (0 as Weight)
            // Standard Error: 0
            .saturating_add((1_749_000 as Weight).saturating_mul(c as Weight))
            // Standard Error: 12_000
            .saturating_add((34_327_000 as Weight).saturating_mul(v as Weight))
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn report_defunct_voter_incorrect(c: u32, v: u32) -> Weight {
        (0 as Weight)
            // Standard Error: 0
            .saturating_add((1_755_000 as Weight).saturating_mul(c as Weight))
            // Standard Error: 9_000
            .saturating_add((34_280_000 as Weight).saturating_mul(v as Weight))
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn submit_candidacy(c: u32) -> Weight {
        (70_892_000 as Weight)
            // Standard Error: 0
            .saturating_add((292_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn renounce_candidacy_candidate(c: u32) -> Weight {
        (43_358_000 as Weight)
            // Standard Error: 0
            .saturating_add((143_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn renounce_candidacy_members() -> Weight {
        (75_956_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn renounce_candidacy_runners_up() -> Weight {
        (46_888_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn remove_member_with_replacement() -> Weight {
        (116_053_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn remove_member_wrong_refund() -> Weight {
        (9_093_000 as Weight).saturating_add(T::DbWeight::get().reads(1 as Weight))
    }
}
