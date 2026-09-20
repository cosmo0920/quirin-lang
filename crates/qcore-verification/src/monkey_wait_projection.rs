//! Sealed source projection for Monkey's bounded timed-wait callback slice.
//!
//! This module derives obligations from independently imported, constructor-
//! private [`SourceSummary`] values. It does not accept booleans claiming that
//! source was checked, and it does not create an event program or certificate.
//! The returned projection states the exact native facts a later composition
//! boundary must bind before Cycle 18 checking can run.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{SourceOperation, SourceSummary};
use qcore_schemas::sha256_hex;
use serde::Serialize;

pub const MONKEY_SOURCE_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
pub const WAIT_CALLBACK: &str = "cb_wait_2_timeout";

const REQUIRED_SUMMARIES: [(&str, &str, usize, usize, &str); 5] = [
    (
        WAIT_CALLBACK,
        "void (evutil_socket_t, short, void *)",
        1,
        0,
        "8308bac83724f6f2417f6d22bbf3e65afbbf491b8ad91b4a9217b1aebd22efdc",
    ),
    (
        "_mk_event_loop_create",
        "void *(int)",
        4,
        3,
        "e0e344755d708e5057a947b2048ef9203d33f71dbc5bdffe7a3a019466af25e8",
    ),
    (
        "_mk_event_wait_with_flags",
        "int (struct mk_event_loop *, int)",
        1,
        0,
        "de51bcb5a96786c5f474afdc4a36bc71a8aa7adaee0875f24eefc828e42bdd58",
    ),
    (
        "_mk_event_wait_2",
        "int (struct mk_event_loop *, int)",
        3,
        2,
        "620ebb4a36f4ae9f33ea6f9e92605769d7d1ce4d7dc75d527d9a5f1245213743",
    ),
    (
        "_mk_event_loop_destroy",
        "void (struct mk_event_ctx *)",
        1,
        0,
        "539a3cf14a83a6f7f8a09bb3370250c0019d354dbd39bb98115fb0930ef093dd",
    ),
];

/// Facts that must come from one independently checked native execution.
/// Source establishes their order and expected values, never their success.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum WaitObservationRequirement {
    BaseCreateSuccess,
    EventCreateSuccess {
        callback: String,
        flags: u16,
        descriptor_absent: bool,
        priority_required: bool,
    },
    EventAddSuccess,
    DispatchBegin,
    TimeoutActivation,
    CallbackEnter,
    CallbackExit,
    DispatchEnd,
    EventFree,
    ArgumentScopeEnd,
    BaseFree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitSourceProjectionError {
    MissingSummary(String),
    DuplicateSummary(String),
    UnexpectedSummary(String),
    SourceRevisionMismatch(String),
    ProfileAssumptionsMismatch(String),
    SignatureMismatch(String),
    PathCountMismatch(String),
    SourceArtifactMismatch(String),
    CallbackIsNotSoleReturn,
    LifecyclePathMismatch(String),
}

/// Constructor-private evidence that the exact pinned source artifacts imply
/// the bounded timed-wait observation obligations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyWaitSourceProjection {
    source_revision: String,
    profile_assumptions: BTreeMap<String, String>,
    source_summary_digests: BTreeMap<String, String>,
    selected_path_indices: BTreeMap<String, usize>,
    required_observations: Vec<WaitObservationRequirement>,
}

impl MonkeyWaitSourceProjection {
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub fn profile_assumptions(&self) -> &BTreeMap<String, String> {
        &self.profile_assumptions
    }

    pub fn source_summary_digests(&self) -> &BTreeMap<String, String> {
        &self.source_summary_digests
    }

    pub fn selected_path_indices(&self) -> &BTreeMap<String, usize> {
        &self.selected_path_indices
    }

    pub fn required_observations(&self) -> &[WaitObservationRequirement] {
        &self.required_observations
    }
}

fn calls(path: &[SourceOperation]) -> Vec<&str> {
    path.iter()
        .filter_map(|operation| match operation {
            SourceOperation::Call { function, .. } => Some(function.as_str()),
            _ => None,
        })
        .collect()
}

fn ends_with_return(path: &[SourceOperation], value: bool) -> bool {
    matches!(
        path.last(),
        Some(SourceOperation::ReturnValue { .. }) if value
    ) || matches!(path.last(), Some(SourceOperation::Return) if !value)
}

fn validate_lifecycle_shape(
    name: &str,
    selected_path: &[SourceOperation],
) -> Result<(), WaitSourceProjectionError> {
    let (expected_calls, returns_value): (&[&str], bool) = match name {
        "_mk_event_loop_create" => (
            &[
                "_mk_event_init",
                "mk_mem_alloc_z",
                "event_base_new",
                "mk_mem_alloc_z",
            ],
            true,
        ),
        "_mk_event_wait_with_flags" => (&["event_base_loop"], true),
        "_mk_event_wait_2" => (
            &[
                "event_new",
                "event_add",
                "_mk_event_wait_with_flags",
                "event_free",
            ],
            true,
        ),
        "_mk_event_loop_destroy" => (
            &["event_base_free", "mk_mem_free", "mk_mem_free"],
            false,
        ),
        _ => return Ok(()),
    };
    if calls(selected_path) != expected_calls || !ends_with_return(selected_path, returns_value) {
        return Err(WaitSourceProjectionError::LifecyclePathMismatch(
            name.to_owned(),
        ));
    }
    Ok(())
}

