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

use std::collections::{BTreeMap, BTreeSet};

use qcore_verification::event_composition_context::{
    ContextValidationError, DeclaredObservationTrust, EVENT_COMPOSITION_EVIDENCE_VERSION,
    EVENT_COMPOSITION_PROPERTY, EventCompositionContext, EventCompositionContextInput,
    EventCompositionPayload, check_event_composition_context_for,
};

const OBSERVATIONS: &[u8] = b"{\"kind\":\"callback_enter\"}\n{\"kind\":\"callback_exit\"}\n";
const BINDINGS: &[u8] = b"{\"kind\":\"registration\"}\n{\"kind\":\"callback\"}\n";

fn digest(label: &str) -> String {
    qcore_schemas::sha256_hex(label.as_bytes())
}

fn payload() -> EventCompositionPayload<'static> {
    EventCompositionPayload {
        observations: OBSERVATIONS,
        bindings: BINDINGS,
    }
}

fn input() -> EventCompositionContextInput {
    EventCompositionContextInput {
        artifact: "fluent-bit/lib/monkey/mk_core/mk_event.c".into(),
        semantic_profile: "monkey-forced-libevent+libevent-linux-epoll".into(),
        scope: BTreeSet::from([
            "cb_event".into(),
            "cb_timeout".into(),
            "cb_wait_2_timeout".into(),
        ]),
        bounds: BTreeMap::from([
            ("callback_executions".into(), 8),
            ("source_paths".into(), 8),
        ]),
        assumptions: BTreeMap::from([
            (
                "observation_authority".into(),
                "producer-supplied runtime evidence".into(),
            ),
            ("task_effects".into(), "unsupported".into()),
        ]),
        source_compile_task_sha256: digest("compile-task"),
        fluent_bit_revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
        monkey_source_sha256: digest("monkey-source"),
        monkey_profile_sha256: digest("monkey-profile"),
        monkey_build_sha256: digest("monkey-build"),
        libevent_revision: "d82464a277d0f42703702c4dfd9af6af38595a83".into(),
        libevent_source_sha256: digest("libevent-source"),
        libevent_profile_sha256: digest("libevent-profile"),
        libevent_build_sha256: digest("libevent-build"),
        contracts_sha256: digest("contracts"),
        source_rules_sha256: digest("source-rules"),
        observation_rules_sha256: digest("observation-rules"),
        observer_binary_sha256: digest("observer-binary"),
        observer_harness_sha256: digest("observer-harness"),
        linked_libraries_sha256: BTreeMap::from([
            ("libevent".into(), digest("linked-libevent")),
            ("monkey".into(), digest("linked-monkey")),
        ]),
        observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    }
}

fn context(input: EventCompositionContextInput) -> EventCompositionContext {
    EventCompositionContext::derive(input, payload()).unwrap()
}

fn assert_mismatch(field: &'static str, mutate: impl FnOnce(&mut EventCompositionContextInput)) {
    let expected = context(input());
    let mut changed = input();
    mutate(&mut changed);
    let presented = context(changed);
    assert_eq!(
        check_event_composition_context_for(&presented, payload(), &expected),
        Err(ContextValidationError::ContextMismatch(field))
    );
}

#[test]
fn exact_caller_held_context_revalidates_payload_and_returns_a_sealed_identity() {
    let expected = context(input());
    let presented = context(input());
    let validated = check_event_composition_context_for(&presented, payload(), &expected).unwrap();
    assert_eq!(validated.context(), &expected);
    assert_eq!(validated.context_sha256(), expected.context_sha256());
    assert_eq!(validated.context_sha256().len(), 64);
    assert_eq!(expected.property(), EVENT_COMPOSITION_PROPERTY);
    assert_eq!(expected.bounds()["source_paths"], 8);
    assert_eq!(expected.assumptions()["task_effects"], "unsupported");
    assert_eq!(
        expected.source_compile_task_sha256(),
        digest("compile-task")
    );
    assert_eq!(
        expected.fluent_bit_revision(),
        "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
    );
    assert_eq!(expected.monkey_profile_sha256(), digest("monkey-profile"));
    assert_eq!(
        expected.libevent_profile_sha256(),
        digest("libevent-profile")
    );
    assert_eq!(expected.contracts_sha256(), digest("contracts"));
    assert_eq!(expected.source_rules_sha256(), digest("source-rules"));
    assert_eq!(
        expected.observation_rules_sha256(),
        digest("observation-rules")
    );
    assert_eq!(
        expected.evidence_version(),
        EVENT_COMPOSITION_EVIDENCE_VERSION
    );
    assert_eq!(
        expected.observation_trust(),
        DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence
    );
}

#[test]
fn claim_scope_bounds_and_assumption_substitutions_reject() {
    assert_mismatch("artifact", |value| value.artifact.push_str(".alternate"));
    assert_mismatch("semantic_profile", |value| {
        value.semantic_profile = "monkey-forced-libevent+libevent-linux-poll".into();
    });
    assert_mismatch("scope", |value| {
        value.scope.remove("cb_timeout");
    });
    assert_mismatch("bounds", |value| {
        value.bounds.insert("source_paths".into(), 7);
    });
    assert_mismatch("assumptions", |value| {
        value
            .assumptions
            .insert("task_effects".into(), "declared".into());
    });
}

