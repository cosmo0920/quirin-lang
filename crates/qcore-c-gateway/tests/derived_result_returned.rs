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

//! Card W43-RESULT-ALIAS-RETURNED — inferred derived-result summaries.
//!
//! A body whose every exit returns either NULL or a pointer derived from one
//! borrowed formal gets the summary a `resource_return_alias` /
//! `resource_return_interior` declaration would have stated, validated against
//! a fresh analysis of the body. Callers apply it through the unchanged
//! W43-LIBC-RESULT-ALIAS rules. Every refused shape the card names has a test
//! below that pins both the wrapper's missing summary and its caller's
//! resulting `call to uncontracted function` refusal.
//!
//! The fixtures use the generic `d_*` surface of `derived_result.rs`
//! (`fixtures/contracts/derived_result_generic.qcore`): `Safe` here is `Safe`
//! over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, ContractTrust, DerivedExtent, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/derived_result_generic.qv");

const DECLARATIONS: &str = "\
char *d_alloc(__SIZE_TYPE__ n); \
void d_free(void *p); \
void d_consume(void *p); \
__SIZE_TYPE__ d_len(const char *s); \
char *d_chr(const char *s, int c); \
void *d_copy(void *restrict dst, const void *restrict src, __SIZE_TYPE__ n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "derived_result_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the derived-result fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent derived-result test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
        LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
        LegacyDiagnosticKind::CustodyConflict { .. } => "custody_conflict".to_owned(),
        LegacyDiagnosticKind::InvalidRelease { .. } => "invalid_release".to_owned(),
        LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
        LegacyDiagnosticKind::KindMismatchedRelease { .. } => "invalid_release".to_owned(),
        LegacyDiagnosticKind::UseAfterClose { .. } => "use_after_release".to_owned(),
        LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of
/// `function` within the analysis of `body`.
fn function_outcome(result: &UnrestrictedLegacyResult, function: &str) -> (String, Vec<String>) {
    let summary = program(result)
        .functions
        .get(function)
        .unwrap_or_else(|| panic!("missing function `{function}`"));
    let mut keys = summary
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (format!("{:?}", summary.status), keys)
}

fn unit_status(result: &UnrestrictedLegacyResult) -> &'static str {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    }
}

/// Assert each listed function's status and complete diagnostic vector.
fn expect(label: &str, body: &str, unit: &str, functions: &[(&str, &str, &[&str])]) {
    let result = analyze(body);
    assert_eq!(unit_status(&result), unit, "{label}: unit status\n{body}");
    for (function, status, vector) in functions {
        let (measured, keys) = function_outcome(&result, function);
        assert_eq!(
            (
                measured.as_str(),
                keys.iter().map(String::as_str).collect::<Vec<_>>()
            ),
            (*status, vector.to_vec()),
            "{label}: `{function}`\n{body}"
        );
    }
}

/// The inferred summaries a fully `Safe` unit's certificate carries.
fn certified_summaries(body: &str) -> BTreeMap<String, ResourceFunctionContract> {
    let result = analyze(body);
    let UnrestrictedLegacyResult::Safe(certificate) = &result else {
        panic!("expected a Safe unit, got {}\n{body}", unit_status(&result));
    };
    check_legacy_safety_certificate(certificate).expect("the certificate rechecks");
    certificate
        .summaries
        .iter()
        .inspect(|record| assert_eq!(record.trust, ContractTrust::InferredValidated))
        .map(|record| (record.function.clone(), record.contract.clone()))
        .collect()
}

fn derived(
    parameters: usize,
    parameter: usize,
    extent: DerivedExtent,
    nullable: bool,
    const_dropped: bool,
) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; parameters],
        ResourceReturnMode::Derived {
            parameter,
            extent,
            nullable,
            const_dropped,
        },
    )
}

const UNCONTRACTED: &str = "unsupported:call to uncontracted function";

// -- Admitted shapes -----------------------------------------------------------

