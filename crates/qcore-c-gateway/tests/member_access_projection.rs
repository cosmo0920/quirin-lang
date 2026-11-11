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

//! Card MEMBER-ACCESS-PROJECTION (gateway rule 38h): an uninitialized
//! declaration the lexical gate refused, read from the Clang AST
//! (`docs/reports/member-access-projection/`).
//!
//! Each fixture is a census shape of the 046accf6 `current` run. Admitted:
//! typedef-specified multi-declarators and arrays, and a constant-expression
//! bound, exactly as their keyword spellings are. Kept: an initialized
//! multi-declarator, a variably modified array, and the flow refusals a
//! declaration never suppresses (a double release, a loop re-acquisition).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const GATE: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let files: BTreeMap<String, String> = [(
        "libc/memory.qv".to_owned(),
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv").to_owned(),
    )]
    .into_iter()
    .collect();
    load_resource_contract_sources(&files).expect("the libc module loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent MEMBER-ACCESS-PROJECTION test: {reason}");
            false
        }
    }
}

/// The entry's status and its diagnostics' descriptions.
fn row(entry: &str, source: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("#include <stdlib.h>\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let function = &summary.functions[entry];
    (
        function.status,
        function
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    )
}

fn assert_safe(entry: &str, source: &str) {
    let (status, diagnostics) = row(entry, source);
    assert_eq!(status, AnalysisStatus::Safe, "{entry}: {diagnostics:?}");
    assert!(diagnostics.is_empty(), "{entry}: {diagnostics:?}");
}

fn assert_gate_refusal(entry: &str, source: &str) {
    let (status, diagnostics) = row(entry, source);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}");
    assert!(
        diagnostics.iter().any(|d| d == GATE),
        "{entry}: {diagnostics:?}"
    );
}

#[test]
fn typedef_specified_declarations_are_read_as_their_keyword_spellings() {
    if !clang_available() {
        return;
    }
    // `rd_segment_t *seg, *tmp;` (rd_buf_destroy).
    assert_safe(
        "multi",
        "typedef struct seg { int v; } seg_t;\nint multi(void) {\n  seg_t *a, *b;\n  a = NULL;\n  b = a;\n  return b == NULL;\n}",
    );
    // `uint32_t odd[32];` (crc32c_zeros_op).
    assert_safe(
        "array",
        "typedef unsigned int u32;\nu32 array(void) {\n  u32 odd[32];\n  odd[0] = 7;\n  return odd[0];\n}",
    );
    // `rd_kafka_group_member_t members[2];` (ut_testConflictingPreviousAssignments).
    assert_safe(
        "records",
        "typedef struct member { int id; } member_t;\nint records(void) {\n  member_t members[2];\n  return 0;\n}",
    );
    // `char buf[1024 * 3];`: a keyword specifier and a constant-expression bound.
    assert_safe(
        "bound",
        "int bound(void) {\n  char buf[1024 * 3];\n  buf[0] = 0;\n  return buf[0];\n}",
    );
    // The keyword spelling the rule mirrors, unchanged.
    assert_safe(
        "keyword",
        "struct kw { int v; };\nint keyword(void) {\n  struct kw *a, *b;\n  a = NULL;\n  b = a;\n  return b == NULL;\n}",
    );
}

/// librdkafka's crc32c.c bodies (Fluent Bit 66910c10 and 046accf6), the
/// prediction's corpus rows: `crc32c_zeros_op` and `crc32c_zeros` are blocked
/// only by `uint32_t odd[32];` / `uint32_t op[32];` and the call between them.
const CRC32C: &str = "#include <stdint.h>
#define POLY 0x82f63b78
static inline uint32_t gf2_matrix_times(uint32_t *mat, uint32_t vec)
{
    uint32_t sum;

    sum = 0;
    while (vec) {
        if (vec & 1)
            sum ^= *mat;
        vec >>= 1;
        mat++;
    }
    return sum;
}

static inline void gf2_matrix_square(uint32_t *square, uint32_t *mat)
{
    int n;

    for (n = 0; n < 32; n++)
        square[n] = gf2_matrix_times(mat, mat[n]);
}

static void crc32c_zeros_op(uint32_t *even, size_t len)
{
    int n;
    uint32_t row;
    uint32_t odd[32];

    odd[0] = POLY;
    row = 1;
    for (n = 1; n < 32; n++) {
        odd[n] = row;
        row <<= 1;
    }
    gf2_matrix_square(even, odd);
    gf2_matrix_square(odd, even);
    do {
        gf2_matrix_square(even, odd);
        len >>= 1;
        if (len == 0)
            return;
        gf2_matrix_square(odd, even);
        len >>= 1;
    } while (len);
    for (n = 0; n < 32; n++)
        even[n] = odd[n];
}

static void crc32c_zeros(uint32_t zeros[][256], size_t len)
{
    uint32_t n;
    uint32_t op[32];

    crc32c_zeros_op(op, len);
    for (n = 0; n < 256; n++) {
        zeros[0][n] = gf2_matrix_times(op, n);
        zeros[1][n] = gf2_matrix_times(op, n << 8);
        zeros[2][n] = gf2_matrix_times(op, n << 16);
        zeros[3][n] = gf2_matrix_times(op, n << 24);
    }
}";

#[test]
fn the_predicted_crc32c_rows() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = row("crc32c_zeros_op", CRC32C);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    // Through the in-unit summary of the now Safe callee.
    let (status, diagnostics) = row("crc32c_zeros", CRC32C);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn every_other_declaration_keeps_the_gate_refusal() {
    if !clang_available() {
        return;
    }
    // An initialized multi-declarator: outside the rule (its own follow-up).
    assert_gate_refusal(
        "initialized",
        "int initialized(void) {\n  char *p = NULL, *q = NULL;\n  return p == q;\n}",
    );
    // A variably modified array: its declaration evaluates `n` (a local, so
    // the formal-shadowing refusal is not what answers).
    assert_gate_refusal(
        "vla",
        "typedef unsigned int u32v;\nint vla(void) {\n  int n = 4;\n  u32v v[n];\n  v[0] = 1;\n  return (int)v[0];\n}",
    );
}

#[test]
fn an_admitted_declaration_suppresses_no_flow_refusal() {
    if !clang_available() {
        return;
    }
    // A double release after a typedef declaration is still found.
    let (status, diagnostics) = row(
        "twice",
        "typedef struct tw { int v; } tw_t;\nvoid twice(void) {\n  tw_t *a, *b;\n  a = malloc(sizeof(tw_t));\n  b = a;\n  free(a);\n  free(b);\n}",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert!(
        !diagnostics.iter().any(|d| d == GATE),
        "the declaration is admitted: {diagnostics:?}"
    );
    // A name redeclared in a loop keeps the loop re-acquisition refusal.
    let (status, diagnostics) = row(
        "redeclared",
        "typedef struct lq { int v; } lq_t;\nvoid redeclared(void) {\n  int i;\n  for (i = 0; i < 2; i++) {\n    lq_t *a, *b;\n    if (i == 0) {\n      a = malloc(4);\n      b = a;\n    } else {\n      free(a);\n    }\n  }\n}",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}
