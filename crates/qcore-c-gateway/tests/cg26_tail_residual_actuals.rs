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

//! Card CG26-TAIL-RESIDUAL-ACTUALS (gateway rule 12g): controls for the
//! Clang reading of a trailing actual that rule 11w's token premises refuse
//! (`docs/reports/cg26-tail-residual-actuals/`).
//!
//! Each body is put in a function whose lexical declarations the token reader
//! cannot read (an array-of-function-pointers formal `int (*cb[1])(int)`; a
//! plain function-pointer formal is read since card
//! FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS), which is the
//! state of 24 of the 34 corpus statements: there T4, T5 and T8 answer
//! nothing. The callees are the production `printf` family from
//! `quirin-provider-libc` (`fprintf`/`snprintf` declare
//! `resource_variadic_borrowed_tail()`, `sprintf` only the data tail), loaded
//! byte for byte beside the CG00 fixture contracts. Per shape:
//!
//! * **accepting:** a tracked pointer name is a `Borrowed` place (the call is
//!   admitted and the release after it makes the body `Safe`); scalar
//!   arithmetic, `sizeof`, a conditional of data arms and the assignment form
//!   `r = snprintf(…)` are data;
//! * **refusing:** a released pointer at the tail is never `Safe`, a dropped
//!   one leaks, and the tail refusal stays for a cast the conversion table
//!   does not read as its operand (rule 12h), pointer arithmetic, a conditional with an identity-carrying arm or
//!   condition, and a pointer name under the data tail (`sprintf`).
//!
//! A `Safe` here is `Safe` over the fixture and libc assumptions and promotes
//! nothing.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/clang_gap.rs"]
mod clang_gap;
#[path = "support/text.rs"]
mod text;

use clang_gap::{DECLARATIONS, clang_available};
use text::normalize_newlines;

const STDIO: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdio.qv");

const PROTOTYPES: &str = "\
typedef struct _IO_FILE FILE;
extern FILE *stderr;
int fprintf(FILE *stream, const char *format, ...);
int snprintf(char *s, size_t maxlen, const char *format, ...);
int sprintf(char *s, const char *format, ...);
typedef enum { CG_RACK_A, CG_RACK_B } cg_rack_t;
extern cg_rack_t cg_rack;
";

