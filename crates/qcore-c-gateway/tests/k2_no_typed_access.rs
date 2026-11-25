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
