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

//! Native C11 backend and reference execution for single-task
//! `CheckedMutexInvariantV1` traces (the `source.mutex.guard.v1` route).
//!
//! Scope, stated before anything else. This backend emits **one** thread that
//! creates real `mtx_t` generations, acquires and releases them around the
//! trace's critical sections, and reads and writes real heap bytes through the
//! declared footprints. It is therefore evidence about acquisition/release
//! pairing, footprint extents, and allocation lifetime on one Linux target and
//! one compiler invocation. It is **not** evidence about contention, about any
//! multi-threaded schedule, about the C memory model, or about the invariant
//! predicate — a trace carrying task structure is outside this backend
//! entirely ([`is_single_task_trace`]), and the predicate stays the
//! `InvariantPredicateDeclared` premise on every section that closes by
//! declaration.
//!
//! [`interpret_mutex_program`] computes the same observation from the checked
//! event list. Agreement between the two is a translation check for the
//! selected model and inputs, in the sense of `docs/QCORE_SPEC_r0_3.md` §L15;
//! it is not a general source refinement.
//!
//! # Protection growth (card `CT34-SANITIZED`)
//!
//! A `CheckedMutexInvariantV1` footprint is a **declared shape**. This backend
//! protects by discipline — the checker decided which accesses may run under
//! which guard — and an `mtx_t` neither knows nor enforces an extent. So a
//! [`MutexInvariantEvent::GrowProtection`] renders as exactly what it is at C
//! level: **no operation on the `mtx_t` at all**, plus the one bookkeeping line
//! that keeps the native and reference observations comparable. Nothing here
//! claims that the C mutex began protecting more memory; what the sanitized run
//! shows is that the accesses the checker admitted after the growth are real,
//! in-bounds accesses to real allocations, performed between the same
//! `mtx_lock`/`mtx_unlock` pair, and that both renderings agree line for line.
//!
//! The backend renders a growth **only** where the supplied certificate admits
//! it: an [`AdmittedProtectionGrowth`] naming this event index, this task, this
//! guard, this generation, this interval and this new predicate identity, whose
//! interval the certificate's own effective
//! [`MutexInvariantCertificate::protected_shape`] carries. A hand-built or
//! edited trace whose growth no certificate admits is refused at
//! [`MutexEmissionRefusal::UnadmittedProtectionGrowth`]: the backend trusts only
//! checked modules, and there is no path from an unchecked growth to emitted C.
use std::collections::BTreeMap;
use std::fmt::Write;

use crate::c_program::{self, Fragment, Section};
use crate::mutex_invariant::{
    AccessFootprint, AccessKind, AdmittedProtectionGrowth, ConcurrencyProfile, GuardId,
    InvariantId, MutexId, MutexInvariantCertificate, MutexInvariantEvent, MutexInvariantProgram,
    SharedResourceId, TaskId,
};

/// One line of the execution observation. The reference interpreter and the
/// emitted C must produce byte-identical sequences.
pub type MutexObservation = Vec<String>;

/// Why this backend refused to emit. Refusal is the only alternative to
/// emission; there is no partial or approximate lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutexEmissionRefusal {
    /// The certificate was not issued by `CheckedMutexInvariantV1`.
    UncheckedProfile,
    /// An event outside the single-task subset this backend implements.
    UnsupportedEvent { event: usize },
    /// A footprint whose extent no initial grant declares.
    UndeclaredResource { resource: SharedResourceId },
    /// A [`MutexInvariantEvent::GrowProtection`] the supplied certificate does
    /// not admit: no [`AdmittedProtectionGrowth`] at this event index, one that
    /// disagrees with the event's own task, guard, generation, interval or new
    /// predicate identity, or one the certificate's effective
    /// [`MutexInvariantCertificate::protected_shape`] does not carry. The
    /// backend renders growths a checker admitted and nothing else.
    UnadmittedProtectionGrowth { event: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexCEmission {
    pub source: String,
    /// Byte length of each emitted resource allocation.
    pub resource_bytes: BTreeMap<SharedResourceId, u32>,
}

/// Whether this backend implements the trace at all.
///
/// The backend emits **one** thread. A trace carrying task structure — task
/// creation, a join, a custody transfer between tasks, or an announced
/// acquisition that only another thread could satisfy — is outside it. Callers
/// use this to distinguish "checked, but this backend renders no native
/// program" from "the backend refused a trace it was supposed to render";
/// [`emit_mutex_c11`] still refuses such a trace if it is handed one.
pub fn is_single_task_trace(program: &MutexInvariantProgram) -> bool {
    !program.events.iter().any(|event| {
        matches!(
            event,
            MutexInvariantEvent::Spawn { .. }
                | MutexInvariantEvent::Join { .. }
                // A guard-local join is still a join: the backend emits one
                // thread, so a trace carrying one is outside it.
                | MutexInvariantEvent::GuardLocalJoin { .. }
                | MutexInvariantEvent::TransferCustody { .. }
                | MutexInvariantEvent::RegisterWaiter { .. }
        )
    })
}

/// One child of an admitted fork/join trace: the `Spawn` that creates it, the
/// `Join` that reclaims it, and the half-open event range between them that is
/// this child's entire contribution to the trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildSegment {
    pub parent: TaskId,
    pub child: TaskId,
    /// Index of the `Spawn`.
    pub spawn: usize,
    /// Index of the matching `Join`.
    pub join: usize,
}

