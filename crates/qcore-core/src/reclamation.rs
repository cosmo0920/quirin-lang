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

//! Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (wave 48, gap entry W10):
//! hazard-pointer and epoch-based reclamation as checked protocols.
//!
//! [`crate::disjoint_footprint`] refuses `AtomicReclamation`: allocation
//! generations alone do not show that lock-free reclamation is safe. This
//! module adds the two classic protocols as checkers over a trace of
//! protocol steps, and a contract layer that maps helper operations to
//! protocol roles.
//!
//! # The protocols
//!
//! An object is `fresh`, then `linked` into the shared structure,
//! `unlinked`, `retired` and finally `reclaimed`.
//!
//! **Hazard pointers** ([`check_hazard_pointer_trace`]).
//! * A thread publishes a hazard in one of its slots, and validates it only
//!   while the object is still linked (the re-read of the source sees it).
//! * It accesses an object only through a validated hazard.
//! * An object is retired only once unlinked, and reclaimed only when
//!   retired and no slot of any thread holds it.
//!
//! **Epoch-based reclamation** ([`check_epoch_trace`]).
//! * A thread enters a critical section at the global epoch, and accesses a
//!   linked object or one unlinked during its critical section.
//! * The epoch advances only when every active thread is in the current
//!   epoch.
//! * A retired object is reclaimed two epochs after its retirement.
//!
//! The checkers enforce only these rules. They never test "not reclaimed"
//! at an access. That no accepted trace accesses a reclaimed object is
//! proved from the rules (`QCore.Reclamation.hp_accepted_safe`,
//! `ebr_accepted_safe`).
//!
//! # Contracts and premises
//!
//! A [`ReclamationContract`] maps each helper operation to a protocol role.
//! It is checked, never trusted:
//! * a duplicate operation, or a role of the other protocol, is refused;
//! * every helper boundary is recorded as a `Declared` assumption on the
//!   certificate (proving the helper's body is a separate obligation);
//! * the contract names the ordering its helpers use for the announce/validate
//!   handshake.
//!
//! The trace model assumes the reclaimer's scan cannot miss a validated
//! reader. That is the store-buffering handshake
//! ([`crate::execution_coverage::announce_validate_handshake`]). The checker
//! decides it over every execution with the contract's declared ordering:
//! * `RaceFree`: the premise is discharged within the coverage bounds;
//! * `Race`: the contract is refused;
//! * `Unknown`: the premise stays `Declared`.
//!
//! A refused trace is `Unknown`, never `Unsafe`: the trace is supplied, not
//! derived from source, so a violation is not a validated reachable defect.
//! The property has no corpus consumer. No C source route builds these
//! traces (`CG29-RECLAMATION-SOURCE-ROUTE`).

use std::collections::{BTreeMap, BTreeSet};

use crate::execution_coverage::{
    CoverageBounds, ProgramVerdict, announce_validate_handshake, check_program_executions,
};
use crate::weak_memory::MemoryOrder;

pub type ObjectId = u32;
pub type ReclaimThreadId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectStatus {
    Fresh,
    Linked,
    Unlinked,
    Retired,
    Reclaimed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HazardSlot {
    Pending(ObjectId),
    Valid(ObjectId),
}

/// One protocol step (`QCore.Reclamation.Ev`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReclamationEvent {
    Link {
        object: ObjectId,
    },
    Unlink {
        object: ObjectId,
    },
    Protect {
        thread: ReclaimThreadId,
        slot: u32,
        object: ObjectId,
    },
    Validate {
        thread: ReclaimThreadId,
        slot: u32,
        object: ObjectId,
    },
    Clear {
        thread: ReclaimThreadId,
        slot: u32,
    },
    /// An access by `thread` (through hazard `slot`, for hazard pointers).
    Access {
        thread: ReclaimThreadId,
        slot: u32,
        object: ObjectId,
    },
    Retire {
        thread: ReclaimThreadId,
        object: ObjectId,
    },
    Reclaim {
        thread: ReclaimThreadId,
        object: ObjectId,
    },
    Enter {
        thread: ReclaimThreadId,
    },
    Exit {
        thread: ReclaimThreadId,
    },
    Advance {
        thread: ReclaimThreadId,
    },
}

