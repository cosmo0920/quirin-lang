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

//! F02 — one source-to-native trace relation.
//!
//! One *admitted causal order* is derived from a reference run of a checked
//! finite task module, and an independently observed trace (for example the
//! stdout of the generated C11 program) is related to it by **admitted causal
//! constraints**, not by demanding an identical total schedule.
//!
//! The relation is deliberately narrow. It relates two renderings of the same
//! run of one program: it preserves resource identity, it accounts for cleanup,
//! and it constrains the order only where the profile actually orders things.
//! Everything else — which of the admitted linearizations a particular native
//! execution really took, whether the C11 backend is a refinement of the
//! reference for programs outside this profile, arbitrary schedules, arbitrary
//! failure injection — is outside every statement made here.
//!
//! Premises, named so they are not silently assumed:
//!
//! * **P1 (sequential actors).** Each task in this profile is one sequential
//!   thread of control, so the reference run's order *restricted to one actor*
//!   is that actor's program order and is a genuine causal constraint. Cross-
//!   actor order in the reference run is not a constraint and is not used.
//! * **P2 (attribution).** An observed line is attributed to the task whose
//!   state change it reports, not to the thread that printed it. The shipped
//!   backend buffers a child's buffer events and prints them from the parent
//!   after a successful join; the relation reads them as the child's.
//! * **P3 (flat scopes).** Every spawned child's parent is the root task. The
//!   checked profile enforces this — workers are nonrecursive task-free leaves
//!   — and [`AdmittedCausalOrder::derive`] re-checks it against the reference
//!   observation's own task table rather than assuming it.
//!
//! A trace that violates an admitted constraint is refused with the constraint
//! it violated. It is never normalised, reordered or repaired.

use crate::scoped_tasks::{ROOT_TASK, ResourceId, TaskId, TaskOutcome};
use crate::task_execution::{TaskEvent, TaskExecutionObservation};
use std::collections::{BTreeMap, BTreeSet};

/// Identity of this relation, bound into recorded native evidence so that a
/// changed relation retires the record instead of silently reinterpreting it.
pub const TRACE_RELATION_IDENTITY: &str = "qcore.task-trace-relation.v1";

/// The outcome an observed completion reports.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservedOutcome {
    Unit,
    I32(i32),
    Buffer(ResourceId),
    Error(String),
    /// A worker result of a §G02t type other than `i32` (card
    /// `W48-QR-SCALAR-TYPES-R6`, `W48-QR-F02-FIXED-OUTCOMES`), rendered
    /// `QCORE_TASK_RESULT TASK TYPE NUMBER` (`true`/`false` for `bool`). The
    /// relation observes an outcome only through its equality and whether it
    /// names a Buffer, and this one never does, so the alphabet grows
    /// conservatively: a trace without one relates exactly as before
    /// (`QCore.TaskTraceFixedOutcomes.fixed_free_traces_relate_exactly_as_before`)
    /// and no pinned digest moves. `ty` is never `i32`.
    Fixed {
        ty: crate::scalar_types::ScalarType,
        value: i128,
    },
}

/// The observable alphabet shared by the reference run and the native run.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservedEvent {
    Allocated {
        task: TaskId,
        buffer: ResourceId,
        payload: i32,
    },
    AllocationFailed {
        task: TaskId,
    },
    Spawned {
        child: TaskId,
        worker: u64,
    },
    /// The one line in the alphabet that names no task. It is *unattributable*
    /// and therefore refused; this profile does not guess a creator.
    CreationFailed {
        worker: u64,
    },
    Read {
        task: TaskId,
        buffer: ResourceId,
        value: i32,
    },
    /// Card `CT03-PROOF-ALPHABET`: the T05 Buffer write, previously refused by
    /// [`reference_trace`] as an unsupported observed event. Admitting it is a
    /// **conservative extension**: a trace without a write relates exactly as
    /// before (`QCore.TaskTraceWrites.write_free_traces_relate_exactly_as_before`),
    /// and every pre-existing digest and verdict pin is unchanged.
    Written {
        task: TaskId,
        buffer: ResourceId,
        value: i32,
    },
    Dropped {
        task: TaskId,
        buffer: ResourceId,
    },
    Loaned {
        borrower: TaskId,
        buffer: ResourceId,
    },
    Returned {
        borrower: TaskId,
        buffer: ResourceId,
    },
    Completed {
        task: TaskId,
        outcome: ObservedOutcome,
    },
    Joined {
        child: TaskId,
    },
}

impl ObservedEvent {
    /// P2. The task whose state change this line reports.
    pub fn actor(&self) -> Option<TaskId> {
        match self {
            Self::Allocated { task, .. }
            | Self::AllocationFailed { task }
            | Self::Read { task, .. }
            | Self::Written { task, .. }
            | Self::Dropped { task, .. }
            | Self::Completed { task, .. } => Some(*task),
            // P3: the creator, lender and joiner is the root task.
            Self::Spawned { .. }
            | Self::Joined { .. }
            | Self::Loaned { .. }
            | Self::Returned { .. } => Some(ROOT_TASK),
            Self::CreationFailed { .. } => None,
        }
    }

