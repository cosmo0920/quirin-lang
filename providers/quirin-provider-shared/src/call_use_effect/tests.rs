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

//! Provider-side reading and refusal matrix for the call-result def/use
//! family, driven over the real projections the gateway emits.
//!
//! A conformance test asserts these fixtures are still the live projections,
//! so the matrix cannot drift into a shape the importer no longer produces.
use super::*;
use qcore_schemas::call_use_summary::{CallUsePathRecord, CallUseSourceInput};

const CLOSE: &str = include_str!("../../fixtures/file_close_call_use_projection.json");
const NATIVE: &str = include_str!("../../fixtures/native_close_call_use_projection.json");

fn input(text: &str) -> CallUseSourceInput {
    serde_json::from_str(text).unwrap()
}

fn body(input: &mut CallUseSourceInput) -> &mut CallUseFunctionRecord {
    match &mut input.family {
        CallUseFamilyInput::FilesystemAttempt { body } => body,
    }
}

fn reading(input: &CallUseSourceInput) -> CallResultUseRecord {
    match derive(input).unwrap() {
        CallUseProposalRecord::FilesystemAttempt(record) => record,
    }
}

#[test]
fn the_wrapper_reading_reports_four_calls_whose_results_are_all_ignored() {
    let record = reading(&input(CLOSE));
    assert_eq!(record.function, "cio_file_close");
    assert_eq!(record.signature, "void (struct cio_chunk *, int)");
    // The early-return path has no call at all; the full path has four.
    assert_eq!(record.paths.len(), 2);
    assert!(record.paths[0].calls.is_empty());
    let calls = &record.paths[1].calls;
    assert_eq!(
        calls
            .iter()
            .map(|call| call.function.as_str())
            .collect::<Vec<_>>(),
        ["munmap_file", "cio_file_native_close", "free", "free"]
    );
    // Operation indices are strictly increasing, which is what the receiver's
    // ordering fact reads.
    assert!(calls.windows(2).all(|w| w[0].operation < w[1].operation));
    assert!(
        calls
            .iter()
            .all(|call| call.uses.is_empty() && call.guard_uses.is_empty()),
        "every wrapper result is ignored"
    );
}

#[test]
fn the_native_reading_reports_a_stored_and_guarded_result_on_both_paths() {
    let record = reading(&input(NATIVE));
    assert_eq!(record.function, "cio_file_native_close");
    assert_eq!(record.paths.len(), 2);
    for path in &record.paths {
        let [call] = path.calls.as_slice() else {
            panic!("one call per path")
        };
        assert_eq!(call.function, "close");
        assert_eq!(call.result_type, "int");
        // Stored into the local, then guarded on.
        assert_eq!(call.uses, vec![2, 3]);
        assert_eq!(call.guard_uses, vec![3]);
    }
}

/// Precision, not silence: a body that stops guarding its result stays
/// readable and the reading *changes*, so the receiver's own comparison and
/// contract evaluation see the change rather than a normalized result.
#[test]
fn dropping_a_guard_changes_the_reading_instead_of_refusing_it() {
    let honest = reading(&input(NATIVE));
    let mut changed = input(NATIVE);
    for path in &mut body(&mut changed).paths {
        path.operations
            .retain(|operation| !matches!(operation, CallUseOperationRecord::Guard { .. }));
    }
    let moved = reading(&changed);
    assert_ne!(moved, honest);
    assert!(
        moved
            .paths
            .iter()
            .all(|path| path.calls.iter().all(|call| call.guard_uses.is_empty())),
        "the guarded use must disappear from the reading"
    );
}

