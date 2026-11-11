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

//! Sealed source projection for Monkey's bounded timed-wait callback slice.
//!
//! This module derives obligations from independently imported, constructor-
//! private [`SourceSummary`] values. It does not accept booleans claiming that
//! source was checked, and it does not create an event program or certificate.
//! The returned projection states the exact native facts a later composition
//! boundary must bind before Cycle 18 checking can run.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use qcore_c_gateway::typed_source::{SourceOperation, SourceSummary};
use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_core::usage::Usage;
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use qcore_frontend::lexer::TokenKind;
use qcore_frontend::source_extern::normalize_type;
use qcore_schemas::sha256_hex;
use serde::Serialize;

mod wait_failure_paths;
pub use wait_failure_paths::{
    CheckedWaitFailureCase, WaitFailureError, WaitFailureEvidenceClass, WaitFailureObservation,
    WaitFailureOutcome, check_wait_failure_paths, wait_failure_policy_digest,
};

const WAIT_SOURCE_POLICY: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/wait_source_policy.qv"
);
const SOURCE_IDENTITY_POLICY: &str = "source_identity";
const PROJECTION_IDENTITY_POLICY: &str = "wait_projection_identity";
const ARTIFACT_SET_POLICY: &str = "artifact_set";

fn bundled_text_fact(function: &str, fact: &str) -> String {
    match contract_fact_literal(WAIT_SOURCE_POLICY, function, fact) {
        Ok(ContractFact::Text(value)) => value,
        Ok(ContractFact::Integer(_)) => {
            panic!("bundled Monkey wait policy fact `{fact}` is not text")
        }
        Err(error) => panic!("bundled Monkey wait policy is invalid: {error}"),
    }
}

static BUNDLED_SOURCE_REVISION: LazyLock<String> =
    LazyLock::new(|| bundled_text_fact(SOURCE_IDENTITY_POLICY, "source_revision"));
static BUNDLED_WAIT_CALLBACK: LazyLock<String> =
    LazyLock::new(|| bundled_text_fact(PROJECTION_IDENTITY_POLICY, "wait_callback"));

/// Pinned revision declared by the bundled executable source policy.
pub fn monkey_source_revision() -> &'static str {
    &BUNDLED_SOURCE_REVISION
}

/// Timed-wait callback declared by the bundled executable source policy.
pub fn wait_callback() -> &'static str {
    &BUNDLED_WAIT_CALLBACK
}

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
    SourcePolicy(String),
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

/// Parsed declarations that state the contract-expressible half of the slice.
///
/// Construction is total-or-reject over the five declarations used by this
/// projection. The declarations remain assumptions until their clauses are
/// checked against the independently imported source summaries below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyWaitContracts {
    contract_digests: BTreeMap<String, String>,
    source_policy: String,
    source_functions: BTreeSet<String>,
    source_revision: String,
    wait_callback: String,
    event_flags: u16,
    descriptor_absent: bool,
    priority_required: bool,
}

fn policy_text_fact(
    source: &str,
    function: &str,
    fact: &str,
) -> Result<String, WaitSourceProjectionError> {
    match contract_fact_literal(source, function, fact)
        .map_err(WaitSourceProjectionError::SourcePolicy)?
    {
        ContractFact::Text(value) => Ok(value),
        ContractFact::Integer(_) => Err(WaitSourceProjectionError::SourcePolicy(format!(
            "policy fact `{fact}` must be text"
        ))),
    }
}

fn policy_integer_fact(
    source: &str,
    function: &str,
    fact: &str,
) -> Result<i128, WaitSourceProjectionError> {
    match contract_fact_literal(source, function, fact)
        .map_err(WaitSourceProjectionError::SourcePolicy)?
    {
        ContractFact::Integer(value) => Ok(value),
        ContractFact::Text(_) => Err(WaitSourceProjectionError::SourcePolicy(format!(
            "policy fact `{fact}` must be an integer"
        ))),
    }
}

impl MonkeyWaitContracts {
    pub fn parse(
        wrapper_contract: &str,
        composition_contract: &str,
    ) -> Result<Self, WaitSourceProjectionError> {
        Self::parse_with_source_policy(wrapper_contract, composition_contract, WAIT_SOURCE_POLICY)
    }

