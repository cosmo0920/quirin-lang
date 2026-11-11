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
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{SourceResult, import_source_functions_from_profile};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use std::path::PathBuf;

use crate::timeout_teardown::{TimeoutTeardownContracts, project_cb_timeout_source};

const CALLBACKS_C: &str = include_str!("../../../../compiler/tests/fixtures/monkey/callbacks.c");
const WRAPPERS_C: &str = include_str!("../../../../compiler/tests/fixtures/monkey/wrappers.c");
const LIBEVENT_EVENT_C: &[u8] = include_bytes!("../../../../third_party/libevent/event.c");
const LIBEVENT_REVISION: &str = "d82464a277d0f42703702c4dfd9af6af38595a83";

fn import(source: &str, role: ImportRole) -> Option<Vec<SourceSummary>> {
    if qcore_c_gateway::clang_command().is_err() {
        return None;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
    let source = source.replace("\r\n", "\n");
    let results = import_source_functions_from_profile(
        &C11TranslationUnit {
            source: &source,
            // Replay the Linux call sequences pinned by the teardown policy.
            definitions: vec!["QCORE_TEST_LINUX_LP64=1".into()],
            ..Default::default()
        },
        monkey_source_revision(),
        &profile,
        "monkey",
        role,
    )
    .unwrap();
    Some(
        results
            .into_iter()
            .filter_map(|result| match result {
                SourceResult::Summary(summary) => Some(summary),
                SourceResult::Unknown { .. } => None,
            })
            .collect(),
    )
}

fn named<'a>(summaries: &'a [SourceSummary], name: &str) -> &'a SourceSummary {
    summaries
        .iter()
        .find(|summary| summary.function() == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

struct Fixture {
    callbacks: Vec<SourceSummary>,
    wrappers: Vec<SourceSummary>,
}

impl Fixture {
    fn load() -> Option<Self> {
        Some(Self {
            callbacks: import(CALLBACKS_C, ImportRole::Callback)?,
            wrappers: import(WRAPPERS_C, ImportRole::Wrapper)?,
        })
    }

    fn callback(&self, name: &str) -> &SourceSummary {
        named(&self.callbacks, name)
    }

    fn wrapper(&self, name: &str) -> &SourceSummary {
        named(&self.wrappers, name)
    }

    fn profile(&self) -> String {
        self.callback("cb_event").profile_assumptions()["profile_digest"].clone()
    }

    fn timeout_projection(&self) -> TimeoutTeardownSourceProjection {
        project_cb_timeout_source(
            self.callback("cb_timeout"),
            &TimeoutTeardownContracts::parse().unwrap(),
            LIBEVENT_REVISION,
            LIBEVENT_EVENT_C,
        )
        .unwrap()
    }

    fn aggregate(&self) -> AggregateFacts {
        AggregateFacts {
            verdict: Cycle18AggregateVerdict::Unknown,
            source_revision: monkey_source_revision().into(),
            profile_sha256: self.profile(),
            callback_source_sha256: CALLBACKS
                .iter()
                .map(|(name, _)| ((*name).to_owned(), summary_sha256(self.callback(name))))
                .collect(),
            joined: CALLBACKS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect(),
            refused: BTreeMap::new(),
            provisional_unsafe: BTreeSet::from([
                "base_creation_failure".into(),
                "fired_allocation_failure".into(),
                "timer_creation_failure".into(),
            ]),
            unresolved: Cycle18Obligation::ALL
                .iter()
                .map(|obligation| (obligation.name().to_owned(), "aggregate text".to_owned()))
                .collect(),
            policy_sha256: "0".repeat(64),
            inventory_sha256: "1".repeat(64),
        }
    }
}

fn with_variant(source: &str, from: &str, to: &str) -> String {
    let source = source.replace("\r\n", "\n");
    assert!(source.contains(from), "variant anchor `{from}` is missing");
    source.replacen(from, to, 1)
}

#[cfg(windows)]
#[test]
fn windows_timeout_summary_refuses_the_linux_composition_pin() {
    let fixture = Fixture::load().unwrap();
    assert!(matches!(
        project_cb_timeout_source(
            fixture.callback("cb_timeout"),
            &TimeoutTeardownContracts::parse().unwrap(),
            LIBEVENT_REVISION,
            LIBEVENT_EVENT_C,
        ),
        Err(crate::timeout_teardown::TimeoutTeardownError::Source(reason))
            if reason.contains("source_summary_digest")
    ));
}

// --- initialization reachability ------------------------------------------

#[test]
fn constant_initialization_refutes_exactly_the_failure_branch() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let discharge = discharge_initialization_reachability(
        fixture.wrapper("_mk_event_init"),
        fixture.wrapper("_mk_event_loop_create"),
        &fixture.profile(),
    )
    .unwrap();
    assert_eq!(discharge.constant_result, 0);
    assert_eq!(discharge.loop_create_paths, 4);
    assert_eq!(discharge.initialization_failure_paths, BTreeSet::from([0]));
    assert_eq!(discharge.infeasible_paths, BTreeSet::from([0]));
    assert_eq!(discharge.feasible_paths, BTreeSet::from([1, 2, 3]));
}

