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

//! Card `P04-B1`: the cleanup-outcome contracts' declared referent vocabulary.
//!
//! Two directions are checked here, and they are different claims:
//!
//! * **Declaration → checker.** A profile that is not exactly the vocabulary
//!   its own contract declares is refused before any path is replayed, and a
//!   declaration the closed grammar cannot express is refused whole.
//! * **Declaration → pinned source.** A declaration naming a field, C type,
//!   syscall or helper the pinned source does not bind is refused by the
//!   existing replay, over real Clang-imported C. That is what makes the
//!   declaration checkable rather than free prose.
//!
//! Nothing here establishes an OS effect. A declared `close`/`unlink` name is a
//! name the source must really call; whether a descriptor was released stays
//! outside every claim in this file, exactly as before.
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_verification::cleanup_outcome::{
    CleanupField, CleanupProfile, ResetValue, check_cleanup_outcomes,
};
use qcore_verification::cleanup_vocabulary::{
    CleanupVocabularyShape, MARKER, declared_cleanup_vocabulary,
};

const OUTCOMES: &str = include_str!(
    "../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv"
);
const DELETE: &str = include_str!(
    "../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete.qv"
);

fn field(name: &str, c_type: &str, reset: ResetValue) -> CleanupField {
    CleanupField {
        name: name.into(),
        c_type: c_type.into(),
        reset,
    }
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}

// ---------------------------------------------------------------------------
// 1. What the pinned contracts now declare, exactly.
// ---------------------------------------------------------------------------

/// The three policies' declarations, read back and pinned literally. This is
/// the referent the P04-NEXT-AB census measured as absent; if a future
/// revision changes it, that is a deliberate edit, not a silent drift.
#[test]
fn the_pinned_contracts_declare_the_profiles_the_checker_binds() {
    let close = declared_cleanup_vocabulary(OUTCOMES, "native_close_outcomes").unwrap();
    assert_eq!(
        *close.shape(),
        CleanupVocabularyShape {
            function: "cio_file_native_close".into(),
            root_type: "struct cio_file *".into(),
            syscall: "close".into(),
            syscall_signature: "int (int)".into(),
            presence_field: "fd".into(),
            fields: vec![field("fd", "int", ResetValue::Integer(-1))],
            argument_fields: names(&["fd"]),
            mapping_field: None,
            errno_function: "__errno_location".into(),
            report_function: "cio_errno_print".into(),
        }
    );

    let unmap = declared_cleanup_vocabulary(OUTCOMES, "native_unmap_outcomes").unwrap();
    assert_eq!(
        *unmap.shape(),
        CleanupVocabularyShape {
            function: "cio_file_native_unmap".into(),
            root_type: "struct cio_file *".into(),
            syscall: "munmap".into(),
            syscall_signature: "int (void *, size_t)".into(),
            presence_field: "map".into(),
            fields: vec![
                field("map", "char *", ResetValue::Null),
                field("alloc_size", "unsigned long", ResetValue::Integer(0)),
                field("map_truncated_warned", "int", ResetValue::Integer(0)),
            ],
            argument_fields: names(&["map", "alloc_size"]),
            mapping_field: None,
            errno_function: "__errno_location".into(),
            report_function: "cio_errno_print".into(),
        }
    );

    let delete = declared_cleanup_vocabulary(DELETE, "native_delete_outcomes").unwrap();
    assert_eq!(
        *delete.shape(),
        CleanupVocabularyShape {
            function: "cio_file_native_delete".into(),
            root_type: "struct cio_file *".into(),
            syscall: "unlink".into(),
            syscall_signature: "int (const char *)".into(),
            presence_field: "fd".into(),
            fields: vec![
                field("fd", "int", ResetValue::Integer(-1)),
                field("map", "char *", ResetValue::Null),
                field("path", "char *", ResetValue::Null),
            ],
            argument_fields: names(&["path"]),
            // Only the guarded-delete policy declares an eligibility mapping
            // field; the two above must not, and do not.
            mapping_field: Some("map".into()),
            errno_function: "__errno_location".into(),
            report_function: "cio_errno_print".into(),
        }
    );

    // The declaration really is the checker's referent: every clause it names
    // is supplied as an observation, marker included.
    assert!(delete.names().contains(MARKER));
    assert!(
        delete
            .names()
            .contains("cleanup_errno_helper____errno_location")
    );
    assert_eq!(delete.facts().len(), delete.names().len());
    assert!(delete.facts().iter().all(|(_, value)| *value == 1));
}