/// How this backend renders a checked trace's task structure.
///
/// Card `MUTEX-SHARE-CROSS-EMISSION`. Classification is total and fail-closed:
/// a trace that is neither of the two rendered shapes is
/// [`TraceShape::Outside`] and carries the index of the first event that put it
/// there, which is the index the refusal reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceShape {
    /// No task structure at all. The wave-`T03` single-thread rendering, byte
    /// for byte what it has always been.
    SingleTask,
    /// Non-nested `Spawn`/`Join` pairs whose children's events are *contiguous*
    /// between them, every other event belonging to the one root task. This is
    /// what `source.mutex.guard.v1`'s declared crossing lowers to.
    ForkJoin { children: Vec<ChildSegment> },
    /// Outside this backend entirely.
    Outside { event: usize },
}

/// Where a rendered object's *declaration* lives.
///
/// The single-task rendering declares its `mtx_t` and its resource pointers as
/// automatic objects of `main`, exactly as it always did. A fork/join rendering
/// cannot: the child runs in another function, so the same objects are declared
/// at file scope and only their initialisation stays in `main`. Nothing else
/// about a rendered event differs between the two, which is what keeps every
/// previously emitted program byte-identical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement {
    Local,
    FileScope,
}

/// The task that performs `event`, for the events this backend renders. `None`
/// for the task-structure events, whose acting task is ambiguous by
/// construction (a `Spawn` is performed by the parent *and* creates the child).
fn event_task(event: &MutexInvariantEvent) -> Option<TaskId> {
    match event {
        MutexInvariantEvent::CreateMutex { task, .. }
        | MutexInvariantEvent::DestroyMutex { task, .. }
        | MutexInvariantEvent::Lock { task, .. }
        | MutexInvariantEvent::Unlock { task, .. }
        | MutexInvariantEvent::ErrorExit { task, .. }
        | MutexInvariantEvent::OpenInvariant { task, .. }
        | MutexInvariantEvent::CloseInvariant { task, .. }
        | MutexInvariantEvent::Access { task, .. }
        | MutexInvariantEvent::BorrowUnderGuard { task, .. }
        | MutexInvariantEvent::EndBorrow { task, .. }
        | MutexInvariantEvent::AssertFact { task, .. }
        | MutexInvariantEvent::UseFact { task, .. }
        | MutexInvariantEvent::GrowProtection { task, .. }
        | MutexInvariantEvent::Destroy { task, .. } => Some(*task),
        _ => None,
    }
}

/// Classify a checked trace's task structure.
///
/// The admitted multi-task shape is deliberately narrow, and every bound below
/// is a refusal rather than an approximation:
///
/// * **No nesting.** A `Spawn` inside an open child segment is outside
///   (residual `MUTEX-SHARE-CROSS-MULTI` owns that question).
/// * **Contiguity.** Every event between a `Spawn` and its matching `Join`
///   belongs to that child. This is what makes the emitted rendering — parent
///   creates the thread, then blocks in `thrd_join` — realise the *checked*
///   order rather than some other interleaving of it.
/// * **One root.** Every event outside a child segment belongs to the task the
///   first `Spawn` names as parent.
/// * **No typed-error exit in a child.** The model gives a child's exit status
///   no meaning at the join, so rendering one would invent semantics
///   (`MUTEX-SHARE-CROSS-EMISSION-CHILD-ERROR-EXIT`).
/// * **No `GuardLocalJoin`, `TransferCustody` or `RegisterWaiter`**, anywhere —
///   the pre-existing exclusions of [`is_single_task_trace`], unmoved.
#[must_use]
pub fn classify_trace(program: &MutexInvariantProgram) -> TraceShape {
    if is_single_task_trace(program) {
        return TraceShape::SingleTask;
    }
    let events = &program.events;
    let mut children = Vec::new();
    let mut root: Option<TaskId> = None;
    let mut index = 0;
    while index < events.len() {
        match &events[index] {
            MutexInvariantEvent::Spawn { parent, child } => {
                if *root.get_or_insert(*parent) != *parent {
                    return TraceShape::Outside { event: index };
                }
                let mut cursor = index + 1;
                let end = loop {
                    // A malformed segment — unmatched, nested, interleaved with
                    // its parent's events, or carrying a typed-error exit — is
                    // reported at its **`Spawn`**, which is the event whose
                    // rendering the backend cannot construct. Reporting the
                    // offending member instead would name an event that is
                    // perfectly renderable on its own.
                    let Some(event) = events.get(cursor) else {
                        return TraceShape::Outside { event: index };
                    };
                    match event {
                        MutexInvariantEvent::Join {
                            parent: joined_parent,
                            child: joined_child,
                        } if joined_parent == parent && joined_child == child => break cursor,
                        // A nested spawn, a mismatched join, a guard-local
                        // join, a custody transfer, a registered waiter, or a
                        // typed-error exit inside the child.
                        MutexInvariantEvent::Spawn { .. }
                        | MutexInvariantEvent::Join { .. }
                        | MutexInvariantEvent::ErrorExit { .. } => {
                            return TraceShape::Outside { event: index };
                        }
                        other => {
                            if event_task(other) != Some(*child) {
                                return TraceShape::Outside { event: index };
                            }
                        }
                    }
                    cursor += 1;
                };
                children.push(ChildSegment {
                    parent: *parent,
                    child: *child,
                    spawn: index,
                    join: end,
                });
                index = end + 1;
            }
            MutexInvariantEvent::Join { .. } => return TraceShape::Outside { event: index },
            other => {
                match event_task(other) {
                    Some(task) => {
                        if *root.get_or_insert(task) != task {
                            return TraceShape::Outside { event: index };
                        }
                    }
                    // A task-structure event this shape does not admit at all:
                    // `GuardLocalJoin`, `TransferCustody`, `RegisterWaiter`.
                    None => return TraceShape::Outside { event: index },
                }
                index += 1;
            }
        }
    }
    TraceShape::ForkJoin { children }
}

