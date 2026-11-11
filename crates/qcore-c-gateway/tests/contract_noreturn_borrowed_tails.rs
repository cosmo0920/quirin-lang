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

//! Card CONTRACT-NORETURN-AND-BORROWED-TAILS: controls for the production
//! declarations this card lands (`docs/reports/contract-noreturn-and-borrowed-tails/`).
//!
//! No route changes: rule 11o reads `resource_no_return()` and rule 11w's T8
//! reads `resource_variadic_borrowed_tail()`. These tests load the production
//! modules byte for byte (the librdkafka declaration from its section marker,
//! because that module's libc includes are not resolved by this standalone
//! loader) beside the CG00 fixture contracts (`support/clang_gap.rs`: the
//! synthetic `q_*` custody API, trusted test assumptions) and check, per
//! declared callee:
//!
//! * **accepting:** a live tracked pointer handed to the tail is borrowed and
//!   the call admitted; a call to `rd_kafka_crash` ends its path;
//! * **withdrawal:** with the card's clause replaced by the old one (the data
//!   tail, or no declaration), the same body keeps its refusal;
//! * **refusing:** a pointer released before the call is never `Safe`
//!   (contract specification C04.2's caller-side rule), a leak is still
//!   reported, the no-return path end needs the Clang `noreturn` declaration,
//!   and a `%n` write through an aliased scalar is never certified.
//!
//! A `Safe` here is `Safe` over the fixture and production assumptions and
//! promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::{
    load_resource_contract_sources, no_return_callees, variadic_borrowed_tails, variadic_data_tails,
};

#[path = "support/clang_gap.rs"]
mod clang_gap;
#[path = "support/text.rs"]
mod text;

use clang_gap::{DECLARATIONS, clang_available};
use text::normalize_newlines;

const STDIO: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdio.qv");
const MONKEY_LOGGING: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/logging.qv");
const FLB_LOGGING: &str =
    include_str!("../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv");
/// `librdkafka/memory.qv` includes libc parents, which this standalone loader
/// does not resolve; the card's declaration is its final section.
const RDKAFKA_MEMORY: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv");
const CRASH_SECTION: &str = "// ## Card CONTRACT-NORETURN-AND-BORROWED-TAILS: `rd_kafka_crash`";

/// A synthetic variadic borrower (CG26's `cg_blog`), used only by the `%n`
/// control so that it also stands without the production modules.
const BLOG: &str = "module contract;\n\
extern fn cnbt_blog(format: ptr<const, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_borrowed(format);\n\
    ensures normal resource_variadic_borrowed_tail();\n\
    ensures normal resource_return_none();\n\
};\n";

const PROTOTYPES: &str = "\
typedef struct _IO_FILE FILE;
extern FILE *stderr;
int printf(const char *format, ...);
int fprintf(FILE *stream, const char *format, ...);
int snprintf(char *s, size_t maxlen, const char *format, ...);
int sprintf(char *s, const char *format, ...);
void mk_print(int type, const char *format, ...);
void flb_log_print(int type, const char *file, int line, const char *fmt, ...);
void cnbt_blog(const char *format, ...);
typedef struct rd_kafka_s rd_kafka_t;
";

const NORETURN_PROTOTYPE: &str = "void __attribute__((noreturn)) rd_kafka_crash(const char *file, \
int line, const char *function, rd_kafka_t *rk, const char *reason);\n";
const PLAIN_PROTOTYPE: &str = "void rd_kafka_crash(const char *file, int line, \
const char *function, rd_kafka_t *rk, const char *reason);\n";

fn section(text: &str, marker: &str) -> String {
    let text = normalize_newlines(text);
    let start = text
        .find(marker)
        .expect("the card's section is in the module");
    format!("module contract;\n{}", &text[start..])
}

