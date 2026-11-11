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

//! Card W44-COLLECTION-ALLOCATION — an unevaluated `sizeof` dereference
//! operand is not an access.
//!
//! `v = x_alloc(n * sizeof(*v))` was refused as `indirect memory access` for
//! its `sizeof(*v)` operand. C11 6.5.3.4p2 leaves an operand whose type is not
//! a variable length array type unevaluated, so the operand reads nothing and
//! dereferences nothing; the allocation's custody is the call's contract
//! alone. A variably modified operand, an array declarator, an undeclared or
//! unresolved name, and a postfix after the operand all keep the refusal.
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
extern fn x_calloc(c: usize, n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_zero(p: ptr<mut, u8>, n: usize) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
void *x_alloc(__SIZE_TYPE__ n); void *x_calloc(__SIZE_TYPE__ c, __SIZE_TYPE__ n); \
void x_free(void *p); void x_zero(void *p, __SIZE_TYPE__ n); \
struct item { int a; long b; }; typedef struct item item_t; typedef unsigned int u32;";

const ACCESS: &str = "unsupported:indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "sizeof_operand.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent sizeof-operand test: {reason}");
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
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(body);
    assert_eq!(
        (
            actual_status,
            actual.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, diagnostics.to_vec()),
        "{body}"
    );
}

#[test]
fn a_parenthesised_dereference_operand_is_unevaluated() {
    expect(
        "int w(unsigned long n) { unsigned long k = n; struct item *p = x_alloc(sizeof(*p) * k); \
         if (!p) return -1; x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn an_unparenthesised_dereference_operand_is_unevaluated() {
    expect(
        "int w(void) { struct item *p; p = x_calloc(1, sizeof *p); \
         if (!p) return -1; x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_literal_subscript_operand_is_unevaluated() {
    expect(
        "int w(unsigned long n) { unsigned long k = n; item_t *p = x_alloc(sizeof(p[0]) * k); \
         if (!p) return -1; x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_typedef_resolving_to_a_scalar_is_admitted() {
    expect(
        "int w(unsigned long n) { u32 *p = x_calloc(n, sizeof(*p)); \
         if (!p) return -1; x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_pointer_to_pointer_array_allocation_binds_the_collection() {
    // The collection's own allocation, which W43-INDEXED-RESOURCE-COLLECTIONS
    // could never reach: after it, every element operation is decided by the
    // collection rules rather than its fallback.
    expect(
        "int w(int count) { int i; int n = count; struct item **v = x_alloc(sizeof(*v) * n); \
         if (!v) return -1; \
         for (i = 0; i < n; i++) v[i] = x_alloc(sizeof(struct item)); \
         for (i = 0; i < n; i++) x_free(v[i]); \
         x_free(v); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_released_pointer_in_the_operand_is_not_a_use() {
    // The operand names `p` after its release; nothing is read.
    expect(
        "int w(void) { struct item *p = x_alloc(sizeof(*p)); struct item *q; \
         x_free(p); q = x_alloc(sizeof(*p)); if (!q) return -1; x_free(q); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_newly_modelled_allocation_can_be_a_leak() {
    // The accusing direction: the admission exposes an unreleased allocation.
    expect(
        "int w(void) { struct item *p = x_alloc(sizeof(*p)); \
         if (!p) return -1; return 0; }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn a_borrowed_data_argument_operand_is_unevaluated() {
    expect(
        "void w(struct item *e) { x_zero(e, sizeof(*e)); }",
        "Safe",
        &[],
    );
}

/// Flipped by card CG08-CASTS-AND-TYPE-QUERIES (rule 11b; W44 residual (c)).
/// `( n * sizeof` still begins like a declaration `n *…` to the lexical
/// declarator reader, but Clang's node shows the operand unevaluated, so the
/// gate reads the marker.
#[test]
fn a_leading_multiplication_the_declaration_reader_refuses_is_read_from_clang() {
    expect(
        "int w(unsigned long n) { struct item *p = x_alloc(n * sizeof(*p)); \
         if (!p) return -1; x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_pointer_to_a_variable_length_array_keeps_the_refusal() {
    // `int (*p)[n]` is variably modified: the declarator reader does not read
    // it, so the whole function keeps every refusal it had.
    expect(
        "int w(int n) { int m = n; int (*p)[m] = x_alloc(sizeof(*p)); x_free(p); return 0; }",
        "Unknown",
        &["unknown_alias:n", "unknown_alias:p", ACCESS],
    );
}

#[test]
fn a_block_scope_variable_length_typedef_keeps_the_refusal() {
    expect(
        "int w(int n) { int m = n; typedef int row[m]; row *p = x_alloc(sizeof(*p)); \
         x_free(p); return 0; }",
        "Unknown",
        &["unknown_alias:p", ACCESS],
    );
}

// Flipped by card CG08-CASTS-AND-TYPE-QUERIES (rule 11b), the residuals of
// `W44-SIZEOF-OPERAND-RESIDUALS`: Clang's node decides the operand's
// evaluation class where the lexical reader could not. An array declarator,
// a file-scope name and a member operand are unevaluated in an ordinary call
// statement (the gate reads the marker); a statement with no ordinary call is
// the elaborator's (the operand has no event). The variably modified operands
// above keep their refusal.

#[test]
fn an_array_declarator_is_read_from_clang() {
    expect(
        "int w(void) { struct item *a[4]; struct item *p = x_alloc(sizeof(*a)); \
         x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_file_scope_name_is_read_from_clang() {
    expect(
        "struct item *g; int w(void) { struct item *p = x_alloc(sizeof(*g)); \
         x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_member_operand_is_read_from_clang() {
    expect(
        "int w(struct item **v) { long *p = x_alloc(sizeof((*v)->b)); x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_statement_without_a_call_is_the_elaborators() {
    expect(
        "unsigned long w(void) { struct item *p = 0; unsigned long n; n = sizeof(*p); return n; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_statement_whose_first_call_is_a_type_operator_is_the_elaborators() {
    expect(
        "int w(void) { struct item *p; unsigned long n; \
         n = sizeof(*p) + sizeof(struct item) * 2; p = x_alloc(n); x_free(p); return 0; }",
        "Safe",
        &[],
    );
}

/// Card CG08-CASTS-AND-TYPE-QUERIES: W44 residual (d), a `sizeof` operand
/// followed by `* n` at a borrowed actual beside a formal pointer.
#[test]
fn a_multiplied_operand_beside_a_formal_is_read_from_clang() {
    expect(
        "int w(struct item *q, unsigned long n) { x_zero(q, sizeof(*q) * n); return 0; }",
        "Safe",
        &[],
    );
}
