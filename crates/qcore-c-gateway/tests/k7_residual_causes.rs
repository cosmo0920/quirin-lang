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

//! Card **K7-RESIDUAL-CAUSES** (gateway rule 78), end to end through the real
//! gateway: a pointer offset under an indirection or a subscript reads as the
//! subscript C11 6.5.2.1p2 defines it to be, with the subscript's obligation,
//! and every near miss keeps its refusal. Every row asserts the status and the
//! complete set of diagnostic keys over the synthetic `k_*` / `d_*` contracts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
int k_open(const char *path); \
int k_close(int fd); \
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
__SIZE_TYPE__ d_len(const char *s); \
char *d_chr(const char *s, int c); \
int ext(int); \
struct buf { const char *content; __SIZE_TYPE__ offset; __SIZE_TYPE__ length; }; \
struct box { long n; char *name; struct box *next; int (*cb)(int); };";

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

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent K7 test: {reason}");
            false
        }
    }
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
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result).functions.get("w").expect("w analysed");
    if status == "Safe" {
        assert_eq!(function.status, AnalysisStatus::Safe);
    }
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

const BRANCH: &str = "unsupported:unmodeled call, aggregate or indirect access in branch condition";
const ACCESS: &str = "unsupported:indirect memory access is outside the resource projection";

/// cJSON's `buffer_skip_whitespace` loop condition, as the production unit
/// preprocesses it (`can_access_at_index` and `buffer_at_offset` expanded).
const PARSE_BUFFER: &str = "typedef struct { const unsigned char *content; __SIZE_TYPE__ length; \
     __SIZE_TYPE__ offset; __SIZE_TYPE__ depth; } parse_buffer;";

fn pair(offset: &str, subscript: &str) {
    let measured = outcome(offset);
    let reference = outcome(subscript);
    assert_eq!(
        measured, reference,
        "the offset spelling reads as the subscript spelling\n{offset}\n{subscript}"
    );
}

// -- (1) the offset is read as its subscript ----------------------------------

/// `*(P + I)`, `*(I + P)`, `*(P - I)` and `(P + I)[K]` read exactly as the
/// subscripts C11 6.5.2.1p2 defines them to be, in a condition, a loop
/// condition, a statement read, a store and a return, on a formal, a member
/// base and a nested place: the same status and the same diagnostics as the
/// subscript spelling, which is `Safe` here.
#[test]
fn an_offset_reads_as_its_subscript() {
    if !clang_available() {
        return;
    }
    let pairs = [
        (
            "int w(const char *str) { if (*(str + 1) == 's') return 1; return 0; }",
            "int w(const char *str) { if (str[1] == 's') return 1; return 0; }",
        ),
        (
            "int w(const char *str) { if (*(1 + str) == 's') return 1; return 0; }",
            "int w(const char *str) { if (str[1] == 's') return 1; return 0; }",
        ),
        (
            "int w(const char *s) { if (*(s - 1) == 0) return 1; return 0; }",
            "int w(const char *s) { if (s[-1] == 0) return 1; return 0; }",
        ),
        (
            "int w(char *str) { switch (*str) { case 'g': return 1; case 'm': if (*(str + 1) == 's') { return 2; } } return 3; }",
            "int w(char *str) { switch (*str) { case 'g': return 1; case 'm': if (str[1] == 's') { return 2; } } return 3; }",
        ),
        (
            "int w(const char *s) { int c = *(s + 2); return c; }",
            "int w(const char *s) { int c = s[2]; return c; }",
        ),
        (
            "int w(char *s) { *(s + 1) = 'x'; return 0; }",
            "int w(char *s) { s[1] = 'x'; return 0; }",
        ),
        (
            "int w(char **v, int i) { if (*(*(v + i) + 1) == 'x') return 1; return 0; }",
            "int w(char **v, int i) { if (v[i][1] == 'x') return 1; return 0; }",
        ),
        (
            "int w(struct box *b) { if (*(b->name + b->n) == 0) return 1; return 0; }",
            "int w(struct box *b) { if (b->name[b->n] == 0) return 1; return 0; }",
        ),
        (
            "int w(const char *s) { if (*((const unsigned char *)s + 1) == 0) return 1; return 0; }",
            "int w(const char *s) { if (((const unsigned char *)s)[1] == 0) return 1; return 0; }",
        ),
        (
            "int w(struct buf *b) { int n = 0; while ((b->content + b->offset)[0] <= 32) { b->offset++; n++; } return n; }",
            "int w(struct buf *b) { int n = 0; while (b->content[b->offset + 0] <= 32) { b->offset++; n++; } return n; }",
        ),
        (
            "int w(struct buf *b) { if ((b != 0) && (b->offset < b->length) && ((b->content + b->offset)[0] <= 32)) return 1; return 0; }",
            "int w(struct buf *b) { if ((b != 0) && (b->offset < b->length) && (b->content[b->offset] <= 32)) return 1; return 0; }",
        ),
    ];
    for (offset, subscript) in pairs {
        pair(offset, subscript);
        assert_eq!(outcome(offset), ("Safe", Vec::new()), "{offset}");
    }
    let cjson = format!(
        "{PARSE_BUFFER} int w(parse_buffer * const buffer) {{ while (((buffer != ((void*)0)) && \
         (((buffer)->offset + 0) < (buffer)->length)) && (((buffer)->content + (buffer)->offset)[0] <= 32)) \
         {{ buffer->offset++; }} return 0; }}"
    );
    assert_eq!(outcome(&cjson), ("Safe", Vec::new()), "{cjson}");
}