#[test]
fn an_initialization_failure_return_keeps_reachability_open() {
    let source = with_variant(
        WRAPPERS_C,
        "#endif\n\n    return 0;\n}",
        "#endif\n    if (mk_event_probe()) {\n        return -1;\n    }\n\n    return 0;\n}",
    );
    let source = with_variant(
        &source,
        "static inline int _mk_event_init()",
        "int mk_event_probe(void);\nstatic inline int _mk_event_init()",
    );
    let Some(wrappers) = import(&source, ImportRole::Wrapper) else {
        return;
    };
    let profile =
        named(&wrappers, "_mk_event_init").profile_assumptions()["profile_digest"].clone();
    let error = discharge_initialization_reachability(
        named(&wrappers, "_mk_event_init"),
        named(&wrappers, "_mk_event_loop_create"),
        &profile,
    )
    .unwrap_err();
    assert!(error.contains("constant"), "{error}");
}

#[test]
fn a_nonliteral_initialization_result_keeps_reachability_open() {
    let source = with_variant(
        WRAPPERS_C,
        "#endif\n\n    return 0;\n}",
        "#endif\n\n    return mk_event_ready();\n}",
    );
    let source = with_variant(
        &source,
        "static inline int _mk_event_init()",
        "int mk_event_ready(void);\nstatic inline int _mk_event_init()",
    );
    let Some(wrappers) = import(&source, ImportRole::Wrapper) else {
        return;
    };
    let profile =
        named(&wrappers, "_mk_event_init").profile_assumptions()["profile_digest"].clone();
    assert!(
        discharge_initialization_reachability(
            named(&wrappers, "_mk_event_init"),
            named(&wrappers, "_mk_event_loop_create"),
            &profile,
        )
        .is_err()
    );
}

#[test]
fn a_guard_the_constant_satisfies_does_not_refute_the_failure_branch() {
    let source = with_variant(
        WRAPPERS_C,
        "if (_mk_event_init() != 0) {",
        "if (_mk_event_init() != 1) {",
    );
    let Some(wrappers) = import(&source, ImportRole::Wrapper) else {
        return;
    };
    let profile =
        named(&wrappers, "_mk_event_init").profile_assumptions()["profile_digest"].clone();
    let error = discharge_initialization_reachability(
        named(&wrappers, "_mk_event_init"),
        named(&wrappers, "_mk_event_loop_create"),
        &profile,
    )
    .unwrap_err();
    assert!(error.contains("not refuted"), "{error}");
}

#[test]
fn initialization_identity_must_match_the_aggregate_profile_and_roles() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let init = fixture.wrapper("_mk_event_init");
    let create = fixture.wrapper("_mk_event_loop_create");
    assert!(discharge_initialization_reachability(init, create, &"f".repeat(64)).is_err());
    assert!(discharge_initialization_reachability(create, init, &fixture.profile()).is_err());
    assert!(
        discharge_initialization_reachability(
            fixture.callback("cb_wait_2_timeout"),
            create,
            &fixture.profile()
        )
        .is_err()
    );
}

// --- activation flags -----------------------------------------------------

