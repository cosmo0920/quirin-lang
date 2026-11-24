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

//! Card **CG25-IN-PLACE-REGEX** (gateway rule 74i): the in-place `regex`
//! kind, read through the production libc declarations of `regcomp`,
//! `regexec`, `regerror` and `regfree` (`regex.qv`).
//!
//! * **Certified use.** A by-address use of a tracked in-place object
//!   (`regexec(&re, …)`, `regerror(r, &re, …)`) is admitted only where the
//!   settled state certifies the object: one identity, not unknown, not
//!   possibly failed, site live or released. A use before the status test,
//!   on the failure edge or before the initialiser refuses. Probes R3 and R4
//!   of the CG25-THREAD-KEY-AUTOMATIC-REVISIT report were `Safe` at base.
//! * **One declaration.** An in-place actual's name must be the one automatic
//!   object every `DeclRefExpr` of the name designates. A block-scope
//!   `regex_t` shadowing a file-scope one the body also names refuses. Probe
//!   R2 was `Safe` at base.
//!
//! The `rd_regex_match` shape without its failure-edge `regerror` is the
//! control: `Safe` at base and after. The real function calls `regerror(r,
//! &re, …)` on the failure edge, which the rule refuses with `regexec` (no
//! contract clause tells them apart): a recorded over-refusal.
//! A `Safe` here is `Safe` over the libc assumptions and promotes nothing.
//! Lean: `QCore.InPlaceCertifiedUse`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const REGEX: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/regex.qv");

// Target-portable spellings: plain `long` is unreadable to the declared-ABI
// reader unless the analysing Clang is LP64 (rule 11q), and `regex.qv` reads
// `re_nsub`, `nmatch`, `errbuf_size` and `regerror`'s result as `usize`, so
// these synthetic declarations spell POSIX `size_t` as `__SIZE_TYPE__`.
const DECLARATIONS: &str = "\
typedef struct { void *buffer; __SIZE_TYPE__ re_nsub; } regex_t;
int regcomp(regex_t *preg, const char *pattern, int cflags);
int regexec(const regex_t *preg, const char *string, __SIZE_TYPE__ nmatch, void *pmatch, int eflags);
__SIZE_TYPE__ regerror(int errcode, const regex_t *preg, char *errbuf, __SIZE_TYPE__ errbuf_size);
void regfree(regex_t *preg);
regex_t global_re;
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/regex.qv".to_owned(),
        REGEX.to_owned(),
    )]))
    .expect("the libc regex module loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn refused(&self, fragment: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.contains(fragment))
        })
    }
}

/// The rule 74i refusal of an uncertified use of `re`.
const UNCERTIFIED: &str =
    "use of in-place object `re` not certified by its initialiser status (rule 74i)";
/// The rule 11v (2) refusal, which rule 74i extends to a name that also
/// designates another object.
const ACTUAL: &str = "in-place resource object is not the address of one automatic object of this function (rule 11v)";

fn run(source: &str) -> Run {
    let text = format!("{DECLARATIONS}\n{source}\n");
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
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// The control: the `rd_regex_match` shape `tests/resource_kinds.rs` pins
/// (its failure-edge `regerror` replaced by a write of `errstr`) uses `re`
/// only after the zero edge. It is `Safe` at base and stays `Safe`, with no
/// diagnostic. The real spelling, with `regerror`, is in
/// `a_regerror_on_the_failure_edge_is_refused`.
#[test]
fn the_rd_regex_match_shape_stays_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *pattern, const char *str, char *errstr, unsigned long errstr_size) { \
         regex_t re; int r; r = regcomp(&re, pattern, 1 | 8); \
         if (r) { if (errstr) errstr[0] = 0; return 0; } \
         r = regexec(&re, str, 0, 0, 0) != 1; regfree(&re); return r; }",
        "int subject(const char *p, const char *s) { regex_t re; int r = regcomp(&re, p, 0); \
         if (r == 0) { r = regexec(&re, s, 0, 0, 0); regfree(&re); } return r; }",
        "int subject(const char *p, const char *s) { regex_t re; int r; r = regcomp(&re, p, 0); \
         if (r) return 0; r = regexec(&re, s, 0, 0, 0); r = regexec(&re, s, 0, 0, 0); \
         regfree(&re); return r; }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
        assert!(run.diagnostics.is_empty(), "{source}: {run:?}");
    }
}

/// Probe R4: `regexec(&re, …)` before `if (r)` tests the status. `Safe` at
/// base; refused now.
#[test]
fn a_use_before_the_status_test_is_refused() {
    if !clang_available() {
        return;
    }
    let source = "int subject(const char *p, const char *s) { regex_t re; int r; int t; \
         r = regcomp(&re, p, 0); t = regexec(&re, s, 0, 0, 0); if (r) return t; \
         regfree(&re); return t; }";
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(UNCERTIFIED), "{run:?}");
}