fn contracts() -> ResourceContracts {
    let mut sources = clang_gap::contract_sources();
    sources.insert("libc/stdio.qv".to_owned(), normalize_newlines(STDIO));
    load_resource_contract_sources(&sources)
        .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
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

/// `subject` with an unreadable formal list (`int (*cb[1])(int)`), or the same
/// statements in a readable one when `readable`.
fn run(statements: &str, readable: bool) -> Run {
    let formals = if readable {
        "int c, int i"
    } else {
        "int (*cb[1])(int), int c, int i"
    };
    let text = format!(
        "{DECLARATIONS}\n{PROTOTYPES}\nvoid subject({formals}) {{ char b[16]; {statements} }}\n"
    );
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
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

/// Each accepted shape, in an unreadable function: the call is admitted and
/// the body, which releases what it acquires, is `Safe`.
const ACCEPTED: &[&str] = &[
    // A tracked pointer name: a `Borrowed` place (T8 read from the node).
    "char *p = q_alloc(); fprintf(stderr, \"%s\", p); q_free(p);",
    // An array name (it decays): borrowed, untracked.
    "fprintf(stderr, \"%s\", b);",
    // Scalar arithmetic, a scalar name and `sizeof`.
    "fprintf(stderr, \"%d %d %zu\", i + 1, i, sizeof(b) * 2);",
    // A conditional whose arms are data, beside a name only this reading
    // admits here (`unittest_msgq_order`'s shape: rule 10z's arm statements
    // keep the tail refusal at `b`).
    "fprintf(stderr, \"%s %s\", b, c ? \"FIFO\" : \"LIFO\");",
    // Arithmetic and comparison operators over scalars.
    "fprintf(stderr, \"%d\", ((i >> 2) & 0xff) == 3 || !c);",
    // A typedef of an anonymous enumeration (`rd_kafka_assignor_ut_rack_config_t`):
    // Clang gives no desugared spelling, its integral promotion is the evidence.
    "fprintf(stderr, \"%d %d\", i, cg_rack);",
    // The assignment form.
    "int r = 0; r = snprintf(b, sizeof(b), \"%i.%i\", (i >> 24) & 0xff, (i >> 16) & 0xff); \
     cg_sink(r);",
];

#[test]
fn an_unreadable_function_reads_its_tail_from_the_clang_node() {
    if !clang_available() {
        return;
    }
    for statements in ACCEPTED {
        let unreadable = run(statements, false);
        assert!(unreadable.safe(), "{statements}: {unreadable:?}");
        // The same statements in a readable function stay `Safe`: the token
        // premises (T4, T8) decide them there, unchanged.
        let readable = run(statements, true);
        assert!(readable.safe(), "{statements} (readable): {readable:?}");
    }
}

/// C04.2's caller-side rule through the node reading: a released pointer in
/// the tail is never `Safe`, and a dropped one is reported as a leak.
#[test]
fn a_borrowed_name_keeps_the_flow_checks() {
    if !clang_available() {
        return;
    }
    let released = run(
        "char *p = q_alloc(); q_free(p); fprintf(stderr, \"%s\", p);",
        false,
    );
    assert!(!released.safe(), "{released:?}");
    let dropped = run("char *p = q_alloc(); fprintf(stderr, \"%s\", p);", false);
    assert!(dropped.leaks(), "{dropped:?}");
    assert!(!dropped.safe(), "{dropped:?}");
}

/// Every refused shape keeps rule 11w's tail refusal, byte for byte.
#[test]
fn casts_pointer_arithmetic_and_identity_arms_stay_refused() {
    if !clang_available() {
        return;
    }
    for (statements, callee) in [
        // Card CAST-FORMALIZATION (rule 12h): the scalar and identity-
        // preserving casts that were pinned here are read through
        // (`casts_the_table_reads_as_their_operand_are_read`); a pointer made
        // an integer keeps the refusal.
        (
            "char *p = q_alloc(); fprintf(stderr, \"%lu\", (unsigned long) p); q_free(p);",
            "fprintf",
        ),
        // Pointer arithmetic: its root is not an identified place.
        (
            "char *p = q_alloc(); fprintf(stderr, \"%s\", p + 1); q_free(p);",
            "fprintf",
        ),
        // A conditional with an identity-carrying arm or condition.
        (
            "char *p = q_alloc(); fprintf(stderr, \"%s\", c ? p : \"none\"); q_free(p);",
            "fprintf",
        ),
        (
            "char *p = q_alloc(); fprintf(stderr, \"%s %s\", b, p ? \"set\" : \"none\"); \
             q_free(p);",
            "fprintf",
        ),
        // A tracked pointer under the data tail only.
        (
            "char *p = q_alloc(); sprintf(b, \"%s\", p); q_free(p);",
            "sprintf",
        ),
    ] {
        let run = run(statements, false);
        assert!(!run.safe(), "{statements}: {run:?}");
        assert!(run.refused(&tail(callee)), "{statements}: {run:?}");
    }
}

/// Card CAST-FORMALIZATION (rule 12h), a deliberate flip of the three cast
/// rows this card pinned refused: a scalar conversion of data is data, and an
/// identity-preserving pointer conversion of a tracked name is that name's
/// `Borrowed` place, so its obligations stay (a release before it is never
/// `Safe`).
#[test]
fn casts_the_table_reads_as_their_operand_are_read() {
    if !clang_available() {
        return;
    }
    for statements in [
        "fprintf(stderr, \"%ld\", (long) i);",
        "fprintf(stderr, \"%f\", (float) i / 1000.0);",
        "char *p = q_alloc(); fprintf(stderr, \"%p\", (void *) p); q_free(p);",
    ] {
        let run = run(statements, false);
        assert!(run.safe(), "{statements}: {run:?}");
        assert!(!run.refused(&tail("fprintf")), "{statements}: {run:?}");
    }
    let run = run(
        "char *p = q_alloc(); q_free(p); fprintf(stderr, \"%p\", (void *) p);",
        false,
    );
    assert!(!run.safe(), "{run:?}");
}

/// A pointer name the body declares twice (shadowed) is not answered by rule
/// 10l, so it stays refused rather than being checked under the wrong object.
#[test]
fn a_shadowed_pointer_name_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = run(
        "char *p = q_alloc(); { char *p = q_alloc(); q_free(p); } \
         fprintf(stderr, \"%s\", p); q_free(p);",
        false,
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(&tail("fprintf")), "{run:?}");
}

/// A conditional alone in the tail is lowered by an older route first (rule
/// 10z: the condition decides, each arm is its own call with a literal tail),
/// readable or not; this card's reading is not asked there.
#[test]
fn a_lone_conditional_is_rule_10z_s() {
    if !clang_available() {
        return;
    }
    for readable in [false, true] {
        let run = run(
            "char *p = q_alloc(); fprintf(stderr, \"%s\", p ? \"set\" : \"none\"); q_free(p);",
            readable,
        );
        assert!(run.safe(), "{readable}: {run:?}");
    }
}