/// Why a step is refused (`QCore.Reclamation.Violation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReclamationViolation {
    NotFresh,
    NotLinked,
    OutOfRange,
    ValidationFailed,
    UnprotectedAccess,
    NotUnlinked,
    NotRetired,
    HazardHeld,
    ForeignEvent,
    NotActive,
    AlreadyActive,
    EpochNotQuiescent,
    GracePeriod,
}

/// The first refused step of a trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolViolation {
    pub index: usize,
    pub violation: ReclamationViolation,
}

fn status(statuses: &BTreeMap<ObjectId, ObjectStatus>, object: ObjectId) -> ObjectStatus {
    statuses
        .get(&object)
        .copied()
        .unwrap_or(ObjectStatus::Fresh)
}

#[derive(Clone, Debug, Default)]
struct HazardState {
    status: BTreeMap<ObjectId, ObjectStatus>,
    slots: BTreeMap<(ReclaimThreadId, u32), HazardSlot>,
}

/// `QCore.Reclamation.hpStep`.
fn hazard_step(
    threads: u32,
    slots: u32,
    state: &mut HazardState,
    event: ReclamationEvent,
) -> Result<(), ReclamationViolation> {
    use ReclamationEvent as E;
    use ReclamationViolation as V;
    match event {
        E::Link { object } => {
            if status(&state.status, object) != ObjectStatus::Fresh {
                return Err(V::NotFresh);
            }
            state.status.insert(object, ObjectStatus::Linked);
        }
        E::Unlink { object } => {
            if status(&state.status, object) != ObjectStatus::Linked {
                return Err(V::NotLinked);
            }
            state.status.insert(object, ObjectStatus::Unlinked);
        }
        E::Protect {
            thread,
            slot,
            object,
        } => {
            if thread >= threads || slot >= slots {
                return Err(V::OutOfRange);
            }
            state
                .slots
                .insert((thread, slot), HazardSlot::Pending(object));
        }
        E::Validate {
            thread,
            slot,
            object,
        } => {
            if state.slots.get(&(thread, slot)) != Some(&HazardSlot::Pending(object))
                || status(&state.status, object) != ObjectStatus::Linked
            {
                return Err(V::ValidationFailed);
            }
            state
                .slots
                .insert((thread, slot), HazardSlot::Valid(object));
        }
        E::Clear { thread, slot } => {
            if thread >= threads || slot >= slots {
                return Err(V::OutOfRange);
            }
            state.slots.remove(&(thread, slot));
        }
        E::Access {
            thread,
            slot,
            object,
        } => {
            if state.slots.get(&(thread, slot)) != Some(&HazardSlot::Valid(object)) {
                return Err(V::UnprotectedAccess);
            }
        }
        E::Retire { object, .. } => {
            if status(&state.status, object) != ObjectStatus::Unlinked {
                return Err(V::NotUnlinked);
            }
            state.status.insert(object, ObjectStatus::Retired);
        }
        E::Reclaim { object, .. } => {
            if status(&state.status, object) != ObjectStatus::Retired {
                return Err(V::NotRetired);
            }
            // The scan reads every slot of every thread.
            if state.slots.values().any(|slot| {
                matches!(slot, HazardSlot::Pending(held) | HazardSlot::Valid(held) if *held == object)
            }) {
                return Err(V::HazardHeld);
            }
            state.status.insert(object, ObjectStatus::Reclaimed);
        }
        E::Enter { .. } | E::Exit { .. } | E::Advance { .. } => return Err(V::ForeignEvent),
    }
    Ok(())
}