#[test]
fn an_exact_wrapper_gets_an_exact_summary_and_certifies() {
    if !clang_available() {
        return;
    }
    // `XXH_memcpy`'s shape.
    let summaries = certified_summaries(
        "void *wcopy(void *dest, const void *src, unsigned long size) \
         { return d_copy(dest, src, size); } \
         void w(void) { char *b; b = d_alloc(8); wcopy(b, \"x\", 1); d_free(b); }",
    );
    assert_eq!(
        summaries.get("wcopy"),
        Some(&derived(3, 0, DerivedExtent::Exact, false, false))
    );
}

#[test]
fn a_null_guarded_interior_wrapper_gets_a_nullable_summary() {
    if !clang_available() {
        return;
    }
    // `mk_string_casestr`'s shape: a null-constant exit beside the derived one.
    let summaries = certified_summaries(
        "char *wfind(char *heystack, char *needle) \
         { if (!heystack || !needle) { return ((void *)0); } return d_chr(heystack, 58); } \
         void w(char *s) { char *p; p = wfind(s, s); if (p) { d_len(p); } }",
    );
    assert_eq!(
        summaries.get("wfind"),
        Some(&derived(2, 0, DerivedExtent::Interior, true, true))
    );
}

#[test]
fn a_derived_pointer_returned_through_a_name_is_admitted() {
    if !clang_available() {
        return;
    }
    let summaries = certified_summaries(
        "char *wname(const char *s) { char *p; p = d_chr(s, 58); return p; } \
         void w(const char *s) { char *p; p = wname(s); d_len(p); }",
    );
    assert_eq!(
        summaries.get("wname"),
        Some(&derived(1, 0, DerivedExtent::Interior, true, true))
    );
}

