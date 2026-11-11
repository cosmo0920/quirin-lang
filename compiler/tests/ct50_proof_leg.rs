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

//! `CT-PROOF-LEG-3` (wave 41) — the named runner behind CT50's `proof`-stage
//! evidence, on the CT01/CT37/CT03 precedent
//! (`compiler/tests/ct0{1,3}_proof_leg.rs`, `ct37_proof_leg.rs`).
//!
//! The Lean side is `proof/QCore/Ct50LifetimeIdentity.lean`: a total decision
//! procedure `dischargesLifetimeIdentity` over the F02 observation alphabet,
//! theorems that its acceptance implies CT50's scenario property (each
//! generation allocated once and released once; the old generation's whole
//! existence closing before the new one begins; **no fact of the trace using
//! the old generation**; the one read naming the new generation and carrying
//! the new allocation's own payload, inside the new generation's own
//! lifetime), the proof that CT50's reference trace is accepted, and the
//! closure `every_admitted_rendering_discharges_the_lifetime_identity` — which
//! for this single-actor run follows from the stronger
//! `an_admitted_rendering_is_the_reference_run`.
//!
//! This test binds those theorems to **CT50's own artifact**: it compiles
//! `compiler/tests/fixtures/ct/CT50/case.qr` through the real pipeline,
//! executes the checked task module's reference execution, derives the
//! admitted causal order with the production observer, and
//!
//! 1. checks the derived events are exactly the trace the Lean `referenceRun`
//!    term transcribes (host-unconditionally — no `cfg`, no Clang, no native
//!    half);
//! 2. recomputes the Lean-proved transcription pin
//!    `QCore.Ct50LifetimeIdentity.ct50_trace_digest` from the artifact's own
//!    events with wrapping `u64` arithmetic (the F02 digest fold), so the Lean
//!    term and the artifact cannot drift apart silently — and the digest
//!    literal below is itself compared with the Lean source's by
//!    `compiler/tests/proof_leg_digests.rs`
//!    (`PROOF-LEG-DIGESTS-UNCROSSCHECKED`);
//! 3. runs the same total decision procedure the Lean definition defines,
//!    transcribed below clause for clause, and requires acceptance;
//! 4. **enumerates** the F02 relation's admitted linearizations of the
//!    artifact's own order and requires the admitted set to be exactly
//!    `{referenceRun}` — the artifact-side measurement of the Lean closure
//!    theorem `an_admitted_rendering_is_the_reference_run`;
//! 5. re-runs the procedure on the eight named mutants the Lean module proves
//!    refused, and requires refusal — the transcription agrees on the refusing
//!    side too — plus, for the four mutants the Lean module proves fail
//!    **exactly one** clause, re-measures that same clause isolation here.
//!
//! What this does **not** establish. CT50's scenario names a *numeric
//! address*: the native run places the second mapping at the first one's
//! address with `MAP_FIXED_NOREPLACE`, and that reuse is invisible to the F02
//! alphabet, which carries generations and not addresses. The property bound
//! here is the capability-level half; the address-level half stays with the
//! existing `run_native_task_scenarios` observation, unproved. Nor is
//! anything claimed about the emitted C's native execution or about a verified
//! extraction between Lean and Rust — the agreement is the checked-digest
//! transcription pattern F02 established.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, admitted_linearizations, relate,
};

const CT50_SOURCE: &str = include_str!("fixtures/ct/CT50/case.qr");

/// `QCore.Ct50LifetimeIdentity.ct50_trace_digest`, proved by the Lean kernel
/// on the `referenceRun` term this artifact must reproduce. Cross-checked
/// against the Lean source by `compiler/tests/proof_leg_digests.rs`.
const CT50_LEAN_TRACE_DIGEST: u64 = 1699395409980170611;

/// `QCore.Ct50LifetimeIdentity.stale` — the first lifetime's generation.
const STALE: u64 = 1;
/// `QCore.Ct50LifetimeIdentity.fresh` — the second lifetime's generation, which
/// the native run places at the first one's numeric address.
const FRESH: u64 = 2;

/* --------------------------------------------------- the F02 digest fold */

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("CT50's trace carries no negative payload")
}

