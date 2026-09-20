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
use qcore_core::event_observation::{
    EventObservationAction as Action, check_timed_wait_observation,
};
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, EventFlags, EventKey, EventOperation as Op,
    EventProgram, ReactorProfile, check_event_program,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const OBSERVED_WAIT_PROFILE: &str = "cycle18.timed_wait.single_dispatcher.v1";
pub const OBSERVED_WAIT_RULES: &str = "v2: exact eleven identity-bearing records; one dispatcher; timeout-only borrowed argument; no payload, descriptor, task or synchronization; source/context/payload equality; exact pthread/runtime-observation/linkage premises; valid native-run-manifest digest; caller-prepared artifact identity; conditional event kernel replay";
pub const REQUIRED_THREADS: &str = "pthreads";
pub const REQUIRED_OBSERVATION_TRUST: &str = "producer_supplied_runtime_evidence";
pub const REQUIRED_LINKAGE_TRUST: &str = "producer_supplied_selected_artifact_linkage";
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
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    let context = admitted.context();
    if context.assumptions().get("threads").map(String::as_str) != Some(REQUIRED_THREADS)
        || context
            .assumptions()
            .get("observation_trust")
            .map(String::as_str)
            != Some(REQUIRED_OBSERVATION_TRUST)
        || context
            .assumptions()
            .get("linkage_trust")
            .map(String::as_str)
            != Some(REQUIRED_LINKAGE_TRUST)
        || context
            .assumptions()
            .get("native_run_manifest_sha256")
            .is_none_or(|digest| !is_sha256(digest))
    {
        return Err("missing or unsupported conditional replay assumption".into());
    }
    if context.contracts_sha256()
        != qcore_schemas::sha256_hex(
            &serde_json::to_vec(source.contract_digests()).map_err(|e| e.to_string())?,
        )
        || context.source_rules_sha256()
            != qcore_schemas::sha256_hex(
                qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes(),
            )
        || context.observation_rules_sha256()
            != qcore_schemas::sha256_hex(OBSERVED_WAIT_RULES.as_bytes())
    {
        return Err("composition rule or contract identity mismatch".into());
    }
    if qcore_schemas::sha256_hex(observations) != context.observations_sha256()
        || qcore_schemas::sha256_hex(bindings) != context.bindings_sha256()
    {
        return Err("observed composition payload mismatch".into());
    }
    if context.semantic_profile() != OBSERVED_WAIT_PROFILE
        || context.fluent_bit_revision() != source.source_revision()
        || source
            .profile_assumptions()
            .get("profile_digest")
            .map(String::as_str)
            != Some(context.monkey_profile_sha256())
        || context.scope() != &source.selected_path_indices().keys().cloned().collect()
        || context.bounds()
            != &BTreeMap::from([
                ("dispatchers".into(), 1),
                ("events".into(), 1),
                ("records".into(), 11),
                ("runs".into(), 1),
            ])
    {
        return Err("source projection does not match admitted context".into());
    }
    for (name, digest) in source.source_summary_digests() {
        if context.assumptions().get(&format!("source_summary.{name}")) != Some(digest) {
            return Err(format!("source summary context mismatch: {name}"));
        }
    }
    for (name, digest) in source.contract_digests() {
        if context.assumptions().get(&format!("contract.{name}")) != Some(digest) {
            return Err(format!("source contract context mismatch: {name}"));
        }
    }
    let decoded = decode_timed_wait_observation(observations, bindings)?;
    for (path, digest) in &decoded.source_apis {
        if context.assumptions().get(&format!("source_file.{path}")) != Some(digest) {
            return Err(format!("observer source identity mismatch: {path}"));
        }
    }
    let native = &decoded.context;
    if context.assumptions().get("backend") != Some(&native.backend)
        || native.callback != "cb_wait_2_timeout"
        || native.flags != EventFlags::TIMEOUT
        || source.required_observations().len() != 11
    {
        return Err("unsupported observed wait profile".into());
    }
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