    /// The resource identity this line mentions, if any.
    pub fn buffer(&self) -> Option<ResourceId> {
        match self {
            Self::Allocated { buffer, .. }
            | Self::Read { buffer, .. }
            | Self::Written { buffer, .. }
            | Self::Dropped { buffer, .. }
            | Self::Loaned { buffer, .. }
            | Self::Returned { buffer, .. } => Some(*buffer),
            Self::Completed {
                outcome: ObservedOutcome::Buffer(buffer),
                ..
            } => Some(*buffer),
            _ => None,
        }
    }

    /// The child task this line is *about*, which is not always its actor: a
    /// creation, a join, a loan and a loan return are all the parent's actions
    /// about a child.
    pub fn subject(&self) -> Option<TaskId> {
        match self {
            Self::Spawned { child, .. } | Self::Joined { child } => Some(*child),
            Self::Loaned { borrower, .. } | Self::Returned { borrower, .. } => Some(*borrower),
            Self::Completed { task, .. } if *task != ROOT_TASK => Some(*task),
            Self::Allocated { task, .. }
            | Self::AllocationFailed { task }
            | Self::Read { task, .. }
            | Self::Written { task, .. }
            | Self::Dropped { task, .. }
                if *task != ROOT_TASK =>
            {
                Some(*task)
            }
            _ => None,
        }
    }
}

/// The admitted causal constraint a trace violated. The relation reports the
/// first violation in a fixed conjunct order, so a classification is stable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceRelationViolation {
    /// R1 — a line that names no task.
    UnattributableEvent { index: usize },
    /// R2 — two indistinguishable occurrences; the relation refuses rather than
    /// guessing which one a constraint is about.
    AmbiguousOccurrence { index: usize },
    /// R2 — the observed trace is not the same collection of facts.
    EventMultisetMismatch,
    /// R3 (P1) — one actor's own events are not in its program order.
    ActorOrder { actor: TaskId },
    /// R4 — one generation allocated twice.
    DuplicateAllocation { buffer: ResourceId },
    /// R4 — a generation used with no allocation in the trace.
    UnallocatedUse { buffer: ResourceId },
    /// R4 — a generation used before its allocation.
    AllocationOrder { buffer: ResourceId },
    /// R4 — a read that does not report the established payload: the
    /// allocated payload for a generation that is never written, or the value
    /// of the nearest preceding allocation or write for one that is.
    PayloadMismatch { buffer: ResourceId },
    /// R5 — one generation released twice.
    DuplicateRelease { buffer: ResourceId },
    /// R5 — a generation used after its release.
    ReleaseOrder { buffer: ResourceId },
    /// R5 — cleanup: a generation allocated and never released.
    UnreleasedBuffer { buffer: ResourceId },
    /// R6 — one child created twice.
    DuplicateCreation { child: TaskId },
    /// R6 — a child that is never created.
    UnknownChild { child: TaskId },
    /// R6 — an event about or by a child before its creation.
    CreationOrder { child: TaskId },
    /// R7 — one child joined twice.
    DuplicateJoin { child: TaskId },
    /// R7 — cleanup: a child created and never joined.
    UnjoinedChild { child: TaskId },
    /// R7 — a join that precedes the child's completion.
    CompletionOrder { child: TaskId },
    /// R7 — a child's own activity after it was joined.
    ActivityAfterJoin { child: TaskId },
    /// R8 — a loan read before the loan, or a loan the join never closed.
    LoanOrder {
        borrower: TaskId,
        buffer: ResourceId,
    },
    /// R9 — the root's completion is not the final fact.
    RootResultNotFinal,
}

impl TraceRelationViolation {
    /// A stable numeric class. The Lean transcription returns exactly these
    /// codes, so a domain census compares class-for-class.
    pub fn code(&self) -> u32 {
        match self {
            Self::UnattributableEvent { .. } => 1,
            Self::AmbiguousOccurrence { .. } => 2,
            Self::EventMultisetMismatch => 3,
            Self::ActorOrder { .. } => 4,
            Self::DuplicateAllocation { .. } => 5,
            Self::UnallocatedUse { .. } => 6,
            Self::AllocationOrder { .. } => 7,
            Self::PayloadMismatch { .. } => 8,
            Self::DuplicateRelease { .. } => 9,
            Self::ReleaseOrder { .. } => 10,
            Self::UnreleasedBuffer { .. } => 11,
            Self::DuplicateCreation { .. } => 12,
            Self::UnknownChild { .. } => 13,
            Self::CreationOrder { .. } => 14,
            Self::DuplicateJoin { .. } => 15,
            Self::UnjoinedChild { .. } => 16,
            Self::CompletionOrder { .. } => 17,
            Self::ActivityAfterJoin { .. } => 18,
            Self::LoanOrder { .. } => 19,
            Self::RootResultNotFinal => 20,
        }
    }
}

impl std::fmt::Display for TraceRelationViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trace relation violation {} ({self:?})", self.code())
    }
}
impl std::error::Error for TraceRelationViolation {}

/// A rendered line the observer could not read at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceParseError {
    pub line: usize,
    pub text: String,
}
impl std::fmt::Display for TraceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unreadable observation line {}: {:?}",
            self.line, self.text
        )
    }
}
impl std::error::Error for TraceParseError {}

