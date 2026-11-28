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

//! Card K2-NO-TYPED-ACCESS (gateway rule 79), real Clang over the generic
//! `d_*`/`k_*` fixture contracts: (1) a connective condition with a `sizeof`
//! type query, (2) `NAME * sizeof` read as no declaration; admitted shapes and
//! the adversarial near-misses that keep their refusal. Each body's base
//! answer (5087f9295) is recorded in docs/reports/k2-no-typed-access/probes/.
//! Section (3) is card K2-BUILTIN-BIT-COUNTS (gateway rule 94): a bit-count
//! builtin under a guard that proves its operand nonzero.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
__SIZE_TYPE__ d_len(const void *s); \
int ext(int); \
struct box { long n; char *name; struct box *next; }; \
typedef struct box box_t; \
struct flex { long n; struct box *e[]; }; \
typedef struct flex flex_t;";

const BRANCH: &str = "unsupported:unmodeled call, aggregate or indirect access in branch condition";
const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "descriptor_generic.qv".to_owned(),
            include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
        ),
        (
            "derived_result_generic.qv".to_owned(),
            include_str!("fixtures/contracts/derived_result_generic.qv").to_owned(),
        ),
    ]))
    .expect("the fixtures load")
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
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}")
            .split([' ', '{'])
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    let status = match &result {
        UnrestrictedLegacyResult::Safe(_) => "Safe",
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result).functions.get("w").expect("w analysed");
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn safe(body: &str) {
    assert_eq!(outcome(body), ("Safe", vec![]), "{body}");
}

fn refused(body: &str, leaf: &str) {
    let (status, keys) = outcome(body);
    assert_eq!(status, "Unknown", "{body}: {keys:?}");
    assert!(keys.iter().any(|k| k == leaf), "{body}: {keys:?}");
}

// ---- (1) a connective condition with a type query ----

#[test]
fn a_connective_with_a_type_query_is_admitted() {
    // LZ4_hashPosition's own spelling, then its shape; each is Unknown at
    // base (probes7.txt, probes5.txt, probes6.txt).
    safe(
        "typedef unsigned long reg_t; typedef enum { clearedTable = 0, byPtr, byU32, byU16 } tableType_t; unsigned w(const void* const p, tableType_t const tableType) { if ((sizeof(reg_t)==8) && (tableType != byU16)) return 1; return 0; }",
    );
    safe(
        "typedef unsigned long reg_t; int w(int t) { if ((sizeof(reg_t) == 8) && (t != 2)) return 1; return 0; }",
    );
    safe(
        "typedef enum { clearedTable = 0, byPtr, byU32, byU16 } tableType_t; int w(tableType_t const t) { if ((sizeof(long) == 8) && (t != byU16)) return 1; return 0; }",
    );
    safe("int w(int t) { if ((sizeof(long) == 8) && (t != 2)) return 1; return 0; }");
    safe("int w(int t) { if ((t != 2) && (sizeof(long) == 8)) return 1; return 0; }");
    safe("int w(int t) { while ((sizeof(long) == 8) && (t > 2)) t = t - 1; return t; }");
    safe("int w(int t) { for (; (sizeof(long) == 8) && (t > 2); t--) { } return t; }");
    safe("int w(int t) { do { t = t - 1; } while ((sizeof(long) == 8) && (t > 2)); return t; }");
}

// The near-misses below are refused by every older route at base, so rule 79
// is asked; each keeps the position's refusal.

#[test]
fn a_call_in_the_region_keeps_the_refusal() {
    refused(
        "int w(int t) { if ((sizeof(long) == 8) && ext(t)) return 1; return 0; }",
        "unsupported:call to uncontracted function `ext` in branch condition",
    );
}

#[test]
fn a_store_in_the_connective_keeps_the_refusal() {
    refused(
        "int g; int w(int t) { if ((sizeof(long) == 8) && (g = t)) return 1; return 0; }",
        BRANCH,
    );
}

