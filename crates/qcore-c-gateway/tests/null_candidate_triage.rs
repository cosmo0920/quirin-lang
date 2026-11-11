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

//! Card NULL-CANDIDATE-TRIAGE (gateway rules 14r-14v): the controls of the
//! candidates' path search, of declared non-null members and results, and of
//! what each unit exports for cross-unit travel, each on a real Clang AST.
//!
//! The fixture contracts are trusted test assumptions, not library facts:
//! `nd_alloc` has an owned nullable result (an allocation), `nd_xalloc` an
//! owned non-null one, `nd_find` a nullable interior result (a lookup).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, NullUnitReport, analyze_null_c11};
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct S { int a; struct S *next; int arr[4]; };
void *nd_alloc(unsigned long n); void *nd_xalloc(unsigned long n);
char *nd_find(const char *s, int c); void nd_use(void *p); struct S *nd_get(int k);
int nd_flag(void);
";

const RESOURCE: &str = "module contract;
extern fn nd_alloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};
extern fn nd_xalloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nonnull);
};
extern fn nd_find(s: ptr<const, u8>, c: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(c);
    ensures normal resource_return_interior(s, nullable);
};
";

/// The member declaration (hosted on `nd_get`) and a declared non-null
/// result (`nd_get`).
const MEMBER: &str = "module contract;
extern fn nd_get(k: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member(S, next);
    ensures normal nonnull_result();
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

fn subject(source: &str) -> NullFunctionRow {
    report(source, &["subject"], None).functions.remove(0)
}

fn statuses(row: &NullFunctionRow) -> Vec<(&str, Option<&str>)> {
    row.triage.iter().map(|t| (t.status, t.class)).collect()
}

#[test]
fn an_unchecked_allocation_is_a_finding_on_allocation_failure() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(int c) { if (c > 3) return 0; struct S *p = nd_alloc(sizeof *p); p->a = 1; return p->a; }",
    );
    assert_eq!(row.verdict, "Unsafe", "{row:#?}");
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-allocation-failure"))],
        "{row:#?}"
    );
    assert_eq!(row.checks[0].outcome, "finding");
    let witness = row.triage[0].witness.as_ref().expect("a witness");
    assert!(witness.condition.contains("nd_alloc"), "{witness:#?}");
    // The path names its decision (the early return's arm) and its event.
    assert!(
        witness.path.iter().any(|s| s.step == "decision"),
        "{witness:#?}"
    );
    assert_eq!(witness.path.last().map(|s| s.step), Some("dereference"));
    assert!(
        row.reasons
            .iter()
            .any(|r| r.starts_with("null-deref-finding"))
    );
}

#[test]
fn a_null_constant_reaching_the_dereference_is_a_normal_path_finding() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(int c) { struct S *p = 0; if (c) p = nd_xalloc(sizeof *p); p->a = 1; return 0; }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    assert_eq!(row.verdict, "Unsafe");
}

#[test]
fn a_repeated_condition_over_unchanged_values_is_decided() {
    if !clang_available() {
        return;
    }
    // The same test guards the assignment and the dereference: no path
    // reaches it NULL.
    let row = subject(
        "int subject(int c) { struct S *p = 0; if (c) p = nd_xalloc(sizeof *p); nd_use(0); if (c) p->a = 1; return 0; }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
    // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15t): an infeasible candidate
    // is removed (before that card the row stayed `Unknown`).
    assert_eq!(row.verdict, "Safe");
    // A flag assigned with the pointer decides the later test too.
    let row = subject(
        "int subject(int c) { int ok = 0; struct S *p = 0; if (c) { p = nd_xalloc(sizeof *p); ok = 1; } if (ok) p->a = 1; return 0; }",
    );
    assert_eq!(statuses(&row), vec![("infeasible", None)], "{row:#?}");
}

#[test]
fn a_correlated_condition_leaves_the_candidate_unresolved() {
    if !clang_available() {
        return;
    }
    // `n` is an expression of the formal, constrained by the first test.
    let row = subject(
        "int subject(int c) { int n = c + 1; struct S *p = 0; if (n > 3) p = nd_xalloc(sizeof *p); if (n > 4) p->a = 1; return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    assert_eq!(row.verdict, "Unknown");
}

#[test]
fn a_memory_value_retested_after_a_call_is_correlated() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct S *s) { struct S *p = 0; if (s->a == 1) p = nd_xalloc(sizeof *p); nd_use(0); if (s->a == 1) p->a = 1; return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
}

#[test]
fn the_functions_own_null_test_falling_through_is_a_finding() {
    if !clang_available() {
        return;
    }
    let row = subject("void subject(struct S *p) { if (!p) nd_use(0); p->a = 1; }");
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    let witness = row.triage[0].witness.as_ref().expect("a witness");
    assert!(witness.condition.contains("own test"), "{witness:#?}");
}

#[test]
fn a_lookup_that_misses_is_a_normal_path_finding() {
    if !clang_available() {
        return;
    }
    let row = subject("void subject(const char *s) { char *q = nd_find(s, 'x'); *q = 0; }");
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
}

#[test]
fn an_uninitialised_pointer_is_indeterminate_not_a_finding() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct S *s, int n) { struct S *p; for (int i = 0; i < n; i++) p = s; return p->a; }",
    );
    assert_eq!(statuses(&row), vec![("indeterminate", None)], "{row:#?}");
    assert_eq!(row.verdict, "Unknown");
}

#[test]
fn a_formal_without_a_test_is_never_searched() {
    if !clang_available() {
        return;
    }
    let row = subject("int subject(struct S *s) { struct S *n = s->next; return n->a; }");
    assert!(row.triage.is_empty(), "{row:#?}");
    assert_eq!(row.verdict, "Unknown");
}

