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

//! Provider-side replay and refusal matrix for the two cleanup-outcome
//! families, driven over the real projections the gateway emits.
//!
//! A conformance test asserts these fixtures are still the live projections,
//! so the matrix cannot drift into a shape the importer no longer produces.
//!
//! Every refusal row here is a *decline*: the process returns no proposal, the
//! host sees no witness, and the obligation stays Unknown. Nothing in this file
//! can make a verdict more permissive.
use super::*;
use qcore_schemas::cleanup_replay_summary::CleanupReplayPathRecord;

const CLOSE: &str = include_str!("../../fixtures/native_close_cleanup_replay_projection.json");
const UNMAP: &str = include_str!("../../fixtures/native_unmap_cleanup_replay_projection.json");
const DELETE: &str = include_str!("../../fixtures/native_delete_cleanup_replay_projection.json");

fn input(text: &str) -> CleanupReplaySourceInput {
    serde_json::from_str(text).unwrap()
}

fn parts(
    input: &mut CleanupReplaySourceInput,
) -> (&mut CleanupReplayFunctionRecord, &mut CleanupProfileRecord) {
    match &mut input.family {
        CleanupReplayFamilyInput::NativeCleanupOutcome { body, profile }
        | CleanupReplayFamilyInput::GuardedDelete { body, profile, .. } => (body, profile),
    }
}

fn reading(input: &CleanupReplaySourceInput) -> CleanupReplayRecord {
    match derive(input).unwrap() {
        CleanupReplayProposalRecord::NativeCleanupOutcome(record)
        | CleanupReplayProposalRecord::GuardedDelete(record) => record,
    }
}

fn refused(input: &CleanupReplaySourceInput) -> bool {
    derive(input).is_none()
}

#[test]
fn the_native_close_replay_covers_four_prestates_and_resets_one_field() {
    let record = reading(&input(CLOSE));
    assert_eq!(record.function, "cio_file_native_close");
    assert_eq!(record.signature, "int (struct cio_file *)");
    assert_eq!(record.cases.len(), 4);
    assert_eq!(
        record
            .cases
            .iter()
            .map(CleanupReplayCaseRecord::prestate)
            .collect::<Vec<_>>(),
        CleanupPrestateRecord::native_cleanup_outcome_domain()
    );
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        field_writes,
        fields_reset,
        cleanup_calls,
        returned,
        source_path,
        initial_fields,
        ..
    } = &record.cases[3]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(field_writes, &["fd"]);
    assert!(fields_reset);
    assert_eq!((*cleanup_calls, *returned), (1, 0));
    // Each prestate selects exactly one path, and the four select distinct
    // ones (the checker requires the whole path set to be covered).
    let paths: BTreeSet<u32> = record
        .cases
        .iter()
        .map(|case| match case {
            CleanupReplayCaseRecord::NativeCleanupOutcome { source_path, .. }
            | CleanupReplayCaseRecord::GuardedDelete { source_path, .. } => *source_path,
        })
        .collect();
    assert_eq!(paths.len(), 4);
    assert!(paths.contains(source_path));
    // The lattice distinguishes a live descriptor's opaque scalar identity
    // from the reset integer the `Absent` prestate starts at.
    assert_eq!(
        initial_fields["fd"],
        CleanupLatticeValue::Symbol { field: "fd".into() }
    );
    let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } = &record.cases[1]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(
        initial_fields["fd"],
        CleanupLatticeValue::Integer { value: -1 }
    );
}

#[test]
fn the_native_unmap_replay_resets_all_three_declared_fields_in_order() {
    let record = reading(&input(UNMAP));
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        field_writes,
        fields_reset,
        fields_preserved,
        ..
    } = &record.cases[3]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(field_writes, &["alloc_size", "map", "map_truncated_warned"]);
    assert!(*fields_reset && !*fields_preserved);
    // The failure arm writes nothing and preserves everything.
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        field_writes,
        fields_preserved,
        ..
    } = &record.cases[2]
    else {
        panic!("wrong case variant")
    };
    assert!(field_writes.is_empty() && *fields_preserved);
}