fn number(word: Option<&str>) -> Option<u64> {
    word?.parse::<u64>().ok()
}
fn signed(word: Option<&str>) -> Option<i32> {
    word?.parse::<i32>().ok()
}

/// A fixed-width worker outcome's `TYPE NUMBER` (card
/// `W48-QR-SCALAR-TYPES-R6`): the type's upper-case name — never `I32`, which
/// is its own arm — and a number of the type, `true`/`false` for `BOOL`. The
/// parser knows no recorded target, so an `ISIZE`/`USIZE` number is read
/// against the widest width; the relation then demands it equal the
/// reference's, which is a value of the recorded one.
fn fixed_outcome(kind: &str, word: Option<&str>) -> Option<ObservedOutcome> {
    use crate::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
    // Card `W48-QR-FLOATING-POINT`: `F32`/`F64` too, a bit pattern or `nan`.
    let ty = crate::scalar_types::ALL
        .into_iter()
        .chain(crate::scalar_types::FLOATS)
        .find(|ty| *ty != ScalarType::I32 && ty.name().to_ascii_uppercase() == kind)?;
    let word = word?;
    let value = if ty == ScalarType::Bool {
        match word {
            "true" => 1,
            "false" => 0,
            _ => return None,
        }
    } else if ty.is_float() && word == "nan" {
        crate::scalar_floats::canonical_nan(ty)
    } else {
        word.parse::<i128>().ok()?
    };
    let widest = RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: String::new(),
        compiler: String::new(),
    };
    ty.contains(value, Some(&widest))
        .then_some(ObservedOutcome::Fixed { ty, value })
}

/// Read a rendered observation — the reference renderer's output, or the
/// generated program's stdout — into the observable alphabet. Any line the
/// alphabet does not cover is an error, never a skipped line.
pub fn parse_observed_trace(text: &str) -> Result<Vec<ObservedEvent>, TraceParseError> {
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let mut words = line.split(' ');
        let tag = words.next().unwrap_or_default();
        let mut rest = words.clone();
        let parsed = match tag {
            "QCORE_BUFFER_ALLOC" => match (
                number(rest.next()),
                number(rest.next()),
                signed(rest.next()),
            ) {
                (Some(task), Some(buffer), Some(payload)) => Some(ObservedEvent::Allocated {
                    task,
                    buffer,
                    payload,
                }),
                _ => None,
            },
            "QCORE_BUFFER_ALLOC_FAILED" => {
                number(rest.next()).map(|task| ObservedEvent::AllocationFailed { task })
            }
            "QCORE_BUFFER_READ" => match (
                number(rest.next()),
                number(rest.next()),
                signed(rest.next()),
            ) {
                (Some(task), Some(buffer), Some(value)) => Some(ObservedEvent::Read {
                    task,
                    buffer,
                    value,
                }),
                _ => None,
            },
            "QCORE_BUFFER_WRITE" => match (
                number(rest.next()),
                number(rest.next()),
                signed(rest.next()),
            ) {
                (Some(task), Some(buffer), Some(value)) => Some(ObservedEvent::Written {
                    task,
                    buffer,
                    value,
                }),
                _ => None,
            },
            "QCORE_BUFFER_DROP" => match (number(rest.next()), number(rest.next())) {
                (Some(task), Some(buffer)) => Some(ObservedEvent::Dropped { task, buffer }),
                _ => None,
            },
            "QCORE_BUFFER_LOAN" => match (number(rest.next()), number(rest.next())) {
                (Some(borrower), Some(buffer)) => Some(ObservedEvent::Loaned { borrower, buffer }),
                _ => None,
            },
            "QCORE_BUFFER_RETURN" => match (number(rest.next()), number(rest.next())) {
                (Some(borrower), Some(buffer)) => {
                    Some(ObservedEvent::Returned { borrower, buffer })
                }
                _ => None,
            },
            "QCORE_TASK_SPAWN" => match (number(rest.next()), number(rest.next())) {
                (Some(child), Some(worker)) => Some(ObservedEvent::Spawned { child, worker }),
                _ => None,
            },
            "QCORE_TASK_CREATE_FAILED" => {
                number(rest.next()).map(|worker| ObservedEvent::CreationFailed { worker })
            }
            "QCORE_TASK_JOIN" => number(rest.next()).map(|child| ObservedEvent::Joined { child }),
            "QCORE_TASK_RESULT" => {
                let task = number(rest.next());
                let kind = rest.next();
                match (task, kind) {
                    (Some(task), Some("UNIT")) if rest.next().is_none() => {
                        Some(ObservedEvent::Completed {
                            task,
                            outcome: ObservedOutcome::Unit,
                        })
                    }
                    (Some(task), Some("I32")) => {
                        signed(rest.next()).map(|value| ObservedEvent::Completed {
                            task,
                            outcome: ObservedOutcome::I32(value),
                        })
                    }
                    (Some(task), Some("BUFFER")) => {
                        number(rest.next()).map(|buffer| ObservedEvent::Completed {
                            task,
                            outcome: ObservedOutcome::Buffer(buffer),
                        })
                    }
                    (Some(task), Some("ERROR")) => {
                        rest.next().map(|tag| ObservedEvent::Completed {
                            task,
                            outcome: ObservedOutcome::Error(tag.to_owned()),
                        })
                    }
                    (Some(task), Some(kind)) => fixed_outcome(kind, rest.next())
                        .map(|outcome| ObservedEvent::Completed { task, outcome }),
                    _ => None,
                }
            }
            _ => None,
        };
        // A trailing word the alphabet does not use is a different line, not
        // this one: read the whole line or refuse it.
        let parsed = parsed.filter(|_| rest.next().is_none());
        events.push(parsed.ok_or_else(|| TraceParseError {
            line: index + 1,
            text: line.to_owned(),
        })?);
    }
    Ok(events)
}

