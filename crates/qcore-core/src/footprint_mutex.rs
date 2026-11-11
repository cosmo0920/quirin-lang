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

//! Card `CG18-FOOTPRINT-CONNECTION` (gap entry W08): the checked
//! mutex-invariant profile composed with derived access footprints.
//!
//! [`crate::mutex_invariant`] takes every [`AccessFootprint`] it is handed on
//! trust and stamps [`ConcurrencyAssumption::DeclaredFootprintShape`]: the
//! declared intervals are the protected object's real layout. It also treats
//! footprints of two distinct resources as never overlapping
//! ([`AccessFootprint::overlaps`]), which is a cross-allocation disjointness
//! claim the footprint profile refuses to make for free (`Q662`, M21).
//!
//! [`compose_footprints_with_mutex_invariant`] runs both checkers unedited and
//! admits the pair only where the footprint profile backs every interval the
//! mutex trace names:
//!
//! * **every footprint is derived.** Each interval anywhere in the mutex
//!   trace (initial shares, protected shapes, accesses, borrows, facts,
//!   growths) must be exactly the exported interval of a live handle the
//!   footprint checker derived, its resource being that handle's allocation.
//!   A bit-field storage unit exports no interval and a descriptor no bytes,
//!   so neither can stand for one. Otherwise the composition refuses
//!   ([`CompositionDiagnostic::UnderivedFootprint`], `Q665` widened);
//! * **every cross-resource pair is backed.** Two bound footprints on distinct
//!   allocations are disjoint only under
//!   [`FootprintAssumption::DistinctAllocationsDisjoint`]: both live
//!   generations were declared fresh. Otherwise the composition refuses
//!   ([`CompositionDiagnostic::CrossAllocationWithoutPremise`], `Q662`
//!   widened).
//!
//! An admitted composition's premise set is the mutex certificate's without
//! `DeclaredFootprintShape`, which the derivation relocates to the footprint
//! profile's own named premises (`AllocationExtentDeclared`,
//! `DeclaredRecordLayout`, and `P-DISTINCT-ALLOCATIONS-DISJOINT` when a
//! cross-resource pair used it), all carried on the composed certificate.
//! The composition decides no scheduling and adds no happens-before edge: the
//! mutex profile's own decisions are unchanged, and a trace it refuses stays
//! refused. The Lean model is `QCore.DisjointFootprint`'s composition section
//! (`composition_backs_every_mutex_disjointness`).

use std::collections::{BTreeMap, BTreeSet};

use crate::concurrency::{AccessFootprint, ConcurrencyAssumption};
use crate::disjoint_footprint::{
    AllocationId, DerivedFootprint, FootprintAssumption, FootprintCertificate, FootprintDiagnostic,
    FootprintExport, FootprintProgram, HandleId, check_disjoint_footprint_program,
};
use crate::mutex_invariant::{
    MutexInvariantCertificate, MutexInvariantDiagnostic, MutexInvariantEvent,
    MutexInvariantProgram, check_mutex_invariant_program,
};

/// A premise of the composed certificate, from either profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComposedAssumption {
    Mutex(ConcurrencyAssumption),
    Footprint(FootprintAssumption),
}

/// Why a composition was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionDiagnostic {
    /// The footprint checker refused its own program; its diagnostics, with
    /// their own codes.
    Footprint(Vec<FootprintDiagnostic>),
    /// The mutex checker refused its own program.
    Mutex(Vec<MutexInvariantDiagnostic>),
    /// `Q665` widened: an interval the mutex trace names that no live derived
    /// handle exports exactly.
    UnderivedFootprint { footprint: AccessFootprint },
    /// `Q662` widened: two bound footprints on distinct allocations whose live
    /// generations were not both declared fresh.
    CrossAllocationWithoutPremise {
        left: AllocationId,
        right: AllocationId,
    },
}

impl CompositionDiagnostic {
    /// The code this refusal surfaces as: a wrapped refusal keeps its first
    /// diagnostic's code; the composition's own two are existing conditions of
    /// the footprint band, widened (no code is allocated).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Footprint(diagnostics) => diagnostics.first().map_or("Q667", |d| d.code()),
            Self::Mutex(diagnostics) => diagnostics.first().map_or("Q600", |d| d.code()),
            Self::UnderivedFootprint { .. } => "Q665",
            Self::CrossAllocationWithoutPremise { .. } => "Q662",
        }
    }
}

/// An admitted composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedFootprintCertificate {
    pub footprint: FootprintCertificate,
    pub mutex: MutexInvariantCertificate,
    /// Every interval the mutex trace names, and the derived handle that
    /// exports it (the least such handle id).
    pub bindings: BTreeMap<AccessFootprint, HandleId>,
    /// Every unordered pair of distinct allocations the bound footprints
    /// span, each decided under `P-DISTINCT-ALLOCATIONS-DISJOINT`.
    pub cross_allocation_pairs: Vec<(AllocationId, AllocationId)>,
    pub assumptions: BTreeSet<ComposedAssumption>,
}

