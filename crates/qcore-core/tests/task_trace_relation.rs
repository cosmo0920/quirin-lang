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

//! F02 — the Rust side of one source-to-native trace relation.
//!
//! Three declared finite domains are enumerated here and, trace for trace,
//! by the Lean transcription in `proof/QCore/TaskTraceRelation.lean`:
//!
//! * **D1** — the `7` schedules of the owned round trip: every ordering of its
//!   facts that preserves each actor's own program order.
//! * **D2** — the `C(11, 2) = 55` schedules of the concurrent round trip. It
//!   measures schedule freedom: how many *distinct total orders* the relation
//!   admits for one run.
//! * **D3** — every single transposition of every D2 schedule, `55 * 55 = 3025`
//!   traces. It measures how much of the surrounding order is genuinely
//!   constrained rather than incidental.
//!
//! Every domain size, trace digest and verdict digest below is recomputed
//! independently by the Lean kernel. The per-class census and the `5040`-trace
//! extension are Rust-only, and the report says so.

use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, TraceRelationViolation,
    admitted_linearizations, parse_observed_trace, relate,
};

const ROUND_TRIP: &str = include_str!("fixtures/f02/round_trip.trace");
const CONCURRENT: &str = include_str!("fixtures/f02/concurrent_round_trip.trace");

// Domain sizes, pinned on both sides.
const D1_SIZE: usize = 7;
const D2_SIZE: usize = 55;
const D3_SIZE: usize = D2_SIZE * D2_SIZE;
/// Rust-only extension: every ordering of the round trip's seven facts.
const EXTENSION_SIZE: usize = 5040;

fn events(text: &str) -> Vec<ObservedEvent> {
    parse_observed_trace(text).unwrap()
}
fn order(text: &str) -> AdmittedCausalOrder {
    AdmittedCausalOrder::declare(events(text)).unwrap()
}
fn verdict(order: &AdmittedCausalOrder, trace: &[ObservedEvent]) -> u32 {
    match relate(order, trace) {
        Ok(_) => 0,
        Err(violation) => violation.code(),
    }
}

/* ------------------------------------------------------------------ digests */

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;
const DOMAIN_SEED: u64 = 14695981039346656037;
const DOMAIN_MUL: u64 = 1099511628211;

fn mix(accumulator: u64, value: u64) -> u64 {
    accumulator
        .wrapping_mul(TRACE_MUL)
        .wrapping_add(value)
        .wrapping_add(1)
}

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("the declared domains carry no negative payload")
}

fn encode_outcome(outcome: &ObservedOutcome) -> u64 {
    match outcome {
        ObservedOutcome::Unit => 0,
        ObservedOutcome::I32(value) => 1 + 4 * natural(*value),
        ObservedOutcome::Buffer(buffer) => 2 + 4 * buffer,
        ObservedOutcome::Error(_) => {
            panic!("typed error outcomes are outside the declared F02 domains")
        }
        // Card `W48-QR-SCALAR-TYPES-R6`: the grown alphabet's fixed-width
        // outcome, which no declared domain of this leg carries.
        ObservedOutcome::Fixed { .. } => {
            panic!("fixed-width outcomes are outside the declared F02 domains")
        }
    }
}

/// Injective for the identifiers the declared domains use (tasks below 8,
/// generations below 64, payloads below 2^32). Mirrored by `encodeEvent`.
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
        ObservedEvent::Completed { task, outcome } => 9 + 16 * (task + 8 * encode_outcome(outcome)),
        ObservedEvent::Joined { child } => 10 + 16 * child,
    }
}

fn mix_trace(trace: &[ObservedEvent]) -> u64 {
    trace.iter().fold(TRACE_SEED, |accumulator, event| {
        mix(accumulator, encode_event(event))
    })
}

fn trace_digest(traces: &[Vec<ObservedEvent>]) -> u64 {
    traces.iter().fold(DOMAIN_SEED, |accumulator, trace| {
        accumulator
            .wrapping_mul(DOMAIN_MUL)
            .wrapping_add(mix_trace(trace))
            .wrapping_add(1)
    })
}