/// An observation event outside this relation's observable alphabet. Since
/// card `CT03-PROOF-ALPHABET` the T05 Buffer write is inside the alphabet
/// ([`ObservedEvent::Written`]); the T06 typed error payload still has no
/// admitted rendering here, and neither do card `WRITE-LEND-TASKS`'s range
/// write lease events, so an observation carrying one of those is refused
/// rather than encoded — never silently dropped.
///
/// The `index` each variant carries is the **offending event's own** position
/// in the observation, and the first such position; proved as
/// `QCore.TaskTraceLeaseBoundary.refusal_points_at_a_refused_event` and
/// `…refusal_is_the_first`, and exercised on a real reference run by
/// `compiler/tests/trace_alphabet_lease_boundary.rs` (card
/// `TRACE-ALPHABET-LEASE-BOUNDARY`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceTraceError {
    UnsupportedOutcome { index: usize },
    UnsupportedObservedEvent { index: usize },
}

/// Deriving an admitted order from a reference run refuses either way: an
/// observation the alphabet cannot express, or a rendering the relation's own
/// constraints reject. Neither refusal admits anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceDeriveError {
    Unsupported(ReferenceTraceError),
    Violation(TraceRelationViolation),
}

fn outcome(value: &TaskOutcome, index: usize) -> Result<ObservedOutcome, ReferenceTraceError> {
    match value {
        TaskOutcome::Unit => Ok(ObservedOutcome::Unit),
        TaskOutcome::I32(value) => Ok(ObservedOutcome::I32(*value)),
        TaskOutcome::OwnedBuffer(buffer) => Ok(ObservedOutcome::Buffer(*buffer)),
        TaskOutcome::Error(tag) => Ok(ObservedOutcome::Error(tag.clone())),
        // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-F02-FIXED-OUTCOMES`): the
        // alphabet grew by the fixed-width worker outcome, so a run carrying
        // one is encoded rather than refused. Card R4 had refused it.
        TaskOutcome::Fixed { ty, value } => Ok(ObservedOutcome::Fixed {
            ty: *ty,
            // Card `W48-QR-FLOATING-POINT`: every NaN is one observation.
            value: crate::scalar_floats::canonical(*ty, *value),
        }),
        TaskOutcome::ErrorWithBuffer { .. } => {
            Err(ReferenceTraceError::UnsupportedOutcome { index })
        }
        // Card `W48-QR-TASK-RECORD-CAPTURES`: the observable alphabet has no
        // record outcome, so a run returning one is refused rather than encoded
        // — never flattened into its leaves' outcomes, which would read as
        // several completions of one task (constraint 15n's refusing side).
        TaskOutcome::Record(_) => Err(ReferenceTraceError::UnsupportedOutcome { index }),
    }
}

