//! Untrusted, standalone evidence-v2 ABI conformance provider.
//! Depends only on the published schemas and JSON; it never imports core replay.
use qcore_schemas::plugin::{
    ProposedEvidenceV2, ProposedVerdict, Reproducibility, VerificationRequestV2,
    VerificationResponseV2,
};
use qcore_schemas::witness::{
    ContractObservationRecord, ContractObservationWitness, NativeRunBinding, ObservedRun,
    SemanticWitness,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prefix = [0u8; 4];
    std::io::stdin().read_exact(&mut prefix)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length > 1024 * 1024 {
        return Err("request frame too large".into());
    }
    let mut bytes = vec![0; length];
    std::io::stdin().read_exact(&mut bytes)?;
    let request: VerificationRequestV2 = serde_json::from_slice(&bytes)?;
    request.validate()?;
    // The regression harness pins the complete host-generated request bytes.
    let expected = std::fs::read("request.bin")?;
    if expected != [prefix.as_slice(), bytes.as_slice()].concat() {
        return Err("unexpected request".into());
    }
    std::fs::write("invoked", b"invoked")?;
    let forged =
        std::fs::read_to_string("response-mode").is_ok_and(|mode| mode == "bad_postcondition");
    let response = VerificationResponseV2 {
        protocol_version: request.protocol_version,
        request_id: request.request_id,
        plugin_id: request.plugin_id,
        verdict: ProposedVerdict::Safe,
        diagnostics: vec![],
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit,
            capability: request.capability,
            coverage: request
                .obligations
                .iter()
                .map(|o| o.obligation_id.clone())
                .collect(),
            assumptions_used: request.assumptions,
            reproducibility: Reproducibility {
                provider_version: "0.1.0".into(),
                package_digest: "c".repeat(64),
                deterministic: true,
            },
            witness: SemanticWitness::ContractObservation(ContractObservationWitness {
                task_digest: request.task_digest,
                runs: vec![ObservedRun {
                    run_id: "fixture-run".into(),
                    binding: NativeRunBinding {
                        emitted_source_digest: "a".repeat(64),
                        executable_digest: "b".repeat(64),
                        compiler_invocation: "cc fixture.c".into(),
                        exit_status: 0,
                        seed_or_input: "size=4".into(),
                        profile: request.target.profile,
                        instrumentation_version: "fixture-v1".into(),
                    },
                    observations: vec![ContractObservationRecord {
                        function: "resize".into(),
                        hook: "resize".into(),
                        values: BTreeMap::from([
                            ("size".into(), 4),
                            ("live".into(), if forged { 0 } else { 1 }),
                        ]),
                    }],
                }],
            }),
        }),
    };
    let encoded = serde_json::to_vec(&response)?;
    std::io::stdout().write_all(&(encoded.len() as u32).to_be_bytes())?;
    std::io::stdout().write_all(&encoded)?;
    Ok(())
}