// ---------------------------------------------------------------------------
// 2. Declaration -> checker: a profile that is not the declaration is refused.
// ---------------------------------------------------------------------------

fn delete_profile() -> CleanupProfile {
    CleanupProfile {
        function: "cio_file_native_delete".into(),
        root_type: "struct cio_file *".into(),
        syscall: "unlink".into(),
        syscall_signature: "int (const char *)".into(),
        presence_field: "fd".into(),
        fields: vec![
            field("fd", "int", ResetValue::Integer(-1)),
            field("map", "char *", ResetValue::Null),
            field("path", "char *", ResetValue::Null),
        ],
        argument_fields: names(&["path"]),
        policy: "native_delete_outcomes".into(),
        errno_function: "__errno_location".into(),
        report_function: "cio_errno_print".into(),
    }
}

/// Every member of the transported profile is load-bearing in the binding,
/// including the reset values that assert what a successful syscall leaves
/// behind and the *order* of the syscall arguments.
#[test]
fn a_profile_that_is_not_the_declaration_is_refused_member_by_member() {
    let declared = declared_cleanup_vocabulary(DELETE, "native_delete_outcomes").unwrap();
    assert!(declared.bind(&delete_profile(), Some("map")).is_ok());

    #[allow(clippy::type_complexity)]
    let mutations: Vec<(&str, Box<dyn Fn(&mut CleanupProfile)>)> = vec![
        (
            "function",
            Box::new(|p: &mut CleanupProfile| p.function = "cio_file_native_close".into()),
        ),
        (
            "root type",
            Box::new(|p: &mut CleanupProfile| p.root_type = "struct cio_chunk *".into()),
        ),
        (
            "syscall",
            Box::new(|p: &mut CleanupProfile| p.syscall = "remove".into()),
        ),
        (
            "syscall signature",
            Box::new(|p: &mut CleanupProfile| p.syscall_signature = "int (char *)".into()),
        ),
        (
            "presence field",
            Box::new(|p: &mut CleanupProfile| p.presence_field = "map".into()),
        ),
        (
            "renamed field",
            Box::new(|p: &mut CleanupProfile| p.fields[0].name = "descriptor".into()),
        ),
        (
            "retyped field",
            Box::new(|p: &mut CleanupProfile| p.fields[0].c_type = "unsigned long".into()),
        ),
        (
            "changed reset value",
            Box::new(|p: &mut CleanupProfile| p.fields[0].reset = ResetValue::Integer(0)),
        ),
        (
            "nulled reset value",
            Box::new(|p: &mut CleanupProfile| p.fields[1].reset = ResetValue::Integer(0)),
        ),
        (
            "reordered fields",
            Box::new(|p: &mut CleanupProfile| p.fields.swap(0, 1)),
        ),
        (
            "duplicated field",
            Box::new(|p: &mut CleanupProfile| p.fields.push(p.fields[0].clone())),
        ),
        (
            "widened inventory",
            Box::new(|p: &mut CleanupProfile| {
                p.fields.push(field("extra", "int", ResetValue::Integer(0)))
            }),
        ),
        (
            "narrowed inventory",
            Box::new(|p: &mut CleanupProfile| {
                p.fields.pop();
            }),
        ),
        (
            "renamed argument",
            Box::new(|p: &mut CleanupProfile| p.argument_fields[0] = "map".into()),
        ),
        (
            "widened arguments",
            Box::new(|p: &mut CleanupProfile| p.argument_fields.push("fd".into())),
        ),
        (
            "errno helper",
            Box::new(|p: &mut CleanupProfile| p.errno_function = "errno_location".into()),
        ),
        (
            "report helper",
            Box::new(|p: &mut CleanupProfile| p.report_function = "cio_print".into()),
        ),
    ];
    let applied = mutations.len();
    for (label, mutate) in mutations {
        let mut profile = delete_profile();
        mutate(&mut profile);
        assert_ne!(profile, delete_profile(), "{label} must really differ");
        assert!(
            declared.bind(&profile, Some("map")).is_err(),
            "{label} was admitted against the declaration"
        );
    }
    assert_eq!(applied, 17, "profile mutation row count");

    // The eligibility field is bound in both directions: the delete policy
    // cannot be read without one, and a different one is refused.
    assert!(declared.bind(&delete_profile(), None).is_err());
    assert!(declared.bind(&delete_profile(), Some("path")).is_err());
    // ... and the close policy, which declares none, refuses one.
    let close = declared_cleanup_vocabulary(OUTCOMES, "native_close_outcomes").unwrap();
    let close_profile = CleanupProfile {
        function: "cio_file_native_close".into(),
        root_type: "struct cio_file *".into(),
        syscall: "close".into(),
        syscall_signature: "int (int)".into(),
        presence_field: "fd".into(),
        fields: vec![field("fd", "int", ResetValue::Integer(-1))],
        argument_fields: names(&["fd"]),
        policy: "native_close_outcomes".into(),
        errno_function: "__errno_location".into(),
        report_function: "cio_errno_print".into(),
    };
    assert!(close.bind(&close_profile, None).is_ok());
    assert!(close.bind(&close_profile, Some("map")).is_err());
    // A profile whose C type has no spelling in the closed grammar has no
    // declaration at all, rather than an approximated one.
    let mut exotic = close_profile.clone();
    exotic.fields[0].c_type = "int (*)(void)".into();
    assert!(close.bind(&exotic, None).is_err());
}

