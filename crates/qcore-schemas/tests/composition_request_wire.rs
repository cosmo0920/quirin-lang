//! Wire compatibility and admission rules for the composition projection — the
//! seventh optional projection on a v2 request.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest, rebuilds the
//! declared prerequisite profile from its own contract, re-establishes every
//! prerequisite and re-runs its own composition before any obligation is
//! covered.
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionSourceInput, UNMAP_WRAPPER_SEMANTIC_PROFILE,
    UnmapWrapperPrerequisites,
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

fn projection() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "prerequisites": {
            "wrapper_call_use": identity("qcore.c_call_result_use.v1", 'a', "rc:0001"),
            "native_cleanup": identity("qcore.c_cleanup_replay.v1", 'b', "rc:0002"),
            "counter": identity("qcore.c_counter_contract.v1", 'c', "rc:0003")
        },
        "family": {
            "family": "unmap_wrapper",
            "wrapper": body("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
            "native": body("cio_file_native_unmap", "int (struct cio_file *)"),
            "native_profile": {
                "function": "cio_file_native_unmap",
                "root_type": "struct cio_file *",
                "syscall": "munmap",
                "syscall_signature": "int (void *, size_t)",
                "presence_field": "map",
                "fields": [{"name":"map","c_type":"char *","reset":{"reset":"null"}}],
                "argument_fields": ["map"],
                "errno_function": "__errno_location",
                "report_function": "cio_errno_print"
            }
        }
    })
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

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    // All six earlier projections and this seventh one serialize away, so an
    // existing request's bytes are unchanged by the 10.0.0 addition.
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
    assert_eq!(
        COMPOSITION_SOURCE_INPUT_KEY,
        "qcore:composition-source-input:v1"
    );
    assert_eq!(
        UNMAP_WRAPPER_SEMANTIC_PROFILE,
        "qcore.c_unmap_wrapper_composition.v1"
    );
}

#[test]
fn a_projection_round_trips_and_carries_no_derived_reading() {
    let input = input();
    assert!(input.version_matches());
    assert_eq!(input.subject_function(), "munmap_file");
    assert!(input.prerequisites.is_complete() && input.prerequisites.are_distinct());
    let text = serde_json::to_string(&input).unwrap();
    for derived in [
        "cases",
        "native_source_path",
        "counter_decrements",
        "wrapper_writes",
        "returned",
        "verdict",
        "certificate",
    ] {
        assert!(!text.contains(derived), "`{derived}` is representable");
    }
    assert_eq!(
        serde_json::from_str::<CompositionSourceInput>(&text).unwrap(),
        input
    );
}

/// Unknown fields are refused rather than ignored, in every nested record a
/// producer could try to widen.
#[test]
fn unknown_fields_are_refused() {
    let mut value = projection();
    value["derived_cases"] = json!([]);
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = projection();
    value["prerequisites"]["extra"] = json!("x");
    assert!(serde_json::from_value::<CompositionSourceInput>(value).is_err());

    let mut value = projection();
    value["family"]["native_profile"]["policy"] = json!("native_unmap_outcomes");
    assert!(
        serde_json::from_value::<CompositionSourceInput>(value).is_err(),
        "the policy name became representable on the wire"
    );
}

/// The request admission rules: exactly one obligation of this projection's own
/// property, for the wrapper it names, never beside another projection, with
/// three complete and distinct prerequisite identities.
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
    // An unsupported version, an incomplete triple and a repeated triple each
    // refuse.
    let mut version = ok.clone();
    version.composition_source.as_mut().unwrap().schema_version = 2;
    assert!(version.validate().is_err());
    let mut incomplete = ok.clone();
    incomplete
        .composition_source
        .as_mut()
        .unwrap()
        .prerequisites
        .native_cleanup
        .obligation_id
        .clear();
    assert!(incomplete.validate().is_err());
    let mut repeated = ok.clone();
    {
        let prerequisites = &mut repeated.composition_source.as_mut().unwrap().prerequisites;
        prerequisites.counter = prerequisites.wrapper_call_use.clone();
    }
    assert!(repeated.validate().is_err());
}

/// The prerequisite triple's distinctness rule is not vacuous.
#[test]
fn the_prerequisite_triple_rules_are_decidable() {
    let complete = input().prerequisites;
    assert!(complete.is_complete() && complete.are_distinct());
    let repeated = UnmapWrapperPrerequisites {
        counter: complete.wrapper_call_use.clone(),
        ..complete.clone()
    };
    assert!(repeated.is_complete() && !repeated.are_distinct());
    let blank = UnmapWrapperPrerequisites {
        counter: qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        },
        ..complete
    };
    assert!(!blank.is_complete());
    assert_ne!(
        UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY,
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY
    );
    assert_ne!(
        UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY,
        UnmapWrapperPrerequisites::COUNTER_KEY
    );
}
