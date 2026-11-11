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

//! Independent audit probes, not assertions that the current behavior is correct.
//! Run with `cargo run -p qcore-compiler --example reevaluation_probe`.
//! Exits 1 when any expected semantic check fails; prints JSON for archiving.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_compiler::weak_memory::{
    C11Execution, MemoryEvent, MemoryEventKind as K, MemoryOrder as O, RelationEdge as E,
    check_c11_execution,
};
use qcore_compiler::{
    BuiltInProvider, FactPredicate, NormalizedFact, PLUGIN_EVIDENCE_VERSION,
    PLUGIN_PROTOCOL_VERSION, PluginId, ProposedEvidence, ProposedVerdict, ProviderFailure,
    Reproducibility, TargetProfile, UnitIdentity, VerificationRequest, VerificationResponse,
    VerifierCapability, VerifierProvider, VerifierRegistry, first_party_manifests,
    first_party_registry,
};

fn event(id: u32, thread: u32, kind: K) -> MemoryEvent {
    MemoryEvent { id, thread, kind }
}

fn store(location: u32, value: i64, order: O) -> K {
    K::AtomicStore {
        location,
        value,
        order,
    }
}

fn load(location: u32, value: i64, order: O) -> K {
    K::AtomicLoad {
        location,
        value,
        order,
    }
}

fn memory_probe(id: &str, execution: C11Execution, expected_accept: bool) -> serde_json::Value {
    let result = check_c11_execution(&execution);
    serde_json::json!({
        "id": id, "expected_accept": expected_accept, "observed_accept": result.is_ok(),
        "pass": result.is_ok() == expected_accept, "detail": format!("{result:?}")
    })
}

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION,
        request_id: "reevaluation".into(),
        plugin_id: PluginId::new("qcore.libevent").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "libevent".into(),
            revision: "unaudited-revision".into(),
            profile: "default".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "nonexistent.c".into(),
        },
        facts: vec![NormalizedFact {
            fact_id: "recognized-only".into(),
            subject: "ctx".into(),
            predicate: FactPredicate::ApiRecognized,
            holds: true,
        }],
        assumptions: BTreeMap::new(),
    }
}

struct EmptyEvidenceProvider(BuiltInProvider);

impl VerifierProvider for EmptyEvidenceProvider {
    fn manifest(&self) -> &qcore_compiler::PluginManifest {
        self.0.manifest()
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        Ok(VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Safe,
            diagnostics: vec![],
            evidence: Some(ProposedEvidence {
                evidence_version: request.evidence_version,
                unit: request.unit.clone(),
                capability: request.capability,
                claims: vec![],
                coverage: BTreeSet::new(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: Reproducibility {
                    provider_version: self.manifest().package_version.clone(),
                    package_digest: self.manifest().package_digest.clone(),
                    deterministic: true,
                },
            }),
        })
    }
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example reevaluation_probe");
    let mut results = vec![];
    for (id, order, expected) in [
        ("WM01_acq_rel_plain_operations", O::AcquireRelease, false),
        ("CONTROL_relaxed_plain_operations", O::Relaxed, true),
    ] {
        results.push(memory_probe(
            id,
            C11Execution {
                events: vec![
                    event(
                        0,
                        0,
                        K::AtomicInit {
                            location: 0,
                            value: 0,
                        },
                    ),
                    event(1, 0, store(0, 1, order)),
                    event(2, 0, load(0, 1, order)),
                ],
                modification_order: vec![E::new(0, 1)],
                reads_from: vec![E::new(1, 2)],
                synchronizes_with: if order == O::AcquireRelease {
                    vec![E::new(1, 2)]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            expected,
        ));
    }
    results.push(memory_probe(
        "WM02_reversed_write_coherence",
        C11Execution {
            events: vec![
                event(
                    0,
                    0,
                    K::AtomicInit {
                        location: 0,
                        value: 0,
                    },
                ),
                event(1, 0, store(0, 1, O::Relaxed)),
                event(2, 0, store(0, 2, O::Relaxed)),
            ],
            modification_order: vec![E::new(0, 2), E::new(2, 1)],
            ..Default::default()
        },
        false,
    ));
    results.push(memory_probe(
        "WM03_sc_store_buffering_zero_zero",
        C11Execution {
            events: vec![
                event(
                    0,
                    0,
                    K::AtomicInit {
                        location: 0,
                        value: 0,
                    },
                ),
                event(
                    1,
                    0,
                    K::AtomicInit {
                        location: 1,
                        value: 0,
                    },
                ),
                event(2, 0, K::Spawn { child: 1 }),
                event(3, 0, K::Spawn { child: 2 }),
                event(4, 1, store(0, 1, O::SequentiallyConsistent)),
                event(5, 1, load(1, 0, O::SequentiallyConsistent)),
                event(6, 2, store(1, 1, O::SequentiallyConsistent)),
                event(7, 2, load(0, 0, O::SequentiallyConsistent)),
            ],
            modification_order: vec![E::new(0, 4), E::new(1, 6)],
            reads_from: vec![E::new(1, 5), E::new(0, 7)],
            sequentially_consistent: vec![4, 5, 6, 7],
            ..Default::default()
        },
        false,
    ));
    results.push(memory_probe(
        "WM04_c11_release_sequence",
        C11Execution {
            events: vec![
                event(
                    0,
                    0,
                    K::AtomicInit {
                        location: 0,
                        value: 0,
                    },
                ),
                event(1, 0, K::Spawn { child: 1 }),
                event(2, 0, K::Spawn { child: 2 }),
                event(3, 1, K::NonAtomicWrite { location: 1 }),
                event(4, 1, store(0, 1, O::Release)),
                event(5, 1, store(0, 2, O::Relaxed)),
                event(6, 2, load(0, 2, O::Acquire)),
                event(7, 2, K::NonAtomicRead { location: 1 }),
            ],
            modification_order: vec![E::new(0, 4), E::new(4, 5)],
            reads_from: vec![E::new(5, 6)],
            synchronizes_with: vec![E::new(4, 6)],
            ..Default::default()
        },
        true,
    ));
    let registry = first_party_registry();
    let unrelated = registry.verify(&request());
    results.push(
        serde_json::json!({"id": "VP01_recognition_is_not_resource_proof",
        "expected": "Unknown", "observed": format!("{:?}", unrelated.verdict),
        "certificate": unrelated.certificate.is_some(),
        "pass": unrelated.verdict == ProposedVerdict::Unknown}),
    );
    let mut empty_request = request();
    empty_request.facts.clear();
    let control = registry.verify(&empty_request);
    results.push(serde_json::json!({"id": "CONTROL_builtin_empty_is_unknown",
        "pass": control.verdict == ProposedVerdict::Unknown,
        "observed": format!("{:?}", control.verdict)}));
    let manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .unwrap();
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(EmptyEvidenceProvider(
            BuiltInProvider::new(manifest).unwrap(),
        )))
        .unwrap();
    let empty = registry.verify(&empty_request);
    results.push(serde_json::json!({"id": "VP02_empty_evidence_firewall",
        "expected": "Unknown", "observed": format!("{:?}", empty.verdict),
        "certificate": empty.certificate.is_some(),
        "pass": empty.verdict == ProposedVerdict::Unknown}));
    let failures = results.iter().filter(|r| r["pass"] == false).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1, "failed_expectations": failures, "probes": results
        }))
        .unwrap()
    );
    if failures != 0 {
        std::process::exit(1);
    }
}
