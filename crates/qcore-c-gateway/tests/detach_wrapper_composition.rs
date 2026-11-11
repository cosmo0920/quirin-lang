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

//! Card DETACH-RESULT-WRAPPER-COMPOSITION (gateway rule 33): checked
//! removed-selection frames over the retained production cJSON wrappers, the
//! decimal-constant caller transport and independent source replay.

use qcore_c_gateway::legacy_c::{AnalysisStatus, LegacyProgramSummary, ResourceContracts};

const FRAME: &str = include_str!("fixtures/detach-wrapper-composition/cjson_wrapper_frame.h");
const PRODUCTION_MODULE: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const HELPERS: &str = r#"module contract;
extern fn dt_alloc() -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_return_owned(nonnull);
};
extern fn dt_free(p: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none();
};
extern fn dt_store(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_owned(parent, nonnull);
 ensures normal resource_stored(item, parent, nonnull); ensures normal resource_return_none();
};
extern fn dt_touch(parent: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_borrowed(parent); ensures normal resource_return_none();
};
"#;
const DECLARATIONS: &str = "struct cJSON *dt_alloc(void); void dt_free(struct cJSON*); void dt_store(struct cJSON*,struct cJSON*); void dt_touch(struct cJSON*);";
const REPRESENTATION: &str = "For this removed-result declaration, the live parent represents exactly its owned children by a finite acyclic child and next chain; child is NULL precisely when the chain is empty, each nonfirst child's prev is its predecessor, the first child's prev is the last child, chain child identities are pairwise distinct, every non-NULL next or prev target is a live child of that same parent, the last child's next is NULL, the parent is not a child, and no concurrent writer mutates these links during the call";
const ARRAY_WRAPPERS: [&str; 2] = ["cJSON_DetachItemFromArray", "cJSON_DeleteItemFromArray"];
const UNCHANGED: [&str; 5] = [
    "cJSON_DetachItemFromObject",
    "cJSON_DetachItemFromObjectCaseSensitive",
    "cJSON_DeleteItemFromObject",
    "cJSON_DeleteItemFromObjectCaseSensitive",
    "cJSON_DetachItemViaPointer",
];

fn environment_from(module: &str) -> ResourceContracts {
    qcore_contract::contract_loader::load_resource_contract_sources(
        &std::collections::BTreeMap::from([
            ("cjson.qv".to_owned(), module.to_owned()),
            ("helpers.qv".to_owned(), HELPERS.to_owned()),
        ]),
    )
    .unwrap()
}

fn environment() -> ResourceContracts {
    environment_from(PRODUCTION_MODULE)
}

fn unit(source: &str, entry: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(source, entry, contracts).unwrap()
}

fn status(program: &LegacyProgramSummary, function: &str) -> AnalysisStatus {
    program.functions[function].status
}

#[test]
fn retained_frames_promote_exactly_the_two_array_wrappers() {
    let mut sources = vec![("retained frame".to_owned(), FRAME.to_owned())];
    if let Some(path) = std::env::var_os("QCORE_SELECTION_PRODUCTION_INPUT") {
        let source = std::fs::read_to_string(&path).expect("explicit production source");
        sources.push((format!("explicit production source {path:?}"), source));
    }
    for (label, source) in sources {
        let program = unit(&source, "cJSON_DetachItemFromArray", &environment());
        for wrapper in ARRAY_WRAPPERS {
            let row = &program.functions[wrapper];
            assert_eq!(
                row.status,
                AnalysisStatus::Safe,
                "{label} {wrapper}: {row:?}"
            );
            assert!(row.diagnostics.is_empty(), "{label} {wrapper}");
        }
        for function in UNCHANGED {
            let row = &program.functions[function];
            assert_eq!(row.status, AnalysisStatus::Unknown, "{label} {function}");
            assert!(!row.diagnostics.is_empty(), "{label} {function}");
        }
        for function in [
            "cJSON_DetachItemFromObject",
            "cJSON_DetachItemFromObjectCaseSensitive",
        ] {
            assert!(
                program.functions[function]
                    .diagnostics
                    .iter()
                    .any(|d| format!("{:?}", d.kind).contains(
                        "unmodeled call, aggregate or indirect access in return expression"
                    )),
                "{label} {function}: the string-keyed boundary is unchanged"
            );
        }
        for function in [
            "cJSON_DeleteItemFromObject",
            "cJSON_DeleteItemFromObjectCaseSensitive",
        ] {
            assert!(
                program.functions[function]
                    .diagnostics
                    .iter()
                    .any(|d| format!("{:?}", d.kind).contains("inside a modeled call")),
                "{label} {function}: the nested object detach still refuses"
            );
        }
        // The rows run the frame's own constituent calls: the declared detach
        // (with its named representation premise) and the checked lookup.
        let cfg = &program.functions["cJSON_DeleteItemFromArray"].cfg;
        let callees = qcore_c_gateway::contract_dependencies::cfg_callees(cfg);
        for callee in [
            "get_array_item",
            "cJSON_DetachItemViaPointer",
            "cJSON_Delete",
        ] {
            assert!(callees.contains(callee), "{label}: {callees:?}");
        }
    }
}

