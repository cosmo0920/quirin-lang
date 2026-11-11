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

//! Wire compatibility and admission rules for the composition projection — the
//! seventh optional projection on a v2 request.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest, rebuilds every
//! declared prerequisite profile from its own contracts, re-establishes every
//! prerequisite **at every level** and re-runs its own compositions before any
//! obligation is covered.
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, COMPOSITION_SOURCE_INPUT_VERSION, CompositionPrerequisiteSet,
    CompositionSourceInput, FILE_CLOSE_DELETE_SEMANTIC_PROFILE, FILE_CLOSE_SEMANTIC_PROFILE,
    FileClosePrerequisites, MEMORY_CLOSE_SEMANTIC_PROFILE, MemoryClosePrerequisites,
    UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapWrapperPrerequisites,
};
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, PathBounds, TaskBounds,
};
use qcore_schemas::{
    PluginId, TargetProfile, UnitIdentity, VerificationRequestV2, VerifierCapability,
};
use serde_json::json;

fn identity(profile: &str, digest: char, id: &str) -> serde_json::Value {
    json!({
        "semantic_profile": profile,
        "task_digest": digest.to_string().repeat(64),
        "obligation_id": id
    })
}

fn body(function: &str, signature: &str) -> serde_json::Value {
    json!({
        "function": function,
        "source_revision": "revision",
        "signature": signature,
        "paths": [{"operations": [
            {"kind":"return_value",
             "value":{"c_type":"int","expression":{"kind":"integer","value":"0"}}}
        ]}],
        "profile_assumptions": {}
    })
}

/// A `void` body, whose complete path ends without a value. Only the
/// composition route's `file_close` wrapper may be projected this way.
fn void_body(function: &str, signature: &str) -> serde_json::Value {
    json!({
        "function": function,
        "source_revision": "revision",
        "signature": signature,
        "paths": [{"operations": [{"kind":"return"}]}],
        "profile_assumptions": {}
    })
}

fn unmap_profile() -> serde_json::Value {
    json!({
        "function": "cio_file_native_unmap",
        "root_type": "struct cio_file *",
        "syscall": "munmap",
        "syscall_signature": "int (void *, size_t)",
        "presence_field": "map",
        "fields": [{"name":"map","c_type":"char *","reset":{"reset":"null"}}],
        "argument_fields": ["map"],
        "errno_function": "__errno_location",
        "report_function": "cio_errno_print"
    })
}

fn close_profile() -> serde_json::Value {
    json!({
        "function": "cio_file_native_close",
        "root_type": "struct cio_file *",
        "syscall": "close",
        "syscall_signature": "int (int)",
        "presence_field": "fd",
        "fields": [{"name":"fd","c_type":"int","reset":{"reset":"integer","value":-1}}],
        "argument_fields": ["fd"],
        "errno_function": "__errno_location",
        "report_function": "cio_errno_print"
    })
}

fn delete_profile() -> serde_json::Value {
    json!({
        "function": "cio_file_native_delete",
        "root_type": "struct cio_file *",
        "syscall": "unlink",
        "syscall_signature": "int (const char *)",
        "presence_field": "fd",
        "fields": [
            {"name":"fd","c_type":"int","reset":{"reset":"integer","value":-1}},
            {"name":"map","c_type":"char *","reset":{"reset":"null"}}
        ],
        "argument_fields": ["path"],
        "errno_function": "__errno_location",
        "report_function": "cio_errno_print"
    })
}

/// The nested composition question, which is the very value the leaf family's
/// own projection carries.
fn nested_question() -> serde_json::Value {
    json!({
        "family": "unmap_wrapper",
        "wrapper": body("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
        "native": body("cio_file_native_unmap", "int (struct cio_file *)"),
        "native_profile": unmap_profile()
    })
}

fn projection() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "prerequisites": {
            "unmap_wrapper": {
                "wrapper_call_use": identity("qcore.c_call_result_use.v1", 'a', "rc:0001"),
                "native_cleanup": identity("qcore.c_cleanup_replay.v1", 'b', "rc:0002"),
                "counter": identity("qcore.c_counter_contract.v1", 'c', "rc:0003")
            }
        },
        "family": nested_question()
    })
}

/// The non-deleting `file_close` question: three prerequisites, one of them the
/// leaf composition, and the nested question carried verbatim.
fn file_close_projection() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "prerequisites": {
            "file_close": {
                "wrapper_call_use": identity("qcore.c_call_result_use.v1", 'd', "rc:0004"),
                "unmap_composition":
                    identity(UNMAP_WRAPPER_SEMANTIC_PROFILE, 'e', "rc:0005"),
                "native_cleanup": identity("qcore.c_cleanup_replay.v1", 'f', "rc:0006")
            }
        },
        "family": {
            "family": "file_close",
            "wrapper": void_body("cio_file_close", "void (struct cio_chunk *, int)"),
            "unmap": nested_question(),
            "close": body("cio_file_native_close", "int (struct cio_file *)"),
            "close_profile": close_profile()
        }
    })
}

/// The deleting `file_close` question: the non-deleting one plus a fourth
/// prerequisite and the guarded-delete body it must recompute.
fn file_close_delete_projection() -> serde_json::Value {
    let mut value = file_close_projection();
    value["prerequisites"]["file_close"]["native_delete"] =
        identity("qcore.c_cleanup_replay.v1", '9', "rc:0007");
    value["family"]["delete"] = json!({
        "body": body("cio_file_native_delete", "int (struct cio_file *)"),
        "profile": delete_profile(),
        "mapping_field": "map"
    });
    value
}

fn obligation(property: ObligationProperty, symbol: &str) -> Obligation {
    Obligation {
        obligation_id: format!("{property}|{symbol}"),
        capability: VerifierCapability::ResourceContracts,
        property,
        subject: ObligationSubject::Function {
            symbol: symbol.into(),
            translation_unit: "cio_file_unix.c".into(),
        },
        scope: ObligationScope::Functions {
            symbols: [symbol.to_owned()].into(),
        },
        bounds: TaskBounds {
            path_exploration: PathBounds::BoundedPaths { paths: 1 },
            schedules_explored: None,
            observed_runs_only: false,
            max_operations: None,
        },
        assumptions: Default::default(),
        evidence_version: 2,
    }
}

