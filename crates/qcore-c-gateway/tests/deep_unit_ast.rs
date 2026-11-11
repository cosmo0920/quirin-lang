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

//! Card **RDHTTP-HEADER-SUMMARY-GAP** (gateway rules 13o, 13p): a unit whose
//! Clang JSON AST nests deeper than `serde_json`'s default recursion limit
//! (128) loses its whole-unit typed AST index, and with it every typed
//! reading of every definition in the unit, the header bodies included.
//!
//! The measured instance is librdkafka's `rdhttp.c`: four definitions there
//! call `curl_easy_setopt`, whose GCC type-check macro (`curl/typecheck-gcc.h`)
//! nests Clang's JSON 212 to 224 levels deep. Every other unit including
//! `rd.h` reads `rd_malloc`'s `rd_assert(p)` (glibc's `assert`, a GNU
//! statement expression under `gnu17`, rule 11g) from the unit index; in
//! `rdhttp.c` no index exists, the statement expression is not read, W4
//! publishes no summary and the unit examined the body with no evidence.
//! `rd_free` (no statement expression) needs no typed reading and validates
//! there.
//!
//! Each fixture puts the wrapper in an included header (a `static inline`
//! definition the selection reaches, as `rd_malloc` in `rd.h`) beside a
//! definition in the main source whose `else if` chain sets the JSON depth:
//!
//! * a shallow unit validates the asserting wrapper (`13m`), as every
//!   librdkafka unit but `rdhttp.c` does;
//! * a deep unit (beyond 128, within the admitted bound) validates it the
//!   same way once the index is read without the default limit (rule 13o);
//! * a unit beyond the admitted bound keeps no index and the wrapper stays
//!   unvalidated (rule 13p: never a fold that ignores the unit);
//! * a release wrapper with no statement expression validates in all three.
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
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyFunctionScope,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent RDHTTP-HEADER-SUMMARY-GAP test: {reason}");
            false
        }
    }
}

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;\n\
void *malloc(size_t size);\n\
void free(void *ptr);\n";

/// `w` returns a fresh allocation, which may be null (`libc::malloc`'s
/// custody, as `rd_malloc` inherits it).
const W_ALLOC: &str = "extern fn w(n: usize) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_owned(nullable);\n};\n";

/// `w` releases its formal, which may be null (`rd_free`'s shape).
const W_RELEASE: &str = "extern fn w(p: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(p, nullable);\n    ensures normal resource_return_none();\n};\n";

/// `rd_malloc`'s body with an explicit glibc-style GNU statement expression.
/// Host assert headers (including Darwin's different expansion) must not
/// change the source shape this depth regression exercises.
const ASSERTING: &str = r#"
extern _Bool __assert_single_arg(_Bool);
extern void __assert_fail(const char *, const char *, unsigned int, const char *)
    __attribute__((__noreturn__));
static inline void *w(size_t n) {
    void *p = malloc(n);
    ((void) sizeof (__assert_single_arg(p)), __extension__ ({
        if (p) ; else __assert_fail("p", "w.h", 1, __extension__ __PRETTY_FUNCTION__);
    }));
    return p;
}
"#;

/// `rd_free`'s body.
const RELEASING: &str = "static inline void w(void *p) { free(p); }";

fn contracts(declared: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("libc/stdlib.qv".to_owned(), normalize_newlines(LIBC_STDLIB)),
        (
            "fixture/w.qv".to_owned(),
            format!("module contract;\n{declared}"),
        ),
    ]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

/// A definition whose `else if` chain has `arms` arms: Clang nests each
/// `IfStmt` two JSON levels inside the previous one.
fn chain(arms: usize) -> String {
    let mut body = String::from("int chain(int x) {\n    if (x == 0) return 0;\n");
    for arm in 1..arms {
        body.push_str(&format!("    else if (x == {arm}) return {arm};\n"));
    }
    body.push_str("    return -1;\n}\n");
    body
}

struct Run {
    summary: LegacyProgramSummary,
    record: UnitRecord,
}