#[test]
fn every_degenerate_or_oversized_projection_is_declined_without_panicking() {
    type Mutate = Box<dyn Fn(&mut CallUseSourceInput)>;
    let rows: Vec<(&str, Mutate)> = vec![
        (
            "wrong schema version",
            Box::new(|input: &mut CallUseSourceInput| input.schema_version = 2),
        ),
        (
            "schema version zero",
            Box::new(|input: &mut CallUseSourceInput| input.schema_version = 0),
        ),
        (
            "empty function name",
            Box::new(|input: &mut CallUseSourceInput| body(input).function = String::new()),
        ),
        (
            "empty revision",
            Box::new(|input: &mut CallUseSourceInput| body(input).source_revision = String::new()),
        ),
        (
            "empty signature",
            Box::new(|input: &mut CallUseSourceInput| body(input).signature = String::new()),
        ),
        (
            "no paths",
            Box::new(|input: &mut CallUseSourceInput| body(input).paths.clear()),
        ),
        (
            "an empty path",
            Box::new(|input: &mut CallUseSourceInput| {
                body(input).paths.push(CallUsePathRecord {
                    operations: Vec::new(),
                })
            }),
        ),
        (
            "more paths than the cap",
            Box::new(|input: &mut CallUseSourceInput| {
                let path = body(input).paths[0].clone();
                for _ in 0..MAX_PATHS {
                    body(input).paths.push(path.clone());
                }
            }),
        ),
        (
            "a path longer than the cap",
            Box::new(|input: &mut CallUseSourceInput| {
                let operation = body(input).paths[0].operations[0].clone();
                for _ in 0..=MAX_OPERATIONS {
                    body(input).paths[0].operations.push(operation.clone());
                }
            }),
        ),
        (
            "an empty callee name",
            Box::new(|input: &mut CallUseSourceInput| {
                for path in &mut body(input).paths {
                    for operation in &mut path.operations {
                        if let CallUseOperationRecord::Call { function, .. } = operation {
                            *function = String::new();
                        }
                    }
                }
            }),
        ),
        (
            "an expression deeper than the cap",
            Box::new(|input: &mut CallUseSourceInput| {
                let deep = deep_value(MAX_EXPRESSION_DEPTH + 4);
                body(input).paths[0]
                    .operations
                    .push(CallUseOperationRecord::ReturnValue { value: deep });
            }),
        ),
    ];
    for (name, mutate) in rows {
        for text in [CLOSE, NATIVE] {
            let mut moved = input(text);
            mutate(&mut moved);
            assert!(
                derive(&moved).is_none(),
                "admitted a bad projection: {name}"
            );
        }
    }
}

/// A left-nested unary chain of the requested depth.
fn deep_value(depth: u32) -> CallUseValueRecord {
    let mut value = CallUseValueRecord {
        c_type: "int".into(),
        expression: CallUseExpressionRecord::Integer { value: "0".into() },
    };
    for _ in 0..depth {
        value = CallUseValueRecord {
            c_type: "int".into(),
            expression: CallUseExpressionRecord::Unary {
                operator: "-".into(),
                value: Box::new(value),
            },
        };
    }
    value
}

/// A projection is data, not authority: the reading is a pure function of the
/// supplied paths, so two producers reading the same bytes agree, and reading
/// one family's projection never answers for the other body.
#[test]
fn the_reading_is_a_pure_function_of_the_supplied_paths() {
    assert_eq!(reading(&input(CLOSE)), reading(&input(CLOSE)));
    assert_ne!(reading(&input(CLOSE)), reading(&input(NATIVE)));
}

#[test]
fn the_request_gate_declines_every_other_profile_capability_and_dialect() {
    let manifest = manifest();
    let base = request(&input(NATIVE));
    assert!(propose(&base, &manifest).is_some());
    for mutate in [
        |request: &mut VerificationRequestV2| {
            request.target.profile = "qcore.c_counter_contract.v1".into()
        },
        |request: &mut VerificationRequestV2| request.target.dialect = "c99".into(),
        |request: &mut VerificationRequestV2| {
            request.capability = VerifierCapability::ChunkLifecycle
        },
        |request: &mut VerificationRequestV2| request.call_use_source = None,
        |request: &mut VerificationRequestV2| {
            request.call_use_source.as_mut().unwrap().schema_version = 9
        },
    ] {
        let mut moved = base.clone();
        mutate(&mut moved);
        assert!(propose(&moved, &manifest).is_none());
    }
}

fn manifest() -> PluginManifest {
    let mut manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    manifest.package_digest = "0".repeat(64);
    manifest
}

fn request(input: &CallUseSourceInput) -> VerificationRequestV2 {
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
        profile: "qcore.c_call_result_use.v1".into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "call-result-use:test".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target,
        unit: UnitIdentity {
            translation_unit: "cio_file_unix.c".into(),
            source_digest: "0".repeat(64),
            build_digest: "1".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "2".repeat(64),
        obligations: Vec::new(),
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: Some(input.clone()),
        cleanup_replay_source: None,
        composition_source: None,
    }
}
