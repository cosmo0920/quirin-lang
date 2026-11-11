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

use super::*;

/// A program shaped like the bound in_tail paths (W47-TAIL-PAUSE-BINDINGS
/// §2): the test thread T starts the engine thread E, E dispatches collector
/// callbacks on the engine loop, the input worker I dispatches on its own
/// loop, and the public pause forks on the threaded flag.
struct Shape {
    program: ThreadProgram,
    ids: BTreeMap<&'static str, FunctionId>,
}

impl Shape {
    fn new(functions: &[&'static str]) -> Self {
        let ids = functions
            .iter()
            .enumerate()
            .map(|(index, name)| (*name, index))
            .collect();
        Self {
            program: ThreadProgram {
                functions: functions.iter().map(|name| (*name).to_owned()).collect(),
                ..ThreadProgram::default()
            },
            ids,
        }
    }

    fn id(&self, name: &str) -> FunctionId {
        self.ids[name]
    }

    fn root(&mut self, name: &str, thread: &str) {
        let function = self.id(name);
        self.program.roots.push(ThreadRoot {
            function,
            context: Context {
                thread: thread.to_owned(),
                mode: Mode::Either,
            },
        });
    }

    fn edge(&mut self, caller: &str, callee: &str, hop: HopKind, effect: ModeEffect) {
        let (caller, callee) = (self.id(caller), self.id(callee));
        self.program.edges.push(Edge {
            caller,
            callee,
            hop,
            effect,
        });
    }

    fn call(&mut self, caller: &str, callee: &str) {
        self.edge(caller, callee, HopKind::Direct, ModeEffect::Keep);
    }

    fn spawn(&mut self, spawner: &str, entry: &str) {
        let (spawner, entry) = (self.id(spawner), self.id(entry));
        self.program.spawns.push((spawner, entry));
    }

    fn verdict(&self, site: &str) -> OwnerVerdict {
        let derived = derive(&self.program);
        let owners = Owners::from_dispatchers(
            Some(&derived[self.id("flb_input_collector_fd")]),
            Some(&derived[self.id("input_collector_fd")]),
        );
        owner_verdict(&derived[self.id(site)], &owners)
    }
}

const FUNCTIONS: &[&str] = &[
    "test_main",
    "flb_start",
    "flb_lib_worker",
    "flb_engine_start",
    "flb_input_collector_fd",
    "input_thread",
    "input_collector_fd",
    "handle_input_event",
    "flb_input_pause",
    "flb_input_plugin_pause",
    "flb_input_thread_instance_pause",
    "in_tail_pause",
    "cb_tail_pause_test",
    "wait_tail_collectors_state",
    "flb_input_instance_init",
    "flb_input_thread_instance_init",
];

/// The part both versions of the test share: E and I and their dispatch.
fn engine() -> Shape {
    engine_with(FUNCTIONS)
}

fn engine_with(functions: &[&'static str]) -> Shape {
    let mut shape = Shape::new(functions);
    shape.root("test_main", "T");
    shape.call("test_main", "flb_start");
    shape.spawn("flb_start", "flb_lib_worker");
    shape.call("flb_lib_worker", "flb_engine_start");
    shape.call("flb_engine_start", "flb_input_collector_fd");
    shape.call("flb_engine_start", "flb_input_instance_init");
    shape.edge(
        "flb_input_instance_init",
        "flb_input_thread_instance_init",
        HopKind::Direct,
        ModeEffect::Require(Mode::Threaded),
    );
    shape.spawn("flb_input_thread_instance_init", "input_thread");
    shape.call("input_thread", "input_collector_fd");
    shape.call("input_thread", "handle_input_event");
    shape.call("handle_input_event", "flb_input_plugin_pause");
    shape.edge(
        "flb_input_pause",
        "flb_input_plugin_pause",
        HopKind::Direct,
        ModeEffect::Require(Mode::NonThreaded),
    );
    shape.edge(
        "flb_input_pause",
        "flb_input_thread_instance_pause",
        HopKind::Direct,
        ModeEffect::Require(Mode::Threaded),
    );
    shape.edge(
        "flb_input_plugin_pause",
        "in_tail_pause",
        HopKind::Vtable,
        ModeEffect::Keep,
    );
    shape
}

/// The revised (fixed) non-threaded test: the pause runs in a collector
/// callback that E dispatches on the engine loop (V* followed as a field
/// registration, R as the collector registry, L as the dispatch).
fn fixed() -> Shape {
    let mut shape = engine();
    shape.edge(
        "flb_input_collector_fd",
        "cb_tail_pause_test",
        HopKind::CollectorRegistry,
        ModeEffect::Dispatch(Mode::NonThreaded),
    );
    shape.edge(
        "input_collector_fd",
        "cb_tail_pause_test",
        HopKind::CollectorRegistry,
        ModeEffect::Dispatch(Mode::Threaded),
    );
    shape.call("cb_tail_pause_test", "flb_input_pause");
    shape
}

/// The previous non-threaded test: T pauses the input itself.
fn previous() -> Shape {
    let mut shape = engine();
    shape.call("test_main", "flb_input_pause");
    shape
}

#[test]
fn nc2_the_callback_on_the_engine_loop_discharges_the_owner_obligation() {
    let shape = fixed();
    let verdict = shape.verdict("cb_tail_pause_test");
    let OwnerVerdict::Discharged { contexts } = verdict else {
        panic!("expected a discharge, got {verdict:?}");
    };
    assert!(contexts.contains(&Context {
        thread: "flb_lib_worker".into(),
        mode: Mode::NonThreaded,
    }));
    // The plugin callback it reaches through the NT fork is on E as well, and
    // through the worker's handler on I in the threaded mode.
    assert!(matches!(
        shape.verdict("in_tail_pause"),
        OwnerVerdict::Discharged { .. }
    ));
}

#[test]
fn nc1_a_pause_from_the_test_thread_is_not_discharged() {
    let shape = previous();
    let OwnerVerdict::NotDischarged { context, witness } = shape.verdict("in_tail_pause") else {
        panic!("expected a refusal");
    };
    assert_eq!(context.thread, "T");
    assert_eq!(context.mode, Mode::NonThreaded);
    let names = witness
        .iter()
        .map(|id| shape.program.functions[*id].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "test_main",
            "flb_input_pause",
            "flb_input_plugin_pause",
            "in_tail_pause"
        ]
    );
}

#[test]
fn nc3_a_running_poll_from_the_test_thread_is_not_discharged_whatever_the_mode() {
    let mut shape = fixed();
    shape.call("test_main", "wait_tail_collectors_state");
    // T is neither loop's owner, so the unfixed mode does not matter.
    assert!(matches!(
        shape.verdict("wait_tail_collectors_state"),
        OwnerVerdict::NotDischarged { .. }
    ));
    // The enqueue discharges only "request available", never "applied".
    let completion = control_completion(Mode::Threaded);
    assert!(claim_discharged(ControlClaim::RequestAvailable, completion));
    assert!(!claim_discharged(
        ControlClaim::OperationApplied,
        completion
    ));
    assert!(!claim_discharged(
        ControlClaim::CallbackFinished,
        completion
    ));
    let synchronous = control_completion(Mode::NonThreaded);
    assert!(claim_discharged(
        ControlClaim::CallbackFinished,
        synchronous
    ));
    assert!(!claim_discharged(
        ControlClaim::OperationApplied,
        synchronous
    ));
    assert_eq!(control_completion(Mode::Either), None);
}

#[test]
fn nc6_resume_while_ingestion_is_inactive_refuses_the_running_claim() {
    let owner = fixed().verdict("in_tail_pause");
    let both = [CollectorResult::Succeeded, CollectorResult::Succeeded];
    assert!(matches!(
        pairing(PairDirection::Resume, both, Ingestion::Inactive, &owner),
        PairingClaim::Refused(_)
    ));
    assert!(matches!(
        pairing(PairDirection::Resume, both, Ingestion::Unknown, &owner),
        PairingClaim::NoConclusion(_)
    ));
    assert_eq!(
        pairing(PairDirection::Resume, both, Ingestion::Active, &owner),
        PairingClaim::BothAtBoundary(PairDirection::Resume)
    );
}

#[test]
fn nc7_a_discarded_or_failed_collector_result_gives_no_conclusion() {
    let owner = fixed().verdict("in_tail_pause");
    for results in [
        [CollectorResult::Ignored, CollectorResult::Ignored],
        [CollectorResult::Succeeded, CollectorResult::Failed],
    ] {
        assert!(matches!(
            pairing(PairDirection::Pause, results, Ingestion::Active, &owner),
            PairingClaim::NoConclusion(_)
        ));
    }
    // A refused owner obligation refuses the pairing claim too.
    let refused = previous().verdict("in_tail_pause");
    assert!(matches!(
        pairing(
            PairDirection::Pause,
            [CollectorResult::Succeeded; 2],
            Ingestion::Active,
            &refused
        ),
        PairingClaim::Refused(_)
    ));
}

#[test]
fn an_unresolved_hop_withholds_the_discharge_and_propagates() {
    let mut shape = fixed();
    let escaped = shape.id("flb_input_pause");
    shape
        .program
        .unresolved
        .insert(escaped, BTreeSet::from(["address taken".to_owned()]));
    let OwnerVerdict::Unresolved { reasons } = shape.verdict("in_tail_pause") else {
        panic!("expected unresolved");
    };
    assert!(reasons.contains("address taken"));
    // The callback itself is upstream of the escape and stays discharged.
    assert!(matches!(
        shape.verdict("cb_tail_pause_test"),
        OwnerVerdict::Discharged { .. }
    ));
    // A refusal on a resolved path wins over an unresolved one.
    let mut both = previous();
    let escaped = both.id("flb_input_pause");
    both.program
        .unresolved
        .insert(escaped, BTreeSet::from(["address taken".to_owned()]));
    assert!(matches!(
        both.verdict("in_tail_pause"),
        OwnerVerdict::NotDischarged { .. }
    ));
}

#[test]
fn a_site_nothing_reaches_is_unresolved_not_vacuously_discharged() {
    let shape = engine();
    let OwnerVerdict::Unresolved { reasons } = shape.verdict("cb_tail_pause_test") else {
        panic!("expected unresolved");
    };
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("no derived caller"))
    );
}

