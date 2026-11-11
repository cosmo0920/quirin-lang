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

//! Card W43-INDEX-TRAVERSAL — index and pointer traversal.
//!
//! Code that walks a buffer or a string does so by index (`s[i]`, `i++`) or by
//! pointer increment (`p++`, `p += k`, `*p++ = c`). This card makes such walks
//! decidable for the resource property — custody, use after release and
//! invalid release — and for nothing else: **no bounds claim**. A stepped
//! pointer is derived from its base with an interior extent, so it never
//! discharges the base's custody; releasing it is refused or accused; using it
//! after the base's release is a candidate; and a loop reaches its fixed point
//! over those facts through the existing work list.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status. The fixtures use the generic `x_*` surface of the typed
//! scalar-access tests: `Safe` here is `Safe` over synthetic trusted test
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
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_chr(s: ptr<const, u8>, c: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_interior(s, nullable); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_note(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
char *x_alloc(__SIZE_TYPE__ n); char *x_fresh(__SIZE_TYPE__ n); void x_free(void *p); \
__SIZE_TYPE__ x_len(const char *s); char *x_chr(const char *s, int c); \
struct x_rec { int count; unsigned long total; struct x_rec *next; }; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r); \
void x_note(void *p); \
enum x_kind { X_ONE = 1, X_TWO = 2 };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "index_traversal.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent indirect-scalar-access test: {reason}");
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

const GATE: &str = "unsupported:unmodeled expression may mutate a formal resource identity";

// -- Walks over objects this projection does not track --------------------------