/// Byte written by the access at `index`. Deterministic and identical in both
/// executions, so a mismatch is a real disagreement rather than noise.
const fn fill_byte(index: usize) -> u32 {
    (index as u32) % 251 + 1
}

fn resource_extents(
    program: &MutexInvariantProgram,
) -> Result<BTreeMap<SharedResourceId, u32>, MutexEmissionRefusal> {
    let mut extents = BTreeMap::<SharedResourceId, u32>::new();
    for grant in &program.initial_shares {
        let entry = extents.entry(grant.footprint.resource).or_default();
        *entry = (*entry).max(grant.footprint.end);
    }
    let check = |footprint: AccessFootprint| match extents.get(&footprint.resource) {
        Some(end) if footprint.end <= *end => Ok(()),
        _ => Err(MutexEmissionRefusal::UndeclaredResource {
            resource: footprint.resource,
        }),
    };
    for event in &program.events {
        match event {
            MutexInvariantEvent::CreateMutex { protects, .. } => {
                for footprint in protects {
                    check(*footprint)?;
                }
            }
            MutexInvariantEvent::Access { footprint, .. }
            | MutexInvariantEvent::BorrowUnderGuard { footprint, .. }
            // A grown interval is memory this backend allocates, so it is
            // subject to exactly the extent rule every other footprint is.
            | MutexInvariantEvent::GrowProtection { footprint, .. } => check(*footprint)?,
            _ => {}
        }
    }
    Ok(extents)
}

/// The bookkeeping line a rendered growth contributes, identical in both
/// executions. A growth performs no `mtx_t` operation, so this line is the whole
/// of its runtime rendering.
fn growth_line(mutex: MutexId, guard: GuardId, footprint: AccessFootprint) -> String {
    format!(
        "grow {mutex} {guard} {} {} {}",
        footprint.resource, footprint.start, footprint.end
    )
}

/// Decide whether the supplied certificate admits the growth at `event`, and
/// return the generation it grew.
///
/// Every field of the event is compared against the certificate's own record,
/// the generation is taken from the guard the trace really minted (never from
/// the record, which would let a record name a generation the trace never
/// locked), and the certificate's effective protected shape must carry the
/// grown interval. Any disagreement is a refusal: this backend renders checked
/// modules, so a growth no certificate admits has no rendering at all.
fn admitted_growth(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    event: usize,
    task: TaskId,
    guard: GuardId,
    footprint: AccessFootprint,
    invariant: InvariantId,
) -> Result<MutexId, MutexEmissionRefusal> {
    let refusal = MutexEmissionRefusal::UnadmittedProtectionGrowth { event };
    // A guard the trace never minted is refused here rather than guessed, in
    // exactly the way `Unlock` refuses one.
    let mutex = holding_mutex(program, event, guard).map_err(|_| refusal.clone())?;
    let matches = |growth: &&AdmittedProtectionGrowth| {
        growth.event == event
            && growth.task == task
            && growth.guard == guard
            && growth.mutex == mutex
            && growth.footprint == footprint
            && growth.to_invariant == invariant
    };
    if !certificate
        .admitted_protection_growths
        .iter()
        .any(|growth| matches(&growth))
    {
        return Err(refusal);
    }
    // The certificate's *effective* shape is a second, independent binding: a
    // record that admitted the growth while the published shape denied it would
    // be a certificate disagreeing with itself, and is refused rather than
    // rendered.
    match certificate.protected_shape.get(&mutex) {
        Some(shape) if shape.contains(&footprint) => Ok(mutex),
        _ => Err(refusal),
    }
}

