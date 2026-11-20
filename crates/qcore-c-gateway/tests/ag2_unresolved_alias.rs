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

//! Card AG2-UNRESOLVED-ALIAS (gateway rule 62): copy sources that carry no
//! resource identity. Each admitted shape is paired with its near misses:
//! a custody use of the copy, a write to the formal, a release through
//! another path, a store or return of the copy, a loop cursor, a join with a
//! live binding, a formal destination, a file-scope object and a shadowed
//! enumerator. Every `Safe` result rechecks its certificate. The Lean model
//! is `QCore.CustodyFreeCopy`; its decision vectors are bound to the Rust
//! table below.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const LEAN: &str = include_str!("../../../proof/QCore/CustodyFreeCopy.lean");

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
           ensures normal resource_return_none(); };\n\
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_cmp(a: ptr<const, u8>, b: ptr<const, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(a);\n\
           ensures normal resource_borrowed(b); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef unsigned long x_size; typedef int x_i32; typedef unsigned int x_u32; \
char *x_fresh(x_size n); void x_free(void *p); \
x_size x_len(const char *s); int x_cmp(const char *a, const char *b); \
void x_unknown(void *p); \
struct x_rec { int count; unsigned long total; struct x_rec *next; char *name; }; \
typedef struct x_rec x_rec_t; \
enum x_err { X_OK = 0, X_FAIL = -1 }; typedef enum x_err x_err_t; \
int x_global; struct x_rec *x_global_rec; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "ag2_unresolved_alias.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent AG2 test: {reason}");
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

const INDIRECT: &str = "unsupported:indirect memory access is outside the resource projection";
const ASSIGNMENT: &str = "unsupported:unsupported assignment expression";

// -- S3: a string literal copied into a local ---------------------------------