/// Derive the source half of one timed-wait composition.
///
/// Exactly the five required summaries are accepted. Their serialized digests
/// pin the complete imported operations, including every unselected failure
/// path, so a body substitution cannot pass by preserving only a call list.
pub fn project_monkey_timed_wait_source<'a>(
    summaries: impl IntoIterator<Item = &'a SourceSummary>,
) -> Result<MonkeyWaitSourceProjection, WaitSourceProjectionError> {
    let required_names = REQUIRED_SUMMARIES
        .iter()
        .map(|(name, ..)| *name)
        .collect::<BTreeSet<_>>();
    let mut by_name = BTreeMap::new();
    for summary in summaries {
        let name = summary.function();
        if !required_names.contains(name) {
            return Err(WaitSourceProjectionError::UnexpectedSummary(name.into()));
        }
        if by_name.insert(name, summary).is_some() {
            return Err(WaitSourceProjectionError::DuplicateSummary(name.into()));
        }
    }

    let callback_assumptions = by_name
        .get(WAIT_CALLBACK)
        .ok_or_else(|| WaitSourceProjectionError::MissingSummary(WAIT_CALLBACK.into()))?
        .profile_assumptions();
    let mut common_assumptions = callback_assumptions.clone();
    common_assumptions.remove("typed_import_role");
    if common_assumptions.get("library").map(String::as_str) != Some("monkey")
        || common_assumptions.get("source_set").map(String::as_str) != Some("fluent-bit")
        || common_assumptions
            .get("profile_digest")
            .is_none_or(|digest| digest.is_empty())
        || callback_assumptions.get("typed_import_role").map(String::as_str) != Some("Callback")
    {
        return Err(WaitSourceProjectionError::ProfileAssumptionsMismatch(
            WAIT_CALLBACK.into(),
        ));
    }

    let mut digests = BTreeMap::new();
    let mut selected = BTreeMap::new();
    for (name, signature, path_count, selected_path, expected_digest) in REQUIRED_SUMMARIES {
        let summary = by_name
            .get(name)
            .ok_or_else(|| WaitSourceProjectionError::MissingSummary(name.into()))?;
        if summary.source_revision() != MONKEY_SOURCE_REVISION {
            return Err(WaitSourceProjectionError::SourceRevisionMismatch(name.into()));
        }
        if summary.signature() != signature
            && !(name == WAIT_CALLBACK && summary.signature() == "void (int, short, void *)")
        {
            return Err(WaitSourceProjectionError::SignatureMismatch(name.into()));
        }
        if summary.paths().len() != path_count {
            return Err(WaitSourceProjectionError::PathCountMismatch(name.into()));
        }
        let mut assumptions = summary.profile_assumptions().clone();
        let role = assumptions.remove("typed_import_role");
        if assumptions != common_assumptions
            || role.as_deref()
                != Some(if name == WAIT_CALLBACK {
                    "Callback"
                } else {
                    "Wrapper"
                })
        {
            return Err(WaitSourceProjectionError::ProfileAssumptionsMismatch(
                name.into(),
            ));
        }
        // Profile assumptions are checked separately and may legitimately gain
        // unrelated declarations. The profile importer already enforces the
        // callback signature constraint. The artifact digest pins source
        // identity and every imported operation/path while remaining stable
        // across Clang's typedef/canonical spelling choice for the top-level
        // callback signature.
        let encoded = serde_json::to_vec(&(
            summary.function(),
            summary.source_revision(),
            summary.paths(),
        ))
            .expect("constructor-private typed source summaries always serialize");
        let digest = sha256_hex(&encoded);
        if digest != expected_digest {
            return Err(WaitSourceProjectionError::SourceArtifactMismatch(name.into()));
        }
        validate_lifecycle_shape(name, &summary.paths()[selected_path].operations)?;
        digests.insert(name.into(), digest);
        selected.insert(name.into(), selected_path);
    }

    let callback = by_name[WAIT_CALLBACK];
    if callback.paths()[0].operations.as_slice() != [SourceOperation::Return] {
        return Err(WaitSourceProjectionError::CallbackIsNotSoleReturn);
    }

    Ok(MonkeyWaitSourceProjection {
        source_revision: MONKEY_SOURCE_REVISION.into(),
        profile_assumptions: common_assumptions,
        source_summary_digests: digests,
        selected_path_indices: selected,
        required_observations: vec![
            WaitObservationRequirement::BaseCreateSuccess,
            WaitObservationRequirement::EventCreateSuccess {
                callback: WAIT_CALLBACK.into(),
                flags: 0x01,
                descriptor_absent: true,
                priority_required: true,
            },
            WaitObservationRequirement::EventAddSuccess,
            WaitObservationRequirement::DispatchBegin,
            WaitObservationRequirement::TimeoutActivation,
            WaitObservationRequirement::CallbackEnter,
            WaitObservationRequirement::CallbackExit,
            WaitObservationRequirement::DispatchEnd,
            WaitObservationRequirement::EventFree,
            WaitObservationRequirement::ArgumentScopeEnd,
            WaitObservationRequirement::BaseFree,
        ],
    })
}
