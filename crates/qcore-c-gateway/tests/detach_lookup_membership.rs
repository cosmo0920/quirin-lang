use qcore_c_gateway::legacy_c::{AnalysisStatus, ResourceContracts, analyze_legacy_c};

const ACTUAL_FRAME: &str = include_str!("fixtures/detach-lookup-membership/cjson_lookup_frame.h");

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
        ACTUAL_FRAME,
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
        &format!("{ACTUAL_FRAME}\n{DECLARATIONS}\nvoid subject(void) {{ {body} }}"),
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
fn retained_lookup_identity_and_transfer_replay_independently() {
    use qcore_c_gateway::legacy_c::{
        C11TranslationUnit, LegacyFunctionScope, UnrestrictedLegacyResult,
        analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/detach-lookup-membership");
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
}

#[test]
fn signed_array_wrapper_preserves_borrowed_result_and_negative_branch() {
    let source = format!(
        "{ACTUAL_FRAME}\ncJSON *cJSON_GetArrayItem(const cJSON *array, int index) {{ if (index < 0) {{ return ((void*)0); }} return get_array_item(array, (size_t)index); }}"
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
