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

//! Card CG10-READER-GAPS (gateway rules 71, 71a and 71b) and its two recorded
//! refusals.
//!
//! * 71: `X && (p)` is a logical AND, not the address of `p`, so the lexical
//!   closure no longer binds `p` for it and an address actual over `p`
//!   (`&rk->rk_conf.interceptors.on_send`) is read
//!   (CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL).
//! * 71a: a whole-call return whose actuals hide places behind
//!   identity-preserving views is read by rule 12i as its uncast call
//!   (CG10-CAST-VIEW-RETURN-POSITION).
//! * 71b: `T *` for a function typedef `T` is a function pointer, which rule
//!   70 does not retain (CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS).
//! * Refusals kept: a formal declared `R (*f)(A)` leaves its function
//!   unsummarised; an object pointer stored into a function-pointer member
//!   (`cio_set_log_callback`) keeps the gate.
//!
//! The fixture contracts are trusted test assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const INDIRECT: &str = "indirect memory access is outside the resource projection";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
int is_cmp(const char *a, const char *b); int is_count(const char *p);
int rg_peek(void *p); int rg_take(void *p); int rg_pair(void *p, const char *q);
typedef struct list_s { void **elems; int cnt; } list_t;
int el_method_add(list_t *list, const char *name, void *func, void *opaque);
int el_count(list_t *head);
typedef struct conf_s conf_t;
struct conf_s { int flags; struct { list_t on_new; list_t on_send; } interceptors; list_t assignors; };
typedef struct kafka_s kafka_t;
struct kafka_s { int type; int initialized; conf_t rk_conf; };
typedef int on_send_t(void *);
struct S { int a; char *name; void *data; struct S *next; };
typedef struct S S_t;
typedef unsigned char ub_t;
typedef union U { int i; char c[4]; } U_t;
";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "indirect_sole.qv",
            include_str!("fixtures/contracts/indirect_sole.qv"),
        ),
        file(
            "embedded_list.qv",
            include_str!("fixtures/contracts/embedded_list.qv"),
        ),
        file(
            "reader_gaps.qv",
            include_str!("fixtures/contracts/reader_gaps.qv"),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG10-READER-GAPS test: {reason}");
            false
        }
    }
}

fn run(source: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    )
}

fn assert_safe(source: &str) {
    let (status, descriptions) = run(source);
    assert_eq!(status, AnalysisStatus::Safe, "{source}\n{descriptions:?}");
}

fn assert_not_safe(source: &str) -> Vec<String> {
    let (status, descriptions) = run(source);
    assert_ne!(status, AnalysisStatus::Safe, "{source}\n{descriptions:?}");
    descriptions
}

fn assert_keeps_the_gate(source: &str) {
    let descriptions = assert_not_safe(source);
    assert!(
        descriptions.iter().any(|d| d == INDIRECT),
        "{source}\n{descriptions:?}"
    );
}

/// The `RD_KAFKA_INTERCEPTOR_REJECT_IF_SHARE(rk)` expansion's spelling.
const REJECT_IF_SHARE: &str =
    "do { if (((rk)->type == 1 && (rk)->rk_conf.flags)) return 3; } while (0);";

#[test]
fn rule_71_reads_a_logical_and_before_a_parenthesised_name_as_no_address() {
    if !clang_available() {
        return;
    }
    // The ten `rd_kafka_interceptor_add_on_*` rows' shape: the three-hop
    // address is read and rule 69 names rule 12t's retention.
    let (status, descriptions) = run(&format!(
        "int subject(kafka_t *rk, const char *n, on_send_t *f, void *o) {{ {REJECT_IF_SHARE} \
         return el_method_add(&rk->rk_conf.interceptors.on_send, n, (void *)f, o); }}"
    ));
    assert_eq!(status, AnalysisStatus::Unknown, "{descriptions:?}");
    assert_eq!(
        descriptions,
        [
            "call to `el_method_add` retains `( void * ) f` in `& rk - > rk_conf . interceptors \
             . on_send`, whose lifetime this body does not decide against it \
             (retention-undecided)"
                .to_owned()
        ]
    );
    // A borrowed-only helper over the same address is admitted.
    assert_safe(&format!(
        "int subject(kafka_t *rk) {{ {REJECT_IF_SHARE} \
         return el_count(&rk->rk_conf.interceptors.on_send); }}"
    ));
    assert_safe(
        "int subject(kafka_t *rk, int x) { if (x && (rk)) return 1; \
         return el_count(&rk->rk_conf.assignors); }",
    );
}

