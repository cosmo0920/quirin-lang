//! Card **DECLARED-ASSUMPTION-BODY-CHECKS** (gateway rules 13m, 13n): a W4
//! conflict that only refines a declaration is settled by the declaration's
//! own entry check (13m), recorded as body evidence and nothing else (13n).
//!
//! Every fixture runs the public C11 entry point under the main-source scope
//! with its helper in an included header (a `static inline` definition the
//! selection reaches, as `rd_free` in `rd.h`), inside a [`Recording`]:
//!
//! * **accepting:** a nullable release wrapper (`free(p)` under
//!   `resource_released(p, nullable)`), its guarded forms, the same wrapper
//!   reported as a row (`rd_kafka_mem_free`'s shape), and a nullable-result
//!   wrapper (glibc's `assert` after `malloc`, `rd_malloc`'s shape) conflict
//!   with W4 and are validated by `13m`;
//! * **refusing:** a wrapper that hands its nullable formal to a non-null
//!   release keeps its `conflict` failure; a wrapper that keeps its formal,
//!   one that calls an uncontracted function, one that releases on one path,
//!   a declared alias result the entry check does not establish, and a
//!   declaration with a counted-user clause are never `13m`;
//! * **evidence only:** every row is identical with and without a recording
//!   (the conflicting row keeps its `SummaryConflict`), and the check adds no
//!   consultation to any function.
//!
//! The candidates in these fixtures are fixture findings, not claims about
//! any upstream body.

use std::collections::BTreeSet;

use qcore_c_gateway::contract_dependencies::{BodyOutcome, Recording, UnitRecord};
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, LegacyProgramSummary, ResourceContracts,
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent DECLARED-ASSUMPTION-BODY-CHECKS test: {reason}");
            false
        }
    }
}

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;\n\
void *malloc(size_t size);\n\
void free(void *ptr);\n\
void sink_nonnull(void *ptr);\n\
void opaque(void *ptr);\n\
void *keep;\n";

fn contract(
    parameters: impl IntoIterator<Item = ResourceParameterMode>,
    result: ResourceReturnMode,
) -> ResourceFunctionContract {
    ResourceFunctionContract::new(parameters, result)
}

fn released(nullable: bool) -> ResourceParameterMode {
    ResourceParameterMode::Released { nullable }
}

/// `libc`-shaped declarations, and `w` declared as `declared`.
fn contracts(declared: &ResourceFunctionContract) -> ResourceContracts {
    let mut contracts = ResourceContracts::default();
    contracts.insert_contract(
        "malloc",
        contract(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Owned { nullable: true },
        ),
    );
    contracts.insert_contract("free", contract([released(true)], ResourceReturnMode::None));
    contracts.insert_contract(
        "sink_nonnull",
        contract([released(false)], ResourceReturnMode::None),
    );
    contracts.insert_contract("w", declared.clone());
    contracts
}

struct Run {
    summary: LegacyProgramSummary,
    record: UnitRecord,
}