// ---------------------------------------------------------------------------
// 3. Declarations the closed grammar refuses whole.
// ---------------------------------------------------------------------------

/// Total or reject, in the loader's idiom. Each row edits the real pinned
/// contract text and must be refused with no partial interpretation.
/// Remove clauses without matching checkout-dependent line endings.
#[test]
fn a_declaration_outside_the_closed_grammar_is_refused_whole() {
    let rows: Vec<(&str, String)> = vec![
        (
            "marker removed",
            DELETE.replace("    requires cleanup_vocabulary_v1;", ""),
        ),
        (
            "duplicated clause",
            DELETE.replace(
                "    requires cleanup_syscall__unlink;",
                "    requires cleanup_syscall__unlink;\n    requires cleanup_syscall__unlink;",
            ),
        ),
        (
            "two syscalls declared",
            DELETE.replace(
                "    requires cleanup_syscall__unlink;",
                "    requires cleanup_syscall__unlink;\n    requires cleanup_syscall__remove;",
            ),
        ),
        (
            "unknown cleanup clause",
            DELETE.replace(
                "    requires cleanup_syscall__unlink;",
                "    requires cleanup_syscall__unlink;\n    requires cleanup_invented_item__x;",
            ),
        ),
        (
            "field index gap",
            DELETE.replace("cleanup_field__1__map", "cleanup_field__7__map"),
        ),
        (
            "duplicate field index",
            DELETE.replace("cleanup_field__2__path", "cleanup_field__1__path"),
        ),
        (
            "non-canonical index",
            DELETE.replace("cleanup_field__1__map", "cleanup_field__01__map"),
        ),
        (
            "field without an index",
            DELETE.replace("cleanup_field__0__fd__int__integer_m1", "cleanup_field__fd"),
        ),
        (
            "unknown C type token",
            DELETE.replace(
                "cleanup_field__1__map__char_ptr__null",
                "cleanup_field__1__map__widget__null",
            ),
        ),
        (
            "unknown reset token",
            DELETE.replace("__int__integer_m1", "__int__whatever"),
        ),
        (
            "reset out of range",
            DELETE.replace("__int__integer_m1", "__int__integer_99999999999"),
        ),
        (
            "unknown result type in a signature",
            DELETE.replace(
                "cleanup_syscall_signature__int__const_char_ptr",
                "cleanup_syscall_signature__widget__const_char_ptr",
            ),
        ),
        (
            "empty payload",
            DELETE.replace("cleanup_syscall__unlink", "cleanup_syscall__"),
        ),
        (
            "root type missing",
            DELETE.replace("    requires cleanup_root_type__struct_cio_file_ptr;", ""),
        ),
        (
            "function missing",
            DELETE.replace("    requires cleanup_function__cio_file_native_delete;", ""),
        ),
        (
            "presence field missing",
            DELETE.replace("    requires cleanup_presence_field__fd;", ""),
        ),
        (
            "report helper missing",
            DELETE.replace("    requires cleanup_report_helper__cio_errno_print;", ""),
        ),
        (
            "no field inventory",
            DELETE
                .replace("    requires cleanup_field__0__fd__int__integer_m1;", "")
                .replace("    requires cleanup_field__1__map__char_ptr__null;", "")
                .replace("    requires cleanup_field__2__path__char_ptr__null;", ""),
        ),
        (
            "no arguments",
            DELETE.replace("    requires cleanup_argument_field__0__path;", ""),
        ),
        (
            "record tag with a separator",
            DELETE.replace("struct_cio_file_ptr", "struct_cio__file_ptr"),
        ),
    ];
    let applied = rows.len();
    for (label, contract) in rows {
        assert_ne!(contract, DELETE, "{label} must really edit the contract");
        assert!(
            declared_cleanup_vocabulary(&contract, "native_delete_outcomes").is_err(),
            "{label} was admitted"
        );
    }
    assert_eq!(applied, 20, "declaration mutation row count");

    // A contract with no such declaration at all — every earlier revision of
    // these two files — is refused, not defaulted to the caller's profile.
    let bare = "module contract;\nextern fn native_delete_outcomes() -> unit\ncontract {\n    requires nonnull_live_exclusive_record;\n    ensures normal fields_preserved;\n};\n";
    assert!(declared_cleanup_vocabulary(bare, "native_delete_outcomes").is_err());
    // A policy the contract does not declare at all is refused by name.
    assert!(declared_cleanup_vocabulary(DELETE, "native_close_outcomes").is_err());
}