fn verdict_digest(order: &AdmittedCausalOrder, traces: &[Vec<ObservedEvent>]) -> u64 {
    traces.iter().fold(DOMAIN_SEED, |accumulator, trace| {
        accumulator
            .wrapping_mul(DOMAIN_MUL)
            .wrapping_add(u64::from(verdict(order, trace)))
            .wrapping_add(1)
    })
}

/* ------------------------------------------------------------------ domains */

/// The `rank`-th permutation of `0..n` in Lehmer (factorial-number-system)
/// order. Mirrored by `permutation` in Lean, so both sides enumerate the same
/// domain in the same order.
fn permutation(n: usize, rank: usize) -> Vec<usize> {
    let mut pool: Vec<usize> = (0..n).collect();
    let mut selected = Vec::with_capacity(n);
    let mut rank = rank;
    let mut factorial: usize = (1..=n).product();
    for remaining in (1..=n).rev() {
        factorial /= remaining;
        let index = rank / factorial;
        rank %= factorial;
        selected.push(pool.remove(index));
    }
    selected
}

fn apply(trace: &[ObservedEvent], indices: &[usize]) -> Vec<ObservedEvent> {
    indices.iter().map(|&index| trace[index].clone()).collect()
}

/// Every ordering of one run's facts that preserves each actor's own program
/// order — that is, every *schedule* a different observer of the same execution
/// could legitimately have rendered. Mirrored by `schedules` in Lean.
fn schedules(reference: &[ObservedEvent], slot_sets: &[Vec<usize>]) -> Vec<Vec<ObservedEvent>> {
    let parent: Vec<ObservedEvent> = reference
        .iter()
        .filter(|event| event.actor() == Some(0))
        .cloned()
        .collect();
    let child: Vec<ObservedEvent> = reference
        .iter()
        .filter(|event| event.actor() == Some(1))
        .cloned()
        .collect();
    slot_sets
        .iter()
        .map(|slots| {
            let mut parents = parent.iter();
            let mut children = child.iter();
            (0..reference.len())
                .map(|position| {
                    if slots.contains(&position) {
                        children.next().unwrap().clone()
                    } else {
                        parents.next().unwrap().clone()
                    }
                })
                .collect()
        })
        .collect()
}

fn pairs(bound: usize) -> Vec<Vec<usize>> {
    (0..bound)
        .flat_map(|first| (first + 1..bound).map(move |second| vec![first, second]))
        .collect()
}

fn domain_one() -> Vec<Vec<ObservedEvent>> {
    let slots: Vec<Vec<usize>> = (0..7).map(|position| vec![position]).collect();
    schedules(&events(ROUND_TRIP), &slots)
}

fn domain_two() -> Vec<Vec<ObservedEvent>> {
    schedules(&events(CONCURRENT), &pairs(11))
}

/// The Rust-only extension: every one of the `7! = 5040` orderings of the round
/// trip's facts, schedule-preserving or not. It is outside the Lean-kernel
/// domain for cost, exactly as recorded in the report.
fn extended_domain() -> Vec<Vec<ObservedEvent>> {
    let base = events(ROUND_TRIP);
    (0..EXTENSION_SIZE)
        .map(|rank| apply(&base, &permutation(base.len(), rank)))
        .collect()
}

fn domain_three() -> Vec<Vec<ObservedEvent>> {
    let mut traces = Vec::new();
    for interleaving in domain_two() {
        for first in 0..interleaving.len() {
            for second in first + 1..interleaving.len() {
                let mut mutated = interleaving.clone();
                mutated.swap(first, second);
                traces.push(mutated);
            }
        }
    }
    traces
}

fn census(order: &AdmittedCausalOrder, traces: &[Vec<ObservedEvent>]) -> Vec<(u32, usize)> {
    let mut counts: std::collections::BTreeMap<u32, usize> = Default::default();
    for trace in traces {
        *counts.entry(verdict(order, trace)).or_default() += 1;
    }
    counts.into_iter().collect()
}