#[test]
fn the_base_may_be_any_formal_and_other_formals_keep_their_modes() {
    if !clang_available() {
        return;
    }
    // The base is the second formal; the first is released by the body and
    // keeps the `Released` mode the ordinary search reads off it.
    let summaries = certified_summaries(
        "void *wsecond(char *gone, void *dest) { d_free(gone); return d_copy(dest, \"x\", 1); } \
         void w(void) { char *a; char *b; a = d_alloc(8); b = d_alloc(8); \
         wsecond(a, b); d_free(b); }",
    );
    assert_eq!(
        summaries.get("wsecond"),
        Some(&ResourceFunctionContract::new(
            [
                ResourceParameterMode::Released { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::Derived {
                parameter: 1,
                extent: DerivedExtent::Exact,
                nullable: false,
                const_dropped: false,
            },
        ))
    );
}

#[test]
fn wrappers_compose_and_an_interior_step_makes_the_chain_interior() {
    if !clang_available() {
        return;
    }
    let summaries = certified_summaries(
        "void *wcopy(void *dest, const void *src, unsigned long size) \
         { return d_copy(dest, src, size); } \
         void *wcopy2(void *dest) { return wcopy(dest, \"x\", 1); } \
         void w(const char *s) { d_len(s); }",
    );
    assert_eq!(
        summaries.get("wcopy2"),
        Some(&derived(1, 0, DerivedExtent::Exact, false, false))
    );
    let summaries = certified_summaries(
        "void *wcopy(void *dest, const void *src, unsigned long size) \
         { return d_copy(dest, src, size); } \
         char *wscan(char *s) { char *p; p = wcopy(s, \"x\", 1); return d_chr(p, 58); } \
         void w(const char *s) { d_len(s); }",
    );
    assert_eq!(
        summaries.get("wscan"),
        Some(&derived(1, 0, DerivedExtent::Interior, true, true))
    );
}

#[test]
fn an_offset_base_is_an_interior_pointer_not_an_exact_alias() {
    if !clang_available() {
        return;
    }
    // `memcpy(p + 1, …)` returns `p + 1`, which is not `p`.
    let summaries = certified_summaries(
        "void *woff(char *dest) { return d_copy(dest + 1, \"x\", 1); } \
         void w(const char *s) { d_len(s); }",
    );
    assert_eq!(
        summaries.get("woff"),
        Some(&derived(1, 0, DerivedExtent::Interior, false, false))
    );
}

// -- Callers apply the summary exactly as a declaration --------------------------

/// The caller `w` gets exactly the status and diagnostic vector through the
/// wrapper that it gets from the declared derived callee directly, and the
/// wrapper itself certifies. `call` is spelled with `$` for the callee.
fn parity(
    label: &str,
    wrapper: &str,
    name: &str,
    direct: &str,
    caller: &str,
) -> (String, Vec<String>) {
    let through = analyze(&format!("{wrapper} {}", caller.replace('$', name)));
    let declared = analyze(&caller.replace('$', direct));
    let (status, keys) = function_outcome(&through, name);
    assert_eq!(
        (status.as_str(), keys.len()),
        ("Safe", 0),
        "{label}: `{name}`"
    );
    let measured = function_outcome(&through, "w");
    assert_eq!(
        measured,
        function_outcome(&declared, "w"),
        "{label}: the wrapper must be applied exactly as `{direct}`"
    );
    assert_eq!(
        unit_status(&through),
        unit_status(&declared),
        "{label}: unit"
    );
    measured
}

const WCOPY: &str = "void *wcopy(void *dst, const void *src, unsigned long n) \
                     { return d_copy(dst, src, n); }";
const WCHR: &str = "char *wchr(const char *s, int c) { if (!s) { return 0; } return d_chr(s, c); }";

#[test]
fn a_wrapper_result_used_after_its_base_is_released_is_the_same_candidate() {
    if !clang_available() {
        return;
    }
    let exact = parity(
        "exact",
        WCOPY,
        "wcopy",
        "d_copy",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = $(b, \"x\", 1); \
         d_free(b); d_len(q); }",
    );
    assert_eq!(exact.0, "Unsafe");
    assert!(exact.1.contains(&"use_after_release".to_owned()));
    let interior = parity(
        "interior",
        WCHR,
        "wchr",
        "d_chr",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = $(b, 58); \
         d_free(b); d_len(p); }",
    );
    assert_eq!(
        interior,
        ("Unsafe".to_owned(), vec!["use_after_release".to_owned()])
    );
}

#[test]
fn releasing_an_interior_wrapper_result_is_the_same_candidate() {
    if !clang_available() {
        return;
    }
    let measured = parity(
        "free(wchr(b)) then free(b)",
        WCHR,
        "wchr",
        "d_chr",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = $(b, 58); \
         d_free(p); d_free(b); }",
    );
    assert_eq!(
        measured,
        ("Unsafe".to_owned(), vec!["invalid_release".to_owned()])
    );
}

#[test]
fn releasing_an_exact_wrapper_result_is_refused_not_credited() {
    if !clang_available() {
        return;
    }
    // W43-RESULT-ALIAS-EXACT-RELEASE stays refused: the base carries an
    // unresolved effect, so no leak is claimed and nothing is credited.
    let measured = parity(
        "free(wcopy(b, …))",
        WCOPY,
        "wcopy",
        "d_copy",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = $(b, \"x\", 1); d_free(q); }",
    );
    assert_eq!(
        measured,
        ("Unknown".to_owned(), vec!["unknown_alias:q".to_owned()])
    );
}

#[test]
fn the_base_stays_the_callers_to_release() {
    if !clang_available() {
        return;
    }
    // The summary discharges nothing: releasing the base once certifies, and
    // forgetting it is a leak.
    let released = parity(
        "the base released once after the wrapper",
        WCOPY,
        "wcopy",
        "d_copy",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = $(b, \"x\", 1); \
         d_len(q); d_free(b); }",
    );
    assert_eq!(released, ("Safe".to_owned(), Vec::new()));
    let leaked = parity(
        "the base never released",
        WCOPY,
        "wcopy",
        "d_copy",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = $(b, \"x\", 1); d_len(q); }",
    );
    assert_eq!(leaked, ("Unsafe".to_owned(), vec!["leak".to_owned()]));
    let null_tested = parity(
        "a null test of the interior result narrows only it",
        WCHR,
        "wchr",
        "d_chr",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = $(b, 58); \
         if (!p) { return; } d_free(b); }",
    );
    assert_eq!(null_tested, ("Unsafe".to_owned(), vec!["leak".to_owned()]));
}

// -- Refused shapes ------------------------------------------------------------

/// The wrapper keeps exactly its pre-card refusal vector (the ordinary
/// contract-free analysis) and its caller sees an uncontracted call.
fn refused(label: &str, wrapper: &str, name: &str, call: &str, wrapper_vector: &[&str]) {
    let body = format!("{wrapper} void w(char *s) {{ char *p; p = {call}; d_len(p); }}");
    let result = analyze(&body);
    let (status, keys) = function_outcome(&result, name);
    assert_ne!(status, "Safe", "{label}: `{name}` must not certify\n{body}");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        wrapper_vector.to_vec(),
        "{label}: `{name}`\n{body}"
    );
    let (status, keys) = function_outcome(&result, "w");
    assert_eq!(status, "Unknown", "{label}: caller\n{body}");
    assert!(
        keys.iter()
            .any(|key| key.starts_with(UNCONTRACTED) && key.contains(name)),
        "{label}: the caller must see an uncontracted call, got {keys:?}\n{body}"
    );
}

const RETURN_REFUSAL: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

#[test]
fn mixed_exits_are_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "a fresh resource on one path",
        "char *wmix(char *d) { if (!d) { return d_alloc(8); } return d_copy(d, \"x\", 1); }",
        "wmix",
        "wmix(s)",
        &[RETURN_REFUSAL],
    );
    refused(
        "an untracked pointer on one path",
        "char *g; char *wmix(char *d) { if (!d) { return g; } return d_copy(d, \"x\", 1); }",
        "wmix",
        "wmix(s)",
        &[RETURN_REFUSAL],
    );
    refused(
        "an offset of the formal on one path",
        "char *wmix(char *d, int n) { if (n) { return d + 1; } return d_copy(d, \"x\", 1); }",
        "wmix",
        "wmix(s, 1)",
        &[RETURN_REFUSAL],
    );
    refused(
        "different formals on different paths",
        "char *wmix(char *d, char *e) { if (!e) { return d_copy(d, \"x\", 1); } \
         return d_copy(e, \"x\", 1); }",
        "wmix",
        "wmix(s, s)",
        &[RETURN_REFUSAL],
    );
}

