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

//! Card SCALAR-STATEMENT-FORMS (round 26 E): probe harness.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_fresh(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nonnull); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef unsigned long x_size; typedef int x_i32; typedef unsigned int x_u32; \
char *x_fresh(x_size n); void x_free(void *p); \
struct x_rec { int count; unsigned long total; struct x_rec *next; }; \
struct x_pair { int a; long b; }; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "scalar_statement_forms.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent scalar-statement-forms test: {reason}");
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
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}")
                .split([' ', '{', '('])
                .next()
                .unwrap_or("")
                .to_owned(),
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

const TYPEOF: &str = "unsupported:C operator `__typeof__` requires type/operand analysis";
const ASSIGNMENT: &str = "unsupported:unsupported assignment expression";
const BRACED: &str = "unsupported:braced initializer, expression, or declaration is not modeled";
const STEP: &str = "unsupported:unmodeled increment or decrement expression";
const INDIRECT: &str = "unsupported:indirect memory access is outside the resource projection";

// -- Admitted: steps and compound assignments ---------------------------------

#[test]
fn steps_and_compound_assignments_of_a_clang_scalar_are_admitted() {
    expect(
        "the lexical reader reads no declaration here (`__typeof__`): Clang's types decide",
        "int w(x_size len) { __typeof__(len) copy = len; len -= 8192*3; len--; len &= 7; \
         ++len; len <<= 1; return (int) (len + copy); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn a_for_clause_step_is_read_from_its_original_spelling() {
    expect(
        "`for` clauses reach no statement fallback; rule 41 reads the clause",
        "int w(x_size n) { __typeof__(n) i; x_size total = 0; \
         for (i = 0; i < n; i++) { total += i; } \
         for (; n >= 255; n -= 255) { } return (int) (total + n); }",
        "Unknown",
        &[TYPEOF],
    );
}

#[test]
fn a_compound_store_through_a_live_record_carries_its_obligation() {
    expect(
        "`r->count += …` through a guarded, live record",
        "void w(void) { struct x_rec *r; __typeof__(r) q = 0; r = x_record(); \
         r->count += 2 * 3; r->count++; x_record_free(r); (void) q; }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn a_compound_store_through_a_released_record_is_a_violation_candidate() {
    expect(
        "the base was released on the path reaching the store",
        "void w(void) { struct x_rec *r; __typeof__(r) q = 0; r = x_record(); \
         x_record_free(r); r->count += 2 * 3; (void) q; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:r", INDIRECT],
    );
}

// -- Admitted: declarations ---------------------------------------------------

#[test]
fn several_scalar_declarators_are_their_declarators_in_order() {
    expect(
        "`size_t of = 0, r;` (rd_kafka_version_str) and `size_t o=0, i=0;` (LZ4F_getFrameInfo)",
        "int w(void) { x_size of = 0, r; x_size o = 0, i = 0; r = of; return (int) (r + o + i); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_type_declaration_runs_nothing() {
    expect(
        "LZ4_stream_t_alignment's local typedef and a static assertion's enum",
        "int w(void) { typedef struct { char c; long t; } t_a; \
         { enum { x_sa = 1/(int)(!!(sizeof(long) >= sizeof(int))) }; }; \
         typedef enum { x_one, x_two } x_e; return (int)(sizeof(t_a) - sizeof(long)); }",
        "Safe",
        &[],
    );
}

#[test]
fn an_unevaluated_sizeof_operand_reads_nothing() {
    expect(
        "LZ4HC_rotatePattern: `size_t const b = (rotate & (sizeof(pattern) - 1)) << 3;`",
        "int w(unsigned long long pattern, x_u32 rotate) { \
         x_size const bits = (rotate & (sizeof(pattern) - 1)) << 3; \
         x_size const low = sizeof(pattern) - 1; return (int) (bits + low); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_braced_scalar_initializer_is_its_expression() {
    expect(
        "C11 6.7.9p11: `int x = { 1 };`",
        "int w(void) { int x = { 1 }; x_size y = { sizeof(x) }; return x + (int) y; }",
        "Safe",
        &[],
    );
}

// -- Refused, by name ---------------------------------------------------------

#[test]
fn a_volatile_object_keeps_its_refusal() {
    expect(
        "cmt_metric_dec's `double volatile val = 1.0;` (rule 11h)",
        "int w(void) { double volatile val = 1.0; return (int) val; }",
        "Unknown",
        &[ASSIGNMENT],
    );
}

#[test]
fn pointer_arithmetic_in_the_value_keeps_its_refusal() {
    expect(
        "LZ4HC_countBack's `int const min = (int)(iMin - ip);`",
        "int w(const char *a, const char *b) { int const d = (int)(b - a); return d; }",
        "Unknown",
        &[ASSIGNMENT],
    );
}

#[test]
fn an_object_of_an_inline_record_keeps_its_refusal() {
    expect(
        "pb_decode_fixed32's `union { uint32_t f; pb_byte_t b[4]; } u;` is rule 11f's",
        "int w(void) { union { int i; char c[4]; } u; u.i = 1; return u.i; }",
        "Unknown",
        &[BRACED],
    );
}

#[test]
fn a_comma_of_steps_in_a_for_clause_keeps_its_refusal() {
    expect(
        "chunk_header's `for (…; j--, c++)`: one clause, two writes",
        "int w(char *out) { int c = 0; int j; for (j = 5; j > 0; j--, c++) { out[c] = 'a'; } \
         return c; }",
        "Unknown",
        &[STEP],
    );
}

#[test]
fn a_shadowed_name_keeps_its_refusal() {
    expect(
        "`k` names a scalar and, in an inner block, a pointer: the projection names by spelling",
        "int w(void) { int k = 0; __typeof__(k) t = 0; k++; { char *k = 0; (void) k; } \
         return k + t; }",
        "Unknown",
        &[INDIRECT, STEP],
    );
}

#[test]
fn a_declaration_with_a_pointer_declarator_keeps_its_refusal() {
    expect(
        "`int a = 0, *p = 0;`: the pointer declarator is not this rule's",
        "int w(void) { int a = 0, *p = 0; return a + (p == 0); }",
        "Unknown",
        &[INDIRECT],
    );
}

#[test]
fn an_enumerator_spelling_an_object_keeps_its_refusal() {
    expect(
        "an enumerator `k` would be read as the object `k` by spelling",
        "int w(void) { int k = 1; { enum { k = 2 }; } return k; }",
        "Unknown",
        &[BRACED],
    );
}
