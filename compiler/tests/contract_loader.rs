use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qcore_compiler::contract_loader::{
    POINTER_ABI_TYPES, SCALAR_ABI_TYPES, check_source_assumptions, load_resource_contract_files,
    load_resource_contract_manifest, load_resource_contract_sources,
};
use qcore_compiler::{
    AnalysisStatus, CCompilerDialect, ContractTrust, LegacyFunctionScope, LegacySafetyCertificate,
    ResourceApiKind, ResourceContracts, ResourceFunctionContract, ResourceParameterMode as P,
    ResourceReturnMode as R, ResourceSummaryRecord, analyze_legacy_c_translation_unit,
    check_legacy_safety_certificate,
};

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract/cycle19-resource-contracts.json")
}

fn load(source: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("test.qcore".into(), source.into())]))
}

const ALLOC: &str = "module contract;
extern fn alloc() -> ptr<mut, u8> contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};";

#[test]
fn production_contracts_preserve_the_previous_resource_modes() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    assert_eq!(contracts.source_assumptions().len(), 5);
    check_source_assumptions(&contracts).unwrap();
    let mut old = ResourceContracts::default();
    for name in [
        "malloc",
        "calloc",
        "mk_mem_alloc",
        "mk_mem_alloc_z",
        "rd_malloc",
        "rd_calloc",
        "rd_strdup",
        "rd_strndup",
        "flb_malloc",
        "flb_calloc",
        "flb_sds_create_len",
        "cio_chunk_open",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([], R::Owned { nullable: true }),
        );
    }
    for name in [
        "free",
        "mk_mem_free",
        "rd_free",
        "flb_free",
        "flb_sds_destroy",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: true }], R::None),
        );
    }
    old.insert("cio_chunk_close", ResourceApiKind::Release);
    old.insert_contract(
        "rd_realloc",
        ResourceFunctionContract::new(
            [P::Owned { nullable: true }, P::Borrowed],
            R::Reallocated {
                parameter: 0,
                nullable: true,
            },
        ),
    );
    // Explicitly borrowed trailing arguments were implicit in the old table.
    fn canonical(mut contract: ResourceFunctionContract) -> ResourceFunctionContract {
        while contract.parameters.last() == Some(&P::Borrowed) {
            contract.parameters.pop();
        }
        contract
    }
    for source in contracts.source_assumptions().values() {
        for declaration in qcore_compiler::parse_source_program(source)
            .unwrap()
            .declarations
        {
            let name = declaration.value.name;
            assert_eq!(
                canonical(contracts.contract(&name).unwrap().clone()),
                canonical(old.contract(&name).unwrap().clone()),
                "{name}"
            );
        }
    }
}

#[test]
fn loaded_contract_changes_drive_custody_verdicts() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    for (source, expected) in [
        (
            "void test(void) { void *p = malloc(8); if (p) free(p); }",
            AnalysisStatus::Safe,
        ),
        (
            "void test(void) { void *p = malloc(8); }",
            AnalysisStatus::Unsafe,
        ),
        (
            "void test(void) { void *p = malloc(8); if (p) { free(p); free(p); } }",
            AnalysisStatus::Unsafe,
        ),
        ("void test(void) { absent(); }", AnalysisStatus::Unknown),
        (
            "void test(void) { void *p = rd_malloc(8); void *q = rd_realloc(p, 16); if (q) { rd_free(q); } else { rd_free(p); } }",
            AnalysisStatus::Safe,
        ),
        (
            "void test(void) { void *p = rd_malloc(8); p = rd_realloc(p, 16); rd_free(p); }",
            AnalysisStatus::Unsafe,
        ),
    ] {
        assert_eq!(
            analyze_legacy_c_translation_unit(source, "test", &contracts)
                .unwrap()
                .status,
            expected,
            "{source}"
        );
    }
    let source = "void test(void) { void *p = alloc(); }";
    assert_eq!(
        analyze_legacy_c_translation_unit(source, "test", &load(ALLOC).unwrap())
            .unwrap()
            .status,
        AnalysisStatus::Unsafe
    );
    let neutral = ALLOC.replace("resource_return_owned(nullable)", "resource_return_none()");
    assert_eq!(
        analyze_legacy_c_translation_unit(source, "test", &load(&neutral).unwrap())
            .unwrap()
            .status,
        AnalysisStatus::Safe
    );
}