/// Analyse `main` with `header` included, under the main-source scope, inside
/// a recording when `record` holds.
fn run(tag: &str, header: &str, main: &str, contracts: &ResourceContracts, record: bool) -> Run {
    let directory = std::env::temp_dir().join(format!(
        "qcore-dabc-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("w.h"), format!("{PRELUDE}{header}\n")).expect("the header");
    let source = format!("#include \"w.h\"\n{main}\n");
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let recording = record.then(Recording::start);
    let result = analyze_unrestricted_c11(&unit, "user", contracts)
        .unwrap_or_else(|error| panic!("fixture rejected: {error:?}\n{source}"));
    let record = recording.map(Recording::finish).unwrap_or_default();
    std::fs::remove_dir_all(&directory).expect("cleanup");
    let summary = match result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("expected an analysable unit: {other:?}"),
    };
    Run { summary, record }
}

const USER: &str = "void user(void) { void *p = malloc(4); w(p); }";

fn evidence(record: &UnitRecord, name: &str) -> BTreeSet<String> {
    record.validated.get(name).cloned().unwrap_or_default()
}

#[test]
fn a_nullable_release_wrapper_is_validated_by_its_declared_entry_check() {
    if !clang_available() {
        return;
    }
    let declared = contract([released(true)], ResourceReturnMode::None);
    let header = "static inline void w(void *p) { free(p); }";
    let run = run("free", header, USER, &contracts(&declared), true);
    assert_eq!(
        run.record.body_outcomes().get("w"),
        Some(&BodyOutcome::Validated),
        "{:?}",
        run.record
    );
    assert_eq!(
        evidence(&run.record, "w"),
        BTreeSet::from(["13m".to_owned()])
    );
    assert!(!run.record.failed.contains_key("w"), "{:?}", run.record);
    // `w` is not a row of the unit: only `user` is reported.
    assert_eq!(
        run.summary.functions.keys().collect::<Vec<_>>(),
        ["user"],
        "{:?}",
        run.summary
    );
}

#[test]
fn a_nullable_result_wrapper_is_validated_by_its_declared_entry_check() {
    if !clang_available() {
        return;
    }
    let declared = contract(
        [ResourceParameterMode::Borrowed],
        ResourceReturnMode::Owned { nullable: true },
    );
    let header = "#include <assert.h>\nstatic inline void *w(size_t n) { void *p = malloc(n); assert(p); return p; }";
    let main = "void user(void) { void *p = w(4); free(p); }";
    let run = run("alloc", header, main, &contracts(&declared), true);
    assert_eq!(
        run.record.body_outcomes().get("w"),
        Some(&BodyOutcome::Validated),
        "{:?}",
        run.record
    );
    // W4 reads the result non-null after the assertion; the declaration's
    // nullable result is the weaker statement, which its entry check accepts.
    assert_eq!(
        evidence(&run.record, "w"),
        BTreeSet::from(["13m".to_owned()])
    );
}

#[test]
fn guarded_release_wrappers_and_a_conflicting_row_are_validated() {
    if !clang_available() {
        return;
    }
    let declared = contract([released(true)], ResourceReturnMode::None);
    for (tag, header) in [
        (
            "guard-not",
            "static inline void w(void *p) { if (!p) return; free(p); }",
        ),
        (
            "guard-if",
            "static inline void w(void *p) { if (p) free(p); }",
        ),
    ] {
        let run = run(tag, header, USER, &contracts(&declared), true);
        assert_eq!(
            evidence(&run.record, "w"),
            BTreeSet::from(["13m".to_owned()]),
            "{tag}: {:?}",
            run.record
        );
    }
    // Reported as a row (`rd_kafka_mem_free`'s shape): the row keeps its
    // `SummaryConflict` and stays non-`Safe`; only the evidence moves.
    let main = format!("void w(void *p) {{ free(p); }}\n{USER}");
    let run = run("row", "", &main, &contracts(&declared), true);
    assert_eq!(
        evidence(&run.record, "w"),
        BTreeSet::from(["13m".to_owned()]),
        "{:?}",
        run.record
    );
    let row = &run.summary.functions["w"];
    assert_ne!(
        row.status,
        qcore_c_gateway::legacy_c::AnalysisStatus::Safe,
        "{row:?}"
    );
    assert!(
        row.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            qcore_c_gateway::legacy_c::LegacyDiagnosticKind::SummaryConflict { .. }
        )),
        "{row:?}"
    );
}

