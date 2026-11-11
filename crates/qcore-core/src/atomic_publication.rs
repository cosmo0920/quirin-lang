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

//! Bounded C11 atomic-publication judgment.
//!
//! This checks one closed, finite publication shape.  The ready value must be
//! distinct from the initializer, so an observed ready value identifies the
//! unique release store rather than guessing a reads-from edge.  Source
//! extraction and the closed-world writer inventory are gateway obligations.

use crate::weak_memory::{
    C11Execution, C11ExecutionCertificate, MemoryEvent, MemoryEventKind, MemoryLocation,
    MemoryOrder, RelationEdge, WeakMemoryError, check_c11_execution,
};

const FLAG_INIT: u32 = 0;
const PAYLOAD_WRITE: u32 = 1;
const FLAG_STORE: u32 = 2;
const FLAG_LOAD: u32 = 3;
const PAYLOAD_READ: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationProgram {
    pub flag_location: MemoryLocation,
    pub payload_location: MemoryLocation,
    pub initial_value: i64,
    pub published_value: i64,
    /// Value observed on the consumer's successful branch.
    pub observed_value: i64,
    pub store_order: MemoryOrder,
    pub load_order: MemoryOrder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationCertificate {
    pub execution: C11ExecutionCertificate,
    pub release_reads_from: RelationEdge,
    pub payload_happens_before: RelationEdge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicPublicationRefusal {
    AliasedLocations,
    AmbiguousPublishedValue,
    SuccessfulBranchNotObserved,
    StoreIsNotRelease,
    LoadIsNotAcquire,
    Execution(Vec<WeakMemoryError>),
    MissingPayloadHappensBefore,
}

pub fn check_atomic_publication(
    program: &AtomicPublicationProgram,
) -> Result<AtomicPublicationCertificate, AtomicPublicationRefusal> {
    if program.flag_location == program.payload_location {
        return Err(AtomicPublicationRefusal::AliasedLocations);
    }
    if program.initial_value == program.published_value {
        return Err(AtomicPublicationRefusal::AmbiguousPublishedValue);
    }
    if program.observed_value != program.published_value {
        return Err(AtomicPublicationRefusal::SuccessfulBranchNotObserved);
    }
    if !matches!(
        program.store_order,
        MemoryOrder::Release | MemoryOrder::SequentiallyConsistent
    ) {
        return Err(AtomicPublicationRefusal::StoreIsNotRelease);
    }
    if !matches!(
        program.load_order,
        MemoryOrder::Acquire | MemoryOrder::SequentiallyConsistent
    ) {
        return Err(AtomicPublicationRefusal::LoadIsNotAcquire);
    }

    let release_reads_from = RelationEdge::new(FLAG_STORE, FLAG_LOAD);
    let payload_happens_before = RelationEdge::new(PAYLOAD_WRITE, PAYLOAD_READ);
    let execution = C11Execution {
        events: vec![
            MemoryEvent {
                id: FLAG_INIT,
                thread: 0,
                kind: MemoryEventKind::AtomicInit {
                    location: program.flag_location,
                    value: program.initial_value,
                },
            },
            MemoryEvent {
                id: PAYLOAD_WRITE,
                thread: 1,
                kind: MemoryEventKind::NonAtomicWrite {
                    location: program.payload_location,
                },
            },
            MemoryEvent {
                id: FLAG_STORE,
                thread: 1,
                kind: MemoryEventKind::AtomicStore {
                    location: program.flag_location,
                    value: program.published_value,
                    order: program.store_order,
                },
            },
            MemoryEvent {
                id: FLAG_LOAD,
                thread: 2,
                kind: MemoryEventKind::AtomicLoad {
                    location: program.flag_location,
                    value: program.observed_value,
                    order: program.load_order,
                },
            },
            MemoryEvent {
                id: PAYLOAD_READ,
                thread: 2,
                kind: MemoryEventKind::NonAtomicRead {
                    location: program.payload_location,
                },
            },
        ],
        // The initializer cannot supply `published_value`; the release store is
        // the closed model's only other flag write.
        reads_from: vec![release_reads_from],
        modification_order: vec![RelationEdge::new(FLAG_INIT, FLAG_STORE)],
        synchronizes_with: vec![release_reads_from],
        sequentially_consistent: match (program.store_order, program.load_order) {
            (MemoryOrder::SequentiallyConsistent, MemoryOrder::SequentiallyConsistent) => {
                vec![FLAG_STORE, FLAG_LOAD]
            }
            (MemoryOrder::SequentiallyConsistent, _) => vec![FLAG_STORE],
            (_, MemoryOrder::SequentiallyConsistent) => vec![FLAG_LOAD],
            _ => vec![],
        },
        source_observation: vec![program.observed_value],
        target_observation: vec![program.observed_value],
        ..C11Execution::default()
    };
    let certificate =
        check_c11_execution(&execution).map_err(AtomicPublicationRefusal::Execution)?;
    if !certificate.happens_before.contains(&payload_happens_before) {
        return Err(AtomicPublicationRefusal::MissingPayloadHappensBefore);
    }
    Ok(AtomicPublicationCertificate {
        execution: certificate,
        release_reads_from,
        payload_happens_before,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> AtomicPublicationProgram {
        AtomicPublicationProgram {
            flag_location: 10,
            payload_location: 11,
            initial_value: 0,
            published_value: 1,
            observed_value: 1,
            store_order: MemoryOrder::Release,
            load_order: MemoryOrder::Acquire,
        }
    }

    #[test]
    fn release_acquire_orders_payload() {
        let certificate = check_atomic_publication(&program()).unwrap();
        assert_eq!(certificate.payload_happens_before, RelationEdge::new(1, 4));
    }

    #[test]
    fn controls_are_refused() {
        let mut case = program();
        case.store_order = MemoryOrder::Relaxed;
        assert_eq!(
            check_atomic_publication(&case),
            Err(AtomicPublicationRefusal::StoreIsNotRelease)
        );
        let mut case = program();
        case.load_order = MemoryOrder::Relaxed;
        assert_eq!(
            check_atomic_publication(&case),
            Err(AtomicPublicationRefusal::LoadIsNotAcquire)
        );
        let mut case = program();
        case.initial_value = 1;
        assert_eq!(
            check_atomic_publication(&case),
            Err(AtomicPublicationRefusal::AmbiguousPublishedValue)
        );
        let mut case = program();
        case.observed_value = 2;
        assert_eq!(
            check_atomic_publication(&case),
            Err(AtomicPublicationRefusal::SuccessfulBranchNotObserved)
        );
        let mut case = program();
        case.payload_location = case.flag_location;
        assert_eq!(
            check_atomic_publication(&case),
            Err(AtomicPublicationRefusal::AliasedLocations)
        );
    }
}
