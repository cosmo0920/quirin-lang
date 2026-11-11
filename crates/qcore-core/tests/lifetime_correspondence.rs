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

//! F01 — one checker/model correspondence boundary.
//!
//! Rust side of the correspondence between the *generation-indexed lifetime
//! relation* of `qcore_core::concurrency::check_concurrent` and its Lean twin
//! `QCore.Concurrency.Lifetime.step` / `.run`
//! (`proof/QCore/LifetimeCorrespondence.lean`).
//!
//! Nothing here changes checker semantics: `check_concurrent` is used only
//! through its public API, and every model function below is a transcription of
//! the Lean definitions, never of the Rust checker.
//!
//! What is *checked* here (see `docs/reports/remaining-f01/README.md`):
//!
//! * `MODEL` is a faithful transcription of `Lifetime.step`/`Lifetime.run`:
//!   pinned by a whole-domain outcome digest that the Lean kernel independently
//!   recomputes (`domain_outcome_digest`).
//! * The enumerated domain is the same on both sides: pinned by a trace digest
//!   and a trace count the Lean kernel independently recomputes
//!   (`domain_trace_digest`, `domain_size`).
//! * Abstraction soundness on that domain: whenever `check_concurrent` raises no
//!   event-level diagnostic on a trace, `Lifetime.run` accepts its translation.
//! * The representation relation `R` on every observable component, probed
//!   through the checker's own diagnostics.
//!
//! What is *not* established: no claim that a Lean theorem proves the Rust
//! implementation correct, and no source-to-trace or trace-to-native relation.

use qcore_core::concurrency::{
    AccessFootprint, AccessKind, AccessPermission, ConcurrencyDiagnostic, ConcurrentEvent,
    ConcurrentProgram, MutexId, SharedResourceId, TaskId,
};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Transcription of QCore.Concurrency.Lifetime (proof/QCore/Concurrency.lean)
// ---------------------------------------------------------------------------

/// Transcription of `QCore.Concurrency.Lifetime.Event`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelEvent {
    Access {
        task: u32,
        resource: u32,
    },
    Transfer {
        sender: u32,
        to: u32,
        resource: u32,
        synchronized: bool,
    },
    Destroy {
        task: u32,
        resource: u32,
        prior_accesses_ordered: bool,
    },
    Spawn {
        parent: u32,
        child: u32,
    },
    Join {
        parent: u32,
        child: u32,
    },
    Lock {
        task: u32,
    },
    Unlock {
        task: u32,
    },
    #[allow(dead_code)]
    TaskAction {
        task: u32,
    },
}

/// Transcription of `QCore.Concurrency.Lifetime.State`. Field order and list
/// order follow the Lean structure exactly; the Lean model conses at the front
/// and `List.erase` removes the first occurrence, so the transcription does the
/// same and the whole-domain outcome digest is order sensitive.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelState {
    known_tasks: Vec<u32>,
    terminal_tasks: Vec<u32>,
    destroyed_resources: Vec<u32>,
    /// `(child, parent)`, as in Lean.
    parents: Vec<(u32, u32)>,
    /// `(resource, task)`, as in Lean.
    custody: Vec<(u32, u32)>,
    /// `(resource, task)`, positive shares only, as in Lean.
    permission_holders: Vec<(u32, u32)>,
    guard_holders: Vec<u32>,
}

impl ModelState {
    /// `State.Active`.
    fn active(&self, task: u32) -> bool {
        self.known_tasks.contains(&task) && !self.terminal_tasks.contains(&task)
    }

    /// `State.Live`.
    fn live(&self, resource: u32) -> bool {
        !self.destroyed_resources.contains(&resource)
    }

    /// `State.OtherAuthorityQuiescent`.
    fn other_authority_quiescent(&self, task: u32, resource: u32) -> bool {
        self.permission_holders.iter().all(|holder| {
            holder.0 != resource || holder.1 == task || self.terminal_tasks.contains(&holder.1)
        }) && self.guard_holders.iter().all(|holder| *holder == task)
    }

    /// `State.ChildQuiescent`.
    fn child_quiescent(&self, child: u32) -> bool {
        !self.guard_holders.contains(&child)
            && self
                .parents
                .iter()
                .all(|relation| relation.1 != child || self.terminal_tasks.contains(&relation.0))
    }
}

fn without(entries: &[(u32, u32)], resource: u32) -> Vec<(u32, u32)> {
    entries
        .iter()
        .copied()
        .filter(|entry| entry.0 != resource)
        .collect()
}