#[test]
fn a_result_derived_from_a_formal_the_body_releases_consumes_or_returns_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "released base",
        "char *wrel(char *d) { char *p; p = d_chr(d, 58); d_free(d); return p; }",
        "wrel",
        "wrel(s)",
        &["unknown_alias:d", "unknown_alias:p"],
    );
    refused(
        "consumed base",
        "char *wrel(char *d) { char *p; p = d_chr(d, 58); d_consume(d); return p; }",
        "wrel",
        "wrel(s)",
        &["unknown_alias:d", "unknown_alias:p"],
    );
    refused(
        "released on a path that returns NULL",
        "char *wrel(char *d, int n) { if (n) { d_free(d); return 0; } \
         return d_copy(d, \"x\", 1); }",
        "wrel",
        "wrel(s, 1)",
        &["unknown_alias:d", RETURN_REFUSAL],
    );
}

#[test]
fn a_result_derived_from_a_non_formal_base_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "a local resource",
        "char *wlocal(char *d) { char *b; b = d_alloc(8); d_len(d); return d_copy(b, \"x\", 1); }",
        "wlocal",
        "wlocal(s)",
        // Card W44-LEAK-PIN-AUDIT: the refused return hands `b` to `d_copy`,
        // so the leak it used to accuse is withdrawn (still non-`Safe`).
        &[RETURN_REFUSAL],
    );
    refused(
        "a string literal",
        "char *wlit(char *d) { d_len(d); return d_chr(\"abc\", 58); }",
        "wlit",
        "wlit(s)",
        &[RETURN_REFUSAL],
    );
}