/// Every interval a mutex trace names, in first-appearance order.
pub fn mutex_footprints(program: &MutexInvariantProgram) -> Vec<AccessFootprint> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut add = |footprint: AccessFootprint| {
        if seen.insert(footprint) {
            out.push(footprint);
        }
    };
    for grant in &program.initial_shares {
        add(grant.footprint);
    }
    for event in &program.events {
        match event {
            MutexInvariantEvent::CreateMutex { protects, .. } => {
                protects.iter().copied().for_each(&mut add);
            }
            MutexInvariantEvent::Access { footprint, .. }
            | MutexInvariantEvent::BorrowUnderGuard { footprint, .. }
            | MutexInvariantEvent::AssertFact { footprint, .. }
            | MutexInvariantEvent::ExtendProtection { footprint, .. }
            | MutexInvariantEvent::GrowProtection { footprint, .. } => add(*footprint),
            MutexInvariantEvent::Spawn { .. }
            | MutexInvariantEvent::Join { .. }
            | MutexInvariantEvent::GuardLocalJoin { .. }
            | MutexInvariantEvent::DestroyMutex { .. }
            | MutexInvariantEvent::RegisterWaiter { .. }
            | MutexInvariantEvent::Lock { .. }
            | MutexInvariantEvent::Unlock { .. }
            | MutexInvariantEvent::ErrorExit { .. }
            | MutexInvariantEvent::OpenInvariant { .. }
            | MutexInvariantEvent::CloseInvariant { .. }
            | MutexInvariantEvent::EndBorrow { .. }
            | MutexInvariantEvent::UseFact { .. }
            | MutexInvariantEvent::TransferCustody { .. }
            | MutexInvariantEvent::Destroy { .. }
            | MutexInvariantEvent::UnsupportedConstruct { .. } => {}
        }
    }
    out
}

/// The live handle that exports exactly `footprint`, if any: the least id.
fn binding(certificate: &FootprintCertificate, footprint: AccessFootprint) -> Option<HandleId> {
    certificate
        .derived_footprints
        .iter()
        .find(|(_, derived)| {
            let DerivedFootprint::Memory {
                alloc, generation, ..
            } = **derived
            else {
                return false;
            };
            certificate.live_allocations.contains(&(alloc, generation))
                && derived.export() == FootprintExport::Interval(footprint)
        })
        .map(|(handle, _)| *handle)
}

/// Compose a mutex-invariant trace with the footprint derivation of every
/// interval it names. Fail-closed: either checker's refusal, an underived
/// interval, or an unbacked cross-allocation pair refuses the whole
/// composition.
pub fn compose_footprints_with_mutex_invariant(
    footprints: &FootprintProgram,
    mutex: &MutexInvariantProgram,
) -> Result<ComposedFootprintCertificate, Vec<CompositionDiagnostic>> {
    let footprint = check_disjoint_footprint_program(footprints)
        .map_err(|diagnostics| vec![CompositionDiagnostic::Footprint(diagnostics)])?;
    let checked = check_mutex_invariant_program(mutex)
        .map_err(|diagnostics| vec![CompositionDiagnostic::Mutex(diagnostics)])?;
    let mut diagnostics = Vec::new();
    let mut bindings = BTreeMap::new();
    for interval in mutex_footprints(mutex) {
        match binding(&footprint, interval) {
            Some(handle) => {
                bindings.insert(interval, handle);
            }
            None => diagnostics.push(CompositionDiagnostic::UnderivedFootprint {
                footprint: interval,
            }),
        }
    }
    let allocations = bindings
        .values()
        .filter_map(|handle| match footprint.derived_footprints.get(handle) {
            Some(DerivedFootprint::Memory {
                alloc, generation, ..
            }) => Some((*alloc, *generation)),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut cross = Vec::new();
    for (index, left) in allocations.iter().enumerate() {
        for right in &allocations[index + 1..] {
            let fresh = footprint.fresh_allocations.contains(left)
                && footprint.fresh_allocations.contains(right);
            if fresh {
                cross.push((left.0, right.0));
            } else {
                diagnostics.push(CompositionDiagnostic::CrossAllocationWithoutPremise {
                    left: left.0,
                    right: right.0,
                });
            }
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut assumptions = checked
        .assumptions
        .iter()
        .filter(|assumption| **assumption != ConcurrencyAssumption::DeclaredFootprintShape)
        .map(|assumption| ComposedAssumption::Mutex(*assumption))
        .collect::<BTreeSet<_>>();
    assumptions.extend(
        footprint
            .assumptions
            .iter()
            .map(|assumption| ComposedAssumption::Footprint(*assumption)),
    );
    if !cross.is_empty() {
        assumptions.insert(ComposedAssumption::Footprint(
            FootprintAssumption::DistinctAllocationsDisjoint,
        ));
    }
    Ok(ComposedFootprintCertificate {
        footprint,
        mutex: checked,
        bindings,
        cross_allocation_pairs: cross,
        assumptions,
    })
}

#[cfg(test)]
#[path = "../tests/unit/footprint_mutex.rs"]
mod tests;