fn request(
    input: Option<CompositionSourceInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "composition".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "revision".into(),
            profile: UNMAP_WRAPPER_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_file_unix.c".into(),
            source_digest: "0".repeat(64),
            build_digest: "1".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "2".repeat(64),
        obligations,
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: input,
    }
}

fn input() -> CompositionSourceInput {
    serde_json::from_value(projection()).unwrap()
}

fn file_close_input() -> CompositionSourceInput {
    serde_json::from_value(file_close_projection()).unwrap()
}

fn file_close_delete_input() -> CompositionSourceInput {
    serde_json::from_value(file_close_delete_projection()).unwrap()
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    // All six earlier projections and this seventh one serialize away, so an
    // existing request's bytes are unchanged by the 11.0.0 change.
    for key in [
        "source_effect",
        "counter_effect",
        "reactor_program",
        "teardown_source",
        "call_use_source",
        "cleanup_replay_source",
        "composition_source",
    ] {
        assert!(value.get(key).is_none(), "{key} appeared in the wire form");
    }
    plain.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<VerificationRequestV2>(value).unwrap(),
        plain
    );
    // The projection's own version and digest key moved together, deliberately,
    // so a version-1 witness cannot be replayed against a version-2 task.
    assert_eq!(COMPOSITION_SOURCE_INPUT_VERSION, 2);
    assert_eq!(
        COMPOSITION_SOURCE_INPUT_KEY,
        "qcore:composition-source-input:v2"
    );
    assert_eq!(
        UNMAP_WRAPPER_SEMANTIC_PROFILE,
        "qcore.c_unmap_wrapper_composition.v1"
    );
    assert_eq!(
        FILE_CLOSE_SEMANTIC_PROFILE,
        "qcore.c_file_close_composition.v1"
    );
    assert_eq!(
        FILE_CLOSE_DELETE_SEMANTIC_PROFILE,
        "qcore.c_file_close_delete_composition.v1"
    );
}

#[test]
fn a_projection_round_trips_and_carries_no_derived_reading() {
    for input in [input(), file_close_input(), file_close_delete_input()] {
        assert!(input.version_matches());
        assert!(input.prerequisites.is_complete() && input.prerequisites.are_distinct());
        assert!(input.family.matches(&input.prerequisites));
        let text = serde_json::to_string(&input).unwrap();
        for derived in [
            "cases",
            "native_source_path",
            "counter_decrements",
            "wrapper_writes",
            "returned",
            "verdict",
            "certificate",
            "unmap_case",
            "close_case",
            "delete_case",
            "deletion",
            "backend_released",
            "path_released",
            "up_counter_decrements",
            "uncovered_paths",
            "residual",
            "unresolved",
        ] {
            assert!(!text.contains(derived), "`{derived}` is representable");
        }
        assert_eq!(
            serde_json::from_str::<CompositionSourceInput>(&text).unwrap(),
            input
        );
    }
    assert_eq!(input().subject_function(), "munmap_file");
    assert_eq!(file_close_input().subject_function(), "cio_file_close");
}

/// The nesting depth is a measured property of the transported question, not an
/// assumption: the leaf family nests once, both `file_close` families twice.
#[test]
fn the_nesting_depth_is_measured_from_the_question() {
    assert_eq!(input().family.nesting_depth(), 1);
    assert!(input().family.nested().is_empty());
    for input in [file_close_input(), file_close_delete_input()] {
        assert_eq!(input.family.nesting_depth(), 2);
        // And the nested question is exactly the leaf family's own projection.
        assert_eq!(input.family.nested(), vec![&self::input().family]);
    }
}

/// The two `file_close` entry points are distinct questions under distinct
/// semantic profiles, and the deleting arm is not merely the non-deleting one
/// with an empty extra field.
#[test]
fn the_two_entry_points_are_distinct_questions() {
    let plain = file_close_input();
    let deleting = file_close_delete_input();
    assert_eq!(plain.family.semantic_profile(), FILE_CLOSE_SEMANTIC_PROFILE);
    assert_eq!(
        deleting.family.semantic_profile(),
        FILE_CLOSE_DELETE_SEMANTIC_PROFILE
    );
    assert_ne!(plain, deleting);
    assert_ne!(plain.digest(), deleting.digest());
    // Absent is not empty: the non-deleting question omits the key entirely.
    let text = serde_json::to_string(&plain).unwrap();
    assert!(!text.contains("delete"));
    assert!(serde_json::to_string(&deleting).unwrap().contains("delete"));
}

/// Unknown fields are refused rather than ignored, in every nested record a
/// producer could try to widen — including inside the nested question.
#[test]
fn unknown_fields_are_refused() {
    let mut value = projection();
    value["derived_cases"] = json!([]);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = projection();
    value["prerequisites"]["unmap_wrapper"]["extra"] = json!("x");
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = projection();
    value["family"]["native_profile"]["policy"] = json!("native_unmap_outcomes");
    assert!(
        serde_json::from_value::<CompositionSourceInput>(value).is_err(),
        "the policy name became representable on the wire"
    );

    let mut value = file_close_projection();
    value["family"]["unmap"]["native_profile"]["policy"] = json!("native_unmap_outcomes");
    assert!(
        serde_json::from_value::<CompositionSourceInput>(value).is_err(),
        "the nested question is not covered by the unknown-field rule"
    );

    let mut value = file_close_delete_projection();
    value["family"]["delete"]["expected_cases"] = json!([]);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = file_close_projection();
    value["prerequisites"]["file_close"]["counter"] =
        identity("qcore.c_counter_contract.v1", 'c', "rc:0003");
    assert!(
        serde_json::from_value::<CompositionSourceInput>(value).is_err(),
        "one family's prerequisite name became readable in another's set"
    );
}

