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

//! Per-callback, per-path Cycle 18 obligation accounting for the three
//! selected Monkey callbacks.
//!
//! [`crate::cycle18_composition`] joins three bounded evidence families and
//! names seven unresolved obligations for the callback set as a whole. This
//! module measures those obligations for every selected callback and every one
//! of its imported source paths, applies the source/build/profile-bound
//! dischargers defined here, and publishes one verdict per callback.
//!
//! A callback is [`CallbackVerdict::Composed`] only when its evidence family
//! joined, no provisional-Unsafe scenario of its registration context
//! remains, and every obligation relevant to it is discharged on every
//! relevant path. Anything else is [`CallbackVerdict::Unknown`] with the
//! remaining obligations and blocking scenarios named individually. The
//! decision rule is modelled by `QCore.Cycle18CallbackObligations`.
//!
//! Discharges are computed here from constructor-private
//! [`SourceSummary`] values and a freshly rechecked aggregate; no caller can
//! supply a discharged status. Evidence kinds stay separate: a
//! [`EvidenceKind::SourceFact`] is a fact of the sealed typed import, a
//! [`EvidenceKind::ModelProof`] is an accepted Cycle 18 event-program check
//! over source-derived operations, and a
//! [`EvidenceKind::BoundedNativeFamily`] is a joined bounded observation
//! family. Faithful C import and Rust/Lean correspondence remain premises.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{
    SourceExpression, SourceOperation, SourcePath, SourceRegion, SourceSummary, SourceValue,
};
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, EventFlags, EventKey, EventOperation,
    EventProgram, PayloadAccessKind, ReactorProfile, check_event_program,
};
use qcore_schemas::sha256_hex;
use serde::Serialize;

use crate::cycle18_composition::{
    Cycle18AggregateVerdict, Cycle18CompositionInput, Cycle18CompositionReport,
    check_cycle18_composition,
};
use crate::monkey_wait_projection::monkey_source_revision;
use crate::timeout_teardown::TimeoutTeardownSourceProjection;

pub const CALLBACK_OBLIGATIONS_PROPERTY: &str = "monkey-cycle18-callback-obligations";
pub const CALLBACK_OBLIGATIONS_EVIDENCE_VERSION: &str = "qcore.cycle18.callback-obligations.v1";

const CALLBACKS: [(&str, usize); 3] =
    [("cb_event", 4), ("cb_timeout", 3), ("cb_wait_2_timeout", 1)];

/// The seven obligations the bounded aggregate leaves unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cycle18Obligation {
    InitializationReachability,
    CallerNullHandling,
    CallerSizeBounds,
    AllocationAuthorityProvenance,
    RegistrationLiveness,
    NaturalActivationReachability,
    AllPathExecutionCoverage,
}

impl Cycle18Obligation {
    pub const ALL: [Self; 7] = [
        Self::InitializationReachability,
        Self::CallerNullHandling,
        Self::CallerSizeBounds,
        Self::AllocationAuthorityProvenance,
        Self::RegistrationLiveness,
        Self::NaturalActivationReachability,
        Self::AllPathExecutionCoverage,
    ];

    /// The aggregate's `unresolved_obligations` key for this obligation.
    pub fn name(self) -> &'static str {
        match self {
            Self::InitializationReachability => "initialization_reachability",
            Self::CallerNullHandling => "caller_null_handling",
            Self::CallerSizeBounds => "caller_size_bounds",
            Self::AllocationAuthorityProvenance => "allocation_authority_provenance",
            Self::RegistrationLiveness => "registration_liveness",
            Self::NaturalActivationReachability => "natural_activation_reachability",
            Self::AllPathExecutionCoverage => "all_path_execution_coverage",
        }
    }
}