// ---------------------------------------------------------------------------
// 4. Declaration -> pinned source, over real Clang-imported C.
// ---------------------------------------------------------------------------

/// Held-out C, unrelated to ChunkIO, in the two shapes the family binds: a
/// one-field descriptor release and a three-field mapping release with two
/// ordered syscall arguments.
/// The declared field and synthetic syscall both use `unsigned long`, avoiding
/// a target-dependent `size_t` alias or an unmodeled integral conversion.
const SAMPLE: &str = r#"
struct sample_file { int handle; char *view; unsigned long view_bytes; int warned; int spare; };
int sample_release(int);
int sample_unmap(void *, unsigned long);
int *sample_errno(void);
int sample_report(int, const char *, int);
#define SAMPLE_REPORT sample_report(*sample_errno(), "sample.c", 7)
int sample_file_release(struct sample_file *sf) {
    if (!sf) return -1;
    if (sf->handle != -1) {
        int result = sample_release(sf->handle);
        if (result == -1) { SAMPLE_REPORT; return -1; }
        sf->handle = -1;
    }
    return 0;
}
int sample_file_unmap(struct sample_file *sf) {
    if (!sf) return -1;
    if (!sf->view) return 0;
    int result = sample_unmap(sf->view, sf->view_bytes);
    if (result != 0) { SAMPLE_REPORT; return -1; }
    sf->view_bytes = 0; sf->view = (void *)0; sf->warned = 0;
    return 0;
}
"#;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent cleanup vocabulary test: {reason}");
            false
        }
    }
}