#[test]
fn unsupported_or_incomplete_contracts_fail_atomically() {
    for bad in [
        ALLOC.replace("resource_contract_v1", "resource_contract_v2"),
        ALLOC.replace(
            "requires resource_contract_v1;",
            "requires resource_contract_v1; requires size > 0;",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "result != error_sentinel",
        ),
        ALLOC.replace("ensures normal", "ensures error Failed"),
        ALLOC.replace("ensures normal", "ensures any"),
        ALLOC.replace("ensures normal", "quant normal"),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(nullable,)",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(nullable) and hidden()",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(maybe)",
        ),
        ALLOC.replace("requires resource_contract_v1;", ""),
        ALLOC.replace("ensures normal resource_return_owned(nullable);", ""),
        ALLOC.replace("ptr<mut, u8> contract", "usize contract"),
        ALLOC.replace("contract {", "effects { hidden } contract {"),
        ALLOC.replace("extern fn", "fn"),
        ALLOC.replace("module contract;", "module library;"),
        ALLOC.replace("alloc()", "alloc(size: usize)"),
        format!("{ALLOC}\nextern fn other() -> unit;"),
        format!("{ALLOC}\nconst other: usize = 1;"),
        "module contract;".into(),
        String::new(),
    ] {
        assert!(load(&bad).is_err(), "accepted {bad}");
    }
    assert!(load_resource_contract_sources(&BTreeMap::new()).is_err());
    assert!(
        load_resource_contract_files(&[manifest().with_file_name("missing-contract.qcore")])
            .is_err()
    );
}

#[test]
fn duplicate_functions_and_inputs_are_rejected_even_when_identical() {
    let sources = BTreeMap::from([("a".into(), ALLOC.into()), ("b".into(), ALLOC.into())]);
    assert!(
        load_resource_contract_sources(&sources)
            .unwrap_err()
            .contains("duplicate function")
    );
    assert!(
        load(&format!(
            "{ALLOC}{}",
            ALLOC.trim_start_matches("module contract;")
        ))
        .is_err()
    );
    let file = manifest().parent().unwrap().join("qcore/libc/memory.qcore");
    assert!(
        load_resource_contract_files(&[file.clone(), file])
            .unwrap_err()
            .contains("duplicate contract input")
    );
    let conflicting = BTreeMap::from([
        ("a".into(), ALLOC.into()),
        ("b".into(), ALLOC.replace("nullable", "nonnull")),
    ]);
    assert!(load_resource_contract_sources(&conflicting).is_err());
}

#[test]
fn all_parameter_modes_are_explicit_and_indexed_by_declared_names() {
    for (predicate, expected) in [
        ("resource_borrowed(p)", P::Borrowed),
        ("resource_owned(p, nonnull)", P::Owned { nullable: false }),
        (
            "resource_consumed(p, nullable)",
            P::Consumed { nullable: true },
        ),
        (
            "resource_released(p, nullable)",
            P::Released { nullable: true },
        ),
        ("resource_out(p, nonnull)", P::Out { nullable: false }),
        (
            "resource_stored(p, owner, nullable)",
            P::Stored {
                owner_parameter: 0,
                nullable: true,
            },
        ),
        (
            "resource_removed(p, owner, nonnull)",
            P::Removed {
                owner_parameter: 0,
                nullable: false,
            },
        ),
    ] {
        let source = format!(
            "module contract;
        extern fn transfer(owner: ptr<mut, u8>, p: ptr<mut, ptr<mut, u8>>) -> unit contract {{
            requires resource_contract_v1;
            ensures normal resource_owned(owner, nonnull);
            ensures normal {predicate};
            ensures normal resource_return_none();
        }};"
        );
        let contracts = load(&source).unwrap();
        assert_eq!(
            contracts.contract("transfer").unwrap().parameters,
            [P::Owned { nullable: false }, expected]
        );
        assert!(
            load(&source.replace(
                "resource_return_none()",
                "resource_return_none(); ensures normal resource_return_none()"
            ))
            .is_err()
        );
    }
}