/// A family's prerequisite set is never read as another family's, and a nested
/// question that is not a leaf composition is refused.
#[test]
fn a_family_never_borrows_another_familys_prerequisite_set() {
    // The leaf family with the file_close set, and the reverse.
    let mut value = projection();
    value["prerequisites"] = file_close_projection()["prerequisites"].clone();
    let crossed: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert!(!crossed.family.matches(&crossed.prerequisites));

    let mut value = file_close_projection();
    value["prerequisites"] = projection()["prerequisites"].clone();
    let crossed: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert!(!crossed.family.matches(&crossed.prerequisites));

    // The deleting body without the deleting prerequisite, and the reverse.
    let mut value = file_close_delete_projection();
    value["prerequisites"]["file_close"]
        .as_object_mut()
        .unwrap()
        .remove("native_delete");
    let half: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert!(!half.family.matches(&half.prerequisites));

    let mut value = file_close_delete_projection();
    value["family"].as_object_mut().unwrap().remove("delete");
    let half: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert!(!half.family.matches(&half.prerequisites));

    // A nested question that is itself a file_close question is refused: this
    // vocabulary nests exactly one level, and says so structurally.
    let mut value = file_close_projection();
    value["family"]["unmap"] = file_close_projection()["family"].clone();
    let deeper: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert_eq!(deeper.family.nesting_depth(), 3);
    assert!(!deeper.family.matches(&deeper.prerequisites));
}

/// The request admission rules: exactly one obligation of this projection's own
/// property, for the wrapper it names, never beside another projection, with
/// complete and distinct prerequisite identities that match the family.
#[test]
fn request_admission_is_exact() {
    let ok = request(
        Some(input()),
        vec![obligation(
            ObligationProperty::CompositionContract,
            "munmap_file",
        )],
    );
    ok.validate().unwrap();
    // Both file_close entry points are admitted for their own subject function.
    for input in [file_close_input(), file_close_delete_input()] {
        request(
            Some(input),
            vec![obligation(
                ObligationProperty::CompositionContract,
                "cio_file_close",
            )],
        )
        .validate()
        .unwrap();
    }

    // The obligation must name the projection's own function and property.
    for wrong in [
        obligation(ObligationProperty::CompositionContract, "cio_file_close"),
        obligation(ObligationProperty::CleanupReplayContract, "munmap_file"),
    ] {
        assert!(request(Some(input()), vec![wrong]).validate().is_err());
    }
    // A projection with no obligation, and an obligation with no projection.
    assert!(request(Some(input()), Vec::new()).validate().is_err());
    assert!(
        request(
            None,
            vec![obligation(
                ObligationProperty::CompositionContract,
                "munmap_file"
            )]
        )
        .validate()
        .is_err()
    );
    // Two projections never travel together.
    let mut both = ok.clone();
    both.call_use_source = Some(
        serde_json::from_value(json!({
            "schema_version": 1,
            "family": {
                "family": "filesystem_attempt",
                "body": {
                    "function": "munmap_file",
                    "source_revision": "revision",
                    "signature": "int (struct cio_ctx *, struct cio_chunk *)",
                    "paths": [],
                    "profile_assumptions": {}
                }
            }
        }))
        .expect("the call-use literal must parse for this row to be meaningful"),
    );
    assert!(both.validate().is_err());
    // The superseded version is refused in this direction too.
    let mut version = ok.clone();
    version.composition_source.as_mut().unwrap().schema_version = 1;
    assert!(version.validate().is_err());
    let mut version = ok.clone();
    version.composition_source.as_mut().unwrap().schema_version = 3;
    assert!(version.validate().is_err());
    // An incomplete identity and a repeated one each refuse, in both families.
    let mut incomplete = projection();
    incomplete["prerequisites"]["unmap_wrapper"]["native_cleanup"]["obligation_id"] = json!("");
    assert!(
        request(
            Some(serde_json::from_value(incomplete).unwrap()),
            vec![obligation(
                ObligationProperty::CompositionContract,
                "munmap_file"
            )]
        )
        .validate()
        .is_err()
    );
    let mut repeated = projection();
    repeated["prerequisites"]["unmap_wrapper"]["counter"] =
        repeated["prerequisites"]["unmap_wrapper"]["wrapper_call_use"].clone();
    assert!(
        request(
            Some(serde_json::from_value(repeated).unwrap()),
            vec![obligation(
                ObligationProperty::CompositionContract,
                "munmap_file"
            )]
        )
        .validate()
        .is_err()
    );
    let mut repeated = file_close_delete_projection();
    repeated["prerequisites"]["file_close"]["native_delete"] =
        repeated["prerequisites"]["file_close"]["native_cleanup"].clone();
    assert!(
        request(
            Some(serde_json::from_value(repeated).unwrap()),
            vec![obligation(
                ObligationProperty::CompositionContract,
                "cio_file_close"
            )]
        )
        .validate()
        .is_err()
    );
    // A family that does not match its prerequisite set refuses at the wire
    // gate, before any core ever sees it.
    let mut crossed = file_close_delete_projection();
    crossed["family"].as_object_mut().unwrap().remove("delete");
    assert!(
        request(
            Some(serde_json::from_value(crossed).unwrap()),
            vec![obligation(
                ObligationProperty::CompositionContract,
                "cio_file_close"
            )]
        )
        .validate()
        .is_err()
    );
}