/// The payload type of each typed protected resource (card
/// `W48-QR-SCALAR-TYPES-R6`, `protected TYPE`). A resource absent from it is a
/// `protected N` byte region, exactly as before the card.
pub type ProtectedPayloads = BTreeMap<SharedResourceId, crate::scalar_types::ScalarType>;

/// The byte a write stores at `index` into a cell of `resource`. A `bool`
/// payload's one byte is `0`/`1` — the low bit of the fill — so after every
/// write every typed protected payload holds a value of its type (card
/// `W48-QR-SCALAR-TYPES-R6`); every other cell takes the historical fill.
fn write_fill(index: usize, resource: SharedResourceId, payloads: &ProtectedPayloads) -> u32 {
    if payloads.get(&resource) == Some(&crate::scalar_types::ScalarType::Bool) {
        fill_byte(index) & 1
    } else {
        fill_byte(index)
    }
}

/// Reference execution of a checked trace. Mirrors the emitted C statement for
/// statement; neither one is derived from the other's output.
pub fn interpret_mutex_program(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
) -> Result<MutexObservation, MutexEmissionRefusal> {
    interpret_typed_mutex_program(program, certificate, &ProtectedPayloads::new())
}

/// [`interpret_mutex_program`] with the typed protected payloads of the
/// source (card `W48-QR-SCALAR-TYPES-R6`). With none it is that function.
pub fn interpret_typed_mutex_program(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    payloads: &ProtectedPayloads,
) -> Result<MutexObservation, MutexEmissionRefusal> {
    if certificate.profile != ConcurrencyProfile::CheckedMutexInvariantV1 {
        return Err(MutexEmissionRefusal::UncheckedProfile);
    }
    let extents = resource_extents(program)?;
    if let TraceShape::Outside { event } = classify_trace(program) {
        return Err(MutexEmissionRefusal::UnsupportedEvent { event });
    }
    let mut bytes: BTreeMap<SharedResourceId, Vec<u8>> = extents
        .iter()
        .map(|(resource, end)| (*resource, vec![0_u8; *end as usize]))
        .collect();
    let mut trace = MutexObservation::new();
    for (resource, end) in &extents {
        trace.push(format!("resource {resource} {end}"));
    }
    for (index, event) in program.events.iter().enumerate() {
        match event {
            // Card `MUTEX-SHARE-CROSS-EMISSION`. The classification above has
            // already established that this pair is non-nested and that the
            // child's events are exactly those between them, so the reference
            // run needs no scheduler: the checked order *is* a total order, and
            // the emitted program realises it by blocking the parent in
            // `thrd_join` for the whole of the child's segment.
            MutexInvariantEvent::Spawn { parent, child } => {
                trace.push(format!("spawn {parent} {child}"));
            }
            MutexInvariantEvent::Join { parent, child } => {
                trace.push(format!("join {parent} {child}"));
            }
            MutexInvariantEvent::CreateMutex { mutex, .. } => {
                trace.push(format!("create {mutex}"));
            }
            MutexInvariantEvent::Lock { mutex, guard, .. } => {
                trace.push(format!("lock {mutex} {guard}"));
            }
            MutexInvariantEvent::Unlock { guard, .. } => {
                trace.push(format!("unlock {guard}"));
            }
            MutexInvariantEvent::ErrorExit { guard, exit, .. } => {
                trace.push(format!("error-exit {guard} {exit}"));
            }
            MutexInvariantEvent::OpenInvariant { guard, .. } => {
                trace.push(format!("open {guard}"));
            }
            MutexInvariantEvent::CloseInvariant { guard, .. } => {
                trace.push(format!("close {guard}"));
            }
            MutexInvariantEvent::Access {
                footprint, kind, ..
            } => {
                let cells = &mut bytes.get_mut(&footprint.resource).expect("declared")
                    [footprint.start as usize..footprint.end as usize];
                match kind {
                    AccessKind::Write => {
                        for cell in cells.iter_mut() {
                            *cell = write_fill(index, footprint.resource, payloads) as u8;
                        }
                        trace.push(format!(
                            "write {} {} {}",
                            footprint.resource, footprint.start, footprint.end
                        ));
                    }
                    AccessKind::Read => {
                        let sum: u32 = cells.iter().map(|byte| u32::from(*byte)).sum();
                        trace.push(format!(
                            "read {} {} {} {}",
                            footprint.resource,
                            footprint.start,
                            footprint.end,
                            sum % 65536
                        ));
                    }
                    _ => return Err(MutexEmissionRefusal::UnsupportedEvent { event: index }),
                }
            }
            // Guard-scoped refinement facts have no runtime representation, in
            // exactly the sense the invariant phases do not: the core decided
            // their extent statically. They are traced so the two observations
            // stay comparable.
            MutexInvariantEvent::AssertFact { fact, .. } => {
                trace.push(format!("assert {fact}"));
            }
            MutexInvariantEvent::UseFact { fact, .. } => {
                trace.push(format!("use-fact {fact}"));
            }
            MutexInvariantEvent::BorrowUnderGuard { borrow, .. } => {
                trace.push(format!("borrow {borrow}"));
            }
            MutexInvariantEvent::EndBorrow { borrow, .. } => {
                trace.push(format!("end-borrow {borrow}"));
            }
            // A growth changes a *declared* shape. There is no `mtx_t`
            // operation to mirror, so the reference execution records the same
            // bookkeeping line the emitted C prints and nothing else.
            MutexInvariantEvent::GrowProtection {
                task,
                guard,
                footprint,
                invariant,
            } => {
                let mutex = admitted_growth(
                    program,
                    certificate,
                    index,
                    *task,
                    *guard,
                    *footprint,
                    *invariant,
                )?;
                trace.push(growth_line(mutex, *guard, *footprint));
            }
            MutexInvariantEvent::DestroyMutex { mutex, .. } => {
                trace.push(format!("destroy-mutex {mutex}"));
            }
            MutexInvariantEvent::Destroy { resource, .. } => {
                trace.push(format!("destroy {resource}"));
            }
            _ => return Err(MutexEmissionRefusal::UnsupportedEvent { event: index }),
        }
    }
    Ok(trace)
}

