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

//! Card **CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE** (gateway rule 74n): the
//! failure-use clause `resource_kind_failure_use(preg, regex)`, which the
//! production `regex.qv` states on `regerror`'s `preg` alone, read by rule
//! 74i's certified-use check.
//!
//! * **Admitted:** `regerror(r, &re, …)` on the failure edge of `regcomp` and
//!   before the status test. Both refused at base (rule 74i's recorded
//!   over-refusal), so `rd_regex_match` as rdregex.c spells it was `Unknown`.
//! * **Still refused:** `regexec` (no clause) on the failure edge, `regerror`
//!   before any `regcomp`, and `regerror` after `regfree` (rule 11v's
//!   `use_after_close` candidate, never admitted by the clause).
//!
//! A `Safe` here is `Safe` over the libc assumptions and promotes nothing.
//! Lean: `QCore.InPlaceFailureUse`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const REGEX: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/regex.qv");

const DECLARATIONS: &str = "\
typedef struct { void *buffer; unsigned long re_nsub; } regex_t;
int regcomp(regex_t *preg, const char *pattern, int cflags);
int regexec(const regex_t *preg, const char *string, unsigned long nmatch, void *pmatch, int eflags);
unsigned long regerror(int errcode, const regex_t *preg, char *errbuf, unsigned long errbuf_size);
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

/// `rd_regex_match` as rdregex.c spells it under `HAVE_REGEX`
/// (`lib/librdkafka-2.15.0/src/rdregex.c`), verbatim from
/// `in_place_regex_certified.rs` at base.
const RD_REGEX_MATCH: &str = "int subject(const char *pattern, const char *str, char *errstr, unsigned long errstr_size) { \
     regex_t re; int r; r = regcomp(&re, pattern, 1 | 8); \
     if (r) { if (errstr) regerror(r, &re, errstr, errstr_size); return 0; } \
     r = regexec(&re, str, 0, 0, 0) != 1; regfree(&re); return r; }";

/// Change pin: `regerror` on the failure edge is admitted, and the real
/// `rd_regex_match` is `Safe` with no diagnostic. Both `Unknown` (rule 74i)
/// at base.
#[test]
fn a_regerror_on_the_failure_edge_is_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) { regerror(r, &re, buf, n); return 0; } \
         regfree(&re); return 1; }",
        RD_REGEX_MATCH,
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
        assert!(run.diagnostics.is_empty(), "{source}: {run:?}");
    }
}

/// Change pin: `regerror` before the status test is admitted: the object is
/// compiled or failed there, and either is defined for `regerror`. So is a
/// `regerror` after a join of the two edges. `Unknown` (rule 74i) at base.
#[test]
fn a_regerror_before_the_status_test_is_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); regerror(r, &re, buf, n); if (r) return 0; \
         regfree(&re); return 1; }",
        "int subject(const char *p, const char *s, char *buf, unsigned long n) { regex_t re; \
         int r; int t = 0; r = regcomp(&re, p, 0); if (r == 0) { t = regexec(&re, s, 0, 0, 0); } \
         regerror(r, &re, buf, n); if (r) return 0; regfree(&re); return t; }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
        assert!(run.diagnostics.is_empty(), "{source}: {run:?}");
    }
}

/// `regexec` carries no failure-use clause, so on the failure edge (probe
/// R3), before the test (probe R4) and after an admitted `regerror` on the
/// failure edge it is refused exactly as at base.
#[test]
fn a_regexec_on_the_failure_edge_is_still_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, const char *s) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) { (void) 0; r = regexec(&re, s, 0, 0, 0); return r; } \
         regfree(&re); return 1; }",
        "int subject(const char *p, const char *s) { regex_t re; int r; int t; \
         r = regcomp(&re, p, 0); t = regexec(&re, s, 0, 0, 0); if (r) return t; \
         regfree(&re); return t; }",
        "int subject(const char *p, const char *s, char *buf, unsigned long n) { regex_t re; \
         int r; r = regcomp(&re, p, 0); \
         if (r) { regerror(r, &re, buf, n); r = regexec(&re, s, 0, 0, 0); return r; } \
         regfree(&re); return 1; }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{source}: {run:?}");
        assert!(run.refused(UNCERTIFIED), "{source}: {run:?}");
    }
}

/// `regerror` before any `regcomp` names an object no initialiser touched:
/// refused, on a straight path and where only some paths reach an
/// initialiser.
#[test]
fn a_regerror_before_the_initialiser_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         regerror(1, &re, buf, n); r = regcomp(&re, p, 0); if (r) return 0; \
         regfree(&re); return 1; }",
        "int subject(const char *p, char *buf, unsigned long n, int c) { regex_t re; int r = 1; \
         if (c) { r = regcomp(&re, p, 0); if (r == 0) regfree(&re); } \
         if (r) regerror(r, &re, buf, n); return 0; }",
    ] {
        let run = run(source);
        assert_ne!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
        assert!(run.refused(UNCERTIFIED), "{source}: {run:?}");
    }
}

/// `regerror` after `regfree` is not admitted by the clause: it stays rule
/// 11v's `use_after_close` candidate, exactly as `regexec` there. So is a
/// `regerror` after a join with a path that freed the object.
#[test]
fn a_regerror_after_regfree_is_not_admitted() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) return 0; regfree(&re); \
         regerror(r, &re, buf, n); return 1; }",
        "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r == 0) regfree(&re); \
         regerror(r, &re, buf, n); return 1; }",
    ] {
        let run = run(source);
        assert_ne!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
        assert!(
            run.diagnostics
                .iter()
                .any(|kind| matches!(kind, LegacyDiagnosticKind::UseAfterClose { .. })),
            "{source}: {run:?}"
        );
    }
}

/// A `regfree` on the failure edge (POSIX: the object is "not a compiled
/// regular expression returned by regcomp()") followed by `regerror`: never
/// `Safe`.
#[test]
fn a_regerror_after_a_failure_edge_regfree_is_not_safe() {
    if !clang_available() {
        return;
    }
    let source = "int subject(const char *p, char *buf, unsigned long n) { regex_t re; int r; \
         r = regcomp(&re, p, 0); if (r) { regfree(&re); regerror(r, &re, buf, n); return 0; } \
         regfree(&re); return 1; }";
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
}
