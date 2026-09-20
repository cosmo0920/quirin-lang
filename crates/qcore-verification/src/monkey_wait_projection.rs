//! Sealed source projection for Monkey's bounded timed-wait callback slice.
//!
//! This module derives obligations from independently imported, constructor-
//! private [`SourceSummary`] values. It does not accept booleans claiming that
//! source was checked, and it does not create an event program or certificate.
//! The returned projection states the exact native facts a later composition
//! boundary must bind before Cycle 18 checking can run.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{SourceOperation, SourceSummary};
use qcore_core::usage::Usage;
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use qcore_frontend::lexer::TokenKind;
use qcore_frontend::source_extern::normalize_type;
use qcore_schemas::sha256_hex;
use serde::Serialize;

pub const MONKEY_SOURCE_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
pub const WAIT_CALLBACK: &str = "cb_wait_2_timeout";

const REQUIRED_SUMMARIES: [(&str, &str, usize, usize); 5] = [
    (WAIT_CALLBACK, "void (evutil_socket_t, short, void *)", 1, 0),
    ("_mk_event_loop_create", "void *(int)", 4, 3),
    (
        "_mk_event_wait_with_flags",
        "int (struct mk_event_loop *, int)",
        1,
        0,
    ),
    (
        "_mk_event_wait_2",
        "int (struct mk_event_loop *, int)",
        3,
        2,
    ),
    (
        "_mk_event_loop_destroy",
        "void (struct mk_event_ctx *)",
        1,
        0,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitCallbackSemantics {
    BorrowedNoEffectNoTasks,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitSourceProjectionError {
    ContractParse(String),
    MissingContractDeclaration(String),
    DuplicateContractDeclaration(String),
    MissingContractBlock(String),
    ContractHeaderMismatch(String),
    ContractClauseMismatch(String),
    MissingSummary(String),
    DuplicateSummary(String),
    UnexpectedSummary(String),
    SourceRevisionMismatch(String),
    ProfileAssumptionsMismatch(String),
    SignatureMismatch(String),
    PathCountMismatch(String),
    SourceShapeMismatch(String),
    SourceArtifactMismatch(String),
    CallbackIsNotSoleReturn,
    LifecyclePathMismatch(String),
}

fn approved_summary_digests(name: &str) -> &'static [&'static str] {
    match name {
        WAIT_CALLBACK => &["8308bac83724f6f2417f6d22bbf3e65afbbf491b8ad91b4a9217b1aebd22efdc"],
        "_mk_event_loop_create" => &[
            // Existing focused fixture.
            "2e32d6c7bbc40031c0f9828031d1591478cc2d556c2504dcf4f2687b0943214d",
            // Pinned production translation unit recorded by this card.
            "d1c48805630eacec138b0ed1eae834195815b9e4ce61a731c40ac3fd46bc7756",
        ],
        "_mk_event_wait_with_flags" => {
            &["3637e3f8ddfb6ae389d571a05f13bb9e32e675e95bfb4d5134ee8ab73923ff1f"]
        }
        "_mk_event_wait_2" => &[
            // Existing focused fixture.
            "88d9d6f2d7dad652503bd218ac9fad80c8c16afc53beb8ec0e174e2232628dff",
            // Pinned production translation unit recorded by this card.
            "69511d42a65bbed00f7c54e2890cc641578cf60029260893a8aa2db585bca4bd",
        ],
        "_mk_event_loop_destroy" => {
            &["c0d3215c02ed7f421f725596fbcee27547d0cf2a5f8534aa532f794cbd9a5e0e"]
        }
        _ => &[],
    }
}

/// Parsed declarations that state the contract-expressible half of the slice.
///
/// Construction is total-or-reject over the five declarations used by this
/// projection. The declarations remain assumptions until their clauses are
/// checked against the independently imported source summaries below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyWaitContracts {
    contract_digests: BTreeMap<String, String>,
}

impl MonkeyWaitContracts {
    pub fn parse(
        wrapper_contract: &str,
        composition_contract: &str,
    ) -> Result<Self, WaitSourceProjectionError> {
        let wrapper = parse_source_program(wrapper_contract)
            .map_err(|error| WaitSourceProjectionError::ContractParse(error.to_string()))?;
        let composition = parse_source_program(composition_contract)
            .map_err(|error| WaitSourceProjectionError::ContractParse(error.to_string()))?;

        check_contract_declaration(
            &wrapper,
            "_mk_event_loop_create",
            ContractHeader {
                parameters: &[("size", "i32", false)],
                result: "ptr<mut,EventContext>",
            },
            &[
                (
                    ClauseExit::Requires,
                    "selected_unix_libevent_source_profile",
                ),
                (
                    ClauseExit::Normal,
                    "initialization_or_context_allocation_failure_returns_null",
                ),
                (
                    ClauseExit::Normal,
                    "event_base_new_result_is_stored_without_a_null_test",
                ),
                (
                    ClauseExit::Normal,
                    "fired_allocation_failure_frees_context_without_base_free",
                ),
                (
                    ClauseExit::Normal,
                    "successful_return_sets_queue_size_and_returns_context",
                ),
            ],
        )?;
        check_contract_declaration(
            &wrapper,
            "_mk_event_wait_with_flags",
            ContractHeader {
                parameters: &[
                    ("loop", "borrow<'call,mut,EventLoop>", true),
                    ("flags", "i32", false),
                ],
                result: "i32",
            },
            &[
                (
                    ClauseExit::Requires,
                    "selected_unix_libevent_source_profile",
                ),
                (ClauseExit::Normal, "fired_count_is_cleared_before_dispatch"),
                (ClauseExit::Normal, "dispatch_result_is_ignored"),
                (
                    ClauseExit::Normal,
                    "final_fired_count_is_copied_to_loop_and_returned",
                ),
            ],
        )?;
        check_contract_declaration(
            &wrapper,
            "_mk_event_wait_2",
            ContractHeader {
                parameters: &[
                    ("loop", "borrow<'call,mut,EventLoop>", true),
                    ("timeout", "i32", false),
                ],
                result: "i32",
            },
            &[
                (
                    ClauseExit::Requires,
                    "selected_unix_libevent_source_profile",
                ),
                (
                    ClauseExit::Normal,
                    "infinite_and_zero_wait_paths_delegate_without_timer_creation",
                ),
                (ClauseExit::Normal, "timed_wait_uses_explicit_stack_timeval"),
                (
                    ClauseExit::Normal,
                    "timeout_event_creation_and_registration_results_are_not_checked",
                ),
                (
                    ClauseExit::Normal,
                    "timed_wait_calls_event_free_after_dispatch_and_returns_dispatch_result",
                ),
            ],
        )?;
        check_contract_declaration(
            &wrapper,
            "_mk_event_loop_destroy",
            ContractHeader {
                parameters: &[("ctx", "own<EventContext>", true)],
                result: "unit",
            },
            &[
                (
                    ClauseExit::Requires,
                    "selected_unix_libevent_source_profile",
                ),
                (
                    ClauseExit::Requires,
                    "context_and_children_have_valid_destruction_provenance",
                ),
                (
                    ClauseExit::Normal,
                    "calls_base_free_then_fired_free_then_context_free",
                ),
                (
                    ClauseExit::Normal,
                    "nested_event_and_map_custody_still_requires_composition",
                ),
            ],
        )?;
        check_contract_declaration(
            &composition,
            WAIT_CALLBACK,
            ContractHeader {
                parameters: &[(
                    "callback_argument",
                    "borrow<'callback,const,MonkeyCallbackArgument>",
                    true,
                )],
                result: "unit",
            },
            &[
                (ClauseExit::Requires, "callback_argument_is_borrowed"),
                (ClauseExit::Normal, "callback_has_no_resource_effect"),
            ],
        )?;

        Ok(Self {
            contract_digests: BTreeMap::from([
                (
                    "monkey/wrapper_paths.qcore".into(),
                    sha256_hex(wrapper_contract.as_bytes()),
                ),
                (
                    "monkey/composition.qcore".into(),
                    sha256_hex(composition_contract.as_bytes()),
                ),
            ]),
        })
    }

    pub fn contract_digests(&self) -> &BTreeMap<String, String> {
        &self.contract_digests
    }

    fn required_observations(&self) -> Vec<WaitObservationRequirement> {
        vec![
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
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ClauseExit {
    Requires,
    Normal,
}

#[derive(Clone, Copy)]
struct ContractHeader<'a> {
    parameters: &'a [(&'a str, &'a str, bool)],
    result: &'a str,
}

fn check_contract_declaration(
    program: &qcore_frontend::frontend::SourceProgram,
    name: &str,
    header: ContractHeader<'_>,
    expected: &[(ClauseExit, &str)],
) -> Result<(), WaitSourceProjectionError> {
    let declarations: Vec<_> = program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.name == name)
        .collect();
    let declaration = match declarations.as_slice() {
        [] => {
            return Err(WaitSourceProjectionError::MissingContractDeclaration(
                name.into(),
            ));
        }
        [declaration] => declaration,
        _ => {
            return Err(WaitSourceProjectionError::DuplicateContractDeclaration(
                name.into(),
            ));
        }
    };
    let contract = declaration
        .value
        .function_contract
        .as_ref()
        .ok_or_else(|| WaitSourceProjectionError::MissingContractBlock(name.into()))?;
    let header_matches = declaration.value.kind == DeclarationKind::ExternFunction
        && declaration.value.body.is_none()
        && !declaration.value.tokens.iter().any(
            |token| matches!(token, TokenKind::Word(word) if word == "unsafe" || word == "effects"),
        )
        && declaration.value.parameters.len() == header.parameters.len()
        && declaration
            .value
            .parameters
            .iter()
            .zip(header.parameters)
            .all(|(actual, (expected_name, expected_type, requires_one))| {
                actual.name.value == *expected_name
                    && normalize_type(&actual.type_source) == *expected_type
                    && actual.usage.as_ref().map(|usage| usage.value)
                        == requires_one.then_some(Usage::ONE)
            })
        && declaration
            .value
            .result_type
            .as_ref()
            .is_some_and(|result| normalize_type(&result.source) == header.result);
    if !header_matches {
        return Err(WaitSourceProjectionError::ContractHeaderMismatch(
            name.into(),
        ));
    }
    let actual = contract
        .clauses
        .iter()
        .map(|clause| {
            let exit = match &clause.value.kind {
                SourceContractClauseKind::Requires => ClauseExit::Requires,
                SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                    ClauseExit::Normal
                }
                _ => return None,
            };
            Some((exit, clause.value.expression_source.trim()))
        })
        .collect::<Option<Vec<_>>>();
    if actual.as_deref() != Some(expected) {
        return Err(WaitSourceProjectionError::ContractClauseMismatch(
            name.into(),
        ));
    }
    Ok(())
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
    contract_digests: BTreeMap<String, String>,
    callback_semantics: WaitCallbackSemantics,
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

    pub fn contract_digests(&self) -> &BTreeMap<String, String> {
        &self.contract_digests
    }

    pub fn callback_semantics(&self) -> WaitCallbackSemantics {
        self.callback_semantics
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
        "_mk_event_loop_destroy" => (&["event_base_free", "mk_mem_free", "mk_mem_free"], false),
        _ => return Ok(()),
    };
    if calls(selected_path) != expected_calls || !ends_with_return(selected_path, returns_value) {
        return Err(WaitSourceProjectionError::LifecyclePathMismatch(
            name.to_owned(),
        ));
    }
    Ok(())
}

fn validate_complete_shape(
    name: &str,
    summary: &SourceSummary,
) -> Result<(), WaitSourceProjectionError> {
    let expected: &[(&[&str], usize)] = match name {
        WAIT_CALLBACK => &[(&[], 1)],
        "_mk_event_loop_create" => &[
            (&["_mk_event_init"], 4),
            (&["_mk_event_init", "mk_mem_alloc_z"], 7),
            (
                &[
                    "_mk_event_init",
                    "mk_mem_alloc_z",
                    "event_base_new",
                    "mk_mem_alloc_z",
                    "mk_mem_free",
                ],
                14,
            ),
            (
                &[
                    "_mk_event_init",
                    "mk_mem_alloc_z",
                    "event_base_new",
                    "mk_mem_alloc_z",
                ],
                14,
            ),
        ],
        "_mk_event_wait_with_flags" => &[(&["event_base_loop"], 10)],
        "_mk_event_wait_2" => &[
            (&["_mk_event_wait_with_flags"], 13),
            (&["_mk_event_wait_with_flags"], 14),
            (
                &[
                    "event_new",
                    "event_add",
                    "_mk_event_wait_with_flags",
                    "event_free",
                ],
                20,
            ),
        ],
        "_mk_event_loop_destroy" => &[(&["event_base_free", "mk_mem_free", "mk_mem_free"], 6)],
        _ => &[],
    };
    if summary.paths().len() != expected.len()
        || summary
            .paths()
            .iter()
            .zip(expected)
            .any(|(path, (expected_calls, operation_count))| {
                path.operations.len() != *operation_count
                    || calls(&path.operations) != *expected_calls
            })
    {
        return Err(WaitSourceProjectionError::SourceShapeMismatch(name.into()));
    }
    Ok(())
}

/// Derive the source half of one timed-wait composition.
///
/// Exactly the five required summaries are accepted. All source paths retain
/// their expected operation counts and call order. Canonical digests of the
/// complete imported paths are exposed so the outer source-identity audit can
/// bind these obligations to its independently pinned translation unit.
pub fn project_monkey_timed_wait_source<'a>(
    summaries: impl IntoIterator<Item = &'a SourceSummary>,
    contracts: &MonkeyWaitContracts,
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
        || callback_assumptions
            .get("typed_import_role")
            .map(String::as_str)
            != Some("Callback")
    {
        return Err(WaitSourceProjectionError::ProfileAssumptionsMismatch(
            WAIT_CALLBACK.into(),
        ));
    }

    let mut digests = BTreeMap::new();
    let mut selected = BTreeMap::new();
    for (name, signature, path_count, selected_path) in REQUIRED_SUMMARIES {
        let summary = by_name
            .get(name)
            .ok_or_else(|| WaitSourceProjectionError::MissingSummary(name.into()))?;
        if summary.source_revision() != MONKEY_SOURCE_REVISION {
            return Err(WaitSourceProjectionError::SourceRevisionMismatch(
                name.into(),
            ));
        }
        if summary.signature() != signature
            && !(name == WAIT_CALLBACK && summary.signature() == "void (int, short, void *)")
        {
            return Err(WaitSourceProjectionError::SignatureMismatch(name.into()));
        }
        if summary.paths().len() != path_count {
            return Err(WaitSourceProjectionError::PathCountMismatch(name.into()));
        }
        validate_complete_shape(name, summary)?;
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
        if !approved_summary_digests(name).contains(&digest.as_str()) {
            return Err(WaitSourceProjectionError::SourceArtifactMismatch(
                name.into(),
            ));
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
        required_observations: contracts.required_observations(),
        contract_digests: contracts.contract_digests.clone(),
        callback_semantics: WaitCallbackSemantics::BorrowedNoEffectNoTasks,
    })
}
