// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card CG18-FOOTPRINT-CONNECTION (W08): `source.mutex.guard.v1`'s checked
//! mutex trace composed with derived footprints. Each refusal is one mutation
//! of an accepted control.

use qcore_core::concurrency::ConcurrencyAssumption;
use qcore_core::disjoint_footprint::{FootprintAssumption, FootprintEvent};
use qcore_core::footprint_mutex::{
    ComposedAssumption, CompositionDiagnostic, compose_footprints_with_mutex_invariant,
};
use qcore_core::mutex_invariant::check_mutex_invariant_program;
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_mutex::{LoweredMutexModule, lower_source_mutex};
use qcore_frontend::source_mutex_footprint::{compose, footprint_program};

fn lower(body: &str) -> LoweredMutexModule {
    let source =
        format!("#import std.concurrency; fn main() -> unit {{ thread scope s {{ {body} }} }}");
    let parsed = parse_source_program(&source).expect("parse");
    let typed = elaborate_source(&parsed).expect("elaborate");
    lower_source_mutex(&parsed, &typed, "main").expect("lower")
}

const ONE: &str = "let buf = protected 4; let m = mutex guards buf; \
     with lock m as g { open g; write buf with g; close g; }";

const TWO: &str = "let a = protected 4; let b = protected 8; \
     let m = mutex guards a; let n = mutex guards b; \
     with lock m as g { open g; write a with g; close g; } \
     with lock n as h { read b with h; }";

#[test]
fn one_protected_resource_discharges_the_declared_shape() {
    let module = lower(ONE);
    let alone = check_mutex_invariant_program(&module.program).expect("route control");
    assert!(
        alone
            .assumptions
            .contains(&ConcurrencyAssumption::DeclaredFootprintShape)
    );
    let composed = compose(&module).expect("composes");
    assert!(!composed.assumptions.contains(&ComposedAssumption::Mutex(
        ConcurrencyAssumption::DeclaredFootprintShape
    )));
    assert!(
        composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::AllocationExtentDeclared
            ))
    );
    // One resource: no cross-allocation pair, so the premise is not taken.
    assert!(composed.cross_allocation_pairs.is_empty());
    assert!(
        !composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::DistinctAllocationsDisjoint
            ))
    );
    // The route's own certificate is unchanged by the composition.
    assert_eq!(composed.mutex, alone);
}

#[test]
fn two_protected_resources_compose_only_under_the_named_premise() {
    let module = lower(TWO);
    let composed = compose(&module).expect("composes");
    assert_eq!(composed.cross_allocation_pairs.len(), 1);
    assert!(
        composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::DistinctAllocationsDisjoint
            ))
    );
    // Without the freshness declarations the two resources are refused, even
    // though the mutex profile alone calls them non-overlapping.
    let mut footprints = footprint_program(&module);
    footprints
        .events
        .retain(|event| !matches!(event, FootprintEvent::DeclareFreshAllocation { .. }));
    let refused = compose_footprints_with_mutex_invariant(&footprints, &module.program)
        .expect_err("no premise");
    assert!(matches!(
        refused.as_slice(),
        [CompositionDiagnostic::CrossAllocationWithoutPremise { .. }]
    ));
    assert_eq!(refused[0].code(), "Q662");
}

#[test]
fn a_mismatched_extent_is_not_a_derivation() {
    let module = lower(ONE);
    let mut footprints = footprint_program(&module);
    // Declare the resource one byte short: its slice no longer exports the
    // interval the trace names.
    for event in &mut footprints.events {
        match event {
            FootprintEvent::Allocate { count, .. }
            | FootprintEvent::DeriveSlice {
                elem_count: count, ..
            } => {
                *count -= 1;
            }
            _ => {}
        }
    }
    let refused = compose_footprints_with_mutex_invariant(&footprints, &module.program)
        .expect_err("underived");
    assert!(matches!(
        refused.as_slice(),
        [CompositionDiagnostic::UnderivedFootprint { .. }]
    ));
    assert_eq!(refused[0].code(), "Q665");
}