#[test]
fn activation_flags_are_read_only_by_the_cb_event_bit_guards() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    for name in ["cb_timeout", "cb_wait_2_timeout"] {
        assert_eq!(
            activation_flag_dependence(fixture.callback(name)).unwrap(),
            ActivationFlagDependence::Independent,
            "{name}"
        );
    }
    let ActivationFlagDependence::BitGuarded {
        tested_bits,
        path_classes,
        exhaustive,
    } = activation_flag_dependence(fixture.callback("cb_event")).unwrap()
    else {
        panic!("cb_event reads its activation flags");
    };
    assert_eq!(tested_bits, BTreeSet::from([2, 4]));
    assert_eq!(path_classes.len(), 4);
    assert!(exhaustive);
}

#[test]
fn flags_flowing_into_memory_or_multi_bit_guards_refuse() {
    let stored = with_variant(
        CALLBACKS_C,
        "fired->mask = mask;",
        "fired->mask = mask | flags;",
    );
    let multi = with_variant(
        CALLBACKS_C,
        "if (flags & EV_WRITE) {",
        "if (flags & (EV_WRITE | EV_READ)) {",
    );
    for source in [stored, multi] {
        let Some(callbacks) = import(&source, ImportRole::Callback) else {
            return;
        };
        assert!(activation_flag_dependence(named(&callbacks, "cb_event")).is_err());
    }
}

// --- registration sites ---------------------------------------------------

#[test]
fn registration_sites_record_unchecked_results_from_source() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let [timeout] = registration_sites("cb_timeout", fixture.wrapper("_mk_event_timeout_create"))
        .try_into()
        .unwrap();
    assert_eq!(timeout.event_flags, Some(17));
    assert!(!timeout.creation_result_checked);
    assert_eq!(timeout.registration_result_checked, Some(false));
    assert!(
        timeout
            .ignored_results
            .iter()
            .any(|call| call.starts_with("_mk_event_add@"))
    );

    let [wait] = registration_sites("cb_wait_2_timeout", fixture.wrapper("_mk_event_wait_2"))
        .try_into()
        .unwrap();
    assert_eq!(wait.event_flags, Some(1));
    assert!(!wait.creation_result_checked);
    assert_eq!(wait.registration_result_checked, Some(false));

    let add = registration_sites("cb_event", fixture.wrapper("_mk_event_add"));
    assert!(!add.is_empty());
    assert!(add.iter().all(
        |site| !site.creation_result_checked && site.registration_result_checked == Some(true)
    ));
    let update = registration_sites("cb_event", fixture.wrapper("_mk_event_update"));
    assert!(update.iter().all(|site| site.creation_result_checked));
    assert!(update.iter().any(|site| site.stale_event_field_on_failure));
    assert!(update.iter().any(|site| !site.stale_event_field_on_failure));
}

// --- cb_timeout path composition -------------------------------------------

#[cfg(not(windows))]
#[test]
fn every_timeout_path_composes_on_the_close_success_branch_only() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let composition = compose_timeout_source_paths(
        fixture.callback("cb_timeout"),
        &fixture.timeout_projection(),
        fixture.wrapper("_mk_event_timeout_create"),
        &fixture.profile(),
    )
    .unwrap();
    assert_eq!(composition.event_flags, 0x11);
    assert_eq!(composition.teardown_paths, BTreeSet::from([0, 1]));
    assert_eq!(composition.non_teardown_paths, BTreeSet::from([2]));
    assert!(composition.map_exclusive_to_registration);
    assert!(composition.close_success_branch_composes());
    assert!(!composition.close_failure_branch_composes());
    assert!(composition.liveness_gap_witnessed());
    let covered = composition
        .programs
        .iter()
        .filter(|program| program.purpose == TimeoutProgramPurpose::Composition && program.accepted)
        .flat_map(|program| program.source_paths.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(covered, BTreeSet::from([0, 1, 2]));
}