/// The reference run's own rendering, in the observable alphabet. An event the
/// alphabet cannot express — a T06 error payload, or a `WRITE-LEND-TASKS` range
/// write lease — refuses the whole observation; nothing outside the admitted
/// path is encoded. On success the result is, position for position, the
/// per-event encoding of the input
/// (`QCore.TaskTraceLeaseBoundary.encoding_is_positionwise`).
pub fn reference_trace(
    observation: &TaskExecutionObservation,
) -> Result<Vec<ObservedEvent>, ReferenceTraceError> {
    observation
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            Ok(match event {
                TaskEvent::BufferAllocated {
                    task,
                    buffer,
                    initial,
                } => ObservedEvent::Allocated {
                    task: *task,
                    buffer: *buffer,
                    payload: *initial,
                },
                TaskEvent::BufferAllocationFailed { task } => {
                    ObservedEvent::AllocationFailed { task: *task }
                }
                TaskEvent::BufferRead {
                    task,
                    buffer,
                    value,
                } => ObservedEvent::Read {
                    task: *task,
                    buffer: *buffer,
                    value: *value,
                },
                TaskEvent::BufferDropped { task, buffer } => ObservedEvent::Dropped {
                    task: *task,
                    buffer: *buffer,
                },
                TaskEvent::Spawn { task, worker } => ObservedEvent::Spawned {
                    child: *task,
                    worker: *worker as u64,
                },
                TaskEvent::CreateFailed { worker } => ObservedEvent::CreationFailed {
                    worker: *worker as u64,
                },
                TaskEvent::BufferLoaned { borrower, buffer } => ObservedEvent::Loaned {
                    borrower: *borrower,
                    buffer: *buffer,
                },
                TaskEvent::BufferReturned { borrower, buffer } => ObservedEvent::Returned {
                    borrower: *borrower,
                    buffer: *buffer,
                },
                TaskEvent::Join { task } => ObservedEvent::Joined { child: *task },
                TaskEvent::Result { task, outcome: o } => ObservedEvent::Completed {
                    task: *task,
                    outcome: outcome(o, index)?,
                },
                // CT03-PROOF-ALPHABET grew this relation's observable
                // alphabet by the Buffer write, so it is encoded rather than
                // refused. The WRITE-LEND-TASKS lease events stay outside the
                // alphabet: an observation containing one is refused here
                // rather than encoded — never silently dropped.
                //
                // Proved and exercised by card `TRACE-ALPHABET-LEASE-BOUNDARY`
                // (wave 32), which discharged the residual
                // `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` that recorded this
                // arm as an unproven integration-time resolution:
                //
                // * `compiler/tests/trace_alphabet_lease_boundary.rs` drives a
                //   **real** lease-carrying reference run — card
                //   `WRITE-LEND-TASKS`' own admitted flagship — into this
                //   function, so the arm is reachable today and the refusal is
                //   pinned rather than assumed. Each of its five tests was
                //   shown non-vacuous by mutating this arm.
                // * `QCore.TaskTraceLeaseBoundary` proves the boundary:
                //   `succeeds_iff_no_event_is_refusable` (no lease slips
                //   through, on any shape of run, and the refusal blocks
                //   nothing else), `encoding_is_positionwise` with
                //   `encoding_preserves_length` (the formal content of "never
                //   silently dropped"), and `refusal_points_at_a_refused_event`
                //   with `refusal_is_the_first` (the index below is the
                //   offending event's own, and the first such).
                //
                // **Do not turn this refusal into a drop.** Measured, and
                // proved as `the_dropped_encoding_would_have_been_admitted`:
                // the flagship's lease-free remainder is not a lossy trace but
                // an *admitted* one — `AdmittedCausalOrder::declare` accepts it
                // as a causal order for a run in which two workers held
                // outstanding write authority the order never mentions. So
                // dropping is a move in the admitting direction, which
                // invariant 1 forbids; refusing only blocks a comparison.
                //
                // The forward question `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`
                // asked whether the observable alphabet should grow to carry
                // leases once `WRITE-LEND-TASKS-EXERCISE` made a worker
                // exercise its authority. That card landed, and the answer is
                // **no** — decided, not deferred, on a ground the exercise
                // itself supplies. F02's `AdmittedCausalOrder` is a *sequence*:
                // to admit a run it must state one causal order over its
                // events. Two workers holding disjoint write leases write
                // their bytes genuinely concurrently, with no causal edge
                // between them and no synchronization until their joins, so
                // any total order the alphabet could carry would assert an
                // ordering the run does not have. That is the same situation
                // constraint 15a answers with a *window-and-emitter* relation
                // rather than a causal one, and is the opposite of F02's
                // premise. Byte events therefore join the lease events on the
                // refusing side — so the alphabet did not grow and no existing
                // digest moved — and write-lend programs stay outside F02
                // relational comparison by construction, their native evidence
                // being sanitizer, allocation-count and the byte-identical
                // stdout of a strict comparison whose windows are the joins.
                TaskEvent::BufferByteWritten { .. } | TaskEvent::BufferByteRead { .. } => {
                    return Err(ReferenceTraceError::UnsupportedObservedEvent { index });
                }
                TaskEvent::BufferWritten {
                    task,
                    buffer,
                    value,
                } => ObservedEvent::Written {
                    task: *task,
                    buffer: *buffer,
                    value: *value,
                },
                // Card `W48-QR-SCALAR-TYPES-R6`: the observable alphabet's
                // payloads are `i32` (`ObservedEvent::Allocated`, `Read`,
                // `Written`), so a typed payload event is refused rather than
                // encoded — never narrowed to an `i32` or dropped. Such a
                // program sits outside F02 relational comparison, as a
                // fixed-width worker outcome does, and its native evidence is
                // the byte comparison of the maintained trace.
                TaskEvent::TypedBufferAllocated { .. }
                | TaskEvent::TypedBufferRead { .. }
                | TaskEvent::TypedBufferWritten { .. } => {
                    return Err(ReferenceTraceError::UnsupportedObservedEvent { index });
                }
                TaskEvent::BufferLeased { .. } | TaskEvent::BufferLeaseReturned { .. } => {
                    return Err(ReferenceTraceError::UnsupportedObservedEvent { index });
                }
            })
        })
        .collect()
}

/// One reference linearization together with the causal constraints it
/// witnesses. Constructible only through validation, so an unchecked sequence
/// cannot become a constraint set that admits anything.
///
/// ```compile_fail
/// use qcore_core::task_trace_relation::{AdmittedCausalOrder, ObservedEvent};
/// fn forge(events: Vec<ObservedEvent>) -> AdmittedCausalOrder {
///     AdmittedCausalOrder { events }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedCausalOrder {
    events: Vec<ObservedEvent>,
}

