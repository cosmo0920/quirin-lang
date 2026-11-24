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

//! Card K5-LOCAL-ORIGIN-ALIASES (gateway rule 77): an automatic local that
//! only ever holds a pointer loaded from an untracked object is decided at a
//! null test and at a `return` as the load itself is. Each admitted shape is
//! a body of the user's `current` run reduced to its alias shape (the base
//! answer, `Unknown` on the local, is noted with it), paired with the same
//! body read through the path directly where one exists. The near misses keep
//! their base answer: a release of the local, a released or rebound or
//! uncontracted-handed root, an acquisition on one path, an address, a step,
//! a call-result root, an interior address, a loop that frees, a copy that is
//! released, a shadowed name and a file-scope array. Every `Safe` rechecks its
//! certificate. The Lean model is `QCore.BorrowedPathLocal`.

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
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ x_size; \
char *x_fresh(x_size n); void x_free(void *p); x_size x_len(const char *s); \
void x_unknown(void *p); void x_out(char **p); \
struct x_rec { int count; unsigned long total; struct x_rec *next; char *name; \
               struct x_rec **kids; void *priv; struct x_opts { char *user; } opts; }; \
struct x_rec *x_record(void);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k5_local_origin_aliases.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent K5 test: {reason}");
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

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS}\n{body}");
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

// -- Admitted: null tests -------------------------------------------------------

