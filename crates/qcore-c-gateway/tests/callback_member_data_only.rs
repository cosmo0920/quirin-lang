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

//! Card CALLBACK-MEMBER-DATA-ONLY-UNITS (gateway rule 12n): a unit's member
//! facts read from its whole Clang AST (`callback_member_closure::read_unit`),
//! the reader a data-only unit is read with: an initializer list mapped onto
//! its record's fields stores each function-pointer entry, a list that cannot
//! be mapped exactly opens the record as before, a unit with no store closes,
//! and a unit whose AST is not read is an error (no facts).

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::callback_member_closure::{Closure, MemberFacts, closure, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent data-only member test: {reason}");
            false
        }
    }
}

const RECORD: &str = "\
typedef int cb_t(void *);
int handler(void *); int other(void *);
struct op { int kind; cb_t *cb; int count; int : 3; int tail; };
";

fn facts(source: &str) -> MemberFacts {
    let text = format!("{RECORD}{source}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn of(source: &str, member: &str) -> Closure {
    closure(&[&facts(source)], member)
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_designated_or_positional_initializer_stores_its_candidate() {
    if !clang_available() {
        return;
    }
    // Designated, positional (past an unnamed bit-field) and implicit
    // entries; a file-scope and a block-scope list.
    let source = "\
const struct op a = { .cb = handler };
struct op b = { 1, other, 2, 3 };
struct op c = { 1 };
void f(void) { struct op d = { .kind = 2, .cb = handler }; (void)d; }
";
    assert_eq!(of(source, "op.cb"), Ok(names(&["handler", "other"])));
}

#[test]
fn a_unit_with_no_store_closes_and_a_null_entry_adds_nothing() {
    if !clang_available() {
        return;
    }
    assert_eq!(of("int x = 1;\n", "op.cb"), Ok(names(&[])));
    assert_eq!(of("struct op z = { 0, 0 };\n", "op.cb"), Ok(names(&[])));
    // The empty file of the measured `ctr_utils.c`.
    let empty = read_unit(
        "",
        &C11TranslationUnit {
            source: "",
            ..Default::default()
        },
    )
    .expect("an empty unit is read");
    assert_eq!(closure(&[&empty], "op.cb"), Ok(names(&[])));
}

#[test]
fn a_union_designator_maps_onto_its_field() {
    if !clang_available() {
        return;
    }
    let source = "union u { int x; cb_t *cb; };\nunion u v = { .cb = other };\n";
    assert_eq!(of(source, "u.cb"), Ok(names(&["other"])));
}

#[test]
fn a_list_that_cannot_be_mapped_opens_the_record() {
    if !clang_available() {
        return;
    }
    // A nested record entry carrying a callback opens the outer record; the
    // nested list is mapped onto its own record.
    let nested = "struct outer { struct op inner; cb_t *g; };\n\
                  struct outer o = { .inner.cb = other, .g = handler };\n";
    assert!(of(nested, "outer.g").is_err());
    assert_eq!(of(nested, "op.cb"), Ok(names(&["other"])));
    // An anonymous member: the fields are not mapped.
    let anonymous = "struct anon { union { cb_t *cb; int n; }; cb_t *h; };\n\
                     struct anon a = { .h = handler };\n";
    assert!(of(anonymous, "anon.h").is_err());
    // A copy of a record value inside the list may carry a callback.
    let copied = "struct holder { struct op inner; cb_t *g; };\n\
                  void f(struct op proto) {\n\
                  struct holder h = { proto, handler }; (void)h; }\n";
    assert!(of(copied, "holder.g").is_err());
    // A record defined inside a function may reuse the tag.
    let shadowed = "void f(void) { struct op { cb_t *cb; } l = { handler }; (void)l; }\n\
                    struct op top = { .cb = other };\n";
    assert!(of(shadowed, "op.cb").is_err());
}

#[test]
fn an_unread_unit_is_an_error() {
    if !clang_available() {
        return;
    }
    let text = "struct op { int (*cb)(void *); };\nthis is not C;\n";
    assert!(
        read_unit(
            text,
            &C11TranslationUnit {
                source: text,
                ..Default::default()
            },
        )
        .is_err()
    );
}

/// In an analysed unit the in-unit decision reads the initializer's
/// candidate too: an effectful candidate stored only by a designated
/// initializer refuses the member invocation (it is not ignored).
#[test]
fn an_analysed_unit_decides_over_its_initializer_candidates() {
    if !clang_available() {
        return;
    }
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "op_callbacks.qv".to_owned(),
        include_str!("fixtures/contracts/op_callbacks.qv").to_owned(),
    )]))
    .expect("the fixture contracts load");
    let text = "\
#include <stddef.h>
struct op { int type; int (*op_cb)(void *rk, void *rkq, struct op *rko); void *payload; };
void *c_alloc(unsigned long n); void c_free(void *p);
void c_op_destroy(struct op *rko);
static int freeing(void *rk, void *rkq, struct op *rko) { c_free(rk); return 0; }
static int quiet(void *rk, void *rkq, struct op *rko) { return 0; }
const struct op template_op = { .op_cb = freeing };
void set_quiet(struct op *o) { o->op_cb = quiet; }
void drop(void) { struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }
";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: text,
            ..Default::default()
        },
        "drop",
        &contracts,
    )
    .expect("analysed");
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let row = &summary.functions["drop"];
    assert_ne!(row.status, AnalysisStatus::Safe, "{row:?}");
    assert!(
        row.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description }
                if description.contains("callback-effectful")
        )),
        "{row:?}"
    );
}