    fn parse_with_source_policy(
        wrapper_contract: &str,
        composition_contract: &str,
        source_policy: &str,
    ) -> Result<Self, WaitSourceProjectionError> {
        let wrapper = parse_source_program(wrapper_contract)
            .map_err(|error| WaitSourceProjectionError::ContractParse(error.to_string()))?;
        let composition = parse_source_program(composition_contract)
            .map_err(|error| WaitSourceProjectionError::ContractParse(error.to_string()))?;
        let policy = parse_source_program(source_policy)
            .map_err(|error| WaitSourceProjectionError::SourcePolicy(error.to_string()))?;
        let source_revision =
            policy_text_fact(source_policy, SOURCE_IDENTITY_POLICY, "source_revision")?;
        let wait_callback =
            policy_text_fact(source_policy, PROJECTION_IDENTITY_POLICY, "wait_callback")?;
        let event_flags =
            policy_integer_fact(source_policy, PROJECTION_IDENTITY_POLICY, "event_flags")?;
        let descriptor_absent = policy_integer_fact(
            source_policy,
            PROJECTION_IDENTITY_POLICY,
            "descriptor_absent",
        )?;
        let priority_required = policy_integer_fact(
            source_policy,
            PROJECTION_IDENTITY_POLICY,
            "priority_required",
        )?;
        let event_flags = u16::try_from(event_flags).map_err(|_| {
            WaitSourceProjectionError::SourcePolicy("event_flags is outside u16".into())
        })?;
        let policy_bool = |name: &str, value: i128| {
            if value == 0 || value == 1 {
                Ok(value != 0)
            } else {
                Err(WaitSourceProjectionError::SourcePolicy(format!(
                    "{name} must be zero or one"
                )))
            }
        };
        let descriptor_absent = policy_bool("descriptor_absent", descriptor_absent)?;
        let priority_required = policy_bool("priority_required", priority_required)?;
        // Validate policy-derived projection values eagerly. Source identity
        // is evaluated later against independently imported summary facts.
        check_typed_contract_facts(
            source_policy,
            PROJECTION_IDENTITY_POLICY,
            &[
                ("wait_callback", ContractFact::Text(wait_callback.clone())),
                (
                    "event_flags",
                    ContractFact::Integer(i128::from(event_flags)),
                ),
                (
                    "descriptor_absent",
                    ContractFact::Integer(i128::from(descriptor_absent)),
                ),
                (
                    "priority_required",
                    ContractFact::Integer(i128::from(priority_required)),
                ),
            ],
        )
        .map_err(WaitSourceProjectionError::SourcePolicy)?;
        let reserved = [
            SOURCE_IDENTITY_POLICY,
            PROJECTION_IDENTITY_POLICY,
            ARTIFACT_SET_POLICY,
        ];
        let source_functions = policy
            .declarations
            .iter()
            .map(|declaration| declaration.value.name.clone())
            .filter(|name| !reserved.contains(&name.as_str()) && !name.starts_with("contract_"))
            .collect::<BTreeSet<_>>();
        if source_functions.is_empty()
            || reserved.iter().any(|name| {
                policy
                    .declarations
                    .iter()
                    .filter(|declaration| declaration.value.name == *name)
                    .count()
                    != 1
            })
        {
            return Err(WaitSourceProjectionError::SourcePolicy(
                "source policy has an incomplete or duplicate declaration set".into(),
            ));
        }
        for name in &source_functions {
            check_contract_declaration([&wrapper, &composition], source_policy, name)?;
        }

        Ok(Self {
            contract_digests: BTreeMap::from([
                (
                    "monkey/wrapper_paths.qv".into(),
                    sha256_hex(wrapper_contract.as_bytes()),
                ),
                (
                    "monkey/composition.qv".into(),
                    sha256_hex(composition_contract.as_bytes()),
                ),
                (
                    "monkey/wait_source_policy.qv".into(),
                    sha256_hex(source_policy.as_bytes()),
                ),
            ]),
            source_policy: source_policy.into(),
            source_functions,
            source_revision,
            wait_callback,
            event_flags,
            descriptor_absent,
            priority_required,
        })
    }

    pub fn contract_digests(&self) -> &BTreeMap<String, String> {
        &self.contract_digests
    }

