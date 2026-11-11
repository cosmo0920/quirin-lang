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

//! Conditionally replay a bounded observed run against sealed source evidence.
//!
//! The caller must prepare artifact identities independently. This module
//! checks their syntax and equality where they meet source/observation data;
//! it does not attest the named compiler, binaries, libraries, linkage, host,
//! or native run manifest. Success is a conditional replay result, neither an
//! all-path source verdict nor artifact attestation.

use crate::event_composition_context::ValidatedEventCompositionContext;
use crate::monkey_wait_projection::{
    MonkeyWaitSourceProjection, WaitCallbackSemantics, WaitObservationRequirement as Requirement,
};
use crate::timed_wait_observation_wire::decode_timed_wait_observation;
use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_core::event_observation::{
    EventObservationAction as Action, EventObservationTrace, ObservationIdentity,
    check_timed_wait_observation,
};
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, EventFlags, EventKey, EventOperation as Op,
    EventProgram, ReactorProfile, check_event_program,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const OBSERVED_WAIT_RULES: &str = "v2: exact eleven identity-bearing records; one dispatcher; timeout-only borrowed argument; no payload, descriptor, task or synchronization; source/context/payload equality; exact pthread/runtime-observation/linkage premises; valid native-run-manifest digest; caller-prepared artifact identity; conditional event kernel replay";
pub const DEFAULT_WAIT_IDENTITY_POLICY: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/wait_identity_policy.qv"
);
pub const CONDITIONAL_REPLAY_CLAIM: &str = "caller_prepared_artifact_identity_conditional_replay";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SingleDispatcher;
impl ReactorProfile for SingleDispatcher {
    fn threading_enabled(self) -> bool {
        false
    }
}

/// Constructor-private conditional replay result, with no safety verdict or
/// artifact-attestation claim.
#[derive(Debug, Serialize)]
pub struct ObservedWaitComposition {
    claim: &'static str,
    caller_prepared_context_sha256: String,
    checked_records: usize,
    checked_operations: usize,
    selected_paths: BTreeMap<String, usize>,
    /// Native identities stay full-width. Model identifiers are fresh names
    /// within separate base, event, argument and dispatcher namespaces.
    identity_mapping: BTreeMap<String, (u64, u32, u32)>,
}

impl ObservedWaitComposition {
    #[must_use]
    pub const fn claim(&self) -> &'static str {
        self.claim
    }

    #[must_use]
    pub fn caller_prepared_context_sha256(&self) -> &str {
        &self.caller_prepared_context_sha256
    }

    #[must_use]
    pub const fn checked_records(&self) -> usize {
        self.checked_records
    }

    #[must_use]
    pub const fn checked_operations(&self) -> usize {
        self.checked_operations
    }

    #[must_use]
    pub fn selected_paths(&self) -> &BTreeMap<String, usize> {
        &self.selected_paths
    }
}

/// Read one exact literal expectation from the default declarative policy.
/// This rejects duplicate, absent, compound or non-literal clauses.
pub fn wait_identity_policy_fact(fact: &str) -> Result<ContractFact, String> {
    contract_fact_literal(DEFAULT_WAIT_IDENTITY_POLICY, "wait_identity_context", fact)
}

/// Read one exact text expectation without maintaining a second Rust copy.
pub fn wait_identity_policy_text(fact: &str) -> Result<String, String> {
    match wait_identity_policy_fact(fact)? {
        ContractFact::Text(value) => Ok(value),
        ContractFact::Integer(_) => Err(format!("wait identity fact {fact} is not text")),
    }
}

/// Semantic profile selected by the bundled identity policy.
pub fn observed_wait_profile() -> Result<String, String> {
    wait_identity_policy_text("semantic_profile")
}

/// Identity of the operational observation rules and the exact declarative
/// policy bytes compiled into this verifier. The framing prevents a boundary
/// ambiguity between the two inputs.
#[must_use]
pub fn wait_observation_rules_digest() -> String {
    wait_observation_rules_digest_with_policy(DEFAULT_WAIT_IDENTITY_POLICY)
}

/// Derive the observation-rule identity for an explicitly caller-selected
/// policy. This exists for fail-closed mutation tests and configured callers;
/// choosing different policy bytes is a trusted caller action, and the
/// admitted context must bind those exact bytes before they can be evaluated.
#[must_use]
pub fn wait_observation_rules_digest_with_policy(policy: &str) -> String {
    let mut framed = Vec::with_capacity(32 + OBSERVED_WAIT_RULES.len() + policy.len());
    framed.extend_from_slice(b"qcore.wait-observation-rules.v1\0");
    framed.extend_from_slice(&(OBSERVED_WAIT_RULES.len() as u64).to_be_bytes());
    framed.extend_from_slice(OBSERVED_WAIT_RULES.as_bytes());
    framed.extend_from_slice(&(policy.len() as u64).to_be_bytes());
    framed.extend_from_slice(policy.as_bytes());
    qcore_schemas::sha256_hex(&framed)
}

