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

//! Pins the task-digest canonicalization rule of
//! `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` open question 6 (accepted):
//! every type that enters a `VerificationTask` digest or a v2 witness uses
//! `BTreeMap`/`BTreeSet`/`Vec`, carries no floating-point field, and never
//! uses `HashMap`/`HashSet`. Under that rule `serde_json` emits a
//! deterministic byte string, which is what the SHA-256 is taken over. A full
//! canonical-JSON specification is not needed while both digesting ends are
//! the core, but the rule must not erode silently.

use std::collections::BTreeMap;

use qcore_schemas::obligation::{
    ChunkEntityRef, DerivationIncomplete, Obligation, ObligationProperty, ObligationScope,
    ObligationSubject, TaskBounds, VERIFICATION_TASK_VERSION, VerificationTask,
};
use qcore_schemas::plugin::{TargetProfile, UnitIdentity, VerifierCapability};

const FORBIDDEN: &[&str] = &["HashMap", "HashSet", "f32", "f64"];

fn module_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("obligation.rs", include_str!("../src/obligation.rs")),
        ("witness.rs", include_str!("../src/witness.rs")),
    ]
}

#[test]
fn task_and_witness_modules_use_ordered_containers_only() {
    for (name, source) in module_sources() {
        for forbidden in FORBIDDEN {
            assert!(
                !source.contains(forbidden),
                "{name} mentions `{forbidden}`; the task-digest canonicalization rule \
                 admits BTreeMap/BTreeSet/Vec and no floating-point fields"
            );
        }
    }
}

#[test]
fn canonical_serialization_is_byte_stable_across_insertion_orders() {
    let build = |reversed: bool| {
        let mut contract_sources = BTreeMap::new();
        let mut assumptions = BTreeMap::new();
        let entries = [
            ("contract/qcore/a.qcore", "a".repeat(64)),
            ("contract/qcore/b.qcore", "b".repeat(64)),
            ("contract/qcore/c.qcore", "c".repeat(64)),
        ];
        let flags = [("observations_trusted", true), ("callbacks_bounded", false)];
        if reversed {
            for (path, digest) in entries.iter().rev() {
                contract_sources.insert((*path).to_owned(), digest.clone());
            }
            for (name, value) in flags.iter().rev() {
                assumptions.insert((*name).to_owned(), *value);
            }
        } else {
            for (path, digest) in &entries {
                contract_sources.insert((*path).to_owned(), digest.clone());
            }
            for (name, value) in &flags {
                assumptions.insert((*name).to_owned(), *value);
            }
        }
        VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                source_digest: "1".repeat(64),
                build_digest: "2".repeat(64),
                translation_unit: "fixture.qc".into(),
            },
            target: TargetProfile {
                library: "chunkio".into(),
                revision: "audit".into(),
                profile: "default".into(),
                dialect: "qcore".into(),
                platform: "linux".into(),
                backend: "native".into(),
            },
            semantic_profile: "qcore.chunk_lifecycle.v1".into(),
            contract_sources,
            bounds: TaskBounds::single_path(),
            assumptions,
            evidence_version: 2,
            obligations: vec![Obligation {
                obligation_id: "chunk_lifecycle:0123456789abcdef01234567".into(),
                capability: VerifierCapability::ChunkLifecycle,
                property: ObligationProperty::ChunkDischarged,
                subject: ObligationSubject::ChunkEntity {
                    entity: ChunkEntityRef::Chunk { id: 0 },
                },
                scope: ObligationScope::WholeUnit,
                bounds: TaskBounds::single_path(),
                assumptions: BTreeMap::new(),
                evidence_version: 2,
            }],
            derivation_incomplete: vec![DerivationIncomplete {
                capability: VerifierCapability::EventTraces,
                category: qcore_schemas::ReasonCategory::IncompleteObligationCoverage,
                reason: "no derivation in this increment".into(),
            }],
        }
    };
    let forward = serde_json::to_vec(&build(false)).unwrap();
    let reversed = serde_json::to_vec(&build(true)).unwrap();
    assert_eq!(forward, reversed, "map insertion order must not be visible");
    assert_eq!(build(false).task_digest(), build(true).task_digest());
    assert_eq!(build(false).task_digest().len(), 64);
}

#[test]
fn observation_values_preserve_full_i128_range_in_tagged_witnesses() {
    use qcore_schemas::witness::{
        ContractObservationRecord, ContractObservationWitness, NativeRunBinding, ObservedRun,
        SemanticWitness,
    };
    let witness = SemanticWitness::ContractObservation(ContractObservationWitness {
        task_digest: "a".repeat(64),
        runs: vec![ObservedRun {
            run_id: "run".into(),
            binding: NativeRunBinding {
                emitted_source_digest: "a".repeat(64),
                compiler_invocation: "cc".into(),
                executable_digest: "b".repeat(64),
                exit_status: 0,
                seed_or_input: "0".into(),
                profile: "default".into(),
                instrumentation_version: "1".into(),
            },
            observations: vec![ContractObservationRecord {
                function: "hook".into(),
                hook: "hook".into(),
                values: BTreeMap::from([
                    ("minimum".into(), i128::MIN),
                    ("maximum".into(), i128::MAX),
                    ("zero".into(), 0),
                ]),
            }],
        }],
    });
    let wire = serde_json::to_string(&witness).unwrap();
    assert!(wire.contains(&format!("\"{}\"", i128::MAX)));
    assert_eq!(
        serde_json::from_str::<SemanticWitness>(&wire).unwrap(),
        witness
    );
    for invalid in [
        "1.5",
        "0",
        "\"170141183460469231731687303715884105728\"",
        "\"-170141183460469231731687303715884105729\"",
        "\"00\"",
        "\"+1\"",
        "\"-0\"",
    ] {
        let wire = format!(r#"{{"function":"hook","hook":"hook","values":{{"x":{invalid}}}}}"#);
        assert!(
            serde_json::from_str::<ContractObservationRecord>(&wire).is_err(),
            "must reject {invalid}"
        );
    }
}
