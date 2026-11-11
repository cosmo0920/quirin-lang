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

//! Card NULL-MEMBER-INVARIANT-SCOPE (gateway rules 14x-14z): the controls of
//! stable members (a non-null fact kept across calls and stores, every store
//! to an existing object recorded, an initializer list a fresh object) and
//! of function-scope entries (the body's entry fact, the callers' checks),
//! each on a real Clang AST.
//!
//! The fixture declarations are trusted test assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, NullUnitReport, analyze_null_c11};
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct S { int a; struct S *next; int arr[4]; };
typedef struct S S_t;
void nd_use(void *p); struct S *nd_make(int k);
void *memset(void *s, int c, unsigned long n);
";

/// The stable member (hosted on `nd_make`) and a scope entry (`scoped`).
const STABLE: &str = "module contract;
extern fn nd_make(k: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member(S, next) where quant #scope [[stable]];
};
extern fn scoped(s: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires nonnull_member(S, next, s) where quant #scope [[scope]];
};
";

/// The scope entry alone (no stable member).
const SCOPE_ONLY: &str = "module contract;
extern fn scoped(s: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires nonnull_member(S, next, s) where quant #scope [[scope]];
};
";

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
    analyze_null_c11(
        &unit,
        &functions,
        &ResourceContracts::default(),
        &bounds(clauses),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error}\n{text}"))
}

fn row<'a>(unit: &'a NullUnitReport, name: &str) -> &'a NullFunctionRow {
    unit.row(name).expect("the row")
}

fn subject_checks(row: &NullFunctionRow, subject: &str) -> Vec<(&'static str, Vec<String>)> {
    row.checks
        .iter()
        .filter(|c| c.subject == subject)
        .map(|c| (c.outcome, c.proofs.clone()))
        .collect()
}

const TESTED_THEN_CALL: &str =
    "int subject(struct S *s) { if (!s || !s->next) return 0; nd_use(s); return s->next->a; }";

#[test]
fn a_stable_members_non_null_fact_survives_a_call() {
    if !clang_available() {
        return;
    }
    let unit = report(TESTED_THEN_CALL, &["subject"], Some(STABLE));
    let subject = row(&unit, "subject");
    assert_eq!(subject.verdict, "Safe", "{subject:#?}");
    let checks = subject_checks(subject, "s->next");
    assert!(
        checks
            .iter()
            .any(|(_, proofs)| proofs.contains(&"result:stable:S.next".to_owned())),
        "{checks:?}"
    );
    assert!(
        subject
            .contract_dependencies
            .contains(&"stable:S.next".to_owned())
    );
    assert!(
        subject
            .assumptions
            .iter()
            .any(|a| a.starts_with("stable:S.next"))
    );
}

#[test]
fn without_the_declaration_the_call_forgets_the_fact() {
    if !clang_available() {
        return;
    }
    let unit = report(TESTED_THEN_CALL, &["subject"], None);
    let subject = row(&unit, "subject");
    assert_eq!(subject.verdict, "Unknown", "{subject:#?}");
    assert!(
        subject
            .checks
            .iter()
            .any(|c| c.subject == "s->next" && c.outcome == "candidate")
    );
}

#[test]
fn a_possibly_null_fact_is_never_kept() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "int subject(struct S *s, int k) { if (!s) return 0; if (k && !s->next) nd_use(0); nd_use(s); return s->next->a; }",
        &["subject"],
        Some(STABLE),
    );
    let subject = row(&unit, "subject");
    assert_eq!(subject.verdict, "Unknown", "{subject:#?}");
}

#[test]
fn a_reassigned_base_and_a_memory_base_keep_nothing() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "int reassigned(struct S *s) { if (!s || !s->next) return 0; s = nd_make(1); if (!s) return 0; nd_use(s); return s->next->a; }\n\
         int memory(struct S **pp) { if (!pp || !*pp || !(*pp)->next) return 0; nd_use(pp); return (*pp)->next->a; }",
        &["reassigned", "memory"],
        Some(STABLE),
    );
    assert_eq!(row(&unit, "reassigned").verdict, "Unknown");
    assert_eq!(row(&unit, "memory").verdict, "Unknown");
}