/* ------------------------------------------------------------- the relation */

#[test]
fn both_reference_orders_relate_to_themselves() {
    for text in [ROUND_TRIP, CONCURRENT] {
        let order = order(text);
        let linearization = relate(&order, order.events()).unwrap();
        assert!(linearization.is_reference_order());
    }
}

#[test]
fn the_declared_domains_have_their_pinned_shape_and_digest() {
    let one = domain_one();
    let two = domain_two();
    let three = domain_three();
    assert_eq!(one.len(), D1_SIZE);
    assert_eq!(two.len(), D2_SIZE);
    assert_eq!(three.len(), D3_SIZE);
    assert_eq!(trace_digest(&one), 1827949402977241471);
    assert_eq!(trace_digest(&two), 7621727749705747350);
    assert_eq!(trace_digest(&three), 9380586544780836206);
    let extension = extended_domain();
    assert_eq!(extension.len(), EXTENSION_SIZE);
    assert_eq!(trace_digest(&extension), 13005161553597186085);
}

#[test]
fn the_round_trip_admits_exactly_one_total_order() {
    let order = order(ROUND_TRIP);
    let domain = domain_one();
    let admitted: Vec<&Vec<ObservedEvent>> = domain
        .iter()
        .filter(|trace| relate(&order, trace).is_ok())
        .collect();
    // Every causal constraint of the owned round trip happens to determine a
    // unique schedule: the reference order is the only admitted one. That is a
    // measurement about this program, not a property of the relation, and it is
    // exactly why byte-equal stdout was an adequate check for CT01.
    assert_eq!(admitted.len(), 1);
    assert_eq!(admitted[0], order.events());
    assert_eq!(verdict_digest(&order, &domain), 6242011289262393457);
    assert_eq!(
        census(&order, &domain),
        vec![(0, 1), (7, 1), (10, 2), (14, 1), (17, 2)]
    );
}

/// The Rust-only extension. Only `C(7, 1) = 7` of the 5040 orderings preserve
/// the per-actor program order, so class `4` dominates; the schedule domain
/// above is exactly the seven that survive it.
#[test]
fn no_ordering_of_the_round_trip_outside_its_one_schedule_is_admitted() {
    let order = order(ROUND_TRIP);
    let domain = extended_domain();
    assert_eq!(
        domain
            .iter()
            .filter(|trace| relate(&order, trace).is_ok())
            .count(),
        1
    );
    assert_eq!(verdict_digest(&order, &domain), 6228814275926525894);
    assert_eq!(
        census(&order, &domain),
        vec![(0, 1), (4, 5033), (7, 1), (10, 2), (14, 1), (17, 2)]
    );
}

#[test]
fn the_concurrent_round_trip_admits_six_distinct_total_orders() {
    let order = order(CONCURRENT);
    let domain = domain_two();
    let admitted: Vec<&Vec<ObservedEvent>> = domain
        .iter()
        .filter(|trace| relate(&order, trace).is_ok())
        .collect();
    assert_eq!(admitted.len(), 6);
    // The relation is not byte equality: it admits six renderings of this one
    // run, of which the shipped backend produces exactly one.
    assert!(admitted.iter().any(|trace| *trace == order.events()));
    assert_eq!(
        admitted
            .iter()
            .filter(|trace| **trace == order.events())
            .count(),
        1
    );
    assert_eq!(verdict_digest(&order, &domain), 3851450914538809003);
    assert_eq!(
        census(&order, &domain),
        vec![(0, 6), (7, 10), (10, 17), (14, 13), (17, 9)]
    );
}