    fn required_observations(&self) -> Vec<WaitObservationRequirement> {
        vec![
            WaitObservationRequirement::BaseCreateSuccess,
            WaitObservationRequirement::EventCreateSuccess {
                callback: self.wait_callback.clone(),
                flags: self.event_flags,
                descriptor_absent: self.descriptor_absent,
                priority_required: self.priority_required,
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

fn check_contract_declaration(
    programs: [&qcore_frontend::frontend::SourceProgram; 2],
    source_policy: &str,
    name: &str,
) -> Result<(), WaitSourceProjectionError> {
    let declarations: Vec<_> = programs
        .into_iter()
        .flat_map(|program| &program.declarations)
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
    let parameter_names = declaration
        .value
        .parameters
        .iter()
        .map(|parameter| parameter.name.value.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let parameter_types = declaration
        .value
        .parameters
        .iter()
        .map(|parameter| normalize_type(&parameter.type_source))
        .collect::<Vec<_>>()
        .join(",");
    let parameter_usages = declaration
        .value
        .parameters
        .iter()
        .map(
            |parameter| match parameter.usage.as_ref().map(|usage| usage.value) {
                None => "none",
                Some(Usage::ONE) => "one",
                Some(_) => "other",
            },
        )
        .collect::<Vec<_>>()
        .join(",");
    let result_type = declaration
        .value
        .result_type
        .as_ref()
        .map(|result| normalize_type(&result.source))
        .unwrap_or_default();
    let clause_sequence = contract
        .clauses
        .iter()
        .map(|clause| {
            let exit = match &clause.value.kind {
                SourceContractClauseKind::Requires => "requires",
                SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => "normal",
                _ => return None,
            };
            Some(format!("{exit}:{}", clause.value.expression_source.trim()))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| WaitSourceProjectionError::ContractClauseMismatch(name.into()))?
        .join("|");
    let policy_function = format!("contract_{name}");
    let actual_facts = vec![
        (
            "declaration_is_extern",
            ContractFact::Integer(i128::from(
                declaration.value.kind == DeclarationKind::ExternFunction,
            )),
        ),
        (
            "declaration_has_body",
            ContractFact::Integer(i128::from(declaration.value.body.is_some())),
        ),
        (
            "declaration_is_unsafe",
            ContractFact::Integer(i128::from(
                declaration
                    .value
                    .tokens
                    .iter()
                    .any(|token| matches!(token, TokenKind::Word(word) if word == "unsafe")),
            )),
        ),
        (
            "declaration_has_effects",
            ContractFact::Integer(i128::from(
                declaration
                    .value
                    .tokens
                    .iter()
                    .any(|token| matches!(token, TokenKind::Word(word) if word == "effects")),
            )),
        ),
        (
            "parameter_names",
            ContractFact::Text(parameter_names.clone()),
        ),
        (
            "parameter_types",
            ContractFact::Text(parameter_types.clone()),
        ),
        (
            "parameter_usages",
            ContractFact::Text(parameter_usages.clone()),
        ),
        ("result_type", ContractFact::Text(result_type.clone())),
        ("clause_sequence", ContractFact::Text(clause_sequence)),
    ];
    if check_typed_contract_facts(source_policy, &policy_function, &actual_facts).is_err() {
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
    source_path_counts: BTreeMap<String, usize>,
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

    /// Complete imported path counts, including the failure paths not selected
    /// by the earlier success-observation projection.
    pub fn source_path_counts(&self) -> &BTreeMap<String, usize> {
        &self.source_path_counts
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

fn call_sequence(path: &[SourceOperation]) -> String {
    calls(path).join(",")
}

fn return_kind(path: &[SourceOperation]) -> &'static str {
    match path.last() {
        Some(SourceOperation::Return) => "return",
        Some(SourceOperation::ReturnValue { .. }) => "return_value",
        _ => "other",
    }
}

fn summary_policy_facts(
    summary: &SourceSummary,
    import_role: &str,
    selected_path_index: usize,
) -> Vec<(&'static str, ContractFact)> {
    let selected = &summary.paths()[selected_path_index].operations;
    vec![
        ("signature", ContractFact::Text(summary.signature().into())),
        (
            "path_count",
            ContractFact::Integer(summary.paths().len() as i128),
        ),
        ("import_role", ContractFact::Text(import_role.into())),
        (
            "path_operation_counts",
            ContractFact::Text(
                summary
                    .paths()
                    .iter()
                    .map(|path| path.operations.len().to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ),
        (
            "path_call_sequences",
            ContractFact::Text(
                summary
                    .paths()
                    .iter()
                    .map(|path| call_sequence(&path.operations))
                    .collect::<Vec<_>>()
                    .join(";"),
            ),
        ),
        (
            "selected_path_index",
            ContractFact::Integer(selected_path_index as i128),
        ),
        (
            "selected_operation_count",
            ContractFact::Integer(selected.len() as i128),
        ),
        (
            "selected_calls",
            ContractFact::Text(call_sequence(selected)),
        ),
        (
            "selected_return_kind",
            ContractFact::Text(return_kind(selected).into()),
        ),
    ]
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
    let mut by_name = BTreeMap::new();
    for summary in summaries {
        let name = summary.function();
        if !contracts.source_functions.contains(name) {
            return Err(WaitSourceProjectionError::UnexpectedSummary(name.into()));
        }
        if by_name.insert(name, summary).is_some() {
            return Err(WaitSourceProjectionError::DuplicateSummary(name.into()));
        }
    }

    for name in &contracts.source_functions {
        if !by_name.contains_key(name.as_str()) {
            return Err(WaitSourceProjectionError::MissingSummary(name.clone()));
        }
    }
    let first = contracts
        .source_functions
        .iter()
        .next()
        .and_then(|name| by_name.get(name.as_str()))
        .expect("a parsed wait policy has at least one source function");
    let mut common_assumptions = first.profile_assumptions().clone();
    common_assumptions.remove("typed_import_role");

    let mut digests = BTreeMap::new();
    let mut selected = BTreeMap::new();
    let mut path_counts = BTreeMap::new();
    for name in &contracts.source_functions {
        let summary = by_name[name.as_str()];
        path_counts.insert(name.clone(), summary.paths().len());
        let mut assumptions = summary.profile_assumptions().clone();
        let role = assumptions
            .remove("typed_import_role")
            .ok_or_else(|| WaitSourceProjectionError::ProfileAssumptionsMismatch(name.clone()))?;
        if assumptions != common_assumptions {
            return Err(WaitSourceProjectionError::ProfileAssumptionsMismatch(
                name.clone(),
            ));
        }
        check_typed_contract_facts(
            &contracts.source_policy,
            SOURCE_IDENTITY_POLICY,
            &[
                (
                    "source_revision",
                    ContractFact::Text(summary.source_revision().into()),
                ),
                (
                    "library",
                    ContractFact::Text(assumptions.get("library").cloned().unwrap_or_default()),
                ),
                (
                    "source_set",
                    ContractFact::Text(assumptions.get("source_set").cloned().unwrap_or_default()),
                ),
                (
                    "profile_digest",
                    ContractFact::Text(
                        assumptions
                            .get("profile_digest")
                            .cloned()
                            .unwrap_or_default(),
                    ),
                ),
            ],
        )
        .map_err(|_| WaitSourceProjectionError::SourceRevisionMismatch(name.clone()))?;

        let accepted_paths = summary
            .paths()
            .iter()
            .enumerate()
            .filter_map(|(path_index, _)| {
                check_typed_contract_facts(
                    &contracts.source_policy,
                    name,
                    &summary_policy_facts(summary, &role, path_index),
                )
                .is_ok()
                .then_some(path_index)
            })
            .collect::<Vec<_>>();
        let [selected_path] = accepted_paths.as_slice() else {
            return Err(WaitSourceProjectionError::SourceShapeMismatch(name.clone()));
        };
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
        digests.insert(name.clone(), digest);
        selected.insert(name.clone(), *selected_path);
    }
    let digest_facts = digests
        .iter()
        .map(|(name, digest)| (format!("digest_{name}"), ContractFact::Text(digest.clone())))
        .collect::<Vec<_>>();
    let digest_fact_refs = digest_facts
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect::<Vec<_>>();
    check_typed_contract_facts(
        &contracts.source_policy,
        ARTIFACT_SET_POLICY,
        &digest_fact_refs,
    )
    .map_err(|_| {
        WaitSourceProjectionError::SourceArtifactMismatch("selected summary set".into())
    })?;

    Ok(MonkeyWaitSourceProjection {
        source_revision: contracts.source_revision.clone(),
        profile_assumptions: common_assumptions,
        source_summary_digests: digests,
        selected_path_indices: selected,
        source_path_counts: path_counts,
        required_observations: contracts.required_observations(),
        contract_digests: contracts.contract_digests.clone(),
        callback_semantics: WaitCallbackSemantics::BorrowedNoEffectNoTasks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use qcore_c_gateway::legacy_c::C11TranslationUnit;
    use qcore_c_gateway::typed_source::{SourceResult, import_source_functions_from_profile};
    use qcore_contract::profile_loader::load_source_set_profile;
    use qcore_schemas::profile::ImportRole;

    const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
    const WRAPPERS: &str = include_str!("../../../compiler/tests/fixtures/monkey/wrappers.c");
    const WRAPPER_CONTRACT: &str = include_str!(
        "../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv"
    );
    const COMPOSITION_CONTRACT: &str =
        include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/composition.qv");

    fn wrapper_source() -> String {
        format!(
            "{WRAPPERS}\n{}",
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
        )
    }

    fn imported(source: &str, role: ImportRole) -> Vec<SourceResult> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let profile =
            load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
        import_source_functions_from_profile(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            monkey_source_revision(),
            &profile,
            "monkey",
            role,
        )
        .unwrap()
    }

    fn summary<'a>(results: &'a [SourceResult], name: &str) -> &'a SourceSummary {
        results
            .iter()
            .find_map(|result| match result {
                SourceResult::Summary(summary) if summary.function() == name => Some(summary),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing summary {name}"))
    }

    fn replace_occurrence(
        source: &str,
        needle: &str,
        occurrence: usize,
        replacement: &str,
    ) -> String {
        let (offset, _) = source
            .match_indices(needle)
            .nth(occurrence)
            .unwrap_or_else(|| panic!("missing occurrence {occurrence} of `{needle}`"));
        let mut changed = String::with_capacity(source.len() - needle.len() + replacement.len());
        changed.push_str(&source[..offset]);
        changed.push_str(replacement);
        changed.push_str(&source[offset + needle.len()..]);
        changed
    }

    #[test]
    fn contract_policy_checks_one_full_actual_fact_vector() {
        let expected_clause =
            "requires:callback_argument_is_borrowed|normal:callback_has_no_resource_effect";
        let policy = WAIT_SOURCE_POLICY
            .replace(
                "    ensures normal parameter_names == \"callback_argument\";\n",
                "",
            )
            .replace(
                &format!("    ensures normal clause_sequence == \"{expected_clause}\";\n"),
                &format!(
                    "    ensures normal (parameter_names == \"callback_argument\") or (clause_sequence == \"{expected_clause}\");\n"
                ),
            );
        let changed = COMPOSITION_CONTRACT.replace("callback_argument", "changed_argument");
        assert_eq!(
            MonkeyWaitContracts::parse_with_source_policy(WRAPPER_CONTRACT, &changed, &policy,),
            Err(WaitSourceProjectionError::ContractClauseMismatch(
                wait_callback().into()
            ))
        );
    }

    #[test]
    fn every_source_policy_declaration_and_clause_is_load_bearing() {
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let callbacks = imported(CALLBACKS, ImportRole::Callback);
        let wrappers = imported(&wrapper_source(), ImportRole::Wrapper);
        let source_summaries = [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ];
        let policy = parse_source_program(WAIT_SOURCE_POLICY).unwrap();
        let declarations = policy
            .declarations
            .iter()
            .map(|declaration| declaration.value.name.clone())
            .collect::<Vec<_>>();
        let clauses = policy
            .declarations
            .iter()
            .flat_map(|declaration| {
                declaration
                    .value
                    .function_contract
                    .as_ref()
                    .unwrap()
                    .clauses
                    .iter()
                    .map(|clause| clause.value.expression_source.clone())
            })
            .collect::<Vec<_>>();

        for name in declarations {
            let original = format!("extern fn {name}(");
            let changed = format!("extern fn removed_{name}(");
            let mutant = WAIT_SOURCE_POLICY.replacen(&original, &changed, 1);
            assert_ne!(
                mutant, WAIT_SOURCE_POLICY,
                "declaration mutation missed {name}"
            );
            let result = MonkeyWaitContracts::parse_with_source_policy(
                WRAPPER_CONTRACT,
                COMPOSITION_CONTRACT,
                &mutant,
            )
            .and_then(|contracts| {
                project_monkey_timed_wait_source(source_summaries, &contracts).map(|_| contracts)
            });
            assert!(
                result.is_err(),
                "renamed policy declaration `{name}` accepted"
            );
        }

        let mut seen_expressions = BTreeMap::<String, usize>::new();
        for expression in clauses {
            let original = format!("ensures normal {expression};");
            let occurrence = seen_expressions.entry(original.clone()).or_default();
            let mutant = replace_occurrence(
                WAIT_SOURCE_POLICY,
                &original,
                *occurrence,
                "ensures normal false;",
            );
            *occurrence += 1;
            assert_ne!(
                mutant, WAIT_SOURCE_POLICY,
                "clause mutation missed `{expression}`"
            );
            let result = MonkeyWaitContracts::parse_with_source_policy(
                WRAPPER_CONTRACT,
                COMPOSITION_CONTRACT,
                &mutant,
            )
            .and_then(|contracts| {
                project_monkey_timed_wait_source(source_summaries, &contracts).map(|_| contracts)
            });
            assert!(
                result.is_err(),
                "disabled policy clause `{expression}` accepted"
            );
        }
    }
}