#[test]
fn a_derivation_through_a_mutated_local_copy_is_refused() {
    if !clang_available() {
        return;
    }
    // Integration of W43-INDEX-TRAVERSAL: a copy stepped forward by a literal
    // amount is an interior pointer strictly after the formal's start, so the
    // wrapper now publishes an interior derived result and its caller, which
    // only reads through it, certifies. Before traversal the copy's binding
    // was dropped and the wrapper was refused.
    for (label, wrapper) in [
        (
            "a copy offset forward in place",
            "char *wmut(char *d) { char *q; q = d; q = q + 1; return d_copy(q, \"x\", 1); }",
        ),
        (
            "a copy incremented",
            "char *wmut(char *d) { char *q; q = d; q++; return d_copy(q, \"x\", 1); }",
        ),
    ] {
        let body = format!(
            "{wrapper} void w(char *s) {{ char *p; p = wmut(s); d_len(p); }} \
             void x(void) {{ char *b; char *p; b = d_alloc(8); p = wmut(b); d_free(p); d_free(b); }}"
        );
        let result = analyze(&body);
        assert_eq!(
            function_outcome(&result, "wmut"),
            ("Safe".to_owned(), vec![]),
            "{label}\n{body}"
        );
        assert_eq!(
            function_outcome(&result, "w"),
            ("Safe".to_owned(), vec![]),
            "{label}\n{body}"
        );
        // Releasing `b + 1` is not a release of `b`: an invalid-release candidate.
        assert_eq!(
            function_outcome(&result, "x"),
            ("Unsafe".to_owned(), vec!["invalid_release".to_owned()]),
            "{label}\n{body}"
        );
    }
    // A copy that may be stepped back to the formal's start is never published
    // as derived, so the wrapper keeps a refusal and its caller sees an
    // uncontracted call.
    for (label, wrapper) in [
        (
            "a copy offset backward in place",
            "char *wmut(char *d) { char *q; q = d; q = q - 1; return d_copy(q, \"x\", 1); }",
        ),
        (
            "a copy decremented",
            "char *wmut(char *d) { char *q; q = d; q--; return d_copy(q, \"x\", 1); }",
        ),
    ] {
        refused(
            label,
            wrapper,
            "wmut",
            "wmut(s)",
            &["unknown_alias:d", RETURN_REFUSAL],
        );
    }
}

// -- The offset base -------------------------------------------------------------