/// The fixture contracts plus the given production texts.
fn load(extra: &[(&str, String)]) -> ResourceContracts {
    let mut sources = clang_gap::contract_sources();
    sources.insert("cnbt_blog.qcore".to_owned(), BLOG.to_owned());
    for (name, text) in extra {
        sources.insert((*name).to_owned(), text.clone());
    }
    load_resource_contract_sources(&sources)
        .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn production() -> ResourceContracts {
    load(&[
        ("libc/stdio.qv", normalize_newlines(STDIO)),
        ("monkey/logging.qv", normalize_newlines(MONKEY_LOGGING)),
        ("fluent_bit/logging.qv", normalize_newlines(FLB_LOGGING)),
        (
            "librdkafka/cnbt_crash.qv",
            section(RDKAFKA_MEMORY, CRASH_SECTION),
        ),
    ])
}

/// The production texts with the card's clauses replaced by the old ones:
/// the data tail for each borrowed tail, and no `rd_kafka_crash` at all.
fn without_card() -> ResourceContracts {
    let old = |text: &str| {
        normalize_newlines(text).replace(
            "ensures normal resource_variadic_borrowed_tail();",
            "ensures normal resource_variadic_data_tail();",
        )
    };
    load(&[
        ("libc/stdio.qv", old(STDIO)),
        ("monkey/logging.qv", old(MONKEY_LOGGING)),
        ("fluent_bit/logging.qv", old(FLB_LOGGING)),
    ])
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    unsupported: Vec<String>,
    diagnostics: Vec<String>,
}

impl Run {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused(&self, description: &str) -> bool {
        self.unsupported.iter().any(|d| d == description)
    }
    fn refused_prefix(&self, prefix: &str) -> bool {
        self.unsupported.iter().any(|d| d.starts_with(prefix))
    }
    fn leaks(&self) -> bool {
        self.diagnostics.iter().any(|d| d.starts_with("Leak"))
    }
}

fn program(result: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

fn run_with(contracts: &ResourceContracts, prototypes: &str, body: &str) -> Run {
    let text = format!("{DECLARATIONS}\n{PROTOTYPES}{prototypes}\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = program(&outcome)
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("no summary for `subject`: {outcome:?}"));
    Run {
        status: result.status,
        unsupported: result
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
        diagnostics: result
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.kind))
            .collect(),
    }
}

fn tail(callee: &str) -> String {
    format!(
        "arguments beyond the declared parameters of `{callee}` are outside the resource projection"
    )
}

// ---------------------------------------------------------------------------
// `%n` (C11 7.21.6.1p8): a write through a trailing `int *`
// ---------------------------------------------------------------------------