#[test]
fn a_declared_member_is_read_non_null_and_every_store_is_evidence() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "int subject(struct S *s) { if (!s) return 0; return s->next->a; }\n\
         void keep(struct S *s, struct S *t) { s->next = t; }\n\
         void link(struct S *s) { s->a = 0; s->next = s; }",
        &["subject", "keep", "link"],
        Some(MEMBER),
    );
    let row = &unit.functions[0];
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    assert!(
        row.checks
            .iter()
            .any(|c| c.proofs.contains(&"result:member:S.next".to_owned())),
        "{row:#?}"
    );
    assert!(
        row.contract_dependencies
            .contains(&"member:S.next".to_owned())
    );
    assert!(
        row.assumptions
            .iter()
            .any(|a| a.starts_with("declared-member:S.next"))
    );
    // `keep` stores a formal it never tested (not proved); `link` stores an
    // address (proved).
    let stores = unit
        .member_stores
        .iter()
        .map(|s| (s.function.as_str(), s.outcome))
        .collect::<Vec<_>>();
    assert!(stores.contains(&("keep", "candidate")), "{stores:?}");
    assert!(stores.contains(&("link", "proved")), "{stores:?}");
}

#[test]
fn an_initializer_omitting_a_declared_member_stores_null() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "void build(struct S *out) { struct S s = { .a = 1 }; *out = s; }",
        &["build"],
        Some(MEMBER),
    );
    assert!(
        unit.member_stores
            .iter()
            .any(|s| s.member == "S.next" && !matches!(s.outcome, "proved" | "moved")),
        "{:?}",
        unit.member_stores
    );
}

#[test]
fn a_declared_result_is_non_null_for_callers_and_its_body_is_exported() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "struct S *nd_get(int k) { static struct S cells[4]; return &cells[k & 3]; }\n\
         int subject(int k) { return nd_get(k)->a; }",
        &["subject", "nd_get"],
        Some(MEMBER),
    );
    let row = &unit.functions[0];
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    assert!(
        row.contract_dependencies
            .contains(&"result:nd_get".to_owned())
    );
    let export = unit
        .definitions
        .iter()
        .find(|d| d.function == "nd_get")
        .expect("nd_get's export");
    assert!(export.external && export.returns_non_null && !export.returns_null_on_a_path);
}

#[test]
fn every_definition_exports_its_inferred_clauses_and_external_calls_are_recorded() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "int deref(struct S *s) { return s->a; }\n\
         static int hidden(struct S *s) { return s->a; }\n\
         int subject(struct S *s, void *p) { nd_use(p); return deref(s) + hidden(s); }",
        &["subject"],
        None,
    );
    let deref = unit
        .definitions
        .iter()
        .find(|d| d.function == "deref")
        .expect("deref");
    assert!(deref.external);
    assert_eq!(deref.inferred, vec![(0, "s".to_owned())]);
    let hidden = unit
        .definitions
        .iter()
        .find(|d| d.function == "hidden")
        .expect("hidden");
    assert!(!hidden.external);
    // `nd_use` is neither defined nor declared here: its actual is recorded.
    assert!(
        unit.external_calls
            .iter()
            .any(|c| c.callee == "nd_use" && c.position == 0 && c.outcome == "candidate"),
        "{:?}",
        unit.external_calls
    );
}

#[test]
fn the_new_clauses_are_checked_by_the_loader() {
    let text = |body: &str| {
        format!(
            "module contract;\nextern fn nd_get(k: i32) -> ptr<mut, u8>\ncontract {{\n    requires bounds_contract_v1;\n    {body}\n}};\n"
        )
    };
    let load = |body: &str| {
        load_bounds_contract_sources(&BTreeMap::from([("m.qv".to_owned(), text(body))]))
    };
    assert!(load("requires nonnull_member(S, next);").is_ok());
    assert!(load("ensures normal nonnull_result();").is_ok());
    // A parameter is no struct tag; a result clause is no precondition.
    assert!(load("requires nonnull_member(k, next);").is_err());
    assert!(load("requires nonnull_result();").is_err());
    assert!(
        load("requires nonnull_member(S, next);\n    requires nonnull_member(S, next);").is_err()
    );
    // A unit result has no non-null result.
    let unit = "module contract;\nextern fn f(k: i32) -> unit\ncontract {\n    requires bounds_contract_v1;\n    ensures normal nonnull_result();\n};\n";
    assert!(
        load_bounds_contract_sources(&BTreeMap::from([("m.qv".to_owned(), unit.to_owned())]))
            .is_err()
    );
    // The bounds consumer reads the environment without them.
    let loaded = load_bounds_contract_sources(&BTreeMap::from([(
        "m.qv".to_owned(),
        text("requires nonnull_member(S, next);"),
    )]))
    .expect("loads");
    assert!(loaded.for_bounds().functions.is_empty());
    assert_eq!(loaded.null_declarations().count(), 1);
}

#[test]
fn a_member_read_through_a_typedef_names_its_struct() {
    if !clang_available() {
        return;
    }
    let clauses = "module contract;
extern fn nd_get(k: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member(T_s, self);
};
";
    let unit = report(
        "typedef struct T_s T_t; struct T_s { int a; T_t *self; };\n\
         int subject(T_t *t) { if (!t) return 0; return t->self->a; }",
        &["subject"],
        Some(clauses),
    );
    let row = &unit.functions[0];
    assert_eq!(row.verdict, "Safe", "{row:#?}");
}