fn import(function: &str) -> SourceSummary {
    let result = import_source_functions(
        &C11TranslationUnit {
            source: SAMPLE,
            ..Default::default()
        },
        "declared-sample-revision",
        &[function],
        &SourceImportProfile {
            records: &["sample_file"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap()
    .remove(0);
    let SourceResult::Summary(summary) = result else {
        panic!("{result:?}");
    };
    summary
}

/// The declaration and the profile move together in every row below, so what
/// is measured is whether the *declared* referent is bound by the pinned
/// source — not whether the caller's profile matches the contract, which §2
/// covers separately.
fn sample_contract(clauses: &str) -> String {
    format!(
        "module contract;\nextern fn sample_release_outcomes() -> unit\ncontract {{\n\
         {clauses}\
             ensures normal not was_null or returned == -1;\n\
             ensures normal was_null or was_present or returned == 0;\n\
             ensures normal not syscall_failed or returned == -1;\n\
             ensures normal not syscall_succeeded or returned == 0;\n\
             ensures normal cleanup_calls == was_present;\n\
             ensures normal syscall_succeeded or field_writes == 0;\n\
             ensures normal not syscall_succeeded or field_writes == reset_field_count;\n\
             ensures normal syscall_succeeded or fields_preserved;\n\
             ensures normal not syscall_succeeded or fields_reset;\n\
             ensures normal root_preserved;\n\
             ensures normal other_fields_preserved;\n}};\n"
    )
}

const RELEASE_CLAUSES: &str = "    requires nullable_live_exclusive_record;
    requires present_field_has_valid_syscall_argument_identity;
    requires syscall_returns_zero_or_minus_one_and_frames_record;
    requires diagnostics_return_and_frame_record;
    requires source_and_abi_bindings;
    requires cleanup_vocabulary_v1;
    requires cleanup_function__sample_file_release;
    requires cleanup_root_type__struct_sample_file_ptr;
    requires cleanup_syscall__sample_release;
    requires cleanup_syscall_signature__int__int;
    requires cleanup_presence_field__handle;
    requires cleanup_field__0__handle__int__integer_m1;
    requires cleanup_argument_field__0__handle;
    requires cleanup_errno_helper__sample_errno;
    requires cleanup_report_helper__sample_report;
";

const UNMAP_CLAUSES: &str = "    requires nullable_live_exclusive_record;
    requires present_field_has_valid_syscall_argument_identity;
    requires syscall_returns_zero_or_minus_one_and_frames_record;
    requires diagnostics_return_and_frame_record;
    requires source_and_abi_bindings;
    requires cleanup_vocabulary_v1;
    requires cleanup_function__sample_file_unmap;
    requires cleanup_root_type__struct_sample_file_ptr;
    requires cleanup_syscall__sample_unmap;
    requires cleanup_syscall_signature__int__void_ptr__unsigned_long;
    requires cleanup_presence_field__view;
    requires cleanup_field__0__view__char_ptr__null;
    requires cleanup_field__1__view_bytes__unsigned_long__integer_0;
    requires cleanup_field__2__warned__int__integer_0;
    requires cleanup_argument_field__0__view;
    requires cleanup_argument_field__1__view_bytes;
    requires cleanup_errno_helper__sample_errno;
    requires cleanup_report_helper__sample_report;
";

/// Derive the profile straight from the declaration, so nothing in these rows
/// is hand-kept in step with the contract text.
fn profile_from(contract: &str) -> CleanupProfile {
    let shape = declared_cleanup_vocabulary(contract, "sample_release_outcomes")
        .unwrap()
        .shape()
        .clone();
    CleanupProfile {
        function: shape.function,
        root_type: shape.root_type,
        syscall: shape.syscall,
        syscall_signature: shape.syscall_signature,
        presence_field: shape.presence_field,
        fields: shape.fields,
        argument_fields: shape.argument_fields,
        policy: "sample_release_outcomes".into(),
        errno_function: shape.errno_function,
        report_function: shape.report_function,
    }
}

#[test]
fn a_declaration_the_pinned_source_does_not_bind_is_refused() {
    if !clang_available() {
        return;
    }
    let release = import("sample_file_release");
    let unmap = import("sample_file_unmap");

    // Positive control, both shapes: the declared vocabulary really is what
    // this source binds, over all four entry/outcome cases.
    let release_contract = sample_contract(RELEASE_CLAUSES);
    let certificate = check_cleanup_outcomes(
        &release,
        &profile_from(&release_contract),
        &release_contract,
    )
    .unwrap();
    assert_eq!(certificate.cases().len(), 4);
    assert_eq!(certificate.cases()[3].field_writes, ["handle"]);
    let unmap_contract = sample_contract(UNMAP_CLAUSES);
    let unmap_certificate =
        check_cleanup_outcomes(&unmap, &profile_from(&unmap_contract), &unmap_contract).unwrap();
    assert_eq!(unmap_certificate.cases().len(), 4);
    assert_eq!(
        unmap_certificate.cases()[3].field_writes,
        ["view_bytes", "view", "warned"]
    );

    // Every row names something the source does not bind. The profile is
    // re-derived from the edited declaration, so the binding of §2 is
    // satisfied and only the source can refuse.
    let rows: Vec<(&str, &SourceSummary, String)> = vec![
        (
            "a field absent from the record",
            &release,
            RELEASE_CLAUSES.replace("__0__handle__int", "__0__absent_field__int"),
        ),
        (
            "a present field at the wrong C type",
            &release,
            RELEASE_CLAUSES.replace("__0__handle__int__integer_m1", "__0__handle__char_ptr__null"),
        ),
        (
            "a syscall the body never calls",
            &release,
            RELEASE_CLAUSES.replace("cleanup_syscall__sample_release", "cleanup_syscall__sample_absent"),
        ),
        (
            "the syscall at the wrong signature",
            &release,
            RELEASE_CLAUSES.replace(
                "cleanup_syscall_signature__int__int",
                "cleanup_syscall_signature__int__void_ptr",
            ),
        ),
        (
            "an errno helper the body never calls",
            &release,
            RELEASE_CLAUSES.replace("cleanup_errno_helper__sample_errno", "cleanup_errno_helper__sample_absent"),
        ),
        (
            "a diagnostic helper the body never calls",
            &release,
            RELEASE_CLAUSES.replace(
                "cleanup_report_helper__sample_report",
                "cleanup_report_helper__sample_absent",
            ),
        ),
        (
            "another function's name",
            &release,
            RELEASE_CLAUSES.replace(
                "cleanup_function__sample_file_release",
                "cleanup_function__sample_file_unmap",
            ),
        ),
        (
            "a record type the source does not use",
            &release,
            RELEASE_CLAUSES.replace("struct_sample_file_ptr", "struct_sample_other_ptr"),
        ),
        (
            "an eligibility field the body does not test",
            &release,
            RELEASE_CLAUSES.replace("cleanup_presence_field__handle", "cleanup_presence_field__absent_field")
                .replace("__0__handle__int__integer_m1", "__0__absent_field__int__integer_m1")
                .replace("cleanup_argument_field__0__handle", "cleanup_argument_field__0__absent_field"),
        ),
        (
            // The inventory cannot be widened past what the body resets: the
            // success case would then write fewer fields than declared.
            "an inventory wider than the body resets",
            &release,
            RELEASE_CLAUSES.replace(
                "    requires cleanup_argument_field__0__handle;",
                "    requires cleanup_field__1__spare__int__integer_0;\n    requires cleanup_argument_field__0__handle;",
            ),
        ),
        (
            "an inventory narrower than the body resets",
            &unmap,
            UNMAP_CLAUSES.replace("    requires cleanup_field__2__warned__int__integer_0;\n", ""),
        ),
        (
            "the syscall arguments in the wrong order",
            &unmap,
            UNMAP_CLAUSES
                .replace("cleanup_argument_field__0__view;", "cleanup_argument_field__0__view_bytes;")
                .replace("cleanup_argument_field__1__view_bytes;", "cleanup_argument_field__1__view;"),
        ),
        (
            "a reset value the body does not write",
            &unmap,
            UNMAP_CLAUSES.replace("__1__view_bytes__unsigned_long__integer_0", "__1__view_bytes__unsigned_long__integer_5"),
        ),
    ];
    let applied = rows.len();
    for (label, summary, clauses) in rows {
        let contract = sample_contract(&clauses);
        assert_ne!(
            contract,
            sample_contract(if std::ptr::eq(summary, &release) {
                RELEASE_CLAUSES
            } else {
                UNMAP_CLAUSES
            }),
            "{label} must really edit the declaration"
        );
        let profile = profile_from(&contract);
        assert!(
            check_cleanup_outcomes(summary, &profile, &contract).is_err(),
            "{label} was admitted against the pinned source"
        );
    }
    assert_eq!(applied, 13, "source-binding row count");
}

/// A vocabulary clause the reader does not recognise cannot be smuggled into a
/// policy: the predicate interpreter has no value for it and refuses. This is
/// what ties the declaration to the checker rather than leaving it decorative.
#[test]
fn an_unsupplied_premise_refuses_rather_than_being_ignored() {
    if !clang_available() {
        return;
    }
    let release = import("sample_file_release");
    let contract = sample_contract(&format!(
        "{RELEASE_CLAUSES}    requires some_undeclared_premise;\n"
    ));
    let profile = profile_from(&contract);
    let error = check_cleanup_outcomes(&release, &profile, &contract).unwrap_err();
    assert!(error.contains("missing observation"), "{error}");
}