#[test]
fn the_bounded_enumerator_finds_the_same_six_schedules() {
    let order = order(CONCURRENT);
    let found = admitted_linearizations(&order, 1_000_000).unwrap();
    let mut expected: Vec<Vec<ObservedEvent>> = domain_two()
        .into_iter()
        .filter(|trace| relate(&order, trace).is_ok())
        .collect();
    let mut found_sorted = found.clone();
    found_sorted.sort();
    expected.sort();
    assert_eq!(found_sorted, expected);
    assert_eq!(found.len(), 6);
    // The bound is honoured rather than silently exceeded.
    assert!(admitted_linearizations(&order, 8).is_none());
}

#[test]
fn single_transpositions_of_admitted_schedules_are_censused() {
    let order = order(CONCURRENT);
    let domain = domain_three();
    assert_eq!(verdict_digest(&order, &domain), 15391469991380593011);
    // Eighteen of the 3025 perturbations land on another admitted schedule, so
    // the relation is genuinely a partial order and not the reference total
    // order under another name.
    assert_eq!(
        census(&order, &domain),
        vec![(0, 18), (4, 2845), (7, 27), (10, 55), (14, 48), (17, 32)]
    );
}

#[test]
fn the_admitted_order_reports_its_own_edges() {
    assert_eq!(order(ROUND_TRIP).edges().len(), 16);
    assert_eq!(order(CONCURRENT).edges().len(), 31);
}

/* ------------------------------------------------------------ named refusals */

fn concurrent_with(mutate: impl Fn(&mut Vec<ObservedEvent>)) -> Vec<ObservedEvent> {
    let mut trace = events(CONCURRENT);
    mutate(&mut trace);
    trace
}

#[test]
fn a_child_fact_after_its_join_is_refused() {
    let order = order(CONCURRENT);
    let trace = concurrent_with(|trace| {
        let completion = trace.remove(6);
        trace.insert(7, completion);
    });
    assert_eq!(
        relate(&order, &trace),
        Err(TraceRelationViolation::CompletionOrder { child: 1 })
    );
    // A child fact strictly after its own join breaks that child's program
    // order too, so it is decided where the join conjunct owns it.
    let trace = concurrent_with(|trace| {
        let read = trace.remove(5);
        trace.insert(7, read);
    });
    assert_eq!(
        AdmittedCausalOrder::declare(trace),
        Err(TraceRelationViolation::ActivityAfterJoin { child: 1 })
    );
    let _ = &order;
}

#[test]
fn a_child_fact_before_its_creation_is_refused() {
    let order = order(CONCURRENT);
    let trace = concurrent_with(|trace| {
        let read = trace.remove(5);
        trace.insert(1, read);
    });
    assert_eq!(
        relate(&order, &trace),
        Err(TraceRelationViolation::CreationOrder { child: 1 })
    );
}

// Reordering two facts of one actor is caught by the per-actor conjunct (R3)
// before any resource conjunct can fire, so the resource conjuncts are
// exercised where they are decided: on a candidate reference order.
#[test]
fn a_use_after_release_is_refused() {
    let trace = concurrent_with(|trace| trace.swap(3, 4));
    assert_eq!(
        AdmittedCausalOrder::declare(trace),
        Err(TraceRelationViolation::ReleaseOrder { buffer: 2 })
    );
}

#[test]
fn a_use_before_allocation_is_refused() {
    let trace = concurrent_with(|trace| {
        let allocation = trace.remove(0);
        trace.insert(6, allocation);
    });
    assert_eq!(
        AdmittedCausalOrder::declare(trace),
        Err(TraceRelationViolation::AllocationOrder { buffer: 1 })
    );
}

#[test]
fn an_actor_reordering_its_own_facts_is_refused() {
    let order = order(CONCURRENT);
    let trace = concurrent_with(|trace| trace.swap(8, 9));
    assert_eq!(
        relate(&order, &trace),
        Err(TraceRelationViolation::ActorOrder { actor: 0 })
    );
}