#[test]
fn frame_mutations_and_contract_changes_keep_the_old_refusals() {
    let mutations = [
        ("if (which < 0)", "if (which < 1)"),
        ("if (which < 0)", "if (which <= 0)"),
        ("(size_t)which));", "(size_t)which + 1));"),
        (
            "cJSON * cJSON_DetachItemFromArray(cJSON *array, int which)",
            "cJSON * cJSON_DetachItemFromArray(cJSON *array, long which)",
        ),
        (
            "cJSON * cJSON_DetachItemFromArray(cJSON *array, int which)",
            "cJSON * cJSON_DetachItemFromArray(const cJSON *array, int which)",
        ),
        (
            "return cJSON_DetachItemViaPointer(array, get_array_item(array, (size_t)which));",
            "return cJSON_DetachItemViaPointer(array, cJSON_GetArrayItem(array, which));",
        ),
        (
            "cJSON * cJSON_DetachItemFromArray(cJSON *array, int which)",
            "inline cJSON * cJSON_DetachItemFromArray(cJSON *array, int which)",
        ),
        ("index--;", "index++;"),
        ("item->next = ((void*)0);\n    return item;", "return item;"),
        (
            "typedef long unsigned int size_t;",
            "typedef long int size_t;",
        ),
    ];
    for (from, to) in mutations {
        let changed = FRAME.replace(from, to);
        assert_ne!(
            changed, FRAME,
            "control must edit the retained source: {from}"
        );
        let program = unit(&changed, "cJSON_DetachItemFromArray", &environment());
        for wrapper in ARRAY_WRAPPERS {
            assert_ne!(
                status(&program, wrapper),
                AnalysisStatus::Safe,
                "{from} -> {to}: {wrapper}"
            );
        }
    }
    let release_mutations = [
        (
            "cJSON_Delete(cJSON_DetachItemFromArray(array, which));",
            "cJSON_Delete(cJSON_DetachItemFromArray(array, 0));",
        ),
        (
            "cJSON_Delete(cJSON_DetachItemFromArray(array, which));",
            "cJSON_Delete(cJSON_DetachItemFromArray(array, which)); cJSON_Delete(cJSON_DetachItemFromArray(array, which));",
        ),
    ];
    for (from, to) in release_mutations {
        let changed = FRAME.replace(from, to);
        assert_ne!(changed, FRAME);
        let program = unit(&changed, "cJSON_DeleteItemFromArray", &environment());
        assert_ne!(
            status(&program, "cJSON_DeleteItemFromArray"),
            AnalysisStatus::Safe,
            "{to}"
        );
        assert_eq!(
            status(&program, "cJSON_DetachItemFromArray"),
            AnalysisStatus::Safe
        );
    }
    // Minimal declared environments: the positive one promotes both rows;
    // each single change to the detach premise or the release mode refuses.
    let module = |premise: &str, release: &str| {
        format!(
            r#"module contract;
extern fn cJSON_Delete(item: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_released(item, {release}); ensures normal resource_return_none();
}};
extern fn cJSON_DetachItemViaPointer(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 requires resource_premise({premise}, "{REPRESENTATION}");
 ensures normal resource_owned(parent, nonnull); ensures normal resource_removed(item, parent, nullable);
 ensures normal resource_return_removed(item, nullable);
}};
"#
        )
    };
    for (premise, release, promoted) in [
        ("detach_list_representation", "nullable", true),
        ("other_representation", "nullable", false),
        ("detach_list_representation", "nonnull", false),
    ] {
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(
            &std::collections::BTreeMap::from([("cjson.qv".to_owned(), module(premise, release))]),
        )
        .expect("minimal declared environment loads");
        let program = unit(FRAME, "cJSON_DeleteItemFromArray", &contracts);
        let delete = status(&program, "cJSON_DeleteItemFromArray") == AnalysisStatus::Safe;
        let detach = status(&program, "cJSON_DetachItemFromArray") == AnalysisStatus::Safe;
        assert_eq!(delete, promoted, "{premise} {release}");
        assert_eq!(
            detach,
            premise == "detach_list_representation",
            "{premise} {release}"
        );
    }
}

