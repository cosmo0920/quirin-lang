use std::collections::BTreeMap;
use std::path::PathBuf;

use qcore_compiler::contract_loader::{
    check_source_assumptions, load_resource_contract_files, load_resource_contract_manifest,
    load_resource_contract_sources,
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