#[test]
fn every_store_to_a_stable_member_is_recorded_by_kind() {
    if !clang_available() {
        return;
    }
    let unit = report(
        "void keep(struct S *s, struct S *t) { s->next = t; }\n\
         void link(struct S *s) { s->a = 0; s->next = s; }\n\
         void fresh(void) { struct S x = { .a = 1 }; nd_use(&x); }\n\
         void copy(struct S *s, struct S *t) { *s = *t; }\n\
         void fill(S_t *s) { memset(s, 0, sizeof *s); }",
        &["keep", "link", "fresh", "copy", "fill"],
        Some(STABLE),
    );
    let stores = unit
        .stable_stores
        .iter()
        .map(|s| (s.function.as_str(), s.kind, s.outcome))
        .collect::<Vec<_>>();
    assert!(
        stores.contains(&("keep", "assignment", "candidate")),
        "{stores:?}"
    );
    assert!(
        stores.contains(&("link", "assignment", "proved")),
        "{stores:?}"
    );
    assert!(
        stores
            .iter()
            .any(|(f, k, _)| *f == "fresh" && *k == "initializer"),
        "{stores:?}"
    );
    assert!(
        stores.contains(&("copy", "whole-object", "candidate")),
        "{stores:?}"
    );
    assert!(
        stores.contains(&("fill", "whole-object", "candidate")),
        "{stores:?}"
    );
    // Rule 14t's member stores are not recorded for a stable member.
    assert!(unit.member_stores.is_empty());
}

#[test]
fn a_scope_entry_is_read_at_entry_and_checked_at_every_call() {
    if !clang_available() {
        return;
    }
    let source = "int scoped(struct S *s) { if (!s) return 0; nd_use(s); return s->next->a; }\n\
                  int good(struct S *s) { if (!s || !s->next) return 0; nd_use(s); return scoped(s); }\n\
                  int bad(struct S *s) { return scoped(s); }";
    let unit = report(source, &["scoped", "good", "bad"], Some(STABLE));
    let scoped = row(&unit, "scoped");
    assert_eq!(scoped.verdict, "Safe", "{scoped:#?}");
    assert!(
        scoped.scope.contains("nonnull_member_scope(S, next, s)"),
        "{}",
        scoped.scope
    );
    assert!(scoped.contract_dependencies.contains(&"scoped".to_owned()));
    let good = row(&unit, "good");
    assert_eq!(good.verdict, "Safe", "{good:#?}");
    assert!(
        good.checks.iter().any(|c| c.kind == "call-precondition"
            && c.subject.starts_with("s->next at `scoped`")
            && matches!(c.outcome, "proved" | "moved")),
        "{good:#?}"
    );
    let bad = row(&unit, "bad");
    assert_eq!(bad.verdict, "Unknown", "{bad:#?}");
    assert!(
        bad.checks
            .iter()
            .any(|c| c.subject.starts_with("s->next at `scoped`") && c.outcome == "candidate"),
        "{bad:#?}"
    );
}

#[test]
fn without_a_stable_member_the_entry_fact_lasts_until_the_first_call() {
    if !clang_available() {
        return;
    }
    let source = "int scoped(struct S *s) { if (!s) return 0; int a = s->next->a; nd_use(s); return a + s->next->a; }";
    let unit = report(source, &["scoped"], Some(SCOPE_ONLY));
    let scoped = row(&unit, "scoped");
    assert_eq!(scoped.verdict, "Unknown", "{scoped:#?}");
    let checks = subject_checks(scoped, "s->next");
    assert!(
        checks.iter().any(|(o, p)| *o == "proved"
            && p.iter()
                .any(|p| p.contains("nonnull_member_scope(S, next, s)"))),
        "{checks:?}"
    );
    assert!(checks.iter().any(|(o, _)| *o == "candidate"), "{checks:?}");
}

#[test]
fn the_new_clauses_are_checked_by_the_loader() {
    let text = |body: &str| {
        format!(
            "module contract;\nextern fn f(s: ptr<mut, u8>, k: i32) -> unit\ncontract {{\n    requires bounds_contract_v1;\n    {body}\n}};\n"
        )
    };
    let load = |body: &str| {
        load_bounds_contract_sources(&BTreeMap::from([("m.qv".to_owned(), text(body))]))
    };
    assert!(load("requires nonnull_member(S, next) where quant #scope [[stable]];").is_ok());
    assert!(load("requires nonnull_member(S, next, s) where quant #scope [[scope]];").is_ok());
    // A parameter is no struct tag; the scope's pointer is a pointer
    // parameter; a stable member is no postcondition.
    assert!(load("requires nonnull_member(s, next) where quant #scope [[stable]];").is_err());
    assert!(load("requires nonnull_member(S, next, k) where quant #scope [[scope]];").is_err());
    assert!(load("requires nonnull_member(S, next, q) where quant #scope [[scope]];").is_err());
    assert!(load("ensures normal nonnull_member(S, next) where quant #scope [[stable]];").is_err());
    assert!(
        load(
            "requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(S, next) where quant #scope [[stable]];"
        )
        .is_err()
    );
    // The bounds consumer reads the environment without them.
    let loaded = load_bounds_contract_sources(&BTreeMap::from([(
        "m.qv".to_owned(),
        text("requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(S, next, s) where quant #scope [[scope]];"),
    )]))
    .expect("loads");
    assert!(loaded.for_bounds().functions.is_empty());
    assert_eq!(loaded.null_declarations().count(), 1);
}