/// `%n` stores the count written so far through a trailing `int *`. The
/// borrowed tail reads that actual as a `Borrowed` place, which takes and
/// transfers nothing: the store is a write through a borrowed position, as
/// `snprintf`'s `s` is. What a borrowed tail must never do is certify a body
/// whose custody decision rests on a scalar the call may have rewritten. In
/// each body below the count is 2 after the call in C, so `p` leaks; none may
/// be `Safe`, whether the counter reaches the tail as `&x` (T7, whose fact is
/// forgotten after the call), through a local pointer to `x` (the projection
/// refuses `px = &x` itself) or as a pointer formal (T8; `*cnt` is not a
/// fact the projection holds).
fn percent_n_is_never_certified(contracts: &ResourceContracts, call: &str) {
    for body in [
        format!(
            "void subject(void) {{ char b[8]; int x = 0; void *p = q_alloc(); \
             {call}\"ab%n\", &x); if (x == 0) {{ q_free(p); }} }}"
        ),
        format!(
            "void subject(void) {{ char b[8]; int x = 0; int *px = &x; void *p = q_alloc(); \
             {call}\"ab%n\", px); if (x == 0) {{ q_free(p); }} }}"
        ),
        format!(
            "void subject(int *cnt) {{ char b[8]; void *p = q_alloc(); *cnt = 0; \
             {call}\"ab%n\", cnt); if (*cnt == 0) {{ q_free(p); }} }}"
        ),
    ] {
        let run = run_with(contracts, "", &body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
    // Without the branch on the count, the `&x` and formal shapes are
    // admitted, so the refusals above are the count's, not the shape's.
    for body in [
        format!(
            "void subject(void) {{ char b[8]; int x = 0; void *p = q_alloc(); \
             {call}\"ab%n\", &x); q_free(p); }}"
        ),
        format!(
            "void subject(int *cnt) {{ char b[8]; void *p = q_alloc(); \
             {call}\"ab%n\", cnt); q_free(p); }}"
        ),
    ] {
        let run = run_with(contracts, "", &body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

/// The callees and the call prefix (up to the format) the tail controls use.
const TAIL_CALLS: &[(&str, &str)] = &[
    ("printf", "printf("),
    ("fprintf", "fprintf(stderr, "),
    ("snprintf", "snprintf(b, sizeof(b), "),
    ("mk_print", "mk_print(1, "),
    ("flb_log_print", "flb_log_print(1, (void *) 0, 0, "),
];

#[test]
fn a_percent_n_write_through_an_aliased_scalar_is_never_certified() {
    if !clang_available() {
        return;
    }
    percent_n_is_never_certified(&load(&[]), "cnbt_blog(");
    let contracts = production();
    for (_, call) in TAIL_CALLS {
        percent_n_is_never_certified(&contracts, call);
    }
}

// ---------------------------------------------------------------------------
// The clauses as loaded
// ---------------------------------------------------------------------------

/// The production texts carry exactly the card's clauses: the five callees
/// declare the borrowed tail (which implies the data tail), `sprintf` keeps
/// the data tail only, and `rd_kafka_crash` declares the no-return exit.
#[test]
fn the_production_declarations_carry_the_card_clauses() {
    let sources = BTreeMap::from([
        ("libc/stdio.qv".to_owned(), normalize_newlines(STDIO)),
        (
            "monkey/logging.qv".to_owned(),
            normalize_newlines(MONKEY_LOGGING),
        ),
        (
            "fluent_bit/logging.qv".to_owned(),
            normalize_newlines(FLB_LOGGING),
        ),
        (
            "librdkafka/cnbt_crash.qv".to_owned(),
            section(RDKAFKA_MEMORY, CRASH_SECTION),
        ),
    ]);
    let borrowed = variadic_borrowed_tails(&sources).expect("loads");
    let data = variadic_data_tails(&sources).expect("loads");
    for (callee, _) in TAIL_CALLS {
        assert!(borrowed.contains(*callee), "{callee}: {borrowed:?}");
        assert!(data.contains(*callee), "{callee}: {data:?}");
    }
    assert!(!borrowed.contains("sprintf") && data.contains("sprintf"));
    let exits = no_return_callees(&sources).expect("loads");
    assert!(exits.contains("rd_kafka_crash"), "{exits:?}");
}

// ---------------------------------------------------------------------------
// `rd_kafka_crash` (CG21-UNDECLARED-NORETURN-CALLEES)
// ---------------------------------------------------------------------------

/// `rd_kafka_assert(rk, p != 0)` as the preprocessor writes it, over a
/// nullable acquisition: the crash arm (`p` NULL) ends its path, so only the
/// non-null `p` reaches the non-null release after it. (A non-null `q_alloc`
/// would make the crash arm dead, and a dead uncontracted call is the
/// gateway's known hard error, `retains an unknown instruction`, at the base:
/// `docs/reports/remaining-c03-nfr/README.md` section 5.)
const ASSERT_SHAPE: &str = "void subject(rd_kafka_t *rk) { void *p = q_maybe_alloc(); \
do { if (__builtin_expect((!(p != 0)), 0)) rd_kafka_crash(\"rdkafka.c\", 1, __FUNCTION__, (rk), \
\"assert: \" \"p != 0\"); } while (0); q_free(p); }";

/// A release before the crash on its arm and a release after it: two
/// releases of `p` only if the crash returned.
const RELEASE_THEN_CRASH: &str = "void subject(rd_kafka_t *rk, int c) { void *p = q_alloc(); \
if (c) { q_free(p); rd_kafka_crash(\"rdkafka.c\", 1, __FUNCTION__, rk, \"x\"); } \
q_free(p); }";

#[test]
fn a_call_to_rd_kafka_crash_ends_its_path() {
    if !clang_available() {
        return;
    }
    let contracts = production();
    for body in [ASSERT_SHAPE, RELEASE_THEN_CRASH] {
        let run = run_with(&contracts, NORETURN_PROTOTYPE, body);
        assert!(run.safe(), "{body}: {run:?}");
        // Without the declaration the call is uncontracted, as at the base.
        let before = run_with(&without_card(), NORETURN_PROTOTYPE, body);
        assert!(
            before.refused("call to uncontracted function `rd_kafka_crash`"),
            "{body}: {before:?}"
        );
    }
}

/// The declaration alone does not end a path: a prototype without
/// `noreturn` keeps the call refused (rule 11o), and the release before and
/// after it is never certified.
#[test]
fn rd_kafka_crash_needs_its_noreturn_declaration() {
    if !clang_available() {
        return;
    }
    let run = run_with(&production(), PLAIN_PROTOTYPE, RELEASE_THEN_CRASH);
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.refused_prefix("no-return contract of `rd_kafka_crash` not established at this call"),
        "{run:?}"
    );
}

/// The no-return call still checks its actuals: an `rk` released before the
/// call is never `Safe`.
#[test]
fn rd_kafka_crash_still_checks_a_released_actual() {
    if !clang_available() {
        return;
    }
    let body = "void subject(void) { void *rk = q_alloc(); q_free(rk); \
                rd_kafka_crash(\"rdkafka.c\", 1, __FUNCTION__, rk, \"x\"); }";
    let run = run_with(&production(), NORETURN_PROTOTYPE, body);
    assert!(!run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// The borrowed tails (CG26-BORROWED-TAIL-DECLARATIONS)
// ---------------------------------------------------------------------------

/// A tracked `char *` in each callee's tail is borrowed: the call is
/// admitted and the buffer's own release decides the body. Under the old data
/// tail the same body keeps O04-TRACE's refusal.
#[test]
fn a_tracked_pointer_in_a_declared_tail_is_borrowed() {
    if !clang_available() {
        return;
    }
    let contracts = production();
    let before = without_card();
    for (callee, call) in TAIL_CALLS {
        let body = format!(
            "void subject(void) {{ char b[8]; char *buf = q_alloc(); \
             {call}\"%d '%s'\", 3, buf); q_free(buf); }}"
        );
        let run = run_with(&contracts, "", &body);
        assert!(run.safe(), "{body}: {run:?}");
        let old = run_with(&before, "", &body);
        assert!(old.refused(&tail(callee)), "{body}: {old:?}");
    }
}

/// The borrowed tail keeps the buffer's obligation (dropping it leaks) and
/// the caller-side refusal (C04.2): a pointer released before the call is
/// never `Safe`.
#[test]
fn the_borrowed_tail_keeps_obligations_and_the_caller_side_refusal() {
    if !clang_available() {
        return;
    }
    let contracts = production();
    for (_, call) in TAIL_CALLS {
        let leaking = format!(
            "void subject(void) {{ char b[8]; char *buf = q_alloc(); {call}\"%s\", buf); }}"
        );
        let run = run_with(&contracts, "", &leaking);
        assert!(!run.safe() && run.leaks(), "{leaking}: {run:?}");
        let released = format!(
            "void subject(void) {{ char b[8]; char *buf = q_alloc(); q_free(buf); \
             {call}\"%s\", buf); }}"
        );
        let run = run_with(&contracts, "", &released);
        assert!(!run.safe(), "{released}: {run:?}");
    }
}

/// `sprintf` keeps the data tail: a tracked pointer in its tail is refused.
#[test]
fn sprintf_keeps_the_data_tail() {
    if !clang_available() {
        return;
    }
    let body = "void subject(void) { char b[64]; char *buf = q_alloc(); \
                sprintf(b, \"%s\", buf); q_free(buf); }";
    let run = run_with(&production(), "", body);
    assert!(run.refused(&tail("sprintf")), "{run:?}");
}