/// Mirrors `QCore.TaskTraceRelation.encodeEvent`, exactly as the F02 harness
/// and the CT01/CT37/CT03 runners do.
fn encode_event(event: &ObservedEvent) -> u64 {
    match event {
        ObservedEvent::Allocated {
            task,
            buffer,
            payload,
        } => 1 + 16 * (task + 8 * (buffer + 64 * natural(*payload))),
        ObservedEvent::AllocationFailed { task } => 2 + 16 * task,
        ObservedEvent::Spawned { child, worker } => 3 + 16 * (child + 8 * worker),
        ObservedEvent::CreationFailed { worker } => 4 + 16 * worker,
        ObservedEvent::Read {
            task,
            buffer,
            value,
        } => 5 + 16 * (task + 8 * (buffer + 64 * natural(*value))),
        ObservedEvent::Written {
            task,
            buffer,
            value,
        } => 11 + 16 * (task + 8 * (buffer + 64 * natural(*value))),
        ObservedEvent::Dropped { task, buffer } => 6 + 16 * (task + 8 * buffer),
        ObservedEvent::Loaned { borrower, buffer } => 7 + 16 * (borrower + 8 * buffer),
        ObservedEvent::Returned { borrower, buffer } => 8 + 16 * (borrower + 8 * buffer),
        ObservedEvent::Completed { task, outcome } => {
            let encoded = match outcome {
                ObservedOutcome::Unit => 0,
                ObservedOutcome::I32(value) => 1 + 4 * natural(*value),
                ObservedOutcome::Buffer(buffer) => 2 + 4 * buffer,
                ObservedOutcome::Error(_) => {
                    panic!("typed error outcomes are outside CT50's trace")
                }
                // Card `W48-QR-SCALAR-TYPES-R6`: the grown alphabet's fixed-width
                // outcome, which no declared domain of this leg carries.
                ObservedOutcome::Fixed { .. } => {
                    panic!("fixed-width outcomes are outside the declared F02 domains")
                }
            };
            9 + 16 * (task + 8 * encoded)
        }
        ObservedEvent::Joined { child } => 10 + 16 * child,
    }
}

/// Mirrors `QCore.TaskTraceRelation.mixTrace`.
fn mix_trace(trace: &[ObservedEvent]) -> u64 {
    trace.iter().fold(TRACE_SEED, |accumulator, event| {
        accumulator
            .wrapping_mul(TRACE_MUL)
            .wrapping_add(encode_event(event))
            .wrapping_add(1)
    })
}

/* ------------------------------------------------ CT50's six facts, named */

fn stale_allocation() -> ObservedEvent {
    ObservedEvent::Allocated {
        task: 0,
        buffer: STALE,
        payload: 7,
    }
}

fn stale_release() -> ObservedEvent {
    ObservedEvent::Dropped {
        task: 0,
        buffer: STALE,
    }
}

fn fresh_allocation() -> ObservedEvent {
    ObservedEvent::Allocated {
        task: 0,
        buffer: FRESH,
        payload: 42,
    }
}

fn fresh_read() -> ObservedEvent {
    ObservedEvent::Read {
        task: 0,
        buffer: FRESH,
        value: 42,
    }
}

fn fresh_release() -> ObservedEvent {
    ObservedEvent::Dropped {
        task: 0,
        buffer: FRESH,
    }
}

fn root_result() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 0,
        outcome: ObservedOutcome::I32(42),
    }
}

/// `QCore.Ct50LifetimeIdentity.referenceRun`.
fn reference_run() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        fresh_read(),
        fresh_release(),
        root_result(),
    ]
}

/* --------------------------------- the Lean decision procedure, clause for
clause. `QCore.Ct50LifetimeIdentity.dischargesLifetimeIdentity`, over the
production `ObservedEvent` alphabet. ------------------------------------- */

/// `QCore.TaskTraceRelation.projection`.
fn projection(trace: &[ObservedEvent], actor: u64) -> Vec<ObservedEvent> {
    trace
        .iter()
        .filter(|event| event.actor() == Some(actor))
        .cloned()
        .collect()
}

/// `QCore.TaskTraceRelation.isAllocationOf`.
fn is_allocation_of(buffer: u64, event: &ObservedEvent) -> bool {
    matches!(event, ObservedEvent::Allocated { buffer: candidate, .. } if *candidate == buffer)
}

/// `QCore.TaskTraceRelation.isReleaseOf`.
fn is_release_of(buffer: u64, event: &ObservedEvent) -> bool {
    matches!(event, ObservedEvent::Dropped { buffer: candidate, .. } if *candidate == buffer)
}

/// `QCore.TaskTraceRelation.bufferOf`.
fn buffer_of(event: &ObservedEvent) -> Option<u64> {
    match event {
        ObservedEvent::Allocated { buffer, .. }
        | ObservedEvent::Read { buffer, .. }
        | ObservedEvent::Written { buffer, .. }
        | ObservedEvent::Dropped { buffer, .. }
        | ObservedEvent::Loaned { buffer, .. }
        | ObservedEvent::Returned { buffer, .. } => Some(*buffer),
        ObservedEvent::Completed {
            outcome: ObservedOutcome::Buffer(generation),
            ..
        } => Some(*generation),
        _ => None,
    }
}