/// The witness `relate` returns: for each observed position, the position the
/// same fact occupies in the reference linearization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceLinearization {
    pub reference_positions: Vec<usize>,
}

impl TraceLinearization {
    /// Whether the observed trace is literally the reference order. The point
    /// of this card is that `relate` succeeds without this being true.
    pub fn is_reference_order(&self) -> bool {
        self.reference_positions
            .iter()
            .enumerate()
            .all(|(index, position)| index == *position)
    }
}

fn attributable(events: &[ObservedEvent]) -> Result<(), TraceRelationViolation> {
    match events.iter().position(|event| event.actor().is_none()) {
        Some(index) => Err(TraceRelationViolation::UnattributableEvent { index }),
        None => Ok(()),
    }
}

fn distinct(events: &[ObservedEvent]) -> Result<(), TraceRelationViolation> {
    let mut seen = BTreeSet::new();
    for (index, event) in events.iter().enumerate() {
        if !seen.insert(event.clone()) {
            return Err(TraceRelationViolation::AmbiguousOccurrence { index });
        }
    }
    Ok(())
}

fn actors(events: &[ObservedEvent]) -> BTreeSet<TaskId> {
    events.iter().filter_map(ObservedEvent::actor).collect()
}

fn projection(events: &[ObservedEvent], actor: TaskId) -> Vec<&ObservedEvent> {
    events
        .iter()
        .filter(|event| event.actor() == Some(actor))
        .collect()
}

/// R4–R9: the order constraints a single trace must satisfy on its own.
fn check_order(events: &[ObservedEvent]) -> Result<(), TraceRelationViolation> {
    use TraceRelationViolation as V;
    let buffers: BTreeSet<ResourceId> = events.iter().filter_map(ObservedEvent::buffer).collect();
    let children: BTreeSet<TaskId> = events.iter().filter_map(ObservedEvent::subject).collect();
    let single = |predicate: &dyn Fn(&ObservedEvent) -> bool| -> Vec<usize> {
        events
            .iter()
            .enumerate()
            .filter(|(_, event)| predicate(event))
            .map(|(index, _)| index)
            .collect()
    };

    // R4: allocation is unique and first.
    let mut allocated: BTreeMap<ResourceId, (usize, i32)> = BTreeMap::new();
    for &buffer in &buffers {
        let sites = single(
            &|event| matches!(event, ObservedEvent::Allocated { buffer: b, .. } if *b == buffer),
        );
        if sites.len() > 1 {
            return Err(V::DuplicateAllocation { buffer });
        }
        if let Some(&index) = sites.first() {
            let ObservedEvent::Allocated { payload, .. } = events[index] else {
                unreachable!("selected an allocation")
            };
            allocated.insert(buffer, (index, payload));
        }
    }
    for &buffer in &buffers {
        if !allocated.contains_key(&buffer) {
            return Err(V::UnallocatedUse { buffer });
        }
    }
    for (&buffer, &(allocation, _)) in &allocated {
        let mentions = single(&|event| event.buffer() == Some(buffer));
        if mentions.iter().any(|&index| index < allocation) {
            return Err(V::AllocationOrder { buffer });
        }
    }
    // The payload rule is per generation and split on whether the generation
    // is ever written, so that a trace without writes is checked by exactly
    // the pre-extension rule (`QCore.TaskTraceWrites` proves the split
    // conservative): a never-written generation's reads must all report the
    // allocated payload, and a written generation's reads must each report
    // the value established by the nearest preceding allocation or write.
    for (&buffer, &(_, payload)) in &allocated {
        let written = events
            .iter()
            .any(|event| matches!(event, ObservedEvent::Written { buffer: b, .. } if *b == buffer));
        if written {
            let mut established: Option<i32> = None;
            for event in events {
                match event {
                    ObservedEvent::Allocated {
                        buffer: b, payload, ..
                    } if *b == buffer => established = Some(*payload),
                    ObservedEvent::Written {
                        buffer: b, value, ..
                    } if *b == buffer => established = Some(*value),
                    ObservedEvent::Read {
                        buffer: b, value, ..
                    } if *b == buffer && established != Some(*value) => {
                        return Err(V::PayloadMismatch { buffer });
                    }
                    _ => {}
                }
            }
        } else if events.iter().any(
            |event| matches!(event, ObservedEvent::Read { buffer: b, value, .. } if *b == buffer && *value != payload),
        ) {
            return Err(V::PayloadMismatch { buffer });
        }
    }

    // R5: release is unique, last, and mandatory.
    let mut released: BTreeMap<ResourceId, usize> = BTreeMap::new();
    for &buffer in &buffers {
        let sites = single(
            &|event| matches!(event, ObservedEvent::Dropped { buffer: b, .. } if *b == buffer),
        );
        if sites.len() > 1 {
            return Err(V::DuplicateRelease { buffer });
        }
        if let Some(&index) = sites.first() {
            released.insert(buffer, index);
        }
    }
    for (&buffer, &release) in &released {
        let mentions = single(&|event| event.buffer() == Some(buffer));
        if mentions.iter().any(|&index| index > release) {
            return Err(V::ReleaseOrder { buffer });
        }
    }
    for &buffer in allocated.keys() {
        if !released.contains_key(&buffer) {
            return Err(V::UnreleasedBuffer { buffer });
        }
    }

    // R6: creation is unique and precedes everything about or by the child.
    let mut created: BTreeMap<TaskId, usize> = BTreeMap::new();
    for &child in &children {
        let sites = single(
            &|event| matches!(event, ObservedEvent::Spawned { child: c, .. } if *c == child),
        );
        if sites.len() > 1 {
            return Err(V::DuplicateCreation { child });
        }
        if let Some(&index) = sites.first() {
            created.insert(child, index);
        }
    }
    for &child in &children {
        if !created.contains_key(&child) {
            return Err(V::UnknownChild { child });
        }
    }
    for (&child, &creation) in &created {
        let about = single(&|event| event.subject() == Some(child));
        if about.iter().any(|&index| index < creation) {
            return Err(V::CreationOrder { child });
        }
    }

    // R7: completion precedes join; a joined child is finished.
    let mut joined: BTreeMap<TaskId, usize> = BTreeMap::new();
    for &child in &children {
        let sites =
            single(&|event| matches!(event, ObservedEvent::Joined { child: c } if *c == child));
        if sites.len() > 1 {
            return Err(V::DuplicateJoin { child });
        }
        if let Some(&index) = sites.first() {
            joined.insert(child, index);
        }
    }
    for &child in created.keys() {
        if !joined.contains_key(&child) {
            return Err(V::UnjoinedChild { child });
        }
    }
    for (&child, &join) in &joined {
        let completions = single(
            &|event| matches!(event, ObservedEvent::Completed { task, .. } if *task == child),
        );
        if completions.len() != 1 || completions[0] > join {
            return Err(V::CompletionOrder { child });
        }
    }
    for (&child, &join) in &joined {
        // A loan return is the lender's action *after* the join, so it is the
        // one fact about a child that legitimately follows its join.
        let after = single(&|event| {
            event.subject() == Some(child) && !matches!(event, ObservedEvent::Returned { .. })
        });
        if after.iter().any(|&index| index > join) {
            return Err(V::ActivityAfterJoin { child });
        }
    }

    // R8: a loan is opened before it is read and closed only by the join.
    for (index, event) in events.iter().enumerate() {
        let ObservedEvent::Loaned { borrower, buffer } = event else {
            continue;
        };
        let reads = single(
            &|other| matches!(other, ObservedEvent::Read { task, buffer: b, .. } if *task == *borrower && *b == *buffer),
        );
        if reads.iter().any(|&read| read < index) {
            return Err(V::LoanOrder {
                borrower: *borrower,
                buffer: *buffer,
            });
        }
        if let Some(&join) = joined.get(borrower) {
            let returns = single(
                &|other| matches!(other, ObservedEvent::Returned { borrower: b, buffer: g } if *b == *borrower && *g == *buffer),
            );
            if returns.len() != 1 || returns[0] < join {
                return Err(V::LoanOrder {
                    borrower: *borrower,
                    buffer: *buffer,
                });
            }
        }
    }

    // R9: the root's completion is the final fact.
    let roots = single(
        &|event| matches!(event, ObservedEvent::Completed { task, .. } if *task == ROOT_TASK),
    );
    if roots.len() != 1 || roots[0] + 1 != events.len() {
        return Err(V::RootResultNotFinal);
    }
    Ok(())
}