#[test]
fn an_owner_that_is_not_a_single_derived_thread_is_unresolved() {
    let mut shape = fixed();
    // A second thread also dispatches the engine loop: no single owner.
    shape.root("flb_input_collector_fd", "other");
    let verdict = shape.verdict("cb_tail_pause_test");
    assert!(
        matches!(
            verdict,
            OwnerVerdict::NotDischarged { .. } | OwnerVerdict::Unresolved { .. }
        ),
        "{verdict:?}"
    );
    assert!(!matches!(verdict, OwnerVerdict::Discharged { .. }));
}

#[test]
fn an_infeasible_threaded_flag_branch_is_pruned() {
    let shape = fixed();
    let derived = derive(&shape.program);
    let enqueue = &derived[shape.id("flb_input_thread_instance_pause")];
    // The callback is dispatched in the non-threaded mode on E and in the
    // threaded mode on I; only the latter reaches the enqueue branch.
    assert!(
        enqueue
            .contexts
            .iter()
            .all(|context| context.mode == Mode::Threaded)
    );
    assert!(
        enqueue
            .contexts
            .iter()
            .all(|context| context.thread != "flb_lib_worker" || context.mode == Mode::Threaded)
    );
}

#[test]
fn the_reactor_refuses_exactly_the_foreign_accesses() {
    let owner = "E".to_owned();
    let fixed = [
        (owner.clone(), LoopOperation::Mutate),
        (owner.clone(), LoopOperation::Read),
        (owner.clone(), LoopOperation::Mutate),
    ];
    assert_eq!(check_through_reactor(&owner, &fixed), Ok(()));
    let previous = [
        ("T".to_owned(), LoopOperation::Mutate),
        ("T".to_owned(), LoopOperation::Read),
    ];
    let diagnostics = check_through_reactor(&owner, &previous).unwrap_err();
    assert!(
        diagnostics
            .iter()
            .any(|d| matches!(d, EventDiagnostic::L05ConflictingAccess { .. }))
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| matches!(d, EventDiagnostic::L04ThreadInitialization { .. }))
    );
    // A foreign read alone still conflicts with the dispatcher's own walk.
    let poll = [("T".to_owned(), LoopOperation::Read)];
    let diagnostics = check_through_reactor(&owner, &poll).unwrap_err();
    assert!(
        diagnostics
            .iter()
            .all(|d| matches!(d, EventDiagnostic::L05ConflictingAccess { .. }))
    );
}