#[test]
fn an_address_taken_or_static_read_keeps_the_refusal() {
    refused(
        "int w(int t) { int u = t; d_len(&u); if ((sizeof(long) == 8) && (u != 2)) return 1; return 0; }",
        BRANCH,
    );
    refused(
        "int g; int w(void) { if ((sizeof(long) == 8) && (g != 2)) return 1; return 0; }",
        BRANCH,
    );
}

#[test]
fn a_volatile_read_keeps_the_refusal() {
    refused(
        "int w(volatile int t) { if ((sizeof(long) == 8) && (t != 2)) return 1; return 0; }",
        BRANCH,
    );
}

#[test]
fn a_path_read_in_an_arm_keeps_its_obligation_and_the_refusal() {
    // `(b + 1)->n` is read only when the type query holds; admitting it as no
    // event would drop its dereference obligation.
    refused(
        "long w(struct box *b) { if ((sizeof(long) == 8) && ((b + 1)->n != 2)) return 1; return 0; }",
        BRANCH,
    );
}

#[test]
fn an_expression_operand_keeps_the_refusal() {
    // The rule reads type-name operands only.
    refused(
        "int w(int t) { if ((sizeof(t) == 4) && (t != 2)) return 1; return 0; }",
        BRANCH,
    );
}

#[test]
fn what_an_older_route_decides_is_unchanged() {
    // Safe at base through rule 5h's desugaring (an assignment, an increment,
    // a member read, a contracted call) or another older route; rule 79 is
    // not asked (probes4.txt, probes5.txt).
    safe("int w(int t) { int u; if ((sizeof(long) == 8) && (u = t)) return 1; return 0; }");
    safe("int w(int t) { if ((sizeof(long) == 8) && (t++ != 2)) return 1; return 0; }");
    safe("long w(struct box *b) { if ((sizeof(long) == 8) && (b->n != 2)) return 1; return 0; }");
    safe("int w(const char *s) { if ((sizeof(long) == 8) && d_len(s)) return 1; return 0; }");
    safe("int w(int n) { if ((sizeof(int[n]) == 8) && (n != 2)) return 1; return 0; }");
    safe("int w(int t) { if ((sizeof(long) == 8) || (t != 2)) return 1; return 0; }");
    safe(
        "typedef enum { by16, by32 } tt; int w(tt const t) { if ((sizeof(void *) == 8) && (t != by16)) return 1; return 0; }",
    );
}

// ---- (2) `NAME * sizeof` is no declaration ----

#[test]
fn a_parenthesised_product_beside_sizeof_deref_is_admitted() {
    // rd_kafka_share_partition_offsets_list_new's statement; the first two
    // are Unknown at base (probes2.txt, probes3.txt).
    safe("long w(box_t *b, long n) { long s; s = sizeof(*b) + (n * sizeof(box_t *)); return s; }");
    safe(
        "long w(struct box *b, long n) { long s; s = sizeof(*b) + (n * sizeof(char *)); return s; }",
    );
    safe("long w(long n) { long s; s = (n * sizeof(long)); return s; }");
}

#[test]
fn the_unparenthesised_forms_were_and_stay_admitted() {
    safe(
        "long w(struct box *b, long n) { long s; s = sizeof(*b) + n * sizeof(char *); return s; }",
    );
    safe("long w(box_t *b, long n) { long s; s = sizeof(*b) + (n * 4); return s; }");
}

#[test]
fn the_flexible_allocation_is_admitted() {
    // Base: Unknown with the gate leaf and, the declared types unread, a
    // possibly-null return under a non-null result mode (probes2.txt).
    let body = "flex_t *w(long n) { flex_t *b; long s; s = sizeof(*b) + (n * sizeof(struct box *)); b = k_alloc(s); return b; }";
    let (status, keys) = outcome(body);
    assert!(!keys.iter().any(|k| k == GATE), "{keys:?}");
    assert_eq!(status, "Safe", "{keys:?}");
}

#[test]
fn a_variably_modified_operand_keeps_the_refusal() {
    let (status, _) = outcome(
        "long w(long n) { long s; int (*v)[n]; s = sizeof(*v) + (n * sizeof(int)); return s; }",
    );
    assert_eq!(status, "Unknown");
}