fn run(tag: &str, header: &str, main: &str, contracts: &ResourceContracts) -> Run {
    let directory = std::env::temp_dir().join(format!(
        "qcore-rhsg-{tag}-{}-{:?}",
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
        dialect: CCompilerDialect::Gnu17,
        ..C11TranslationUnit::default()
    };
    let recording = Recording::start();
    let result = analyze_unrestricted_c11(&unit, "user", contracts)
        .unwrap_or_else(|error| panic!("fixture rejected: {error:?}"));
    let record = recording.finish();
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

/// The beyond-bound fixtures run on a 256 MiB thread: the token route's
/// `else if` recursion over 130 arms needs more than a debug test thread's
/// default stack, at the base as after this card (measured: the base's test
/// binary aborts with a stack overflow there). The deep fixture (80 arms),
/// whose typed index this card admits, runs on the default test thread.
fn on_a_large_stack(
    tag: &'static str,
    header: &'static str,
    main: String,
    declared: &'static str,
) -> Run {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || run(tag, header, &main, &contracts(declared)))
        .expect("a fixture thread")
        .join()
        .expect("the fixture thread")
}

fn evidence(record: &UnitRecord, name: &str) -> BTreeSet<String> {
    record.validated.get(name).cloned().unwrap_or_default()
}

const ALLOC_USER: &str = "void user(void) { void *p = w(4); free(p); }";
const RELEASE_USER: &str = "void user(void) { void *p = malloc(4); w(p); }";

/// Clang's JSON for `chain(n)` in these fixtures nests `2n + 13` deep
/// (measured: 4 arms 21, 80 arms 173, 130 arms 273): shallow, deep beyond
/// 128 but within the admitted 256, and beyond the admitted bound.
const SHALLOW: usize = 4;
const DEEP: usize = 80;
const BEYOND: usize = 130;

#[test]
fn an_asserting_wrapper_is_validated_in_a_shallow_unit() {
    if !clang_available() {
        return;
    }
    let main = format!("{}{ALLOC_USER}", chain(SHALLOW));
    let run = run("shallow", ASSERTING, &main, &contracts(W_ALLOC));
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
}

#[test]
fn an_asserting_wrapper_is_validated_in_a_unit_deeper_than_the_default_limit() {
    if !clang_available() {
        return;
    }
    let main = format!("{}{ALLOC_USER}", chain(DEEP));
    let run = run("deep", ASSERTING, &main, &contracts(W_ALLOC));
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
    // The deep definition itself is read, not refused for want of an AST.
    let row = &run.summary.functions["chain"];
    assert_eq!(row.status, AnalysisStatus::Safe, "{row:?}");
}

#[test]
fn beyond_the_admitted_bound_the_unit_keeps_no_index_and_nothing_validates() {
    if !clang_available() {
        return;
    }
    let main = format!("{}{ALLOC_USER}", chain(BEYOND));
    let run = on_a_large_stack("beyond", ASSERTING, main, W_ALLOC);
    // The body was examined and nothing validated it: the unit stays in the
    // fold as unvalidated (rule 13g), never skipped.
    assert_eq!(
        run.record.body_outcomes().get("w"),
        Some(&BodyOutcome::Unvalidated),
        "{:?}",
        run.record
    );
    assert!(evidence(&run.record, "w").is_empty(), "{:?}", run.record);
}

#[test]
fn a_release_wrapper_without_a_statement_expression_validates_at_every_depth() {
    if !clang_available() {
        return;
    }
    for (tag, arms) in [
        ("r-shallow", SHALLOW),
        ("r-deep", DEEP),
        ("r-beyond", BEYOND),
    ] {
        let main = format!("{}{RELEASE_USER}", chain(arms));
        let run = if arms == BEYOND {
            on_a_large_stack(tag, RELEASING, main, W_RELEASE)
        } else {
            run(tag, RELEASING, &main, &contracts(W_RELEASE))
        };
        assert_eq!(
            evidence(&run.record, "w"),
            BTreeSet::from(["13m".to_owned()]),
            "{tag}: {:?}",
            run.record
        );
    }
}
