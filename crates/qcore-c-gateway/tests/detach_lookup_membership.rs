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

use qcore_c_gateway::legacy_c::{AnalysisStatus, ResourceContracts, analyze_legacy_c};

const ACTUAL_FRAME: &str = include_str!("fixtures/detach-lookup-membership/cjson_lookup_frame.h");

#[path = "support/retained_frame.rs"]
mod retained_frame;

/// The retained frame as the target's preprocessor prints it.
fn frame() -> String {
    retained_frame::for_target(ACTUAL_FRAME)
}

#[test]
fn production_lookup_baseline_refuses_the_unresolved_returned_cursor() {
    let mut sources = vec![("retained tracked frame".to_owned(), ACTUAL_FRAME.to_owned())];
    if let Some(path) = std::env::var_os("QCORE_LOOKUP_PRODUCTION_INPUT") {
        let source = std::fs::read_to_string(&path)
            .expect("explicit retained production source must be readable");
        sources.push((format!("explicit retained source {path:?}"), source));
    }
    for (label, source) in sources {
        let row =
            analyze_legacy_c(&source, "get_array_item", &ResourceContracts::default()).unwrap();
        println!("{label}: {row:#?}");
        assert_eq!(row.status, AnalysisStatus::Unknown, "{label}: {row:?}");
        assert!(
            row.diagnostics
                .iter()
                .any(|d| format!("{:?}", d.kind).contains("current_child")),
            "{label}: {row:?}"
        );
    }
}

#[test]
fn checked_lookup_unit_reports_borrowed_result_without_fresh_identity() {
    let program = qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        &frame(),
        "get_array_item",
        &ResourceContracts::default(),
    )
    .unwrap();
    println!("checked lookup unit: {program:#?}");
    assert_eq!(
        program.functions["get_array_item"].status,
        AnalysisStatus::Safe
    );
}

const REPRESENTATION: &str = "For this removed-result declaration, the live parent represents exactly its owned children by a finite acyclic child and next chain; child is NULL precisely when the chain is empty, each nonfirst child's prev is its predecessor, the first child's prev is the last child, chain child identities are pairwise distinct, every non-NULL next or prev target is a live child of that same parent, the last child's next is NULL, the parent is not a child, and no concurrent writer mutates these links during the call";
fn environment() -> ResourceContracts {
    qcore_contract::contract_loader::load_resource_contract_sources(&std::collections::BTreeMap::from([
        ("lookup.qv".to_owned(), format!(r#"module contract;
extern fn dt_alloc() -> ptr<mut, u8> contract {{
 requires resource_contract_v1; ensures normal resource_return_owned(nonnull);
}};
extern fn dt_free(p: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none();
}};
extern fn dt_store(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_owned(parent, nonnull);
 ensures normal resource_stored(item, parent, nonnull); ensures normal resource_return_none();
}};
extern fn dt_touch(parent: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_borrowed(parent); ensures normal resource_return_none();
}};
extern fn dt_detach(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 requires resource_premise(detach_list_representation, "{REPRESENTATION}");
 ensures normal resource_owned(parent, nonnull); ensures normal resource_removed(item, parent, nullable);
 ensures normal resource_return_removed(item, nullable);
}};
"#))])).unwrap()
}
const DECLARATIONS: &str = "struct cJSON *dt_alloc(void); void dt_free(struct cJSON*); void dt_store(struct cJSON*,struct cJSON*); void dt_touch(struct cJSON*); struct cJSON *dt_detach(struct cJSON*,struct cJSON*);";
fn subject(body: &str) -> qcore_c_gateway::legacy_c::LegacyProgramSummary {
    qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        &format!(
            "{frame}\n{DECLARATIONS}\nvoid subject(void) {{ {body} }}",
            frame = frame()
        ),
        "subject",
        &environment(),
    )
    .unwrap()
}
#[test]
fn assigned_lookup_uses_existing_head_and_preserves_same_item_transfer() {
    for body in [
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *a=s; struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(a);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *t=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(t);",
        "struct cJSON *p=dt_alloc(); p->child=0; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(r); dt_free(p);",
        "struct cJSON *p=dt_alloc(); p->child=0; struct cJSON *s=get_array_item(p,100); struct cJSON *r=dt_detach(p,s); dt_free(r); dt_free(p);",
    ] {
        let program = subject(body);
        println!("assigned lookup: {program:#?}");
        assert_eq!(
            program.functions["subject"].status,
            AnalysisStatus::Safe,
            "{body}: {program:?}"
        );
    }
}
#[test]
fn provenance_missing_order_wrong_parent_and_intervening_mutation_refuse() {
    for body in [
        "struct cJSON *p=dt_alloc(); p->child=0; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); p->child=0; struct cJSON *s=get_array_item(p,256); struct cJSON *r=dt_detach(p,s); dt_free(r); dt_free(p);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); p->child=0; struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,00); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,-1); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,1); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; dt_touch(p); struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); dt_touch(p); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r);",
        "struct cJSON *p=dt_alloc(); struct cJSON *q=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(q,s); dt_free(p); dt_free(q); dt_free(r);",
    ] {
        let program = subject(body);
        assert_eq!(
            program.functions["subject"].status,
            AnalysisStatus::Unknown,
            "{body}: {program:?}"
        );
    }
}