#[test]
fn a_missing_or_extra_fact_is_refused_rather_than_normalised() {
    let order = order(CONCURRENT);
    let short = concurrent_with(|trace| {
        trace.remove(9);
    });
    assert_eq!(
        relate(&order, &short),
        Err(TraceRelationViolation::EventMultisetMismatch)
    );
    let renamed = concurrent_with(|trace| {
        trace[9] = ObservedEvent::Dropped { task: 0, buffer: 3 };
    });
    assert_eq!(
        relate(&order, &renamed),
        Err(TraceRelationViolation::EventMultisetMismatch)
    );
}

#[test]
fn leaked_and_unjoined_runs_cannot_become_admitted_orders() {
    let leaked = concurrent_with(|trace| {
        trace.remove(9);
    });
    assert_eq!(
        AdmittedCausalOrder::declare(leaked),
        Err(TraceRelationViolation::UnreleasedBuffer { buffer: 1 })
    );
    let unjoined = concurrent_with(|trace| {
        trace.remove(7);
    });
    assert_eq!(
        AdmittedCausalOrder::declare(unjoined),
        Err(TraceRelationViolation::UnjoinedChild { child: 1 })
    );
    let trailing = concurrent_with(|trace| trace.swap(9, 10));
    assert_eq!(
        AdmittedCausalOrder::declare(trailing),
        Err(TraceRelationViolation::RootResultNotFinal)
    );
}

#[test]
fn an_unattributable_or_ambiguous_observation_is_refused() {
    let order = order(CONCURRENT);
    let unattributable = concurrent_with(|trace| {
        trace[3] = ObservedEvent::CreationFailed { worker: 0 };
    });
    assert_eq!(
        relate(&order, &unattributable),
        Err(TraceRelationViolation::UnattributableEvent { index: 3 })
    );
    let ambiguous = concurrent_with(|trace| trace[8] = trace[5].clone());
    assert_eq!(
        relate(&order, &ambiguous),
        Err(TraceRelationViolation::AmbiguousOccurrence { index: 8 })
    );
}

#[test]
fn a_read_that_reports_the_wrong_payload_is_refused() {
    let mut trace = events(CONCURRENT);
    trace[8] = ObservedEvent::Read {
        task: 0,
        buffer: 1,
        value: 43,
    };
    assert_eq!(
        AdmittedCausalOrder::declare(trace),
        Err(TraceRelationViolation::PayloadMismatch { buffer: 1 })
    );
}

/* ---------------------------------------------------------------- the parser */

#[test]
fn the_observer_reads_the_whole_line_or_refuses_it() {
    assert!(parse_observed_trace("QCORE_TASK_JOIN 1\n").is_ok());
    for line in [
        "QCORE_TASK_JOIN",
        "QCORE_TASK_JOIN 1 2",
        "QCORE_TASK_JOIN -1",
        "QCORE_TASK_RESULT 0 FLOAT 1",
        "QCORE_BUFFER_ALLOC 0 1",
        "warning: something else entirely",
    ] {
        let error = parse_observed_trace(line).unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(error.text, line);
    }
}

#[test]
fn every_alphabet_line_round_trips_through_the_observer() {
    let text = "QCORE_BUFFER_ALLOC 0 1 42\nQCORE_BUFFER_ALLOC_FAILED 3\nQCORE_BUFFER_READ 1 1 42\n\
                QCORE_BUFFER_WRITE 0 1 7\n\
                QCORE_BUFFER_DROP 0 1\nQCORE_BUFFER_LOAN 2 1\nQCORE_BUFFER_RETURN 2 1\n\
                QCORE_TASK_SPAWN 1 0\nQCORE_TASK_CREATE_FAILED 4\nQCORE_TASK_JOIN 1\n\
                QCORE_TASK_RESULT 0 UNIT\nQCORE_TASK_RESULT 0 I32 -7\nQCORE_TASK_RESULT 0 BUFFER 9\n\
                QCORE_TASK_RESULT 0 ERROR AllocError\n";
    let parsed = parse_observed_trace(text).unwrap();
    assert_eq!(parsed.len(), 14);
    assert_eq!(
        parsed[3],
        ObservedEvent::Written {
            task: 0,
            buffer: 1,
            value: 7
        }
    );
    assert_eq!(
        parsed[13],
        ObservedEvent::Completed {
            task: 0,
            outcome: ObservedOutcome::Error("AllocError".into())
        }
    );
    assert_eq!(parsed[8].actor(), None);
}