#[test]
fn a_real_declaration_after_a_parenthesis_is_still_read() {
    // `( T * p )` stays a declaration candidate; the function's types are read.
    safe(
        "long w(long n) { long s; for (box_t *p = 0; p; ) { break; } s = (n * sizeof(long)); return s; }",
    );
}

// ---- (3) Card K2-BUILTIN-BIT-COUNTS (rule 94): a bit-count builtin under a
// nonzero guard. Each body's base answer (287dce8ed) is recorded in
// docs/reports/k2-builtin-bit-counts/probes/ (`probe1`, `probe3`): every one
// was Unknown there. An undischarged site keeps that refusal byte for byte.

const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

fn uncontracted(builtin: &str) -> String {
    format!("unsupported:call to uncontracted function `{builtin}`")
}

fn unknown_with(body: &str, keys: &[&str]) {
    let expected = keys.iter().map(|k| (*k).to_owned()).collect::<Vec<_>>();
    assert_eq!(outcome(body), ("Unknown", expected), "{body}");
}

#[test]
fn a_guarded_bit_count_is_admitted_in_every_width() {
    // snappy.c's `log2_floor` (`rd_clz` is `__builtin_clz`).
    safe("int w(unsigned n) { return n == 0 ? -1 : 31 ^ __builtin_clz(n); }");
    safe("int w(unsigned n) { return n ? __builtin_ctz(n) : 32; }");
    safe("int w(unsigned n) { return n > 0 ? __builtin_ctz(n) : 32; }");
    // A subject's own `unsigned long` formal (no contract position).
    safe("int w(unsigned long n) { return n ? __builtin_clzl(n) : 64; }"); // llp64-scan: allow
    safe("int w(unsigned long n) { return n ? __builtin_ctzl(n) : 64; }"); // llp64-scan: allow
    safe("int w(unsigned long long n) { return n ? __builtin_clzll(n) : 64; }");
    safe("int w(unsigned long long n) { return n != 0 ? __builtin_ctzll(n) : 64; }");
    // A widening operand, a signed operand of the same rank, a `const` local.
    safe("int w(unsigned n) { return n ? __builtin_ctzll(n) : 64; }");
    safe("int w(int n) { return n > 0 ? __builtin_ctz(n) : 0; }");
    safe("int w(unsigned m) { const unsigned n = m; return n ? __builtin_ctz(n) : 0; }");
    // `&&`, `||` and a connective guard.
    safe("int w(unsigned n) { return n && __builtin_clz(n) > 3; }");
    safe("int w(unsigned n) { return n == 0 || __builtin_clz(n) > 3; }");
    safe("int w(unsigned n) { return (n != 0 && n < 8) ? __builtin_clz(n) : 0; }");
}

#[test]
fn a_statement_guard_is_admitted_on_structured_flow() {
    safe("int w(unsigned n) { if (n != 0) return __builtin_ctz(n); return -1; }");
    safe("int w(unsigned n) { if (n == 0) return -1; return __builtin_clz(n); }");
    safe("int w(unsigned n) { if (!n) return 32; return __builtin_ctz(n); }");
    safe("int w(unsigned n) { if (n == 0) { return -1; } return 31 ^ __builtin_clz(n); }");
    safe("int w(unsigned n) { int r; if (n) { r = __builtin_ctz(n); } else { r = 0; } return r; }");
    safe("int w(unsigned n) { int r; if (n == 0) r = 0; else r = __builtin_ctz(n); return r; }");
}

