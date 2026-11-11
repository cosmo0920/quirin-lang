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

//! Card `CG18-FOOTPRINT-CONNECTION` (gap entry W08): the mutex-invariant
//! evidence of `source.mutex.guard.v1` composed with derived footprints.
//!
//! [`crate::source_mutex`] lowers every `let NAME = protected N;` (or
//! `protected TYPE`) to one shared resource of a declared byte size, and every
//! interval its trace names is that whole resource. [`footprint_program`]
//! derives the same intervals instead: one allocation per resource, its id the
//! resource id, `N` one-byte elements, and one slice over all of them. Each
//! declaration is a distinct object of the source language, which is what an
//! allocator contract says of a fresh allocation, so each is declared fresh
//! and two resources compose under the named premise
//! `P-DISTINCT-ALLOCATIONS-DISJOINT` rather than on the mutex profile's word.
//!
//! [`compose`] hands both programs to
//! [`qcore_core::footprint_mutex::compose_footprints_with_mutex_invariant`]
//! unedited. It changes nothing the route publishes: the route's own
//! certificate keeps `DeclaredFootprintShape`, and the composed certificate
//! is a separate, stronger statement a consumer may ask for.

use qcore_core::disjoint_footprint::{ElementShape, FootprintEvent, FootprintProgram};
use qcore_core::footprint_mutex::{
    ComposedFootprintCertificate, CompositionDiagnostic, compose_footprints_with_mutex_invariant,
};

use crate::source_mutex::LoweredMutexModule;

/// The derived footprints of a lowered module: per resource, in declaration
/// order, an allocation of `bytes` one-byte elements declared fresh and one
/// slice over the whole of it. Handle ids are the resource's index.
pub fn footprint_program(module: &LoweredMutexModule) -> FootprintProgram {
    let mut events = Vec::new();
    for (index, resource) in module.resources.iter().enumerate() {
        events.push(FootprintEvent::Allocate {
            alloc: resource.resource,
            generation: 1,
            shape: ElementShape::Scalar { size: 1 },
            count: resource.bytes,
        });
        events.push(FootprintEvent::DeclareFreshAllocation {
            alloc: resource.resource,
        });
        events.push(FootprintEvent::DeriveSlice {
            handle: u32::try_from(index).unwrap_or(u32::MAX),
            alloc: resource.resource,
            generation: 1,
            elem_offset: 0,
            elem_count: resource.bytes,
            declared_bytes: None,
        });
    }
    FootprintProgram {
        layout_contracts: Vec::new(),
        events,
    }
}

/// Compose the module's checked mutex trace with its derived footprints.
pub fn compose(
    module: &LoweredMutexModule,
) -> Result<ComposedFootprintCertificate, Vec<CompositionDiagnostic>> {
    compose_footprints_with_mutex_invariant(&footprint_program(module), &module.program)
}