fn subject(body: &str) -> LegacyProgramSummary {
    unit(
        &format!("{FRAME}\n{DECLARATIONS}\nvoid subject(void) {{ {body} }}"),
        "subject",
        &environment(),
    )
}

#[test]
fn decimal_constant_callers_transport_the_existing_identity() {
    for body in [
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *r=cJSON_DetachItemFromArray(p,0); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); struct cJSON *r; dt_store(p,i); p->child=i; r=cJSON_DetachItemFromArray(p,0); dt_free(r); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,0); dt_free(p);",
        "struct cJSON *p=dt_alloc(); p->child=0; cJSON_DeleteItemFromArray(p,0); dt_free(p);",
        "struct cJSON *p=dt_alloc(); p->child=0; struct cJSON *r=cJSON_DetachItemFromArray(p,255); dt_free(r); dt_free(p);",
    ] {
        let program = subject(body);
        assert_eq!(
            status(&program, "subject"),
            AnalysisStatus::Safe,
            "{body}: {program:?}"
        );
    }
}

#[test]
fn every_other_caller_shape_or_missing_fact_refuses() {
    for body in [
        // Nonliteral, negative, octal-looking and out-of-range indices.
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); int k=0; dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,k); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,-1); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,00); dt_free(p);",
        "struct cJSON *p=dt_alloc(); p->child=0; cJSON_DeleteItemFromArray(p,256); dt_free(p);",
        // No physical link, a wrong position, an intervening call.
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); cJSON_DeleteItemFromArray(p,0); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,1); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; dt_touch(p); cJSON_DeleteItemFromArray(p,0); dt_free(p);",
        // The same identity cannot be released twice through an alias.
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *r=cJSON_DetachItemFromArray(p,0); dt_free(p); dt_free(r); dt_free(i);",
        // A dropped detached child is not discharged.
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DetachItemFromArray(p,0); dt_free(p);",
        // Nested, wrong-parent and unknown-parent uses keep their refusals.
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; dt_free(cJSON_DetachItemFromArray(p,0)); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *q=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(q,0); dt_free(p); dt_free(q);",
        "struct cJSON *p; cJSON_DeleteItemFromArray(p,0);",
    ] {
        let program = subject(body);
        assert_ne!(
            status(&program, "subject"),
            AnalysisStatus::Safe,
            "{body}: {program:?}"
        );
    }
}

#[test]
fn a_spelled_reserved_place_disables_every_expansion() {
    let source = format!(
        "{FRAME}\n{DECLARATIONS}\nvoid subject(void) {{ struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,0); dt_free(p); }}\nint __qcore_selection_item_0;"
    );
    let program = unit(&source, "subject", &environment());
    assert_ne!(status(&program, "subject"), AnalysisStatus::Safe);
    for wrapper in ARRAY_WRAPPERS {
        assert_ne!(status(&program, wrapper), AnalysisStatus::Safe, "{wrapper}");
    }
}

#[test]
fn transported_identity_and_frames_replay_independently() {
    use qcore_c_gateway::legacy_c::{
        C11TranslationUnit, LegacyFunctionScope, UnrestrictedLegacyResult,
        analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/detach-wrapper-composition");
    let source = format!(
        "#include \"selection_replay.h\"\n{DECLARATIONS}\nvoid subject(void) {{ struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,0); dt_free(p); }}"
    );
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            include_paths: vec![root.to_string_lossy().into_owned()],
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        },
        "subject",
        &environment(),
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        certificate
            .contracts
            .source_assumptions()
            .values()
            .any(|s| s.contains("detach_list_representation"))
    );
    check_legacy_safety_certificate(&certificate).expect("selection transport must replay");
    for (from, to) in [
        ("if (which < 0)", "if (which < 1)"),
        ("index--", "index++"),
        ("p->child=i;", "p->child=0;"),
    ] {
        let mut changed = certificate.clone();
        changed.preprocessed_source = changed.preprocessed_source.replace(from, to);
        assert_ne!(
            changed.preprocessed_source, certificate.preprocessed_source,
            "{from}"
        );
        assert!(
            check_legacy_safety_certificate(&changed).is_err(),
            "edited source cannot replay the original proof: {from}"
        );
    }
}