#[test]
fn a_declared_alias_the_entry_check_does_not_establish_keeps_its_failure() {
    if !clang_available() {
        return;
    }
    use qcore_c_gateway::legacy_c::DerivedExtent;
    let declared = contract(
        [ResourceParameterMode::Borrowed],
        ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Exact,
            nullable: false,
            const_dropped: false,
        },
    );
    let header = "static inline void *w(void *p) { return p; }";
    let main = "void user(void) { void *p = malloc(4); void *q = w(p); (void) q; free(p); }";
    let run = run("alias", header, main, &contracts(&declared), true);
    assert!(evidence(&run.record, "w").is_empty(), "{:?}", run.record);
    assert!(
        run.record
            .failed
            .get("w")
            .is_some_and(|why| why.contains("conflict")),
        "{:?}",
        run.record
    );
}

#[test]
fn a_nullable_formal_handed_to_a_non_null_release_is_not_validated() {
    if !clang_available() {
        return;
    }
    // W4 reads `Released { nullable: false }` (a refinement of the declared
    // nullable release, so the mode gate withdraws nothing); under the
    // declaration's entry the formal may be null where `sink_nonnull`
    // requires it is not, so the check fails and the conflict stays a failure.
    let declared = contract([released(true)], ResourceReturnMode::None);
    let header = "static inline void w(void *p) { sink_nonnull(p); }";
    let run = run("nonnull", header, USER, &contracts(&declared), true);
    assert_ne!(
        run.record.body_outcomes().get("w"),
        Some(&BodyOutcome::Validated),
        "{:?}",
        run.record
    );
    assert!(evidence(&run.record, "w").is_empty(), "{:?}", run.record);
    // The 13m route ran and kept the conflict a failure.
    assert!(
        run.record
            .failed
            .get("w")
            .is_some_and(|why| why.contains("conflict")),
        "{:?}",
        run.record
    );
}

#[test]
fn a_contradicted_or_unmodelled_body_is_not_validated() {
    if !clang_available() {
        return;
    }
    let declared = contract([released(true)], ResourceReturnMode::None);
    for (tag, header) in [
        // Keeps its formal: a duty contradiction (the mode gate withdraws it).
        ("keep", "static inline void w(void *p) { keep = p; }"),
        // An uncontracted call: no reading of the body at all.
        (
            "opaque",
            "static inline void w(void *p) { opaque(p); free(p); }",
        ),
        // Releases only on one path.
        (
            "branch",
            "static inline void w(void *p) { if (keep) free(p); }",
        ),
    ] {
        let run = run(tag, header, USER, &contracts(&declared), true);
        assert_ne!(
            run.record.body_outcomes().get("w"),
            Some(&BodyOutcome::Validated),
            "{tag}: {:?}",
            run.record
        );
        let by = evidence(&run.record, "w");
        assert!(!by.contains("13m") && !by.contains("13m"), "{tag}: {by:?}");
    }
}

#[test]
fn a_declaration_with_a_container_clause_is_never_checked() {
    if !clang_available() {
        return;
    }
    let mut declared = contract([released(true)], ResourceReturnMode::None);
    declared.counted_users.neutral = true;
    let header = "static inline void w(void *p) { free(p); }";
    let run = run("users", header, USER, &contracts(&declared), true);
    let by = evidence(&run.record, "w");
    assert!(!by.contains("13m") && !by.contains("13m"), "{by:?}");
}

#[test]
fn the_check_is_evidence_only() {
    if !clang_available() {
        return;
    }
    let declared = contract([released(true)], ResourceReturnMode::None);
    let header = "static inline void w(void *p) { free(p); }";
    let contracts = contracts(&declared);
    let unrecorded = run("only-a", header, USER, &contracts, false);
    let recorded = run("only-b", header, USER, &contracts, true);
    // Every row identical: status, diagnostics, CFG and exits.
    assert_eq!(unrecorded.summary, recorded.summary);
    // Without a recording nothing is recorded.
    assert!(unrecorded.record.validated.is_empty());
    // The check consults nothing on behalf of `user`, which spells no `free`.
    let user = recorded.record.functions.get("user").expect("user's log");
    assert!(!user.found.contains("free"), "{user:?}");
}