#[test]
fn a_null_test_of_a_loaded_local_is_decided() {
    // Base: `Unknown`, `unknown_alias:n` at the test (rule 10b's arm).
    expect(
        "assigned in the condition (rd_kafka_topic_partition_get_leader_epoch)",
        "int w(const struct x_rec *r) { const struct x_rec *n; \
         if (!(n = r->next)) return -1; return n->count; }",
        "Safe",
        &[],
    );
    expect(
        "an initializer through `.` members and a void * member (rd_kafka_sasl_cyrus_ready)",
        "int w(struct x_rec *r, int t) { struct x_rec *h = r->priv; char *u = r->opts.user; \
         if (!t) return 1; if (!h || !u) return 0; return h->count > 0; }",
        "Safe",
        &[],
    );
    expect(
        "the same body read through the path (the unguarded reading, Safe at base)",
        "int w(struct x_rec *r) { return r->next->count; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_cursor_and_nested_loads_are_decided() {
    // Base: `Unknown`, `unknown_alias:c` (and `s` below) at the tests.
    expect(
        "a cursor loading from itself (cJSON_GetArraySize)",
        "int w(const struct x_rec *a) { struct x_rec *c = 0; x_size s = 0; \
         if (a == 0) return 0; c = a->next; while (c != 0) { s++; c = c->next; } \
         return (int)s; }",
        "Safe",
        &[],
    );
    expect(
        "two nested loaded locals under a stepped index (request_data_point_count)",
        "int w(struct x_rec *q, x_size m) { x_size i, j; struct x_rec *r, *s; \
         for (i = 0; i < m; i++) { r = q->kids[i]; if (r == 0) return -1; \
           for (j = 0; j < m; j++) { s = r->kids[j]; if (s == 0) return -1; \
             if (s->count) return 1; } } return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_literal_rebinding_and_a_borrowed_actual_keep_the_reading() {
    // Base: `Unknown`, `unknown_alias:u` at the test.
    expect(
        "loaded, tested, reassigned a literal (format_acl_error_message)",
        "x_size w(struct x_rec *r) { char *u = r->name; if (u == 0) { u = \"\"; } \
         return x_len(u); }",
        "Safe",
        &[],
    );
    expect(
        "tested, then handed to a `Borrowed` position",
        "x_size w(struct x_rec *r) { char *n = r->name; if (n == 0) return 0; \
         return x_len(n); }",
        "Safe",
        &[],
    );
}

// -- Admitted: returns ----------------------------------------------------------

#[test]
fn a_returned_loaded_local_escapes_nothing() {
    // Base: `Unknown`, `unknown_alias:c` at the return.
    expect(
        "returned after a use (get_hash_table)",
        "char *w(struct x_rec *e) { char *t; t = e->name; x_len(t); return t; }",
        "Safe",
        &[],
    );
    expect(
        "joined with null and returned",
        "struct x_rec *w(struct x_rec *a) { struct x_rec *c; if (a) c = a->next; \
         else c = 0; return c; }",
        "Safe",
        &[],
    );
    expect(
        "a cursor returned from a loop",
        "struct x_rec *w(struct x_rec *a, int k) { struct x_rec *c = a->next; \
         while (c) { if (c->count == k) return c; c = c->next; } return 0; }",
        "Safe",
        &[],
    );
    expect(
        "the same load returned directly (rule 10k (2), Safe at base)",
        "struct x_rec *w(struct x_rec *a) { return a->next; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_store_through_the_root_after_the_load_changes_nothing() {
    // The identity-free store is admitted by its own rule; the loaded object
    // is still the caller's. Base: `Unknown`, `unknown_alias:n` at the test.
    expect(
        "a null stored into the loaded member after the load",
        "int w(struct x_rec *r) { struct x_rec *n = r->next; r->next = 0; \
         if (n) return n->count; return 0; }",
        "Safe",
        &[],
    );
}

// -- Near misses: unchanged from base -------------------------------------------

#[test]
fn a_release_of_a_loaded_local_still_refuses() {
    expect(
        "released after the test",
        "int w(struct x_rec *r) { char *n = r->name; if (n) { x_free(n); return 1; } \
         return 0; }",
        "Unknown",
        &["unknown_alias:n"],
    );
    expect(
        "copied, then the copy released",
        "int w(struct x_rec *r) { char *n = r->name; char *m; if (!n) return 0; m = n; \
         x_free(m); return 1; }",
        "Unknown",
        &["unknown_alias:m"],
    );
    expect(
        "a cursor that frees",
        "void w(struct x_rec *a) { struct x_rec *c; for (c = a->next; c; c = c->next) \
         x_free(c); }",
        "Unknown",
        &["unknown_alias:c"],
    );
}

#[test]
fn a_root_that_is_released_rebound_or_handed_on_is_no_root() {
    expect(
        "the root released between load and test",
        "int w(struct x_rec *r) { struct x_rec *n = r->next; x_free(r); \
         if (n) return n->count; return 0; }",
        "Unknown",
        &["unknown_alias:n", "unknown_alias:n/r", "unknown_alias:r"],
    );
    expect(
        "the root rebound",
        "int w(struct x_rec *r) { struct x_rec *n; r = r->next; n = r->next; \
         if (n) return n->count; return 0; }",
        "Unknown",
        &["unknown_alias:n"],
    );
    expect(
        "the root handed to an uncontracted callee",
        "int w(struct x_rec *r) { struct x_rec *n; x_unknown(r); n = r->next; \
         if (n) return n->count; return 0; }",
        "Unknown",
        &[
            "unknown_alias:n",
            "unknown_alias:r",
            "unsupported:call to uncontracted function `x_unknown`",
        ],
    );
    expect(
        "a call result as the base",
        "int w(void) { struct x_rec *q = x_record(); struct x_rec *n = q->next; \
         if (n) return n->count; return 0; }",
        "Unknown",
        &[
            "unknown_alias:n",
            "unknown_alias:q",
            "unsupported:call to uncontracted function `x_record`",
        ],
    );
}

#[test]
fn a_local_written_or_used_otherwise_is_not_read() {
    expect(
        "an acquisition on one path: the leak stays",
        "int w(struct x_rec *r, int c) { char *n = r->name; if (c) n = x_fresh(4); \
         if (n) return 1; return 0; }",
        "Unsafe",
        &["Leak", "unknown_alias:n"],
    );
    expect(
        "its address handed out",
        "int w(struct x_rec *r) { char *n = r->name; x_out(&n); if (n) return 1; return 0; }",
        "Unknown",
        &[
            "unknown_alias:n",
            "unsupported:call to uncontracted function `x_out`",
        ],
    );
    expect(
        "stepped",
        "int w(struct x_rec *r) { char *n = r->name; if (!n) return 0; n++; return n[0]; }",
        "Unknown",
        &["unknown_alias:n"],
    );
    expect(
        "an interior address returned (rule 5j)",
        "struct x_rec *w(struct x_rec *r) { struct x_rec *n = &r->next[0]; return n; }",
        "Unknown",
        &["unknown_alias:n"],
    );
    expect(
        "a shadowed spelling",
        "int w(struct x_rec *r, int c) { struct x_rec *n = r->next; \
         if (c) { struct x_rec *n = r; if (n) return 2; } if (n) return n->count; return 0; }",
        "Unknown",
        &["unknown_alias:n", "unknown_alias:r"],
    );
    expect(
        "a file-scope array's decay (AG2's S7 boundary)",
        "struct x_prop { const char *name; int scope; }; \
         static const struct x_prop x_props[] = {{\"a\", 1}, {0, 0}}; \
         const struct x_prop *w(int scope) { const struct x_prop *p; \
         for (p = x_props; p->name; p++) { if (p->scope & scope) return p; } return 0; }",
        "Unknown",
        &["unknown_alias:p", "unknown_alias:x_props"],
    );
}

#[test]
fn a_leak_on_the_tested_path_is_reported_as_before() {
    // The guard used to mark both edges unmodelled; the leak on the null edge
    // was already reported at base (with the alias refusal beside it).
    expect(
        "the acquisition leaks when the loaded local is null",
        "int w(struct x_rec *r) { struct x_rec *n = r->next; char *x = x_fresh(1); \
         if (n) { x_free(x); return 1; } return 0; }",
        "Unsafe",
        &["Leak"],
    );
    expect(
        "released on both paths",
        "int w(struct x_rec *r) { struct x_rec *n = r->next; char *x = x_fresh(1); \
         if (n) { x_free(x); return 1; } x_free(x); return 0; }",
        "Safe",
        &[],
    );
}