/// The exact variant inventory the composition route's transported bodies can
/// express, pinned by name.
///
/// `P04-COMPOSE` recorded this pin as the mechanism that makes
/// `P04-COMPOSE-SHARED-PATHS` real — a future widening of the shared step and
/// expression enums must fail *here* and be decided, instead of silently
/// widening a family that reuses them. **The test it named did not exist**;
/// card `P04-COMPOSE-A` found the dangling reference while adding the ninth
/// operation form (`return`, for the `void` `cio_file_close` wrapper) and wrote
/// the pin rather than correcting the claim downwards.
///
/// A schemas-level pin can only bind the *inventory*; which variants a given
/// family actually admits is decided by the gateway projector and by each
/// producer, both of which refuse by name. Those refusals are pinned in their
/// own crates.
#[test]
fn composition_vocabulary_is_pinned() {
    use qcore_schemas::cleanup_replay_summary::{
        CleanupReplayExpressionRecord as Expr, CleanupReplayOperationRecord as Op,
        CleanupReplayValueRecord,
    };
    let value = || CleanupReplayValueRecord {
        c_type: "int".into(),
        expression: Expr::Integer {
            value: "0".to_owned(),
        },
    };
    let kind = |v: &serde_json::Value| v["kind"].as_str().unwrap().to_owned();

    let operations = [
        Op::Read { place: value() },
        Op::Write {
            place: value(),
            value: value(),
        },
        Op::LocalStart {
            local: 0,
            c_type: "int".into(),
        },
        Op::LocalWrite {
            local: 0,
            value: value(),
        },
        Op::LocalEnd { local: 0 },
        Op::Guard {
            condition: value(),
            taken: true,
        },
        Op::Call {
            function: "f".into(),
            signature: "int (void)".into(),
            arguments: Vec::new(),
            result_type: "int".into(),
        },
        Op::ReturnValue { value: value() },
        Op::Return,
    ];
    let mut kinds: Vec<String> = operations
        .iter()
        .map(|op| kind(&serde_json::to_value(op).unwrap()))
        .collect();
    kinds.sort();
    assert_eq!(
        kinds,
        vec![
            "call",
            "guard",
            "local_end",
            "local_start",
            "local_write",
            "read",
            "return",
            "return_value",
            "write",
        ],
        "the transported operation inventory moved; decide which families admit \
         the change before updating this pin"
    );

    let expressions = [
        Expr::Parameter { index: 0 },
        Expr::Symbol { operation: 0 },
        Expr::Integer {
            value: "0".to_owned(),
        },
        Expr::String {
            value: "s".to_owned(),
        },
        Expr::Field {
            base: Box::new(value()),
            name: "f".into(),
        },
        Expr::Cast {
            cast: "NoOp".into(),
            value: Box::new(value()),
        },
        Expr::Unary {
            operator: "!".into(),
            value: Box::new(value()),
        },
        Expr::Binary {
            operator: "==".into(),
            left: Box::new(value()),
            right: Box::new(value()),
        },
    ];
    let mut kinds: Vec<String> = expressions
        .iter()
        .map(|expression| kind(&serde_json::to_value(expression).unwrap()))
        .collect();
    kinds.sort();
    assert_eq!(
        kinds,
        vec![
            "binary",
            "cast",
            "field",
            "integer",
            "parameter",
            "string",
            "symbol",
            "unary",
        ],
        "the transported expression inventory moved; decide which families admit \
         the change before updating this pin"
    );

    // The ninth operation form is admitted **here** and refused at parse time by
    // the cleanup-replay family, whose own wire test asserts the refusal. Both
    // halves are checked, so the widening cannot become a silent one.
    let value_free: CompositionSourceInput =
        serde_json::from_value(file_close_projection()).unwrap();
    assert!(
        serde_json::to_string(&value_free)
            .unwrap()
            .contains("return"),
        "the void wrapper's value-free return did not travel"
    );

    // The transported family tags, pinned so a third cannot appear without a
    // decision here.
    let mut families: Vec<String> = [
        projection(),
        file_close_projection(),
        file_close_delete_projection(),
        memory_close_projection(),
        filesystem_close_projection(),
    ]
    .iter()
    .map(|value| value["family"]["family"].as_str().unwrap().to_owned())
    .collect();
    families.push(
        unsupported_close_projection()["family"]["family"]
            .as_str()
            .unwrap()
            .to_owned(),
    );
    families.sort();
    families.dedup();
    // Deliberate pin update (card `P04-COMPOSE-UNSUPPORTED`): a fifth family
    // tag, after `P04-COMPOSE-FS`'s fourth. The **load-bearing negative halves
    // above are untouched** — the operation and expression inventories did not
    // move, so this card widened no shared enum either.
    assert_eq!(
        families,
        vec![
            "file_close".to_string(),
            "filesystem_chunk_close".into(),
            "memory_close".into(),
            "unmap_wrapper".into(),
            "unsupported_close".into()
        ]
    );
}

/// The prerequisite identity rules are not vacuous, in either family.
#[test]
fn the_prerequisite_rules_are_decidable() {
    let complete = input().prerequisites;
    assert!(complete.is_complete() && complete.are_distinct());
    let CompositionPrerequisiteSet::UnmapWrapper(leaf) = complete else {
        panic!("the leaf projection must carry the leaf set");
    };
    let repeated = UnmapWrapperPrerequisites {
        counter: leaf.wrapper_call_use.clone(),
        ..leaf.clone()
    };
    assert!(repeated.is_complete() && !repeated.are_distinct());
    let blank = UnmapWrapperPrerequisites {
        counter: qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        },
        ..leaf
    };
    assert!(!blank.is_complete());

    let CompositionPrerequisiteSet::FileClose(close) = file_close_delete_input().prerequisites
    else {
        panic!("the file_close projection must carry the file_close set");
    };
    assert!(close.is_complete() && close.are_distinct());
    // The optional fourth identity is subject to both rules, not exempt.
    let repeated = FileClosePrerequisites {
        native_delete: Some(close.native_cleanup.clone()),
        ..close.clone()
    };
    assert!(repeated.is_complete() && !repeated.are_distinct());
    let blank = FileClosePrerequisites {
        native_delete: Some(qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        }),
        ..close.clone()
    };
    assert!(!blank.is_complete());
    // A three-identity set is complete and distinct on its own.
    let plain = FileClosePrerequisites {
        native_delete: None,
        ..close
    };
    assert!(plain.is_complete() && plain.are_distinct());

    // Every digest key is distinct, so no prerequisite's movement can be
    // absorbed by another's key.
    let keys = [
        UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY,
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY,
        UnmapWrapperPrerequisites::COUNTER_KEY,
        FileClosePrerequisites::WRAPPER_CALL_USE_KEY,
        FileClosePrerequisites::UNMAP_COMPOSITION_KEY,
        FileClosePrerequisites::NATIVE_CLEANUP_KEY,
        FileClosePrerequisites::NATIVE_DELETE_KEY,
    ];
    let mut sorted = keys.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), keys.len());
}