#[test]
fn the_guarded_delete_replay_covers_five_prestates_and_preserves_every_field() {
    let record = reading(&input(DELETE));
    assert_eq!(record.cases.len(), 5);
    assert_eq!(
        record
            .cases
            .iter()
            .map(CleanupReplayCaseRecord::prestate)
            .collect::<Vec<_>>(),
        CleanupPrestateRecord::guarded_delete_domain()
    );
    for case in &record.cases {
        let CleanupReplayCaseRecord::GuardedDelete {
            descriptor_field_active,
            mapping_field_active,
            syscall_calls,
            diagnostic_calls,
            syscall_result,
            fields_preserved,
            ..
        } = case
        else {
            panic!("wrong case variant")
        };
        assert!(fields_preserved);
        assert_eq!(
            *syscall_calls,
            u32::from(!descriptor_field_active && !mapping_field_active)
        );
        assert_eq!(*diagnostic_calls, u32::from(*syscall_result == Some(-1)));
    }
}

/// Precision, not silence: a body that stops resetting a field still replays
/// — the *reading changes*, so the receiver's own comparison and contract
/// evaluation see the change rather than a normalized result.
///
/// This is the one behaviour that distinguishes a replay from a fold, so it is
/// asserted rather than assumed: the checker's structural gate refuses the
/// changed body, which is why this row measures the refusal instead.
#[test]
fn dropping_a_field_reset_is_refused_rather_than_normalised() {
    let mut changed = input(UNMAP);
    {
        let (body, _) = parts(&mut changed);
        for path in &mut body.paths {
            let mut writes = 0;
            path.operations.retain(|operation| {
                if matches!(operation, CleanupReplayOperationRecord::Write { .. }) {
                    writes += 1;
                    writes > 1
                } else {
                    true
                }
            });
        }
    }
    assert!(refused(&changed));
}

/// The transported profile is an input the replay is defined over, so
/// falsifying it changes what is computed — and every falsification the
/// checker's structural gate rejects is declined here too.
#[test]
fn a_falsified_profile_never_yields_a_reading_the_core_would_accept() {
    // 1. A reset value the body does not write.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.fields[0].reset = CleanupResetRecord::Integer { value: 0 };
    assert!(refused(&changed));
    // 2. A field type the body does not use.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.fields[0].c_type = "char *".into();
    assert!(refused(&changed));
    // 3. A renamed field, so no access binds.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.fields[0].name = "descriptor".into();
    parts(&mut changed).1.presence_field = "descriptor".into();
    parts(&mut changed).1.argument_fields = vec!["descriptor".into()];
    assert!(refused(&changed));
    // 4. A syscall the body never calls.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.syscall = "closesocket".into();
    assert!(refused(&changed));
    // 5. The syscall at the wrong declared signature.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.syscall_signature = "int (long)".into();
    assert!(refused(&changed));
    // 6. Reordered syscall arguments.
    let mut changed = input(UNMAP);
    parts(&mut changed).1.argument_fields = vec!["alloc_size".into(), "map".into()];
    assert!(refused(&changed));
    // 7. An eligibility field that is not the one the body guards on.
    let mut changed = input(UNMAP);
    parts(&mut changed).1.presence_field = "map_truncated_warned".into();
    assert!(refused(&changed));
    // 8. An errno helper the body never calls.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.errno_function = "errno_location".into();
    assert!(refused(&changed));
    // 9. A diagnostic helper the body never calls.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.report_function = "cio_print".into();
    assert!(refused(&changed));
    // 10. A record pointer type the body does not use.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.root_type = "struct cio_chunk *".into();
    assert!(refused(&changed));
    // 11. A widened inventory the successful arm does not reset.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.fields.push(CleanupFieldRecord {
        name: "unrelated".into(),
        c_type: "int".into(),
        reset: CleanupResetRecord::Integer { value: 0 },
    });
    assert!(refused(&changed));
    // 12. A narrowed inventory the successful arm resets more than.
    let mut changed = input(UNMAP);
    parts(&mut changed).1.fields.truncate(2);
    assert!(refused(&changed));
    // 13. A duplicated field name.
    let mut changed = input(CLOSE);
    let duplicate = parts(&mut changed).1.fields[0].clone();
    parts(&mut changed).1.fields.push(duplicate);
    assert!(refused(&changed));
    // 14. An empty inventory, and an empty argument list.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.fields.clear();
    assert!(refused(&changed));
    let mut changed = input(CLOSE);
    parts(&mut changed).1.argument_fields.clear();
    assert!(refused(&changed));
    // 15. An argument naming a field outside the inventory.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.argument_fields = vec!["ghost".into()];
    assert!(refused(&changed));
    // 16. A function name the body does not have.
    let mut changed = input(CLOSE);
    parts(&mut changed).1.function = "cio_file_native_unmap".into();
    assert!(refused(&changed));
}