/* --------------------------- the write-aware payload rule (CT03-PROOF-ALPHABET)

A generation that is never written keeps exactly the pinned rule (the tests
above measure that: every domain digest and census is unchanged); a written
generation's reads must each report the value established by the nearest
preceding allocation or write. ------------------------------------------- */

fn written(task: u64, buffer: u64, value: i32) -> ObservedEvent {
    ObservedEvent::Written {
        task,
        buffer,
        value,
    }
}

#[test]
fn a_written_generation_admits_reads_of_the_established_value() {
    let trace = parse_observed_trace(
        "QCORE_BUFFER_ALLOC 0 1 1\nQCORE_BUFFER_WRITE 0 1 42\nQCORE_BUFFER_READ 0 1 42\n\
         QCORE_BUFFER_WRITE 0 1 7\nQCORE_BUFFER_READ 0 1 7\nQCORE_BUFFER_DROP 0 1\n\
         QCORE_TASK_RESULT 0 I32 7\n",
    )
    .unwrap();
    let order = AdmittedCausalOrder::declare(trace.clone()).unwrap();
    assert!(relate(&order, &trace).unwrap().is_reference_order());
}

#[test]
fn a_stale_read_after_a_write_is_refused() {
    // The read reports the allocation's initial payload, not the value the
    // preceding write established.
    let trace = vec![
        ObservedEvent::Allocated {
            task: 0,
            buffer: 1,
            payload: 1,
        },
        written(0, 1, 42),
        ObservedEvent::Read {
            task: 0,
            buffer: 1,
            value: 1,
        },
        ObservedEvent::Dropped { task: 0, buffer: 1 },
        ObservedEvent::Completed {
            task: 0,
            outcome: ObservedOutcome::I32(1),
        },
    ];
    assert_eq!(
        AdmittedCausalOrder::declare(trace),
        Err(TraceRelationViolation::PayloadMismatch { buffer: 1 })
    );
}

#[test]
fn a_write_outside_the_generations_lifetime_is_refused() {
    // A write is a mention: after the release it is a use-after-release,
    // before the allocation it is a use-before-allocation.
    let after_release = vec![
        ObservedEvent::Allocated {
            task: 0,
            buffer: 1,
            payload: 1,
        },
        ObservedEvent::Dropped { task: 0, buffer: 1 },
        written(0, 1, 42),
        ObservedEvent::Completed {
            task: 0,
            outcome: ObservedOutcome::I32(0),
        },
    ];
    assert_eq!(
        AdmittedCausalOrder::declare(after_release),
        Err(TraceRelationViolation::ReleaseOrder { buffer: 1 })
    );
    let before_allocation = vec![
        written(0, 1, 42),
        ObservedEvent::Allocated {
            task: 0,
            buffer: 1,
            payload: 1,
        },
        ObservedEvent::Dropped { task: 0, buffer: 1 },
        ObservedEvent::Completed {
            task: 0,
            outcome: ObservedOutcome::I32(0),
        },
    ];
    assert_eq!(
        AdmittedCausalOrder::declare(before_allocation),
        Err(TraceRelationViolation::AllocationOrder { buffer: 1 })
    );
}

#[test]
fn a_rendering_with_a_write_against_a_write_free_order_is_refused() {
    // Conservativity's observable face: no pre-existing admitted order gains
    // a rendering — a write is a fact the write-free reference never
    // reported, so it refuses as a multiset mismatch.
    let order = order(ROUND_TRIP);
    let mut trace = events(ROUND_TRIP);
    trace[4] = written(0, 1, 42);
    assert_eq!(
        relate(&order, &trace),
        Err(TraceRelationViolation::EventMultisetMismatch)
    );
}