/// The memory-close question: four prerequisites, a wrapper and a probe body
/// imported from **derived** text, and the derivation carried as a set of
/// claims the core checks against its own.
fn memory_close_derivation() -> serde_json::Value {
    json!({
        "recipe": "\nint probe(void) { return CIO_STORE_MEM; }\n",
        "recipe_function": "probe",
        "base_source_digest": "3".repeat(64),
        "derived_source_digest": "4".repeat(64),
        "base_input_class": "declared_file",
        "derived_input_class": "anonymous_text"
    })
}

fn memory_close_callees() -> serde_json::Value {
    json!({
        "reset": {"function": "cio_error_reset", "signature": "void (struct cio_chunk *)"},
        "backend": {"function": "cio_memfs_close", "signature": "void (struct cio_chunk *)"},
        "unlink": {"function": "mk_list_del", "signature": "void (struct mk_list *)"},
        "counter": {"function": "cio_chunk_counter_total_sub",
                    "signature": "unsigned long (struct cio_ctx *)"}
    })
}

fn memory_close_projection() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "prerequisites": {
            "memory_close": {
                "reset": identity("qcore.c_frame_helper_contract.v1", '1', "rc:1001"),
                "backend": identity("qcore.c_field_destructor_contract.v1", '2', "rc:1002"),
                "unlink": identity("qcore.c_list_unlink_contract.v1", '3', "rc:1003"),
                "counter": identity("qcore.c_counter_contract.v1", '4', "rc:1004")
            }
        },
        "family": {
            "family": "memory_close",
            "wrapper": void_body("cio_chunk_close", "void (struct cio_chunk *, int)"),
            "probe": body("probe", "int (void)"),
            "derivation": memory_close_derivation(),
            "callees": memory_close_callees()
        }
    })
}

fn memory_close_input() -> CompositionSourceInput {
    serde_json::from_value(memory_close_projection()).unwrap()
}

/// The memory-close question round-trips and carries **no** derived reading —
/// including the two figures only this family has: the probed discriminant and
/// the per-entry replay state.
#[test]
fn the_memory_close_question_round_trips_and_carries_no_derived_reading() {
    let input = memory_close_input();
    assert!(input.version_matches());
    assert_eq!(input.subject_function(), "cio_chunk_close");
    assert_eq!(
        input.family.semantic_profile(),
        MEMORY_CLOSE_SEMANTIC_PROFILE
    );
    assert_eq!(input.family.nesting_depth(), 1);
    assert!(input.family.nested().is_empty());
    assert!(input.family.native().is_none());
    assert!(input.family.native_profile().is_none());
    let derivation = input.family.derivation().expect("the derived family");
    assert!(derivation.is_complete() && derivation.is_a_derivation());
    let bytes = serde_json::to_string(&input).unwrap();
    for absent in [
        "memory_kind",
        "cases",
        "checked_paths",
        "uncovered_paths",
        "backend_discharged",
        "primary_unlinked",
        "state_unlinked",
        "name_released",
        "chunk_released",
        "counter_decremented",
        "certificate",
        "verdict",
        "source_path",
    ] {
        assert!(
            !bytes.contains(absent),
            "the memory-close question must not carry `{absent}`"
        );
    }
    let round: CompositionSourceInput = serde_json::from_str(&bytes).unwrap();
    assert_eq!(round, input);
}

/// A derivation whose derived text equals its base text is not a derivation,
/// and a projection is refused rather than treated as a plain import.
#[test]
fn a_derivation_that_derives_nothing_is_not_a_derivation() {
    let mut value = memory_close_projection();
    value["family"]["derivation"]["derived_source_digest"] = json!("3".repeat(64));
    let input: CompositionSourceInput = serde_json::from_value(value).unwrap();
    let derivation = input.family.derivation().unwrap();
    assert!(derivation.is_complete());
    assert!(
        !derivation.is_a_derivation(),
        "equal digests mean the recipe changed nothing"
    );
    for field in [
        "recipe",
        "recipe_function",
        "base_source_digest",
        "derived_source_digest",
        "base_input_class",
        "derived_input_class",
    ] {
        let mut value = memory_close_projection();
        value["family"]["derivation"][field] = json!("");
        let input: CompositionSourceInput = serde_json::from_value(value).unwrap();
        assert!(
            !input.family.derivation().unwrap().is_complete(),
            "an empty `{field}` leaves the derivation incomplete"
        );
    }
}

/// The memory-close family names the memory-close set, and no other.
#[test]
fn the_memory_close_family_never_borrows_another_familys_prerequisite_set() {
    let memory = memory_close_input();
    assert!(memory.family.matches(&memory.prerequisites));
    assert!(memory.prerequisites.memory_close().is_some());
    assert!(memory.prerequisites.unmap_wrapper().is_none());
    assert!(memory.prerequisites.file_close().is_none());

    // Neither direction: the memory-close family under another set, and another
    // family under the memory-close set.
    let leaf = input();
    assert!(!memory.family.matches(&leaf.prerequisites));
    assert!(!leaf.family.matches(&memory.prerequisites));
    let file_close = file_close_input();
    assert!(!memory.family.matches(&file_close.prerequisites));
    assert!(!file_close.family.matches(&memory.prerequisites));
}

/// Unknown fields are refused in every new record.
#[test]
fn unknown_fields_are_refused_in_the_memory_close_records() {
    for pointer in [
        vec!["family", "derivation"],
        vec!["family", "callees"],
        vec!["family", "callees", "reset"],
        vec!["prerequisites", "memory_close"],
    ] {
        let mut value = memory_close_projection();
        let mut node = &mut value;
        for step in &pointer {
            node = &mut node[*step];
        }
        node["invented"] = json!("x");
        assert!(
            serde_json::from_value::<CompositionSourceInput>(value).is_err(),
            "an unknown field survived at {pointer:?}"
        );
    }
}