#[test]
fn invalid_parameter_bindings_and_types_are_rejected() {
    let source = "module contract;
        extern fn transfer(owner: ptr<mut, u8>, p: ptr<mut, u8>) -> unit contract {
            requires resource_contract_v1;
            ensures normal resource_owned(owner, nonnull);
            ensures normal resource_stored(p, owner, nullable);
            ensures normal resource_return_none();
        };";
    for bad in [
        source.replace("resource_stored(p, owner", "resource_stored(p, p"),
        source.replace("resource_stored(p, owner", "resource_stored(p, missing"),
        source.replace("resource_owned(owner, nonnull)", "resource_borrowed(owner)"),
        source.replace(
            "resource_stored(p, owner, nullable)",
            "resource_out(p, nullable)",
        ),
        source.replace("p: ptr<mut, u8>", "p: usize"),
        source.replace(
            "resource_stored(p, owner, nullable)",
            "resource_borrowed(owner)",
        ),
        source.replace("p: ptr<mut, u8>", "owner: ptr<mut, u8>"),
        source
            .replace("unit contract", "ptr<mut, u8> contract")
            .replace(
                "resource_return_none()",
                "resource_return_reallocated(p, nullable)",
            ),
    ] {
        assert!(load(&bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn certificates_replay_exact_assumptions_and_reject_table_mutation() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    let source = "void test(void) { void *p = malloc(8); free(p); }";
    let summary = analyze_legacy_c_translation_unit(source, "test", &contracts).unwrap();
    let mut certificate = LegacySafetyCertificate {
        preprocessed_source: source.into(),
        entry: "test".into(),
        contracts,
        dialect: CCompilerDialect::C11,
        function_scope: LegacyFunctionScope::PreprocessedTranslationUnit,
        checked_functions: summary.functions.keys().cloned().collect(),
        summary,
        // `test` takes no parameters, releases its own acquisition, and
        // returns nothing, so its validated summary is the neutral contract.
        summaries: vec![ResourceSummaryRecord::new(
            "test",
            ContractTrust::InferredValidated,
            ResourceFunctionContract::default(),
        )],
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    assert_eq!(certificate.contracts.source_assumptions().len(), 5);
    certificate
        .contracts
        .insert("malloc", ResourceApiKind::Neutral);
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("source assumptions")
    );
}

/// Every scalar in the ABI table is an opaque borrowed-only projection: it can
/// be a borrowed argument or a plain result, and every custody mode on it is
/// rejected rather than approximated.
#[test]
fn every_abi_scalar_is_a_borrowed_only_projection() {
    assert_eq!(
        SCALAR_ABI_TYPES,
        [
            "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize"
        ]
    );
    assert_eq!(
        POINTER_ABI_TYPES,
        ["ptr<mut, u8>", "ptr<const, u8>", "ptr<mut, ptr<mut, u8>>"]
    );
    for scalar in SCALAR_ABI_TYPES {
        let argument = format!(
            "module contract;
        extern fn probe(n: {scalar}) -> ptr<mut, u8> contract {{
            requires resource_contract_v1;
            ensures normal resource_borrowed(n);
            ensures normal resource_return_owned(nullable);
        }};"
        );
        let contracts = load(&argument).unwrap_or_else(|e| panic!("{scalar} argument: {e}"));
        assert_eq!(
            contracts.contract("probe").unwrap(),
            &ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
            "{scalar}"
        );
        let result = format!(
            "module contract;
        extern fn probe(p: ptr<mut, u8>) -> {scalar} contract {{
            requires resource_contract_v1;
            ensures normal resource_released(p, nonnull);
            ensures normal resource_return_none();
        }};"
        );
        let contracts = load(&result).unwrap_or_else(|e| panic!("{scalar} result: {e}"));
        assert_eq!(
            contracts.contract("probe").unwrap(),
            &ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
            "{scalar}"
        );
        // A scalar never carries custody, and a non-`None` result needs a
        // pointer return type.
        for bad in [
            argument.replace("resource_borrowed(n)", "resource_owned(n, nonnull)"),
            argument.replace("resource_borrowed(n)", "resource_released(n, nonnull)"),
            argument.replace("resource_borrowed(n)", "resource_out(n, nonnull)"),
            result.replace("resource_return_none()", "resource_return_owned(nullable)"),
        ] {
            assert!(load(&bad).is_err(), "accepted {bad}");
        }
    }
    // Types outside the table stay rejected: `unit` is a result type only, and
    // floats have no resource API in scope.
    for parameter in ["unit", "f32", "f64", "ptr<const, ptr<const, u8>>"] {
        let source = format!(
            "module contract;
        extern fn probe(n: {parameter}) -> unit contract {{
            requires resource_contract_v1;
            ensures normal resource_borrowed(n);
            ensures normal resource_return_none();
        }};"
        );
        assert!(load(&source).is_err(), "accepted parameter {parameter}");
    }
}

/// The `requires resource_contract_v1` marker is the interpreter selector.
/// Asking this loader for a `ResourceContracts` environment over an
/// observation/lifecycle module must fail closed — never silently drop the
/// clauses it cannot check and return a weaker table.
#[test]
fn every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract/qcore");
    let mut modules = Vec::new();
    collect_qcore_sources(&root, &mut modules);
    modules.sort();
    // Includes the runtime-only reopen-failure and integrity observation
    // modules; neither carries the v1 marker, so both are rejected here.
    assert_eq!(modules.len(), 60, "corpus module count");

    let mut selected = 0;
    let mut v1_declarations = 0;
    let mut rejected_declarations = 0;
    for path in &modules {
        let relative = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(path).unwrap();
        let parsed = qcore_compiler::parse_source_program(&source).unwrap();
        let marked = |declaration: &qcore_compiler::SourceDeclaration| {
            declaration.function_contract.as_ref().is_some_and(|c| {
                c.clauses
                    .iter()
                    .any(|clause| clause.value.expression_source.trim() == "resource_contract_v1")
            })
        };
        let all_marked = parsed.declarations.iter().all(|d| marked(&d.value));
        let loaded =
            load_resource_contract_sources(&BTreeMap::from([(relative.clone(), source.clone())]));
        assert_eq!(
            loaded.is_ok(),
            all_marked,
            "{relative}: marker and loadability disagree ({loaded:?})"
        );
        if all_marked {
            selected += 1;
        }
        // Per declaration, so a marked declaration cannot smuggle an unmarked
        // sibling through a module that happens to load.
        for declaration in &parsed.declarations {
            let text = &source[declaration.span.start..declaration.span.end];
            let single =
                BTreeMap::from([(relative.clone(), format!("module contract;\n{text}\n"))]);
            let outcome = load_resource_contract_sources(&single);
            assert_eq!(
                outcome.is_ok(),
                marked(&declaration.value),
                "{relative}: {} marker and loadability disagree ({outcome:?})",
                declaration.value.name
            );
            if marked(&declaration.value) {
                v1_declarations += 1;
            } else {
                rejected_declarations += 1;
            }
        }
    }
    // The five manifest-selected resource projections and their 19 assumptions;
    // every other declaration belongs to another interpreter and is rejected.
    assert_eq!(
        (selected, v1_declarations, rejected_declarations),
        (5, 19, 145)
    );
    assert_eq!(
        load_resource_contract_manifest(&manifest())
            .unwrap()
            .functions()
            .len(),
        v1_declarations
    );

    // The marker is a selector, not a bypass: attaching it to a lifecycle
    // declaration still rejects, because the rest of that declaration is
    // owned by the language front end.
    let lifecycle = std::fs::read_to_string(root.join("chunkio/chunk_destructor.qcore")).unwrap();
    let marked = lifecycle.replace(
        "requires chunk_is_live;",
        "requires resource_contract_v1; requires chunk_is_live;",
    );
    assert_ne!(marked, lifecycle);
    assert!(load(&marked).is_err());
}

fn collect_qcore_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("contract directory is readable") {
        let path = entry.expect("contract entry is readable").path();
        if path.is_dir() {
            collect_qcore_sources(&path, sources);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "qcore")
        {
            sources.push(path);
        }
    }
}

#[test]
fn manifest_selection_rejects_unknown_schema_missing_inputs_and_traversal() {
    let directory =
        std::env::temp_dir().join(format!("qcore-contract-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("selection.json");
    std::fs::write(directory.join("api.qcore"), ALLOC).unwrap();
    for text in [
        r#"{"schema_version":2,"modules":["api.qcore"]}"#,
        r#"{"schema_version":1,"modules":[]}"#,
        r#"{"schema_version":1,"modules":["api.qcore"],"fallback":true}"#,
        r#"{"schema_version":1,"modules":["api.qcore","absent.qcore"]}"#,
        r#"{"schema_version":1,"modules":["../api.qcore"]}"#,
        r#"{"schema_version":1,"modules":["/api.qcore"]}"#,
        r#"{"schema_version":1,"modules":["api.qcore","api.qcore"]}"#,
        "invalid JSON",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(
            load_resource_contract_manifest(&path).is_err(),
            "accepted {text}"
        );
    }
    std::fs::write(&path, r#"{"schema_version":1,"modules":["api.qcore"]}"#).unwrap();
    let loaded = load_resource_contract_manifest(&path).unwrap();
    assert_eq!(loaded.source_assumptions().values().next().unwrap(), ALLOC);
    // Immutable evidence survives deletion of the original selected file.
    std::fs::remove_dir_all(&directory).unwrap();
    check_source_assumptions(&loaded).unwrap();
}