/// `QCore.TaskTraceRelation.mentions`.
fn mentions(buffer: u64, event: &ObservedEvent) -> bool {
    buffer_of(event) == Some(buffer)
}

/// `QCore.Ct50LifetimeIdentity.isRead`.
fn is_read(event: &ObservedEvent) -> bool {
    matches!(event, ObservedEvent::Read { .. })
}

/// `QCore.Ct50LifetimeIdentity.readsBack`.
fn reads_back(buffer: u64, payload: i32, event: &ObservedEvent) -> bool {
    matches!(
        event,
        ObservedEvent::Read { buffer: candidate, value, .. }
            if *candidate == buffer && *value == payload
    )
}

/// `QCore.Ct50LifetimeIdentity.isStaleUse` — a fact that names the old
/// generation and is neither that generation's own allocation nor its own
/// release.
fn is_stale_use(event: &ObservedEvent) -> bool {
    mentions(STALE, event) && !is_allocation_of(STALE, event) && !is_release_of(STALE, event)
}

fn count(trace: &[ObservedEvent], predicate: impl Fn(&ObservedEvent) -> bool) -> usize {
    trace.iter().filter(|event| predicate(event)).count()
}

/// `List.Sublist` on the F02 alphabet: the events of `chain` occur in `trace`
/// in that relative order, not necessarily adjacently.
fn is_sublist(chain: &[ObservedEvent], trace: &[ObservedEvent]) -> bool {
    let mut wanted = chain.iter();
    let mut next = wanted.next();
    for event in trace {
        if let Some(target) = next
            && event == target
        {
            next = wanted.next();
        }
    }
    next.is_none()
}

/// The transcription of `dischargesLifetimeIdentity`, clause for clause and in
/// the Lean definition's own order.
fn discharges_lifetime_identity(trace: &[ObservedEvent]) -> bool {
    trace.len() == 6
        && projection(trace, 0) == trace
        && count(trace, |event| is_allocation_of(STALE, event)) == 1
        && count(trace, |event| is_release_of(STALE, event)) == 1
        && count(trace, |event| is_allocation_of(FRESH, event)) == 1
        && count(trace, |event| is_release_of(FRESH, event)) == 1
        && count(trace, is_stale_use) == 0
        && count(trace, is_read) == 1
        && count(trace, |event| reads_back(FRESH, 42, event)) == 1
        && is_sublist(
            &[stale_allocation(), stale_release(), fresh_allocation()],
            trace,
        )
        && is_sublist(&[fresh_allocation(), fresh_read(), fresh_release()], trace)
}

/* ------------------------------------------------------------- the artifact */

/// CT50's artifact: the fixture compiled through the real pipeline, its
/// checked task module executed by the reference execution, the admitted
/// causal order derived by the production observer.
fn ct50_admitted_order() -> AdmittedCausalOrder {
    let module = compile_files(
        &[SourceFile::new("case.qr", CT50_SOURCE)],
        &CompileOptions::default(),
    )
    .expect("CT50 is the manifest's accepted lifetime-identity case");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT50 lowers to a checked task module");
    let observation = execute_task_module(checked, &TaskExecutionScript::default())
        .expect("CT50's default reference run succeeds");
    AdmittedCausalOrder::derive(&observation)
        .expect("CT50's reference run is inside the F02 observable alphabet")
}

/// The `proof`-stage executor: CT50's own artifact is the trace term the Lean
/// theorems are about, and the same total decision procedure the Lean
/// definition defines accepts it.
#[test]
fn ct50_artifact_is_the_lean_proved_lifetime_identity_shape() {
    let order = ct50_admitted_order();
    let events = order.events().to_vec();
    // The artifact's reference run is exactly the trace the Lean
    // `referenceRun` term transcribes, event for event.
    assert_eq!(events, reference_run());
    // The Lean-proved transcription pin, recomputed from the artifact.
    assert_eq!(mix_trace(&events), CT50_LEAN_TRACE_DIGEST);
    // The same total decision procedure the Lean definition defines.
    assert!(discharges_lifetime_identity(&events));
    // The artifact's rendering is admitted by the production `relate` against
    // its own declared order.
    assert!(relate(&order, &events).is_ok());
    // `an_admitted_rendering_is_the_reference_run`, measured on the artifact:
    // the relation's admitted set over this order is the singleton
    // `{referenceRun}`, so the closure's degenerate shape is a measurement
    // here and a theorem in Lean, not an assumption in either place.
    let admitted = admitted_linearizations(&order, 100_000)
        .expect("CT50's six-event order is well inside the search bound");
    assert_eq!(admitted, vec![reference_run()]);
}