#[test]
fn an_unguarded_bit_count_keeps_the_base_refusal() {
    for builtin in ["__builtin_clz", "__builtin_ctz"] {
        let body = format!("int w(unsigned n) {{ return {builtin}(n); }}");
        unknown_with(&body, &[&uncontracted(builtin)]);
    }
    unknown_with(
        "int w(unsigned long n) { return __builtin_clzl(n); }", // llp64-scan: allow
        &[&uncontracted("__builtin_clzl")],
    );
    unknown_with(
        "int w(unsigned long long n) { return __builtin_ctzll(n); }",
        &[&uncontracted("__builtin_ctzll")],
    );
    unknown_with(
        "int w(unsigned n) { int r; r = __builtin_ctz(n); return r; }",
        &[&uncontracted("__builtin_ctz")],
    );
    // lz4.c's `LZ4_NbCommonBytes`: `assert(val != 0)` is `((void)0)` under
    // NDEBUG, so nothing proves the operand nonzero.
    unknown_with(
        "unsigned w(unsigned long long val) { ((void)0); return (unsigned)__builtin_ctzll((unsigned long long)val) >> 3; }",
        &[RETURN],
    );
}

#[test]
fn a_guard_that_does_not_prove_the_operand_nonzero_keeps_the_refusal() {
    // The wrong arm, another object, a test that says nothing of zero.
    unknown_with(
        "int w(unsigned n) { return n == 0 ? __builtin_clz(n) : -1; }",
        &[RETURN],
    );
    unknown_with(
        "int w(unsigned n, unsigned m) { return n == 0 ? -1 : __builtin_clz(m); }",
        &[RETURN],
    );
    unknown_with(
        "int w(unsigned n) { return n >= 0 ? __builtin_clz(n) : 0; }",
        &[RETURN],
    );
    unknown_with(
        "int w(unsigned n) { return n == 1 ? __builtin_clz(n) : 0; }",
        &[RETURN],
    );
    // A branch that does not always return; a write after the test.
    unknown_with(
        "int w(unsigned n) { if (n) {} return __builtin_clz(n); }",
        &[&uncontracted("__builtin_clz")],
    );
    unknown_with(
        "int w(unsigned n) { if (n == 0) { n = 1; } return __builtin_clz(n); }",
        &[&uncontracted("__builtin_clz")],
    );
    unknown_with(
        "int w(unsigned n) { if (n) { n = 0; return __builtin_ctz(n); } return 0; }",
        &[&uncontracted("__builtin_ctz")],
    );
    unknown_with(
        "int w(unsigned n) { int r = 0; while (n) { r += __builtin_ctz(n); n &= n - 1; } return r; }",
        &[&uncontracted("__builtin_ctz")],
    );
}

#[test]
fn an_unstable_or_narrowed_operand_keeps_the_refusal() {
    unknown_with(
        "int w(unsigned long long v) { return v ? __builtin_ctz((unsigned)v) : 0; }",
        &[RETURN],
    );
    unknown_with(
        "int w(volatile unsigned n) { return n ? __builtin_ctz(n) : 0; }",
        &[RETURN],
    );
    unknown_with(
        "int w(void) { static unsigned s = 1; return s ? __builtin_ctz(s) : 0; }",
        &[RETURN],
    );
    unknown_with(
        "unsigned g; int w(void) { return g ? __builtin_ctz(g) : 0; }",
        &[RETURN],
    );
    let (status, keys) =
        outcome("int w(unsigned n) { unsigned *p = &n; return n ? __builtin_ctz(n) : 0; }");
    assert_eq!(status, "Unknown");
    assert!(keys.iter().any(|k| k == RETURN), "{keys:?}");
}

#[test]
fn unstructured_flow_or_a_mixed_body_keeps_the_refusal() {
    unknown_with(
        "int w(unsigned n) { if (n == 0) return -1; L: return __builtin_clz(n); }",
        &[&uncontracted("__builtin_clz")],
    );
    unknown_with(
        "int w(unsigned n, int k) { switch (k) { case 0: if (n == 0) return -1; case 1: return __builtin_clz(n); } return 0; }",
        &[&uncontracted("__builtin_clz")],
    );
    // One undischarged site leaves the whole body at base.
    let mixed =
        "int w(unsigned n) { int a = n ? __builtin_ctz(n) : 0; return a + __builtin_clz(n); }";
    let mut expected = vec![uncontracted("__builtin_ctz"), RETURN.to_owned()];
    expected.sort();
    assert_eq!(outcome(mixed), ("Unknown", expected), "{mixed}");
}