#[test]
fn an_offset_over_a_tracked_resource_never_reaches_the_derived_rule() {
    if !clang_available() {
        return;
    }
    // The interior reading of `place ± offset` (card W43-RESULT-ALIAS-RETURNED)
    // is reachable only over a base this projection does not track — a
    // borrowed formal, which is exactly the inference case
    // (`an_offset_base_is_an_interior_pointer_not_an_exact_alias`). Over a
    // tracked resource the pre-existing borrowed-expression gate refuses the
    // call first, so the reading moves no verdict there.
    expect(
        "free(memcpy(b + 1, ...)) then free(b)",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = d_copy(b + 1, \"x\", 1); \
         d_free(q); d_free(b); }",
        "Unknown",
        &[(
            "w",
            "Unknown",
            &[
                "unknown_alias:q",
                "unsupported:call has an unmodeled formal address or borrowed expression",
            ],
        )],
    );
}

// Rule 36: these use the same synthetic trusted fixture contracts as above.
#[test]
fn exact_original_formals_publish_checked_borrow_relations() {
    if !clang_available() {
        return;
    }
    for (definition, call, count, parameter, nullable) in [
        (
            "char *identity(char *p) { return p; }",
            "identity(p)",
            1,
            0,
            false,
        ),
        (
            "char *identity(char *p) { if (!p) return 0; return p; }",
            "identity(p)",
            1,
            0,
            true,
        ),
        (
            "char *identity(int n, char *p) { return p; }",
            "identity(1,p)",
            2,
            1,
            false,
        ),
        (
            "const char *identity(const char *p) { return p; }",
            "identity(p)",
            1,
            0,
            false,
        ),
        (
            "char *identity(char *const p) { return p; }",
            "identity(p)",
            1,
            0,
            false,
        ),
        (
            "char *identity(char *p) { d_copy(p,\"x\",1); return p; }",
            "identity(p)",
            1,
            0,
            false,
        ),
        (
            "char *identity(char *p,int n) { if(n) return p; return d_copy(p,\"x\",1); }",
            "identity(p,1)",
            2,
            0,
            false,
        ),
        (
            "char *identity(char *p,int n) { if(n) return p; return p; }",
            "identity(p,1)",
            2,
            0,
            false,
        ),
    ] {
        let body = format!(
            "{definition} void w(void) {{ char *p=d_alloc(8); if(!p)return; const char *q={call}; if(q) d_len(q); d_free(p); }}"
        );
        let summaries = certified_summaries(&body);
        assert_eq!(
            summaries.get("identity"),
            Some(&derived(
                count,
                parameter,
                DerivedExtent::Exact,
                nullable,
                false
            )),
            "{definition}"
        );
    }
}

#[test]
fn exact_formal_results_keep_caller_lifetime_and_allocation_obligations() {
    if !clang_available() {
        return;
    }
    let helper = "char *identity(char *p) { return p; }";
    for (caller, required) in [
        (
            "void w(void) { char*p=d_alloc(8); if(!p)return; identity(p); }",
            "leak",
        ),
        (
            "void w(void) { char*p=d_alloc(8); if(!p)return; char*q=identity(p); d_free(p); d_len(q); }",
            "use_after_release",
        ),
        (
            "void w(void) { char*p=d_alloc(8); if(!p)return; char*q=identity(p); d_free(q); }",
            "unknown_alias:q",
        ),
    ] {
        let result = analyze(&format!("{helper} {caller}"));
        let (status, diagnostics) = function_outcome(&result, "w");
        assert_ne!(status, "Safe");
        assert!(
            diagnostics.iter().any(|d| d == required),
            "{caller}: {diagnostics:?}"
        );
    }
}

#[test]
fn unproved_effects_and_rebound_or_shadowed_formals_do_not_certify() {
    if !clang_available() {
        return;
    }
    for definition in [
        "char *identity(char*p,char*q) { p=q; return p; }",
        "char *identity(char*p,char*q) { p=p; return p; }",
        "char *identity(char*p,char*q) { { char *p=q; return p; } }",
        "char *identity(char*p,char*q) { { enum { p=0 }; return p; } }",
        "char *identity(char*p,char*q) { d_free(p); return p; }",
        "char *identity(char*p,char*q) { d_consume(p); return p; }",
        "void mystery(void); char *identity(char*p,char*q) { mystery(); return p; }",
        "char *identity(char*p,char*q) { char*x=d_alloc(8); return p; }",
        "struct metric { char *value; }; char *identity(char*p,struct metric*q) { q->value=d_alloc(8); return p; }",
    ] {
        let body = format!("{definition} void w(char*p) {{ char*q=identity(p,0); d_len(q); }}");
        let result = analyze(&body);
        assert_ne!(function_outcome(&result, "w").0, "Safe", "{definition}");
    }
}

#[test]
fn older_assigned_derived_route_keeps_its_summary() {
    if !clang_available() {
        return;
    }
    let summaries = certified_summaries(
        "char *identity(char*p) { p=d_copy(p,\"x\",1); return p; } void w(void) {}",
    );
    assert_eq!(
        summaries.get("identity"),
        Some(&derived(1, 0, DerivedExtent::Exact, false, false))
    );
}

#[test]
fn exact_summary_does_not_turn_a_null_actual_into_a_nonnull_result() {
    if !clang_available() {
        return;
    }
    let summaries = certified_summaries("char *identity(char*p) { return p; } void w(void) {}");
    let mut environment = contracts();
    environment.insert_contract("identity", summaries["identity"].clone());
    let source = "char *identity(char*p) { return p; } int w(void) { char*p=identity((char*)0); return *p; }";
    let report = qcore_c_gateway::null_deref::analyze_null_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &["identity".into(), "w".into()],
        &environment,
        &qcore_contract::bounds_contract::BoundsContracts::default(),
    )
    .unwrap();
    let caller = report.functions.iter().find(|r| r.function == "w").unwrap();
    assert_ne!(caller.verdict, "Safe", "{caller:?}");
    assert!(
        !report
            .functions
            .iter()
            .find(|r| r.function == "identity")
            .unwrap()
            .returns_non_null
    );
}