/// The place keeps the subscript's per-hop obligation: a read through a
/// released pointer is the same `use_after_release` candidate, an unchecked
/// allocation the same `unknown_alias`, a live checked one `Safe`.
#[test]
fn the_offset_keeps_the_subscripts_obligation() {
    if !clang_available() {
        return;
    }
    let pairs = [
        (
            "int w(void) { char *p = k_alloc(4); if (!p) return 0; k_free(p); if (*(p + 1) == 's') return 1; return 0; }",
            "int w(void) { char *p = k_alloc(4); if (!p) return 0; k_free(p); if (p[1] == 's') return 1; return 0; }",
        ),
        (
            "int w(void) { char *p = k_alloc(4); if (*(p + 1) == 's') { k_free(p); return 1; } k_free(p); return 0; }",
            "int w(void) { char *p = k_alloc(4); if (p[1] == 's') { k_free(p); return 1; } k_free(p); return 0; }",
        ),
        (
            "int w(void) { char *p = k_alloc(4); int r; if (!p) return 0; r = *(p + 1) == 's'; k_free(p); return r; }",
            "int w(void) { char *p = k_alloc(4); int r; if (!p) return 0; r = p[1] == 's'; k_free(p); return r; }",
        ),
    ];
    for (offset, subscript) in pairs {
        pair(offset, subscript);
    }
    assert_eq!(
        outcome(pairs[0].0),
        (
            "Unsafe",
            vec!["UseAfterRelease".to_owned(), "unknown_alias:p".to_owned()]
        )
    );
    assert_eq!(
        outcome(pairs[1].0),
        ("Unknown", vec!["unknown_alias:p".to_owned()])
    );
    assert_eq!(outcome(pairs[2].0), ("Safe", Vec::new()));
}

// -- (2) near misses keep their refusal ---------------------------------------

/// An index that is not an integer expression the reader reads (a pointer
/// difference), an offset under `->` (not a subscript), a pointer operand
/// that is itself an offset, and the compound-assignment route the rule does
/// not touch keep their older refusal byte for byte.
#[test]
fn near_misses_keep_their_refusal() {
    if !clang_available() {
        return;
    }
    for (body, refusal) in [
        (
            "int w(const char *s, const char *e) { if (*(s + (e - s)) == 0) return 1; return 0; }",
            BRANCH,
        ),
        (
            "int w(struct box *b) { if ((b + 1)->n == 0) return 1; return 0; }",
            BRANCH,
        ),
        (
            "int w(const char *s, int i, int j) { if (*((s + i) + j) == 0) return 1; return 0; }",
            BRANCH,
        ),
        (
            "int w(int *a, int i) { int s = 0; while (*(a + i) != 0) { s += *(a + i); i++; } return s; }",
            ACCESS,
        ),
    ] {
        assert_eq!(
            outcome(body),
            ("Unknown", vec![refusal.to_owned()]),
            "{body}"
        );
    }
}