fn text(value: impl Into<String>) -> ContractFact {
    ContractFact::Text(value.into())
}

fn integer(value: impl Into<i128>) -> ContractFact {
    ContractFact::Integer(value.into())
}

/// Mechanically count the identities present in an observed trace. These are
/// actual data facts; the declarative policy separately decides which counts
/// its bounded profile accepts.
#[must_use]
pub fn observed_wait_bounds(trace: &EventObservationTrace) -> BTreeMap<String, u64> {
    let runs = trace
        .records
        .iter()
        .map(|record| record.run)
        .collect::<BTreeSet<_>>();
    let dispatchers = trace
        .records
        .iter()
        .map(|record| record.dispatcher)
        .collect::<BTreeSet<_>>();
    let mut events = BTreeSet::<ObservationIdentity>::new();
    for record in &trace.records {
        match &record.action {
            Action::EventNew { event, .. }
            | Action::EventAdd { event, .. }
            | Action::Activate { event, .. }
            | Action::CallbackEnter { event, .. }
            | Action::CallbackExit { event, .. }
            | Action::EventFree { event } => {
                events.insert(*event);
            }
            Action::BaseCreate { .. }
            | Action::DispatchBegin { .. }
            | Action::DispatchEnd { .. }
            | Action::ArgumentEnd { .. }
            | Action::BaseFree { .. } => {}
        }
    }
    BTreeMap::from([
        ("dispatchers".into(), dispatchers.len() as u64),
        ("events".into(), events.len() as u64),
        ("records".into(), trace.records.len() as u64),
        ("runs".into(), runs.len() as u64),
    ])
}

fn check_identity_entry(
    policy: &str,
    kind: &str,
    name: &str,
    actual: Option<&String>,
    expected: &str,
) -> Result<(), String> {
    check_typed_contract_facts(
        policy,
        "wait_identity_entry",
        &[
            ("identity_actual", text(actual.cloned().unwrap_or_default())),
            ("identity_expected", text(expected)),
        ],
    )
    .map_err(|error| format!("{kind} identity mismatch: {name}: {error}"))
}

/// Replay one exact observed lifecycle under caller-prepared artifact identity
/// premises. A successful return confirms the bounded source/trace/model join;
/// it does not authenticate any digest or establish that the named artifacts
/// produced the bytes. Configured callers must derive and snapshot those
/// identities independently before invoking this function.
pub fn check_observed_wait_composition(
    source: &MonkeyWaitSourceProjection,
    admitted: &ValidatedEventCompositionContext,
    observations: &[u8],
    bindings: &[u8],
) -> Result<ObservedWaitComposition, String> {
    check_observed_wait_composition_with_policy(
        source,
        admitted,
        observations,
        bindings,
        DEFAULT_WAIT_IDENTITY_POLICY,
    )
}

