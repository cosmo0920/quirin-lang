//! Join sealed source evidence with an identity-checked, bounded observed run.
//!
//! Success is conditional on producer-supplied observations and the recorded
//! build identity. It is neither an all-path source verdict nor attestation.

use std::collections::{BTreeMap, BTreeSet};
use qcore_core::event_observation::check_timed_wait_observation;
use qcore_core::event_reactor::{ActivationReason, ArgumentMode, CallbackSummary,
    EventFlags, EventKey, EventOperation as Op, EventProgram, ReactorProfile,
    check_event_program};
use serde::Serialize;
use crate::event_composition_context::ValidatedEventCompositionContext;
use crate::monkey_wait_projection::MonkeyWaitSourceProjection;
use crate::timed_wait_observation_wire::decode_timed_wait_observation;

pub const OBSERVED_WAIT_PROFILE: &str = "cycle18.timed_wait.single_dispatcher.v1";
pub const OBSERVED_WAIT_RULES: &str = "v1: exact eleven identity-bearing records; one dispatcher; timeout-only borrowed argument; no payload, descriptor, task or synchronization; source/context/payload equality; event kernel replay";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SingleDispatcher;
impl ReactorProfile for SingleDispatcher {
    fn threading_enabled(self) -> bool { false }
}

/// Constructor-private result of the complete join, with no safety verdict.
#[derive(Debug, Serialize)]
pub struct ObservedWaitComposition {
    context_sha256: String,
    checked_records: usize,
    checked_operations: usize,
    selected_paths: BTreeMap<String, usize>,
    /// Native identities stay full-width. Model identifiers are fresh names
    /// within separate base, event, argument and dispatcher namespaces.
    identity_mapping: BTreeMap<String, (u64, u32, u32)>,
}

pub fn check_observed_wait_composition(
    source: &MonkeyWaitSourceProjection,
    admitted: &ValidatedEventCompositionContext,
    observations: &[u8],
    bindings: &[u8],
) -> Result<ObservedWaitComposition, String> {
    let context = admitted.context();
    if qcore_schemas::sha256_hex(observations) != context.observations_sha256()
        || qcore_schemas::sha256_hex(bindings) != context.bindings_sha256()
    { return Err("observed composition payload mismatch".into()); }
    if context.semantic_profile() != OBSERVED_WAIT_PROFILE
        || context.fluent_bit_revision() != source.source_revision()
        || source.profile_assumptions().get("profile_digest").map(String::as_str)
            != Some(context.monkey_profile_sha256())
        || context.scope() != &source.selected_path_indices().keys().cloned().collect()
        || context.bounds() != &BTreeMap::from([
            ("dispatchers".into(), 1), ("events".into(), 1),
            ("records".into(), 11), ("runs".into(), 1),
        ])
    { return Err("source projection does not match admitted context".into()); }
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
        || native.callback != "cb_wait_2_timeout" || native.flags != EventFlags::TIMEOUT
        || source.required_observations().len() != 11
    { return Err("unsupported observed wait profile".into()); }
    let observed = check_timed_wait_observation(&decoded.trace, native)
        .map_err(|error| format!("observation refused: {error:?}"))?;
    // There is exactly one identity per typed namespace. Assigning the fresh
    // model name 1 is injective on each singleton and never truncates a pointer.
    let event = EventKey::new(1, native.event.generation);
    let program = EventProgram {
        live_arguments: BTreeSet::from([1]),
        operations: vec![
            Op::CreateBase { base: 1, profile: SingleDispatcher },
            Op::Assign { event, base: 1, descriptor: None,
                flags: EventFlags::new(native.flags), priority: native.priority,
                summary: CallbackSummary { argument: 1, argument_mode: ArgumentMode::Borrowed,
                    trigger_reasons: BTreeSet::from([ActivationReason::Timeout]),
                    payload_authority: BTreeMap::new(), may_defer: false, may_spawn_tasks: false } },
            Op::Add { event }, Op::BeginDispatch { base: 1, dispatcher: 1 },
            Op::Activate { event, reason: ActivationReason::Timeout, cross_thread: false },
            Op::CallbackEnter { event, reason: ActivationReason::Timeout },
            Op::CallbackExit { event }, Op::EndDispatch { base: 1 },
            Op::FreeEvent { event }, Op::DropArgument { argument: 1 }, Op::FreeBase { base: 1 },
        ],
    };
    let checked = check_event_program(&program, &[SingleDispatcher])
        .map_err(|errors| format!("event program refused: {errors:?}"))?;
    if checked.checked_operations != observed.checked_records()
        || checked.freed_events != BTreeSet::from([event])
        || checked.dropped_arguments != BTreeSet::from([1])
    { return Err("event program did not discharge the observed lifecycle".into()); }
    Ok(ObservedWaitComposition {
        context_sha256: admitted.context_sha256(),
        checked_records: observed.checked_records(), checked_operations: checked.checked_operations,
        selected_paths: source.selected_path_indices().clone(),
        identity_mapping: BTreeMap::from([
            ("base".into(), (native.base.id, native.base.generation, 1)),
            ("event".into(), (native.event.id, native.event.generation, 1)),
            ("argument".into(), (native.argument.id, native.argument.generation, 1)),
            ("dispatcher".into(), (native.dispatcher, 1, 1)),
        ]),
    })
}