/* ----------------------------------------------------------- the mutants */

fn overlapping_lifetimes() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        fresh_allocation(),
        stale_release(),
        fresh_read(),
        fresh_release(),
        root_result(),
    ]
}

fn escaping_stale_capability() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        fresh_read(),
        fresh_release(),
        ObservedEvent::Completed {
            task: 0,
            outcome: ObservedOutcome::Buffer(STALE),
        },
    ]
}

fn read_after_the_fresh_release() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        fresh_release(),
        fresh_read(),
        root_result(),
    ]
}

fn result_claimed_by_a_second_actor() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        fresh_read(),
        fresh_release(),
        ObservedEvent::Completed {
            task: 1,
            outcome: ObservedOutcome::I32(42),
        },
    ]
}

/// Refused, but **not** an isolating mutant and deliberately not presented as
/// one: moving the read off the root task also moves it out of the read-window
/// chain, so two clauses catch it. Measured by this card's harness (`R4`
/// reddened nothing when the single-actor clause alone was removed).
fn read_by_a_second_actor() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        ObservedEvent::Read {
            task: 1,
            buffer: FRESH,
            value: 42,
        },
        fresh_release(),
        root_result(),
    ]
}

fn read_of_the_stale_generation() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        ObservedEvent::Read {
            task: 0,
            buffer: STALE,
            value: 7,
        },
        fresh_release(),
        root_result(),
    ]
}

fn read_carrying_the_old_payload() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        ObservedEvent::Read {
            task: 0,
            buffer: FRESH,
            value: 7,
        },
        fresh_release(),
        root_result(),
    ]
}

fn leaked_stale_generation() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        fresh_allocation(),
        fresh_read(),
        fresh_release(),
        root_result(),
    ]
}

fn double_release_of_the_fresh_generation() -> Vec<ObservedEvent> {
    vec![
        stale_allocation(),
        stale_release(),
        fresh_allocation(),
        fresh_read(),
        fresh_release(),
        ObservedEvent::Dropped {
            task: 0,
            buffer: FRESH,
        },
        root_result(),
    ]
}

/// The transcription agrees on the refusing side too: the nine mutants the
/// Lean module proves refused are refused here, mutant for mutant.
#[test]
fn the_procedure_refuses_the_lean_named_mutations() {
    for (name, mutant) in [
        ("overlapping lifetimes", overlapping_lifetimes()),
        ("an escaping stale capability", escaping_stale_capability()),
        (
            "a read after the fresh release",
            read_after_the_fresh_release(),
        ),
        (
            "a result claimed by a second actor",
            result_claimed_by_a_second_actor(),
        ),
        ("a read by a second actor", read_by_a_second_actor()),
        (
            "a read of the stale generation",
            read_of_the_stale_generation(),
        ),
        (
            "a read carrying the old payload",
            read_carrying_the_old_payload(),
        ),
        ("a leaked stale generation", leaked_stale_generation()),
        (
            "a double release of the fresh generation",
            double_release_of_the_fresh_generation(),
        ),
    ] {
        assert!(
            !discharges_lifetime_identity(&mutant),
            "{name} must be refused"
        );
    }
}