#[test]
fn a_literal_copied_into_a_local_carries_no_identity() {
    expect(
        "`s = \"bb\";` (cmt_encode_text append_summary_metric_value)",
        "x_size w(int c) { const char *s = \"a\"; if (c) s = \"bb\"; return x_len(s); }",
        "Safe",
        &[],
    );
    expect(
        "both arms assign a literal (mk_print's header_title)",
        "x_size w(int c) { const char *s; if (c) s = \"a\"; else s = \"bb\"; return x_len(s); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_release_of_a_literal_copy_still_refuses() {
    expect(
        "the copy is unbound: freeing it names the copy, never a site",
        "void w(int c) { char *s = \"a\"; x_free(s); }",
        "Unknown",
        &["unknown_alias:s"],
    );
    expect(
        "bound on one path, a literal on the other: the release refuses",
        "void w(int c) { char *s = x_fresh(4); if (c) s = \"a\"; x_free(s); }",
        "Unknown",
        &["unknown_alias:s"],
    );
}

#[test]
fn a_literal_over_a_live_binding_keeps_the_leak() {
    expect(
        "the overwritten acquisition is still live at the exit",
        "void w(void) { char *s = x_fresh(4); s = \"a\"; (void) s; }",
        "Unsafe",
        &["Leak"],
    );
}

#[test]
fn a_shadowed_destination_is_not_read() {
    expect(
        "two locals spelled `s`: the projection names them as one, so no reading",
        "void w(void) { char *s = x_fresh(4); { char *s = \"a\"; (void) s; } x_free(s); }",
        "Unknown",
        &["unknown_alias:__qcore_literal__", "unknown_alias:s"],
    );
}

#[test]
fn a_literal_into_a_formal_is_not_read() {
    expect(
        "the destination must be an automatic local",
        "x_size w(const char *s) { s = \"a\"; return x_len(s); }",
        "Unknown",
        &["unknown_alias:__qcore_literal__"],
    );
}

// -- S2: an enumeration constant copied into a local --------------------------

#[test]
fn an_enumerator_of_an_anonymous_enum_carries_no_identity() {
    expect(
        "`rd_kafka_resp_err_t err = RD_KAFKA_RESP_ERR_NO_ERROR;` (typedef of an anonymous enum)",
        "typedef enum { Y_OK = 0, Y_BAD = 1 } y_err_t; \
         y_err_t w(int c) { y_err_t err = Y_OK; if (c) err = Y_BAD; return err; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_shadowed_enumerator_is_the_local_it_names() {
    expect(
        "a local spelled like an enumerator is a declared scalar (unchanged)",
        "typedef enum { Z_OK = 0 } z_err_t; int w(void) { int Z_OK = 3; int e = Z_OK; return e; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_file_scope_object_is_not_read() {
    expect(
        "rule 54 keeps static objects out; so does rule 62",
        "int w(void) { int r; r = x_global; return r; }",
        "Unknown",
        &["unknown_alias:x_global"],
    );
    expect(
        "a static pointer object",
        "int w(void) { struct x_rec *r; r = x_global_rec; return r->count; }",
        "Unknown",
        &["unknown_alias:x_global_rec"],
    );
}

// -- S1: an unbound pointer formal copied into a read-only local --------------

#[test]
fn a_callback_context_copy_reads_as_the_formal() {
    expect(
        "`decode_context = (T *) context;` under a null test (cmt_decode_msgpack)",
        "static int w(void *reader, void *ctx) { struct x_rec *r; \
         if (0 == reader || 0 == ctx) return -1; r = (struct x_rec *) ctx; \
         return r->next->count; } int (*x_cbp)(void *, void *) = w;",
        "Safe",
        &[],
    );
    expect(
        "the same body on the formal itself (the control)",
        "static int w(void *reader, struct x_rec *ctx) { if (0 == reader || 0 == ctx) return -1; \
         return ctx->next->count; } int (*x_cbp)(void *, struct x_rec *) = w;",
        "Safe",
        &[],
    );
    expect(
        "`encoder = (T *) context; return encoder->arena;` (get_context_arena)",
        "struct x_rec *w(void *ctx) { struct x_rec *r = (struct x_rec *) ctx; return r->next; } \
         struct x_rec *(*x_g)(void *) = w;",
        "Safe",
        &[],
    );
    expect(
        "the same body on the formal itself (the control)",
        "struct x_rec *w(struct x_rec *r) { return r->next; } \
         struct x_rec *(*x_g)(struct x_rec *) = w;",
        "Safe",
        &[],
    );
}

#[test]
fn a_released_peer_still_blocks_a_read_through_the_copy() {
    expect(
        "q may alias the copy: its release blocks the read (LEXICAL-SCALAR-NOOP-ALIAS-READ)",
        "int w(void *ctx, struct x_rec *q) { struct x_rec *r = ctx; if (r == 0) return 0; \
         x_record_free(q); return r->next->count; } int (*x_p)(void *, struct x_rec *) = w;",
        "Unknown",
        &["unknown_alias:q", "unknown_alias:r/q"],
    );
}

#[test]
fn a_custody_use_of_a_copy_withholds_the_reading() {
    expect(
        "a second copy released before the first is read",
        "int w(void *ctx) { struct x_rec *r = ctx; struct x_rec *s = ctx; if (r == 0) return 0; \
         x_record_free(s); return r->next->count; } int (*x_p)(void *) = w;",
        "Unknown",
        &[
            "unknown_alias:ctx",
            "unknown_alias:r",
            "unknown_alias:r/s",
            "unknown_alias:s",
        ],
    );
    expect(
        "the copy stored into a member",
        "void w(void *ctx, struct x_rec *o) { struct x_rec *r = ctx; if (r == 0) return; \
         o->next = r; } void (*x_p)(void *, struct x_rec *) = w;",
        "Unknown",
        &["unknown_alias:ctx", "unknown_alias:r", INDIRECT],
    );
    expect(
        "the copy handed back: rule 36's question, not this rule's",
        "struct x_rec *w(void *ctx) { struct x_rec *r = ctx; if (r == 0) return 0; return r; } \
         struct x_rec *(*x_p)(void *) = w;",
        "Unknown",
        &["unknown_alias:ctx", "unknown_alias:r"],
    );
    expect(
        "the copy handed to an uncontracted callee",
        "int w(void *ctx) { struct x_rec *r = ctx; if (r == 0) return 0; x_unknown(r); \
         return r->next->count; } int (*x_p)(void *) = w;",
        "Unknown",
        &[
            "unknown_alias:ctx",
            "unknown_alias:r",
            "unsupported:call to uncontracted function `x_unknown`",
        ],
    );
    expect(
        "the copy's address taken",
        "int w(void *ctx) { struct x_rec *r = ctx; struct x_rec **pp = &r; if (r == 0) return 0; \
         return (*pp)->count; } int (*x_p)(void *) = w;",
        "Unknown",
        &[
            "unknown_alias:ctx",
            "unknown_alias:r",
            "unsupported:unmodeled call, aggregate or indirect access in return expression",
            ASSIGNMENT,
        ],
    );
}

#[test]
fn a_released_copy_keeps_the_w4_signal() {
    // W4 summary derivation binds the formal the copy names, so the release
    // through the copy validates as a `Released` summary exactly as before.
    for (label, body) in [
        (
            "released through the copy",
            "int w(void *ctx) { struct x_rec *r = ctx; if (r == 0) return 0; x_record_free(r); \
             return 0; } int (*x_p)(void *) = w;",
        ),
        (
            "released through a copy of the copy",
            "int w(void *ctx) { struct x_rec *r = ctx; struct x_rec *s; if (r == 0) return 0; \
             s = r; x_record_free(s); return 0; } int (*x_p)(void *) = w;",
        ),
        (
            "released without a null test",
            "void w(void *ctx) { struct x_rec *r = ctx; x_record_free(r); }",
        ),
    ] {
        expect(label, body, "Safe", &[]);
    }
}

#[test]
fn a_rebound_formal_and_a_loop_cursor_keep_their_answers() {
    expect(
        "the formal is written before the copy",
        "int w(void *ctx) { struct x_rec *r; ctx = x_record(); r = ctx; if (r == 0) return 0; \
         return r->next->count; } int (*x_p)(void *) = w;",
        "Unsafe",
        &["Leak", "unknown_alias:r->next"],
    );
    expect(
        "a cursor walking from the formal",
        "int w(void *ctx) { struct x_rec *r; int n = 0; \
         for (r = (struct x_rec *) ctx; r; r = r->next) n += r->count; return n; } \
         int (*x_p)(void *) = w;",
        "Unknown",
        &["unknown_alias:r", ASSIGNMENT],
    );
}

#[test]
fn the_lean_decision_vectors_match_the_shared_table() {
    let lean = LEAN.replace("\r\n", "\n");
    let block = lean
        .split("def decisionVectors ")
        .nth(1)
        .expect("decisionVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(|field| field.trim().trim_start_matches('.').to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let expected: Vec<Vec<String>> = include_str!("../src/custody_free_copy.rs")
        .split("pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[")
        .nth(1)
        .expect("the Rust table")
        .split("];")
        .next()
        .expect("end of the Rust table")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            line.trim_start_matches('(')
                .trim_end_matches("),")
                .split(',')
                .map(|field| field.trim().trim_matches('"').to_owned())
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), 20);
    assert_eq!(rows, expected);
}