const INCLUDES: &str = "#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n";

const PRELUDE: &str = r#"

/* Generated by qcore_core::mutex_c_emit for a checked
   CheckedMutexInvariantV1 trace. One thread, real mtx_t generations, real
   heap footprints. Emitted only for a trace the core already checked. */
static void qcore_fail(const char *what) {
    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME,
        UINT64_C(0), UINT64_C(0), INT64_C(1), what);
    abort();
}
"#;

/// Emit C11 for a checked trace: one thread for a single-task trace, real
/// `thrd_t` children for an admitted fork/join trace.
pub fn emit_mutex_c11(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
) -> Result<MutexCEmission, MutexEmissionRefusal> {
    emit_typed_mutex_c11(program, certificate, &ProtectedPayloads::new())
}

/// [`emit_mutex_c11`] with the typed protected payloads of the source (card
/// `W48-QR-SCALAR-TYPES-R6`). With none it is that function, byte for byte;
/// a typed payload changes only what a write into a `bool` cell stores.
pub fn emit_typed_mutex_c11(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    payloads: &ProtectedPayloads,
) -> Result<MutexCEmission, MutexEmissionRefusal> {
    if certificate.profile != ConcurrencyProfile::CheckedMutexInvariantV1 {
        return Err(MutexEmissionRefusal::UncheckedProfile);
    }
    let extents = resource_extents(program)?;
    match classify_trace(program) {
        TraceShape::SingleTask => emit_single_task(program, certificate, extents, payloads),
        TraceShape::ForkJoin { children } => {
            emit_fork_join(program, certificate, extents, &children, payloads)
        }
        TraceShape::Outside { event } => Err(MutexEmissionRefusal::UnsupportedEvent { event }),
    }
}

/// The wave-`T03` rendering, unmoved: one thread, automatic `mtx_t` and
/// resource pointers, every byte of previously emitted output preserved.
fn emit_single_task(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    extents: BTreeMap<SharedResourceId, u32>,
    payloads: &ProtectedPayloads,
) -> Result<MutexCEmission, MutexEmissionRefusal> {
    let mut source = String::new();
    for (resource, end) in &extents {
        writeln!(
            source,
            "    unsigned char *r{resource} = malloc({end});\n    \
             if (r{resource} == NULL) qcore_fail(\"allocation\");\n    \
             memset(r{resource}, 0, {end});\n    \
             qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE, {resource}, {end}, INT64_C(0), (const char *)0);"
        )
        .unwrap();
    }
    for (index, event) in program.events.iter().enumerate() {
        render_event(
            &mut source,
            program,
            certificate,
            index,
            event,
            Placement::Local,
            payloads,
        )?;
    }
    for resource in extents.keys() {
        writeln!(source, "    free(r{resource});").unwrap();
    }
    Ok(MutexCEmission {
        source: c_program::assemble(
            &typed_preamble(payloads),
            &c_program::Entry {
                lead: String::new(),
                brace: c_program::BraceStyle::KAndR,
                // The exit status a typed-error exit set, carried across the
                // releases the epilogue performs.
                prologue: "    int qcore_exit = 0;\n".to_owned(),
                body: source,
                epilogue: "    return qcore_exit;\n}\n".to_owned(),
            },
        ),
        resource_bytes: extents,
    })
}

