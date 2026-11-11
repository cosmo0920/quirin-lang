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

//! Fail-closed aggregate for the three Monkey callbacks in Cycle 18.
//!
//! This module derives the eight-path denominator from sealed typed-source
//! summaries and replays each existing evidence checker.  Its only publishable
//! verdict is [`Cycle18AggregateVerdict::Unknown`].  A complete bounded join is
//! useful evidence, but the known source and caller obligations prevent it from
//! becoming a universal callback-safety certificate.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::SourceSummary;
use qcore_c_gateway::typed_source::{SourceExpression, SourceOperation};
use qcore_contract::contract_observation::{ContractFact, check_typed_contract_facts};
use qcore_core::event_reactor::CallbackSummary;
use qcore_schemas::sha256_hex;
use serde::Serialize;

use crate::event_composition_context::ValidatedEventCompositionContext;
use crate::event_payload::{
    EVENT_PAYLOAD_EVIDENCE_VERSION, EVENT_PAYLOAD_PROPERTY, EventPayloadBoundary,
    EventPayloadEvidence, EventPayloadProjection, EventPayloadWriteAuthority,
    check_event_payload_boundary,
};
use crate::monkey_wait_projection::{
    CheckedWaitFailureCase, MonkeyWaitSourceProjection, WaitFailureOutcome,
    check_wait_failure_paths,
};
use crate::observed_wait_composition::{
    CONDITIONAL_REPLAY_CLAIM, ObservedWaitComposition, check_observed_wait_composition,
};
use crate::timeout_teardown::{
    TimeoutTeardownEvidence, TimeoutTeardownSourceProjection, check_timeout_teardown_evidence,
};

pub const CYCLE18_COMPOSITION_POLICY: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/cycle18_composition.qv"
);
pub const CYCLE18_COMPOSITION_PROPERTY: &str = "monkey-cycle18-callback-composition";
pub const CYCLE18_COMPOSITION_EVIDENCE_VERSION: &str = "qcore.cycle18.callback-composition.v1";

const CALLBACKS: [(&str, usize); 3] =
    [("cb_event", 4), ("cb_timeout", 3), ("cb_wait_2_timeout", 1)];