/// Hop C (card W47-LOOP-OWNER-COROUTINE-HOP): the input coroutine's entry,
/// resumed by the loop dispatchers, dispatches a collector body.
fn coroutine() -> Shape {
    let mut functions = FUNCTIONS.to_vec();
    functions.extend([
        "input_pre_cb_collect",
        "cb_coro",
        "net_write",
        "other_entry",
    ]);
    let mut shape = engine_with(&functions);
    shape.edge(
        "flb_input_collector_fd",
        "input_pre_cb_collect",
        HopKind::Coroutine,
        ModeEffect::Dispatch(Mode::NonThreaded),
    );
    shape.edge(
        "input_collector_fd",
        "input_pre_cb_collect",
        HopKind::Coroutine,
        ModeEffect::Dispatch(Mode::Threaded),
    );
    shape.edge(
        "input_pre_cb_collect",
        "cb_coro",
        HopKind::CollectorRegistry,
        ModeEffect::Keep,
    );
    let (entry, body) = (shape.id("input_pre_cb_collect"), shape.id("cb_coro"));
    shape.program.coroutine_bodies.push((entry, body));
    let yielding = shape.id("net_write");
    shape
        .program
        .yields
        .insert(yielding, "`flb_coro_yield` at src/flb_io.c:487".to_owned());
    shape
}