/// Render one checked event into `source`.
///
/// Shared verbatim by both placements: a rendered event's text depends on the
/// event and on nothing else, except the two *declaration* sites a file-scope
/// object cannot carry. That is the whole of the difference between the two
/// backends, and it is why an admitted single-task program emits exactly the
/// bytes it emitted before this card.
fn render_event(
    source: &mut String,
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    index: usize,
    event: &MutexInvariantEvent,
    placement: Placement,
    payloads: &ProtectedPayloads,
) -> Result<(), MutexEmissionRefusal> {
    match event {
        MutexInvariantEvent::CreateMutex { mutex, .. } => match placement {
            Placement::Local => writeln!(
                source,
                "    mtx_t m{mutex};\n    \
                 if (mtx_init(&m{mutex}, mtx_plain) != thrd_success) qcore_fail(\"mtx_init\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_MUTEX_CREATE, {mutex}, UINT64_C(0), INT64_C(0), (const char *)0);"
            )
            .unwrap(),
            // The generation is declared at file scope so the child function
            // can name it; the initialisation stays exactly where the checked
            // trace puts it.
            Placement::FileScope => writeln!(
                source,
                "    if (mtx_init(&m{mutex}, mtx_plain) != thrd_success) qcore_fail(\"mtx_init\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_MUTEX_CREATE, {mutex}, UINT64_C(0), INT64_C(0), (const char *)0);"
            )
            .unwrap(),
        },
            MutexInvariantEvent::Lock { mutex, guard, .. } => writeln!(
                source,
                "    if (mtx_lock(&m{mutex}) != thrd_success) qcore_fail(\"mtx_lock\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_MUTEX_LOCK, {mutex}, {guard}, INT64_C(0), (const char *)0);"
            )
            .unwrap(),
            MutexInvariantEvent::Unlock { guard, .. } => {
                let mutex = holding_mutex(program, index, *guard)?;
                writeln!(
                    source,
                    "    if (mtx_unlock(&m{mutex}) != thrd_success) qcore_fail(\"mtx_unlock\");\n    \
                     qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_MUTEX_UNLOCK, {guard}, UINT64_C(0), INT64_C(0), (const char *)0);"
                )
                .unwrap();
            }
            MutexInvariantEvent::ErrorExit { guard, exit, .. } => {
                let mutex = holding_mutex(program, index, *guard)?;
                // The typed-error exit releases the section before the raise
                // leaves it: the release is emitted, never skipped.
                writeln!(
                    source,
                    "    qcore_exit = {exit};\n    \
                     if (mtx_unlock(&m{mutex}) != thrd_success) qcore_fail(\"mtx_unlock\");\n    \
                     qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME, {guard}, UINT64_C(0), {exit}, \"error-exit\");"
                )
                .unwrap();
            }
            // The invariant phases have no runtime representation: they are
            // static bookkeeping the core already checked. They are traced so
            // the native and reference observations stay comparable.
            MutexInvariantEvent::OpenInvariant { guard, .. } => {
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_OPEN, {guard}, UINT64_C(0), INT64_C(0), (const char *)0);").unwrap();
            }
            MutexInvariantEvent::CloseInvariant { guard, .. } => {
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_CLOSE, {guard}, UINT64_C(0), INT64_C(0), (const char *)0);").unwrap();
            }
            MutexInvariantEvent::Access {
                footprint, kind, ..
            } => {
                let (resource, start, end) = (footprint.resource, footprint.start, footprint.end);
                match kind {
                    AccessKind::Write => writeln!(
                        source,
                        "    for (unsigned i = {start}u; i < {end}u; i++) \
                         r{resource}[i] = (unsigned char){};\n    \
                         qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_WRITE, {resource}, {start}, {end}, (const char *)0);",
                        write_fill(index, resource, payloads)
                    )
                    .unwrap(),
                    // The sum is computed by C; retain the declared endpoint
                    // as metadata so a typed sink can recover the full range.
                    AccessKind::Read => writeln!(
                        source,
                        "    {{ unsigned long sum = 0;\n      \
                         for (unsigned i = {start}u; i < {end}u; i++) sum += r{resource}[i];\n      \
                         qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_READ, {resource}, {start}, (int64_t)(sum % 65536UL), \"{end}\"); }}"
                    )
                    .unwrap(),
                    _ => return Err(MutexEmissionRefusal::UnsupportedEvent { event: index }),
                }
            }
            MutexInvariantEvent::AssertFact { fact, .. } => {
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_FACT, {fact}, UINT64_C(0), INT64_C(1), \"assert\");").unwrap();
            }
            MutexInvariantEvent::UseFact { fact, .. } => {
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_FACT, {fact}, UINT64_C(0), INT64_C(2), \"use\");").unwrap();
            }
            MutexInvariantEvent::BorrowUnderGuard {
                footprint, borrow, ..
            } => writeln!(
                source,
                "    unsigned char *b{borrow} = &r{resource}[{start}];\n    \
                 if (b{borrow} == NULL) qcore_fail(\"borrow\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_READ, {borrow}, {resource}, {start}, \"borrow\");",
                resource = footprint.resource,
                start = footprint.start
            )
            .unwrap(),
            MutexInvariantEvent::EndBorrow { borrow, .. } => writeln!(
                source,
                "    b{borrow} = NULL;\n    \
                 if (b{borrow} != NULL) qcore_fail(\"borrow\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_READ, {borrow}, UINT64_C(0), INT64_C(0), \"end-borrow\");"
            )
            .unwrap(),
            // Protection growth emits **no** `mtx_t` operation: the profile's
            // footprints are declared shapes and a C mutex has no extent to
            // widen. What the emitted program keeps is the bookkeeping line, so
            // the two observations stay comparable, and the ordinary accesses
            // the checker admitted afterwards — real, in-bounds reads and writes
            // of the grown allocation, inside this section's own lock/unlock
            // pair. No hardware or library protection of the grown region is
            // claimed here or anywhere else.
            MutexInvariantEvent::GrowProtection {
                task,
                guard,
                footprint,
                invariant,
            } => {
                let mutex = admitted_growth(
                    program,
                    certificate,
                    index,
                    *task,
                    *guard,
                    *footprint,
                    *invariant,
                )?;
                // Keep the guard and interval start as well as the resource
                // and endpoint: a sink must not lose part of the growth record.
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_FACT, {mutex}, {}, INT64_C({}), \"{}\");", footprint.resource, footprint.end, growth_line(mutex, *guard, *footprint))
                    .unwrap();
            }
            MutexInvariantEvent::DestroyMutex { mutex, .. } => writeln!(
                source,
                "    mtx_destroy(&m{mutex});\n    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_MUTEX_DESTROY, {mutex}, UINT64_C(0), INT64_C(0), (const char *)0);"
            )
            .unwrap(),
            MutexInvariantEvent::Destroy { resource, .. } => writeln!(
                source,
                "    free(r{resource});\n    r{resource} = NULL;\n    \
                 qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_RELEASE, {resource}, UINT64_C(0), INT64_C(0), (const char *)0);"
            )
            .unwrap(),
        _ => return Err(MutexEmissionRefusal::UnsupportedEvent { event: index }),
    }
    Ok(())
}