#[test]
fn a_string_walk_over_a_copy_of_a_formal_certifies() {
    expect(
        "`p = s; while (*p) p++;`: the copy of an unbound pointer is unbound",
        "void w(const char *s) { const char *p; p = s; while (*p) p++; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_for_walk_with_a_typed_condition_certifies() {
    expect(
        "`for (p = s; *p != '\\0'; p++)`",
        "void w(const char *s) { const char *p; for (p = s; *p != '\\0'; p++) { } }",
        "Safe",
        &[],
    );
}

#[test]
fn an_index_scan_that_returns_its_index_certifies() {
    expect(
        "`while (i < len && s[i] != ':') i++; return i;`",
        "unsigned long w(const char *s, unsigned long len) { unsigned long i = 0; \
         while (i < len && s[i] != ':') i++; return i; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_counting_loop_with_an_indexed_branch_certifies() {
    expect(
        "a counter joined over the loop has no fact, and returning it escapes nothing",
        "unsigned long w(const char *s, unsigned long n) { unsigned long i; \
         unsigned long c = 0; for (i = 0; i < n; i++) { if (s[i] == 'x') c++; } return c; }",
        "Safe",
        &[],
    );
}

#[test]
fn scalar_copies_and_counters_certify() {
    for body in [
        "unsigned long w(unsigned long n) { unsigned long m; m = n; return m; }",
        "unsigned long w(unsigned long n) { unsigned long i = 0; while (i < n) i++; return i; }",
        "int w(unsigned long n) { unsigned long i; int c = 0; \
         for (i = 0; i < n; i++) { c++; } if (c) { return 1; } return 0; }",
    ] {
        expect("index facts at a loop's join", body, "Safe", &[]);
    }
}

#[test]
fn every_step_form_over_unbound_pointers_certifies() {
    expect(
        "`s += k`, `p = s + k`, `p -= 1`, `++p`, `--p`, a record pointer's `q++`",
        "void w(char *s, unsigned long k, struct x_rec *r) { char *p; struct x_rec *q; \
         s += k; p = s + k; p -= 1; ++p; --p; char *e = s + 2; \
         for (q = r; q->count; q++) { } }",
        "Safe",
        &[],
    );
}

#[test]
fn a_copy_loop_with_stepped_accesses_certifies() {
    expect(
        "`while (*i) *o++ = *i++; *o = 0;`",
        "void w(char *out, const char *s) { char *o; const char *i; o = out; i = s; \
         while (*i) *o++ = *i++; *o = 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn every_condition_position_reads_typed_accesses() {
    expect(
        "`switch (s[0])`, `do … while (*p)`, pointer comparisons and the null constant",
        "int w(const char *s, const char *end) { const char *p; p = s; \
         while (p < end && *p != ':') p++; while (p != 0 && *p) p++; \
         do { p++; } while (*p); if (s[1] == 'b') { return 2; } \
         switch (s[0]) { case 'a': return 1; default: return 0; } }",
        "Safe",
        &[],
    );
}

// -- Walks over a tracked allocation ---------------------------------------------

#[test]
fn walking_a_copy_of_a_live_allocation_and_releasing_the_base_certifies() {
    for body in [
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p; q++; x_free(p); }",
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p + 1; x_free(p); }",
        "void w(void) { char *p; char *q; p = x_fresh(4); for (q = p; *q; q++) { } x_free(p); }",
    ] {
        expect(
            "the base keeps its identity; the walker owns nothing",
            body,
            "Safe",
            &[],
        );
    }
}

#[test]
fn an_unguarded_nullable_base_refuses_the_walk() {
    expect(
        "the walker inherits the base's possible nullness",
        "void w(void) { char *p; char *q; p = x_alloc(4); \
         for (q = p; *q; q++) { } x_free(p); }",
        "Unknown",
        &["unknown_alias:q"],
    );
}

// -- The base must not be lost ---------------------------------------------------

#[test]
fn stepping_the_only_name_of_an_allocation_refuses() {
    expect(
        "`p = malloc(n); p++; free(p - 1);` is refused, never credited or accused",
        "void w(void) { char *p; p = x_fresh(4); p++; x_free(p - 1); }",
        "Unknown",
        &[
            "unknown_alias:p",
            "unsupported:arguments do not match resource contract for `x_free`",
        ],
    );
    expect(
        "no leak is claimed for an object named only by arithmetic",
        "void w(void) { char *p; p = x_fresh(4); p++; }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

// -- Violation candidates --------------------------------------------------------

#[test]
fn releasing_a_forward_stepped_pointer_is_an_invalid_release_candidate() {
    expect(
        "`q = p + 1; free(q);` frees an interior pointer and leaks `p`",
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p + 1; x_free(q); }",
        "Unsafe",
        &["invalid_release", "leak"],
    );
}

#[test]
fn a_stepped_pointer_that_may_be_at_its_base_is_refused_not_accused() {
    expect(
        "`q--` may return to the start",
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p + 2; q--; x_free(q); }",
        "Unknown",
        &["unknown_alias:q"],
    );
    expect(
        "a loop's header joins the base identity with the stepped pointer",
        "void w(void) { char *p; char *q; p = x_fresh(4); for (q = p; *q; q++) { } x_free(q); }",
        "Unknown",
        &["unknown_alias:q"],
    );
}

#[test]
fn using_a_stepped_pointer_after_its_base_is_released_is_a_candidate() {
    expect(
        "a store through an interior pointer after the base's release",
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p + 1; x_free(p); *q = 0; }",
        "Unsafe",
        &["unknown_alias:q", "use_after_release"],
    );
    expect(
        "a condition read after a walk and the base's release",
        "void w(void) { char *p; char *q; p = x_fresh(4); q = p; while (*q) q++; \
         x_free(p); if (*q) { } }",
        "Unsafe",
        &["unknown_alias:q", "use_after_release"],
    );
}

#[test]
fn a_stepped_and_a_declared_derivation_join_to_mixed() {
    expect(
        "`strchr` result on one path, its step on another: the families stay apart",
        "void w(int c) { char *p; char *q; p = x_fresh(4); q = x_chr(p, 58); \
         if (q == 0) { x_free(p); return; } if (c) q++; *q = 0; x_free(p); }",
        "Unknown",
        &["unknown_alias:q"],
    );
}

// -- Refusals kept ---------------------------------------------------------------

