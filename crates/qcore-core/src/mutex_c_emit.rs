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

use crate::diagnostic_log_c_emit::DIAGNOSTIC_LOG_PRELUDE;
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

/// Reference execution of a checked trace. Mirrors the emitted C statement for
/// statement; neither one is derived from the other's output.
pub fn interpret_mutex_program(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
) -> Result<MutexObservation, MutexEmissionRefusal> {
    if certificate.profile != ConcurrencyProfile::CheckedMutexInvariantV1 {
        return Err(MutexEmissionRefusal::UncheckedProfile);
    }
    let extents = resource_extents(program)?;
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
                            *cell = fill_byte(index) as u8;
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

/// Emit single-threaded C11 for a checked trace.
pub fn emit_mutex_c11(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
) -> Result<MutexCEmission, MutexEmissionRefusal> {
    if certificate.profile != ConcurrencyProfile::CheckedMutexInvariantV1 {
        return Err(MutexEmissionRefusal::UncheckedProfile);
    }
    let extents = resource_extents(program)?;
    let mut source = String::from(include_str!("c11_threads_compat.h"));
    source.push_str(INCLUDES);
    source.push_str(DIAGNOSTIC_LOG_PRELUDE);
    source.push_str(PRELUDE);
    source.push_str("int main(void) {\n    int qcore_exit = 0;\n");
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
        match event {
            MutexInvariantEvent::CreateMutex { mutex, .. } => writeln!(
                source,
                "    mtx_t m{mutex};\n    \
                 if (mtx_init(&m{mutex}, mtx_plain) != thrd_success) qcore_fail(\"mtx_init\");\n    \
                 qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_MUTEX_CREATE, {mutex}, UINT64_C(0), INT64_C(0), (const char *)0);"
            )
            .unwrap(),
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
                        fill_byte(index)
                    )
                    .unwrap(),
                    AccessKind::Read => writeln!(
                        source,
                        "    {{ unsigned long sum = 0;\n      \
                         for (unsigned i = {start}u; i < {end}u; i++) sum += r{resource}[i];\n      \
                         qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_READ, {resource}, {start}, (int64_t)(sum % 65536UL), (const char *)0); }}"
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
                writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_INVARIANT_FACT, {mutex}, {}, INT64_C({}), \"grow\");", footprint.resource, footprint.end)
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
    }
    for resource in extents.keys() {
        writeln!(source, "    free(r{resource});").unwrap();
    }
    source.push_str("    return qcore_exit;\n}\n");
    Ok(MutexCEmission {
        source,
        resource_bytes: extents,
    })
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