/// The name of the C function carrying child task `child`'s segment.
fn child_function(child: TaskId) -> String {
    format!("qcore_mutex_task_{child}")
}

/// Render an admitted fork/join trace with real C11 threads.
///
/// Card `MUTEX-SHARE-CROSS-EMISSION`. What this rendering claims, stated before
/// the code: the payload really crosses a thread boundary, and the generation
/// protecting it is really acquired and released **by the child's own thread**
/// through `mtx_lock`/`mtx_unlock`. What it does **not** claim is contention.
/// The checked trace is a total order in which a child's events are contiguous,
/// so the faithful realisation spawns the child and immediately joins it: the
/// parent performs no trace event while the child runs. That is a property of
/// the *checked* order, not a limitation worked around here — a rendering that
/// let the parent run ahead would realise an order the checker never saw.
/// Consequently no data-race evidence is available from this backend, and none
/// is claimed anywhere (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`).
fn emit_fork_join(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
    extents: BTreeMap<SharedResourceId, u32>,
    children: &[ChildSegment],
    payloads: &ProtectedPayloads,
) -> Result<MutexCEmission, MutexEmissionRefusal> {
    let mut file_scope = String::from(
        "\n/* Card MUTEX-SHARE-CROSS-EMISSION: a checked fork/join mutex trace.\n   \
         The generations and the protected allocations are declared here so a\n   \
         child's own function can name them; every acquisition and release is\n   \
         performed by the thread the checked trace says performs it. */\n",
    );
    for resource in extents.keys() {
        writeln!(file_scope, "static unsigned char *r{resource};").unwrap();
    }
    let mut mutexes: Vec<MutexId> = program
        .events
        .iter()
        .filter_map(|event| match event {
            MutexInvariantEvent::CreateMutex { mutex, .. } => Some(*mutex),
            _ => None,
        })
        .collect();
    mutexes.sort_unstable();
    mutexes.dedup();
    for mutex in &mutexes {
        writeln!(file_scope, "static mtx_t m{mutex};").unwrap();
    }
    // One function per child, in trace order, each carrying exactly the events
    // the classification assigned to it.
    for segment in children {
        let mut body = String::new();
        for index in (segment.spawn + 1)..segment.join {
            render_event(
                &mut body,
                program,
                certificate,
                index,
                &program.events[index],
                Placement::FileScope,
                payloads,
            )?;
        }
        writeln!(
            file_scope,
            "\nstatic int {name}(void *qcore_task_arg)\n{{\n    \
             (void)qcore_task_arg;\n{body}    return 0;\n}}",
            name = child_function(segment.child)
        )
        .unwrap();
    }

    let mut source = String::new();
    for (resource, end) in &extents {
        writeln!(
            source,
            "    r{resource} = malloc({end});\n    \
             if (r{resource} == NULL) qcore_fail(\"allocation\");\n    \
             memset(r{resource}, 0, {end});\n    \
             qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE, {resource}, {end}, INT64_C(0), (const char *)0);"
        )
        .unwrap();
    }
    for child in children.iter().map(|segment| segment.child) {
        writeln!(source, "    thrd_t qcore_thread_{child};").unwrap();
    }
    let mut index = 0;
    while index < program.events.len() {
        if let Some(segment) = children.iter().find(|segment| segment.spawn == index) {
            let (parent, child) = (segment.parent, segment.child);
            // The spawn is traced *before* the thread starts and the join
            // *after* it has been reclaimed, so the emitted line order is the
            // checked event order and not a race between two threads' writes.
            writeln!(
                source,
                "    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_SPAWN, {parent}, {child}, INT64_C(0), (const char *)0);\n    \
                 if (thrd_create(&qcore_thread_{child}, {name}, (void *)0) != thrd_success) qcore_fail(\"thrd_create\");",
                name = child_function(child)
            )
            .unwrap();
            writeln!(
                source,
                "    {{ int qcore_task_result_{child} = 0;\n      \
                 if (thrd_join(qcore_thread_{child}, &qcore_task_result_{child}) != thrd_success) qcore_fail(\"thrd_join\");\n      \
                 if (qcore_task_result_{child} != 0) qcore_fail(\"child task\"); }}\n    \
                 qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_JOIN, {parent}, {child}, INT64_C(0), (const char *)0);"
            )
            .unwrap();
            index = segment.join + 1;
            continue;
        }
        render_event(
            &mut source,
            program,
            certificate,
            index,
            &program.events[index],
            Placement::FileScope,
            payloads,
        )?;
        index += 1;
    }
    for resource in extents.keys() {
        writeln!(source, "    free(r{resource});").unwrap();
    }
    let mut fragments = typed_preamble(payloads);
    fragments.push(Fragment::new(
        Section::FileScope,
        "mutex.fork_join",
        &file_scope,
    ));
    Ok(MutexCEmission {
        source: c_program::assemble(
            &fragments,
            &c_program::Entry {
                lead: String::new(),
                brace: c_program::BraceStyle::KAndR,
                prologue: "    int qcore_exit = 0;\n".to_owned(),
                body: source,
                epilogue: "    return qcore_exit;\n}\n".to_owned(),
            },
        ),
        resource_bytes: extents,
    })
}

