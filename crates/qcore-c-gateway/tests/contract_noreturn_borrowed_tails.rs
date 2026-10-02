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
    load_resource_contract_sources, no_return_callees, variadic_borrowed_tails,
    variadic_data_tails,
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
    format!("arguments beyond the declared parameters of `{callee}` are outside the resource projection")
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
fn percent_n_is_never_certified(contracts: &ResourceContracts, callee_call: &str) {
    for body in [
        format!(
            "void subject(void) {{ int x = 0; void *p = q_alloc(); \
             {callee_call}(\"ab%n\", &x); if (x == 0) {{ q_free(p); }} }}"
        ),
        format!(
            "void subject(void) {{ int x = 0; int *px = &x; void *p = q_alloc(); \
             {callee_call}(\"ab%n\", px); if (x == 0) {{ q_free(p); }} }}"
        ),
        format!(
            "void subject(int *cnt) {{ void *p = q_alloc(); *cnt = 0; \
             {callee_call}(\"ab%n\", cnt); if (*cnt == 0) {{ q_free(p); }} }}"
        ),
    ] {
        let run = run_with(contracts, "", &body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
    // Without the branch on the count, the `&x` and formal shapes are
    // admitted, so the refusals above are the count's, not the shape's.
    for body in [
        format!(
            "void subject(void) {{ int x = 0; void *p = q_alloc(); \
             {callee_call}(\"ab%n\", &x); q_free(p); }}"
        ),
        format!(
            "void subject(int *cnt) {{ void *p = q_alloc(); \
             {callee_call}(\"ab%n\", cnt); q_free(p); }}"
        ),
    ] {
        let run = run_with(contracts, "", &body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

#[test]
fn a_percent_n_write_through_an_aliased_scalar_is_never_certified() {
    if !clang_available() {
        return;
    }
    let contracts = load(&[]);
    percent_n_is_never_certified(&contracts, "cnbt_blog");
}