/// The four mutants the Lean module proves fail **exactly one** clause fail
/// exactly that clause here too: every other clause of the procedure still
/// holds of them. This is what separates "the binding caught it" from "some
/// clause caught it" — a mutant refused by the length clause would be evidence
/// about nothing.
#[test]
fn the_isolating_mutants_fail_only_their_named_clause() {
    // `overlapping_lifetimes_fail_only_the_old_lifetime_clause`.
    let trace = overlapping_lifetimes();
    assert_eq!(trace.len(), 6);
    assert_eq!(projection(&trace, 0), trace);
    assert_eq!(count(&trace, |event| is_allocation_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_allocation_of(FRESH, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(FRESH, event)), 1);
    assert_eq!(count(&trace, is_stale_use), 0);
    assert_eq!(count(&trace, is_read), 1);
    assert_eq!(count(&trace, |event| reads_back(FRESH, 42, event)), 1);
    assert!(is_sublist(
        &[fresh_allocation(), fresh_read(), fresh_release()],
        &trace
    ));
    assert!(!is_sublist(
        &[stale_allocation(), stale_release(), fresh_allocation()],
        &trace
    ));

    // `an_escaping_stale_capability_fails_only_the_stale_use_clause`.
    let trace = escaping_stale_capability();
    assert_eq!(trace.len(), 6);
    assert_eq!(projection(&trace, 0), trace);
    assert_eq!(count(&trace, |event| is_allocation_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_allocation_of(FRESH, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(FRESH, event)), 1);
    assert_eq!(count(&trace, is_read), 1);
    assert_eq!(count(&trace, |event| reads_back(FRESH, 42, event)), 1);
    assert!(is_sublist(
        &[stale_allocation(), stale_release(), fresh_allocation()],
        &trace
    ));
    assert!(is_sublist(
        &[fresh_allocation(), fresh_read(), fresh_release()],
        &trace
    ));
    assert_eq!(count(&trace, is_stale_use), 1);

    // `a_read_after_the_fresh_release_fails_only_the_read_window_clause`.
    let trace = read_after_the_fresh_release();
    assert_eq!(trace.len(), 6);
    assert_eq!(projection(&trace, 0), trace);
    assert_eq!(count(&trace, |event| is_allocation_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_allocation_of(FRESH, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(FRESH, event)), 1);
    assert_eq!(count(&trace, is_stale_use), 0);
    assert_eq!(count(&trace, is_read), 1);
    assert_eq!(count(&trace, |event| reads_back(FRESH, 42, event)), 1);
    assert!(is_sublist(
        &[stale_allocation(), stale_release(), fresh_allocation()],
        &trace
    ));
    assert!(!is_sublist(
        &[fresh_allocation(), fresh_read(), fresh_release()],
        &trace
    ));

    // `a_result_claimed_by_a_second_actor_fails_only_the_single_actor_clause`.
    let trace = result_claimed_by_a_second_actor();
    assert_eq!(trace.len(), 6);
    assert_eq!(count(&trace, |event| is_allocation_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(STALE, event)), 1);
    assert_eq!(count(&trace, |event| is_allocation_of(FRESH, event)), 1);
    assert_eq!(count(&trace, |event| is_release_of(FRESH, event)), 1);
    assert_eq!(count(&trace, is_stale_use), 0);
    assert_eq!(count(&trace, is_read), 1);
    assert_eq!(count(&trace, |event| reads_back(FRESH, 42, event)), 1);
    assert!(is_sublist(
        &[stale_allocation(), stale_release(), fresh_allocation()],
        &trace
    ));
    assert!(is_sublist(
        &[fresh_allocation(), fresh_read(), fresh_release()],
        &trace
    ));
    assert_ne!(projection(&trace, 0), trace);
}

/// `a_read_by_a_second_actor_fails_two_clauses`, and the measured redundancy
/// beside it. Both are findings of this card's mutation harness, published
/// rather than left to be rediscovered: a read moved off the root task fails
/// **two** clauses, so it isolates neither; and the payload clause is *implied*
/// by the read-window chain together with the read count
/// (`QCore.Ct50LifetimeIdentity.the_payload_clause_is_implied_by_the_read_window_and_the_read_count`),
/// so no trace can separate them and no mutation of it can redden anything.
#[test]
fn the_measured_non_isolating_mutant_and_the_implied_clause_are_stated() {
    let trace = read_by_a_second_actor();
    assert_ne!(projection(&trace, 0), trace);
    assert!(!is_sublist(
        &[fresh_allocation(), fresh_read(), fresh_release()],
        &trace
    ));

    // The implication, checked on every trace this file names: wherever the
    // read-window chain holds and there is exactly one read, the payload
    // clause holds too.
    for trace in [
        reference_run(),
        overlapping_lifetimes(),
        escaping_stale_capability(),
        result_claimed_by_a_second_actor(),
    ] {
        if is_sublist(&[fresh_allocation(), fresh_read(), fresh_release()], &trace)
            && count(&trace, is_read) == 1
        {
            assert_eq!(count(&trace, |event| reads_back(FRESH, 42, event)), 1);
        }
    }
}

/// The two mutants whose refusal the Lean module names as *also* a length
/// refusal are measured here as such, so the report's clause table is a
/// measurement rather than a reading of the definition.
#[test]
fn the_length_changing_mutants_are_named_as_such() {
    let leaked = leaked_stale_generation();
    assert_eq!(leaked.len(), 5);
    assert_eq!(count(&leaked, |event| is_release_of(STALE, event)), 0);

    let doubled = double_release_of_the_fresh_generation();
    assert_eq!(doubled.len(), 7);
    assert_eq!(count(&doubled, |event| is_release_of(FRESH, event)), 2);
}