/// The four identity rules are non-vacuous, and the four digest keys are
/// distinct from each other and from every other family's.
#[test]
fn the_memory_close_prerequisite_rules_are_decidable() {
    let honest = memory_close_input();
    let set = honest.prerequisites.memory_close().unwrap().clone();
    assert!(set.is_complete() && set.are_distinct());
    for slot in 0..4 {
        let mut forged = set.clone();
        *[
            &mut forged.reset,
            &mut forged.backend,
            &mut forged.unlink,
            &mut forged.counter,
        ][slot] = qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        };
        assert!(!forged.is_complete(), "slot {slot} is load-bearing");
    }
    let mut repeated = set.clone();
    repeated.unlink = repeated.counter.clone();
    assert!(!repeated.are_distinct());

    let mut keys = vec![
        MemoryClosePrerequisites::RESET_KEY,
        MemoryClosePrerequisites::BACKEND_KEY,
        MemoryClosePrerequisites::UNLINK_KEY,
        MemoryClosePrerequisites::COUNTER_KEY,
        UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY,
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY,
        UnmapWrapperPrerequisites::COUNTER_KEY,
        FileClosePrerequisites::WRAPPER_CALL_USE_KEY,
        FileClosePrerequisites::UNMAP_COMPOSITION_KEY,
        FileClosePrerequisites::NATIVE_CLEANUP_KEY,
        FileClosePrerequisites::NATIVE_DELETE_KEY,
    ];
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(
        keys.len(),
        count,
        "every prerequisite digest key is distinct"
    );
}

// ---------------------------------------------------------------------------
// The filesystem family (card `P04-COMPOSE-FS`).
// ---------------------------------------------------------------------------

fn filesystem_close_derivation() -> serde_json::Value {
    json!({
        "recipe": "\nint qcore_cio_filesystem_store_value(void) { return CIO_STORE_FS; }\n",
        "recipe_function": "qcore_cio_filesystem_store_value",
        "base_source_digest": "3".repeat(64),
        "derived_source_digest": "7".repeat(64),
        "base_input_class": "declared_file",
        "derived_input_class": "anonymous_text"
    })
}

fn filesystem_close_projection() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "prerequisites": {
            "filesystem_chunk_close": {
                "memory_close": identity("qcore.c_memory_close_composition.v1", '5', "rc:1005"),
                "file_close_delete":
                    identity("qcore.c_file_close_delete_composition.v1", '6', "rc:1006")
            }
        },
        "family": {
            "family": "filesystem_chunk_close",
            "memory": memory_close_projection()["family"].clone(),
            "backend": file_close_delete_projection()["family"].clone(),
            "probe": body("fs_probe", "int (void)"),
            "derivation": filesystem_close_derivation()
        }
    })
}

fn filesystem_close_input() -> CompositionSourceInput {
    serde_json::from_value(filesystem_close_projection()).unwrap()
}

/// The filesystem question round-trips and carries **no** derived reading, at
/// any of its three levels.
#[test]
fn the_filesystem_question_round_trips_and_carries_no_derived_reading() {
    let input = filesystem_close_input();
    assert!(input.version_matches());
    // Its subject is the nested memory question's wrapper: this family carries
    // no body of its own but the probe.
    assert_eq!(input.subject_function(), "cio_chunk_close");
    assert_eq!(
        input.family.semantic_profile(),
        qcore_schemas::composition_summary::FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
    );
    assert_eq!(input.family.nesting_depth(), 3);
    assert_eq!(input.family.nested().len(), 2);
    assert!(input.family.native().is_none());
    assert!(input.family.native_profile().is_none());
    assert_eq!(input.family.wrapper(), input.family.nested()[0].wrapper());
    let derivation = input.family.derivation().expect("the derived family");
    assert!(derivation.is_complete() && derivation.is_a_derivation());
    // **Two** derivations, and this family's own is not the nested one's.
    let nested = input.family.nested()[0]
        .derivation()
        .expect("the nested question derives too");
    assert_ne!(derivation.recipe, nested.recipe);
    assert_ne!(derivation.recipe_function, nested.recipe_function);
    assert_ne!(
        derivation.derived_source_digest,
        nested.derived_source_digest
    );
    assert_eq!(derivation.base_source_digest, nested.base_source_digest);
    let bytes = serde_json::to_string(&input).unwrap();
    for absent in [
        "filesystem_kind",
        "memory_kind",
        "cases",
        "checked_paths",
        "uncovered_paths",
        "combined_uncovered_paths",
        "backend_case",
        "unmap_case",
        "close_case",
        "delete_case",
        "chunk_released",
        "backend_discharged",
        "certificate",
        "verdict",
        "source_path",
    ] {
        assert!(
            !bytes.contains(absent),
            "the filesystem question must not carry `{absent}`"
        );
    }
    let round: CompositionSourceInput = serde_json::from_str(&bytes).unwrap();
    assert_eq!(round, input);
}

/// The filesystem family never borrows another family's prerequisite set, and
/// no other family borrows its.
#[test]
fn the_filesystem_family_and_its_prerequisite_set_are_bound_together() {
    let honest = filesystem_close_input();
    assert!(honest.family.matches(&honest.prerequisites));
    for other in [input(), file_close_delete_input(), memory_close_input()] {
        assert!(
            !honest.family.matches(&other.prerequisites),
            "the filesystem family accepted another family's set"
        );
        assert!(
            !other.family.matches(&honest.prerequisites),
            "another family accepted the filesystem set"
        );
    }
    // Both nested slots are shape-checked: the memory half must be the memory
    // family, and the backend half the **deleting** file_close family.
    let mut swapped = filesystem_close_projection();
    let memory = swapped["family"]["memory"].clone();
    swapped["family"]["memory"] = swapped["family"]["backend"].clone();
    swapped["family"]["backend"] = memory;
    let swapped: CompositionSourceInput = serde_json::from_value(swapped).unwrap();
    assert!(
        !swapped.family.matches(&swapped.prerequisites),
        "the two nested slots are interchangeable"
    );
    let mut non_deleting = filesystem_close_projection();
    non_deleting["family"]["backend"] = file_close_projection()["family"].clone();
    let non_deleting: CompositionSourceInput = serde_json::from_value(non_deleting).unwrap();
    assert!(
        !non_deleting.family.matches(&non_deleting.prerequisites),
        "the non-deleting backend arm was admitted"
    );
}