#[test]
fn rule_71_keeps_every_real_address_of_the_name() {
    if !clang_available() {
        return;
    }
    // `a && &(rk)`: three `&`, the last a unary address of `rk`.
    assert_keeps_the_gate(
        "int subject(kafka_t *rk, int x) { if (x && &(rk) != 0) return 1; \
         return el_count(&rk->rk_conf.assignors); }",
    );
    // `& (rk)` as an actual: the address of `rk`.
    assert_keeps_the_gate(
        "void keep(kafka_t **p); int subject(kafka_t *rk) { keep(&(rk)); \
         return el_count(&rk->rk_conf.assignors); }",
    );
}

#[test]
fn rule_71a_reads_a_returned_call_with_its_views_removed() {
    if !clang_available() {
        return;
    }
    // print_string's shape: a character view of a pointer member.
    assert_safe("int subject(struct S *a) { return is_count((const char *)a->name); }");
    // Through a typedef'd base the lexical reader does not resolve.
    assert_safe("int subject(const S_t *const a) { return is_count((const char *)a->name); }");
    // strcmp's shape in sort_by_map_elem_val_toppar_list_cnt: two views.
    assert_safe(
        "int subject(const S_t *a, const S_t *b) \
         { return is_cmp((const char *)a->data, (const char *)b->data); }",
    );
    // pb_enc_fixed_length_bytes' shape: a view to a scalar typedef.
    assert_safe("int subject(struct S *a) { return rg_peek((const ub_t *)a->data); }");
    // A tagged struct pointee.
    assert_safe("int subject(struct S *a) { return rg_peek((struct S *)a->data); }");
}

#[test]
fn rule_71a_keeps_every_other_returned_view_refused() {
    if !clang_available() {
        return;
    }
    // LZ4F_localSaveDict's shape: a view to a union typedef is rule 10w's.
    assert_keeps_the_gate("int subject(struct S *a) { return rg_peek((U_t *)a->data); }");
    // A kept view inside a scalar expression keeps a refusal.
    assert_not_safe("int subject(struct S *a) { return rg_peek((U_t *)a->data) + 0; }");
    // One kept view keeps the whole call refused, beside a removable one.
    assert_keeps_the_gate(
        "int subject(struct S *a) { return rg_pair((U_t *)a->data, (const char *)a->name); }",
    );
    // A scalar call inside an expression: not a whole-call return.
    assert_not_safe("int subject(struct S *a) { return 1 + is_count((const char *)a->name); }");
    // A releasing position: the uncast place is handed to a release.
    assert_not_safe("int subject(struct S *a) { return rg_take((void *)a->data); }");
    // A released tracked object behind a view is that object (H-U0's repair).
    assert_not_safe(
        "int subject(void) { void *t = q_alloc(); q_free(t); return is_count((const char *)t); }",
    );
}

const HOLDER: &str = "\
typedef int (cb_t)(void *);
typedef char obj_t;
struct h { cb_t *cb; void *o; obj_t *p; void (*free_cb)(void *); };
";

#[test]
fn rule_71b_reads_a_function_typedef_pointer_as_carrying_no_lifetime() {
    if !clang_available() {
        return;
    }
    // The callee's summary retains `o` but not `cb`: a caller handing its own
    // function-pointer formal discharges the call.
    assert_safe(&format!(
        "{HOLDER} void set(struct h *h, cb_t *cb, void *o) {{ h->cb = cb; h->o = o; }} \
         void subject(struct h *h, cb_t *f) {{ set(h, f, 0); }}"
    ));
    // Control: an object typedef pointer is retained (more obligation).
    let descriptions = assert_not_safe(&format!(
        "{HOLDER} void set(struct h *h, obj_t *p, void *o) {{ h->p = p; h->o = o; }} \
         void subject(struct h *h, obj_t *q) {{ set(h, q, 0); }}"
    ));
    assert!(
        descriptions
            .iter()
            .any(|d| d.ends_with("(retention-undecided)")),
        "{descriptions:?}"
    );
}

#[test]
fn a_function_pointer_declarator_formal_leaves_its_function_unsummarised() {
    if !clang_available() {
        return;
    }
    // Recorded refusal: the W4 lexical parameter reader reads no `R (*f)(A)`.
    let descriptions = assert_not_safe(&format!(
        "{HOLDER} void set(struct h *h, void *o, void (*free_cb)(void *)) \
         {{ h->o = o; h->free_cb = free_cb; }} void subject(struct h *h) {{ set(h, 0, 0); }}"
    ));
    assert!(
        descriptions
            .iter()
            .any(|d| d == "call to uncontracted function `set`"),
        "{descriptions:?}"
    );
}

#[test]
fn an_object_pointer_stored_into_a_function_pointer_member_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    // Recorded refusal: cio_set_log_callback's `void (*log_cb)` is an object
    // pointer, and its conversion to a function pointer is not C's.
    assert_not_safe(
        "struct ctx { struct { int (*log_cb)(struct ctx *, int); } options; }; \
         void subject(struct ctx *ctx, void (*log_cb)) { ctx->options.log_cb = log_cb; }",
    );
}