#[test]
fn a_coroutine_resumed_only_on_the_owners_runs_on_the_owners() {
    let shape = coroutine();
    let OwnerVerdict::Discharged { contexts } = shape.verdict("cb_coro") else {
        panic!("expected discharged");
    };
    assert_eq!(
        contexts,
        BTreeSet::from([
            Context {
                thread: "flb_lib_worker".to_owned(),
                mode: Mode::NonThreaded,
            },
            Context {
                thread: "input_thread".to_owned(),
                mode: Mode::Threaded,
            },
        ])
    );
}

#[test]
fn a_coroutine_resumed_from_a_non_owner_thread_refuses() {
    let mut shape = coroutine();
    shape.edge(
        "test_main",
        "input_pre_cb_collect",
        HopKind::Coroutine,
        ModeEffect::Keep,
    );
    let OwnerVerdict::NotDischarged { context, .. } = shape.verdict("cb_coro") else {
        panic!("expected a refusal");
    };
    assert_eq!(context.thread, "T");
}

#[test]
fn a_coroutine_body_that_may_yield_has_an_underived_resumer() {
    let mut shape = coroutine();
    shape.call("cb_coro", "net_write");
    let OwnerVerdict::Unresolved { reasons } = shape.verdict("cb_coro") else {
        panic!("expected unresolved");
    };
    assert!(reasons.contains(&underived_resumer(
        "input_pre_cb_collect",
        "cb_coro",
        "`flb_coro_yield` at src/flb_io.c:487"
    )));
    // The same callee dispatched only directly (not a coroutine body) does
    // not depend on the hop.
    shape.program.coroutine_bodies.clear();
    assert!(matches!(
        shape.verdict("cb_coro"),
        OwnerVerdict::Discharged { .. }
    ));
}

#[test]
fn a_yield_inside_another_coroutine_is_not_this_bodys() {
    let mut shape = coroutine();
    shape.edge(
        "cb_coro",
        "other_entry",
        HopKind::Coroutine,
        ModeEffect::Keep,
    );
    shape.call("other_entry", "net_write");
    assert!(matches!(
        shape.verdict("cb_coro"),
        OwnerVerdict::Discharged { .. }
    ));
}

#[test]
fn an_unresolved_resumer_never_discharges_the_coroutine() {
    let mut shape = coroutine();
    let entry = shape.id("input_pre_cb_collect");
    shape.program.unresolved.insert(
        entry,
        BTreeSet::from(["a handle not derived from the constructor".to_owned()]),
    );
    assert!(matches!(
        shape.verdict("cb_coro"),
        OwnerVerdict::Unresolved { .. }
    ));
}