/// Replay with explicitly caller-selected declarative policy bytes.
///
/// The caller's selection is trusted configuration, not evidence. Admission
/// still refuses unless the validated context was prepared with the digest of
/// these exact bytes and [`OBSERVED_WAIT_RULES`].
pub fn check_observed_wait_composition_with_policy(
    source: &MonkeyWaitSourceProjection,
    admitted: &ValidatedEventCompositionContext,
    observations: &[u8],
    bindings: &[u8],
    policy: &str,
) -> Result<ObservedWaitComposition, String> {
    let context = admitted.context();
    let decoded = decode_timed_wait_observation(observations, bindings)?;
    let actual_bounds = observed_wait_bounds(&decoded.trace);
    let assumptions = context.assumptions();
    let empty = String::new();
    let assumption = |name: &str| assumptions.get(name).unwrap_or(&empty).clone();
    let expected_scope: BTreeSet<_> = source.selected_path_indices().keys().cloned().collect();
    let actual_scope = serde_json::to_string(context.scope()).map_err(|e| e.to_string())?;
    let expected_scope = serde_json::to_string(&expected_scope).map_err(|e| e.to_string())?;
    let expected_contracts_sha256 = qcore_schemas::sha256_hex(
        &serde_json::to_vec(source.contract_digests()).map_err(|e| e.to_string())?,
    );
    let expected_source_rules_sha256 =
        qcore_schemas::sha256_hex(qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes());
    let expected_observation_rules_sha256 = wait_observation_rules_digest_with_policy(policy);
    // Policy identity is immutable admission, checked before policy execution.
    // A caller-selected policy cannot delete or weaken the clause that would
    // otherwise have been responsible for binding its own bytes.
    if context.observation_rules_sha256() != expected_observation_rules_sha256 {
        return Err("selected wait identity policy does not match admitted context".into());
    }
    let expected_profile_digest = source
        .profile_assumptions()
        .get("profile_digest")
        .cloned()
        .unwrap_or_default();
    check_typed_contract_facts(
        policy,
        "wait_identity_context",
        &[
            ("semantic_profile", text(context.semantic_profile())),
            ("threads", text(assumption("threads"))),
            ("observation_trust", text(assumption("observation_trust"))),
            ("linkage_trust", text(assumption("linkage_trust"))),
            (
                "native_run_manifest_sha256_length",
                integer(
                    assumptions
                        .get("native_run_manifest_sha256")
                        .map_or(0, |digest| digest.len()) as i128,
                ),
            ),
            (
                "native_run_manifest_invalid_lower_hex_count",
                integer(
                    assumptions
                        .get("native_run_manifest_sha256")
                        .map_or(0, |digest| {
                            digest
                                .bytes()
                                .filter(|byte| {
                                    !(byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
                                })
                                .count()
                        }) as i128,
                ),
            ),
            ("contracts_sha256_actual", text(context.contracts_sha256())),
            ("contracts_sha256_expected", text(expected_contracts_sha256)),
            (
                "source_rules_sha256_actual",
                text(context.source_rules_sha256()),
            ),
            (
                "source_rules_sha256_expected",
                text(expected_source_rules_sha256),
            ),
            (
                "observation_rules_sha256_actual",
                text(context.observation_rules_sha256()),
            ),
            (
                "observation_rules_sha256_expected",
                text(expected_observation_rules_sha256),
            ),
            (
                "observations_sha256_actual",
                text(qcore_schemas::sha256_hex(observations)),
            ),
            (
                "observations_sha256_expected",
                text(context.observations_sha256()),
            ),
            (
                "bindings_sha256_actual",
                text(qcore_schemas::sha256_hex(bindings)),
            ),
            ("bindings_sha256_expected", text(context.bindings_sha256())),
            (
                "fluent_bit_revision_actual",
                text(context.fluent_bit_revision()),
            ),
            (
                "fluent_bit_revision_expected",
                text(source.source_revision()),
            ),
            (
                "monkey_profile_sha256_actual",
                text(context.monkey_profile_sha256()),
            ),
            (
                "monkey_profile_sha256_expected",
                text(expected_profile_digest),
            ),
            ("scope_actual", text(actual_scope)),
            ("scope_expected", text(expected_scope)),
            ("bound_entry_count", integer(context.bounds().len() as i128)),
            (
                "dispatchers_bound_actual",
                integer(i128::from(actual_bounds["dispatchers"])),
            ),
            (
                "dispatchers_bound_declared",
                integer(i128::from(
                    *context.bounds().get("dispatchers").unwrap_or(&0),
                )),
            ),
            (
                "events_bound_actual",
                integer(i128::from(actual_bounds["events"])),
            ),
            (
                "events_bound_declared",
                integer(i128::from(*context.bounds().get("events").unwrap_or(&0))),
            ),
            (
                "records_bound_actual",
                integer(i128::from(actual_bounds["records"])),
            ),
            (
                "records_bound_declared",
                integer(i128::from(*context.bounds().get("records").unwrap_or(&0))),
            ),
            (
                "runs_bound_actual",
                integer(i128::from(actual_bounds["runs"])),
            ),
            (
                "runs_bound_declared",
                integer(i128::from(*context.bounds().get("runs").unwrap_or(&0))),
            ),
        ],
    )?;
    for (name, digest) in source.source_summary_digests() {
        check_identity_entry(
            policy,
            "source summary context",
            name,
            context.assumptions().get(&format!("source_summary.{name}")),
            digest,
        )?;
    }
    for (name, digest) in source.contract_digests() {
        check_identity_entry(
            policy,
            "source contract context",
            name,
            context.assumptions().get(&format!("contract.{name}")),
            digest,
        )?;
    }
    for (path, digest) in &decoded.source_apis {
        check_identity_entry(
            policy,
            "observer source",
            path,
            context.assumptions().get(&format!("source_file.{path}")),
            digest,
        )?;
    }
    let native = &decoded.context;
    let (expected_callback, expected_flags) = source
        .required_observations()
        .iter()
        .find_map(|requirement| match requirement {
            Requirement::EventCreateSuccess {
                callback, flags, ..
            } => Some((callback.as_str(), *flags)),
            _ => None,
        })
        .unwrap_or(("", 0));
    check_typed_contract_facts(
        policy,
        "wait_observed_profile",
        &[
            ("backend_actual", text(&native.backend)),
            ("backend_expected", text(assumption("backend"))),
            ("callback_actual", text(&native.callback)),
            ("callback_expected", text(expected_callback)),
            ("flags_actual", integer(i128::from(native.flags))),
            ("flags_expected", integer(i128::from(expected_flags))),
            (
                "source_requirement_count",
                integer(source.required_observations().len() as i128),
            ),
            (
                "observed_record_count",
                integer(decoded.trace.records.len() as i128),
            ),
        ],
    )?;
    let observed = check_timed_wait_observation(&decoded.trace, native)
        .map_err(|error| format!("observation refused: {error:?}"))?;
    // There is exactly one identity per typed namespace. Assigning the fresh
    // model name 1 is injective on each singleton and never truncates a pointer.
    let event = EventKey::new(1, native.event.generation);
    let summary = match source.callback_semantics() {
        WaitCallbackSemantics::BorrowedNoEffectNoTasks => CallbackSummary {
            argument: 1,
            argument_mode: ArgumentMode::Borrowed,
            trigger_reasons: BTreeSet::from([ActivationReason::Timeout]),
            payload_authority: BTreeMap::new(),
            may_defer: false,
            may_spawn_tasks: false,
        },
    };
    let operations = source
        .required_observations()
        .iter()
        .zip(&decoded.trace.records)
        .map(|(requirement, record)| {
            Ok(match (requirement, &record.action) {
                (Requirement::BaseCreateSuccess, Action::BaseCreate { .. }) => Op::CreateBase {
                    base: 1,
                    profile: SingleDispatcher,
                },
                (
                    Requirement::EventCreateSuccess {
                        callback,
                        flags,
                        descriptor_absent,
                        priority_required,
                    },
                    Action::EventNew { .. },
                ) if callback == &native.callback
                    && *flags == native.flags
                    && *descriptor_absent
                    && *priority_required =>
                {
                    Op::Assign {
                        event,
                        base: 1,
                        descriptor: None,
                        flags: EventFlags::new(*flags),
                        priority: native.priority,
                        summary: summary.clone(),
                    }
                }
                (Requirement::EventAddSuccess, Action::EventAdd { .. }) => Op::Add { event },
                (Requirement::DispatchBegin, Action::DispatchBegin { .. }) => Op::BeginDispatch {
                    base: 1,
                    dispatcher: 1,
                },
                (Requirement::TimeoutActivation, Action::Activate { .. }) => Op::Activate {
                    event,
                    reason: ActivationReason::Timeout,
                    cross_thread: false,
                },
                (Requirement::CallbackEnter, Action::CallbackEnter { .. }) => Op::CallbackEnter {
                    event,
                    reason: ActivationReason::Timeout,
                },
                (Requirement::CallbackExit, Action::CallbackExit { .. }) => {
                    Op::CallbackExit { event }
                }
                (Requirement::DispatchEnd, Action::DispatchEnd { .. }) => {
                    Op::EndDispatch { base: 1 }
                }
                (Requirement::EventFree, Action::EventFree { .. }) => Op::FreeEvent { event },
                (Requirement::ArgumentScopeEnd, Action::ArgumentEnd { .. }) => {
                    Op::DropArgument { argument: 1 }
                }
                (Requirement::BaseFree, Action::BaseFree { .. }) => Op::FreeBase { base: 1 },
                _ => return Err("source obligation does not match observed operation".to_owned()),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let program = EventProgram {
        live_arguments: BTreeSet::from([1]),
        operations,
    };
    let checked = check_event_program(&program, &[SingleDispatcher])
        .map_err(|errors| format!("event program refused: {errors:?}"))?;
    if checked.checked_operations != observed.checked_records()
        || checked.freed_events != BTreeSet::from([event])
        || checked.dropped_arguments != BTreeSet::from([1])
    {
        return Err("event program did not discharge the observed lifecycle".into());
    }
    Ok(ObservedWaitComposition {
        claim: CONDITIONAL_REPLAY_CLAIM,
        caller_prepared_context_sha256: admitted.context_sha256(),
        checked_records: observed.checked_records(),
        checked_operations: checked.checked_operations,
        selected_paths: source.selected_path_indices().clone(),
        identity_mapping: BTreeMap::from([
            ("base".into(), (native.base.id, native.base.generation, 1)),
            (
                "event".into(),
                (native.event.id, native.event.generation, 1),
            ),
            (
                "argument".into(),
                (native.argument.id, native.argument.generation, 1),
            ),
            ("dispatcher".into(), (native.dispatcher, 1, 1)),
        ]),
    })
}
