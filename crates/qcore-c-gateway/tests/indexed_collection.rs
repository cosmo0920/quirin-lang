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

//! Card W43-INDEXED-RESOURCE-COLLECTIONS — arrays whose elements are owned
//! resources, filled and released in index loops.
//!
//! A heap array this body acquired, declared `T **v`, is a collection: its
//! element obligations are ranges `[lo, hi)` over symbolic bounds. A fill loop
//! keeps `[0, i)` live, a release loop keeps `[0, j)` released, the partial
//! cleanup releases exactly the filled prefix, and anything unproved refuses.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status, and every `Safe` certificate is rechecked. The fixtures use a
//! generic `x_*` surface: `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_fresh(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nonnull); };\n\
extern fn x_dup(s: ptr<const, u8>) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_close(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_note(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_copy(p: ptr<mut, u8>) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_sopen(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable);\n\
           ensures normal resource_return_kind(stream); };\n\
extern fn x_sclose(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nonnull);\n\
           ensures normal resource_kind(p, stream);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_take(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_consumed(p, nullable);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
void *x_alloc(__SIZE_TYPE__ n); char *x_fresh(__SIZE_TYPE__ n); char *x_dup(const char *s); \
void x_free(void *p); void x_close(void *p); void x_note(void *p); void x_take(void *p); char *x_copy(void *p); \
void *x_sopen(__SIZE_TYPE__ n); void x_sclose(void *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "indexed_collection.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent indexed-collection test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
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

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
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
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
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
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    if !clang_available() {
        return;
    }
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

const INDIRECT: &str = "unsupported:indirect memory access is outside the resource projection";

/// The brief's shape: fill with a partial cleanup on failure, release loop.
const FILL: &str = "v = x_alloc(n * 8); if (!v) return 0; \
     for (i = 0; i < n; i++) { v[i] = x_dup(s); \
       if (!v[i]) { for (j = 0; j < i; j++) x_free(v[j]); x_free(v); return 0; } }";

// -- Decided: Safe -----------------------------------------------------------

#[test]
fn fill_with_partial_cleanup_then_release_certifies() {
    expect(
        "the brief's shape, every element released exactly once on every path",
        &format!(
            "int w(const char *s, unsigned long n) {{ char **v; unsigned long i; unsigned long j; \
             {FILL} for (i = 0; i < n; i++) x_free(v[i]); x_free(v); return 1; }}"
        ),
        "Safe",
        &[],
    );
}

#[test]
fn returning_the_filled_collection_transfers_every_element() {
    expect(
        "the whole collection escapes with its elements",
        &format!(
            "char **w(const char *s, unsigned long n) {{ char **v; unsigned long i; unsigned long j; \
             {FILL} return v; }}"
        ),
        "Safe",
        &[],
    );
}

#[test]
fn a_never_filled_array_owes_nothing() {
    expect(
        "allocate, test, release: no element was ever stored",
        "void w(void) { char **v = x_alloc(64); if (!v) return; x_free(v); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_null_initialized_array_owes_nothing() {
    expect(
        "elements stored as null owe nothing and are not released",
        "void w(void) { char **v = x_alloc(64); if (!v) return; v[0] = 0; v[1] = (void *) 0; \
         x_free(v); }",
        "Safe",
        &[],
    );
}

#[test]
fn an_append_counter_fills_a_prefix_that_a_counted_loop_releases() {
    expect(
        "`v[n++] = …` on two paths, then `for (i = 0; i < n; i++)`",
        "void w(const char *s, int k) { char **v; unsigned long n = 0; unsigned long i; \
         v = x_alloc(64); if (!v) return; if (k) { v[n++] = x_dup(s); } v[n++] = x_dup(s); \
         for (i = 0; i < n; i++) x_free(v[i]); x_free(v); }",
        "Safe",
        &[],
    );
}

#[test]
fn constant_indices_are_filled_and_released() {
    expect(
        "`v[0]`, `v[1]` stored from a non-null acquirer and released by a non-null release",
        "void w(void) { char **v = x_alloc(16); if (!v) return; v[0] = x_fresh(1); \
         v[1] = x_fresh(2); x_close(v[0]); x_close(v[1]); x_free(v); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_consumed_collection_takes_its_elements_along() {
    expect(
        "the array handed to a consuming callee",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_take(v); }",
        "Safe",
        &[],
    );
}

// -- Decided: violation candidates ------------------------------------------

#[test]
fn freeing_the_array_with_live_elements_is_an_element_leak() {
    expect(
        "the release loop is missing",
        "void w(const char *s, unsigned long n) { char **v; unsigned long i; \
         v = x_alloc(n * 8); if (!v) return; for (i = 0; i < n; i++) v[i] = x_dup(s); x_free(v); }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn a_partial_cleanup_that_frees_only_the_array_leaks_the_prefix() {
    expect(
        "the failure branch frees `v` but not `v[0..i)`",
        "int w(const char *s, unsigned long n) { char **v; unsigned long i; \
         v = x_alloc(n * 8); if (!v) return 0; \
         for (i = 0; i < n; i++) { v[i] = x_dup(s); if (!v[i]) { x_free(v); return 0; } } \
         for (i = 0; i < n; i++) x_free(v[i]); x_free(v); return 1; }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn releasing_an_element_twice_is_a_double_release() {
    expect(
        "the same element released twice",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_free(v[0]); x_free(v[0]); x_free(v); }",
        "Unsafe",
        &["double_release"],
    );
}

#[test]
fn a_second_release_loop_is_a_double_release() {
    expect(
        "two release loops over the same filled prefix",
        "void w(const char *s, unsigned long n) { char **v; unsigned long i; \
         v = x_alloc(n * 8); if (!v) return; for (i = 0; i < n; i++) v[i] = x_dup(s); \
         for (i = 0; i < n; i++) x_free(v[i]); for (i = 0; i < n; i++) x_free(v[i]); \
         x_free(v); }",
        "Unsafe",
        &["double_release"],
    );
}

#[test]
fn testing_a_released_element_is_a_use_after_release() {
    expect(
        "the element's value is indeterminate after its release",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_free(v[0]); if (v[0]) { x_note(v); } x_free(v); }",
        "Unsafe",
        &["use_after_release"],
    );
}

#[test]
fn an_element_operation_through_a_released_array_is_a_use_after_release() {
    expect(
        "the array is released before its element",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_free(v); x_free(v[0]); }",
        "Unsafe",
        &["leak", INDIRECT, "use_after_release"],
    );
}

#[test]
fn overwriting_a_live_element_is_a_leak() {
    expect(
        "a non-null element stored over",
        "void w(void) { char **v = x_alloc(16); if (!v) return; v[0] = x_fresh(1); \
         v[0] = x_fresh(2); x_close(v[0]); x_free(v); }",
        "Unsafe",
        &["leak"],
    );
}

// -- Refused: never assumed ---------------------------------------------------

#[test]
fn releasing_beyond_the_filled_prefix_refuses() {
    expect(
        "the cleanup releases `[0, n)` although only `[0, i)` was filled",
        "int w(const char *s, unsigned long n) { char **v; unsigned long i; unsigned long j; \
         v = x_alloc(n * 8); if (!v) return 0; \
         for (i = 0; i < n; i++) { v[i] = x_dup(s); \
           if (!v[i]) { for (j = 0; j < n; j++) x_free(v[j]); x_free(v); return 0; } } \
         for (i = 0; i < n; i++) x_free(v[i]); x_free(v); return 1; }",
        "Unknown",
        &["unknown_alias:v[j]"],
    );
}

#[test]
fn a_body_that_writes_the_bound_is_not_a_counted_loop() {
    expect(
        "`n--` in the release loop",
        "void w(const char *s, unsigned long n) { char **v; unsigned long i; \
         v = x_alloc(n * 8); if (!v) return; for (i = 0; i < n; i++) v[i] = x_dup(s); \
         for (i = 0; i < n; i++) { x_free(v[i]); n--; } x_free(v); }",
        "Unknown",
        &["unknown_alias:v[i]"],
    );
}

/// Re-pinned deliberately by card CG04-LOOP-SITE-GENERATIONS: the refused
/// element store leaves each iteration's element with an unresolved effect,
/// and rule 11a refuses merging a possibly-live generation with it (it would
/// hide the obligation), which also poisons the path the later statements
/// read. More refusal leaves, the same `Unknown`.
#[test]
fn a_break_keeps_the_exit_equation_from_holding() {
    expect(
        "a `break` leaves the fill loop before `i == n`",
        "void w(const char *s, unsigned long n, int k) { char **v; unsigned long i; \
         v = x_alloc(n * 8); if (!v) return; \
         for (i = 0; i < n; i++) { v[i] = x_dup(s); if (k) break; } \
         for (i = 0; i < n; i++) x_free(v[i]); x_free(v); }",
        "Unknown",
        &[
            "unknown_alias:v[i]",
            INDIRECT,
            "unsupported:loop re-acquisition: an earlier generation that may be live is still held",
            "unsupported:resource call with a formal embedded in an unmodeled expression",
        ],
    );
}

/// Re-pinned deliberately by card CG04-LOOP-SITE-GENERATIONS: the refused
/// element store leaves each iteration's element with an unresolved effect,
/// and rule 11a refuses merging a possibly-live generation with it (it would
/// hide the obligation), which also poisons the path the later statements
/// read. More refusal leaves, the same `Unknown`.
#[test]
fn a_narrower_unsigned_index_is_not_a_counted_loop() {
    expect(
        "`unsigned int i` below an `unsigned long` bound may wrap",
        "void w(const char *s, unsigned long n) { char **v; unsigned int i; \
         v = x_alloc(n * 8); if (!v) return; for (i = 0; i < n; i++) v[i] = x_dup(s); \
         x_free(v); }",
        "Unknown",
        &[
            "unknown_alias:v[i]",
            "unsupported:loop re-acquisition: an earlier generation that may be live is still held",
            "unsupported:resource call with a formal embedded in an unmodeled expression",
        ],
    );
}

#[test]
fn handing_a_filled_collection_to_a_borrowing_callee_refuses() {
    expect(
        "a borrow could move or free elements the ranges do not see",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_note(v); x_free(v[0]); x_free(v); }",
        "Unknown",
        &["unknown_alias:v", "unknown_alias:v[0]"],
    );
}

#[test]
fn an_element_store_whose_call_borrows_the_collection_refuses() {
    expect(
        "the acquiring call is handed the array that already owes `v[0]`",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         v[1] = x_copy(v); x_free(v[0]); x_free(v[1]); x_free(v); }",
        "Unknown",
        &[
            "unknown_alias:v",
            "unknown_alias:v[0]",
            "unknown_alias:v[1]",
        ],
    );
}

#[test]
fn a_possibly_null_element_refuses_a_non_null_release() {
    expect(
        "`x_close` requires a non-null element; `x_dup` may return null",
        "void w(const char *s) { char **v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_close(v[0]); x_free(v); }",
        "Unknown",
        &["unknown_alias:v[0]"],
    );
}

#[test]
fn an_element_stored_on_one_path_only_refuses_its_release() {
    expect(
        "the other path never filled `v[0]`",
        "void w(const char *s, int k) { char **v = x_alloc(16); if (!v) return; \
         if (k) v[0] = x_dup(s); x_free(v[0]); x_free(v); }",
        "Unknown",
        &["unknown_alias:v[0]"],
    );
}

#[test]
fn an_unguarded_array_keeps_the_refusal_it_had() {
    expect(
        "`v` may be null, so the store keeps its original refusal byte for byte",
        "void w(const char *s) { char **v = x_alloc(16); v[0] = x_dup(s); x_free(v); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn a_callers_array_keeps_the_refusal_it_had() {
    expect(
        "a formal's elements are the caller's: the release keeps its refusal",
        "void w(char **v, unsigned long n) { unsigned long i; \
         for (i = 0; i < n; i++) x_free(v[i]); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn an_index_expression_keeps_the_refusal_it_had() {
    expect(
        "`v[i + 1]` is not an admitted index",
        "void w(const char *s) { unsigned long i = 2; char **v = x_alloc(16); if (!v) return; \
         v[i + 1] = x_dup(s); x_free(v); }",
        "Unknown",
        &["unsupported:resource call with a formal embedded in an unmodeled expression"],
    );
}

#[test]
fn storing_a_named_pointer_keeps_the_refusal_it_had() {
    // The store keeps its refusal, so the collection never holds `p`'s
    // object: releasing `v[0]` releases an unfilled element and refuses. The
    // leak of `p`'s site this card measured was the pre-card answer for a
    // release through a refused statement. Card W44-LEAK-PIN-AUDIT withdraws
    // it deliberately: the refused store hands `p` on as its value, so `p`
    // carries an unresolved effect (the refused hand-on rule) and the function
    // is `Unknown`, never `Safe`. On this one-line body the rule's line key also
    // reads `x_free(v)`, so `v` is withdrawn too and the element release no
    // longer reaches the `v[0]` alias refusal.
    expect(
        "`v[0] = p` would leave `p` an alias of a collection element",
        "void w(const char *s) { char **v = x_alloc(16); char *p; if (!v) return; \
         p = x_dup(s); v[0] = p; x_free(v[0]); x_free(v); }",
        "Unknown",
        &[INDIRECT],
    );
}

/// Re-pinned deliberately by card CG04-LOOP-SITE-GENERATIONS. Each
/// iteration's array is a new object. Before the card the re-acquisition
/// wrote the one site over the old array, whose element ranges it still
/// carried, so the store into the fresh array was refused (`unknown_alias`
/// on `v[0]`). Now the old array is the previous generation with its own
/// ranges and the store into the fresh one is admitted; the older
/// generations' ranges are merged into one summary, which establishes no
/// ranges, so the exit refuses instead. Still `Unknown` (the source leaks
/// every element; nothing claims otherwise).
#[test]
fn an_array_reallocated_in_a_loop_refuses() {
    expect(
        "each iteration's array is a new generation; the older ones' ranges are not joined",
        "void w(const char *s, unsigned long n) { char **v; unsigned long i; \
         for (i = 0; i < n; i++) { v = x_alloc(16); if (!v) return; v[0] = x_dup(s); \
         x_free(v); } }",
        "Unknown",
        &["unsupported:indexed collection element ranges are not established at this exit"],
    );
}

#[test]
fn paths_that_disagree_on_the_ranges_refuse_at_the_exit() {
    expect(
        "one path fills `v[0]`, the other does not; neither form covers both",
        "void w(const char *s, int k) { char **v = x_alloc(16); if (!v) return; \
         if (k) v[0] = x_dup(s); x_free(v); }",
        "Unknown",
        &["unsupported:indexed collection element ranges are not established at this exit"],
    );
}

#[test]
fn a_helper_returning_a_filled_collection_publishes_no_summary() {
    expect(
        "the helper is Safe, but a one-identity summary would hide its elements from `w`",
        "static char **mk(const char *s) { char **v = x_alloc(16); if (!v) return 0; \
         v[0] = x_fresh(1); return v; } \
         void w(const char *s) { char **v = mk(s); if (!v) return; x_free(v); }",
        "Unknown",
        &[
            "unknown_alias:v",
            "unsupported:call to uncontracted function `mk`",
        ],
    );
}

// -- Kinds (integration of W43-DESCRIPTOR-RESOURCE-TRACKING) ------------------

#[test]
fn a_stream_element_keeps_the_refusal_it_had() {
    expect(
        "a collection holds heap elements only: a stream-kind acquisition is not stored",
        "void w(void) { char **v = x_alloc(16); if (!v) return; v[0] = x_sopen(1); \
         x_sclose(v[0]); x_free(v); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn a_stream_release_of_a_heap_element_keeps_the_refusal_it_had() {
    // The kind-mismatched release keeps its refusal, so the element is still
    // owed when the array is freed: it was an element leak candidate, exactly
    // as a refused release of a named heap pointer leaves its site owed.
    //
    // Card W44-LEAK-PIN-AUDIT: the one-line body's refused store names `v`'s
    // line, and on that line `x_free(v)` hands `v` to a call, so the refused
    // hand-on rule (keyed by line) withdraws the array and its elements. The
    // release still never discharges the element: the verdict is `Unknown`,
    // never `Safe`.
    expect(
        "`x_sclose` releases a stream, so it never discharges a heap element",
        "void w(void) { char **v = x_alloc(16); if (!v) return; v[0] = x_fresh(1); \
         x_sclose(v[0]); x_free(v); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn a_scalar_array_is_not_a_collection() {
    expect(
        "`char *buf` elements are scalars; the typed scalar route keeps them",
        "void w(void) { char *buf = x_alloc(16); if (!buf) return; buf[0] = 0; x_free(buf); }",
        "Safe",
        &[],
    );
}