/// Probe R3: `regexec(&re, …)` on the nonzero (failure) edge. `Safe` at base;
/// refused now. POSIX leaves the object's contents undefined after a failed
/// `regcomp`.
#[test]
fn a_use_on_the_failure_edge_is_refused() {
    if !clang_available() {
        return;
    }
    let source = "int subject(const char *p, const char *s) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) { (void) 0; r = regexec(&re, s, 0, 0, 0); return r; } \
         regfree(&re); return 1; }";
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(UNCERTIFIED), "{run:?}");
}

/// A use before any initialiser reads an object nothing compiled. `Safe` at
/// base; refused now.
#[test]
fn a_use_before_the_initialiser_is_refused() {
    if !clang_available() {
        return;
    }
    let source = "int subject(const char *p, const char *s) { regex_t re; int r; int t; \
         t = regexec(&re, s, 0, 0, 0); r = regcomp(&re, p, 0); if (r) return t; \
         regfree(&re); return t; }";
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(UNCERTIFIED), "{run:?}");
}

/// `regerror(r, &re, …)` on the failure edge is a use of the same kind and
/// position, and the rule reads no callee by name, so it refuses too. POSIX
/// permits it there; this is a recorded over-refusal
/// (`CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE`), not a finding.
#[test]
fn a_regerror_on_the_failure_edge_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) { regerror(r, &re, buf, n); return 0; } \
         regfree(&re); return 1; }",
        // `rd_regex_match` as rdregex.c spells it under HAVE_REGEX: the
        // published `Safe` row this card moves to `Unknown`.
        "int subject(const char *pattern, const char *str, char *errstr, unsigned long errstr_size) { \
         regex_t re; int r; r = regcomp(&re, pattern, 1 | 8); \
         if (r) { if (errstr) regerror(r, &re, errstr, errstr_size); return 0; } \
         r = regexec(&re, str, 0, 0, 0) != 1; regfree(&re); return r; }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{source}: {run:?}");
        assert!(run.refused(UNCERTIFIED), "{source}: {run:?}");
    }
}

/// A use after `regfree` stays rule 11v's `use_after_close` candidate; the
/// released site is certified for this rule, so no refusal is added.
#[test]
fn a_use_after_regfree_stays_a_candidate() {
    if !clang_available() {
        return;
    }
    let source = "int subject(const char *p, const char *s) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) return 0; regfree(&re); \
         r = regexec(&re, s, 0, 0, 0); return r; }";
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(
        run.diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::UseAfterClose { .. })),
        "{run:?}"
    );
    assert!(!run.refused(UNCERTIFIED), "{run:?}");
}

/// A use through a pointer this body does not track keeps the plain borrowed
/// reading: no obligation of this body is touched.
#[test]
fn an_untracked_use_keeps_the_borrowed_reading() {
    if !clang_available() {
        return;
    }
    let run =
        run("int subject(const regex_t *re, const char *s) { return regexec(re, s, 0, 0, 0); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Probe R2: a block-scope `regex_t global_re` shadows the file-scope one, and
/// the `regfree` after the block frees the file-scope object. `Safe` at base,
/// though the local is never freed; refused now. So is the same name read
/// before the local's declarator.
#[test]
fn a_shadowing_local_beside_a_file_scope_reference_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p) { { regex_t global_re; int r; \
         r = regcomp(&global_re, p, 0); if (r) return 0; } regfree(&global_re); return 1; }",
        "int subject(const char *p, const char *s) { int t; t = regexec(&global_re, s, 0, 0, 0); \
         { regex_t global_re; int r; r = regcomp(&global_re, p, 0); if (r) return t; \
         regfree(&global_re); } return t; }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{source}: {run:?}");
        assert!(run.refused(ACTUAL), "{source}: {run:?}");
    }
}

/// A shadowing local whose every reference is its own is the one object, and
/// is tracked: freed, it is `Safe`; never freed, a `leak` candidate.
#[test]
fn a_shadowing_local_alone_is_the_one_object() {
    if !clang_available() {
        return;
    }
    let run_freed = run("int subject(const char *p) { { regex_t global_re; int r; \
         r = regcomp(&global_re, p, 0); if (r) return 0; regfree(&global_re); } return 1; }");
    assert_eq!(run_freed.status, AnalysisStatus::Safe, "{run_freed:?}");
    let run_leaked = run("int subject(const char *p) { { regex_t global_re; int r; \
         r = regcomp(&global_re, p, 0); if (r) return 0; } return 1; }");
    assert_eq!(run_leaked.status, AnalysisStatus::Unsafe, "{run_leaked:?}");
    assert!(
        run_leaked
            .diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "{run_leaked:?}"
    );
}
