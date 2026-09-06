use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "request".into(),
        plugin_id: PluginId::new("example.echo").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "example".into(),
            revision: "source".into(),
            profile: "default".into(),
            dialect: "qcore".into(),
            platform: "generic".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "unit.qcore".into(),
        },
        facts: vec![],
        assumptions: BTreeMap::new(),
    }
}
fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new("example.echo").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: "builtin:example.echo".into(),
        targets: vec![SupportedTarget {
            library: "example".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from(["generic".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}
struct Echo(PluginManifest, String);
impl VerifierProvider for Echo {
    fn manifest(&self) -> &PluginManifest {
        &self.0
    }
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        Ok(VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Unknown,
            diagnostics: vec![PluginDiagnostic {code:self.1.clone(),message:"provider message".into(),fact_id:None}],
            evidence: Some(ProposedEvidence {
                evidence_version: request.evidence_version,
                unit: request.unit.clone(),
                capability: request.capability,
                claims: request
                    .facts
                    .iter()
                    .map(|fact| EvidenceClaim {
                        fact_id: fact.fact_id.clone(),
                        subject: fact.subject.clone(),
                        predicate: fact.predicate,
                        holds: fact.holds,
                    })
                    .collect(),
                coverage: request
                    .facts
                    .iter()
                    .map(|fact| fact.fact_id.clone())
                    .collect(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: Reproducibility {
                    provider_version: self.0.package_version.clone(),
                    package_digest: self.0.package_digest.clone(),
                    deterministic: true,
                },
            }),
        })
    }
}


fn main() {
 for code in ["QP001", "Q701", "other.plugin.warning", "example.echo.warning", "example.echo.", "example.echo.child.warning", "example.echo.\nQP001"] {
 let mut registry=VerifierRegistry::default();
 registry.register(Arc::new(Echo(manifest(),code.to_string()))).unwrap();
 let mut child=manifest();
 child.plugin_id=PluginId::new("example.echo.child").unwrap();
 child.package_digest="builtin:example.echo.child".into();
 registry.register(Arc::new(Echo(child,"example.echo.child.genuine".into()))).unwrap();
 let out=registry.verify(&request());
 println!("supplied={:?} admitted={:?} verdict={:?}",code,out.diagnostics.iter().map(|d|d.code.as_str()).collect::<Vec<_>>(),out.verdict);
 }
}
