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

//! Card CG10-SMALL-BUNDLE (gateway rules 74, 74a and 74b).
//!
//! * 74: a whole-call return whose uncast call (rule 71a) meets an
//!   uncontracted callee names that callee (rule 12k), never a verdict move
//!   (CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION).
//! * 74a: a formal stored into a member of the body's own tracked result is
//!   **not** derived as `resource_retained_by(p, result)`: the recorded
//!   refusal (CG10-RESULT-HOLDER-RETAINED-BY).
//! * 74b: a formal declared `R (*f)(A)` that the body never calls is read by
//!   the W4 lexical parameter reader as a function-pointer formal
//!   (CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS).
//!
//! The fixture contracts are trusted test assumptions, not library facts.
//! Every fixture is target-portable C (no `long`, no host header).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const INDIRECT: &str = "indirect memory access is outside the resource projection";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
int rg_peek(void *p);
int un_peek(const unsigned char *p, void *q);
int un_bool(void *stream, _Bool *dest);
int un_bytes(void *stream, const unsigned char *p, __SIZE_TYPE__ n);
struct S { int a; char *name; void *data; __SIZE_TYPE__ size; struct S *next; };
typedef union U { int i; char c[4]; } U_t;
";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
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
            eprintln!("skipping Clang-dependent CG10-SMALL-BUNDLE test: {reason}");
            false
        }
    }
}

fn run_function(source: &str, function: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        function,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get(function))
        .unwrap_or_else(|| panic!("`{function}` projected: {outcome:?}"));
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

fn run(source: &str) -> (AnalysisStatus, Vec<String>) {
    run_function(source, "subject")
}

fn assert_safe(source: &str) {
    let (status, descriptions) = run(source);
    assert_eq!(status, AnalysisStatus::Safe, "{source}\n{descriptions:?}");
}

fn assert_unknown_with(source: &str, wanted: &str) -> Vec<String> {
    let (status, descriptions) = run(source);
    assert_eq!(
        status,
        AnalysisStatus::Unknown,
        "{source}\n{descriptions:?}"
    );
    assert!(
        descriptions.iter().any(|d| d == wanted),
        "{source}\nwanted {wanted:?} in {descriptions:?}"
    );
    descriptions
}

// ---- rule 74 ----------------------------------------------------------

#[test]
fn rule_74_names_the_uncontracted_callee_of_an_uncast_return() {
    if !clang_available() {
        return;
    }
    // print_string's shape: a character view of a pointer member, handed to
    // an uncontracted callee.
    let descriptions = assert_unknown_with(
        "int subject(struct S *item, void *p) \
         { return un_peek((const unsigned char *)item->name, p); }",
        "call to uncontracted function `un_peek`",
    );
    assert!(
        !descriptions.iter().any(|d| d == INDIRECT),
        "{descriptions:?}"
    );
    // pb_dec_bool's shape: a scalar view.
    assert_unknown_with(
        "int subject(void *stream, struct S *field) \
         { return un_bool(stream, (_Bool *)field->data); }",
        "call to uncontracted function `un_bool`",
    );
}

#[test]
fn rule_74_keeps_the_gate_where_rule_71a_keeps_the_view() {
    if !clang_available() {
        return;
    }
    // A view to a union typedef is rule 10w's: no uncast call exists.
    assert_unknown_with(
        "int subject(struct S *item, void *p) { return un_peek((U_t *)item->data, p); }",
        INDIRECT,
    );
    // A return inside an expression is not a whole call.
    let (status, _) = run("int subject(struct S *item, void *p) \
         { return 1 + un_peek((const unsigned char *)item->name, p); }");
    assert_eq!(status, AnalysisStatus::Unknown);
    // A contracted callee is rule 71a's admission, unchanged.
    assert_safe("int subject(struct S *a) { return rg_peek((const char *)a->name); }");
}

#[test]
fn rule_74_is_never_a_verdict_move() {
    if !clang_available() {
        return;
    }
    // pb_enc_fixed_length_bytes' shape with a scalar conversion of a member
    // beside the view: whatever the leaf, the row stays Unknown.
    let (status, descriptions) = run("int subject(void *stream, struct S *field) \
         { return un_bytes(stream, (const unsigned char *)field->data, (__SIZE_TYPE__)field->size); }");
    assert_eq!(status, AnalysisStatus::Unknown, "{descriptions:?}");
}

// ---- rule 74a (recorded refusal) --------------------------------------

const RESULT_HOLDER: &str = "\
struct pmi { void *member; int match; };
struct pmi *make(void *member, int match)
{ struct pmi *p = q_alloc(); p->member = member; p->match = match; return p; }
";

