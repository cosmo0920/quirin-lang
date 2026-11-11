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

//! Bounded failure-path classification for Monkey loop creation and timed wait.

use super::{MonkeyWaitSourceProjection, monkey_source_revision};
use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};

const POLICY: &str = include_str!(
    "../../../../providers/qcore-provider-libevent/contracts/monkey/wait_failure_policy.qv"
);

pub fn wait_failure_policy_digest() -> String {
    sha256_hex(POLICY.as_bytes())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitFailureOutcome {
    Unsafe,
    Unknown,
}
impl WaitFailureOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unsafe => "unsafe",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitFailureEvidenceClass {
    InstrumentedSourceHarness,
    SourceModel,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WaitFailureObservation {
    pub schema_version: u32,
    pub source_revision: String,
    pub scenario: String,
    pub path: String,
    pub trace: Vec<String>,
    pub returned_null: bool,
    pub returned_context_base_nonnull: bool,
    pub base_created: bool,
    pub base_freed: bool,
    pub context_freed: bool,
    pub returned_dispatch_result: bool,
    pub event_add_argument_nonnull: bool,
    pub event_free_argument_nonnull: bool,
    pub event_add_result: i32,
    pub timeout_activated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckedWaitFailureCase {
    pub scenario: String,
    pub outcome: WaitFailureOutcome,
    pub evidence_class: WaitFailureEvidenceClass,
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: String,
    pub assumptions: Vec<String>,
    pub evidence_version: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitFailureError {
    Decode(String),
    SourceProjectionMismatch,
    MissingScenario(String),
    DuplicateScenario(String),
    UnexpectedScenario(String),
    Policy(String),
}

const SCENARIOS: [&str; 6] = [
    "context_allocation_failure",
    "base_creation_failure",
    "fired_allocation_failure",
    "timer_creation_failure",
    "timer_registration_failure",
    "success_observation",
];
fn integer(value: bool) -> ContractFact {
    ContractFact::Integer(i128::from(value))
}
fn text_fact(function: &str, name: &str) -> Result<String, WaitFailureError> {
    match contract_fact_literal(POLICY, function, name).map_err(WaitFailureError::Policy)? {
        ContractFact::Text(value) => Ok(value),
        ContractFact::Integer(_) => Err(WaitFailureError::Policy(format!(
            "{function}: {name} is not text"
        ))),
    }
}
fn policy_outcome(function: &str) -> Result<WaitFailureOutcome, WaitFailureError> {
    match text_fact(function, "outcome")?.as_str() {
        "unsafe" => Ok(WaitFailureOutcome::Unsafe),
        "unknown" => Ok(WaitFailureOutcome::Unknown),
        other => Err(WaitFailureError::Policy(format!(
            "{function}: unsupported outcome {other}"
        ))),
    }
}
fn tuple(
    scenario: &str,
    outcome: WaitFailureOutcome,
    evidence_class: WaitFailureEvidenceClass,
    artifact: &str,
    scope: &str,
    bounds: &str,
    assumptions: &[&str],
) -> Result<CheckedWaitFailureCase, WaitFailureError> {
    Ok(CheckedWaitFailureCase {
        scenario: scenario.into(),
        outcome,
        evidence_class,
        artifact: artifact.into(),
        property: text_fact(scenario, "property")?,
        semantic_profile: "fluent-bit/unix-libevent source projection".into(),
        scope: scope.into(),
        bounds: bounds.into(),
        assumptions: assumptions.iter().map(|value| (*value).into()).collect(),
        evidence_version: sha256_hex(POLICY.as_bytes()),
        reason: text_fact(scenario, "reason")?,
    })
}

/// Check the complete source inventory plus six explicit harness observations.
pub fn check_wait_failure_paths(
    source: &MonkeyWaitSourceProjection,
    bytes: &[u8],
) -> Result<Vec<CheckedWaitFailureCase>, WaitFailureError> {
    if source.source_revision() != monkey_source_revision()
        || source.selected_path_indices().get("_mk_event_loop_create") != Some(&3)
        || source.selected_path_indices().get("_mk_event_wait_2") != Some(&2)
        || source.source_path_counts().get("_mk_event_loop_create") != Some(&4)
        || source.source_path_counts().get("_mk_event_wait_2") != Some(&3)
    {
        return Err(WaitFailureError::SourceProjectionMismatch);
    }
    let observations: Vec<WaitFailureObservation> = serde_json::from_slice(bytes)
        .map_err(|error| WaitFailureError::Decode(error.to_string()))?;
    let mut checked = Vec::new();
    for scenario in SCENARIOS {
        let matches = observations
            .iter()
            .filter(|row| row.scenario == scenario)
            .collect::<Vec<_>>();
        let row = match matches.as_slice() {
            [] => return Err(WaitFailureError::MissingScenario(scenario.into())),
            [row] => *row,
            _ => return Err(WaitFailureError::DuplicateScenario(scenario.into())),
        };
        if row.schema_version != 1 || row.source_revision != monkey_source_revision() {
            return Err(WaitFailureError::Policy(format!(
                "{scenario}: identity mismatch"
            )));
        }
        let outcome = policy_outcome(scenario)?;
        let facts = [
            ("source_bound", ContractFact::Integer(1)),
            ("path", ContractFact::Text(row.path.clone())),
            ("outcome", ContractFact::Text(outcome.as_str().into())),
            (
                "property",
                ContractFact::Text(text_fact(scenario, "property")?),
            ),
            ("reason", ContractFact::Text(text_fact(scenario, "reason")?)),
            ("trace", ContractFact::Text(row.trace.join(","))),
            ("returned_null", integer(row.returned_null)),
            (
                "returned_context_base_nonnull",
                integer(row.returned_context_base_nonnull),
            ),
            ("base_created", integer(row.base_created)),
            ("base_freed", integer(row.base_freed)),
            ("context_freed", integer(row.context_freed)),
            (
                "returned_dispatch_result",
                integer(row.returned_dispatch_result),
            ),
            (
                "event_add_argument_nonnull",
                integer(row.event_add_argument_nonnull),
            ),
            (
                "event_free_argument_nonnull",
                integer(row.event_free_argument_nonnull),
            ),
            (
                "event_add_result",
                ContractFact::Integer(i128::from(row.event_add_result)),
            ),
            ("timeout_activated", integer(row.timeout_activated)),
        ];
        check_typed_contract_facts(POLICY, scenario, &facts).map_err(WaitFailureError::Policy)?;
        checked.push(tuple(
            scenario,
            outcome,
            WaitFailureEvidenceClass::InstrumentedSourceHarness,
            "approved Monkey wrapper fixture with stubbed dependency fault model",
            "one source-preserving harness scenario; no real Libevent behavior claim",
            "loop size 2; one dispatcher; fixed stub dependency outcomes",
            &[
                "fixture digest is bound by the sealed source projection",
                "stub results model permitted dependency failures",
            ],
        )?);
    }
    if observations.len() != SCENARIOS.len() {
        let unexpected = observations
            .iter()
            .find(|row| !SCENARIOS.contains(&row.scenario.as_str()));
        return Err(WaitFailureError::UnexpectedScenario(
            unexpected.map_or("duplicate", |row| &row.scenario).into(),
        ));
    }
    for (scenario, scope, assumption) in [
        (
            "loop_init_failure",
            "source-model path only",
            "opaque call reachability is not established",
        ),
        (
            "loop_size_bounds",
            "source-model allocation extent only",
            "no caller range proof is supplied",
        ),
    ] {
        let model_facts = [
            ("source_bound", ContractFact::Integer(1)),
            ("path", ContractFact::Text("_mk_event_loop_create".into())),
            (
                "outcome",
                ContractFact::Text(policy_outcome(scenario)?.as_str().into()),
            ),
            ("path_count", ContractFact::Integer(4)),
            (
                "property",
                ContractFact::Text(text_fact(scenario, "property")?),
            ),
            ("reason", ContractFact::Text(text_fact(scenario, "reason")?)),
        ];
        check_typed_contract_facts(POLICY, scenario, &model_facts)
            .map_err(WaitFailureError::Policy)?;
        checked.push(tuple(
            scenario,
            policy_outcome(scenario)?,
            WaitFailureEvidenceClass::SourceModel,
            "sealed complete Monkey source path inventory",
            scope,
            "four imported loop-create paths",
            &[assumption],
        )?);
    }
    Ok(checked)
}