/// The delete family's extra shape rules, which are what keep the second
/// eligibility field from being read as an ordinary member.
#[test]
fn the_guarded_delete_shape_rules_are_each_load_bearing() {
    let mapping = |input: &mut CleanupReplaySourceInput, value: &str| {
        let CleanupReplayFamilyInput::GuardedDelete { mapping_field, .. } = &mut input.family
        else {
            panic!("wrong family")
        };
        *mapping_field = value.to_owned();
    };
    // The eligibility field must be a distinct declared field.
    for name in ["fd", "path", "absent", ""] {
        let mut changed = input(DELETE);
        mapping(&mut changed, name);
        assert!(refused(&changed), "accepted mapping field `{name}`");
    }
    // The three-field shape, and each field's declared type and reset.
    let mut changed = input(DELETE);
    parts(&mut changed).1.fields.truncate(2);
    assert!(refused(&changed));
    let mut changed = input(DELETE);
    parts(&mut changed).1.fields[0].reset = CleanupResetRecord::Integer { value: 0 };
    assert!(refused(&changed));
    let mut changed = input(DELETE);
    parts(&mut changed).1.fields[1].c_type = "void *".into();
    assert!(refused(&changed));
    // The delete family answers over five prestates and the outcome family
    // over four: a body read under the wrong family is refused, not folded.
    let CleanupReplayFamilyInput::GuardedDelete { body, profile, .. } =
        input(DELETE).family.clone()
    else {
        panic!("wrong family")
    };
    let mut swapped = input(DELETE);
    swapped.family = CleanupReplayFamilyInput::NativeCleanupOutcome { body, profile };
    assert!(refused(&swapped));
}

/// The whole projection is declined rather than partially read.
#[test]
fn degenerate_and_oversized_projections_decline_without_panicking() {
    // Unsupported schema version.
    let mut changed = input(CLOSE);
    changed.schema_version = 2;
    assert!(refused(&changed));
    changed.schema_version = 0;
    assert!(refused(&changed));
    // An incomplete prerequisite triple is a lookup key that names nothing.
    for blank in 0..3 {
        let mut changed = input(CLOSE);
        match blank {
            0 => changed.prerequisite.semantic_profile.clear(),
            1 => changed.prerequisite.task_digest.clear(),
            _ => changed.prerequisite.obligation_id.clear(),
        }
        assert!(refused(&changed));
    }
    // No path, an empty path, too many paths and too long a path.
    let mut changed = input(CLOSE);
    parts(&mut changed).0.paths.clear();
    assert!(refused(&changed));
    let mut changed = input(CLOSE);
    parts(&mut changed).0.paths.push(CleanupReplayPathRecord {
        operations: Vec::new(),
    });
    assert!(refused(&changed));
    let mut changed = input(CLOSE);
    let first = parts(&mut changed).0.paths[0].clone();
    for _ in 0..MAX_PATHS {
        parts(&mut changed).0.paths.push(first.clone());
    }
    assert!(refused(&changed));
    let mut changed = input(CLOSE);
    let op = parts(&mut changed).0.paths[0].operations[0].clone();
    for _ in 0..MAX_OPERATIONS {
        parts(&mut changed).0.paths[0].operations.push(op.clone());
    }
    assert!(refused(&changed));
    // Empty identity strings, and a signature that is not the record's.
    for blank in 0..3 {
        let mut changed = input(CLOSE);
        match blank {
            0 => parts(&mut changed).0.function.clear(),
            1 => parts(&mut changed).0.source_revision.clear(),
            _ => parts(&mut changed).0.signature.clear(),
        }
        assert!(refused(&changed));
    }
    let mut changed = input(CLOSE);
    parts(&mut changed).0.signature = "void (struct cio_file *)".into();
    assert!(refused(&changed));
    // A deeply nested expression is declined, not recursed into.
    let mut changed = input(CLOSE);
    let mut deep = CleanupReplayValueRecord {
        c_type: "int".into(),
        expression: CleanupReplayExpressionRecord::Integer { value: "0".into() },
    };
    for _ in 0..(MAX_EXPRESSION_DEPTH + 4) {
        deep = CleanupReplayValueRecord {
            c_type: "int".into(),
            expression: CleanupReplayExpressionRecord::Unary {
                operator: "-".into(),
                value: Box::new(deep),
            },
        };
    }
    parts(&mut changed).0.paths[0].operations.insert(
        0,
        CleanupReplayOperationRecord::Guard {
            condition: deep,
            taken: true,
        },
    );
    assert!(refused(&changed));
}