/// Which side of the trust boundary a discharge rests on (rule 28).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A fact of the sealed typed import under the selected profile.
    SourceFact,
    /// An accepted Cycle 18 event-program check over source-derived operations.
    ModelProof,
    /// A joined bounded native evidence family; an observation, not a proof.
    BoundedNativeFamily,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PartialDischarge {
    pub kind: EvidenceKind,
    pub method: String,
    pub established: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObligationStatus {
    Discharged {
        kind: EvidenceKind,
        method: String,
        established: String,
    },
    Open {
        needs: String,
        partial: Vec<PartialDischarge>,
    },
}

impl ObligationStatus {
    pub fn is_discharged(&self) -> bool {
        matches!(self, Self::Discharged { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelevantObligation {
    pub paths: BTreeSet<usize>,
    pub relevance: String,
    pub status: ObligationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CallbackVerdict {
    Composed,
    Unknown {
        open_obligations: BTreeSet<Cycle18Obligation>,
        blocking_scenarios: BTreeSet<String>,
        evidence_family_refused: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackObligationRow {
    pub callback: String,
    pub source_paths: usize,
    pub source_summary_sha256: String,
    pub obligations: BTreeMap<Cycle18Obligation, RelevantObligation>,
    pub blocking_scenarios: BTreeMap<String, String>,
    pub verdict: CallbackVerdict,
}

/// One cell of the per-callback, per-path obligation table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PathObligationCell {
    pub callback: String,
    pub path: usize,
    pub obligation: Cycle18Obligation,
    pub discharged: bool,
    pub kind: Option<EvidenceKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObligationTable {
    pub callbacks: Vec<CallbackObligationRow>,
    pub cells: Vec<PathObligationCell>,
    pub relevant_cells: usize,
    pub discharged_cells: usize,
    pub open_cells: usize,
    pub composed_callbacks: usize,
    pub unknown_callbacks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackObligationReport {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: BTreeSet<String>,
    pub bounds: BTreeMap<String, u64>,
    pub assumptions: BTreeMap<String, String>,
    pub evidence_version: String,
    pub source_revision: String,
    pub profile_sha256: String,
    pub aggregate_verdict: Cycle18AggregateVerdict,
    pub aggregate_policy_sha256: String,
    pub aggregate_source_inventory_sha256: String,
    pub aggregate_unresolved_obligations: BTreeMap<String, String>,
    pub aggregate_provisional_unsafe_scenarios: BTreeSet<String>,
    pub source_summary_sha256: BTreeMap<String, String>,
    pub before: ObligationTable,
    pub after: ObligationTable,
    pub initialization: Option<InitializationDischarge>,
    pub initialization_refusal: Option<String>,
    pub activation: BTreeMap<String, ActivationFlagDependence>,
    pub timeout_paths: Option<TimeoutPathComposition>,
    pub timeout_paths_refusal: Option<String>,
    pub registration_sites: BTreeMap<String, Vec<RegistrationSite>>,
    pub universal_safety_certificate: bool,
}

/// Selected-source summaries beyond the three callbacks. Every summary must
/// come from the same pinned import as the aggregate's callback summaries.
pub struct CallbackObligationSources<'a> {
    pub callbacks: [&'a SourceSummary; 3],
    pub event_init: &'a SourceSummary,
    pub loop_create: &'a SourceSummary,
    pub event_add: &'a SourceSummary,
    pub event_update: &'a SourceSummary,
    pub timeout_create: &'a SourceSummary,
    pub wait_2: &'a SourceSummary,
    pub timeout_projection: &'a TimeoutTeardownSourceProjection,
}

/// Facts read from a freshly checked aggregate. Construction is private, so a
/// serialized or hand-built aggregate cannot supply joined families.
#[derive(Clone, Debug)]
struct AggregateFacts {
    verdict: Cycle18AggregateVerdict,
    source_revision: String,
    profile_sha256: String,
    callback_source_sha256: BTreeMap<String, String>,
    joined: BTreeSet<String>,
    refused: BTreeMap<String, String>,
    provisional_unsafe: BTreeSet<String>,
    unresolved: BTreeMap<String, String>,
    policy_sha256: String,
    inventory_sha256: String,
}

impl AggregateFacts {
    fn from_checked(report: &Cycle18CompositionReport) -> Self {
        Self {
            verdict: report.verdict,
            source_revision: report.source_revision.clone(),
            profile_sha256: report.profile_sha256.clone(),
            callback_source_sha256: report.callback_source_sha256.clone(),
            joined: report.joined_evidence_families.clone(),
            refused: report.refused_evidence_families.clone(),
            provisional_unsafe: report.provisional_unsafe_scenarios.clone(),
            unresolved: report.unresolved_obligations.clone(),
            policy_sha256: report.policy_sha256.clone(),
            inventory_sha256: report.source_inventory_sha256.clone(),
        }
    }
}

/// Recheck the bounded aggregate, then measure and discharge the per-callback
/// obligations over the same pinned source import.
pub fn check_cycle18_callback_obligations(
    input: Cycle18CompositionInput<'_>,
    sources: &CallbackObligationSources<'_>,
) -> Result<CallbackObligationReport, String> {
    let aggregate = check_cycle18_composition(input).map_err(|error| error.to_string())?;
    assemble(&AggregateFacts::from_checked(&aggregate), sources)
}

// ---------------------------------------------------------------------------
// Typed-source expression helpers.

fn strip(value: &SourceValue) -> &SourceExpression {
    let mut current = &value.expression;
    while let SourceExpression::Cast { value, .. } = current {
        current = &value.expression;
    }
    current
}

/// Evaluate a closed integer expression built from literals, `|`, `&`, `+`
/// and unary minus, at each node's C type, to the value it has on every
/// target (card CG20-NUMERIC-DOMAIN, absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`).
/// Anything else, including a symbol or parameter, a value outside its
/// type's portable range, an inexact conversion or an unsigned wrap, is None.
fn evaluate_integer(value: &SourceValue) -> Option<i128> {
    qcore_c_gateway::c_numeric::portable_constant(value)
}

fn expression_mentions(
    expression: &SourceExpression,
    predicate: &dyn Fn(&SourceExpression) -> bool,
) -> bool {
    if predicate(expression) {
        return true;
    }
    match expression {
        SourceExpression::Aggregate { elements } => elements
            .iter()
            .any(|element| expression_mentions(&element.expression, predicate)),
        SourceExpression::Cast { value, .. } | SourceExpression::Unary { value, .. } => {
            expression_mentions(&value.expression, predicate)
        }
        SourceExpression::Binary { left, right, .. } => {
            expression_mentions(&left.expression, predicate)
                || expression_mentions(&right.expression, predicate)
        }
        SourceExpression::Field { base, .. } => expression_mentions(&base.expression, predicate),
        SourceExpression::Index { base, index } => {
            expression_mentions(&base.expression, predicate)
                || expression_mentions(&index.expression, predicate)
        }
        _ => false,
    }
}

fn value_mentions(value: &SourceValue, predicate: &dyn Fn(&SourceExpression) -> bool) -> bool {
    expression_mentions(&value.expression, predicate)
}

fn region_mentions(region: &SourceRegion, predicate: &dyn Fn(&SourceExpression) -> bool) -> bool {
    region
        .operations
        .iter()
        .any(|operation| operation_mentions(operation, predicate))
        || value_mentions(&region.result, predicate)
}

fn operation_mentions(
    operation: &SourceOperation,
    predicate: &dyn Fn(&SourceExpression) -> bool,
) -> bool {
    match operation {
        SourceOperation::ScanLoop {
            initial,
            header,
            matches,
            early_return,
            increment,
            ..
        } => {
            value_mentions(initial, predicate)
                || region_mentions(header, predicate)
                || region_mentions(matches, predicate)
                || region_mentions(early_return, predicate)
                || value_mentions(increment, predicate)
        }
        SourceOperation::Read { place } => value_mentions(place, predicate),
        SourceOperation::Write { place, value } => {
            value_mentions(place, predicate) || value_mentions(value, predicate)
        }
        SourceOperation::LocalWrite { value, .. } => value_mentions(value, predicate),
        SourceOperation::Call { arguments, .. } => arguments
            .iter()
            .any(|argument| value_mentions(argument, predicate)),
        SourceOperation::Guard { condition, .. } => value_mentions(condition, predicate),
        SourceOperation::ReturnValue { value } => value_mentions(value, predicate),
        SourceOperation::LocalStart { .. }
        | SourceOperation::LocalEnd { .. }
        | SourceOperation::Return
        | SourceOperation::Terminate => false,
    }
}

fn is_symbol(index: usize) -> impl Fn(&SourceExpression) -> bool {
    move |expression| matches!(expression, SourceExpression::Symbol { operation } if *operation == index)
}

fn is_parameter(index: usize) -> impl Fn(&SourceExpression) -> bool {
    move |expression| matches!(expression, SourceExpression::Parameter { index: found } if *found == index)
}

fn is_local(local: usize) -> impl Fn(&SourceExpression) -> bool {
    move |expression| matches!(expression, SourceExpression::LocalStorage { local: found } if *found == local)
}

fn summary_sha256(summary: &SourceSummary) -> String {
    sha256_hex(&serde_json::to_vec(summary).expect("sealed source summaries always serialize"))
}

fn projection_sha256(summary: &SourceSummary) -> String {
    sha256_hex(
        &serde_json::to_vec(&(
            summary.function(),
            summary.source_revision(),
            summary.paths(),
        ))
        .expect("sealed source paths always serialize"),
    )
}

fn check_import_identity(
    summary: &SourceSummary,
    function: &str,
    role: &str,
    revision: &str,
    profile: &str,
) -> Result<(), String> {
    let assumptions = summary.profile_assumptions();
    if summary.function() != function
        || summary.source_revision() != revision
        || revision != monkey_source_revision()
        || assumptions.get("profile_digest").map(String::as_str) != Some(profile)
        || profile.len() != 64
        || assumptions.get("library").map(String::as_str) != Some("monkey")
        || assumptions.get("typed_import_role").map(String::as_str) != Some(role)
    {
        return Err(format!(
            "{function} is not a {role} summary of the pinned Monkey import under the aggregate profile"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Initialization reachability.

/// Source fact: `_mk_event_init` is a constant function under the selected
/// profile, so every `_mk_event_loop_create` path whose guard on its result
/// disagrees with that constant is infeasible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InitializationDischarge {
    pub event_init_summary_sha256: String,
    pub loop_create_summary_sha256: String,
    pub source_revision: String,
    pub profile_sha256: String,
    pub constant_result: i128,
    pub loop_create_paths: usize,
    pub initialization_failure_paths: BTreeSet<usize>,
    pub infeasible_paths: BTreeSet<usize>,
    pub feasible_paths: BTreeSet<usize>,
}

fn constant_return(summary: &SourceSummary) -> Result<i128, String> {
    let mut constant = None;
    if summary.paths().is_empty() {
        return Err("event initialization has no imported path".into());
    }
    for path in summary.paths() {
        let [SourceOperation::ReturnValue { value }] = path.operations.as_slice() else {
            return Err(
                "event initialization has a path with an operation other than one constant return"
                    .into(),
            );
        };
        let value =
            evaluate_integer(value).ok_or("event initialization returns a non-literal value")?;
        match constant {
            None => constant = Some(value),
            Some(previous) if previous == value => {}
            Some(_) => {
                return Err(
                    "event initialization returns different constants on different paths".into(),
                );
            }
        }
    }
    constant.ok_or_else(|| "event initialization has no return".into())
}

pub fn discharge_initialization_reachability(
    event_init: &SourceSummary,
    loop_create: &SourceSummary,
    expected_profile_sha256: &str,
) -> Result<InitializationDischarge, String> {
    let revision = monkey_source_revision();
    check_import_identity(
        event_init,
        "_mk_event_init",
        "Wrapper",
        revision,
        expected_profile_sha256,
    )?;
    check_import_identity(
        loop_create,
        "_mk_event_loop_create",
        "Wrapper",
        revision,
        expected_profile_sha256,
    )?;
    if !matches!(event_init.signature(), "int ()" | "int (void)") {
        return Err("event initialization signature is not `int ()`".into());
    }
    let constant = constant_return(event_init)?;
    let mut failure_paths = BTreeSet::new();
    let mut infeasible = BTreeSet::new();
    let mut feasible = BTreeSet::new();
    for (index, path) in loop_create.paths().iter().enumerate() {
        let calls = path
            .operations
            .iter()
            .enumerate()
            .filter_map(|(position, operation)| match operation {
                SourceOperation::Call {
                    function,
                    arguments,
                    ..
                } => Some((position, function.as_str(), arguments.len())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let [(call, "_mk_event_init", 0), rest @ ..] = calls.as_slice() else {
            return Err(format!(
                "loop-create path {index} does not begin with one argument-free initialization call"
            ));
        };
        if rest
            .iter()
            .any(|(_, function, _)| *function == "_mk_event_init")
        {
            return Err(format!(
                "loop-create path {index} repeats the initialization call"
            ));
        }
        let guards = path
            .operations
            .iter()
            .filter_map(|operation| match operation {
                SourceOperation::Guard { condition, taken }
                    if value_mentions(condition, &is_symbol(*call)) =>
                {
                    Some((condition, *taken))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let [(condition, taken)] = guards.as_slice() else {
            return Err(format!(
                "loop-create path {index} does not test the initialization result exactly once"
            ));
        };
        let SourceExpression::Binary {
            operator,
            left,
            right,
        } = strip(condition)
        else {
            return Err(format!(
                "loop-create path {index} tests initialization with a non-comparison"
            ));
        };
        let compared = match (strip(left), strip(right)) {
            (SourceExpression::Symbol { operation }, _) if operation == call => {
                evaluate_integer(right)
            }
            (_, SourceExpression::Symbol { operation }) if operation == call => {
                evaluate_integer(left)
            }
            _ => None,
        }
        .ok_or_else(|| {
            format!("loop-create path {index} compares initialization with a non-literal")
        })?;
        let holds = match operator.as_str() {
            "!=" => constant != compared,
            "==" => constant == compared,
            other => {
                return Err(format!(
                    "loop-create path {index} uses unsupported initialization comparison {other}"
                ));
            }
        };
        // The initialization-failure branch is the path whose only call is
        // the initialization call itself: it returns before any allocation.
        if rest.is_empty() {
            failure_paths.insert(index);
        }
        if holds == *taken {
            feasible.insert(index);
        } else {
            infeasible.insert(index);
        }
    }
    if failure_paths.is_empty() {
        return Err("loop creation has no initialization-failure branch to refute".into());
    }
    if !failure_paths.is_subset(&infeasible) {
        return Err(format!(
            "initialization-failure paths {:?} are not refuted by the constant initialization result {constant}",
            failure_paths.difference(&infeasible).collect::<Vec<_>>()
        ));
    }
    if feasible.is_empty() {
        return Err("the constant initialization result refutes every loop-create path".into());
    }
    Ok(InitializationDischarge {
        event_init_summary_sha256: summary_sha256(event_init),
        loop_create_summary_sha256: summary_sha256(loop_create),
        source_revision: revision.into(),
        profile_sha256: expected_profile_sha256.into(),
        constant_result: constant,
        loop_create_paths: loop_create.paths().len(),
        initialization_failure_paths: failure_paths,
        infeasible_paths: infeasible,
        feasible_paths: feasible,
    })
}

// ---------------------------------------------------------------------------
// Activation-flag dependence.

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "dependence", rename_all = "snake_case")]
pub enum ActivationFlagDependence {
    /// No operation on any path reads the activation-flags parameter.
    Independent,
    /// Only single-bit guards read the flags; `exhaustive` says every
    /// combination of the tested bits selects exactly one imported path.
    BitGuarded {
        tested_bits: BTreeSet<i128>,
        path_classes: BTreeMap<usize, BTreeMap<i128, bool>>,
        exhaustive: bool,
    },
}

const CALLBACK_SIGNATURES: [&str; 2] = [
    "void (int, short, void *)",
    "void (evutil_socket_t, short, void *)",
];

pub fn activation_flag_dependence(
    summary: &SourceSummary,
) -> Result<ActivationFlagDependence, String> {
    if !CALLBACK_SIGNATURES.contains(&summary.signature()) {
        return Err(format!(
            "{} does not have the Libevent callback signature",
            summary.function()
        ));
    }
    let flags = is_parameter(1);
    let mut tested_bits = BTreeSet::new();
    let mut path_classes = BTreeMap::new();
    for (index, path) in summary.paths().iter().enumerate() {
        let mut class = BTreeMap::new();
        for operation in &path.operations {
            if !operation_mentions(operation, &flags) {
                continue;
            }
            let SourceOperation::Guard { condition, taken } = operation else {
                return Err(format!(
                    "{} path {index} uses activation flags outside a guard",
                    summary.function()
                ));
            };
            let bit = match strip(condition) {
                SourceExpression::Binary {
                    operator,
                    left,
                    right,
                } if operator == "&" && flags(strip(left)) => evaluate_integer(right),
                _ => None,
            }
            .filter(|bit| *bit > 0 && bit.count_ones() == 1)
            .ok_or_else(|| {
                format!(
                    "{} path {index} tests activation flags with a non-single-bit guard",
                    summary.function()
                )
            })?;
            if class
                .insert(bit, *taken)
                .is_some_and(|previous| previous != *taken)
            {
                return Err(format!(
                    "{} path {index} tests one activation bit inconsistently",
                    summary.function()
                ));
            }
            tested_bits.insert(bit);
        }
        path_classes.insert(index, class);
    }
    if tested_bits.is_empty() {
        return Ok(ActivationFlagDependence::Independent);
    }
    let complete_classes = path_classes
        .values()
        .filter(|class| class.len() == tested_bits.len())
        .cloned()
        .collect::<BTreeSet<_>>();
    let exhaustive = tested_bits.len() < 16
        && path_classes
            .values()
            .all(|class| class.len() == tested_bits.len())
        && complete_classes.len() == 1usize << tested_bits.len()
        && complete_classes.len() == path_classes.len();
    Ok(ActivationFlagDependence::BitGuarded {
        tested_bits,
        path_classes,
        exhaustive,
    })
}

// ---------------------------------------------------------------------------
// Registration sites.

/// Source facts of one registration wrapper path that creates the callback's
/// event. They explain why registration failure and liveness stay open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegistrationSite {
    pub wrapper: String,
    pub path: usize,
    pub event_flags: Option<i128>,
    pub creation_result_checked: bool,
    /// `None` when the path returns before registering the created event.
    pub registration_result_checked: Option<bool>,
    pub stale_event_field_on_failure: bool,
    pub ignored_results: Vec<String>,
}

const RESULT_BEARING_CALLS: [&str; 6] = [
    "event_new",
    "event_add",
    "_mk_event_add",
    "mk_event_libevent_socketpair",
    "mk_mem_alloc_z",
    "event_base_new",
];

fn guarded_after(path: &SourcePath, symbol: usize) -> bool {
    path.operations
        .iter()
        .skip(symbol + 1)
        .any(|operation| matches!(operation, SourceOperation::Guard { condition, .. } if value_mentions(condition, &is_symbol(symbol))))
}

fn is_field_named(value: &SourceValue, name: &str) -> bool {
    matches!(strip(value), SourceExpression::Field { name: found, .. } if found == name)
}

fn read_place<'a>(path: &'a SourcePath, value: &SourceValue) -> Option<&'a SourceValue> {
    let SourceExpression::Symbol { operation } = strip(value) else {
        return None;
    };
    match path.operations.get(*operation) {
        Some(SourceOperation::Read { place }) => Some(place),
        _ => None,
    }
}

pub fn registration_sites(callback: &str, wrapper: &SourceSummary) -> Vec<RegistrationSite> {
    let mut sites = Vec::new();
    for (index, path) in wrapper.paths().iter().enumerate() {
        let Some(creation) = path.operations.iter().position(|operation| {
            matches!(operation, SourceOperation::Call { function, arguments, .. }
                if function == "event_new"
                    && arguments.iter().any(|argument| matches!(strip(argument), SourceExpression::Function { name, .. } if name == callback)))
        }) else {
            continue;
        };
        let SourceOperation::Call { arguments, .. } = &path.operations[creation] else {
            unreachable!("position selected a call")
        };
        let event_flags = arguments.get(2).and_then(evaluate_integer);
        let first_use = path
            .operations
            .iter()
            .enumerate()
            .skip(creation + 1)
            .find(|(_, operation)| {
                matches!(operation, SourceOperation::Call { arguments, .. }
                    if arguments.iter().any(|argument| value_mentions(argument, &is_symbol(creation))))
            })
            .map(|(position, _)| position);
        let creation_result_checked = path
            .operations
            .iter()
            .enumerate()
            .skip(creation + 1)
            .take_while(|(position, _)| first_use.is_none_or(|first| *position < first))
            .any(|(_, operation)| matches!(operation, SourceOperation::Guard { condition, .. } if value_mentions(condition, &is_symbol(creation))));
        let registration = path.operations.iter().position(|operation| {
            matches!(operation, SourceOperation::Call { function, arguments, .. }
                if function == "event_add"
                    && arguments.first().is_some_and(|argument| value_mentions(argument, &is_symbol(creation))))
        });
        let registration_result_checked =
            registration.map(|registration| guarded_after(path, registration));
        let ignored_results = path
            .operations
            .iter()
            .enumerate()
            .filter_map(|(position, operation)| match operation {
                SourceOperation::Call { function, .. }
                    if RESULT_BEARING_CALLS.contains(&function.as_str())
                        && !guarded_after(path, position) =>
                {
                    Some(format!("{function}@{position}"))
                }
                _ => None,
            })
            .collect();
        // A path that frees `X.event` and returns early without storing a
        // replacement leaves the map's event field dangling.
        let frees_event_field = path.operations.iter().any(|operation| {
            matches!(operation, SourceOperation::Call { function, arguments, .. }
                if function == "event_free"
                    && arguments.first().and_then(|argument| read_place(path, argument)).is_some_and(|place| is_field_named(place, "event")))
        });
        let stores_event_field = path.operations.iter().any(|operation| {
            matches!(operation, SourceOperation::Write { place, .. } if is_field_named(place, "event"))
        });
        let returns_failure = matches!(
            path.operations.last(),
            Some(SourceOperation::ReturnValue { value }) if evaluate_integer(value) == Some(-1)
        );
        sites.push(RegistrationSite {
            wrapper: wrapper.function().into(),
            path: index,
            event_flags,
            creation_result_checked,
            registration_result_checked,
            stale_event_field_on_failure: frees_event_field
                && !stores_event_field
                && returns_failure,
            ignored_results,
        });
    }
    sites
}

// ---------------------------------------------------------------------------
// cb_timeout source-path composition.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutStep {
    EndpointSend,
    EffectFree,
    CloseWriteEndpoint,
    DeleteCurrentEvent,
    FreeCurrentEvent,
    DropMapArgument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutProgramPurpose {
    /// Must be accepted for the paths it names to compose.
    Composition,
    /// Expected to be refused; its refusal witnesses a remaining obligation.
    LivenessWitness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckedTimeoutProgram {
    pub name: String,
    pub purpose: TimeoutProgramPurpose,
    pub source_paths: Vec<usize>,
    pub close_succeeds: bool,
    pub accepted: bool,
    pub checked_operations: usize,
    pub diagnostics: Vec<String>,
}

/// Model composition of every imported `cb_timeout` path with the event
/// program derived from its registration in `_mk_event_timeout_create`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeoutPathComposition {
    pub callback_summary_sha256: String,
    pub registration_summary_sha256: String,
    pub registration_paths: Vec<usize>,
    pub event_flags: u16,
    pub path_steps: BTreeMap<usize, Vec<TimeoutStep>>,
    pub teardown_paths: BTreeSet<usize>,
    pub non_teardown_paths: BTreeSet<usize>,
    pub map_exclusive_to_registration: bool,
    pub programs: Vec<CheckedTimeoutProgram>,
}

impl TimeoutPathComposition {
    fn accepted(&self, close_succeeds: bool) -> bool {
        self.programs
            .iter()
            .filter(|program| {
                program.purpose == TimeoutProgramPurpose::Composition
                    && program.close_succeeds == close_succeeds
            })
            .all(|program| program.accepted)
    }

    /// Every source path composes on the branch where the ignored close of
    /// the write endpoint succeeds.
    pub fn close_success_branch_composes(&self) -> bool {
        self.accepted(true)
    }

    /// Every source path also composes when that close fails.
    pub fn close_failure_branch_composes(&self) -> bool {
        self.accepted(false)
    }

    /// The model refuses a trace in which only the non-teardown path runs
    /// before the base is freed: release needs a later teardown activation.
    pub fn liveness_gap_witnessed(&self) -> bool {
        self.programs.iter().any(|program| {
            program.purpose == TimeoutProgramPurpose::LivenessWitness && !program.accepted
        })
    }
}

fn is_map_field(value: &SourceValue, name: &str) -> bool {
    matches!(strip(value), SourceExpression::Field { base, name: found }
        if found == name && is_parameter(2)(strip(base)))
}

fn is_map_write_endpoint(value: &SourceValue) -> bool {
    matches!(strip(value), SourceExpression::Index { base, index }
        if evaluate_integer(index) == Some(1) && is_map_field(base, "pipe"))
}

fn timeout_steps(path: &SourcePath) -> Result<Vec<TimeoutStep>, String> {
    let mut steps = Vec::new();
    for operation in &path.operations {
        match operation {
            SourceOperation::Call {
                function,
                arguments,
                ..
            } => {
                let reads = |predicate: &dyn Fn(&SourceValue) -> bool| {
                    arguments
                        .first()
                        .and_then(|argument| read_place(path, argument))
                        .is_some_and(predicate)
                };
                let step = match function.as_str() {
                    "send" if arguments.len() == 4 && reads(&is_map_write_endpoint) => {
                        TimeoutStep::EndpointSend
                    }
                    "__errno_location" | "__error" if arguments.is_empty() => {
                        TimeoutStep::EffectFree
                    }
                    "perror"
                        if arguments.len() == 1
                            && matches!(strip(&arguments[0]), SourceExpression::String { .. }) =>
                    {
                        TimeoutStep::EffectFree
                    }
                    "evutil_closesocket"
                        if arguments.len() == 1 && reads(&is_map_write_endpoint) =>
                    {
                        TimeoutStep::CloseWriteEndpoint
                    }
                    "event_del"
                        if arguments.len() == 1 && reads(&|place| is_map_field(place, "event")) =>
                    {
                        TimeoutStep::DeleteCurrentEvent
                    }
                    "event_free"
                        if arguments.len() == 1 && reads(&|place| is_map_field(place, "event")) =>
                    {
                        TimeoutStep::FreeCurrentEvent
                    }
                    "mk_mem_free"
                        if arguments.len() == 1 && is_parameter(2)(strip(&arguments[0])) =>
                    {
                        TimeoutStep::DropMapArgument
                    }
                    other => {
                        return Err(format!(
                            "cb_timeout call `{other}` has no admitted lifecycle mapping for its arguments"
                        ));
                    }
                };
                steps.push(step);
            }
            SourceOperation::Write { .. } => {
                return Err("cb_timeout writes through memory; no mapping is admitted".into());
            }
            SourceOperation::ScanLoop { .. } | SourceOperation::Terminate => {
                return Err("cb_timeout path has an unsupported control operation".into());
            }
            _ => {}
        }
    }
    Ok(steps)
}

struct RegistrationFacts {
    paths: Vec<usize>,
    flags: u16,
    exclusive: bool,
}

fn symbol_escapes(path: &SourcePath, symbol: usize, admitted: &dyn Fn(usize) -> bool) -> bool {
    let mentions = is_symbol(symbol);
    let mut aliases = BTreeSet::new();
    for (position, operation) in path.operations.iter().enumerate() {
        match operation {
            SourceOperation::LocalWrite { local, value } if value_mentions(value, &mentions) => {
                aliases.insert(*local);
            }
            SourceOperation::Call { arguments, .. }
                if arguments
                    .iter()
                    .any(|argument| value_mentions(argument, &mentions))
                    && !admitted(position) =>
            {
                return true;
            }
            SourceOperation::Write { value, .. }
                if value_mentions(value, &mentions) && !admitted(position) =>
            {
                return true;
            }
            SourceOperation::ReturnValue { value } if value_mentions(value, &mentions) => {
                return true;
            }
            _ => {}
        }
    }
    // A local holding the pointer escapes when its storage is itself named
    // (address taken) by any later operation.
    aliases.iter().any(|local| {
        path.operations
            .iter()
            .any(|operation| operation_mentions(operation, &is_local(*local)))
    })
}

fn timeout_registration(registration: &SourceSummary) -> Result<RegistrationFacts, String> {
    let mut paths = Vec::new();
    let mut flags = BTreeSet::new();
    let mut exclusive = true;
    for (index, path) in registration.paths().iter().enumerate() {
        let creations = path
            .operations
            .iter()
            .enumerate()
            .filter(|(_, operation)| {
                matches!(operation, SourceOperation::Call { function, arguments, .. }
                    if function == "event_new"
                        && arguments.iter().any(|argument| matches!(strip(argument), SourceExpression::Function { name, .. } if name == "cb_timeout")))
            })
            .collect::<Vec<_>>();
        let [(creation, SourceOperation::Call { arguments, .. })] = creations.as_slice() else {
            if creations.is_empty() {
                continue;
            }
            return Err(format!(
                "registration path {index} creates cb_timeout more than once"
            ));
        };
        let [_, descriptor, event_flags, callback, argument] = arguments.as_slice() else {
            return Err(format!(
                "registration path {index} has a non-five-argument event_new"
            ));
        };
        if evaluate_integer(descriptor) != Some(-1)
            || !matches!(strip(callback), SourceExpression::Function { name, .. } if name == "cb_timeout")
        {
            return Err(format!(
                "registration path {index} does not create a descriptor-free cb_timeout event"
            ));
        }
        let flag_value = evaluate_integer(event_flags)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| format!("registration path {index} has non-literal event flags"))?;
        let SourceExpression::Symbol { operation: map } = strip(argument) else {
            return Err(format!(
                "registration path {index} passes a callback argument that is not a local result"
            ));
        };
        if !matches!(path.operations.get(*map), Some(SourceOperation::Call { function, .. }) if function == "mk_mem_alloc_z")
        {
            return Err(format!(
                "registration path {index} passes a callback argument not allocated on that path"
            ));
        }
        let stores_event = path.operations.iter().any(|operation| {
            matches!(operation, SourceOperation::Write { place, value }
                if matches!(strip(place), SourceExpression::Field { base, name } if name == "event" && is_symbol(*map)(strip(base)))
                    && is_symbol(*creation)(strip(value)))
        });
        if !stores_event {
            return Err(format!(
                "registration path {index} does not store the created event in its map"
            ));
        }
        let creation = *creation;
        let map_admitted = |position: usize| position == creation;
        let event_admitted = |position: usize| {
            matches!(&path.operations[position], SourceOperation::Call { function, .. } if function == "event_add")
                || matches!(&path.operations[position], SourceOperation::Write { place, .. }
                    if matches!(strip(place), SourceExpression::Field { base, name } if name == "event" && is_symbol(*map)(strip(base))))
        };
        exclusive &= !symbol_escapes(path, *map, &map_admitted)
            && !symbol_escapes(path, creation, &event_admitted);
        flags.insert(flag_value);
        paths.push(index);
    }
    let [flags] = flags.into_iter().collect::<Vec<_>>()[..] else {
        return Err("cb_timeout registrations do not agree on one flag set".into());
    };
    if paths.is_empty() {
        return Err("no registration path creates cb_timeout".into());
    }
    Ok(RegistrationFacts {
        paths,
        flags,
        exclusive,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObligationProfile;

impl ReactorProfile for ObligationProfile {
    fn threading_enabled(self) -> bool {
        true
    }
}

const MODEL_BASE: u32 = 1;
const MODEL_DISPATCHER: u32 = 1;
const MODEL_ARGUMENT: u32 = 100;
const MODEL_PEER: i32 = 10;
const MODEL_WRITE: i32 = 11;

fn timeout_program(
    flags: u16,
    activations: &[(&[TimeoutStep], bool)],
    peer_close_before: usize,
) -> EventProgram<ObligationProfile> {
    let event = EventKey::new(1, 1);
    let mut operations = vec![
        EventOperation::CreateSocketPair {
            descriptors: Some((MODEL_PEER, MODEL_WRITE)),
        },
        EventOperation::CreateBase {
            base: MODEL_BASE,
            profile: ObligationProfile,
        },
        EventOperation::Assign {
            event,
            base: MODEL_BASE,
            descriptor: None,
            flags: EventFlags::new(flags),
            priority: 0,
            summary: CallbackSummary {
                argument: MODEL_ARGUMENT,
                argument_mode: ArgumentMode::Owned,
                trigger_reasons: BTreeSet::from([ActivationReason::Timeout]),
                payload_authority: BTreeMap::<u32, PayloadAccessKind>::new(),
                may_defer: false,
                may_spawn_tasks: false,
            },
        },
        EventOperation::Add { event },
        EventOperation::BeginDispatch {
            base: MODEL_BASE,
            dispatcher: MODEL_DISPATCHER,
        },
    ];
    for (index, (steps, close_succeeds)) in activations.iter().enumerate() {
        if index == peer_close_before {
            // The caller's `_mk_event_timeout_destroy` closes the peer
            // endpoint; this is an enclosing premise, not a callback step.
            operations.push(EventOperation::CloseSocket {
                descriptor: MODEL_PEER,
                succeeds: true,
                declared_failure: None,
            });
        }
        operations.push(EventOperation::Activate {
            event,
            reason: ActivationReason::Timeout,
            cross_thread: false,
        });
        operations.push(EventOperation::CallbackEnter {
            event,
            reason: ActivationReason::Timeout,
        });
        for step in steps.iter() {
            match step {
                TimeoutStep::EndpointSend | TimeoutStep::EffectFree => {}
                TimeoutStep::CloseWriteEndpoint => operations.push(EventOperation::CloseSocket {
                    descriptor: MODEL_WRITE,
                    succeeds: *close_succeeds,
                    declared_failure: None,
                }),
                TimeoutStep::DeleteCurrentEvent => {
                    operations.push(EventOperation::DeleteCurrentCallback {
                        event,
                        dispatcher: MODEL_DISPATCHER,
                    })
                }
                TimeoutStep::FreeCurrentEvent => {
                    operations.push(EventOperation::FreeCurrentCallbackEvent {
                        event,
                        dispatcher: MODEL_DISPATCHER,
                    })
                }
                TimeoutStep::DropMapArgument => operations.push(EventOperation::DropArgument {
                    argument: MODEL_ARGUMENT,
                }),
            }
        }
        operations.push(EventOperation::CallbackExit { event });
    }
    if peer_close_before >= activations.len() {
        operations.push(EventOperation::CloseSocket {
            descriptor: MODEL_PEER,
            succeeds: true,
            declared_failure: None,
        });
    }
    operations.push(EventOperation::EndDispatch { base: MODEL_BASE });
    operations.push(EventOperation::FreeBase { base: MODEL_BASE });
    EventProgram {
        live_arguments: BTreeSet::from([MODEL_ARGUMENT]),
        operations,
    }
}

fn check_timeout_program(
    name: String,
    purpose: TimeoutProgramPurpose,
    source_paths: Vec<usize>,
    close_succeeds: bool,
    program: &EventProgram<ObligationProfile>,
) -> CheckedTimeoutProgram {
    match check_event_program(program, &[ObligationProfile]) {
        Ok(certificate) => CheckedTimeoutProgram {
            name,
            purpose,
            source_paths,
            close_succeeds,
            accepted: true,
            checked_operations: certificate.checked_operations,
            diagnostics: Vec::new(),
        },
        Err(diagnostics) => CheckedTimeoutProgram {
            name,
            purpose,
            source_paths,
            close_succeeds,
            accepted: false,
            checked_operations: program.operations.len(),
            diagnostics: diagnostics.iter().map(ToString::to_string).collect(),
        },
    }
}

const TEARDOWN: [TimeoutStep; 4] = [
    TimeoutStep::CloseWriteEndpoint,
    TimeoutStep::DeleteCurrentEvent,
    TimeoutStep::FreeCurrentEvent,
    TimeoutStep::DropMapArgument,
];

pub fn compose_timeout_source_paths(
    callback: &SourceSummary,
    projection: &TimeoutTeardownSourceProjection,
    registration: &SourceSummary,
    expected_profile_sha256: &str,
) -> Result<TimeoutPathComposition, String> {
    let revision = monkey_source_revision();
    check_import_identity(
        callback,
        "cb_timeout",
        "Callback",
        revision,
        expected_profile_sha256,
    )?;
    check_import_identity(
        registration,
        "_mk_event_timeout_create",
        "Wrapper",
        revision,
        expected_profile_sha256,
    )?;
    if projection.source_revision() != revision
        || projection.source_summary_digest() != projection_sha256(callback)
        || projection.path_call_sequences().len() != callback.paths().len()
    {
        return Err(
            "the sealed teardown projection does not describe this cb_timeout summary".into(),
        );
    }
    let facts = timeout_registration(registration)?;
    if facts.flags & EventFlags::TIMEOUT == 0 || facts.flags & EventFlags::PERSIST == 0 {
        return Err("cb_timeout is not registered as a persistent timer".into());
    }
    let mut path_steps = BTreeMap::new();
    let mut teardown_paths = BTreeSet::new();
    let mut non_teardown_paths = BTreeSet::new();
    for (index, path) in callback.paths().iter().enumerate() {
        let steps = timeout_steps(path)?;
        if steps.first() != Some(&TimeoutStep::EndpointSend)
            || steps
                .iter()
                .skip(1)
                .any(|step| *step == TimeoutStep::EndpointSend)
        {
            return Err(format!(
                "cb_timeout path {index} does not use its write endpoint exactly once, first"
            ));
        }
        let lifecycle = steps
            .iter()
            .copied()
            .filter(|step| !matches!(step, TimeoutStep::EndpointSend | TimeoutStep::EffectFree))
            .collect::<Vec<_>>();
        if lifecycle.is_empty() {
            non_teardown_paths.insert(index);
        } else if lifecycle == TEARDOWN {
            teardown_paths.insert(index);
        } else {
            return Err(format!(
                "cb_timeout path {index} has a partial or reordered teardown {lifecycle:?}"
            ));
        }
        path_steps.insert(index, steps);
    }
    let projected = projection
        .teardown_path_indices()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if projected != teardown_paths || teardown_paths.is_empty() {
        return Err("model teardown paths disagree with the sealed teardown projection".into());
    }
    let mut programs = Vec::new();
    for close_succeeds in [true, false] {
        for teardown in &teardown_paths {
            let steps = &path_steps[teardown];
            let program = timeout_program(facts.flags, &[(steps, close_succeeds)], 0);
            programs.push(check_timeout_program(
                format!("teardown_path_{teardown}"),
                TimeoutProgramPurpose::Composition,
                vec![*teardown],
                close_succeeds,
                &program,
            ));
            for other in &non_teardown_paths {
                let program = timeout_program(
                    facts.flags,
                    &[
                        (&path_steps[other], close_succeeds),
                        (steps, close_succeeds),
                    ],
                    1,
                );
                programs.push(check_timeout_program(
                    format!("path_{other}_then_teardown_path_{teardown}"),
                    TimeoutProgramPurpose::Composition,
                    vec![*other, *teardown],
                    close_succeeds,
                    &program,
                ));
            }
        }
    }
    for other in &non_teardown_paths {
        let program = timeout_program(facts.flags, &[(&path_steps[other], true)], 1);
        programs.push(check_timeout_program(
            format!("liveness_gap_path_{other}_then_base_free"),
            TimeoutProgramPurpose::LivenessWitness,
            vec![*other],
            true,
            &program,
        ));
    }
    Ok(TimeoutPathComposition {
        callback_summary_sha256: summary_sha256(callback),
        registration_summary_sha256: summary_sha256(registration),
        registration_paths: facts.paths,
        event_flags: facts.flags,
        path_steps,
        teardown_paths,
        non_teardown_paths,
        map_exclusive_to_registration: facts.exclusive,
        programs,
    })
}

// ---------------------------------------------------------------------------
// Relevance, status and verdict.

const ADMITTED_CALLBACK_CALLS: [&str; 8] = [
    "send",
    "__errno_location",
    "__error",
    "perror",
    "evutil_closesocket",
    "event_del",
    "event_free",
    "mk_mem_free",
];

fn has_unadmitted_call(summary: &SourceSummary) -> bool {
    summary.paths().iter().any(|path| {
        path.operations.iter().any(|operation| {
            matches!(operation, SourceOperation::Call { function, .. } if !ADMITTED_CALLBACK_CALLS.contains(&function.as_str()))
                || matches!(operation, SourceOperation::ScanLoop { .. })
        })
    })
}

fn path_reads_field(path: &SourcePath, name: &str) -> bool {
    path.operations.iter().any(|operation| {
        operation_mentions(operation, &|expression| {
            matches!(expression, SourceExpression::Field { name: found, .. } if found == name)
        })
    })
}

fn path_uses_argument(path: &SourcePath) -> bool {
    path.operations
        .iter()
        .any(|operation| operation_mentions(operation, &is_parameter(2)))
}

fn relevant_paths(
    summary: &SourceSummary,
    obligation: Cycle18Obligation,
) -> (BTreeSet<usize>, String) {
    let all = (0..summary.paths().len()).collect::<BTreeSet<_>>();
    let opaque = has_unadmitted_call(summary);
    match obligation {
        Cycle18Obligation::InitializationReachability => (
            all,
            "every selected callback is registered on a context returned by _mk_event_loop_create"
                .into(),
        ),
        Cycle18Obligation::CallerNullHandling => (
            all,
            "each registration wrapper dereferences the loop context the caller obtained".into(),
        ),
        Cycle18Obligation::CallerSizeBounds => {
            let paths = all
                .iter()
                .copied()
                .filter(|path| opaque || path_reads_field(&summary.paths()[*path], "fired"))
                .collect();
            (
                paths,
                "paths that access the fired array sized by the caller (or call an unadmitted function)".into(),
            )
        }
        Cycle18Obligation::AllocationAuthorityProvenance => {
            let paths = all
                .iter()
                .copied()
                .filter(|path| opaque || path_uses_argument(&summary.paths()[*path]))
                .collect();
            (
                paths,
                "paths that dereference, write through or release the callback argument".into(),
            )
        }
        Cycle18Obligation::RegistrationLiveness => (
            all,
            "every activation depends on the callback's event registration".into(),
        ),
        Cycle18Obligation::NaturalActivationReachability => {
            (all, "every path is entered by a Libevent activation".into())
        }
        Cycle18Obligation::AllPathExecutionCoverage => {
            (all, "every imported source path must be composed".into())
        }
    }
}

fn scenario_relevance(scenario: &str, callback: &str) -> Option<String> {
    match scenario {
        "base_creation_failure" | "fired_allocation_failure" | "context_allocation_failure" => {
            Some("_mk_event_loop_create precedes every callback registration".into())
        }
        "timer_creation_failure" | "timer_registration_failure" => (callback
            == "cb_wait_2_timeout")
            .then(|| "_mk_event_wait_2 registers cb_wait_2_timeout".into()),
        // An unrecognized scenario blocks every callback (fail-closed).
        _ => Some("unrecognized provisional-Unsafe scenario; conservatively relevant".into()),
    }
}

fn decide(
    family_refused: Option<String>,
    obligations: &BTreeMap<Cycle18Obligation, RelevantObligation>,
    blocking: &BTreeMap<String, String>,
) -> CallbackVerdict {
    let open = obligations
        .iter()
        .filter(|(_, row)| !row.paths.is_empty() && !row.status.is_discharged())
        .map(|(obligation, _)| *obligation)
        .collect::<BTreeSet<_>>();
    if family_refused.is_none() && open.is_empty() && blocking.is_empty() {
        CallbackVerdict::Composed
    } else {
        CallbackVerdict::Unknown {
            open_obligations: open,
            blocking_scenarios: blocking.keys().cloned().collect(),
            evidence_family_refused: family_refused,
        }
    }
}

struct Dischargers {
    initialization: Result<InitializationDischarge, String>,
    activation: BTreeMap<String, Result<ActivationFlagDependence, String>>,
    timeout: Result<TimeoutPathComposition, String>,
    registration: BTreeMap<String, Vec<RegistrationSite>>,
}

fn aggregate_needs(aggregate: &AggregateFacts, obligation: Cycle18Obligation) -> String {
    aggregate
        .unresolved
        .get(obligation.name())
        .cloned()
        .unwrap_or_else(|| format!("{} is unresolved", obligation.name()))
}

fn open(needs: impl Into<String>, partial: Vec<PartialDischarge>) -> ObligationStatus {
    ObligationStatus::Open {
        needs: needs.into(),
        partial,
    }
}

fn registration_needs(sites: &[RegistrationSite]) -> String {
    let mut groups =
        BTreeMap::<(String, bool, Option<bool>, bool, BTreeSet<String>), Vec<usize>>::new();
    for site in sites {
        let ignored = site
            .ignored_results
            .iter()
            .map(|call| call.split('@').next().unwrap_or(call).to_owned())
            .collect();
        groups
            .entry((
                site.wrapper.clone(),
                site.creation_result_checked,
                site.registration_result_checked,
                site.stale_event_field_on_failure,
                ignored,
            ))
            .or_default()
            .push(site.path);
    }
    let facts = groups
        .into_iter()
        .map(
            |((wrapper, creation, registration, stale, ignored), paths)| {
                format!(
                    "{wrapper} paths {paths:?}: event_new result {}, event_add result {}{}{}",
                    if creation { "checked" } else { "unchecked" },
                    match registration {
                        Some(true) => "checked",
                        Some(false) => "unchecked",
                        None => "not reached",
                    },
                    if stale {
                        ", the failure return leaves the map's freed event field in place"
                    } else {
                        ""
                    },
                    if ignored.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ", unguarded results of {}",
                            ignored.into_iter().collect::<Vec<_>>().join(",")
                        )
                    }
                )
            },
        )
        .collect::<Vec<_>>()
        .join("; ");
    if facts.is_empty() {
        "no registration site was identified in the supplied wrapper summaries".into()
    } else {
        facts
    }
}

fn status_for(
    aggregate: &AggregateFacts,
    callback: &str,
    obligation: Cycle18Obligation,
    family_joined: bool,
    dischargers: Option<&Dischargers>,
) -> ObligationStatus {
    let needs = aggregate_needs(aggregate, obligation);
    let family = |kind_text: &str| ObligationStatus::Discharged {
        kind: EvidenceKind::BoundedNativeFamily,
        method: "joined bounded evidence family of the rechecked aggregate".into(),
        established: kind_text.into(),
    };
    match (obligation, callback) {
        (Cycle18Obligation::AllPathExecutionCoverage, "cb_event") if family_joined => {
            family("four controlled-activation executions cover the four cb_event source paths")
        }
        (Cycle18Obligation::AllPathExecutionCoverage, "cb_wait_2_timeout") if family_joined => {
            family("one identity-bearing observed execution covers the sole cb_wait_2_timeout path")
        }
        _ => {
            let Some(dischargers) = dischargers else {
                return open(needs, Vec::new());
            };
            discharged_status(obligation, callback, needs, dischargers)
        }
    }
}

fn discharged_status(
    obligation: Cycle18Obligation,
    callback: &str,
    needs: String,
    dischargers: &Dischargers,
) -> ObligationStatus {
    match obligation {
        Cycle18Obligation::InitializationReachability => match &dischargers.initialization {
            Ok(discharge) => ObligationStatus::Discharged {
                kind: EvidenceKind::SourceFact,
                method: "constant _mk_event_init result refutes the initialization-failure guard"
                    .into(),
                established: format!(
                    "_mk_event_init returns {} on every imported path; loop-create paths {:?} are infeasible and {:?} remain",
                    discharge.constant_result, discharge.infeasible_paths, discharge.feasible_paths
                ),
            },
            Err(reason) => open(format!("{needs}; discharger refused: {reason}"), Vec::new()),
        },
        Cycle18Obligation::NaturalActivationReachability => {
            match dischargers.activation.get(callback) {
                Some(Ok(ActivationFlagDependence::Independent)) => ObligationStatus::Discharged {
                    kind: EvidenceKind::SourceFact,
                    method: "no operation on any imported path reads the activation flags".into(),
                    established: format!(
                        "every activation of {callback}, natural or controlled, selects its path from its argument state alone"
                    ),
                },
                Some(Ok(ActivationFlagDependence::BitGuarded {
                    tested_bits,
                    exhaustive,
                    ..
                })) => open(
                    "path selection is settled from source, but the dispatcher state at entry (fired_count against the fired capacity) under natural multi-readiness is not bounded; it reduces to allocation_authority_provenance",
                    vec![PartialDischarge {
                        kind: EvidenceKind::SourceFact,
                        method: "single-bit activation guards".into(),
                        established: format!(
                            "activation flags are read only by single-bit guards on bits {tested_bits:?}; exhaustive path partition: {exhaustive}"
                        ),
                    }],
                ),
                Some(Err(reason)) => {
                    open(format!("{needs}; discharger refused: {reason}"), Vec::new())
                }
                None => open(needs, Vec::new()),
            }
        }
        Cycle18Obligation::AllPathExecutionCoverage if callback == "cb_timeout" => {
            match &dischargers.timeout {
                Ok(composition)
                    if composition.close_success_branch_composes()
                        && composition.close_failure_branch_composes() =>
                {
                    ObligationStatus::Discharged {
                        kind: EvidenceKind::ModelProof,
                        method: "source-derived Cycle 18 event programs for every cb_timeout path"
                            .into(),
                        established: "every path composes on both close outcomes".into(),
                    }
                }
                Ok(composition) => open(
                    "evutil_closesocket(ev_map->pipe[1])'s result is ignored and its failure post-state is indeterminate in the Cycle 18 close vocabulary, so the close-failure branch of the teardown paths is not composed",
                    if composition.close_success_branch_composes() {
                        vec![PartialDischarge {
                            kind: EvidenceKind::ModelProof,
                            method: "source-derived Cycle 18 event programs".into(),
                            established: format!(
                                "teardown paths {:?} and non-teardown paths {:?} compose on the close-success branch, including a non-teardown activation followed by a teardown activation",
                                composition.teardown_paths, composition.non_teardown_paths
                            ),
                        }]
                    } else {
                        Vec::new()
                    },
                ),
                Err(reason) => open(format!("{needs}; discharger refused: {reason}"), Vec::new()),
            }
        }
        Cycle18Obligation::AllocationAuthorityProvenance if callback == "cb_timeout" => {
            let partial = match &dischargers.timeout {
                Ok(composition) if composition.map_exclusive_to_registration => vec![PartialDischarge {
                    kind: EvidenceKind::SourceFact,
                    method: "registration-local escape check".into(),
                    established: "_mk_event_timeout_create allocates the map on the registering path and hands it only to event_new; the created event is stored only in that map and registered".into(),
                }],
                _ => Vec::new(),
            };
            open(
                "the owned-argument mode of the teardown model is declared; whole-program non-use of Libevent's callback-argument accessors is not established",
                partial,
            )
        }
        Cycle18Obligation::AllocationAuthorityProvenance if callback == "cb_event" => open(
            format!(
                "{needs}; cb_event writes ctx->fired[ctx->fired_count] with no capacity guard, so exclusive authority needs a bound on activations per dispatch against queue_size"
            ),
            Vec::new(),
        ),
        Cycle18Obligation::RegistrationLiveness => {
            let sites = dischargers
                .registration
                .get(callback)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let subject = if callback == "cb_event" {
                "establish behavior after event creation or registration failure against the selected backend".to_owned()
            } else {
                needs
            };
            let mut text = format!("{subject}; source facts: {}", registration_needs(sites));
            if callback == "cb_timeout"
                && let Ok(composition) = &dischargers.timeout
                && composition.liveness_gap_witnessed()
            {
                text.push_str(
                    "; the model refuses a trace in which the non-teardown path runs and the base is freed before a teardown activation",
                );
            }
            open(text, Vec::new())
        }
        _ => open(needs, Vec::new()),
    }
}

fn build_table(
    aggregate: &AggregateFacts,
    summaries: &BTreeMap<&str, &SourceSummary>,
    dischargers: Option<&Dischargers>,
) -> ObligationTable {
    let mut rows = Vec::new();
    let mut cells = Vec::new();
    for (callback, _) in CALLBACKS {
        let summary = summaries[callback];
        let family_joined = aggregate.joined.contains(callback) && aggregate.refused.is_empty();
        let family_refused = (!family_joined).then(|| {
            aggregate
                .refused
                .iter()
                .map(|(family, reason)| format!("{family}: {reason}"))
                .collect::<Vec<_>>()
                .join("; ")
        });
        let family_refused = family_refused.map(|reason| {
            if reason.is_empty() {
                "evidence family did not join".into()
            } else {
                reason
            }
        });
        let mut obligations = BTreeMap::new();
        for obligation in Cycle18Obligation::ALL {
            let (paths, relevance) = relevant_paths(summary, obligation);
            if paths.is_empty() {
                continue;
            }
            let status = status_for(aggregate, callback, obligation, family_joined, dischargers);
            for path in &paths {
                cells.push(PathObligationCell {
                    callback: callback.into(),
                    path: *path,
                    obligation,
                    discharged: status.is_discharged(),
                    kind: match &status {
                        ObligationStatus::Discharged { kind, .. } => Some(*kind),
                        ObligationStatus::Open { .. } => None,
                    },
                });
            }
            obligations.insert(
                obligation,
                RelevantObligation {
                    paths,
                    relevance,
                    status,
                },
            );
        }
        let blocking = aggregate
            .provisional_unsafe
            .iter()
            .filter_map(|scenario| {
                scenario_relevance(scenario, callback).map(|reason| (scenario.clone(), reason))
            })
            .collect::<BTreeMap<_, _>>();
        let verdict = decide(family_refused, &obligations, &blocking);
        rows.push(CallbackObligationRow {
            callback: callback.into(),
            source_paths: summary.paths().len(),
            source_summary_sha256: summary_sha256(summary),
            obligations,
            blocking_scenarios: blocking,
            verdict,
        });
    }
    let relevant_cells = cells.len();
    let discharged_cells = cells.iter().filter(|cell| cell.discharged).count();
    let composed_callbacks = rows
        .iter()
        .filter(|row| row.verdict == CallbackVerdict::Composed)
        .count();
    ObligationTable {
        unknown_callbacks: rows.len() - composed_callbacks,
        composed_callbacks,
        callbacks: rows,
        cells,
        relevant_cells,
        discharged_cells,
        open_cells: relevant_cells - discharged_cells,
    }
}

fn assemble(
    aggregate: &AggregateFacts,
    sources: &CallbackObligationSources<'_>,
) -> Result<CallbackObligationReport, String> {
    if aggregate.verdict != Cycle18AggregateVerdict::Unknown
        || aggregate.source_revision != monkey_source_revision()
    {
        return Err("aggregate does not describe the pinned Monkey source".into());
    }
    let profile = aggregate.profile_sha256.as_str();
    let mut summaries = BTreeMap::new();
    for summary in sources.callbacks {
        check_import_identity(
            summary,
            summary.function(),
            "Callback",
            &aggregate.source_revision,
            profile,
        )?;
        if summaries.insert(summary.function(), summary).is_some() {
            return Err(format!("duplicate callback summary {}", summary.function()));
        }
    }
    for (callback, paths) in CALLBACKS {
        let summary = summaries
            .get(callback)
            .ok_or_else(|| format!("callback summary {callback} is missing"))?;
        if summary.paths().len() != paths
            || aggregate.callback_source_sha256.get(callback) != Some(&summary_sha256(summary))
        {
            return Err(format!(
                "{callback} summary is not the one the aggregate rechecked"
            ));
        }
    }
    if summaries.len() != CALLBACKS.len() {
        return Err("callback summaries are not exactly the three selected callbacks".into());
    }
    for (summary, name) in [
        (sources.event_add, "_mk_event_add"),
        (sources.event_update, "_mk_event_update"),
        (sources.wait_2, "_mk_event_wait_2"),
    ] {
        check_import_identity(
            summary,
            name,
            "Wrapper",
            &aggregate.source_revision,
            profile,
        )?;
    }
    let initialization =
        discharge_initialization_reachability(sources.event_init, sources.loop_create, profile);
    let activation = summaries
        .iter()
        .map(|(name, summary)| ((*name).to_owned(), activation_flag_dependence(summary)))
        .collect::<BTreeMap<_, _>>();
    let timeout = compose_timeout_source_paths(
        summaries["cb_timeout"],
        sources.timeout_projection,
        sources.timeout_create,
        profile,
    );
    let registration = BTreeMap::from([
        (
            "cb_event".to_owned(),
            [sources.event_add, sources.event_update]
                .into_iter()
                .flat_map(|wrapper| registration_sites("cb_event", wrapper))
                .collect::<Vec<_>>(),
        ),
        (
            "cb_timeout".to_owned(),
            registration_sites("cb_timeout", sources.timeout_create),
        ),
        (
            "cb_wait_2_timeout".to_owned(),
            registration_sites("cb_wait_2_timeout", sources.wait_2),
        ),
    ]);
    let dischargers = Dischargers {
        initialization,
        activation,
        timeout,
        registration,
    };
    let before = build_table(aggregate, &summaries, None);
    let after = build_table(aggregate, &summaries, Some(&dischargers));
    let total_paths = CALLBACKS
        .iter()
        .map(|(_, paths)| *paths as u64)
        .sum::<u64>();
    Ok(CallbackObligationReport {
        artifact: "fluent-bit/monkey/libevent selected callbacks".into(),
        property: CALLBACK_OBLIGATIONS_PROPERTY.into(),
        semantic_profile: "fluent-bit/monkey/libevent".into(),
        scope: CALLBACKS.iter().map(|(name, _)| (*name).to_owned()).collect(),
        bounds: BTreeMap::from([
            ("callbacks".into(), CALLBACKS.len() as u64),
            ("source_paths".into(), total_paths),
            ("obligations".into(), Cycle18Obligation::ALL.len() as u64),
            ("relevant_cells_after".into(), after.relevant_cells as u64),
            ("discharged_cells_after".into(), after.discharged_cells as u64),
        ]),
        assumptions: BTreeMap::from([
            (
                "source_import".into(),
                "faithful Clang/C extraction of the pinned translation unit under the selected profile".into(),
            ),
            (
                "lean_rust_correspondence".into(),
                "the decision rule and dischargers match QCore.Cycle18CallbackObligations by review, not extraction".into(),
            ),
            (
                "enclosing_peer_close".into(),
                "the cb_timeout model closes the peer endpoint as the caller's _mk_event_timeout_destroy does; this is an enclosing premise".into(),
            ),
            (
                "runtime_provenance".into(),
                "joined native families identify presented bytes; they do not attest execution".into(),
            ),
        ]),
        evidence_version: CALLBACK_OBLIGATIONS_EVIDENCE_VERSION.into(),
        source_revision: aggregate.source_revision.clone(),
        profile_sha256: aggregate.profile_sha256.clone(),
        aggregate_verdict: aggregate.verdict,
        aggregate_policy_sha256: aggregate.policy_sha256.clone(),
        aggregate_source_inventory_sha256: aggregate.inventory_sha256.clone(),
        aggregate_unresolved_obligations: aggregate.unresolved.clone(),
        aggregate_provisional_unsafe_scenarios: aggregate.provisional_unsafe.clone(),
        source_summary_sha256: [
            sources.event_init,
            sources.loop_create,
            sources.event_add,
            sources.event_update,
            sources.timeout_create,
            sources.wait_2,
        ]
        .into_iter()
        .chain(sources.callbacks)
        .map(|summary| (summary.function().to_owned(), summary_sha256(summary)))
        .collect(),
        before,
        after,
        initialization_refusal: dischargers.initialization.as_ref().err().cloned(),
        initialization: dischargers.initialization.ok(),
        activation: dischargers
            .activation
            .into_iter()
            .filter_map(|(name, result)| result.ok().map(|value| (name, value)))
            .collect(),
        timeout_paths_refusal: dischargers.timeout.as_ref().err().cloned(),
        timeout_paths: dischargers.timeout.ok(),
        registration_sites: dischargers.registration,
        universal_safety_certificate: false,
    })
}

#[cfg(test)]
#[path = "../tests/unit/cycle18_callback_obligations.rs"]
#[cfg_attr(windows, allow(dead_code))]
mod tests;