/// The filesystem prerequisite rules are not vacuous, and its two digest keys
/// are distinct from every other family's.
#[test]
fn the_filesystem_prerequisite_rules_are_decidable_and_its_keys_are_distinct() {
    use qcore_schemas::composition_summary::FilesystemChunkClosePrerequisites;
    let complete = filesystem_close_input().prerequisites;
    assert!(complete.is_complete() && complete.are_distinct());
    let CompositionPrerequisiteSet::FilesystemChunkClose(set) = complete else {
        panic!("the filesystem projection must carry the filesystem set");
    };
    let repeated = FilesystemChunkClosePrerequisites {
        file_close_delete: set.memory_close.clone(),
        ..set.clone()
    };
    assert!(repeated.is_complete() && !repeated.are_distinct());
    let blank = FilesystemChunkClosePrerequisites {
        memory_close: qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        },
        ..set.clone()
    };
    assert!(!blank.is_complete());

    let mut keys = vec![
        UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY,
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY,
        UnmapWrapperPrerequisites::COUNTER_KEY,
        FileClosePrerequisites::WRAPPER_CALL_USE_KEY,
        FileClosePrerequisites::UNMAP_COMPOSITION_KEY,
        FileClosePrerequisites::NATIVE_CLEANUP_KEY,
        FileClosePrerequisites::NATIVE_DELETE_KEY,
        MemoryClosePrerequisites::RESET_KEY,
        MemoryClosePrerequisites::BACKEND_KEY,
        MemoryClosePrerequisites::UNLINK_KEY,
        MemoryClosePrerequisites::COUNTER_KEY,
        FilesystemChunkClosePrerequisites::MEMORY_CLOSE_KEY,
        FilesystemChunkClosePrerequisites::FILE_CLOSE_DELETE_KEY,
    ];
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(
        keys.len(),
        count,
        "all thirteen prerequisite digest keys across the four families are distinct"
    );
}

/// Unknown fields are refused in this family's own records.
#[test]
fn unknown_fields_are_refused_in_the_filesystem_records() {
    let mut value = filesystem_close_projection();
    value["family"]["derivation"]["extra"] = json!(true);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = filesystem_close_projection();
    value["prerequisites"]["filesystem_chunk_close"]["extra"] = json!(true);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());
}

// ---------------------------------------------------------------------------
// The symbolic other-backend family (card `P04-COMPOSE-UNSUPPORTED`).
// ---------------------------------------------------------------------------

fn unsupported_close_projection() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "prerequisites": {
            "unsupported_close": {
                "filesystem_chunk_close": identity(
                    "qcore.c_filesystem_chunk_close_composition.v1", '8', "rc:1008")
            }
        },
        "family": {
            "family": "unsupported_close",
            "filesystem": filesystem_close_projection()["family"].clone()
        }
    })
}

fn unsupported_close_input() -> CompositionSourceInput {
    serde_json::from_value(unsupported_close_projection()).unwrap()
}

/// The symbolic question round-trips and carries **no** derived reading, at any
/// of its four levels — and no classification word either.
#[test]
fn the_symbolic_question_round_trips_and_carries_no_derived_reading() {
    let input = unsupported_close_input();
    assert!(input.version_matches());
    // Its subject is the nested memory question's wrapper, two levels down: this
    // family carries no body of its own at all.
    assert_eq!(input.subject_function(), "cio_chunk_close");
    assert_eq!(
        input.family.semantic_profile(),
        qcore_schemas::composition_summary::UNSUPPORTED_CLOSE_SEMANTIC_PROFILE
    );
    assert!(input.family.derivation().is_none(), "it derives no text");
    assert!(input.family.probe().is_none(), "it imports no probe");
    assert!(input.family.native().is_none());
    assert!(input.family.native_profile().is_none());
    // Four levels, one nested question.
    assert_eq!(input.family.nesting_depth(), 4);
    let nested = input.family.nested();
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].nesting_depth(), 3);
    // Its wrapper *is* the nested question's, which is the memory question's.
    assert_eq!(input.family.wrapper(), nested[0].wrapper());
    // Both derivations it depends on are inside the one nested question.
    assert!(nested[0].derivation().is_some());
    assert!(nested[0].nested()[0].derivation().is_some());

    let bytes = serde_json::to_string(&input).unwrap();
    for absent in [
        "excluded_kinds",
        "source_path",
        "cases",
        "memory_kind",
        "filesystem_kind",
        "checked_paths",
        "uncovered_paths",
        "combined_uncovered_paths",
        "verdict",
        "certificate",
        "Unknown",
        "backend_custody_unresolved",
        "nested_discharge_permitted",
    ] {
        assert!(!bytes.contains(absent), "`{absent}` travelled");
    }
    let round_tripped: CompositionSourceInput = serde_json::from_str(&bytes).unwrap();
    assert_eq!(round_tripped, input);
}