#[test]
fn a_reordered_or_partial_teardown_is_refused_by_the_model() {
    for steps in [
        vec![
            TimeoutStep::EndpointSend,
            TimeoutStep::CloseWriteEndpoint,
            TimeoutStep::DropMapArgument,
            TimeoutStep::DeleteCurrentEvent,
            TimeoutStep::FreeCurrentEvent,
        ],
        vec![
            TimeoutStep::EndpointSend,
            TimeoutStep::CloseWriteEndpoint,
            TimeoutStep::FreeCurrentEvent,
            TimeoutStep::DropMapArgument,
        ],
        vec![
            TimeoutStep::EndpointSend,
            TimeoutStep::DeleteCurrentEvent,
            TimeoutStep::FreeCurrentEvent,
            TimeoutStep::DropMapArgument,
        ],
    ] {
        let program = timeout_program(0x11, &[(&steps, true)], 0);
        assert!(
            check_event_program(&program, &[ObligationProfile]).is_err(),
            "{steps:?}"
        );
    }
    let complete = [
        TimeoutStep::EndpointSend,
        TimeoutStep::CloseWriteEndpoint,
        TimeoutStep::DeleteCurrentEvent,
        TimeoutStep::FreeCurrentEvent,
        TimeoutStep::DropMapArgument,
    ];
    let program = timeout_program(0x11, &[(&complete, true)], 0);
    assert!(check_event_program(&program, &[ObligationProfile]).is_ok());
    // The reactor model does not itself distinguish a persistent timer on
    // re-activation; persistence is the separate source fact checked from
    // the registration path (see the non-persistent registration control).
}

#[cfg(not(windows))]
#[test]
fn a_non_persistent_or_escaping_registration_is_not_admitted() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let non_persistent = with_variant(
        WRAPPERS_C,
        "EV_TIMEOUT | EV_PERSIST,\n                      cb_timeout, ev_map);",
        "EV_TIMEOUT,\n                      cb_timeout, ev_map);",
    );
    let escaping = with_variant(
        WRAPPERS_C,
        "    ev_map->event = libev;\n\n    event_add(libev, &timev);",
        "    ev_map->event = libev;\n    mk_event_publish_map(ev_map);\n\n    event_add(libev, &timev);",
    );
    let escaping = with_variant(
        &escaping,
        "static inline int _mk_event_timeout_create(",
        "void mk_event_publish_map(void *);\nstatic inline int _mk_event_timeout_create(",
    );
    let Some(wrappers) = import(&non_persistent, ImportRole::Wrapper) else {
        return;
    };
    let error = compose_timeout_source_paths(
        fixture.callback("cb_timeout"),
        &fixture.timeout_projection(),
        named(&wrappers, "_mk_event_timeout_create"),
        &fixture.profile(),
    )
    .unwrap_err();
    assert!(error.contains("persistent"), "{error}");

    let wrappers = import(&escaping, ImportRole::Wrapper).unwrap();
    let composition = compose_timeout_source_paths(
        fixture.callback("cb_timeout"),
        &fixture.timeout_projection(),
        named(&wrappers, "_mk_event_timeout_create"),
        &fixture.profile(),
    )
    .unwrap();
    assert!(!composition.map_exclusive_to_registration);
}

#[cfg(not(windows))]
#[test]
fn the_teardown_projection_must_describe_the_composed_callback() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let changed = with_variant(
        CALLBACKS_C,
        "        evutil_closesocket(ev_map->pipe[1]);\n        event_del(ev_map->event);",
        "        event_del(ev_map->event);\n        evutil_closesocket(ev_map->pipe[1]);",
    );
    let callbacks = import(&changed, ImportRole::Callback).unwrap();
    let error = compose_timeout_source_paths(
        named(&callbacks, "cb_timeout"),
        &fixture.timeout_projection(),
        fixture.wrapper("_mk_event_timeout_create"),
        &fixture.profile(),
    )
    .unwrap_err();
    assert!(error.contains("projection"), "{error}");
}

// --- decision rule --------------------------------------------------------

fn discharged() -> ObligationStatus {
    ObligationStatus::Discharged {
        kind: EvidenceKind::SourceFact,
        method: "test".into(),
        established: "test".into(),
    }
}

fn relevant(paths: &[usize], status: ObligationStatus) -> RelevantObligation {
    RelevantObligation {
        paths: paths.iter().copied().collect(),
        relevance: "test".into(),
        status,
    }
}

