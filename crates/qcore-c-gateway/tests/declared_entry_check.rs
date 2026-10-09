//! Card **DECLARED-ASSUMPTION-BODY-CHECKS** (gateway rules 13m–13o): the
//! declared-entry check of a body the derivation scope holds but no row
//! reports (13m), and of a body whose W4 summary only refines its declaration
//! (13n), recorded as body evidence and nothing else (13o).
//!
//! Every fixture runs the public C11 entry point under the main-source scope
//! with its helper in an included header (a `static inline` definition the
//! selection reaches, as `rd_free` in `rd.h`), inside a [`Recording`]:
//!
//! * **accepting:** a nullable release wrapper (`free(p)` under
//!   `resource_released(p, nullable)`) conflicts with W4's non-null summary
//!   and its declaration is validated by `13n`; a nullable-result wrapper
//!   (`assert` after `malloc`) likewise;
//! * **refusing:** a wrapper that hands its nullable formal to a non-null
//!   release, a wrapper that keeps its formal, one that calls an uncontracted
//!   function, and a body with a container clause are not validated, and a
//!   conflict the check does not establish stays a failure;
//! * **evidence only:** every row is identical with and without a recording,
//!   and the check adds no consultation to any function.
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
void __assert_fail(const char *a, const char *f, unsigned int l, const char *fn);\n\
#define assert(e) ((e) ? (void)0 : __assert_fail(#e, __FILE__, __LINE__, __func__))\n\
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
    contracts.insert_contract(
        "__assert_fail",
        contract(
            [ResourceParameterMode::Borrowed; 4],
            ResourceReturnMode::None,
        ),
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
        BTreeSet::from(["13n".to_owned()])
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
    let header = "static inline void *w(size_t n) { void *p = malloc(n); assert(p); return p; }";
    let main = "void user(void) { void *p = w(4); free(p); }";
    let run = run("alloc", header, main, &contracts(&declared), true);
    assert_eq!(
        run.record.body_outcomes().get("w"),
        Some(&BodyOutcome::Validated),
        "{:?}",
        run.record
    );
    let by = evidence(&run.record, "w");
    assert!(
        by.contains("13n") || by.contains("W4") || by.contains("13m"),
        "{by:?}"
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
    // The 13n route ran and kept the conflict a failure.
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
        assert!(!by.contains("13m") && !by.contains("13n"), "{tag}: {by:?}");
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
    assert!(!by.contains("13m") && !by.contains("13n"), "{by:?}");
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