impl AdmittedCausalOrder {
    /// Declare a reference linearization. Every conjunct the relation will
    /// later demand of an observation is first demanded of the reference, so a
    /// malformed reference cannot become a vacuous constraint set.
    pub fn declare(events: Vec<ObservedEvent>) -> Result<Self, TraceRelationViolation> {
        attributable(&events)?;
        distinct(&events)?;
        check_order(&events)?;
        Ok(Self { events })
    }

    /// Derive the admitted order from a reference run, re-checking P3 against
    /// the run's own task table rather than assuming it.
    pub fn derive(observation: &TaskExecutionObservation) -> Result<Self, TraceDeriveError> {
        for task in &observation.tasks {
            if task.parent.is_some_and(|parent| parent != ROOT_TASK) {
                return Err(TraceDeriveError::Violation(
                    TraceRelationViolation::UnknownChild { child: task.task },
                ));
            }
        }
        let events = reference_trace(observation).map_err(TraceDeriveError::Unsupported)?;
        Self::declare(events).map_err(TraceDeriveError::Violation)
    }

    pub fn events(&self) -> &[ObservedEvent] {
        &self.events
    }

    /// The admitted causal edges this order witnesses, as index pairs into
    /// [`Self::events`]. Reported for evidence and for counting how much order
    /// the relation actually demands; [`relate`] re-derives every constraint
    /// from the observation instead of replaying this list.
    pub fn edges(&self) -> Vec<(usize, usize)> {
        let mut edges = BTreeSet::new();
        let index_of = |predicate: &dyn Fn(&ObservedEvent) -> bool| -> Option<usize> {
            self.events.iter().position(predicate)
        };
        // P1: each actor's own events are program-ordered.
        for actor in actors(&self.events) {
            let indices: Vec<usize> = self
                .events
                .iter()
                .enumerate()
                .filter(|(_, event)| event.actor() == Some(actor))
                .map(|(index, _)| index)
                .collect();
            for pair in indices.windows(2) {
                edges.insert((pair[0], pair[1]));
            }
        }
        for (index, event) in self.events.iter().enumerate() {
            if let Some(buffer) = event.buffer() {
                if let Some(allocation) = index_of(
                    &|other| matches!(other, ObservedEvent::Allocated { buffer: b, .. } if *b == buffer),
                ) {
                    edges.insert((allocation, index));
                }
                if let Some(release) = index_of(
                    &|other| matches!(other, ObservedEvent::Dropped { buffer: b, .. } if *b == buffer),
                ) {
                    edges.insert((index, release));
                }
            }
            if let Some(child) = event.subject() {
                if let Some(creation) = index_of(
                    &|other| matches!(other, ObservedEvent::Spawned { child: c, .. } if *c == child),
                ) {
                    edges.insert((creation, index));
                }
                if !matches!(event, ObservedEvent::Returned { .. }) {
                    if let Some(join) = index_of(
                        &|other| matches!(other, ObservedEvent::Joined { child: c } if *c == child),
                    ) {
                        edges.insert((index, join));
                    }
                } else if let Some(join) = index_of(
                    &|other| matches!(other, ObservedEvent::Joined { child: c } if *c == child),
                ) {
                    edges.insert((join, index));
                }
            }
            if let ObservedEvent::Loaned { borrower, buffer } = event {
                for (other, candidate) in self.events.iter().enumerate() {
                    if matches!(candidate, ObservedEvent::Read { task, buffer: b, .. } if task == borrower && b == buffer)
                    {
                        edges.insert((index, other));
                    }
                }
            }
            // R9: the root's completion is the final fact.
            if let Some(root) = index_of(
                &|other| matches!(other, ObservedEvent::Completed { task, .. } if *task == ROOT_TASK),
            ) {
                edges.insert((index, root));
            }
        }
        edges.retain(|&(from, to)| from != to);
        edges.into_iter().collect()
    }
}