#[test]
fn source_profile_and_build_substitutions_reject() {
    assert_mismatch("source_compile_task_sha256", |value| {
        value.source_compile_task_sha256 = digest("other-compile-task");
    });
    assert_mismatch("fluent_bit_revision", |value| {
        value.fluent_bit_revision = "76910c10a4d7eafa810b84229cf2dcf7b0f26f97".into();
    });
    assert_mismatch("monkey_source_sha256", |value| {
        value.monkey_source_sha256 = digest("other-monkey-source");
    });
    assert_mismatch("monkey_profile_sha256", |value| {
        value.monkey_profile_sha256 = digest("other-monkey-profile");
    });
    assert_mismatch("monkey_build_sha256", |value| {
        value.monkey_build_sha256 = digest("other-monkey-build");
    });
    assert_mismatch("libevent_revision", |value| {
        value.libevent_revision = "e82464a277d0f42703702c4dfd9af6af38595a83".into();
    });
    assert_mismatch("libevent_source_sha256", |value| {
        value.libevent_source_sha256 = digest("other-libevent-source");
    });
    assert_mismatch("libevent_profile_sha256", |value| {
        value.libevent_profile_sha256 = digest("other-libevent-profile");
    });
    assert_mismatch("libevent_build_sha256", |value| {
        value.libevent_build_sha256 = digest("other-libevent-build");
    });
}

#[test]
fn contract_rule_observer_and_library_substitutions_reject() {
    assert_mismatch("contracts_sha256", |value| {
        value.contracts_sha256 = digest("other-contracts");
    });
    assert_mismatch("source_rules_sha256", |value| {
        value.source_rules_sha256 = digest("other-source-rules");
    });
    assert_mismatch("observation_rules_sha256", |value| {
        value.observation_rules_sha256 = digest("other-observation-rules");
    });
    assert_mismatch("observer_binary_sha256", |value| {
        value.observer_binary_sha256 = digest("other-observer-binary");
    });
    assert_mismatch("observer_harness_sha256", |value| {
        value.observer_harness_sha256 = digest("other-observer-harness");
    });
    assert_mismatch("linked_libraries_sha256", |value| {
        value
            .linked_libraries_sha256
            .insert("libevent".into(), digest("other-linked-libevent"));
    });
}

#[test]
fn stale_wrapper_payload_substitution_rejects_before_context_equality() {
    let expected = context(input());
    let presented = context(input());
    let substituted = EventCompositionPayload {
        observations: b"{\"kind\":\"callback_exit\"}\n",
        bindings: BINDINGS,
    };
    assert_eq!(
        check_event_composition_context_for(&presented, substituted, &expected),
        Err(ContextValidationError::PayloadDigestMismatch(
            "observations"
        ))
    );

    let substituted = EventCompositionPayload {
        observations: OBSERVATIONS,
        bindings: b"{\"kind\":\"different_callback\"}\n",
    };
    assert_eq!(
        check_event_composition_context_for(&presented, substituted, &expected),
        Err(ContextValidationError::PayloadDigestMismatch("bindings"))
    );
}

#[test]
fn coherent_payload_substitution_is_distinct_from_the_expected_run() {
    let expected = context(input());
    let changed_payload = EventCompositionPayload {
        observations: b"{\"kind\":\"callback_enter\",\"event\":2}\n",
        bindings: BINDINGS,
    };
    let presented = EventCompositionContext::derive(input(), changed_payload).unwrap();
    assert_eq!(
        check_event_composition_context_for(&presented, changed_payload, &expected),
        Err(ContextValidationError::ContextMismatch(
            "observations_sha256"
        ))
    );
}

#[test]
fn incomplete_or_noncanonical_context_refuses_at_construction() {
    let mut missing = input();
    missing.scope.clear();
    assert_eq!(
        EventCompositionContext::derive(missing, payload()),
        Err(ContextValidationError::InvalidField("scope"))
    );

    let mut missing = input();
    missing.linked_libraries_sha256.remove("libevent");
    assert_eq!(
        EventCompositionContext::derive(missing, payload()),
        Err(ContextValidationError::InvalidField(
            "linked_libraries_sha256"
        ))
    );

    let mut malformed = input();
    malformed.observer_binary_sha256 = "ABC".repeat(21) + "D";
    assert_eq!(
        EventCompositionContext::derive(malformed, payload()),
        Err(ContextValidationError::InvalidField(
            "observer_binary_sha256"
        ))
    );

    assert_eq!(
        EventCompositionContext::derive(
            input(),
            EventCompositionPayload {
                observations: b"",
                bindings: BINDINGS,
            }
        ),
        Err(ContextValidationError::InvalidField("observations"))
    );
}