/// The family and its prerequisite set are bound together, in both directions,
/// and the nested question's own shape rule is enforced at this level too.
#[test]
fn the_symbolic_family_and_its_prerequisite_set_are_bound_together() {
    let input = unsupported_close_input();
    assert!(input.family.matches(&input.prerequisites));
    // Every other family's set is refused under this family.
    for other in [
        projection(),
        file_close_projection(),
        file_close_delete_projection(),
        memory_close_projection(),
        filesystem_close_projection(),
    ] {
        let borrowed: CompositionSourceInput = serde_json::from_value(other).unwrap();
        assert!(
            !input.family.matches(&borrowed.prerequisites),
            "the symbolic family borrowed another family's set"
        );
        assert!(
            !borrowed.family.matches(&input.prerequisites),
            "another family borrowed the symbolic family's set"
        );
    }
    // A nested question of the wrong family is refused here.
    for wrong in ["memory_close", "file_close", "unmap_wrapper"] {
        let mut value = unsupported_close_projection();
        value["family"]["filesystem"] = match wrong {
            "memory_close" => memory_close_projection()["family"].clone(),
            "file_close" => file_close_projection()["family"].clone(),
            _ => projection()["family"].clone(),
        };
        let borrowed: CompositionSourceInput = serde_json::from_value(value).unwrap();
        assert!(
            !borrowed.family.matches(&borrowed.prerequisites),
            "a `{wrong}` question was accepted as the symbolic family's nested half"
        );
    }
    // And a nested filesystem question whose backend arm is the *non-deleting*
    // one is refused here as well, one level down.
    let mut value = unsupported_close_projection();
    value["family"]["filesystem"]["backend"] = file_close_projection()["family"].clone();
    let borrowed: CompositionSourceInput = serde_json::from_value(value).unwrap();
    assert!(
        !borrowed.family.matches(&borrowed.prerequisites),
        "a non-deleting backend arm reached the symbolic family"
    );
}

/// The one-member prerequisite set's rules, and the fourteen distinct digest
/// keys across all five families.
#[test]
fn the_symbolic_prerequisite_rules_are_decidable_and_its_key_is_distinct() {
    use qcore_schemas::composition_summary::UnsupportedClosePrerequisites;
    let complete = unsupported_close_input().prerequisites;
    assert!(complete.is_complete() && complete.are_distinct());
    let CompositionPrerequisiteSet::UnsupportedClose(set) = complete else {
        panic!("the symbolic projection must carry the symbolic set");
    };
    let blank = UnsupportedClosePrerequisites {
        filesystem_chunk_close: qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        },
    };
    assert!(!blank.is_complete());
    // Recorded rather than dressed up: distinctness is *vacuous* on a one-member
    // set, so completeness is the rule that carries the weight here.
    assert!(
        blank.are_distinct(),
        "distinctness is vacuous on a singleton, and is pinned as such"
    );
    let _ = set;

    let mut keys = vec![
        UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY,
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY,
        UnmapWrapperPrerequisites::COUNTER_KEY,
        FileClosePrerequisites::WRAPPER_CALL_USE_KEY,
        FileClosePrerequisites::UNMAP_COMPOSITION_KEY,
        FileClosePrerequisites::NATIVE_CLEANUP_KEY,
        FileClosePrerequisites::NATIVE_DELETE_KEY,
        MemoryClosePrerequisites::RESET_KEY,
        MemoryClosePrerequisites::BACKEND_KEY,
        MemoryClosePrerequisites::UNLINK_KEY,
        MemoryClosePrerequisites::COUNTER_KEY,
        qcore_schemas::composition_summary::FilesystemChunkClosePrerequisites::MEMORY_CLOSE_KEY,
        qcore_schemas::composition_summary::FilesystemChunkClosePrerequisites::FILE_CLOSE_DELETE_KEY,
        UnsupportedClosePrerequisites::FILESYSTEM_CHUNK_CLOSE_KEY,
    ];
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(
        keys.len(),
        count,
        "all fourteen prerequisite digest keys across the five families are distinct"
    );
}

/// Unknown fields are refused in this family's own records, and the proposal
/// record has no field a conclusion could hide in.
///
/// **A measured boundary, recorded rather than asserted away.** The family enum
/// is internally tagged, and serde does not apply `deny_unknown_fields` to an
/// internally-tagged variant's own object, so an unrecognised key *beside* the
/// `family` tag is silently dropped. That is true of all five families, not just
/// this one, and the test below pins it in both places so the limitation is
/// visible and cannot be mistaken for this card's doing. It buys a producer
/// nothing — a dropped key is not in the projection the core digests, so the
/// digest, the task and the comparison are all unaffected — but it is a real
/// difference from every `deny_unknown_fields` record here, and it is named:
/// `P04-COMPOSE-TAGGED-UNKNOWN`.
#[test]
fn unknown_fields_are_refused_in_the_symbolic_records() {
    use qcore_schemas::composition_summary::UnsupportedCloseCompositionRecord;
    let mut value = unsupported_close_projection();
    value["extra"] = json!(true);
    assert!(
        serde_json::from_value::<CompositionSourceInput>(value).is_err(),
        "the projection envelope denies unknown fields"
    );

    let mut value = unsupported_close_projection();
    value["prerequisites"]["unsupported_close"]["extra"] = json!(true);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    // The nested question's own `deny_unknown_fields` records still refuse,
    // through this family's box.
    let mut value = unsupported_close_projection();
    value["family"]["filesystem"]["derivation"]["extra"] = json!(true);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    // `P04-COMPOSE-TAGGED-UNKNOWN`, pinned in both places: the key beside the
    // tag is dropped, and the parsed value is exactly the honest one, so nothing
    // it carried survives into anything the core digests.
    let mut value = unsupported_close_projection();
    value["family"]["extra"] = json!(true);
    let parsed: CompositionSourceInput = serde_json::from_value(value)
        .expect("an internally-tagged variant drops a key beside its tag");
    assert_eq!(parsed, unsupported_close_input(), "the key was dropped");
    let mut value = filesystem_close_projection();
    value["family"]["extra"] = json!(true);
    let parsed: CompositionSourceInput = serde_json::from_value(value)
        .expect("the same is true of the filesystem family, and every other");
    assert_eq!(parsed, filesystem_close_input());

    let record = json!({
        "function": "cio_chunk_close",
        "source_revision": "0".repeat(40),
        "signature": "void (struct cio_chunk *, int)",
        "memory_kind": 1,
        "filesystem_kind": 0,
        "source_path": 3,
        "profile_assumptions": {},
        "status": "Safe"
    });
    assert!(
        serde_json::from_value::<UnsupportedCloseCompositionRecord>(record).is_err(),
        "a conclusion smuggled into the proposal record was accepted"
    );
}