#[test]
fn exact_formal_certificate_replay_rejects_a_wrong_parameter_record() {
    if !clang_available() {
        return;
    }
    let result = analyze("char *identity(char*p) { return p; } void w(void) {}");
    let UnrestrictedLegacyResult::Safe(mut certificate) = result else {
        panic!("identity must certify")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    certificate
        .summaries
        .iter_mut()
        .find(|r| r.function == "identity")
        .unwrap()
        .contract
        .result = ResourceReturnMode::Derived {
        parameter: 1,
        extent: DerivedExtent::Exact,
        nullable: false,
        const_dropped: false,
    };
    assert!(check_legacy_safety_certificate(&certificate).is_err());
}

#[test]
fn returning_an_exact_borrow_of_a_fresh_allocation_is_not_an_owned_handoff() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "char *identity(char*p) { return p; } char *carrier(void) { char*p=d_alloc(8); if(!p)return 0; return identity(p); } void w(void) {}",
    );
    assert_ne!(function_outcome(&result, "carrier").0, "Safe");
}

#[test]
fn a_released_other_formal_cannot_revive_an_alias_when_actuals_coincide() {
    if !clang_available() {
        return;
    }
    let helper = "char *identity(char*p,char*q) { d_free(q); return p; }";
    let summaries = certified_summaries(&format!("{helper} void w(void) {{}}"));
    let contract = &summaries["identity"];
    assert_eq!(contract.parameters[0], ResourceParameterMode::Borrowed);
    assert!(matches!(
        contract.parameters[1],
        ResourceParameterMode::Released { .. }
    ));
    assert_eq!(
        contract.result,
        ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Exact,
            nullable: false,
            const_dropped: false
        }
    );
    let result = analyze(&format!(
        "{helper} void w(void) {{ char*p=d_alloc(8); if(!p)return; char*q=identity(p,p); d_len(q); }}"
    ));
    let (status, diagnostics) = function_outcome(&result, "w");
    assert_ne!(status, "Safe");
    assert!(
        diagnostics.iter().any(|d| d == "use_after_release"),
        "{diagnostics:?}"
    );
}

#[test]
fn an_unresolved_exact_alias_base_cannot_make_null_search_infeasible() {
    if !clang_available() {
        return;
    }
    let summaries = certified_summaries("char *identity(char*p) { return p; } void w(void) {}");
    let mut environment = contracts();
    environment.insert_contract("identity", summaries["identity"].clone());
    let source = "char*external(void); char*identity(char*p){return p;} int w(void){char*p=external();char*q=identity(p);return*q;}";
    let report = qcore_c_gateway::null_deref::analyze_null_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &["identity".into(), "w".into()],
        &environment,
        &qcore_contract::bounds_contract::BoundsContracts::default(),
    )
    .unwrap();
    let caller = report.functions.iter().find(|r| r.function == "w").unwrap();
    assert_ne!(caller.verdict, "Safe", "{caller:?}");
    assert!(
        caller.triage.iter().all(|r| r.status != "infeasible"),
        "{caller:?}"
    );
}