#[test]
fn composed_requires_a_joined_family_no_blocker_and_no_open_obligation() {
    let all_discharged = Cycle18Obligation::ALL
        .iter()
        .map(|obligation| (*obligation, relevant(&[0, 1], discharged())))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        decide(None, &all_discharged, &BTreeMap::new()),
        CallbackVerdict::Composed
    );

    let mut one_open = all_discharged.clone();
    one_open.insert(
        Cycle18Obligation::RegistrationLiveness,
        relevant(&[1], open("needs", Vec::new())),
    );
    assert_eq!(
        decide(None, &one_open, &BTreeMap::new()),
        CallbackVerdict::Unknown {
            open_obligations: BTreeSet::from([Cycle18Obligation::RegistrationLiveness]),
            blocking_scenarios: BTreeSet::new(),
            evidence_family_refused: None,
        }
    );

    let blocker = BTreeMap::from([("base_creation_failure".into(), "loop".into())]);
    assert!(matches!(
        decide(None, &all_discharged, &blocker),
        CallbackVerdict::Unknown { .. }
    ));
    assert!(matches!(
        decide(Some("refused".into()), &all_discharged, &BTreeMap::new()),
        CallbackVerdict::Unknown { .. }
    ));

    // An obligation with no relevant path cannot hold a callback open.
    let mut irrelevant = all_discharged;
    irrelevant.insert(
        Cycle18Obligation::CallerSizeBounds,
        relevant(&[], open("needs", Vec::new())),
    );
    assert_eq!(
        decide(None, &irrelevant, &BTreeMap::new()),
        CallbackVerdict::Composed
    );
}

#[test]
fn unrecognized_scenarios_block_every_callback() {
    for (callback, _) in CALLBACKS {
        assert!(scenario_relevance("new_failure_scenario", callback).is_some());
        assert!(scenario_relevance("base_creation_failure", callback).is_some());
    }
    assert!(scenario_relevance("timer_creation_failure", "cb_event").is_none());
    assert!(scenario_relevance("timer_creation_failure", "cb_wait_2_timeout").is_some());
}

// --- assembled table -----------------------------------------------------

fn sources(
    fixture: &Fixture,
    projection: &TimeoutTeardownSourceProjection,
) -> CallbackObligationSources<'static> {
    // Leak the fixture so the borrowed report inputs have a simple lifetime.
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        callbacks: fixture.callbacks.clone(),
        wrappers: fixture.wrappers.clone(),
    }));
    let projection: &'static TimeoutTeardownSourceProjection =
        Box::leak(Box::new(projection.clone()));
    CallbackObligationSources {
        callbacks: [
            fixture.callback("cb_event"),
            fixture.callback("cb_timeout"),
            fixture.callback("cb_wait_2_timeout"),
        ],
        event_init: fixture.wrapper("_mk_event_init"),
        loop_create: fixture.wrapper("_mk_event_loop_create"),
        event_add: fixture.wrapper("_mk_event_add"),
        event_update: fixture.wrapper("_mk_event_update"),
        timeout_create: fixture.wrapper("_mk_event_timeout_create"),
        wait_2: fixture.wrapper("_mk_event_wait_2"),
        timeout_projection: projection,
    }
}