/// Checks a hazard-pointer trace with `threads` threads of `slots` hazard
/// slots each. `Err` names the first refused step.
pub fn check_hazard_pointer_trace(
    threads: u32,
    slots: u32,
    events: &[ReclamationEvent],
) -> Result<(), ProtocolViolation> {
    let mut state = HazardState::default();
    for (index, event) in events.iter().enumerate() {
        hazard_step(threads, slots, &mut state, *event)
            .map_err(|violation| ProtocolViolation { index, violation })?;
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct EpochState {
    status: BTreeMap<ObjectId, ObjectStatus>,
    epoch: u64,
    announced: BTreeMap<ReclaimThreadId, u64>,
    reach: BTreeMap<ReclaimThreadId, BTreeSet<ObjectId>>,
    retired_at: BTreeMap<ObjectId, u64>,
}

/// `QCore.Reclamation.ebrStep`.
fn epoch_step(
    threads: u32,
    state: &mut EpochState,
    event: ReclamationEvent,
) -> Result<(), ReclamationViolation> {
    use ReclamationEvent as E;
    use ReclamationViolation as V;
    match event {
        E::Link { object } => {
            if status(&state.status, object) != ObjectStatus::Fresh {
                return Err(V::NotFresh);
            }
            state.status.insert(object, ObjectStatus::Linked);
        }
        E::Enter { thread } => {
            if thread >= threads {
                return Err(V::OutOfRange);
            }
            if state.announced.contains_key(&thread) {
                return Err(V::AlreadyActive);
            }
            state.announced.insert(thread, state.epoch);
            state.reach.remove(&thread);
        }
        E::Exit { thread } => {
            if state.announced.remove(&thread).is_none() {
                return Err(V::NotActive);
            }
            state.reach.remove(&thread);
        }
        E::Unlink { object } => {
            if status(&state.status, object) != ObjectStatus::Linked {
                return Err(V::NotLinked);
            }
            state.status.insert(object, ObjectStatus::Unlinked);
            for thread in state.announced.keys() {
                state.reach.entry(*thread).or_default().insert(object);
            }
        }
        E::Access { thread, object, .. } => {
            let reachable = status(&state.status, object) == ObjectStatus::Linked
                || state
                    .reach
                    .get(&thread)
                    .is_some_and(|objects| objects.contains(&object));
            if !state.announced.contains_key(&thread) || !reachable {
                return Err(V::UnprotectedAccess);
            }
        }
        E::Retire { object, .. } => {
            if status(&state.status, object) != ObjectStatus::Unlinked {
                return Err(V::NotUnlinked);
            }
            state.status.insert(object, ObjectStatus::Retired);
            state.retired_at.insert(object, state.epoch);
        }
        E::Advance { .. } => {
            if state
                .announced
                .values()
                .any(|entered| *entered != state.epoch)
            {
                return Err(V::EpochNotQuiescent);
            }
            state.epoch += 1;
        }
        E::Reclaim { object, .. } => {
            if status(&state.status, object) != ObjectStatus::Retired {
                return Err(V::NotRetired);
            }
            let Some(retired) = state.retired_at.get(&object) else {
                return Err(V::NotRetired);
            };
            if retired + 2 > state.epoch {
                return Err(V::GracePeriod);
            }
            state.status.insert(object, ObjectStatus::Reclaimed);
        }
        E::Protect { .. } | E::Validate { .. } | E::Clear { .. } => return Err(V::ForeignEvent),
    }
    Ok(())
}

/// Checks an epoch-based trace with `threads` threads. `Err` names the first
/// refused step.
pub fn check_epoch_trace(
    threads: u32,
    events: &[ReclamationEvent],
) -> Result<(), ProtocolViolation> {
    let mut state = EpochState::default();
    for (index, event) in events.iter().enumerate() {
        epoch_step(threads, &mut state, *event)
            .map_err(|violation| ProtocolViolation { index, violation })?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReclamationProtocol {
    HazardPointers { threads: u32, slots: u32 },
    EpochBased { threads: u32 },
}

/// The protocol role a helper operation plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolRole {
    Link,
    Unlink,
    Protect,
    Validate,
    /// Publish a hazard and re-read the source until it is stable: a
    /// `Protect` then a `Validate` of the same slot.
    ProtectAndValidate,
    Clear,
    Access,
    Retire,
    Reclaim,
    Enter,
    Exit,
    Advance,
}

impl ProtocolRole {
    const fn belongs_to(self, protocol: ReclamationProtocol) -> bool {
        match self {
            Self::Link | Self::Unlink | Self::Access | Self::Retire | Self::Reclaim => true,
            Self::Protect | Self::Validate | Self::ProtectAndValidate | Self::Clear => {
                matches!(protocol, ReclamationProtocol::HazardPointers { .. })
            }
            Self::Enter | Self::Exit | Self::Advance => {
                matches!(protocol, ReclamationProtocol::EpochBased { .. })
            }
        }
    }
}

/// One helper operation and its role. Its body is not checked here: the
/// boundary is a `Declared` assumption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationContract {
    pub name: String,
    pub role: ProtocolRole,
}

/// The ordering the helpers use for the announce/validate handshake: the
/// order of the announcement store and the validating load, and whether a
/// seq_cst fence separates them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandshakeOrdering {
    pub access_order: MemoryOrder,
    pub fences: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReclamationContract {
    pub protocol: ReclamationProtocol,
    pub operations: Vec<OperationContract>,
    pub handshake: HandshakeOrdering,
}

/// One call of a helper in the trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolCall {
    pub thread: ReclaimThreadId,
    pub operation: String,
    pub object: Option<ObjectId>,
    pub slot: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReclamationRefusal {
    DuplicateOperation(String),
    RoleOfOtherProtocol {
        operation: String,
        role: ProtocolRole,
    },
    UncontractedOperation(String),
    MissingOperand {
        index: usize,
        operation: String,
    },
    /// The handshake with the contract's ordering races in some execution:
    /// a reader can validate while the reclaimer misses its announcement.
    HandshakeRaces,
    Protocol(ProtocolViolation),
}

/// The handshake premise of a certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandshakeBasis {
    /// Decided over every execution of the handshake fragment; the string is
    /// the coverage evidence's assumption.
    ExecutionCoverage(String),
    /// Not decided (the coverage verdict was `Unknown`).
    Declared,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReclamationCertificate {
    pub property: &'static str,
    pub protocol: ReclamationProtocol,
    pub steps: usize,
    pub handshake: HandshakeBasis,
    pub assumptions: Vec<String>,
}

/// The handshake fragment's bounds: two threads, at most twelve events and
/// two values per location (`announce_validate_handshake`).
pub const HANDSHAKE_BOUNDS: CoverageBounds = CoverageBounds {
    threads: 2,
    events: 12,
    values: 2,
    candidates: 1024,
};

/// Checks a contract and a trace of its helpers' calls.
pub fn check_reclamation(
    contract: &ReclamationContract,
    calls: &[ProtocolCall],
) -> Result<ReclamationCertificate, ReclamationRefusal> {
    let mut roles = BTreeMap::new();
    for operation in &contract.operations {
        if !operation.role.belongs_to(contract.protocol) {
            return Err(ReclamationRefusal::RoleOfOtherProtocol {
                operation: operation.name.clone(),
                role: operation.role,
            });
        }
        if roles
            .insert(operation.name.clone(), operation.role)
            .is_some()
        {
            return Err(ReclamationRefusal::DuplicateOperation(
                operation.name.clone(),
            ));
        }
    }
    let fragment =
        announce_validate_handshake(contract.handshake.access_order, contract.handshake.fences);
    let handshake = match check_program_executions(&fragment, HANDSHAKE_BOUNDS) {
        ProgramVerdict::RaceFree(evidence) => {
            HandshakeBasis::ExecutionCoverage(evidence.assumption())
        }
        ProgramVerdict::Race { .. } => return Err(ReclamationRefusal::HandshakeRaces),
        ProgramVerdict::Unknown(_) => HandshakeBasis::Declared,
    };
    let mut events = Vec::new();
    let mut used = BTreeSet::new();
    for (index, call) in calls.iter().enumerate() {
        let role = *roles
            .get(&call.operation)
            .ok_or_else(|| ReclamationRefusal::UncontractedOperation(call.operation.clone()))?;
        used.insert(call.operation.clone());
        let missing = || ReclamationRefusal::MissingOperand {
            index,
            operation: call.operation.clone(),
        };
        let thread = call.thread;
        let object = || call.object.ok_or_else(missing);
        let slot = || call.slot.ok_or_else(missing);
        use ReclamationEvent as E;
        match role {
            ProtocolRole::Link => events.push(E::Link { object: object()? }),
            ProtocolRole::Unlink => events.push(E::Unlink { object: object()? }),
            ProtocolRole::Protect => events.push(E::Protect {
                thread,
                slot: slot()?,
                object: object()?,
            }),
            ProtocolRole::Validate => events.push(E::Validate {
                thread,
                slot: slot()?,
                object: object()?,
            }),
            ProtocolRole::ProtectAndValidate => {
                let (slot, object) = (slot()?, object()?);
                events.push(E::Protect {
                    thread,
                    slot,
                    object,
                });
                events.push(E::Validate {
                    thread,
                    slot,
                    object,
                });
            }
            ProtocolRole::Clear => events.push(E::Clear {
                thread,
                slot: slot()?,
            }),
            ProtocolRole::Access => events.push(E::Access {
                thread,
                slot: call.slot.unwrap_or(0),
                object: object()?,
            }),
            ProtocolRole::Retire => events.push(E::Retire {
                thread,
                object: object()?,
            }),
            ProtocolRole::Reclaim => events.push(E::Reclaim {
                thread,
                object: object()?,
            }),
            ProtocolRole::Enter => events.push(E::Enter { thread }),
            ProtocolRole::Exit => events.push(E::Exit { thread }),
            ProtocolRole::Advance => events.push(E::Advance { thread }),
        }
    }
    match contract.protocol {
        ReclamationProtocol::HazardPointers { threads, slots } => {
            check_hazard_pointer_trace(threads, slots, &events)
        }
        ReclamationProtocol::EpochBased { threads } => check_epoch_trace(threads, &events),
    }
    .map_err(ReclamationRefusal::Protocol)?;
    let mut assumptions = vec![
        "the trace is one interleaving of the protocol steps, supplied by the caller, not derived from C source (CG29-RECLAMATION-SOURCE-ROUTE)".to_owned(),
        "the Rust protocol checker corresponds to QCore.Reclamation by review, bound on the shared trace vectors".to_owned(),
    ];
    assumptions.extend(contract.operations.iter().filter(|operation| used.contains(&operation.name)).map(|operation| {
        format!(
            "Declared boundary: helper `{}` performs the {:?} step; its body is not checked",
            operation.name, operation.role
        )
    }));
    assumptions.push(match &handshake {
        HandshakeBasis::ExecutionCoverage(assumption) => format!(
            "the announce/validate handshake with the declared ordering ({:?}, fences {}) is race-free: {assumption}; that the helpers use this ordering is Declared",
            contract.handshake.access_order, contract.handshake.fences
        ),
        HandshakeBasis::Declared => {
            "Declared: the announce/validate handshake orders the reclaimer's scan after every validated reader".to_owned()
        }
    });
    Ok(ReclamationCertificate {
        property: "no access to a reclaimed object in the checked protocol trace",
        protocol: contract.protocol,
        steps: events.len(),
        handshake,
        assumptions,
    })
}

#[cfg(test)]
#[path = "../tests/unit/reclamation.rs"]
mod tests;
