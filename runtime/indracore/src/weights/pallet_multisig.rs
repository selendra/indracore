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
//! Autogenerated weights for pallet_multisig
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-02-22, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("indracore-dev"), DB CACHE: 128

// Executed Command:
// target/release/indracore
// benchmark
// --chain=indracore-dev
// --steps=50
// --repeat=20
// --pallet=pallet_multisig
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

/// Weight functions for pallet_multisig.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_multisig::WeightInfo for WeightInfo<T> {
    fn as_multi_threshold_1(z: u32) -> Weight {
        (13_844_000 as Weight)
            // Standard Error: 0
            .saturating_add((1_000 as Weight).saturating_mul(z as Weight))
    }
    fn as_multi_create(s: u32, z: u32) -> Weight {
        (55_452_000 as Weight)
            // Standard Error: 0
            .saturating_add((121_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 0
            .saturating_add((1_000 as Weight).saturating_mul(z as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn as_multi_create_store(s: u32, z: u32) -> Weight {
        (61_233_000 as Weight)
            // Standard Error: 0
            .saturating_add((132_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 0
            .saturating_add((3_000 as Weight).saturating_mul(z as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn as_multi_approve(s: u32, z: u32) -> Weight {
        (32_213_000 as Weight)
            // Standard Error: 0
            .saturating_add((144_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 0
            .saturating_add((1_000 as Weight).saturating_mul(z as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn as_multi_approve_store(s: u32, z: u32) -> Weight {
        (58_941_000 as Weight)
            // Standard Error: 0
            .saturating_add((158_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 0
            .saturating_add((3_000 as Weight).saturating_mul(z as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn as_multi_complete(s: u32, z: u32) -> Weight {
        (76_173_000 as Weight)
            // Standard Error: 0
            .saturating_add((280_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 0
            .saturating_add((5_000 as Weight).saturating_mul(z as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn approve_as_multi_create(s: u32) -> Weight {
        (54_804_000 as Weight)
            // Standard Error: 0
            .saturating_add((128_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn approve_as_multi_approve(s: u32) -> Weight {
        (31_803_000 as Weight)
            // Standard Error: 0
            .saturating_add((145_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn approve_as_multi_complete(s: u32) -> Weight {
        (150_305_000 as Weight)
            // Standard Error: 0
            .saturating_add((286_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn cancel_as_multi(s: u32) -> Weight {
        (102_884_000 as Weight)
            // Standard Error: 0
            .saturating_add((132_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
}
