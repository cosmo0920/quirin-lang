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

//! Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3 (gateway rule 67).
//!
//! * 67: a scalar statement whose value holds several sibling calls (none
//!   inside another's actuals), each to a callee whose contract has a `None`
//!   result and every position `Borrowed`, is each call alone (with rule
//!   W44's reads), in source order, then the statement's typed reads. It
//!   lifts rule 14a's one-call limit.
//! * 68: a record member declared through a pointer typedef every
//!   declaration of which is `typedef SPEC * A;` is the single pointer member
//!   `SPEC * NAME` (AG1-POINTER-TYPEDEF-MEMBER).
//! * 12h (amended, round 3): an identity-preserving pointer view of a
//!   member-chain place at a call actual is removed by hook H-U like a
//!   view of a plain name, and the uncast statement is lowered as before.
//!
//! Every admission has a control that keeps the gate's refusal (or another
//! refusal) and never `Safe`. The fixture contracts are the CG00 harness's
//! `q_*` assumptions, `indirect_sole.qv` and `residual_r2.qv`: trusted test
//! assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
int is_cmp(const char *a, const char *b); char *is_dup(const char *p);
int is_count(const char *p); int is_unknown(const char *p);
__INT64_TYPE__ rr_load(__INT64_TYPE__ *p); int rr_release_count(void *p);
struct S { int a; char *name; struct S *next; __INT64_TYPE__ *counts; __INT64_TYPE__ arr[4]; char *names[4]; };
typedef char *ps_t; typedef void *vh_t;
struct P { ps_t name; vh_t handle; ps_t *names; int n; };
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "indirect_sole.qcore",
            include_str!("fixtures/contracts/indirect_sole.qv"),
        ),
        file(
            "residual_r2.qcore",
            include_str!("fixtures/contracts/residual_r2.qv"),
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
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    descriptions: Vec<String>,
}

fn run(source: &str) -> Run {
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
    Run {
        status: result.status,
        descriptions: result
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

fn assert_safe(source: &str) {
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

/// The function is not `Safe` and keeps the gate's refusal.
fn assert_keeps_the_gate(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
    assert!(
        run.descriptions
            .iter()
            .any(|description| description == INDIRECT),
        "{source}\n{run:?}"
    );
}

/// The function is not `Safe`.
fn assert_not_safe(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

#[test]
fn rule_67_reads_several_sibling_borrowed_calls() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "void subject(struct S *a) { a->a = is_count(a->name) + is_count(a->next->name); }",
    );
    assert_safe(
        "int subject(struct S *a, struct S *b) { int n = is_count(a->name) + is_count(b->name) + 2; return n; }",
    );
    assert_safe(
        "__INT64_TYPE__ subject(struct S *a, int i) { __INT64_TYPE__ v = rr_load(&a->arr[i]) - rr_load(&a->counts[i]) + is_cmp(a->name, a->next->name); return v; }",
    );
}

#[test]
fn rule_67_keeps_the_refusal_when_any_call_takes_custody_or_nests() {
    if !clang_available() {
        return;
    }
    // A releasing sibling: the order C leaves open would decide the other
    // operand's read differently.
    assert_not_safe(
        "void subject(struct S *a) { a->a = is_count(a->name) + rr_release_count(a->name); }",
    );
    // An owned result beside a borrowed call.
    assert_not_safe(
        "void subject(struct S *a) { a->a = is_count(a->name) + (is_dup(a->name) != 0); }",
    );
    // An uncontracted sibling.
    assert_not_safe(
        "void subject(struct S *a) { a->a = is_count(a->name) + is_unknown(a->next->name); }",
    );
    // A nested call (one call's value is another's actual) is not read here.
    assert_not_safe(
        "__INT64_TYPE__ subject(struct S *a) { __INT64_TYPE__ v = rr_load(&a->arr[is_count(a->name)]); return v; }",
    );
}

#[test]
fn rule_68_reads_a_member_through_a_pointer_typedef() {
    if !clang_available() {
        return;
    }
    // `ps_t name;` is `char *name;`: an owned result stored into it on a
    // record this body allocated is O04-B08's field link, as for the `char *`
    // spelling.
    assert_safe(
        "struct P *subject(const char *s) { struct P *p = q_alloc(); if (p == 0) return 0; p->n = 0; p->name = is_dup(s); return p; }",
    );
}

#[test]
fn rule_68_keeps_the_refusal_for_other_typedef_members() {
    if !clang_available() {
        return;
    }
    // A pointer to the alias is a pointer to a pointer: no member class.
    assert_not_safe(
        "struct P *subject(const char *s) { struct P *p = q_alloc(); if (p == 0) return 0; p->n = 0; *p->names = is_dup(s); return p; }",
    );
    // A `void *` typedef names no pointee: no single-pointer member.
    assert_keeps_the_gate(
        "struct P *subject(const char *s) { struct P *p = q_alloc(); if (p == 0) return 0; p->n = 0; p->handle = is_dup(s); return p; }",
    );
}

#[test]
fn rule_12h_round_3_reads_a_pointer_view_of_a_member_at_an_actual() {
    if !clang_available() {
        return;
    }
    assert_safe("int subject(struct S *a) { int n = is_count((const char *)a->name); return n; }");
    assert_safe(
        "void subject(struct S *a) { int n; n = is_count((const char *)a->name); (void)n; }",
    );
    assert_safe(
        "int subject(struct S *a, struct S *b) { int n = is_cmp((const char *)a->name, (char *)b->name); return n; }",
    );
}

#[test]
fn rule_12h_round_3_keeps_every_other_cast_refused() {
    if !clang_available() {
        return;
    }
    // A cast from an integer: no place's identity.
    assert_keeps_the_gate(
        "int subject(struct S *a) { int n = is_count((const char *)(long)a->a); return n; }",
    );
    // A cast of a decayed array member is not a member-chain place view.
    assert_keeps_the_gate(
        "typedef unsigned int u32_t; struct H { u32_t tab[8]; int n; }; __INT64_TYPE__ subject(struct H *h) { __INT64_TYPE__ v = rr_load((__INT64_TYPE__ *)h->tab); return v; }",
    );
    // A cast of an offset stays rule 14d's refusal.
    assert_keeps_the_gate("int subject(struct S *a) { return is_count((char *)a->next + a->a); }");
    // A typed member destination is rule 14a's Clang-matched reading, which
    // the uncast spelling (no Clang node) never reaches: kept.
    assert_keeps_the_gate("void subject(struct S *a) { a->a = is_count((const char *)a->name); }");
    // The return position has no H-U placement (a route-map row): kept.
    assert_keeps_the_gate("int subject(struct S *a) { return is_count((const char *)a->name); }");
    // A releasing position: the view hands the callee the same identity the
    // uncast place would, which no reading here discharges.
    assert_not_safe("void subject(struct S *a) { q_free((void *)a->name); }");
}
