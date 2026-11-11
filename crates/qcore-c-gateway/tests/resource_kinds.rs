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

//! Card **CG25-RESOURCE-KINDS** (gateway rule 11v) — the `dynamic_library`
//! kind and the in-place `regex` kind.
//!
//! Every body below asserts the **complete** diagnostic vector of the function
//! together with its status, as `descriptor_custody.rs` does, so a rule that
//! silently dropped a pre-existing finding while adding its own would fail.
//! The fixtures use a generic `k_*` surface (`fixtures/contracts/
//! resource_kinds_generic.qv`); `Safe` here is `Safe` over synthetic trusted
//! test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, ResourceKind, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/resource_kinds_generic.qv");

/// The C declarations the fixture contracts describe.
const DECLARATIONS: &str = "\
typedef struct { void *buffer; unsigned long used; } regex_t; \
void *k_dlopen(const char *path); \
int k_dlclose(void *handle); \
void *k_dlsym(void *handle, const char *name); \
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
int k_regcomp(regex_t *preg, const char *pattern, int flags); \
int k_regexec(const regex_t *preg, const char *text); \
void k_regfree(regex_t *preg); \
void k_wipe(void *p, __SIZE_TYPE__ n); \
struct k_holder { regex_t re; }; \
regex_t g_re;";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "resource_kinds_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the resource-kind fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent resource-kind test: {reason}");
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
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquired, released, ..
        } => format!(
            "kind_mismatched_release:{}<-{}",
            acquired.spelling(),
            released.spelling()
        ),
        LegacyDiagnosticKind::UseAfterClose { kind, .. } => {
            format!("use_after_close:{}", kind.spelling())
        }
        LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
    }
}

/// The program status and the complete, sorted, de-duplicated diagnostic
/// vector of `w`. A `Safe` result's certificate must recheck.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result)
        .functions
        .get("w")
        .unwrap_or_else(|| panic!("missing function `w`"));
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    let _ = matches!(function.status, AnalysisStatus::Safe);
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    let (measured, keys) = outcome(body);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

const GUARD: &str = "unsupported:tracked in-place object `re` used outside a declared in-place position is outside the resource projection (rule 11v)";
const FORMAL_ADDRESS: &str =
    "unsupported:call has an unmodeled formal address or borrowed expression";
const WITNESS: &str = "unsupported:in-place initialiser status not held by a plain, not address-taken name (rule 11v)";
const ACTUAL: &str = "unsupported:in-place resource object is not the address of one automatic object of this function (rule 11v)";

// -- The loader ---------------------------------------------------------------

#[test]
fn the_two_kinds_load_onto_their_positions() {
    let contracts = contracts();
    let open = contracts.contract("k_dlopen").expect("declared");
    assert_eq!(open.result_kind, ResourceKind::DynamicLibrary);
    assert_eq!(
        contracts
            .contract("k_dlclose")
            .expect("declared")
            .parameter_kind(0),
        ResourceKind::DynamicLibrary
    );
    for name in ["k_regcomp", "k_regexec", "k_regfree"] {
        assert_eq!(
            contracts
                .contract(name)
                .expect("declared")
                .parameter_kind(0),
            ResourceKind::Regex,
            "{name}"
        );
    }
    assert!(ResourceKind::Regex.is_in_place());
    assert!(!ResourceKind::DynamicLibrary.is_in_place());
    assert!(!ResourceKind::DynamicLibrary.is_integer());
}

// -- dynamic_library -----------------------------------------------------------