/// Transcription of `QCore.Concurrency.Lifetime.step`.
fn model_step(state: &ModelState, event: ModelEvent) -> Option<ModelState> {
    match event {
        ModelEvent::Access { task, resource } => {
            if state.active(task) && state.live(resource) {
                Some(state.clone())
            } else {
                None
            }
        }
        ModelEvent::Transfer {
            sender,
            to,
            resource,
            synchronized,
        } => {
            if state.active(sender)
                && state.active(to)
                && state.live(resource)
                && state.custody.contains(&(resource, sender))
                && synchronized
            {
                let mut next = state.clone();
                let mut custody = vec![(resource, to)];
                custody.extend(without(&state.custody, resource));
                next.custody = custody;
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::Destroy {
            task,
            resource,
            prior_accesses_ordered,
        } => {
            if state.active(task)
                && state.live(resource)
                && state.custody.contains(&(resource, task))
                && state.other_authority_quiescent(task, resource)
                && prior_accesses_ordered
            {
                let mut next = state.clone();
                let mut destroyed = vec![resource];
                destroyed.extend(state.destroyed_resources.iter().copied());
                next.destroyed_resources = destroyed;
                next.custody = without(&state.custody, resource);
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::Spawn { parent, child } => {
            if state.active(parent) && !state.known_tasks.contains(&child) {
                let mut next = state.clone();
                let mut known = vec![child];
                known.extend(state.known_tasks.iter().copied());
                next.known_tasks = known;
                let mut parents = vec![(child, parent)];
                parents.extend(state.parents.iter().copied());
                next.parents = parents;
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::Join { parent, child } => {
            if state.active(parent)
                && state.active(child)
                && state.parents.contains(&(child, parent))
                && state.child_quiescent(child)
                && !state.guard_holders.contains(&parent)
            {
                let mut next = state.clone();
                let mut terminal = vec![child];
                terminal.extend(state.terminal_tasks.iter().copied());
                next.terminal_tasks = terminal;
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::Lock { task } => {
            if state.active(task) {
                let mut next = state.clone();
                let mut guards = vec![task];
                guards.extend(state.guard_holders.iter().copied());
                next.guard_holders = guards;
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::Unlock { task } => {
            if state.active(task) && state.guard_holders.contains(&task) {
                let mut next = state.clone();
                // `List.erase` removes the first occurrence only.
                let position = next
                    .guard_holders
                    .iter()
                    .position(|holder| *holder == task)
                    .expect("membership was checked");
                next.guard_holders.remove(position);
                Some(next)
            } else {
                None
            }
        }
        ModelEvent::TaskAction { task } => {
            if state.active(task) {
                Some(state.clone())
            } else {
                None
            }
        }
    }
}

/// Transcription of `QCore.Concurrency.Lifetime.run`.
fn model_run(initial: &ModelState, events: &[ModelEvent]) -> Option<ModelState> {
    let mut state = initial.clone();
    for &event in events {
        state = model_step(&state, event)?;
    }
    Some(state)
}

// ---------------------------------------------------------------------------
// The declared finite domain (mirrored in Lean as `alphabet` / `domainTraces`)
// ---------------------------------------------------------------------------

/// Depth of the domain the Lean kernel independently evaluates. Every digest
/// and count below is pinned on both sides at this depth.
const KERNEL_DEPTH: usize = 3;
/// Depth of the Rust-only differential extension. The Lean domain stops at
/// `KERNEL_DEPTH` because `decide` over the depth-4 domain needs a recursion
/// limit high enough to risk a stack overflow on other hosts; the correspondence
/// properties themselves are cheap here and are exercised at both depths.
const EXTENDED_DEPTH: usize = 4;
const RESOURCE: SharedResourceId = 7;

/// `QCore.Concurrency.LifetimeCorrespondence.domainInitial`.
fn model_initial() -> ModelState {
    ModelState {
        known_tasks: vec![0],
        terminal_tasks: Vec::new(),
        destroyed_resources: Vec::new(),
        parents: Vec::new(),
        custody: vec![(RESOURCE, 0)],
        permission_holders: vec![(RESOURCE, 0), (RESOURCE, 1)],
        guard_holders: Vec::new(),
    }
}

/// The fixed permission set and initial custody every domain program carries.
/// The two shares sum to `FULL_ACCESS`, so `validate_permissions` contributes
/// no diagnostic and the correspondence is about the event loop only.
fn domain_permissions() -> Vec<AccessPermission> {
    vec![
        AccessPermission {
            task: 0,
            footprint: AccessFootprint::new(RESOURCE, 0, 4),
            share: 512,
        },
        AccessPermission {
            task: 1,
            footprint: AccessFootprint::new(RESOURCE, 0, 4),
            share: 512,
        },
    ]
}

fn program(events: &[ConcurrentEvent]) -> ConcurrentProgram {
    ConcurrentProgram {
        permissions: domain_permissions(),
        initial_custody: BTreeMap::from([(RESOURCE, 0)]),
        events: events.to_vec(),
    }
}

/// `QCore.Concurrency.LifetimeCorrespondence.alphabet`, in Lean order.
fn model_alphabet() -> Vec<ModelEvent> {
    vec![
        ModelEvent::Spawn {
            parent: 0,
            child: 1,
        },
        ModelEvent::Spawn {
            parent: 1,
            child: 2,
        },
        ModelEvent::Join {
            parent: 0,
            child: 1,
        },
        ModelEvent::Join {
            parent: 1,
            child: 2,
        },
        ModelEvent::Lock { task: 0 },
        ModelEvent::Lock { task: 1 },
        ModelEvent::Unlock { task: 0 },
        ModelEvent::Unlock { task: 1 },
        ModelEvent::Access {
            task: 1,
            resource: RESOURCE,
        },
        ModelEvent::Transfer {
            sender: 0,
            to: 1,
            resource: RESOURCE,
            synchronized: true,
        },
        ModelEvent::Destroy {
            task: 0,
            resource: RESOURCE,
            prior_accesses_ordered: true,
        },
        ModelEvent::Destroy {
            task: 0,
            resource: RESOURCE,
            prior_accesses_ordered: false,
        },
    ]
}

/// The checker-side alphabet. Two distinct `Lock` events by task 0 collapse onto
/// the single Lean `.lock 0`: the model abstracts mutex identity, which is one of
/// the measured divergence classes below.
fn checker_alphabet() -> Vec<ConcurrentEvent> {
    vec![
        ConcurrentEvent::Spawn {
            parent: 0,
            child: 1,
        },
        ConcurrentEvent::Spawn {
            parent: 1,
            child: 2,
        },
        ConcurrentEvent::Join {
            parent: 0,
            child: 1,
        },
        ConcurrentEvent::Join {
            parent: 1,
            child: 2,
        },
        ConcurrentEvent::Lock {
            task: 0,
            mutex: 4,
            rank: 1,
        },
        ConcurrentEvent::Lock {
            task: 0,
            mutex: 5,
            rank: 1,
        },
        ConcurrentEvent::Lock {
            task: 1,
            mutex: 6,
            rank: 1,
        },
        ConcurrentEvent::Unlock { task: 0, mutex: 5 },
        ConcurrentEvent::Unlock { task: 1, mutex: 6 },
        ConcurrentEvent::Access {
            task: 1,
            footprint: AccessFootprint::new(RESOURCE, 0, 2),
            kind: AccessKind::Read,
            guarded_by: None,
        },
        ConcurrentEvent::TransferCustody {
            from: 0,
            to: 1,
            resource: RESOURCE,
            synchronized: true,
        },
        ConcurrentEvent::Destroy {
            task: 0,
            resource: RESOURCE,
        },
    ]
}

/// The event translation `tau`. `Destroy` is the one event whose Lean image
/// depends on a bit the abstract state does not carry: the vector-clock
/// ordering of prior accesses, which `Lifetime.destroy` takes as an input.
/// An *accepted* `Destroy` in the checker necessarily computed
/// `prior_accesses_ordered = true`, so the soundness direction translates with
/// `true`; the `false` variant is in the Lean alphabet and is exercised there.
fn translate(event: &ConcurrentEvent) -> ModelEvent {
    match *event {
        ConcurrentEvent::Spawn { parent, child } => ModelEvent::Spawn { parent, child },
        ConcurrentEvent::Join { parent, child } => ModelEvent::Join { parent, child },
        ConcurrentEvent::Lock { task, .. } => ModelEvent::Lock { task },
        ConcurrentEvent::Unlock { task, .. } => ModelEvent::Unlock { task },
        ConcurrentEvent::Access {
            task, footprint, ..
        } => ModelEvent::Access {
            task,
            resource: footprint.resource,
        },
        ConcurrentEvent::TransferCustody {
            from,
            to,
            resource,
            synchronized,
        } => ModelEvent::Transfer {
            sender: from,
            to,
            resource,
            synchronized,
        },
        ConcurrentEvent::Destroy { task, resource } => ModelEvent::Destroy {
            task,
            resource,
            prior_accesses_ordered: true,
        },
    }
}

/// `levels 0 ++ levels 1 ++ ... ++ levels depth`, in Lean order.
fn enumerate<T: Clone>(alphabet: &[T], depth: usize) -> Vec<Vec<T>> {
    let mut all = Vec::new();
    let mut level = vec![Vec::new()];
    all.extend(level.iter().cloned());
    for _ in 0..depth {
        let mut next = Vec::with_capacity(level.len() * alphabet.len());
        for trace in &level {
            for event in alphabet {
                let mut extended = trace.clone();
                extended.push(event.clone());
                next.push(extended);
            }
        }
        all.extend(next.iter().cloned());
        level = next;
    }
    all
}

// ---------------------------------------------------------------------------
// Digests (mirrored in Lean; all arithmetic is mod 2^64)
// ---------------------------------------------------------------------------

const TRACE_SEED: u64 = 1_469_598_103_934_665_603;
const TRACE_MUL: u64 = 1_000_003;
const DOMAIN_SEED: u64 = 14_695_981_039_346_656_037;
const DOMAIN_MUL: u64 = 1_099_511_628_211;

/// Injective on task ids below 8 and resource ids below 64, which the declared
/// domain respects.
fn encode_event(event: ModelEvent) -> u64 {
    let (tag, payload) = match event {
        ModelEvent::Access { task, resource } => (1u64, u64::from(task) + 8 * u64::from(resource)),
        ModelEvent::Transfer {
            sender,
            to,
            resource,
            synchronized,
        } => (
            2,
            u64::from(sender)
                + 8 * (u64::from(to) + 8 * (u64::from(resource) + 64 * u64::from(synchronized))),
        ),
        ModelEvent::Destroy {
            task,
            resource,
            prior_accesses_ordered,
        } => (
            3,
            u64::from(task) + 8 * (u64::from(resource) + 64 * u64::from(prior_accesses_ordered)),
        ),
        ModelEvent::Spawn { parent, child } => (4, u64::from(parent) + 8 * u64::from(child)),
        ModelEvent::Join { parent, child } => (5, u64::from(parent) + 8 * u64::from(child)),
        ModelEvent::Lock { task } => (6, u64::from(task)),
        ModelEvent::Unlock { task } => (7, u64::from(task)),
        ModelEvent::TaskAction { task } => (8, u64::from(task)),
    };
    tag + 16 * payload
}

fn mix(accumulator: u64, value: u64) -> u64 {
    accumulator
        .wrapping_mul(TRACE_MUL)
        .wrapping_add(value)
        .wrapping_add(1)
}

fn mix_trace(events: &[ModelEvent]) -> u64 {
    events.iter().fold(TRACE_SEED, |accumulator, &event| {
        mix(accumulator, encode_event(event))
    })
}

fn trace_digest(traces: &[Vec<ModelEvent>]) -> u64 {
    traces.iter().fold(DOMAIN_SEED, |accumulator, trace| {
        accumulator
            .wrapping_mul(DOMAIN_MUL)
            .wrapping_add(mix_trace(trace))
            .wrapping_add(1)
    })
}

fn mix_nats(seed: u64, values: &[u32]) -> u64 {
    values.iter().fold(
        mix(seed, u64::try_from(values.len()).expect("small")),
        |accumulator, &value| mix(accumulator, u64::from(value)),
    )
}

fn mix_pairs(seed: u64, values: &[(u32, u32)]) -> u64 {
    values.iter().fold(
        mix(seed, u64::try_from(values.len()).expect("small")),
        |accumulator, &(left, right)| mix(accumulator, u64::from(left) * 64 + u64::from(right)),
    )
}

fn encode_state(state: &ModelState) -> u64 {
    let mut accumulator = mix_nats(TRACE_SEED, &state.known_tasks);
    accumulator = mix_nats(accumulator, &state.terminal_tasks);
    accumulator = mix_nats(accumulator, &state.destroyed_resources);
    accumulator = mix_pairs(accumulator, &state.parents);
    accumulator = mix_pairs(accumulator, &state.custody);
    accumulator = mix_pairs(accumulator, &state.permission_holders);
    mix_nats(accumulator, &state.guard_holders)
}

fn encode_outcome(outcome: Option<&ModelState>) -> u64 {
    match outcome {
        None => 0,
        Some(state) => encode_state(state).wrapping_add(1),
    }
}

fn outcome_digest(initial: &ModelState, traces: &[Vec<ModelEvent>]) -> u64 {
    traces.iter().fold(DOMAIN_SEED, |accumulator, trace| {
        accumulator
            .wrapping_mul(DOMAIN_MUL)
            .wrapping_add(encode_outcome(model_run(initial, trace).as_ref()))
            .wrapping_add(1)
    })
}

// ---------------------------------------------------------------------------
// Checker-side observation helpers
// ---------------------------------------------------------------------------

/// End-of-trace sweeps are not per-event decisions and have no counterpart in
/// the abstract lifetime relation, so the correspondence is stated over the
/// event-level diagnostics only.
fn event_level(diagnostics: &[ConcurrencyDiagnostic]) -> Vec<ConcurrencyDiagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            !matches!(
                diagnostic,
                ConcurrencyDiagnostic::OutstandingTask { .. }
                    | ConcurrencyDiagnostic::OutstandingLock { .. }
            )
        })
        .cloned()
        .collect()
}

fn check(events: &[ConcurrentEvent]) -> Result<Vec<TaskId>, Vec<ConcurrencyDiagnostic>> {
    match qcore_core::concurrency::check_concurrent(&program(events)) {
        Ok(certificate) => Ok(certificate.completed_tasks),
        Err(diagnostics) => Err(diagnostics),
    }
}

fn checker_event_level(events: &[ConcurrentEvent]) -> Vec<ConcurrencyDiagnostic> {
    match qcore_core::concurrency::check_concurrent(&program(events)) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => event_level(&diagnostics),
    }
}

/// Diagnostics raised by appending one event to a trace whose event-level
/// diagnostic set is already empty.
fn appended(prefix: &[ConcurrentEvent], event: ConcurrentEvent) -> Vec<ConcurrencyDiagnostic> {
    let mut extended = prefix.to_vec();
    extended.push(event);
    checker_event_level(&extended)
}

/// The tasks whose activeness `check_concurrent` tests for an event, in the
/// order it tests them.
fn checked_actors(event: &ConcurrentEvent) -> Vec<TaskId> {
    match *event {
        ConcurrentEvent::Spawn { parent, .. } | ConcurrentEvent::Join { parent, .. } => {
            vec![parent]
        }
        ConcurrentEvent::Lock { task, .. }
        | ConcurrentEvent::Unlock { task, .. }
        | ConcurrentEvent::Access { task, .. }
        | ConcurrentEvent::Destroy { task, .. } => vec![task],
        ConcurrentEvent::TransferCustody { from, to, .. } => vec![from, to],
    }
}

fn probe_lock(task: TaskId) -> ConcurrentEvent {
    // A mutex no domain trace ever takes, at a rank no held guard can violate,
    // so the only diagnostics this probe can raise are the `active_task` ones.
    ConcurrentEvent::Lock {
        task,
        mutex: 90 + MutexId::from(u16::try_from(task).expect("small")),
        rank: 9,
    }
}

// ---------------------------------------------------------------------------
// Pinned constants. Every one of these is independently recomputed by the Lean
// kernel in proof/QCore/LifetimeCorrespondence.lean.
// ---------------------------------------------------------------------------

const DOMAIN_SIZE: usize = 1885;
const TRACE_DIGEST: u64 = 16_753_757_158_032_361_952;
const OUTCOME_DIGEST: u64 = 12_252_691_954_508_661_170;
const MODEL_ACCEPTED_TRACES: usize = 53;
const MODEL_TERMINAL_REACHING_TRACES: usize = 6;
const MODEL_TERMINAL_PARTICIPANT_PROBES: usize = 42;
const TERMINAL_REFUSAL_PROBES_AT_KERNEL_DEPTH: usize = 36;
const TERMINAL_REFUSAL_PROBES_AT_EXTENDED_DEPTH: usize = 144;

#[test]
fn declared_domain_matches_the_lean_enumeration() {
    let traces = enumerate(&model_alphabet(), KERNEL_DEPTH);
    assert_eq!(traces.len(), DOMAIN_SIZE);
    assert_eq!(trace_digest(&traces), TRACE_DIGEST);
}

#[test]
fn model_transcription_matches_the_lean_evaluation() {
    let traces = enumerate(&model_alphabet(), KERNEL_DEPTH);
    let initial = model_initial();
    assert_eq!(outcome_digest(&initial, &traces), OUTCOME_DIGEST);
    let accepted = traces
        .iter()
        .filter(|trace| model_run(&initial, trace).is_some())
        .count();
    assert_eq!(accepted, MODEL_ACCEPTED_TRACES);
    let terminal_reaching = traces
        .iter()
        .filter(|trace| {
            model_run(&initial, trace).is_some_and(|state| !state.terminal_tasks.is_empty())
        })
        .count();
    assert_eq!(terminal_reaching, MODEL_TERMINAL_REACHING_TRACES);
}

/// Recomputes, and pins, the same numbers the Lean kernel evaluates in
/// `domain_terminal_participant_probe_count`. Printing them keeps the
/// regeneration path obvious when the domain is deliberately changed.
#[test]
fn lean_pinned_constants_are_recomputed_here() {
    let traces = enumerate(&model_alphabet(), KERNEL_DEPTH);
    let initial = model_initial();
    let alphabet = model_alphabet();
    let probes: usize = traces
        .iter()
        .map(|trace| match model_run(&initial, trace) {
            None => 0,
            Some(state) => alphabet
                .iter()
                .filter(|event| {
                    model_participants(**event)
                        .iter()
                        .any(|task| state.terminal_tasks.contains(task))
                })
                .count(),
        })
        .sum();
    println!("DOMAIN_SIZE {}", traces.len());
    println!("TRACE_DIGEST {}", trace_digest(&traces));
    println!("OUTCOME_DIGEST {}", outcome_digest(&initial, &traces));
    println!(
        "MODEL_ACCEPTED_TRACES {}",
        traces
            .iter()
            .filter(|trace| model_run(&initial, trace).is_some())
            .count()
    );
    println!(
        "MODEL_TERMINAL_REACHING_TRACES {}",
        traces
            .iter()
            .filter(|trace| model_run(&initial, trace)
                .is_some_and(|state| !state.terminal_tasks.is_empty()))
            .count()
    );
    println!("MODEL_TERMINAL_PARTICIPANT_PROBES {}", probes);
    assert_eq!(MODEL_TERMINAL_PARTICIPANT_PROBES, probes);
}

/// `QCore.Concurrency.LifetimeCorrespondence.participants`.
fn model_participants(event: ModelEvent) -> Vec<u32> {
    match event {
        ModelEvent::Access { task, .. }
        | ModelEvent::Destroy { task, .. }
        | ModelEvent::Lock { task }
        | ModelEvent::Unlock { task }
        | ModelEvent::TaskAction { task } => vec![task],
        ModelEvent::Transfer { sender, to, .. } => vec![sender, to],
        ModelEvent::Spawn { parent, child } => vec![parent, child],
        ModelEvent::Join { parent, child } => vec![parent, child],
    }
}

// ---------------------------------------------------------------------------
// The correspondence itself
// ---------------------------------------------------------------------------

#[test]
fn checker_acceptance_implies_model_acceptance_over_the_declared_domain() {
    for depth in [KERNEL_DEPTH, EXTENDED_DEPTH] {
        checker_acceptance_implies_model_acceptance(depth);
    }
}

fn checker_acceptance_implies_model_acceptance(depth: usize) {
    let initial = model_initial();
    let mut clean = 0usize;
    for trace in enumerate(&checker_alphabet(), depth) {
        let diagnostics = checker_event_level(&trace);
        if !diagnostics.is_empty() {
            continue;
        }
        clean += 1;
        let translated: Vec<ModelEvent> = trace.iter().map(translate).collect();
        assert!(
            model_run(&initial, &translated).is_some(),
            "checker accepted {trace:?} but the lifetime model refused its translation"
        );
    }
    assert!(clean > 0, "the domain must contain accepted traces");
}

#[test]
fn certificate_projections_equal_the_model_state() {
    for depth in [KERNEL_DEPTH, EXTENDED_DEPTH] {
        certificate_projections_equal_the_model(depth);
    }
}

fn certificate_projections_equal_the_model(depth: usize) {
    let initial = model_initial();
    let mut certificates = 0usize;
    for trace in enumerate(&checker_alphabet(), depth) {
        let Ok(completed) = check(&trace) else {
            continue;
        };
        certificates += 1;
        let translated: Vec<ModelEvent> = trace.iter().map(translate).collect();
        let state = model_run(&initial, &translated).expect("acceptance implies model acceptance");
        let mut terminal = state.terminal_tasks.clone();
        terminal.sort_unstable();
        assert_eq!(completed, terminal, "completed_tasks vs terminalTasks");

        let certificate = qcore_core::concurrency::check_concurrent(&program(&trace))
            .expect("just checked to be Ok");
        let mut destroyed = state.destroyed_resources.clone();
        destroyed.sort_unstable();
        assert_eq!(
            certificate.destroyed_resources, destroyed,
            "destroyed_resources vs destroyedResources"
        );
    }
    assert!(certificates > 0, "the domain must contain certificates");
}

#[test]
fn representation_relation_holds_on_every_observable_component() {
    for depth in [KERNEL_DEPTH, EXTENDED_DEPTH] {
        representation_relation_holds(depth);
    }
}

fn representation_relation_holds(depth: usize) {
    let initial = model_initial();
    let mut probed = 0usize;
    for trace in enumerate(&checker_alphabet(), depth) {
        if !checker_event_level(&trace).is_empty() {
            continue;
        }
        let translated: Vec<ModelEvent> = trace.iter().map(translate).collect();
        let state = model_run(&initial, &translated).expect("acceptance implies model acceptance");
        let index = trace.len();
        probed += 1;

        // knownTasks and terminalTasks, read off the `active_task` decision.
        for task in 0u32..3 {
            let raised = appended(&trace, probe_lock(task));
            if !state.known_tasks.contains(&task) {
                assert_eq!(
                    raised,
                    vec![ConcurrencyDiagnostic::UnknownTask { event: index, task }],
                    "task {task} absent from knownTasks after {trace:?}"
                );
            } else if state.terminal_tasks.contains(&task) {
                assert_eq!(
                    raised,
                    vec![ConcurrencyDiagnostic::TerminalTask { event: index, task }],
                    "task {task} terminal after {trace:?}"
                );
            } else {
                assert!(
                    raised.is_empty(),
                    "task {task} active after {trace:?} but the checker raised {raised:?}"
                );
            }
            assert_eq!(
                model_step(&state, ModelEvent::Lock { task }).is_some(),
                raised.is_empty(),
                "lock probe disagreement for task {task} after {trace:?}"
            );
        }

        // destroyedResources and custody, read off the `Destroy` decision order.
        for task in 0u32..3 {
            let raised = appended(
                &trace,
                ConcurrentEvent::Destroy {
                    task,
                    resource: RESOURCE,
                },
            );
            if state.active(task) {
                assert_eq!(
                    raised.contains(&ConcurrencyDiagnostic::DeadResource {
                        event: index,
                        resource: RESOURCE
                    }),
                    !state.live(RESOURCE),
                    "destroyedResources disagreement after {trace:?}"
                );
                if state.live(RESOURCE) {
                    assert_eq!(
                        raised.contains(&ConcurrencyDiagnostic::WrongCustodian {
                            event: index,
                            resource: RESOURCE
                        }),
                        !state.custody.contains(&(RESOURCE, task)),
                        "custody disagreement for task {task} after {trace:?}"
                    );
                    if state.custody.contains(&(RESOURCE, task))
                        && !state.other_authority_quiescent(task, RESOURCE)
                    {
                        assert!(
                            raised.contains(&ConcurrencyDiagnostic::NonquiescentDestruction {
                                event: index,
                                resource: RESOURCE
                            }),
                            "quiescence disagreement for task {task} after {trace:?}"
                        );
                    }
                }
            }
            if raised.is_empty() {
                assert!(
                    model_step(
                        &state,
                        ModelEvent::Destroy {
                            task,
                            resource: RESOURCE,
                            prior_accesses_ordered: true,
                        }
                    )
                    .is_some(),
                    "accepted destroy refused by the model after {trace:?}"
                );
            }
        }

        // guardHolders, as a multiset, read off `Unlock` capability.
        for (task, mutexes) in [(0u32, &[4u32, 5u32][..]), (1, &[6][..]), (2, &[][..])] {
            let clean_unlocks = mutexes
                .iter()
                .filter(|&&mutex| {
                    appended(&trace, ConcurrentEvent::Unlock { task, mutex }).is_empty()
                })
                .count();
            assert_eq!(
                clean_unlocks,
                state
                    .guard_holders
                    .iter()
                    .filter(|holder| **holder == task)
                    .count(),
                "guardHolders disagreement for task {task} after {trace:?}"
            );
        }

        // parents and terminalTasks, read off the `UnknownJoin` decision.
        for (parent, child) in [(0u32, 1u32), (1, 2), (0, 2), (2, 1)] {
            let raised = appended(&trace, ConcurrentEvent::Join { parent, child });
            if state.active(parent) {
                assert_eq!(
                    raised.contains(&ConcurrencyDiagnostic::UnknownJoin {
                        event: index,
                        child
                    }),
                    !(state.parents.contains(&(child, parent))
                        && !state.terminal_tasks.contains(&child)),
                    "parents disagreement for ({parent},{child}) after {trace:?}"
                );
            }
            if raised.is_empty() {
                assert!(
                    model_step(&state, ModelEvent::Join { parent, child }).is_some(),
                    "accepted join refused by the model after {trace:?}"
                );
            }
        }
    }
    assert!(probed > 0, "the domain must contain probed states");
}

#[test]
fn terminal_tasks_are_refused_before_every_authority_check() {
    assert_eq!(
        terminal_refusal_probes(KERNEL_DEPTH),
        TERMINAL_REFUSAL_PROBES_AT_KERNEL_DEPTH
    );
    assert_eq!(
        terminal_refusal_probes(EXTENDED_DEPTH),
        TERMINAL_REFUSAL_PROBES_AT_EXTENDED_DEPTH
    );
}

fn terminal_refusal_probes(depth: usize) -> usize {
    let initial = model_initial();
    let mut refusals = 0usize;
    for trace in enumerate(&checker_alphabet(), depth) {
        if !checker_event_level(&trace).is_empty() {
            continue;
        }
        let translated: Vec<ModelEvent> = trace.iter().map(translate).collect();
        let state = model_run(&initial, &translated).expect("acceptance implies model acceptance");
        if state.terminal_tasks.is_empty() {
            continue;
        }
        let index = trace.len();
        for event in checker_alphabet() {
            let Some(&terminal) = checked_actors(&event)
                .iter()
                .find(|task| state.terminal_tasks.contains(task))
            else {
                continue;
            };
            let raised = appended(&trace, event.clone());
            assert!(
                raised.contains(&ConcurrencyDiagnostic::TerminalTask {
                    event: index,
                    task: terminal
                }),
                "terminal task {terminal} was not refused for {event:?} after {trace:?}: {raised:?}"
            );
            assert!(
                model_step(&state, translate(&event)).is_none(),
                "the model admitted {event:?} for terminal task {terminal} after {trace:?}"
            );
            refusals += 1;
        }
    }
    assert!(
        refusals > 0,
        "the terminal-authority probe must not be vacuous"
    );
    refusals
}

// ---------------------------------------------------------------------------
// The converse fails: the model is a strictly coarser abstraction. This census
// measures where, so no reader can mistake the relation for an equivalence.
// ---------------------------------------------------------------------------

const COARSER_ABSTRACTION_TRACES_AT_KERNEL_DEPTH: usize = 35;
const COARSER_ABSTRACTION_TRACES_AT_EXTENDED_DEPTH: usize = 285;

#[test]
fn the_abstraction_is_strictly_coarser_than_the_checker() {
    assert_eq!(
        coarser_abstraction_traces(KERNEL_DEPTH),
        COARSER_ABSTRACTION_TRACES_AT_KERNEL_DEPTH
    );
    assert_eq!(
        coarser_abstraction_traces(EXTENDED_DEPTH),
        COARSER_ABSTRACTION_TRACES_AT_EXTENDED_DEPTH
    );
}

fn coarser_abstraction_traces(depth: usize) -> usize {
    let initial = model_initial();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for trace in enumerate(&checker_alphabet(), depth) {
        let diagnostics = checker_event_level(&trace);
        if diagnostics.is_empty() {
            continue;
        }
        let translated: Vec<ModelEvent> = trace.iter().map(translate).collect();
        if model_run(&initial, &translated).is_none() {
            continue;
        }
        total += 1;
        let mut kinds: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| {
                format!("{diagnostic:?}")
                    .split(' ')
                    .next()
                    .expect("nonempty")
                    .to_owned()
            })
            .collect();
        kinds.sort();
        kinds.dedup();
        *classes.entry(kinds.join("+")).or_default() += 1;
    }
    for (kind, count) in &classes {
        println!("coarser depth {depth} {kind} {count}");
    }
    assert!(
        !classes.is_empty(),
        "the abstraction must be strictly coarser somewhere"
    );
    total
}

/// The one divergence class the enumerated domain cannot exhibit.
///
/// `Lifetime.destroy` takes `priorAccessesOrdered` as an *input*: the abstract
/// state carries no vector clock. Inside the declared domain that input is
/// never the deciding factor — every checker-accepted `Destroy` there has
/// ordered prior accesses, because a permission holder can only become
/// quiescent by being joined and a join merges its clock into the parent. This
/// witness leaves the domain (one permission instead of two, a guarded access
/// by an unjoined child) to show the class is real: the model, told
/// `prior_accesses_ordered = true`, admits a destruction the checker refuses
/// with `NonquiescentDestruction`.
///
/// This is why the translation `tau` may only carry `true` in the soundness
/// direction — an *accepted* `Destroy` necessarily computed `true` — and never
/// in the converse direction.
#[test]
fn the_vector_clock_ordering_bit_is_outside_the_abstraction() {
    let events = vec![
        ConcurrentEvent::Spawn {
            parent: 0,
            child: 1,
        },
        ConcurrentEvent::Lock {
            task: 1,
            mutex: 6,
            rank: 1,
        },
        ConcurrentEvent::Access {
            task: 1,
            footprint: AccessFootprint::new(RESOURCE, 0, 2),
            kind: AccessKind::Read,
            guarded_by: Some(6),
        },
        ConcurrentEvent::Unlock { task: 1, mutex: 6 },
        ConcurrentEvent::Destroy {
            task: 0,
            resource: RESOURCE,
        },
    ];
    let sole_holder = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 0,
            footprint: AccessFootprint::new(RESOURCE, 0, 4),
            share: 1024,
        }],
        initial_custody: BTreeMap::from([(RESOURCE, 0)]),
        events: events.clone(),
    };
    let diagnostics = qcore_core::concurrency::check_concurrent(&sole_holder)
        .expect_err("the unordered access must refuse the destruction");
    assert!(
        event_level(&diagnostics).contains(&ConcurrencyDiagnostic::NonquiescentDestruction {
            event: 4,
            resource: RESOURCE,
        }),
        "expected NonquiescentDestruction, got {diagnostics:?}"
    );

    // The abstraction of that same program, whose only permission holder is the
    // destroying task, admits it once the ordering bit is supplied as `true`.
    let initial = ModelState {
        known_tasks: vec![0],
        terminal_tasks: Vec::new(),
        destroyed_resources: Vec::new(),
        parents: Vec::new(),
        custody: vec![(RESOURCE, 0)],
        permission_holders: vec![(RESOURCE, 0)],
        guard_holders: Vec::new(),
    };
    let translated: Vec<ModelEvent> = events.iter().map(translate).collect();
    let state = model_run(&initial, &translated)
        .expect("the model has no vector clock and admits the destruction");
    assert_eq!(state.destroyed_resources, vec![RESOURCE]);

    // Supplying the bit the checker actually computed refuses it again, so the
    // divergence is exactly the missing input and not a modelling error.
    let mut honest = translated.clone();
    honest.pop();
    honest.push(ModelEvent::Destroy {
        task: 0,
        resource: RESOURCE,
        prior_accesses_ordered: false,
    });
    assert!(model_run(&initial, &honest).is_none());
}
