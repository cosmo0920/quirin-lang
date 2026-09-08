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
use std::collections::BTreeMap;
use std::fmt::Write;

use crate::mutex_invariant::{
    AccessFootprint, AccessKind, ConcurrencyProfile, MutexInvariantCertificate,
    MutexInvariantEvent, MutexInvariantProgram, SharedResourceId,
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
            | MutexInvariantEvent::BorrowUnderGuard { footprint, .. } => check(*footprint)?,
            _ => {}
        }
    }
    Ok(extents)
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

const PRELUDE: &str = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Generated by qcore_core::mutex_c_emit for a checked
   CheckedMutexInvariantV1 trace. One thread, real mtx_t generations, real
   heap footprints. Emitted only for a trace the core already checked. */
static void qcore_fail(const char *what) {
    fprintf(stderr, "qcore mutex runtime failure: %s\n", what);
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
    source.push_str(PRELUDE);
    source.push_str("int main(void) {\n    int qcore_exit = 0;\n");
    for (resource, end) in &extents {
        writeln!(
            source,
            "    unsigned char *r{resource} = malloc({end});\n    \
             if (r{resource} == NULL) qcore_fail(\"allocation\");\n    \
             memset(r{resource}, 0, {end});\n    \
             printf(\"resource {resource} {end}\\n\");"
        )
        .unwrap();
    }
    for (index, event) in program.events.iter().enumerate() {
        match event {
            MutexInvariantEvent::CreateMutex { mutex, .. } => writeln!(
                source,
                "    mtx_t m{mutex};\n    \
                 if (mtx_init(&m{mutex}, mtx_plain) != thrd_success) qcore_fail(\"mtx_init\");\n    \
                 printf(\"create {mutex}\\n\");"
            )
            .unwrap(),
            MutexInvariantEvent::Lock { mutex, guard, .. } => writeln!(
                source,
                "    if (mtx_lock(&m{mutex}) != thrd_success) qcore_fail(\"mtx_lock\");\n    \
                 printf(\"lock {mutex} {guard}\\n\");"
            )
            .unwrap(),
            MutexInvariantEvent::Unlock { guard, .. } => {
                let mutex = holding_mutex(program, index, *guard)?;
                writeln!(
                    source,
                    "    if (mtx_unlock(&m{mutex}) != thrd_success) qcore_fail(\"mtx_unlock\");\n    \
                     printf(\"unlock {guard}\\n\");"
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
                     printf(\"error-exit {guard} {exit}\\n\");"
                )
                .unwrap();
            }
            // The invariant phases have no runtime representation: they are
            // static bookkeeping the core already checked. They are traced so
            // the native and reference observations stay comparable.
            MutexInvariantEvent::OpenInvariant { guard, .. } => {
                writeln!(source, "    printf(\"open {guard}\\n\");").unwrap();
            }
            MutexInvariantEvent::CloseInvariant { guard, .. } => {
                writeln!(source, "    printf(\"close {guard}\\n\");").unwrap();
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
                         printf(\"write {resource} {start} {end}\\n\");",
                        fill_byte(index)
                    )
                    .unwrap(),
                    AccessKind::Read => writeln!(
                        source,
                        "    {{ unsigned long sum = 0;\n      \
                         for (unsigned i = {start}u; i < {end}u; i++) sum += r{resource}[i];\n      \
                         printf(\"read {resource} {start} {end} %lu\\n\", sum % 65536UL); }}"
                    )
                    .unwrap(),
                    _ => return Err(MutexEmissionRefusal::UnsupportedEvent { event: index }),
                }
            }
            MutexInvariantEvent::AssertFact { fact, .. } => {
                writeln!(source, "    printf(\"assert {fact}\\n\");").unwrap();
            }
            MutexInvariantEvent::UseFact { fact, .. } => {
                writeln!(source, "    printf(\"use-fact {fact}\\n\");").unwrap();
            }
            MutexInvariantEvent::BorrowUnderGuard {
                footprint, borrow, ..
            } => writeln!(
                source,
                "    unsigned char *b{borrow} = &r{}[{}];\n    \
                 if (b{borrow} == NULL) qcore_fail(\"borrow\");\n    \
                 printf(\"borrow {borrow}\\n\");",
                footprint.resource, footprint.start
            )
            .unwrap(),
            MutexInvariantEvent::EndBorrow { borrow, .. } => writeln!(
                source,
                "    b{borrow} = NULL;\n    \
                 if (b{borrow} != NULL) qcore_fail(\"borrow\");\n    \
                 printf(\"end-borrow {borrow}\\n\");"
            )
            .unwrap(),
            MutexInvariantEvent::DestroyMutex { mutex, .. } => writeln!(
                source,
                "    mtx_destroy(&m{mutex});\n    printf(\"destroy-mutex {mutex}\\n\");"
            )
            .unwrap(),
            MutexInvariantEvent::Destroy { resource, .. } => writeln!(
                source,
                "    free(r{resource});\n    r{resource} = NULL;\n    \
                 printf(\"destroy {resource}\\n\");"
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