/// A path shape outside the vocabulary's own rules is declined: the terminal
/// step must be a `ReturnValue` of `int`, and a symbol must name an earlier
/// operation of the same path.
#[test]
fn path_shapes_outside_the_replay_rules_are_declined() {
    // A non-terminal return value.
    let mut changed = input(CLOSE);
    let terminal = parts(&mut changed).0.paths[0]
        .operations
        .last()
        .unwrap()
        .clone();
    parts(&mut changed).0.paths[0]
        .operations
        .insert(0, terminal);
    assert!(refused(&changed));
    // A return of the wrong C type.
    let mut changed = input(CLOSE);
    if let Some(CleanupReplayOperationRecord::ReturnValue { value }) =
        parts(&mut changed).0.paths[0].operations.last_mut()
    {
        value.c_type = "long".into();
    }
    assert!(refused(&changed));
    // A dangling symbol: no producer for the referenced operation.
    let mut changed = input(CLOSE);
    parts(&mut changed).0.paths[0].operations.insert(
        0,
        CleanupReplayOperationRecord::Guard {
            condition: CleanupReplayValueRecord {
                c_type: "int".into(),
                expression: CleanupReplayExpressionRecord::Symbol { operation: 900 },
            },
            taken: true,
        },
    );
    assert!(refused(&changed));
    // A call to a name the profile does not declare.
    let mut changed = input(CLOSE);
    for path in &mut parts(&mut changed).0.paths {
        for operation in &mut path.operations {
            if let CleanupReplayOperationRecord::Call { function, .. } = operation {
                *function = "cio_close".into();
            }
        }
    }
    assert!(refused(&changed));
}

/// The transport contract: the request declines unless it is the resource
/// contract capability at this route's own semantic profile, with a
/// projection present.
#[test]
fn the_route_declines_every_request_that_is_not_its_own() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let base = VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "r".into(),
        plugin_id: manifest.plugin_id.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "r".into(),
            profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_file_unix.c".into(),
            source_digest: "0".repeat(64),
            build_digest: "0".repeat(64),
        },
        facts: Vec::new(),
        assumptions: BTreeMap::new(),
        task_digest: "0".repeat(64),
        obligations: Vec::new(),
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        composition_source: None,
        cleanup_replay_source: Some(input(CLOSE)),
    };
    assert!(propose(&base, &manifest).is_some());
    let mut wrong_capability = base.clone();
    wrong_capability.capability = VerifierCapability::ChunkLifecycle;
    assert!(propose(&wrong_capability, &manifest).is_none());
    let mut wrong_profile = base.clone();
    wrong_profile.target.profile = "qcore.c_call_result_use.v1".into();
    assert!(propose(&wrong_profile, &manifest).is_none());
    let mut wrong_dialect = base.clone();
    wrong_dialect.target.dialect = "qcore".into();
    assert!(propose(&wrong_dialect, &manifest).is_none());
    let mut absent = base.clone();
    absent.cleanup_replay_source = None;
    assert!(propose(&absent, &manifest).is_none());
}
