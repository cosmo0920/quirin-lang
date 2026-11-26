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

//! Card MEMBER-PATH-INVOCATION-PREMISE (gateway rules 30 and 13v amended;
//! contract specification C04.19.6): a candidate's `#released` or `#moved`
//! member-path state reads `gone` under some or no arm only where the
//! invoking declaration's body, defined in the unit, calls the invoked
//! formal, member or table entry on every path to every normal exit. Each
//! form below (synchronous `c_op_call`, member `c_op_invoke`, table
//! `c_op_table`) is read over the same caller body with an invoker that
//! calls on every path (unchanged: `Safe`), and with invokers that may call
//! no candidate (an early return, a call under a condition, a loop that may
//! run zero times, a short-circuit, no body in the unit): the reading
//! refuses `member-path-invocation-unestablished` and the row is `Unknown`.
//! The fixture callees are trusted test assumptions; a `Safe` here promotes
//! nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const INVOCATION: &str = "member-path-invocation-unestablished";

/// Beside rule 30's `member_paths.qv`: a borrowing op constructor, the
/// three borrowing invokers, the op's plain drop (no arm), and the item's
/// release.
const INVOKERS: &str = "module contract;

extern fn c_op_new(type: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(type);
    ensures normal resource_return_owned(nonnull);
};

extern fn c_op_call(rko: ptr<mut, u8>, cb: fnptr) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rko);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, _, _, rko);
    ensures normal resource_return_none();
};

extern fn c_op_invoke(rko: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rko);
    ensures normal resource_callback_member_invoked(rko, op.op_cb, _, _, rko);
    ensures normal resource_return_none();
};

extern fn c_op_table(table: ptr<mut, u8>, rko: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(table);
    ensures normal resource_borrowed(rko);
    ensures normal resource_callback_table(table, _, _, rko);
    ensures normal resource_return_none();
};

extern fn c_op_drop(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_return_none();
};

