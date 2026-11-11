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
//!   wrapper (a failed check of `malloc`'s result ending at the no-return `__assert_fail`, `rd_malloc`'s shape) conflict
//!   with W4 and are validated by `13m`;
//! * **refusing:** a wrapper that hands its nullable formal to a non-null
//!   release keeps its `conflict` failure; a wrapper that keeps its formal,
//!   one that calls an uncontracted function, one that releases on one path,
//!   a declared alias returning unrelated NULL, and a
//!   declaration with a counted-user clause are never `13m`;
//! * **evidence only:** every row is identical with and without a recording
//!   (the conflicting row keeps its `SummaryConflict`), and the check adds no
//!   consultation to any function.
//!
//! The candidates in these fixtures are fixture findings, not claims about
//! any upstream body.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const LIBC_STDLIB: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdlib.qv");

use qcore_c_gateway::contract_dependencies::{BodyOutcome, Recording, UnitRecord};
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
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

/// The fixture module: `sink_nonnull` and the declaration `w` under test.
const SINK: &str = "extern fn sink_nonnull(ptr: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(ptr, nonnull);\n    ensures normal resource_return_none();\n};\n";

/// `w` releases its formal, which may be null.
const W_RELEASE: &str = "extern fn w(p: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(p, nullable);\n    ensures normal resource_return_none();\n};\n";

/// The same with a counted-user clause.
const W_RELEASE_USERS: &str = "extern fn w(p: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(p, nullable);\n    ensures normal resource_user_neutral();\n    ensures normal resource_return_none();\n};\n";

/// `w` returns a fresh allocation, which may be null.
const W_ALLOC: &str = "extern fn w(n: usize) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_owned(nullable);\n};\n";

/// `w` returns its formal, never null.
const W_ALIAS: &str = "extern fn w(p: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_return_alias(p, nonnull);\n};\n";

/// The libc modules, byte for byte from the provider (`malloc`, `free`, and
/// `__assert_fail` with its `resource_no_return()`), and the fixture module
/// declaring `w` as `declared`.
fn contracts(declared: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("libc/stdlib.qv".to_owned(), normalize_newlines(LIBC_STDLIB)),
        (
            "fixture/w.qv".to_owned(),
            format!("module contract;\n{SINK}{declared}"),
        ),
    ]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
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
    let declared = W_RELEASE;
    let header = "static inline void w(void *p) { free(p); }";
    let run = run("free", header, USER, &contracts(declared), true);
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
    let declared = W_ALLOC;
    // Declare the modelled glibc entry explicitly, independent of host headers.
    let header = "extern void __assert_fail(const char *, const char *, unsigned int, const char *) __attribute__((__noreturn__));\n\
         static inline void *w(size_t n) { void *p = malloc(n); \
         if (!p) __assert_fail(\"p\", \"w.h\", 1, \"w\"); return p; }";
    let main = "void user(void) { void *p = w(4); free(p); }";
    let run = run("alloc", header, main, &contracts(declared), true);
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
    let declared = W_RELEASE;
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
        let run = run(tag, header, USER, &contracts(declared), true);
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
    let run = run("row", "", &main, &contracts(declared), true);
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
fn a_declared_original_formal_alias_is_now_validated_by_w4() {
    if !clang_available() {
        return;
    }
    let header = "static inline void *w(void *p) { return p; }";
    let main = "void user(void) { void *p = malloc(4); void *q = w(p); (void) q; free(p); }";
    let run = run("alias", header, main, &contracts(W_ALIAS), true);
    assert_eq!(
        evidence(&run.record, "w"),
        BTreeSet::from(["W4".to_owned()])
    );
    assert!(!run.record.failed.contains_key("w"), "{:?}", run.record);
}

#[test]
fn a_declared_alias_returning_unrelated_null_keeps_its_failure() {
    if !clang_available() {
        return;
    }
    let declared = W_ALIAS;
    let header = "static inline void *w(void *p) { return 0; }";
    let main = "void user(void) { void *p = malloc(4); void *q = w(p); (void) q; free(p); }";
    let run = run("alias-null", header, main, &contracts(declared), true);
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
    let declared = W_RELEASE;
    let header = "static inline void w(void *p) { sink_nonnull(p); }";
    let run = run("nonnull", header, USER, &contracts(declared), true);
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
    let declared = W_RELEASE;
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
        let run = run(tag, header, USER, &contracts(declared), true);
        assert_ne!(
            run.record.body_outcomes().get("w"),
            Some(&BodyOutcome::Validated),
            "{tag}: {:?}",
            run.record
        );
        let by = evidence(&run.record, "w");
        assert!(!by.contains("13m"), "{tag}: {by:?}");
    }
}

#[test]
fn a_declaration_with_a_counted_user_clause_is_never_checked() {
    if !clang_available() {
        return;
    }
    let header = "static inline void w(void *p) { free(p); }";
    let run = run("users", header, USER, &contracts(W_RELEASE_USERS), true);
    assert!(
        !evidence(&run.record, "w").contains("13m"),
        "{:?}",
        run.record
    );
}

#[test]
fn the_check_is_evidence_only() {
    if !clang_available() {
        return;
    }
    let declared = W_RELEASE;
    let header = "static inline void w(void *p) { free(p); }";
    let contracts = contracts(declared);
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
