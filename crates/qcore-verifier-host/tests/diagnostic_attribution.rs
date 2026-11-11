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

//! Host-owned attribution rules, independently exercised for both evidence versions.
//! These tests do not install a permissive witness checker or admit model claims.
use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const PARENT: &str = "example.echo";
const CHILD: &str = "example.echo.child";

fn target() -> TargetProfile {
    TargetProfile {
        library: "example".into(),
        revision: "source".into(),
        profile: "default".into(),
        dialect: "qcore".into(),
        platform: "generic".into(),
        backend: "native".into(),
    }
}
fn manifest(id: &str) -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(id).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1, 2]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{id}"),
        targets: vec![SupportedTarget {
            library: "example".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from(["generic".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ChunkLifecycle]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_fixture".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}
fn task() -> VerificationTask {
    VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: UnitIdentity {
            translation_unit: "fixture.qcore".into(),
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
        },
        target: target(),
        semantic_profile: "attribution-envelope-test".into(),
        contract_sources: BTreeMap::new(),
        bounds: TaskBounds::single_path(),
        assumptions: BTreeMap::new(),
        evidence_version: 2,
        obligations: vec![Obligation {
            obligation_id: "trusted-inventory".into(),
            capability: VerifierCapability::ChunkLifecycle,
            property: ObligationProperty::ChunkDischarged,
            subject: ObligationSubject::Unit,
            scope: ObligationScope::WholeUnit,
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: 2,
        }],
        derivation_incomplete: vec![],
    }
}
struct DiagnosticProvider {
    manifest: PluginManifest,
    code: String,
    verdict: ProposedVerdict,
}
impl DiagnosticProvider {
    fn diagnostics(&self) -> Vec<PluginDiagnostic> {
        vec![PluginDiagnostic {
            code: self.code.clone(),
            message: "provider-owned message".into(),
            fact_id: None,
        }]
    }
    fn reproducibility(&self) -> Reproducibility {
        Reproducibility {
            provider_version: self.manifest.package_version.clone(),
            package_digest: self.manifest.package_digest.clone(),
            deterministic: true,
        }
    }
}
impl VerifierProvider for DiagnosticProvider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        Ok(VerificationResponse {
            protocol_version: 1,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: self.verdict,
            diagnostics: self.diagnostics(),
            evidence: Some(ProposedEvidence {
                evidence_version: 1,
                unit: request.unit.clone(),
                capability: request.capability,
                claims: vec![],
                coverage: BTreeSet::new(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: self.reproducibility(),
            }),
        })
    }
    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        Ok(VerificationResponseV2 {
            protocol_version: 1,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: self.verdict,
            diagnostics: self.diagnostics(),
            evidence: Some(ProposedEvidenceV2 {
                evidence_version: 2,
                unit: request.unit.clone(),
                capability: request.capability,
                coverage: request.coverage(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: self.reproducibility(),
                witness: SemanticWitness::ChunkLifecycle(ChunkLifecycleWitness {
                    task_digest: request.task_digest.clone(),
                    schedule: vec![],
                }),
            }),
        })
    }
}
fn outcome(id: &str, code: &str, version: u16, verdict: ProposedVerdict) -> VerificationOutcome {
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(DiagnosticProvider {
            manifest: manifest(id),
            code: code.into(),
            verdict,
        }))
        .unwrap();
    // A separately registered nested provider makes the ownership ambiguity
    // concrete: the parent must not emit the child's complete diagnostic code.
    if id == PARENT {
        registry
            .register(Arc::new(DiagnosticProvider {
                manifest: manifest(CHILD),
                code: format!("{CHILD}.genuine"),
                verdict: ProposedVerdict::Unknown,
            }))
            .unwrap();
    }
    let task = task();
    let request = VerificationRequest {
        protocol_version: 1,
        evidence_version: version,
        request_id: "request".into(),
        plugin_id: PluginId::new(id).unwrap(),
        capability: VerifierCapability::ChunkLifecycle,
        target: task.target.clone(),
        unit: task.unit.clone(),
        facts: vec![],
        assumptions: task.assumptions.clone(),
    };
    if version == 1 {
        return registry.verify(&request);
    }
    registry.verify_v2(
        &VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: request.request_id,
            plugin_id: request.plugin_id,
            capability: request.capability,
            target: request.target,
            unit: request.unit,
            facts: request.facts,
            assumptions: request.assumptions,
            task_digest: task.task_digest(),
            obligations: task.obligations.clone(),
            source_effect: None,
            counter_effect: None,
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        },
        &task,
        None,
    )
}
fn invalid_codes() -> Vec<String> {
    [
        "QP001",
        "Q701",
        "other.plugin.warning",
        "example.echo",
        "example.echo.",
        "example.echo.child.warning",
        "example.echo.warning.extra",
        "example.echo..warning",
        "example.echo.\nQP001",
        "example.echo.warn\r",
        "example.echo.warn\t",
        "example.echo.warn\0",
        "example.echo.warn\u{7f}",
        "example.echo.é",
        "example.echo. warning",
        "example.echo.warn/extra",
        "example.echo2.warning",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([format!("{PARENT}.{}", "a".repeat(129))])
    .collect()
}
fn rejects_invalid(version: u16) {
    for code in invalid_codes() {
        for verdict in [
            ProposedVerdict::Unknown,
            ProposedVerdict::Unsafe,
            ProposedVerdict::Safe,
        ] {
            let result = outcome(PARENT, &code, version, verdict);
            assert_eq!(result.verdict, ProposedVerdict::Unknown, "{code:?}");
            assert!(result.certificate.is_none());
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(
                result.diagnostics[0].code, "QP007",
                "v{version} {code:?} {verdict:?}"
            );
        }
    }
}
#[test]
fn invalid_codes_are_rejected_before_v1_verdict_handling() {
    rejects_invalid(1);
}
#[test]
fn invalid_codes_are_rejected_before_v2_verdict_handling() {
    rejects_invalid(2);
}
#[test]
fn exact_selected_provider_and_ascii_local_codes_are_preserved() {
    for version in [1, 2] {
        for id in [PARENT, CHILD] {
            for local in ["L00", "warning", "mixed_CASE-123", "_", "-"] {
                let code = format!("{id}.{local}");
                let result = outcome(id, &code, version, ProposedVerdict::Unknown);
                assert_eq!(result.diagnostics[0].code, code);
                assert_eq!(result.diagnostics[0].message, "provider-owned message");
                assert_eq!(result.verdict, ProposedVerdict::Unknown);
                assert!(result.certificate.is_none());
            }
        }
    }
}
#[test]
fn maximum_valid_plugin_and_local_component_lengths_are_accepted() {
    let id = format!("{}.{}.{}", "a".repeat(48), "b".repeat(48), "c".repeat(30));
    assert_eq!(id.len(), 128);
    let code = format!("{id}.{}", "X".repeat(128));
    assert_eq!(code.len(), 257);
    for version in [1, 2] {
        assert_eq!(
            outcome(&id, &code, version, ProposedVerdict::Unknown).diagnostics[0].code,
            code
        );
    }
}
#[test]
fn a_valid_diagnostic_never_supplies_missing_evidence_authority() {
    let code = format!("{PARENT}.L00");
    let v1 = outcome(PARENT, &code, 1, ProposedVerdict::Safe);
    assert_eq!(v1.verdict, ProposedVerdict::Unknown);
    assert_eq!(v1.diagnostics[0].code, "QP009");
    assert!(v1.certificate.is_none());
    let v2 = outcome(PARENT, &code, 2, ProposedVerdict::Safe);
    assert_eq!(v2.verdict, ProposedVerdict::Unknown);
    assert_eq!(v2.diagnostics[0].code, "QP016");
    assert!(v2.certificate.is_none());
}