#[cfg(not(windows))]
#[test]
fn assembled_table_discharges_only_source_bound_obligations_and_composes_nothing() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let projection = fixture.timeout_projection();
    let report = assemble(&fixture.aggregate(), &sources(&fixture, &projection)).unwrap();
    assert!(!report.universal_safety_certificate);
    assert_eq!(report.before.composed_callbacks, 0);
    assert_eq!(report.after.composed_callbacks, 0);
    assert_eq!(report.after.unknown_callbacks, 3);
    assert!(report.initialization.is_some());
    assert!(report.timeout_paths.is_some());
    assert!(report.after.discharged_cells > report.before.discharged_cells);
    assert_eq!(report.before.relevant_cells, report.after.relevant_cells);

    let row = |table: &ObligationTable, name: &str| {
        table
            .callbacks
            .iter()
            .find(|row| row.callback == name)
            .unwrap()
            .clone()
    };
    for name in ["cb_event", "cb_timeout", "cb_wait_2_timeout"] {
        let before = row(&report.before, name);
        let after = row(&report.after, name);
        assert!(
            !before.obligations[&Cycle18Obligation::InitializationReachability]
                .status
                .is_discharged()
        );
        assert!(
            after.obligations[&Cycle18Obligation::InitializationReachability]
                .status
                .is_discharged()
        );
        // No discharger exists for these; they keep every callback Unknown.
        for obligation in [
            Cycle18Obligation::CallerNullHandling,
            Cycle18Obligation::RegistrationLiveness,
        ] {
            assert!(!after.obligations[&obligation].status.is_discharged());
        }
        let CallbackVerdict::Unknown {
            open_obligations,
            blocking_scenarios,
            ..
        } = &after.verdict
        else {
            panic!("{name} must stay Unknown");
        };
        assert!(open_obligations.contains(&Cycle18Obligation::RegistrationLiveness));
        assert!(blocking_scenarios.contains("base_creation_failure"));
    }
    let event = row(&report.after, "cb_event");
    assert!(
        event
            .obligations
            .contains_key(&Cycle18Obligation::CallerSizeBounds)
    );
    assert!(
        !event.obligations[&Cycle18Obligation::NaturalActivationReachability]
            .status
            .is_discharged()
    );
    let timeout = row(&report.after, "cb_timeout");
    assert!(
        !timeout
            .obligations
            .contains_key(&Cycle18Obligation::CallerSizeBounds)
    );
    assert!(
        timeout.obligations[&Cycle18Obligation::NaturalActivationReachability]
            .status
            .is_discharged()
    );
    assert!(
        !timeout.obligations[&Cycle18Obligation::AllPathExecutionCoverage]
            .status
            .is_discharged()
    );
    let wait = row(&report.after, "cb_wait_2_timeout");
    assert!(
        !wait
            .obligations
            .contains_key(&Cycle18Obligation::AllocationAuthorityProvenance)
    );
    assert!(
        wait.blocking_scenarios
            .contains_key("timer_creation_failure")
    );
    assert_eq!(
        wait.obligations
            .values()
            .filter(|row| !row.status.is_discharged())
            .count(),
        2,
        "cb_wait_2_timeout keeps caller null handling and registration liveness open"
    );
}

#[cfg(not(windows))]
#[test]
fn a_substituted_callback_or_refused_family_is_not_admitted() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let projection = fixture.timeout_projection();
    let mut aggregate = fixture.aggregate();
    aggregate
        .callback_source_sha256
        .insert("cb_event".into(), "2".repeat(64));
    assert!(assemble(&aggregate, &sources(&fixture, &projection)).is_err());

    let mut aggregate = fixture.aggregate();
    aggregate.profile_sha256 = "3".repeat(64);
    assert!(assemble(&aggregate, &sources(&fixture, &projection)).is_err());

    let mut aggregate = fixture.aggregate();
    aggregate.joined.remove("cb_event");
    aggregate
        .refused
        .insert("payload".into(), "refused in test".into());
    let report = assemble(&aggregate, &sources(&fixture, &projection)).unwrap();
    for row in &report.after.callbacks {
        let CallbackVerdict::Unknown {
            evidence_family_refused,
            ..
        } = &row.verdict
        else {
            panic!("refused evidence cannot compose");
        };
        assert!(evidence_family_refused.is_some());
    }
    let event = report
        .after
        .callbacks
        .iter()
        .find(|row| row.callback == "cb_event")
        .unwrap();
    assert!(
        !event.obligations[&Cycle18Obligation::AllPathExecutionCoverage]
            .status
            .is_discharged()
    );
}

#[cfg(not(windows))]
#[test]
fn fixture_report_serializes_with_the_verdict_contract() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let projection = fixture.timeout_projection();
    let report = assemble(&fixture.aggregate(), &sources(&fixture, &projection)).unwrap();
    let value = serde_json::to_value(&report).unwrap();
    for field in [
        "artifact",
        "property",
        "semantic_profile",
        "scope",
        "bounds",
        "assumptions",
        "evidence_version",
    ] {
        assert!(
            value.get(field).is_some_and(|field| !field.is_null()),
            "{field}"
        );
    }
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}