const UNRESOLVED: [(&str, &str); 7] = [
    (
        "initialization_reachability",
        "prove the opaque event_init branch unreachable or check it under the selected production profile",
    ),
    (
        "caller_null_handling",
        "compose every production caller with the null loop/context return paths",
    ),
    (
        "caller_size_bounds",
        "derive the loop size and allocation multiplication bound from every production caller",
    ),
    (
        "allocation_authority_provenance",
        "derive storage extent and exclusive callback write authority without caller declaration",
    ),
    (
        "registration_liveness",
        "establish behavior after timer registration failure against the selected backend",
    ),
    (
        "natural_activation_reachability",
        "observe or prove production readiness/timeout activation rather than controlled activation",
    ),
    (
        "all_path_execution_coverage",
        "compose evidence for both cb_timeout teardown paths and its non-teardown success path",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cycle18AggregateVerdict {
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cycle18CompositionReport {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: BTreeSet<String>,
    pub bounds: BTreeMap<String, u64>,
    pub assumptions: BTreeMap<String, String>,
    pub evidence_version: String,
    pub verdict: Cycle18AggregateVerdict,
    pub source_revision: String,
    pub profile_sha256: String,
    pub callback_source_sha256: BTreeMap<String, String>,
    pub source_inventory_sha256: String,
    pub policy_sha256: String,
    pub context_identities: BTreeMap<String, String>,
    pub joined_evidence_families: BTreeSet<String>,
    pub refused_evidence_families: BTreeMap<String, String>,
    pub provisional_unsafe_scenarios: BTreeSet<String>,
    pub unresolved_obligations: BTreeMap<String, String>,
    pub checked_caller_size_bounds: Vec<Cycle18CallerSizeBound>,
    pub universal_safety_certificate: bool,
}

pub struct Cycle18CompositionInput<'a> {
    /// Exactly the three callback summaries. Their order is irrelevant.
    pub callback_summaries: [&'a SourceSummary; 3],
    pub wait_source: &'a MonkeyWaitSourceProjection,
    pub wait_failure_source: &'a MonkeyWaitSourceProjection,
    pub wait_context: &'a ValidatedEventCompositionContext,
    pub wait_observations: &'a [u8],
    pub wait_bindings: &'a [u8],
    pub wait_failure_observations: &'a [u8],
    pub payload_source: &'a EventPayloadProjection,
    pub payload_context: &'a ValidatedEventCompositionContext,
    pub payload_evidence: EventPayloadEvidence<'a>,
    pub payload_authority: &'a EventPayloadWriteAuthority,
    pub payload_callback: &'a CallbackSummary,
    pub timeout_source: &'a TimeoutTeardownSourceProjection,
    pub timeout_evidence: &'a TimeoutTeardownEvidence,
    pub wait_failure_identity: &'a Cycle18PreparedSourceIdentity,
    pub timeout_identity: &'a Cycle18PreparedSourceIdentity,
    /// Caller summaries selected by the configured caller inventory.  Each
    /// checked row is useful evidence, but this slice does not claim that the
    /// inventory is complete.
    pub caller_summaries: &'a [&'a SourceSummary],
}

/// Caller-prepared source/build identity derived from configured immutable
/// inputs.  The aggregate checks syntax and exact revision/profile joins; the
/// digest values identify bytes but do not attest that those bytes executed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cycle18PreparedSourceIdentity {
    pub source_revision: String,
    pub source_compile_task_sha256: String,
    pub source_sha256: String,
    pub profile_sha256: String,
    pub build_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cycle18CallerSizeBound {
    pub caller: String,
    pub source_revision: String,
    pub profile_sha256: String,
    pub source_summary_sha256: String,
    pub call_count: usize,
    pub capacities: BTreeSet<u64>,
    pub element_size: u64,
    pub maximum_allocation_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cycle18CompositionError {
    SourceInventory(String),
    Policy(String),
}

impl std::fmt::Display for Cycle18CompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Cycle18CompositionError {}

struct SourceInventory {
    revision: String,
    profile_sha256: String,
    callback_revisions: BTreeMap<String, String>,
    callback_profiles: BTreeMap<String, String>,
    callback_path_counts: BTreeMap<String, usize>,
    callback_libraries: BTreeMap<String, String>,
    callback_roles: BTreeMap<String, String>,
    callback_source_sha256: BTreeMap<String, String>,
    wait_callback_projection_sha256: String,
    timeout_callback_projection_sha256: String,
    inventory_sha256: String,
}

fn source_inventory(
    summaries: [&SourceSummary; 3],
) -> Result<SourceInventory, Cycle18CompositionError> {
    let mut by_name = BTreeMap::new();
    for summary in summaries {
        if by_name.insert(summary.function(), summary).is_some() {
            return Err(Cycle18CompositionError::SourceInventory(format!(
                "duplicate callback summary {}",
                summary.function()
            )));
        }
    }
    let actual_names = by_name.keys().copied().collect::<BTreeSet<_>>();
    let expected_names = CALLBACKS
        .iter()
        .map(|(name, _)| *name)
        .collect::<BTreeSet<_>>();
    if actual_names != expected_names {
        return Err(Cycle18CompositionError::SourceInventory(
            "callback inventory is not exactly the three Cycle 18 callbacks".into(),
        ));
    }

    let first = by_name[CALLBACKS[0].0];
    let revision = first.source_revision().to_owned();
    let profile_sha256 = first
        .profile_assumptions()
        .get("profile_digest")
        .cloned()
        .unwrap_or_default();
    if profile_sha256.len() != 64 {
        return Err(Cycle18CompositionError::SourceInventory(
            "callback source profile has no canonical digest".into(),
        ));
    }

    let mut callback_source_sha256 = BTreeMap::new();
    let mut callback_revisions = BTreeMap::new();
    let mut callback_profiles = BTreeMap::new();
    let mut callback_path_counts = BTreeMap::new();
    let mut callback_libraries = BTreeMap::new();
    let mut callback_roles = BTreeMap::new();
    let mut wait_callback_projection_sha256 = String::new();
    let mut timeout_callback_projection_sha256 = String::new();
    for (name, _expected_paths) in CALLBACKS {
        let summary = by_name[name];
        let summary_profile = summary
            .profile_assumptions()
            .get("profile_digest")
            .cloned()
            .unwrap_or_default();
        if summary_profile.len() != 64 {
            return Err(Cycle18CompositionError::SourceInventory(format!(
                "{name} has no canonical source profile digest"
            )));
        }
        callback_revisions.insert(name.into(), summary.source_revision().into());
        callback_profiles.insert(name.into(), summary_profile);
        callback_path_counts.insert(name.into(), summary.paths().len());
        callback_libraries.insert(
            name.into(),
            summary
                .profile_assumptions()
                .get("library")
                .cloned()
                .unwrap_or_default(),
        );
        callback_roles.insert(
            name.into(),
            summary
                .profile_assumptions()
                .get("typed_import_role")
                .cloned()
                .unwrap_or_default(),
        );
        let bytes = serde_json::to_vec(summary).map_err(|error| {
            Cycle18CompositionError::SourceInventory(format!(
                "cannot encode {name} summary: {error}"
            ))
        })?;
        callback_source_sha256.insert(name.into(), sha256_hex(&bytes));
        if matches!(name, "cb_wait_2_timeout" | "cb_timeout") {
            let bytes = serde_json::to_vec(&(
                summary.function(),
                summary.source_revision(),
                summary.paths(),
            ))
            .map_err(|error| {
                Cycle18CompositionError::SourceInventory(format!(
                    "cannot encode wait callback projection: {error}"
                ))
            })?;
            if name == "cb_wait_2_timeout" {
                wait_callback_projection_sha256 = sha256_hex(&bytes);
            } else {
                timeout_callback_projection_sha256 = sha256_hex(&bytes);
            }
        }
    }
    let inventory_sha256 = sha256_hex(
        &serde_json::to_vec(&(
            &revision,
            &profile_sha256,
            &callback_source_sha256,
            CALLBACKS,
        ))
        .expect("source inventory serialization is infallible"),
    );
    Ok(SourceInventory {
        revision,
        profile_sha256,
        callback_revisions,
        callback_profiles,
        callback_path_counts,
        callback_libraries,
        callback_roles,
        callback_source_sha256,
        wait_callback_projection_sha256,
        timeout_callback_projection_sha256,
        inventory_sha256,
    })
}

fn text(value: impl Into<String>) -> ContractFact {
    ContractFact::Text(value.into())
}

fn integer(value: usize) -> ContractFact {
    ContractFact::Integer(value as i128)
}

fn canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn prepared_identity_is_well_formed(identity: &Cycle18PreparedSourceIdentity) -> bool {
    identity.source_revision.len() == 40
        && identity
            .source_revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && canonical_sha256(&identity.source_compile_task_sha256)
        && canonical_sha256(&identity.source_sha256)
        && canonical_sha256(&identity.profile_sha256)
        && canonical_sha256(&identity.build_sha256)
}

fn check_policy(
    function: &str,
    facts: &[(&str, ContractFact)],
) -> Result<(), Cycle18CompositionError> {
    check_typed_contract_facts(CYCLE18_COMPOSITION_POLICY, function, facts)
        .map_err(Cycle18CompositionError::Policy)
}

/// Prove the allocation multiplication bound for every
/// `mk_event_loop_create` call retained in one sealed caller summary.
///
/// This is deliberately caller-local.  It does not claim that the configured
/// list of caller summaries is a complete production call graph, and it does
/// not discharge null handling.
pub fn check_cycle18_caller_size_bound(
    summary: &SourceSummary,
    expected_source_revision: &str,
    expected_profile_sha256: &str,
    element_size: u64,
) -> Result<Cycle18CallerSizeBound, Cycle18CompositionError> {
    let profile = summary
        .profile_assumptions()
        .get("profile_digest")
        .cloned()
        .unwrap_or_default();
    let mut capacities = BTreeSet::new();
    let mut call_count = 0usize;
    for path in summary.paths() {
        for operation in &path.operations {
            let SourceOperation::Call {
                function,
                arguments,
                ..
            } = operation
            else {
                continue;
            };
            if function != "mk_event_loop_create" {
                continue;
            }
            call_count += 1;
            let [argument] = arguments.as_slice() else {
                return Err(Cycle18CompositionError::SourceInventory(format!(
                    "{} has a non-unary mk_event_loop_create call",
                    summary.function()
                )));
            };
            let SourceExpression::Integer { value } = &argument.expression else {
                return Err(Cycle18CompositionError::SourceInventory(format!(
                    "{} has a non-literal event-loop size",
                    summary.function()
                )));
            };
            let value = value.parse::<u64>().map_err(|_| {
                Cycle18CompositionError::SourceInventory(format!(
                    "{} has an unsupported event-loop size literal {value}",
                    summary.function()
                ))
            })?;
            capacities.insert(value);
        }
    }
    let minimum = capacities.iter().next().copied().unwrap_or(0);
    let maximum = capacities.iter().next_back().copied().unwrap_or(0);
    let maximum_allocation_bytes = maximum.checked_mul(element_size).ok_or_else(|| {
        Cycle18CompositionError::SourceInventory(format!(
            "{} event-loop allocation overflows",
            summary.function()
        ))
    })?;
    check_policy(
        "caller_size_bound",
        &[
            ("caller_name", text(summary.function())),
            ("source_revision", text(summary.source_revision())),
            ("expected_source_revision", text(expected_source_revision)),
            ("profile", text(&profile)),
            ("expected_profile", text(expected_profile_sha256)),
            ("call_count", integer(call_count)),
            ("minimum_size", ContractFact::Integer(i128::from(minimum))),
            ("maximum_size", ContractFact::Integer(i128::from(maximum))),
            (
                "signed_size_max",
                ContractFact::Integer(i128::from(i32::MAX)),
            ),
            (
                "element_size",
                ContractFact::Integer(i128::from(element_size)),
            ),
            (
                "maximum_allocation_bytes",
                ContractFact::Integer(i128::from(maximum_allocation_bytes)),
            ),
        ],
    )?;
    let source_summary_sha256 = sha256_hex(&serde_json::to_vec(summary).map_err(|error| {
        Cycle18CompositionError::SourceInventory(format!(
            "cannot encode caller {}: {error}",
            summary.function()
        ))
    })?);
    Ok(Cycle18CallerSizeBound {
        caller: summary.function().into(),
        source_revision: summary.source_revision().into(),
        profile_sha256: profile,
        source_summary_sha256,
        call_count,
        capacities,
        element_size,
        maximum_allocation_bytes,
    })
}

fn contexts_join(
    inventory: &SourceInventory,
    wait: &ValidatedEventCompositionContext,
    payload: &ValidatedEventCompositionContext,
) -> Result<BTreeMap<String, String>, String> {
    let wait = wait.context();
    let payload = payload.context();
    check_typed_contract_facts(
        CYCLE18_COMPOSITION_POLICY,
        "context_identity_join",
        &[
            ("wait_source_revision", text(wait.fluent_bit_revision())),
            (
                "payload_source_revision",
                text(payload.fluent_bit_revision()),
            ),
            ("aggregate_source_revision", text(&inventory.revision)),
            ("wait_monkey_profile", text(wait.monkey_profile_sha256())),
            (
                "payload_monkey_profile",
                text(payload.monkey_profile_sha256()),
            ),
            ("aggregate_profile", text(&inventory.profile_sha256)),
            ("wait_libevent_revision", text(wait.libevent_revision())),
            (
                "payload_libevent_revision",
                text(payload.libevent_revision()),
            ),
            (
                "wait_libevent_profile",
                text(wait.libevent_profile_sha256()),
            ),
            (
                "payload_libevent_profile",
                text(payload.libevent_profile_sha256()),
            ),
        ],
    )
    .map_err(|error| format!("context identity policy refused: {error}"))?;
    let mut identities = BTreeMap::new();
    for (prefix, context) in [("wait", wait), ("payload", payload)] {
        for (name, value) in [
            ("context", context.context_sha256()),
            (
                "source_compile_task",
                context.source_compile_task_sha256().into(),
            ),
            ("monkey_source", context.monkey_source_sha256().into()),
            ("monkey_profile", context.monkey_profile_sha256().into()),
            ("monkey_build", context.monkey_build_sha256().into()),
            ("libevent_source", context.libevent_source_sha256().into()),
            ("libevent_profile", context.libevent_profile_sha256().into()),
            ("libevent_build", context.libevent_build_sha256().into()),
            ("observer_binary", context.observer_binary_sha256().into()),
            ("observer_harness", context.observer_harness_sha256().into()),
        ] {
            identities.insert(format!("{prefix}_{name}_sha256"), value);
        }
    }
    Ok(identities)
}

fn validate_wait_failures(cases: &[CheckedWaitFailureCase]) -> Result<BTreeSet<String>, String> {
    let expected = BTreeSet::from([
        "base_creation_failure",
        "context_allocation_failure",
        "fired_allocation_failure",
        "loop_init_failure",
        "loop_size_bounds",
        "success_observation",
        "timer_creation_failure",
        "timer_registration_failure",
    ]);
    let actual = cases
        .iter()
        .map(|case| case.scenario.as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected || cases.len() != expected.len() {
        return Err("wait failure evidence does not contain the exact eight-case inventory".into());
    }
    let evidence_version = crate::monkey_wait_projection::wait_failure_policy_digest();
    if cases.iter().any(|case| {
        case.artifact.is_empty()
            || case.property.is_empty()
            || case.semantic_profile != "fluent-bit/unix-libevent source projection"
            || case.scope.is_empty()
            || case.bounds.is_empty()
            || case.assumptions.is_empty()
            || case.evidence_version != evidence_version
            || case.reason.is_empty()
    }) {
        return Err("wait failure result has incomplete verdict-contract metadata".into());
    }
    Ok(cases
        .iter()
        .filter(|case| case.outcome == WaitFailureOutcome::Unsafe)
        .map(|case| case.scenario.clone())
        .collect())
}

fn canonical_map_sha256(map: &BTreeMap<String, String>) -> String {
    sha256_hex(&serde_json::to_vec(map).expect("string identity maps always serialize"))
}

fn check_wait_failure_source_join(
    observed: &MonkeyWaitSourceProjection,
    failure: &MonkeyWaitSourceProjection,
) -> Result<(), String> {
    check_typed_contract_facts(
        CYCLE18_COMPOSITION_POLICY,
        "wait_failure_source_join",
        &[
            (
                "observed_summary_map",
                text(canonical_map_sha256(observed.source_summary_digests())),
            ),
            (
                "failure_summary_map",
                text(canonical_map_sha256(failure.source_summary_digests())),
            ),
            (
                "observed_contract_map",
                text(canonical_map_sha256(observed.contract_digests())),
            ),
            (
                "failure_contract_map",
                text(canonical_map_sha256(failure.contract_digests())),
            ),
        ],
    )
    .map_err(|error| format!("wait failure source policy refused: {error}"))
}

fn validate_wait_composition(
    value: &ObservedWaitComposition,
    context: &ValidatedEventCompositionContext,
) -> Result<(), String> {
    if value.claim() != CONDITIONAL_REPLAY_CLAIM
        || value.caller_prepared_context_sha256() != context.context_sha256()
        || value.checked_records() != 11
        || value.checked_operations() == 0
        || value.selected_paths().get("cb_wait_2_timeout").is_none()
    {
        return Err("observed wait result has incomplete or mismatched metadata".into());
    }
    Ok(())
}

/// Recheck and join the three callback evidence families.
///
/// Structurally invalid source inventory is an error because no eight-path
/// denominator can be derived. Missing, mismatched, or incomplete runtime
/// evidence instead remains visible in the returned Unknown report.
pub fn check_cycle18_composition(
    input: Cycle18CompositionInput<'_>,
) -> Result<Cycle18CompositionReport, Cycle18CompositionError> {
    let inventory = source_inventory(input.callback_summaries)?;
    check_policy(
        "callback_inventory",
        &[
            (
                "callback_names",
                text(
                    inventory
                        .callback_path_counts
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            (
                "callback_count",
                integer(inventory.callback_path_counts.len()),
            ),
            (
                "selected_path_count",
                integer(inventory.callback_path_counts.values().sum()),
            ),
            (
                "cb_event_paths",
                integer(inventory.callback_path_counts["cb_event"]),
            ),
            (
                "cb_timeout_paths",
                integer(inventory.callback_path_counts["cb_timeout"]),
            ),
            (
                "cb_wait_2_timeout_paths",
                integer(inventory.callback_path_counts["cb_wait_2_timeout"]),
            ),
            (
                "cb_event_revision",
                text(&inventory.callback_revisions["cb_event"]),
            ),
            (
                "cb_timeout_revision",
                text(&inventory.callback_revisions["cb_timeout"]),
            ),
            (
                "cb_wait_revision",
                text(&inventory.callback_revisions["cb_wait_2_timeout"]),
            ),
            ("expected_source_revision", text(&inventory.revision)),
            (
                "cb_event_profile",
                text(&inventory.callback_profiles["cb_event"]),
            ),
            (
                "cb_timeout_profile",
                text(&inventory.callback_profiles["cb_timeout"]),
            ),
            (
                "cb_wait_profile",
                text(&inventory.callback_profiles["cb_wait_2_timeout"]),
            ),
            ("expected_profile", text(&inventory.profile_sha256)),
            (
                "cb_event_library",
                text(&inventory.callback_libraries["cb_event"]),
            ),
            (
                "cb_timeout_library",
                text(&inventory.callback_libraries["cb_timeout"]),
            ),
            (
                "cb_wait_library",
                text(&inventory.callback_libraries["cb_wait_2_timeout"]),
            ),
            ("cb_event_role", text(&inventory.callback_roles["cb_event"])),
            (
                "cb_timeout_role",
                text(&inventory.callback_roles["cb_timeout"]),
            ),
            (
                "cb_wait_role",
                text(&inventory.callback_roles["cb_wait_2_timeout"]),
            ),
        ],
    )?;

    let mut joined = BTreeSet::new();
    let mut refused = BTreeMap::new();
    let mut payload_element_size = None;
    let context_identities =
        match contexts_join(&inventory, input.wait_context, input.payload_context) {
            Ok(identities) => identities,
            Err(reason) => {
                refused.insert("context".into(), reason);
                BTreeMap::new()
            }
        };
    let mut context_identities = context_identities;
    for (prefix, identity) in [
        ("failure", input.wait_failure_identity),
        ("timeout", input.timeout_identity),
    ] {
        if !prepared_identity_is_well_formed(identity)
            || identity.source_revision != inventory.revision
            || identity.profile_sha256 != inventory.profile_sha256
        {
            refused.insert(
                format!("{prefix}_identity"),
                "prepared source/build identity is malformed or does not match the aggregate source/profile".into(),
            );
        } else {
            context_identities.insert(
                format!("{prefix}_source_compile_task_sha256"),
                identity.source_compile_task_sha256.clone(),
            );
            context_identities.insert(
                format!("{prefix}_source_sha256"),
                identity.source_sha256.clone(),
            );
            context_identities.insert(
                format!("{prefix}_profile_sha256"),
                identity.profile_sha256.clone(),
            );
            context_identities.insert(
                format!("{prefix}_build_sha256"),
                identity.build_sha256.clone(),
            );
        }
    }

    let wait_projection_matches = input.wait_source.source_revision() == inventory.revision
        && input
            .wait_source
            .profile_assumptions()
            .get("profile_digest")
            == Some(&inventory.profile_sha256)
        && input
            .wait_source
            .source_path_counts()
            .get("cb_wait_2_timeout")
            == Some(&1)
        && input
            .wait_source
            .source_summary_digests()
            .get("cb_wait_2_timeout")
            == Some(&inventory.wait_callback_projection_sha256);
    if !wait_projection_matches {
        refused.insert(
            "wait".into(),
            "wait projection does not match the independent callback inventory".into(),
        );
    } else {
        let failure_projection_join =
            check_wait_failure_source_join(input.wait_source, input.wait_failure_source);
        match (
            check_observed_wait_composition(
                input.wait_source,
                input.wait_context,
                input.wait_observations,
                input.wait_bindings,
            ),
            failure_projection_join.and_then(|()| {
                check_wait_failure_paths(input.wait_failure_source, input.wait_failure_observations)
                    .map_err(|error| format!("failure replay: {error:?}"))
            }),
        ) {
            (Ok(observed), Ok(cases)) => {
                match (
                    validate_wait_composition(&observed, input.wait_context),
                    validate_wait_failures(&cases),
                ) {
                    (Ok(()), Ok(unsafe_cases)) => {
                        joined.insert("cb_wait_2_timeout".into());
                        // Assigned below after all joins; keeping it here would
                        // let a caller-provided label replace checked cases.
                        refused.insert(
                            "__provisional_unsafe".into(),
                            unsafe_cases.into_iter().collect::<Vec<_>>().join(","),
                        );
                    }
                    (left, right) => {
                        refused.insert(
                            "wait".into(),
                            left.err()
                                .or_else(|| right.err())
                                .unwrap_or_else(|| "wait metadata mismatch".into()),
                        );
                    }
                }
            }
            (observed, failures) => {
                refused.insert(
                    "wait".into(),
                    observed
                        .err()
                        .map(|error| format!("observed replay: {error}"))
                        .or_else(|| failures.err())
                        .unwrap_or_else(|| "wait evidence refused".into()),
                );
            }
        }
    }

    let payload_digest_matches = input.payload_source.source_revision() == inventory.revision
        && input.payload_source.selected_paths() == 4
        && input.payload_source.source_summary_sha256()
            == inventory.callback_source_sha256["cb_event"];
    if !payload_digest_matches {
        refused.insert(
            "payload".into(),
            "payload projection does not match the independent callback inventory".into(),
        );
    } else {
        match check_event_payload_boundary(
            input.payload_source,
            input.payload_context,
            input.payload_evidence,
            input.payload_authority,
            input.payload_callback,
        ) {
            EventPayloadBoundary::Checked(report)
                if report.property == EVENT_PAYLOAD_PROPERTY
                    && report.semantic_profile
                        == input.payload_context.context().semantic_profile()
                    && report.scope == BTreeSet::from(["cb_event".into()])
                    && report.bounds.get("selected_paths") == Some(&4)
                    && report.bounds.get("checked_executions") == Some(&4)
                    && report.evidence_version == EVENT_PAYLOAD_EVIDENCE_VERSION
                    && report.source_revision == inventory.revision
                    && report.source_summary_sha256
                        == inventory.callback_source_sha256["cb_event"]
                    && report.caller_prepared_context_sha256
                        == input.payload_context.context_sha256()
                    && report.selected_paths == 4
                    && report.checked_executions == 4
                    && !report.assumptions.is_empty()
                    && !report.universal_safety_certificate =>
            {
                payload_element_size = Some(report.contextual_element_size);
                joined.insert("cb_event".into());
            }
            EventPayloadBoundary::Checked(_) => {
                refused.insert("payload".into(), "payload report metadata mismatch".into());
            }
            EventPayloadBoundary::Unknown { reason } => {
                refused.insert("payload".into(), reason);
            }
        }
    }

    let timeout_digest_matches = input.timeout_source.source_revision() == inventory.revision
        && input
            .timeout_source
            .profile_assumptions()
            .get("profile_digest")
            == Some(&inventory.profile_sha256)
        && input.timeout_source.source_summary_digest()
            == inventory.timeout_callback_projection_sha256
        && input.timeout_source.path_call_sequences().len() == 3;
    if !timeout_digest_matches {
        refused.insert(
            "timeout".into(),
            "timeout projection does not match the independent callback inventory".into(),
        );
    } else {
        match check_timeout_teardown_evidence(input.timeout_source, input.timeout_evidence) {
            Ok(certificate)
                if certificate.artifact == "monkey/cb_timeout"
                    && certificate.property
                        == "running-callback self teardown retires its event, map, and write endpoint exactly once"
                    && certificate.semantic_profile == "fluent-bit/monkey/libevent"
                    && !certificate.scope.is_empty()
                    && !certificate.bounds.is_empty()
                    && !certificate.assumptions.is_empty()
                    && certificate.evidence_version == "cycle18.timeout-self-teardown.v1"
                    && certificate.checked_operations == 15 =>
            {
                joined.insert("cb_timeout".into());
            }
            Ok(_) => {
                refused.insert(
                    "timeout".into(),
                    "timeout certificate metadata mismatch".into(),
                );
            }
            Err(error) => {
                refused.insert("timeout".into(), format!("{error:?}"));
            }
        }
    }

    let provisional_unsafe_scenarios: BTreeSet<String> = refused
        .remove("__provisional_unsafe")
        .map(|names| {
            names
                .split(',')
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if joined.len() == 3 && refused.is_empty() {
        let wait_context = input.wait_context.context();
        let payload_context = input.payload_context.context();
        check_policy(
            "evidence_join",
            &[
                ("checked_callback_families", integer(3)),
                ("payload_checked_paths", integer(4)),
                ("wait_checked_paths", integer(1)),
                ("timeout_source_paths", integer(3)),
                ("timeout_checked_executions", integer(1)),
                (
                    "wait_source_revision",
                    text(input.wait_source.source_revision()),
                ),
                (
                    "payload_source_revision",
                    text(input.payload_source.source_revision()),
                ),
                (
                    "timeout_source_revision",
                    text(input.timeout_source.source_revision()),
                ),
                ("aggregate_source_revision", text(&inventory.revision)),
                ("wait_profile", text(&inventory.profile_sha256)),
                ("payload_profile", text(&inventory.profile_sha256)),
                ("timeout_profile", text(&inventory.profile_sha256)),
                ("aggregate_profile", text(&inventory.profile_sha256)),
                (
                    "wait_callback_digest",
                    text(&inventory.wait_callback_projection_sha256),
                ),
                (
                    "expected_wait_callback_digest",
                    text(&inventory.wait_callback_projection_sha256),
                ),
                (
                    "payload_callback_digest",
                    text(input.payload_source.source_summary_sha256()),
                ),
                (
                    "expected_payload_callback_digest",
                    text(&inventory.callback_source_sha256["cb_event"]),
                ),
                (
                    "timeout_callback_digest",
                    text(input.timeout_source.source_summary_digest()),
                ),
                (
                    "expected_timeout_callback_digest",
                    text(&inventory.timeout_callback_projection_sha256),
                ),
                (
                    "wait_context_revision",
                    text(wait_context.fluent_bit_revision()),
                ),
                (
                    "payload_context_revision",
                    text(payload_context.fluent_bit_revision()),
                ),
                (
                    "wait_context_profile",
                    text(wait_context.monkey_profile_sha256()),
                ),
                (
                    "payload_context_profile",
                    text(payload_context.monkey_profile_sha256()),
                ),
                (
                    "wait_monkey_source",
                    text(wait_context.monkey_source_sha256()),
                ),
                (
                    "wait_monkey_build",
                    text(wait_context.monkey_build_sha256()),
                ),
                (
                    "wait_libevent_source",
                    text(wait_context.libevent_source_sha256()),
                ),
                (
                    "wait_libevent_build",
                    text(wait_context.libevent_build_sha256()),
                ),
                (
                    "payload_monkey_source",
                    text(payload_context.monkey_source_sha256()),
                ),
                (
                    "payload_monkey_build",
                    text(payload_context.monkey_build_sha256()),
                ),
                (
                    "payload_libevent_source",
                    text(payload_context.libevent_source_sha256()),
                ),
                (
                    "payload_libevent_build",
                    text(payload_context.libevent_build_sha256()),
                ),
                (
                    "failure_identity_revision",
                    text(&input.wait_failure_identity.source_revision),
                ),
                (
                    "failure_identity_profile",
                    text(&input.wait_failure_identity.profile_sha256),
                ),
                (
                    "failure_compile_task",
                    text(&input.wait_failure_identity.source_compile_task_sha256),
                ),
                (
                    "failure_source",
                    text(&input.wait_failure_identity.source_sha256),
                ),
                (
                    "failure_build",
                    text(&input.wait_failure_identity.build_sha256),
                ),
                (
                    "wait_context_compile_task",
                    text(wait_context.source_compile_task_sha256()),
                ),
                (
                    "wait_context_monkey_source",
                    text(wait_context.monkey_source_sha256()),
                ),
                (
                    "wait_context_monkey_build",
                    text(wait_context.monkey_build_sha256()),
                ),
                (
                    "timeout_identity_revision",
                    text(&input.timeout_identity.source_revision),
                ),
                (
                    "timeout_identity_profile",
                    text(&input.timeout_identity.profile_sha256),
                ),
                (
                    "timeout_compile_task",
                    text(&input.timeout_identity.source_compile_task_sha256),
                ),
                (
                    "timeout_source",
                    text(&input.timeout_identity.source_sha256),
                ),
                ("timeout_build", text(&input.timeout_identity.build_sha256)),
            ],
        )?;
    }

    let checked_caller_size_bounds = payload_element_size
        .map(|element_size| {
            input
                .caller_summaries
                .iter()
                .map(|summary| {
                    check_cycle18_caller_size_bound(
                        summary,
                        &inventory.revision,
                        &inventory.profile_sha256,
                        element_size,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let unresolved_obligations = UNRESOLVED
        .into_iter()
        .map(|(name, discharge)| (name.to_owned(), discharge.to_owned()))
        .collect::<BTreeMap<_, _>>();
    check_policy(
        "aggregate_acceptance",
        &[
            ("verdict", text("unknown")),
            ("universal_safety_certificate", integer(0)),
            ("initialization_reachability", text("unknown")),
            ("caller_null_handling", text("unknown")),
            ("caller_size_bounds", text("unknown")),
            ("allocation_authority_provenance", text("unknown")),
            ("registration_liveness", text("unknown")),
            ("natural_activation_reachability", text("unknown")),
            ("all_path_execution_coverage", text("unknown")),
            (
                "unresolved_obligation_count",
                integer(unresolved_obligations.len()),
            ),
            (
                "provisional_unsafe_scenario_count",
                integer(provisional_unsafe_scenarios.len()),
            ),
        ],
    )?;

    Ok(Cycle18CompositionReport {
        artifact: "fluent-bit/monkey/libevent callback aggregate".into(),
        property: CYCLE18_COMPOSITION_PROPERTY.into(),
        semantic_profile: "fluent-bit/monkey/libevent".into(),
        scope: CALLBACKS
            .into_iter()
            .map(|(name, _)| name.to_owned())
            .collect(),
        bounds: BTreeMap::from([
            ("callbacks".into(), 3),
            ("source_paths".into(), 8),
            ("joined_evidence_families".into(), joined.len() as u64),
            (
                "unresolved_obligations".into(),
                unresolved_obligations.len() as u64,
            ),
        ]),
        assumptions: BTreeMap::from([
            (
                "source_import".into(),
                "faithful Clang/C extraction remains a declared premise".into(),
            ),
            (
                "lean_rust_correspondence".into(),
                "matching Lean models are review-maintained, not extraction theorems".into(),
            ),
            (
                "runtime_provenance".into(),
                "validated digests identify presented bytes but do not attest execution".into(),
            ),
        ]),
        evidence_version: CYCLE18_COMPOSITION_EVIDENCE_VERSION.into(),
        verdict: Cycle18AggregateVerdict::Unknown,
        source_revision: inventory.revision,
        profile_sha256: inventory.profile_sha256,
        callback_source_sha256: inventory.callback_source_sha256,
        source_inventory_sha256: inventory.inventory_sha256,
        policy_sha256: sha256_hex(CYCLE18_COMPOSITION_POLICY.as_bytes()),
        context_identities,
        joined_evidence_families: joined,
        refused_evidence_families: refused,
        provisional_unsafe_scenarios,
        unresolved_obligations,
        checked_caller_size_bounds,
        universal_safety_certificate: false,
    })
}

#[cfg(test)]
#[cfg_attr(windows, allow(dead_code))]
mod tests {
    use super::*;
    use qcore_c_gateway::legacy_c::C11TranslationUnit;
    use qcore_c_gateway::typed_source::{
        SourceImportProfile, SourceResult, import_source_functions,
        import_source_functions_from_profile,
    };
    use qcore_contract::profile_loader::load_source_set_profile;
    use qcore_schemas::profile::ImportRole;
    use std::path::PathBuf;

    fn imported_caller(argument: &str) -> Option<SourceSummary> {
        if qcore_c_gateway::clang_command().is_err() {
            return None;
        }
        let source = format!(
            "struct mk_event_loop;\n\
             struct mk_event_loop *mk_event_loop_create(int);\n\
             int selected_caller(void) {{\n\
                 struct mk_event_loop *loop = mk_event_loop_create({argument});\n\
                 return loop == (void *)0;\n\
             }}"
        );
        let [result] = import_source_functions(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "66910c10a4d7eafa810b84229cf2dcf7b0f26f97",
            &["selected_caller"],
            &SourceImportProfile {
                records: &[],
                sizeof_types: &[],
                signature_constraints: &[],
                restricted_calls: &["mk_event_loop_create"],
            },
        )
        .unwrap()
        .try_into()
        .ok()?;
        match result {
            SourceResult::Summary(summary) => Some(summary),
            SourceResult::Unknown { reason, .. } => panic!("caller import failed: {reason}"),
        }
    }

    fn selected_summary<'a>(results: &'a [SourceResult], name: &str) -> &'a SourceSummary {
        results
            .iter()
            .find_map(|result| match result {
                SourceResult::Summary(summary) if summary.function() == name => Some(summary),
                _ => None,
            })
            .unwrap()
    }

    fn wait_projection(contract_suffix: &str) -> Option<MonkeyWaitSourceProjection> {
        if qcore_c_gateway::clang_command().is_err() {
            return None;
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let profile =
            load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
        let callbacks = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
        let wrappers = format!(
            "{}\n{}",
            include_str!("../../../compiler/tests/fixtures/monkey/wrappers.c"),
            r#"
int event_base_loop(struct event_base *, int);
int _mk_event_wait_with_flags(struct mk_event_loop *loop, int flags)
{
    struct mk_event_ctx *ctx = loop->data;
    ctx->fired_count = 0;
    event_base_loop(ctx->base, flags);
    loop->n_events = ctx->fired_count;
    return loop->n_events;
}
"#
        );
        let import = |source: &str, role| {
            import_source_functions_from_profile(
                &C11TranslationUnit {
                    source,
                    ..Default::default()
                },
                crate::monkey_wait_projection::monkey_source_revision(),
                &profile,
                "monkey",
                role,
            )
            .unwrap()
        };
        let callbacks = import(callbacks, ImportRole::Callback);
        let wrappers = import(&wrappers, ImportRole::Wrapper);
        let wrapper_contract = format!(
            "{}\n{contract_suffix}",
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv"
            )
        );
        let contracts = crate::monkey_wait_projection::MonkeyWaitContracts::parse(
            &wrapper_contract,
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/composition.qv"
            ),
        )
        .unwrap();
        Some(
            crate::monkey_wait_projection::project_monkey_timed_wait_source(
                [
                    selected_summary(&callbacks, "cb_wait_2_timeout"),
                    selected_summary(&wrappers, "_mk_event_loop_create"),
                    selected_summary(&wrappers, "_mk_event_wait_with_flags"),
                    selected_summary(&wrappers, "_mk_event_wait_2"),
                    selected_summary(&wrappers, "_mk_event_loop_destroy"),
                ],
                &contracts,
            )
            .unwrap(),
        )
    }

    #[test]
    fn sealed_callback_inventory_derives_three_callbacks_and_eight_paths() {
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let profile =
            load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
        let source = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
        let imported = import_source_functions_from_profile(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            crate::monkey_wait_projection::monkey_source_revision(),
            &profile,
            "monkey",
            ImportRole::Callback,
        )
        .unwrap();
        let summary = |name: &str| {
            imported
                .iter()
                .find_map(|result| match result {
                    SourceResult::Summary(summary) if summary.function() == name => Some(summary),
                    _ => None,
                })
                .unwrap()
        };
        let inventory = source_inventory([
            summary("cb_event"),
            summary("cb_timeout"),
            summary("cb_wait_2_timeout"),
        ])
        .unwrap();
        assert_eq!(inventory.callback_source_sha256.len(), 3);
        assert_eq!(inventory.revision.len(), 40);
        assert_eq!(inventory.profile_sha256.len(), 64);
        assert!(
            source_inventory([
                summary("cb_event"),
                summary("cb_event"),
                summary("cb_wait_2_timeout")
            ])
            .is_err()
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn failure_projection_contract_map_cannot_cross_at_same_source_identity() {
        let Some(observed) = wait_projection("") else {
            return;
        };
        let failure =
            wait_projection("// independently changed configured contract bytes").unwrap();
        assert_eq!(
            observed.source_revision(),
            failure.source_revision(),
            "control must retain the same revision"
        );
        assert_eq!(
            observed.profile_assumptions(),
            failure.profile_assumptions(),
            "control must retain the same profile"
        );
        assert_eq!(
            observed.source_summary_digests(),
            failure.source_summary_digests(),
            "control changes the contract map, not source summaries"
        );
        assert_ne!(observed.contract_digests(), failure.contract_digests());
        check_wait_failure_source_join(&observed, &observed).unwrap();
        assert!(check_wait_failure_source_join(&observed, &failure).is_err());
    }

    #[test]
    fn literal_caller_size_is_derived_and_multiplication_checked() {
        let Some(summary) = imported_caller("256") else {
            return;
        };
        let checked =
            check_cycle18_caller_size_bound(&summary, summary.source_revision(), "", 72).unwrap();
        assert_eq!(checked.caller, "selected_caller");
        assert_eq!(checked.call_count, summary.paths().len());
        assert_eq!(checked.capacities, BTreeSet::from([256]));
        assert_eq!(checked.maximum_allocation_bytes, 18_432);
    }

    #[test]
    fn nonliteral_zero_and_wrong_identity_never_gain_a_size_bound() {
        let Some(nonliteral) = imported_caller("1 + 1") else {
            return;
        };
        assert!(
            check_cycle18_caller_size_bound(&nonliteral, nonliteral.source_revision(), "", 72)
                .is_err()
        );
        let zero = imported_caller("0").unwrap();
        assert!(check_cycle18_caller_size_bound(&zero, zero.source_revision(), "", 72).is_err());
        let bounded = imported_caller("8").unwrap();
        assert!(
            check_cycle18_caller_size_bound(&bounded, "0".repeat(40).as_str(), "", 72).is_err()
        );
    }

    #[test]
    fn aggregate_policy_cannot_publish_safe_or_drop_a_named_obligation() {
        assert!(
            check_typed_contract_facts(
                CYCLE18_COMPOSITION_POLICY,
                "aggregate_acceptance",
                &[
                    ("verdict", text("safe")),
                    ("universal_safety_certificate", integer(1)),
                    ("initialization_reachability", text("proved")),
                    ("caller_null_handling", text("unknown")),
                    ("caller_size_bounds", text("unknown")),
                    ("allocation_authority_provenance", text("unknown")),
                    ("registration_liveness", text("unknown")),
                    ("natural_activation_reachability", text("unknown")),
                    ("all_path_execution_coverage", text("unknown")),
                    ("unresolved_obligation_count", integer(6)),
                    ("provisional_unsafe_scenario_count", integer(3)),
                ],
            )
            .is_err()
        );
    }
}
