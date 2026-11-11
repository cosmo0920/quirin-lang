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

//! Card NULL-TRIAGE-UNRESOLVED-PATHS (gateway rules 15q-15u): the controls of
//! the path search's exact and inexact readings, each on a real Clang AST.
//!
//! The fixture contracts are trusted test assumptions, not library facts:
//! `nd_xalloc` has an owned non-null result; `nd_count` and `nd_ucount` are
//! uncontracted integer results unless a fixture declares a range.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, NullUnitReport, analyze_null_c11};
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct S { int a; struct S *next; };
void *nd_xalloc(unsigned long n); void nd_use(void *p);
int nd_count(void); unsigned long nd_ucount(void);
";

const RESOURCE: &str = "module contract;
extern fn nd_xalloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nonnull);
};
";

/// A declared result range of `nd_count`.
const RANGE: &str = "module contract;
extern fn nd_count() -> i32
contract {
    requires bounds_contract_v1;
    ensures normal nonnull_result_range(0, 2147483647);
};
";

fn resource() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("nd.qv".to_owned(), RESOURCE.to_owned())]))
        .expect("the fixture resource contracts load")
}

fn bounds(text: Option<&str>) -> BoundsContracts {
    match text {
        Some(text) => load_bounds_contract_sources(&BTreeMap::from([(
            "nd_bounds.qv".to_owned(),
            text.to_owned(),
        )]))
        .expect("the fixture bounds contracts load"),
        None => BoundsContracts::default(),
    }
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

fn report(source: &str, functions: &[&str], clauses: Option<&str>) -> NullUnitReport {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let functions = functions
        .iter()
        .map(|f| (*f).to_owned())
        .collect::<Vec<_>>();
    analyze_null_c11(&unit, &functions, &resource(), &bounds(clauses))
        .unwrap_or_else(|error| panic!("fixture rejected: {error}\n{text}"))
}

fn subject_with(source: &str, clauses: Option<&str>) -> NullFunctionRow {
    report(source, &["subject"], clauses).functions.remove(0)
}

fn subject(source: &str) -> NullFunctionRow {
    subject_with(source, None)
}

fn statuses(row: &NullFunctionRow) -> Vec<(&str, Option<&str>)> {
    row.triage.iter().map(|t| (t.status, t.class)).collect()
}

/// `copy_label_values`' shape: the NULL path needs the count to be negative.
const COUNT: &str = "int subject(void) { struct S *p = 0; int s = nd_count(); \
     if (s == 0) return 0; if (s > 0) p = nd_xalloc(sizeof *p); p->a = 1; return 1; }";

#[test]
fn a_call_result_is_never_free() {
    if !clang_available() {
        return;
    }
    // Rule 15q: before the card the search read `s` as any int and validated
    // `s < 0`, a value the callee may never return. Now the path is inexact.
    let row = subject(COUNT);
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    assert!(
        row.triage[0]
            .reason
            .starts_with("an inexact path reaches the dereference NULL"),
        "{row:#?}"
    );
    assert!(row.triage[0].reason.contains("nd_count"), "{row:#?}");
    assert_eq!(row.verdict, "Unknown");
}

#[test]
fn a_declared_result_range_bounds_the_result_and_removes_the_candidate() {
    if !clang_available() {
        return;
    }
    // Rules 15q, 15t: with `nd_count` in [0, INT_MAX] the `s < 0` arm is
    // excluded, every path is explored and none reaches `p` NULL.
    let row = subject_with(COUNT, Some(RANGE));
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    let check = &row.checks[0];
    assert_eq!(check.outcome, "proved", "{row:#?}");
    assert!(check.proofs.contains(&"path-search:infeasible".to_owned()));
    assert!(
        check
            .proofs
            .contains(&"result:declared-range:nd_count".to_owned()),
        "{row:#?}"
    );
    assert!(
        row.contract_dependencies
            .contains(&"range:nd_count".to_owned())
    );
    assert!(
        row.assumptions
            .iter()
            .any(|a| a.starts_with("path-search:"))
    );
}

#[test]
fn a_refuting_body_takes_the_range_out_of_force() {
    if !clang_available() {
        return;
    }
    let unit = report(
        &format!("int nd_count(void) {{ return -1; }}\n{COUNT}"),
        &["subject"],
        Some(RANGE),
    );
    // The declaration is out of force here; this unit's own summary of the
    // body (it returns -1 only) decides both tests, so the NULL path is a
    // real one.
    let row = &unit.functions[0];
    assert_eq!(
        statuses(row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    assert!(
        !row.contract_dependencies
            .contains(&"range:nd_count".to_owned())
    );
    let export = unit
        .definitions
        .iter()
        .find(|d| d.function == "nd_count")
        .expect("the definition's export");
    assert_eq!(export.result_range, Some("refuted"));
}

#[test]
fn a_validating_body_and_an_in_unit_summary_bound_the_result() {
    if !clang_available() {
        return;
    }
    let counter = "static int count_up(struct S *s) { int r = 0; if (!s) return 0; \
         while (s) { r++; s = s->next; } return r; }";
    // The in-unit summary alone (no declaration) bounds `count_up`'s result.
    let row = subject(&format!(
        "{counter}\nint subject(struct S *l) {{ struct S *p = 0; int s = count_up(l); \
         if (s == 0) return 0; if (s > 0) p = nd_xalloc(sizeof *p); p->a = 1; return 1; }}"
    ));
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
    // A defined `nd_count` whose body validates the declaration.
    let unit = report(
        &format!("int nd_count(void) {{ int n = 0; n += 2; return n; }}\n{COUNT}"),
        &["subject"],
        Some(RANGE),
    );
    let export = unit
        .definitions
        .iter()
        .find(|d| d.function == "nd_count")
        .expect("the definition's export");
    assert_eq!(export.result_range, Some("validated"));
    assert_eq!(
        statuses(&unit.functions[0]),
        vec![("infeasible", None)],
        "{unit:#?}"
    );
}

#[test]
fn an_unsigned_result_is_never_negative_from_its_type() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(void) { struct S *p = 0; unsigned long s = nd_ucount(); \
         if (s == 0) return 0; if (s > 0) p = nd_xalloc(sizeof *p); p->a = 1; return 1; }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
}

#[test]
fn a_relation_takes_an_arm_only_when_the_intervals_admit_it() {
    if !clang_available() {
        return;
    }
    // Rule 15s: `a > b` with `a` in [0, 255] and `b` above 1000 is never
    // true (before the card both arms were taken whatever the ranges).
    let row = subject(
        "void subject(unsigned char a, int b, int c) { struct S *p = 0; \
         if (c) p = nd_xalloc(sizeof *p); if (b > 1000) { if (a > b) p->a = 1; } }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
    // Two free formals: both arms are real.
    let row = subject(
        "void subject(int a, int b, int c) { struct S *p = 0; \
         if (c) p = nd_xalloc(sizeof *p); if (a < b) p->a = 1; }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
}

#[test]
fn an_offset_of_a_formal_is_followed_exactly() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "void subject(int i, int c) { struct S *p = 0; if (c) p = nd_xalloc(sizeof *p); \
         if (i >= 0 && i < 10) { if (i + 2 < 5) p->a = 1; } }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    let row = subject(
        "void subject(int i, int c) { struct S *p = 0; if (c) p = nd_xalloc(sizeof *p); \
         if (i >= 0 && i < 10) { if (i + 2 > 20) p->a = 1; } }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
}

#[test]
fn a_variable_assignment_forgets_the_memory_it_spells() {
    if !clang_available() {
        return;
    }
    // Rule 15u: `s->a` after `s = t` is another object's member (before the
    // card the cached `1` decided the test and hid the path).
    let row = subject(
        "int subject(struct S *s, struct S *t, int c) { struct S *p = 0; \
         if (c) p = nd_xalloc(sizeof *p); s->a = 1; s = t; if (s->a != 1) p->a = 1; return 0; }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
}

#[test]
fn an_inexact_condition_is_explored_for_infeasibility() {
    if !clang_available() {
        return;
    }
    // Rule 15r: an ordered pointer comparison is read inexactly; the flag
    // decides the dereference on both arms.
    let row = subject(
        "void subject(char *a, char *b) { int ok = 0; struct S *p = 0; \
         if (a < b) { p = nd_xalloc(sizeof *p); ok = 1; } if (ok) p->a = 1; }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
    // A memory value re-read after a call: both arms explored, the NULL
    // path is inexact, so the candidate stays unresolved.
    let row = subject(
        "int subject(struct S *s) { struct S *p = 0; if (s->a == 1) p = nd_xalloc(sizeof *p); \
         nd_use(0); if (s->a == 1) p->a = 1; return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    assert!(
        row.triage[0].reason.contains("a correlated condition"),
        "{row:#?}"
    );
    // A switch over a floating value's conversion: each case explored.
    let row = subject(
        "void subject(double d) { int ok = 0; struct S *p = 0; \
         switch ((int)d) { case 1: p = nd_xalloc(sizeof *p); ok = 1; break; default: break; } \
         if (ok) p->a = 1; }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
}

#[test]
fn the_loader_reads_the_range_clause() {
    let loaded = bounds(Some(RANGE));
    assert!(
        loaded
            .null_declarations()
            .any(|(name, _)| name == "nd_count")
    );
    // A pointer result, an empty range and a malformed bound are refused.
    for text in [
        "module contract;\nextern fn f() -> ptr<mut, u8>\ncontract {\n    requires bounds_contract_v1;\n    ensures normal nonnull_result_range(0, 1);\n};\n",
        "module contract;\nextern fn f() -> i32\ncontract {\n    requires bounds_contract_v1;\n    ensures normal nonnull_result_range(2, 1);\n};\n",
        "module contract;\nextern fn f() -> i32\ncontract {\n    requires bounds_contract_v1;\n    ensures normal nonnull_result_range(01, 2);\n};\n",
        // USER DECISION 2026-10-11: the element is `nonnull_result_range`;
        // the earlier spelling is no alias.
        "module contract;\nextern fn f() -> i32\ncontract {\n    requires bounds_contract_v1;\n    ensures normal null_result_range(0, 1);\n};\n",
    ] {
        assert!(
            load_bounds_contract_sources(&BTreeMap::from([("x.qv".to_owned(), text.to_owned())]))
                .is_err(),
            "{text}"
        );
    }
    let negative = "module contract;\nextern fn f() -> i32\ncontract {\n    requires bounds_contract_v1;\n    ensures normal nonnull_result_range(-1, 5);\n};\n";
    let loaded =
        load_bounds_contract_sources(&BTreeMap::from([("x.qv".to_owned(), negative.to_owned())]))
            .expect("a negative bound loads");
    assert!(loaded.null_declarations().any(|(name, _)| name == "f"));
    // The bounds consumer never reads it.
    assert!(loaded.for_bounds().null_declarations().next().is_none());
}