#[test]
fn stepping_a_read_only_scalar_pointer_formal_certifies() {
    expect(
        "W48 batch 2: a formal's slot is local and scalar reads expose no identity",
        "void w(const char *s) { while (*s) s++; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_condition_with_a_call_or_a_write_keeps_its_refusal() {
    expect(
        "an uncontracted call in the condition",
        "int u(const char *s); void w(const char *s) { const char *p; p = s; \
         while (u(p) && *p) p++; }",
        "Unknown",
        // The copy of the formal keeps its unresolved alias (the signal W4
        // binds a formal on), and the refused condition keeps W4 from
        // producing a summary that would seed it.
        &[
            "unknown_alias:s",
            "unsupported:unmodeled call, aggregate or indirect access in loop condition",
        ],
    );
    expect(
        "`while (*p++)` writes in the condition",
        "void w(const char *s) { const char *p; p = s; while (*p++) { } }",
        "Unknown",
        &[
            "unknown_alias:s",
            "unsupported:unmodeled call, aggregate or indirect access in loop condition",
        ],
    );
}

#[test]
fn an_index_that_writes_is_sequenced_after_its_access() {
    expect(
        "the W43-INDIRECT-ACCESS-GENERAL hole: `out[m++]` used to keep `m == 0`, \
         prune the `return`, and certify a leaking body",
        "void w(char *out) { char *p; int m = 0; p = x_fresh(4); out[m++] = 'a'; \
         if (m) { return; } x_free(p); }",
        "Unsafe",
        &["leak"],
    );
    expect(
        "an index write the stepped-access shape does not cover stays refused",
        "void w(char *out) { int m = 0; out[m += 1] = 'a'; }",
        "Unknown",
        &["unsupported:indirect memory access is outside the resource projection"],
    );
}

#[test]
fn a_stepped_pointer_is_never_returned_or_published() {
    expect(
        "returning an interior pointer into a tracked object refuses",
        "char *w(void) { char *p; char *q; p = x_fresh(4); q = p + 1; return q; }",
        // The refused escape leaves the object's fate unresolved: no leak is
        // claimed for it either.
        "Unknown",
        &["unknown_alias:q"],
    );
}

#[test]
fn formal_steps_keep_calls_copies_addresses_and_pointee_writes_refused() {
    for body in [
        "void w(char *s) { while (*s) { x_note(s); s++; } }",
        "void w(char *s) { char *q = s; while (*s) s++; x_note(q); }",
        "void w(char *s) { while (*s) { *s = 0; s++; } }",
        "void w(char *s) { while (*s) { x_note(&*s); s++; } }",
        "void w(char *s) { while (*s) { char **q = &s; s++; } }",
    ] {
        let (status, diagnostics) = outcome(body);
        assert_eq!(status, "Unknown", "{body}: {diagnostics:?}");
        assert!(
            diagnostics.iter().any(|d| d == GATE),
            "{body}: {diagnostics:?}"
        );
    }
}

#[test]
fn a_matrix_reader_body_and_its_caller_validate_bottom_up() {
    expect(
        "gf2_matrix_times shape: read scalar values, step the by-value formal",
        "static unsigned helper(unsigned *mat, unsigned vec) { unsigned sum = 0; \
         while (vec) { if (vec & 1) sum ^= *mat; vec >>= 1; mat++; } return sum; } \
         unsigned w(unsigned *mat, unsigned vec) { return helper(mat, vec); }",
        "Safe",
        &[],
    );
}

#[test]
fn an_own_custody_or_out_slot_declaration_keeps_the_formal_gate() {
    for (mode, parameter_type, c_type) in [
        ("resource_owned(s, nonnull)", "ptr<mut, u8>", "char *"),
        (
            "resource_out(s, nonnull)",
            "ptr<mut, ptr<mut, u8>>",
            "char **",
        ),
    ] {
        let specification = format!(
            "{CONTRACTS}\nextern fn w(s: {parameter_type}) -> unit\ncontract {{ \
            requires resource_contract_v1; ensures normal {mode}; \
            ensures normal resource_return_none(); }};"
        );
        let env = load_resource_contract_sources(&BTreeMap::from([(
            "formal.qv".to_owned(),
            specification,
        )]))
        .unwrap();
        let source = format!("void w({c_type}s) {{ while (*s) s++; }}");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "w",
            &env,
        )
        .unwrap();
        let row = &program(&result).functions["w"];
        assert_ne!(
            row.status,
            qcore_c_gateway::legacy_c::AnalysisStatus::Safe,
            "{mode}: {row:?}"
        );
        assert!(row.diagnostics.iter().any(|diagnostic| matches!(&diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description } if description == "unmodeled expression may mutate a formal resource identity")), "{mode}: {row:?}");
    }
}
