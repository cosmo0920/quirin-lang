//! A schemas-only proposal producer. The receiver rechecks source paths and
//! evaluates its own pinned contract; this process has no trusted checker.
use qcore_schemas::SemanticWitness;
use qcore_schemas::chunk_summary::BooleanFieldContractWitness;
use qcore_schemas::plugin::*;

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile != "qcore.c_boolean_field_contract.v1"
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let source = request.source_effect.as_ref()?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
                task_digest: request.task_digest.clone(),
                summary: source.summary.clone(),
            })),
        }),
    })
}