/// The file-scope text `CheckedMutexInvariantV1` contributes to a program it is
/// part of (card `C-EMIT-MAIN-UNIFY`).
#[must_use]
/// [`preamble`] for a module with typed protected payloads (card
/// `W48-QR-FLOAT-BUFFER-PAYLOADS`): a `protected f32`/`f64` payload's size is
/// binary32's and binary64's, so the program asserts the IEEE environment
/// (`scalar_floats::c_environment`, with `sizeof(float) == 4` and
/// `sizeof(double) == 8`) when it is compiled. Exactly [`preamble`] for every
/// other module, byte for byte.
fn typed_preamble(payloads: &ProtectedPayloads) -> Vec<Fragment> {
    let mut fragments = preamble();
    if payloads.values().any(|ty| ty.is_float()) {
        fragments.push(Fragment::new(
            Section::Includes,
            "scalar-floats.includes",
            crate::scalar_floats::C_INCLUDES,
        ));
        fragments.push(Fragment::new(
            Section::Prelude,
            "mutex.float-environment",
            format!("\n{}", crate::scalar_floats::c_environment()),
        ));
    }
    fragments
}

pub fn preamble() -> Vec<Fragment> {
    vec![
        Fragment::new(
            Section::Compat,
            "c11_threads.compat",
            include_str!("c11_threads_compat.h"),
        ),
        Fragment::new(Section::Includes, "mutex.includes", INCLUDES),
        crate::diagnostic_log_c_emit::diagnostic_log_fragment(),
        Fragment::new(Section::Prelude, "mutex.prelude", PRELUDE),
    ]
}

/// The mutex generation whose critical section `guard` names, found by walking
/// back to the `Lock` that minted it. A guard the trace never minted is a
/// refusal, not a guess.
fn holding_mutex(
    program: &MutexInvariantProgram,
    event: usize,
    guard: u32,
) -> Result<u32, MutexEmissionRefusal> {
    program.events[..event]
        .iter()
        .rev()
        .find_map(|candidate| match candidate {
            MutexInvariantEvent::Lock {
                mutex,
                guard: minted,
                ..
            } if *minted == guard => Some(*mutex),
            _ => None,
        })
        .ok_or(MutexEmissionRefusal::UnsupportedEvent { event })
}

#[cfg(test)]
#[path = "../tests/unit/mutex_c_emit.rs"]
mod tests;