#[test]
fn differently_typed_same_spelled_link_cannot_supply_lookup_identity() {
    let source = format!(
        "{frame}\nstruct Other {{ struct cJSON *child; }};\n{DECLARATIONS}\nvoid subject(void) {{ void *p=dt_alloc(); struct Other *q=p; struct cJSON *i=dt_alloc(); dt_store(p,i); q->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r); }}",
        frame = frame()
    );
    let program = qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        &source,
        "subject",
        &environment(),
    )
    .unwrap();
    assert_eq!(
        program.functions["subject"].status,
        AnalysisStatus::Unknown,
        "wrong defining record must not establish the callee's physical link: {program:?}"
    );
    assert!(
        program.functions["subject"]
            .cfg
            .nodes
            .iter()
            .all(|node| node.lookup.is_none()),
        "an unresolved caller record must not carry checked lookup metadata"
    );
    let source = format!(
        "{frame}\nstruct Other {{ struct cJSON *child; }};\n{DECLARATIONS}\nvoid subject(void) {{ struct cJSON *p=dt_alloc(); void *v=p; struct Other *q=v; struct cJSON *i=dt_alloc(); dt_store(p,i); q->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r); }}",
        frame = frame()
    );
    let program = qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        &source,
        "subject",
        &environment(),
    )
    .unwrap();
    assert!(
        program.functions["subject"]
            .cfg
            .nodes
            .iter()
            .any(|node| node
                .lookup
                .as_ref()
                .is_some_and(|lookup| lookup.record == "cJSON")),
        "the actual call remains typed; the Other store supplies no cJSON link"
    );
    assert_eq!(program.functions["subject"].status, AnalysisStatus::Unknown);
}

#[test]
fn retained_lookup_identity_and_transfer_replay_independently() {
    use qcore_c_gateway::legacy_c::{
        C11TranslationUnit, LegacyFunctionScope, UnrestrictedLegacyResult,
        analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let root = retained_frame::include_root(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/detach-lookup-membership"),
        &["cjson_lookup_frame.h"],
    );
    let source = format!(
        "#include \"cjson_lookup_frame.h\"\n{DECLARATIONS}\nvoid subject(void) {{ struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; struct cJSON *s=get_array_item(p,0); struct cJSON *r=dt_detach(p,s); dt_free(p); dt_free(r); }}"
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
            .any(|s| s.contains(REPRESENTATION))
    );
    check_legacy_safety_certificate(&certificate).expect("lookup identity source must replay");
    let mut changed = certificate.clone();
    changed.preprocessed_source = changed.preprocessed_source.replace("index--", "index++");
    assert!(
        check_legacy_safety_certificate(&changed).is_err(),
        "stale source cannot replay lookup proof"
    );
    let mut wrong_record = certificate.clone();
    wrong_record.preprocessed_source = wrong_record.preprocessed_source.replace(
        "p->child=i;",
        "{ struct Other { struct cJSON *child; }; struct Other *q=(void*)p; q->child=i; }",
    );
    assert_ne!(
        wrong_record.preprocessed_source,
        certificate.preprocessed_source
    );
    assert!(
        check_legacy_safety_certificate(&wrong_record).is_err(),
        "a different stored-link record cannot replay the original source-bound relation"
    );
}

#[test]
fn signed_array_wrapper_preserves_borrowed_result_and_negative_branch() {
    let source = format!(
        "{frame}\ncJSON *cJSON_GetArrayItem(const cJSON *array, int index) {{ if (index < 0) {{ return ((void*)0); }} return get_array_item(array, (size_t)index); }}",
        frame = frame()
    );
    let program = qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        &source,
        "cJSON_GetArrayItem",
        &ResourceContracts::default(),
    )
    .unwrap();
    assert_eq!(
        program.functions["cJSON_GetArrayItem"].status,
        AnalysisStatus::Safe,
        "{program:?}"
    );
}

#[test]
fn native_exact_lookup_frame_returns_existing_nodes_or_null() {
    let mut clang = match qcore_c_gateway::clang_command() {
        Ok(clang) => clang,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "invalid explicit Clang: {reason}"
            );
            eprintln!("Clang unavailable for native lookup control: {reason}");
            return;
        }
    };
    let root = retained_frame::include_root(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/detach-lookup-membership"),
        &["cjson_lookup_frame.h", "native_case.c"],
    );
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "qcore-lookup-native-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let executable = directory.join(if cfg!(windows) {
        "lookup.exe"
    } else {
        "lookup"
    });
    clang.args(["-std=c11", "-Wall", "-Wextra", "-Werror"]);
    if cfg!(target_os = "linux") {
        clang.args([
            "-O1",
            "-g",
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
        ]);
    }
    let built = clang
        .arg(root.join("native_case.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "native build {directory:?}: {}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = std::process::Command::new(&executable).output().unwrap();
    assert!(
        ran.status.success(),
        "native run {directory:?}: {}\n{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    std::fs::remove_file(&executable).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}
