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

//! Fail-closed admission of host-specific native evidence rows.

use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const POLICY: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/native_platform_matrix.qv"
);
const REQUIRED_PROFILES: [&str; 7] = [
    "linux_epoll_asan_profile",
    "linux_poll_asan_profile",
    "linux_select_asan_profile",
    "macos_kqueue_asan_profile",
    "windows_win32_asan_profile",
    "windows_iocp_asan_profile",
    "linux_epoll_tsan_profile",
];
pub fn native_platform_policy_digest() -> String {
    sha256_hex(POLICY.as_bytes())
}

pub fn native_platform_source_revision() -> Result<String, String> {
    literal("native_platform_source_identity", "libevent_revision")
}

pub fn native_platform_historical_digest() -> Result<String, String> {
    literal(
        "native_platform_source_identity",
        "historical_validation_sha256",
    )
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformRowStatus {
    Observed,
    Unknown,
}
impl PlatformRowStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformDimensions {
    pub host: String,
    pub backend: String,
    pub sanitizers: String,
    pub thread_adapter: String,
    pub observer: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub libevent_revision: String,
    pub harness_sha256: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
    pub compiler_path: String,
    pub compiler_version: String,
    pub command_sha256: String,
    pub libevent_core_sha256: String,
    pub libevent_pthreads_sha256: String,
    pub executable_sha256: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub system: String,
    pub machine: String,
    pub release: String,
    pub selected_backend: String,
    pub dispatch_thread: u64,
    pub activating_thread: u64,
    pub callback_thread: u64,
    pub process_exit: i32,
    pub activation_count: u32,
    pub callback_count: u32,
    pub callback_flags: u32,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEvidence {
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub runtime: RuntimeIdentity,
    pub property: String,
    pub observer_scope: String,
    pub observer_transparency: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalNativeResult {
    pub source_document_sha256: String,
    pub source_revision: String,
    pub recorded_result: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformRow {
    pub policy: String,
    pub dimensions: PlatformDimensions,
    pub status: PlatformRowStatus,
    pub reason: Option<String>,
    pub evidence: Option<ObservedEvidence>,
    pub historical_native_result: Option<HistoricalNativeResult>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativePlatformMatrix {
    pub schema_version: u32,
    pub artifact: String,
    pub rows: Vec<PlatformRow>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckedNativePlatformMatrix {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: String,
    pub assumptions: Vec<String>,
    pub evidence_version: String,
    pub rows: Vec<PlatformRow>,
}

fn text(value: &str) -> ContractFact {
    ContractFact::Text(value.into())
}
fn integer(value: impl Into<i128>) -> ContractFact {
    ContractFact::Integer(value.into())
}
fn literal(function: &str, fact: &str) -> Result<String, String> {
    match contract_fact_literal(POLICY, function, fact)? {
        ContractFact::Text(value) => Ok(value),
        _ => Err(format!("{function}.{fact} must be text")),
    }
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn check_native_platform_matrix(
    matrix: NativePlatformMatrix,
    expected_harness_sha256: &str,
) -> Result<CheckedNativePlatformMatrix, String> {
    if matrix.schema_version != 1
        || matrix.artifact != literal("native_platform_source_identity", "artifact")?
    {
        return Err("unsupported native platform matrix".into());
    }
    let policies = matrix
        .rows
        .iter()
        .map(|row| row.policy.as_str())
        .collect::<BTreeSet<_>>();
    if matrix.rows.len() != REQUIRED_PROFILES.len()
        || policies != REQUIRED_PROFILES.into_iter().collect()
    {
        return Err(
            "native platform matrix must contain every required profile exactly once".into(),
        );
    }
    for row in &matrix.rows {
        let historical_label = literal(&row.policy, "historical_label")?;
        let profile_facts = [
            ("host", text(&row.dimensions.host)),
            ("backend", text(&row.dimensions.backend)),
            ("sanitizers", text(&row.dimensions.sanitizers)),
            ("thread_adapter", text(&row.dimensions.thread_adapter)),
            ("observer", text(&row.dimensions.observer)),
            (
                "historical_label",
                text(
                    row.historical_native_result
                        .as_ref()
                        .map_or("", |history| history.recorded_result.as_str()),
                ),
            ),
            (
                "unknown_reason",
                text(&literal(&row.policy, "unknown_reason")?),
            ),
        ];
        check_typed_contract_facts(POLICY, &row.policy, &profile_facts)?;
        match &row.historical_native_result {
            Some(history)
                if valid_sha256(&history.source_document_sha256)
                    && history.source_document_sha256 == native_platform_historical_digest()?
                    && history.source_revision == native_platform_source_revision()?
                    && history.recorded_result == historical_label => {}
            None if historical_label.is_empty() => {}
            _ => {
                return Err(format!(
                    "{} historical native identity mismatch",
                    row.policy
                ));
            }
        }
        match row.status {
            PlatformRowStatus::Observed => {
                if row.reason.is_some() {
                    return Err("observed row carries an Unknown reason".into());
                }
                let evidence = row.evidence.as_ref().ok_or("observed row lacks evidence")?;
                for digest in [
                    &evidence.source.harness_sha256,
                    &evidence.build.command_sha256,
                    &evidence.build.libevent_core_sha256,
                    &evidence.build.libevent_pthreads_sha256,
                    &evidence.build.executable_sha256,
                ] {
                    if !valid_sha256(digest) {
                        return Err("observed row carries an invalid digest".into());
                    }
                }
                if evidence.build.compiler_path.is_empty()
                    || evidence.build.compiler_version.is_empty()
                    || evidence.runtime.machine.is_empty()
                    || evidence.runtime.release.is_empty()
                {
                    return Err("observed row lacks attributable build/runtime identity".into());
                }
                let expected_system = match row.dimensions.host.as_str() {
                    "linux" => "Linux",
                    "macos" => "Darwin",
                    "windows" => "Windows",
                    _ => "",
                };
                let facts = [
                    ("status", text(row.status.as_str())),
                    ("property", text(&evidence.property)),
                    ("observer_scope", text(&evidence.observer_scope)),
                    (
                        "observer_transparency",
                        text(&evidence.observer_transparency),
                    ),
                    (
                        "source_revision_actual",
                        text(&evidence.source.libevent_revision),
                    ),
                    (
                        "source_revision_expected",
                        text(&native_platform_source_revision()?),
                    ),
                    (
                        "harness_sha256_actual",
                        text(&evidence.source.harness_sha256),
                    ),
                    ("harness_sha256_expected", text(expected_harness_sha256)),
                    ("runtime_host_actual", text(&evidence.runtime.system)),
                    ("runtime_host_expected", text(expected_system)),
                    (
                        "runtime_backend_actual",
                        text(&evidence.runtime.selected_backend),
                    ),
                    ("runtime_backend_expected", text(&row.dimensions.backend)),
                    ("process_exit", integer(evidence.runtime.process_exit)),
                    (
                        "activation_count",
                        integer(evidence.runtime.activation_count),
                    ),
                    ("callback_count", integer(evidence.runtime.callback_count)),
                    ("callback_flags", integer(evidence.runtime.callback_flags)),
                    (
                        "dispatch_thread",
                        integer(i128::from(evidence.runtime.dispatch_thread)),
                    ),
                    (
                        "activating_thread",
                        integer(i128::from(evidence.runtime.activating_thread)),
                    ),
                    (
                        "callback_thread",
                        integer(i128::from(evidence.runtime.callback_thread)),
                    ),
                ];
                check_typed_contract_facts(POLICY, "observed_row", &facts)?;
            }
            PlatformRowStatus::Unknown => {
                if row.evidence.is_some() {
                    return Err("Unknown row carries current observed evidence".into());
                }
                let facts = [
                    ("status", text(row.status.as_str())),
                    ("reason_actual", text(row.reason.as_deref().unwrap_or(""))),
                    (
                        "reason_expected",
                        text(&literal(&row.policy, "unknown_reason")?),
                    ),
                    ("current_evidence_item_count", integer(0)),
                    ("observer_transparency", text("not_claimed")),
                ];
                check_typed_contract_facts(POLICY, "unknown_row", &facts)?;
            }
        }
    }
    Ok(CheckedNativePlatformMatrix {
        artifact: matrix.artifact, property: "generic_libevent_cross_thread_activation".into(),
        semantic_profile: "cycle18.native_platform_matrix.v1".into(),
        scope: "generic Libevent probes only; no Monkey callback or wrapper behavior".into(),
        bounds: "selected host/backend/sanitizer/thread/observer rows; each current result stands only for its own dimensions".into(),
        assumptions: vec!["producer-supplied runtime and linkage identities identify the measured artifacts".into(), "observer instrumentation does not establish transparency".into(), "carried historical results do not substitute for current selected-observer evidence".into()],
        evidence_version: native_platform_policy_digest(), rows: matrix.rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn history(label: &str) -> HistoricalNativeResult {
        HistoricalNativeResult {
            source_document_sha256: native_platform_historical_digest().unwrap(),
            source_revision: native_platform_source_revision().unwrap(),
            recorded_result: label.into(),
        }
    }
    fn matrix() -> NativePlatformMatrix {
        NativePlatformMatrix {
            schema_version: 1,
            artifact: "libevent/generic-cross-thread-activation".into(),
            rows: REQUIRED_PROFILES
                .iter()
                .map(|policy| {
                    let label = literal(policy, "historical_label").unwrap();
                    PlatformRow {
                        policy: (*policy).into(),
                        dimensions: PlatformDimensions {
                            host: literal(policy, "host").unwrap(),
                            backend: literal(policy, "backend").unwrap(),
                            sanitizers: literal(policy, "sanitizers").unwrap(),
                            thread_adapter: literal(policy, "thread_adapter").unwrap(),
                            observer: literal(policy, "observer").unwrap(),
                        },
                        status: PlatformRowStatus::Unknown,
                        reason: Some(literal(policy, "unknown_reason").unwrap()),
                        evidence: None,
                        historical_native_result: (!label.is_empty()).then(|| history(&label)),
                    }
                })
                .collect(),
        }
    }
    fn observe_first(mut value: NativePlatformMatrix) -> NativePlatformMatrix {
        value.rows[0].status = PlatformRowStatus::Observed;
        value.rows[0].reason = None;
        value.rows[0].evidence = Some(ObservedEvidence {
            source: SourceIdentity {
                libevent_revision: native_platform_source_revision().unwrap(),
                harness_sha256: "a".repeat(64),
            },
            build: BuildIdentity {
                compiler_path: "/usr/bin/clang".into(),
                compiler_version: "clang".into(),
                command_sha256: "b".repeat(64),
                libevent_core_sha256: "c".repeat(64),
                libevent_pthreads_sha256: "d".repeat(64),
                executable_sha256: "e".repeat(64),
            },
            runtime: RuntimeIdentity {
                system: "Linux".into(),
                machine: "x86_64".into(),
                release: "test".into(),
                selected_backend: "epoll".into(),
                dispatch_thread: 10,
                activating_thread: 11,
                callback_thread: 10,
                process_exit: 0,
                activation_count: 1,
                callback_count: 1,
                callback_flags: 2,
            },
            property: "generic_libevent_cross_thread_activation".into(),
            observer_scope: "generic_libevent_only".into(),
            observer_transparency: "not_claimed".into(),
        });
        value
    }
    #[test]
    fn all_unknown_gap_matrix_is_valid() {
        assert!(check_native_platform_matrix(matrix(), &"a".repeat(64)).is_ok());
    }
    #[test]
    fn substituted_backend_refuses() {
        let mut value = matrix();
        value.rows[0].dimensions.backend = "poll".into();
        assert!(check_native_platform_matrix(value, &"a".repeat(64)).is_err());
    }
    #[test]
    fn substituted_artifact_refuses() {
        let mut value = matrix();
        value.artifact = "monkey/callback-composition".into();
        assert!(check_native_platform_matrix(value, &"a".repeat(64)).is_err());
    }
    #[test]
    fn foreign_history_does_not_promote() {
        let value = matrix();
        assert!(
            value
                .rows
                .iter()
                .all(|row| row.status == PlatformRowStatus::Unknown)
        );
    }
    #[test]
    fn primitive_thread_identities_are_policy_checked() {
        let value = observe_first(matrix());
        assert!(check_native_platform_matrix(value.clone(), &"a".repeat(64)).is_ok());
        let mut same_thread = value;
        same_thread.rows[0]
            .evidence
            .as_mut()
            .unwrap()
            .runtime
            .activating_thread = 10;
        assert!(check_native_platform_matrix(same_thread, &"a".repeat(64)).is_err());
    }
    #[test]
    fn observer_transparency_cannot_be_asserted() {
        let mut value = observe_first(matrix());
        value.rows[0]
            .evidence
            .as_mut()
            .unwrap()
            .observer_transparency = "transparent".into();
        assert!(check_native_platform_matrix(value, &"a".repeat(64)).is_err());
    }
}