/// Relate an independently observed trace to an admitted causal order.
///
/// Success means: the observation reports exactly the same facts, each actor's
/// own events are in its program order, and every admitted causal constraint
/// holds. It does **not** mean the observation is the reference order, and it
/// does not mean the underlying native execution took the order it rendered.
pub fn relate(
    order: &AdmittedCausalOrder,
    observed: &[ObservedEvent],
) -> Result<TraceLinearization, TraceRelationViolation> {
    attributable(observed)?;
    distinct(observed)?;
    let reference: BTreeMap<&ObservedEvent, usize> = order
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (event, index))
        .collect();
    if observed.len() != order.events.len()
        || observed.iter().any(|event| !reference.contains_key(event))
    {
        return Err(TraceRelationViolation::EventMultisetMismatch);
    }
    for actor in actors(&order.events) {
        if projection(observed, actor) != projection(&order.events, actor) {
            return Err(TraceRelationViolation::ActorOrder { actor });
        }
    }
    check_order(observed)?;
    Ok(TraceLinearization {
        reference_positions: observed.iter().map(|event| reference[event]).collect(),
    })
}

/// Every total order of the observed facts that the admitted constraints
/// accept. Bounded on purpose: the caller supplies the bound and gets `None`
/// when the search would exceed it, rather than an answer of unknown cost.
pub fn admitted_linearizations(
    order: &AdmittedCausalOrder,
    bound: usize,
) -> Option<Vec<Vec<ObservedEvent>>> {
    let events = order.events.clone();
    let mut admitted = Vec::new();
    let mut prefix: Vec<ObservedEvent> = Vec::new();
    let mut used = vec![false; events.len()];
    let mut explored = 0usize;
    fn search(
        order: &AdmittedCausalOrder,
        events: &[ObservedEvent],
        used: &mut Vec<bool>,
        prefix: &mut Vec<ObservedEvent>,
        admitted: &mut Vec<Vec<ObservedEvent>>,
        explored: &mut usize,
        bound: usize,
    ) -> bool {
        if prefix.len() == events.len() {
            if relate(order, prefix).is_ok() {
                admitted.push(prefix.clone());
            }
            return true;
        }
        for index in 0..events.len() {
            if used[index] {
                continue;
            }
            *explored += 1;
            if *explored > bound {
                return false;
            }
            used[index] = true;
            prefix.push(events[index].clone());
            // Prefix pruning: an actor's own order is a prefix-closed
            // constraint, so a prefix that already breaks it cannot be extended
            // into an admitted trace.
            let feasible = actors(events).into_iter().all(|actor| {
                let taken = projection(prefix, actor);
                let expected = projection(events, actor);
                taken.len() <= expected.len() && taken[..] == expected[..taken.len()]
            });
            if feasible && !search(order, events, used, prefix, admitted, explored, bound) {
                return false;
            }
            prefix.pop();
            used[index] = false;
        }
        true
    }
    search(
        order,
        &events,
        &mut used,
        &mut prefix,
        &mut admitted,
        &mut explored,
        bound,
    )
    .then_some(admitted)
}