#[test]
fn a_guarded_library_handle_closed_once_is_safe() {
    if !clang_available() {
        return;
    }
    expect(
        "open, test, close",
        "void w(void) { void *h = k_dlopen(\"x\"); if (!h) return; k_dlclose(h); }",
        "Safe",
        &[],
    );
    expect(
        "open, test, look up, close",
        "void w(void) { void *h = k_dlopen(\"x\"); void *s; if (h == 0) return; s = k_dlsym(h, \"f\"); k_dlclose(h); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_library_handle_is_not_a_heap_pointer() {
    if !clang_available() {
        return;
    }
    expect(
        "dlclose on a malloc pointer",
        "void w(void) { void *p = k_alloc(8); if (!p) return; k_dlclose(p); }",
        "Unsafe",
        &["kind_mismatched_release:heap<-dynamic_library", "leak"],
    );
    expect(
        "free on a dlopen handle",
        "void w(void) { void *h = k_dlopen(\"x\"); if (!h) return; k_free(h); }",
        "Unsafe",
        &["kind_mismatched_release:dynamic_library<-heap", "leak"],
    );
}

#[test]
fn the_library_lifecycle_violations_are_candidates() {
    if !clang_available() {
        return;
    }
    expect(
        "double dlclose",
        "void w(void) { void *h = k_dlopen(\"x\"); if (!h) return; k_dlclose(h); k_dlclose(h); }",
        "Unsafe",
        &["double_release"],
    );
    expect(
        "dlsym after dlclose",
        "void w(void) { void *h = k_dlopen(\"x\"); if (!h) return; k_dlclose(h); k_dlsym(h, \"f\"); }",
        "Unsafe",
        &["unknown_alias:h", "use_after_close:dynamic_library"],
    );
    expect(
        "a handle never closed",
        "void w(void) { void *h = k_dlopen(\"x\"); if (!h) return; k_dlsym(h, \"f\"); }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn the_null_sentinel_refines_and_its_absence_does_not() {
    if !clang_available() {
        return;
    }
    // Without the test, the handle may be NULL at a `nonnull` release.
    let (status, _) = outcome("void w(void) { void *h = k_dlopen(\"x\"); k_dlclose(h); }");
    assert_ne!(status, "Safe");
    // A returned handle escapes (the cast-return repair of W44-LEAK-PIN-AUDIT).
    expect(
        "returned through a cast",
        "typedef struct lib lib_t; lib_t *w(void) { void *h = k_dlopen(\"x\"); if (!h) return 0; return (lib_t *)h; }",
        "Safe",
        &[],
    );
}

// -- regex (in place) ----------------------------------------------------------

#[test]
fn a_tested_compile_freed_once_is_safe() {
    if !clang_available() {
        return;
    }
    expect(
        "the rd_regex_match shape",
        "int w(const char *p, const char *s) { regex_t re; int r; r = k_regcomp(&re, p, 0); if (r) { return 0; } r = k_regexec(&re, s); k_regfree(&re); return r; }",
        "Safe",
        &[],
    );
    expect(
        "the rd_regex_match shape, verbatim (rdregex.c)",
        "int w(const char *pattern, const char *str, char *errstr, unsigned long errstr_size) { \
         regex_t re; int r; r = k_regcomp(&re, pattern, 1 | 8); \
         if (r) { if (errstr) k_wipe(errstr, errstr_size); return 0; } \
         r = k_regexec(&re, str) != 1; k_regfree(&re); return r; }",
        "Safe",
        &[],
    );
    expect(
        "zero test",
        "int w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); if (r == 0) { k_regfree(&re); } return r; }",
        "Safe",
        &[],
    );
}

#[test]
fn an_untested_status_decides_nothing() {
    if !clang_available() {
        return;
    }
    // Finalised without knowing the compile succeeded.
    let (status, _) =
        outcome("void w(const char *p) { regex_t re; k_regcomp(&re, p, 0); k_regfree(&re); }");
    assert_ne!(status, "Safe");
    // The status overwritten before the test witnesses nothing.
    let (status, _) = outcome(
        "void w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); r = 0; if (r) return; k_regfree(&re); }",
    );
    assert_ne!(status, "Safe");
    // A status no plain name holds is refused outright: discarded, or
    // consumed by the condition that tests it.
    expect(
        "discarded status",
        "void w(const char *p) { regex_t re; k_regcomp(&re, p, 0); }",
        "Unknown",
        &[WITNESS],
    );
    expect(
        "status consumed by its condition: the condition route keeps its refusal",
        "void w(const char *p) { regex_t re; if (k_regcomp(&re, p, 0) != 0) return; k_regfree(&re); }",
        "Unknown",
        &[
            "unknown_alias:re",
            "unsupported:unmodeled call, aggregate or indirect access in branch condition",
        ],
    );
    // The condition's own assignment holds it.
    expect(
        "assigned in the condition",
        "void w(const char *p) { regex_t re; int r; if ((r = k_regcomp(&re, p, 0)) != 0) return; k_regfree(&re); }",
        "Safe",
        &[],
    );
    // An address-taken status is refused outright.
    let (status, keys) = outcome(
        "void w(const char *p) { regex_t re; int r; int *q = &r; r = k_regcomp(&re, p, 0); if (r) return; k_regfree(&re); }",
    );
    assert_eq!(status, "Unknown", "{keys:?}");
}

#[test]
fn the_regex_lifecycle_violations_are_candidates() {
    if !clang_available() {
        return;
    }
    expect(
        "double regfree",
        "void w(const char *p) { regex_t re; int r; if ((r = k_regcomp(&re, p, 0)) != 0) return; k_regfree(&re); k_regfree(&re); }",
        "Unsafe",
        &["double_release"],
    );
    expect(
        "regexec after regfree",
        "void w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); if (r) return; k_regfree(&re); k_regexec(&re, p); }",
        "Unsafe",
        &["unknown_alias:re", "use_after_close:regex"],
    );
    expect(
        "never freed",
        "void w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); if (r) return; k_regexec(&re, p); }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn the_storage_is_custody_never_data() {
    if !clang_available() {
        return;
    }
    expect(
        "an unkinded borrowed position may overwrite the object",
        "void w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); if (r) return; k_wipe(&re, sizeof re); k_regfree(&re); }",
        "Unknown",
        &["unknown_alias:re", FORMAL_ADDRESS],
    );
    expect(
        "a struct copy",
        "void w(const char *p) { regex_t re; regex_t other; int r = k_regcomp(&re, p, 0); if (r) return; other = re; k_regfree(&re); }",
        "Unknown",
        &["unknown_alias:re", GUARD],
    );
    expect(
        "free of the object's address",
        "void w(const char *p) { regex_t re; int r = k_regcomp(&re, p, 0); if (r) return; k_free(&re); }",
        "Unknown",
        &[FORMAL_ADDRESS],
    );
}

#[test]
fn only_an_automatic_object_carries_the_obligation() {
    if !clang_available() {
        return;
    }
    for body in [
        "void w(const char *p) { static regex_t re; if (k_regcomp(&re, p, 0)) return; k_regfree(&re); }",
        "void w(const char *p) { if (k_regcomp(&g_re, p, 0)) return; k_regfree(&g_re); }",
        "void w(struct k_holder *h, const char *p) { if (k_regcomp(&h->re, p, 0)) return; k_regfree(&h->re); }",
        "void w(regex_t *re, const char *p) { if (k_regcomp(re, p, 0)) return; k_regfree(re); }",
        "void w(regex_t *re) { k_regfree(re); }",
    ] {
        let (status, keys) = outcome(body);
        assert_eq!(status, "Unknown", "{body}\n{keys:?}");
        assert!(
            keys.iter().any(|key| key.starts_with("unsupported:")),
            "{body}\n{keys:?}"
        );
    }
    // A use through a pointer this body does not track keeps the plain
    // borrowed reading.
    expect(
        "an untracked use",
        "int w(const regex_t *re, const char *s) { return k_regexec(re, s); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_kind_mismatch_on_the_in_place_kind_is_not_safe() {
    if !clang_available() {
        return;
    }
    expect(
        "regfree of a heap pointer",
        "void w(void) { void *p = k_alloc(8); if (!p) return; k_regfree(p); k_free(p); }",
        "Unknown",
        &[ACTUAL],
    );
}

/// The `mk_plugin_load_dynamic` shape: a possibly-NULL handle returned under
/// a contract-free exit refuses, because inferred summaries stay heap
/// (contract spec §C04.6, rule 7), whereas the same body over the heap
/// validates a nullable owned result. A guard that returns on NULL decides it.
#[test]
fn a_possibly_null_handle_returned_without_a_contract_refuses() {
    if !clang_available() {
        return;
    }
    expect(
        "heap control",
        "void *w(void) { void *p = k_alloc(8); if (!p) k_wipe(0, 0); return p; }",
        "Safe",
        &[],
    );
    for body in [
        "void *w(void) { void *h = k_dlopen(\"x\"); if (!h) k_wipe(0, 0); return h; }",
        "void *w(void) { void *h = k_dlopen(\"x\"); return h; }",
    ] {
        expect(
            "a nullable library handle returned",
            body,
            "Unknown",
            &[
                "unknown_alias:h",
                "unsupported:nullable resource `h` used by a non-null contract",
            ],
        );
    }
    expect(
        "returned only when non-null",
        "void *w(void) { void *h = k_dlopen(\"x\"); if (!h) return 0; return h; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_heap_only_body_is_unchanged() {
    if !clang_available() {
        return;
    }
    expect(
        "heap",
        "void w(void) { void *p = k_alloc(8); if (!p) return; k_free(p); }",
        "Safe",
        &[],
    );
}