extern fn c_item_free(it: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(it, nonnull);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "\
#include <stddef.h>
struct item { int x; };
struct op;
typedef int (*op_cb_t)(void *rk, void *rkq, struct op *rko);
struct op { int type; op_cb_t op_cb; struct item *payload; struct item *aux; };
struct entry { const char *name; op_cb_t handler; };
struct item *c_item_new(int n);
struct op *c_op_new(int type);
struct op *c_op_new_cb(int type, op_cb_t cb);
void c_op_destroy(struct op *rko);
int c_op_call(struct op *rko, op_cb_t cb);
int c_op_invoke(struct op *rko);
int c_op_table(struct entry *table, struct op *rko);
void c_op_drop(struct op *rko);
void c_item_free(struct item *it);
int c_cb_releases(void *rk, void *rkq, struct op *rko);
int c_cb_moves(void *rk, void *rkq, struct op *rko);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "member_paths.qv".to_owned(),
            include_str!("fixtures/contracts/member_paths.qv").to_owned(),
        ),
        ("invokers.qv".to_owned(), INVOKERS.to_owned()),
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
            eprintln!("skipping Clang-dependent invocation-premise test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe && self.unsupported.is_empty()
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status == AnalysisStatus::Unknown && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The row of `f` in the unit `PRELUDE + invoker + caller`.
fn row(invoker: &str, caller: &str) -> Row {
    let text = format!("{PRELUDE}\n{invoker}\n{caller}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "f",
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = &summary.functions["f"];
    Row {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// A fresh item linked at the op's payload, the dispatch, then the op's
/// drop, which releases no member: the payload is released only by a
/// candidate that runs.
const LINKED: &str = "struct item *it; struct op *o; it = c_item_new(n); \
                      if (!it) { return; }";

fn synchronous(cb: &str) -> String {
    format!(
        "void f(int n) {{ {LINKED} o = c_op_new(2); o->payload = it; c_op_call(o, {cb}); \
         c_op_drop(o); }}"
    )
}

fn member(cb: &str) -> String {
    format!(
        "void f(int n) {{ {LINKED} o = c_op_new_cb(2, {cb}); o->payload = it; c_op_invoke(o); \
         c_op_drop(o); }}"
    )
}

fn table(cb: &str) -> String {
    format!(
        "void f(int n) {{ {LINKED} o = c_op_new(2); o->payload = it; \
         struct entry table[] = {{ {{\"{cb}\", {cb}}}, {{NULL, NULL}} }}; \
         c_op_table(table, o); c_op_drop(o); }}"
    )
}

/// Invokers that call on every path.
const SYNC_EVERY: &str =
    "int c_op_call(struct op *rko, op_cb_t cb) { return cb(NULL, NULL, rko); }";
const MEMBER_EVERY: &str = "int c_op_invoke(struct op *rko) { int r; r = rko->op_cb(NULL, NULL, \
                            rko); return r; }";
const TABLE_EVERY: &str = "int c_op_table(struct entry *table, struct op *rko) { \
                           (*table[0].handler)(NULL, NULL, rko); return 0; }";

/// An invoker that calls on every path is read as before this card: the
/// candidate's `#released` or `#moved` payload is taken, so dropping the op
/// is `Safe`, and freeing the item again is never `Safe`.
#[test]
fn an_invoker_calling_on_every_path_reads_as_before() {
    if !clang_available() {
        return;
    }
    for cb in ["c_cb_releases", "c_cb_moves"] {
        for (form, invoker, caller) in [
            ("synchronous", SYNC_EVERY, synchronous(cb)),
            ("member", MEMBER_EVERY, member(cb)),
            ("table", TABLE_EVERY, table(cb)),
        ] {
            let taken = row(invoker, &caller);
            assert!(taken.safe(), "{form} {cb}: {taken:?}");
            let twice = row(
                invoker,
                &caller.replace(
                    "c_op_drop(o); }",
                    "o->payload = NULL; c_item_free(it); c_op_drop(o); }",
                ),
            );
            assert!(!twice.safe(), "{form} {cb}: {twice:?}");
        }
    }
}

/// Invokers that may call no candidate: each refuses the `gone` reading, and
/// the row is `Unknown` (it was `Safe` before this card).
#[test]
fn an_invoker_that_may_call_no_candidate_refuses_gone() {
    if !clang_available() {
        return;
    }
    let sync = [
        (
            "early return",
            "int c_op_call(struct op *rko, op_cb_t cb) { if (!rko->payload) { return 0; } \
             return cb(NULL, NULL, rko); }",
        ),
        (
            "under a condition",
            "int c_op_call(struct op *rko, op_cb_t cb) { int r = 0; if (rko->type == 1) { \
             r = cb(NULL, NULL, rko); } return r; }",
        ),
        (
            "a loop that may run zero times",
            "int c_op_call(struct op *rko, op_cb_t cb) { int r = 0; int i; \
             for (i = 0; i < rko->type; i++) { r = cb(NULL, NULL, rko); } return r; }",
        ),
        (
            "a conditional operator",
            "int c_op_call(struct op *rko, op_cb_t cb) { return rko->type ? \
             cb(NULL, NULL, rko) : 0; }",
        ),
        ("no body in the unit", ""),
    ];
    let member_forms = [
        (
            "early return",
            "int c_op_invoke(struct op *rko) { if (rko->type == 1) { return 0; } \
             return rko->op_cb(NULL, NULL, rko); }",
        ),
        (
            "a short-circuit",
            "int c_op_invoke(struct op *rko) { return rko->op_cb && \
             rko->op_cb(NULL, NULL, rko); }",
        ),
        (
            "a goto past the call",
            "int c_op_invoke(struct op *rko) { int r = 0; if (rko->type == 1) { goto out; } \
             r = rko->op_cb(NULL, NULL, rko); out: return r; }",
        ),
        ("no body in the unit", ""),
    ];
    let tables = [
        (
            "a loop that may run zero times",
            "int c_op_table(struct entry *table, struct op *rko) { int i; \
             for (i = 0; table[i].name; i++) { table[i].handler(NULL, NULL, rko); } return 0; }",
        ),
        (
            "a switch with a case that does not call",
            "int c_op_table(struct entry *table, struct op *rko) { switch (rko->type) { \
             case 1: break; default: table[0].handler(NULL, NULL, rko); break; } return 0; }",
        ),
        ("no body in the unit", ""),
    ];
    for cb in ["c_cb_releases", "c_cb_moves"] {
        for (shape, invoker) in sync {
            let refused = row(invoker, &synchronous(cb));
            assert!(
                refused.refused_with(INVOCATION),
                "synchronous {shape} {cb}: {refused:?}"
            );
        }
        for (shape, invoker) in member_forms {
            let refused = row(invoker, &member(cb));
            assert!(
                refused.refused_with(INVOCATION),
                "member {shape} {cb}: {refused:?}"
            );
        }
        for (shape, invoker) in tables {
            let refused = row(invoker, &table(cb));
            assert!(
                refused.refused_with(INVOCATION),
                "table {shape} {cb}: {refused:?}"
            );
        }
    }
}

/// A call that hands another object, or a formal the body rebinds, is not
/// the composed invocation.
#[test]
fn a_call_of_another_object_or_a_rebound_formal_establishes_nothing() {
    if !clang_available() {
        return;
    }
    for invoker in [
        "int c_op_call(struct op *rko, op_cb_t cb) { return cb(NULL, NULL, NULL); }",
        "int c_op_call(struct op *rko, op_cb_t cb) { cb = c_cb_moves; \
         return cb(NULL, NULL, rko); }",
        "int c_op_call(struct op *rko, op_cb_t cb) { struct op *other = rko; \
         rko = NULL; return cb(NULL, NULL, other); }",
    ] {
        // Rule 13y already withdraws these synchronous declarations.
        let refused = row(invoker, &synchronous("c_cb_releases"));
        assert!(!refused.safe(), "{invoker}: {refused:?}");
    }
    // No body check withdraws a member invocation: this card's check reads
    // them.
    for invoker in [
        "int c_op_invoke(struct op *rko) { return rko->op_cb(NULL, NULL, NULL); }",
        "int c_op_invoke(struct op *rko) { struct op *other = rko; rko = NULL; \
         return other->op_cb(NULL, NULL, other); }",
        "int c_op_invoke(struct op *rko) { struct op *other = c_op_new(2); \
         return other->op_cb(NULL, NULL, rko); }",
    ] {
        let refused = row(invoker, &member("c_cb_releases"));
        assert!(refused.refused_with(INVOCATION), "{invoker}: {refused:?}");
    }
}

/// Where every arm releases the path, the reading does not rest on the
/// invocation: `{#released, #kept}` under such an arm is `gone` either way
/// (rule 30's type-1 arm of `c_op_destroy`), so a bodiless invoker still
/// reads it. Under no arm (type 2) it refuses.
#[test]
fn an_arm_releasing_the_path_needs_no_invocation() {
    if !clang_available() {
        return;
    }
    let caller = |type_: u32| {
        format!(
            "void f(int n) {{ {LINKED} o = c_op_new_cb({type_}, c_cb_releases); \
             o->payload = it; c_op_destroy(o); }}"
        )
    };
    let every = row("", &caller(1));
    assert!(every.safe(), "{every:?}");
    let none = row("", &caller(2));
    assert!(none.refused_with(INVOCATION), "{none:?}");
}