#[test]
fn rule_74a_keeps_a_formal_stored_into_the_result_refused() {
    if !clang_available() {
        return;
    }
    // PartitionMemberInfo_new's shape: the store into the body's own tracked
    // result keeps rule 10v's refusal, and no summary is published.
    let (status, descriptions) = run_function(RESULT_HOLDER, "make");
    assert_eq!(status, AnalysisStatus::Unknown, "{descriptions:?}");
    // Every caller sees an uncontracted callee, whatever it does after.
    assert_unknown_with(
        &format!(
            "{RESULT_HOLDER} void subject(void *m) {{ struct pmi *x = make(m, 1); q_free(x); }}"
        ),
        "call to uncontracted function `make`",
    );
    // The hazard the refusal names: a destructor that releases the member
    // holding the caller's object. A record ended by this release would let
    // the caller release `m` again unseen.
    assert_unknown_with(
        &format!(
            "{RESULT_HOLDER} void pmi_free(struct pmi *p) {{ q_free(p->member); q_free(p); }} \
             void subject(void) {{ void *m = q_alloc(); struct pmi *x = make(m, 1); \
             pmi_free(x); q_free(m); }}"
        ),
        "call to uncontracted function `make`",
    );
}

// ---- rule 74b ---------------------------------------------------------

const HOLDER: &str = "\
struct h { void *o; void (*free_cb)(void *); int (*cmp)(const void *, const void *); };
void my_free(void *p);
";

#[test]
fn rule_74b_reads_a_function_pointer_formal_the_body_never_calls() {
    if !clang_available() {
        return;
    }
    // rd_kafka_buf_set_maker's shape: a retention beside a function pointer
    // stored into the same object; the caller's static actuals discharge it.
    let set = "void set(struct h *h, void *o, void (*free_cb)(void *)) \
               { h->o = o; h->free_cb = free_cb; }";
    assert_safe(&format!(
        "{HOLDER} {set} void subject(struct h *h) {{ set(h, 0, 0); }}"
    ));
    assert_safe(&format!(
        "{HOLDER} {set} void subject(struct h *h) {{ set(h, 0, my_free); }}"
    ));
    // The caller's own object is still a retention it must discharge (rule
    // 12t through rule 70: the summary is published, the record stands).
    assert_unknown_with(
        &format!("{HOLDER} {set} void subject(struct h *h, void *o) {{ set(h, o, my_free); }}"),
        "a holder retaining `o` may outlive this function's frame (retention-undecided)",
    );
    // A neutral body with a comparator formal: the caller's object is
    // borrowed, and the caller releases it once.
    assert_safe(&format!(
        "{HOLDER} int count(const void *a, int (*cmp)(const void *, const void *), int n) \
         {{ return a != 0 && n > 0; }} \
         int subject(void) {{ void *t = q_alloc(); int r = count(t, 0, 1); q_free(t); return r; }}"
    ));
    // A releasing body: the summary states the release.
    assert_safe(&format!(
        "{HOLDER} void drop(void *p, void (*cb)(void *)) {{ q_free(p); }} \
         void subject(void) {{ void *t = q_alloc(); drop(t, my_free); }}"
    ));
    let (status, _) = run(&format!(
        "{HOLDER} void drop(void *p, void (*cb)(void *)) {{ q_free(p); }} \
         void subject(void) {{ void *t = q_alloc(); drop(t, my_free); q_free(t); }}"
    ));
    assert_ne!(status, AnalysisStatus::Safe);
}

#[test]
fn rule_74b_keeps_a_called_function_pointer_formal_refused() {
    if !clang_available() {
        return;
    }
    // An invocation of the caller's function may release what it is handed.
    assert_unknown_with(
        &format!(
            "{HOLDER} void apply(void *p, void (*free_cb)(void *)) {{ free_cb(p); }} \
             void subject(void) {{ void *t = q_alloc(); apply(t, my_free); q_free(t); }}"
        ),
        "call to uncontracted function `apply`",
    );
    // Either call spelling.
    assert_unknown_with(
        &format!(
            "{HOLDER} void apply(void *p, void (*free_cb)(void *)) {{ (*free_cb)(p); }} \
             void subject(void) {{ void *t = q_alloc(); apply(t, my_free); q_free(t); }}"
        ),
        "call to uncontracted function `apply`",
    );
    // A formal named like a contracted function is never read as it.
    assert_unknown_with(
        &format!(
            "{HOLDER} void apply(void *p, void (*q_free)(void *)) {{ q_free(p); }} \
             void subject(void) {{ void *t = q_alloc(); apply(t, my_free); q_free(t); }}"
        ),
        "call to uncontracted function `apply`",
    );
}

#[test]
fn rule_74b_keeps_every_other_parenthesised_formal_refused() {
    if !clang_available() {
        return;
    }
    // `int (cmp)(…)`: a parenthesised function declarator, adjusted to a
    // pointer (C11 6.7.6.3p8), not the shape read.
    assert_unknown_with(
        &format!(
            "{HOLDER} int count(const void *a, int (cmp)(const void *, const void *)) \
             {{ return a != 0; }} \
             int subject(void) {{ void *t = q_alloc(); int r = count(t, 0); q_free(t); return r; }}"
        ),
        "call to uncontracted function `count`",
    );
    // A variadic list stays the old reader's refusal.
    assert_unknown_with(
        &format!(
            "{HOLDER} int count(const void *a, int (*cmp)(const void *, const void *), ...) \
             {{ return a != 0; }} \
             int subject(void) {{ void *t = q_alloc(); int r = count(t, 0); q_free(t); return r; }}"
        ),
        "call to uncontracted function `count`",
    );
}
