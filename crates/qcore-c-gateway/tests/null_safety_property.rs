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

//! Card NULL-SAFETY-PROPERTY (gateway rules 14k-14p): the null-dereference
//! property's controls (brief task 6), each on a real Clang AST.
//!
//! The fixture contracts are trusted test assumptions, not library facts:
//! `nd_alloc` has an owned nullable result (an allocation), `nd_xalloc` an
//! owned non-null one, `nd_find` a nullable interior result (a lookup);
//! `nd_use` has none.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, analyze_null_c11};
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct S { int a; struct S *next; int arr[4]; struct S *kids; };
void *nd_alloc(unsigned long n); void *nd_xalloc(unsigned long n);
char *nd_find(const char *s, int c); void nd_use(void *p);
void __assert_fail(const char *, const char *, unsigned int, const char *) __attribute__((noreturn));
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

fn rows_with(
    source: &str,
    functions: &[&str],
    clauses: Option<&str>,
    defines: &[&str],
) -> Vec<NullFunctionRow> {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let unit = C11TranslationUnit {
        source: &text,
        definitions: defines.iter().map(|d| (*d).to_owned()).collect(),
        ..Default::default()
    };
    let functions = functions
        .iter()
        .map(|f| (*f).to_owned())
        .collect::<Vec<_>>();
    analyze_null_c11(&unit, &functions, &resource(), &bounds(clauses))
        .unwrap_or_else(|error| panic!("fixture rejected: {error}\n{text}"))
        .functions
}

fn subject_row(source: &str) -> NullFunctionRow {
    rows_with(source, &["subject"], None, &[]).remove(0)
}

fn outcomes(row: &NullFunctionRow) -> Vec<(&str, &str)> {
    row.checks
        .iter()
        .map(|check| (check.subject.as_str(), check.outcome))
        .collect()
}

#[test]
fn an_unchecked_allocation_dereferenced_is_a_candidate() {
    if !clang_available() {
        return;
    }
    let row = subject_row(
        "int subject(void) { struct S *p = nd_alloc(sizeof *p); p->a = 1; return p->a; }",
    );
    // Card NULL-CANDIDATE-TRIAGE (rule 14r): the candidate's path is
    // validated (NULL only when the allocation fails), so the check is a
    // finding and the row provisional `Unsafe`; its sources are unchanged.
    assert_eq!(row.verdict, "Unsafe", "{row:#?}");
    assert_eq!(
        outcomes(&row),
        vec![("p", "finding"), ("p", "moved")],
        "{row:#?}"
    );
    assert_eq!(
        row.checks[0].sources,
        vec!["allocation:nd_alloc".to_owned()]
    );
    // The second dereference's question is the first's (decision 1).
    assert_eq!(
        row.checks[1].proofs,
        vec![format!("moved:{}", row.checks[0].id)]
    );
}

#[test]
fn a_checked_allocation_is_proved() {
    if !clang_available() {
        return;
    }
    let row = subject_row(
        "int subject(void) { struct S *p = nd_alloc(sizeof *p); if (!p) return -1; p->a = 1; return p->a; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    assert!(row.checks.iter().all(|c| c.outcome == "proved"), "{row:#?}");
    // A non-null contract result needs no test, and is a listed assumption.
    let row = subject_row("int subject(void) { struct S *p = nd_xalloc(8); return p->a; }");
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    assert!(
        row.assumptions
            .iter()
            .any(|a| a.starts_with("declared:nd_xalloc")),
        "{row:#?}"
    );
}

#[test]
fn a_null_test_then_a_dereference_is_proved() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(struct S *s) { if (s) return s->a; return 0; }",
        "int subject(struct S *s) { if (s != 0) return s->a; return 0; }",
        "int subject(struct S *s) { if (!s) return 0; return s->a; }",
        "int subject(struct S *s) { return s && s->a; }",
        "int subject(struct S *s) { return s ? s->a : 0; }",
        "int subject(struct S *s) { if (__builtin_expect(!!(s), 1)) return s->a; return 0; }",
        "int subject(struct S *s) { if (s == (void *)0 || s->a) return 1; return 0; }",
        "int subject(struct S *s) { while (s) { if (s->a) return 1; s = s->next; } return 0; }",
        "int subject(struct S *s) { if (!s->next) return 0; return s->next->a; }",
    ] {
        let row = subject_row(source);
        let candidates = row
            .checks
            .iter()
            .filter(|c| c.outcome != "proved" && c.outcome != "moved")
            .collect::<Vec<_>>();
        assert!(candidates.is_empty(), "{source}\n{row:#?}");
        assert_eq!(row.verdict, "Safe", "{source}\n{row:#?}");
    }
}

#[test]
fn a_dereference_then_a_member_address_moves_the_question() {
    if !clang_available() {
        return;
    }
    // `p` is read from memory (possibly null): its first dereference is a
    // candidate; `&p->arr[1]` is then non-null with the question moved there.
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; int n = p->a; \
         struct S *k = (struct S *)&p->arr[1]; return n + k->a; }",
    );
    let first = row
        .checks
        .iter()
        .find(|c| c.subject == "p")
        .expect("p checked");
    assert_eq!(first.outcome, "candidate", "{row:#?}");
    assert_eq!(first.sources, vec!["memory-field:s->next".to_owned()]);
    let through = row
        .checks
        .iter()
        .find(|c| c.subject == "k")
        .expect("k checked");
    assert_eq!(through.outcome, "moved", "{row:#?}");
    assert_eq!(through.proofs, vec![format!("moved:{}", first.id)]);
    // Forming `&B->G` is not a dereference: without the earlier read, the
    // alias's dereference is its own candidate, from `p`'s source.
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; \
         struct S *k = (struct S *)&p->arr[1]; return k->a; }",
    );
    let derefs = row
        .checks
        .iter()
        .filter(|c| c.subject != "s")
        .collect::<Vec<_>>();
    assert_eq!(derefs.len(), 1, "{row:#?}");
    assert_eq!(derefs[0].subject, "k");
    assert_eq!(derefs[0].outcome, "candidate");
    assert_eq!(derefs[0].sources, vec!["memory-field:s->next".to_owned()]);
}

#[test]
fn a_reassignment_between_does_not_move_the_question() {
    if !clang_available() {
        return;
    }
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; int n = p->a; \
         p = p->next; return n + p->a; }",
    );
    let checks = row
        .checks
        .iter()
        .filter(|c| c.subject == "p")
        .map(|c| c.outcome)
        .collect::<Vec<_>>();
    assert_eq!(checks, vec!["candidate", "moved", "candidate"], "{row:#?}");
    let last = row.checks.iter().rfind(|c| c.subject == "p").expect("p");
    assert_eq!(
        last.sources,
        vec!["memory-field:p->next".to_owned()],
        "{row:#?}"
    );
    // An offset is no reassignment of its nullness: `p + 1` is non-null
    // exactly when `p` is (C11 6.5.6p8).
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->kids; int n = p->a; \
         p++; return n + p->a; }",
    );
    let checks = row
        .checks
        .iter()
        .filter(|c| c.subject == "p")
        .map(|c| c.outcome)
        .collect::<Vec<_>>();
    assert_eq!(checks, vec!["candidate", "moved"], "{row:#?}");
}

#[test]
fn sizeof_of_a_dereference_is_not_a_dereference() {
    if !clang_available() {
        return;
    }
    let row =
        subject_row("unsigned long subject(struct S *s) { return sizeof(*s) + sizeof s->a; }");
    assert!(row.checks.is_empty(), "{row:#?}");
    assert_eq!(row.verdict, "Safe", "{row:#?}");
}

#[test]
fn a_clause_caller_with_an_unproved_actual_is_a_candidate() {
    if !clang_available() {
        return;
    }
    let clause = "module contract;
extern fn nd_use(p: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant null-deref #p;
};
";
    let rows = rows_with(
        "void subject(struct S *s) { if (!s) return; nd_use(s->next); nd_use(s); }",
        &["subject"],
        Some(clause),
        &[],
    );
    let row = &rows[0];
    let calls = row
        .checks
        .iter()
        .filter(|c| c.kind == "call-precondition")
        .map(|c| (c.subject.as_str(), c.outcome))
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        vec![
            ("s->next at quant null-deref #p", "candidate"),
            ("s at quant null-deref #p", "proved")
        ],
        "{row:#?}"
    );
    assert_eq!(row.verdict, "Unknown");
    assert!(row.contract_dependencies.contains(&"nd_use".to_owned()));
    // After the call the actual's question is the call's.
    let rows = rows_with(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; nd_use(p); return p->a; }",
        &["subject"],
        Some(clause),
        &[],
    );
    let after = rows[0]
        .checks
        .iter()
        .find(|c| c.kind == "member-access" && c.subject == "p")
        .expect("p->a");
    assert_eq!(after.outcome, "moved", "{rows:#?}");
    // A formal handed to the clause untested infers the clause in turn.
    let rows = rows_with(
        "void subject(void *q) { nd_use(q); }",
        &["subject"],
        Some(clause),
        &[],
    );
    assert_eq!(
        rows[0].inferred_preconditions,
        vec!["#q".to_owned()],
        "{rows:#?}"
    );
}

#[test]
fn a_clause_body_with_a_prior_null_test_is_stronger_than_necessary() {
    if !clang_available() {
        return;
    }
    let clause = "module contract;
extern fn tested(s: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant null-deref #s;
};
extern fn used(s: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant null-deref #s;
};
";
    let rows = rows_with(
        "int tested(struct S *s) { if (!s) return 0; return s->a; }
         int used(struct S *s) { return s->a; }",
        &["tested", "used"],
        Some(clause),
        &[],
    );
    assert_eq!(
        rows[0].clause_standing[0].standing, "stronger-than-necessary",
        "{rows:#?}"
    );
    assert_eq!(rows[0].verdict, "Safe", "never wrong: {rows:#?}");
    assert_eq!(
        rows[1].clause_standing[0].standing, "necessary",
        "{rows:#?}"
    );
    assert_eq!(rows[1].verdict, "Safe", "{rows:#?}");
    assert!(
        rows[1].scope.contains("declared quant null-deref #s"),
        "{rows:#?}"
    );
}

#[test]
fn inference_needs_a_dereference_on_every_path() {
    if !clang_available() {
        return;
    }
    let rows = rows_with(
        "static int every(struct S *s) { if (s->a) return 1; return s->next != 0; }
         static int some(struct S *s, int k) { if (k) return s->a; return 0; }
         int caller(struct S *t) { return every(t) + some(t, 1); }",
        &["every", "some", "caller"],
        None,
        &[],
    );
    assert_eq!(
        rows[0].inferred_preconditions,
        vec!["#s".to_owned()],
        "{rows:#?}"
    );
    assert_eq!(rows[0].verdict, "Safe", "{rows:#?}");
    assert!(rows[1].inferred_preconditions.is_empty(), "{rows:#?}");
    assert_eq!(rows[1].verdict, "Unknown", "{rows:#?}");
    // The caller carries `every`'s obligation (and not `some`'s).
    let calls = rows[2]
        .checks
        .iter()
        .filter(|c| c.kind == "call-precondition")
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1, "{rows:#?}");
    assert!(calls[0].subject.contains("every"), "{rows:#?}");
    // ... which `caller` infers in turn: it passes its formal untested.
    assert_eq!(
        rows[2].inferred_preconditions,
        vec!["#t".to_owned()],
        "{rows:#?}"
    );
}

#[test]
fn an_assertion_is_a_guard_only_without_ndebug() {
    if !clang_available() {
        return;
    }
    // glibc's expansion, spelled out (the unit declares no input file, so
    // `<assert.h>`'s `__FILE__` is refused by the gateway).
    let source = "#ifdef NDEBUG\n# define nd_assert(e) ((void) 0)\n#else\n\
                  # define nd_assert(e) ((void) sizeof ((e) ? 1 : 0), __extension__ ({ if (e) ; \
                  else __assert_fail (#e, \"nd.c\", 1, __func__); }))\n#endif\n\
                  int subject(struct S *s) { if (!s) return 0; \
                  struct S *p = s->next; nd_assert(p); return p->a; }";
    let with = rows_with(source, &["subject"], None, &[]).remove(0);
    assert_eq!(with.verdict, "Safe", "{with:#?}");
    assert!(
        with.assumptions.iter().any(|a| a.starts_with("asserted:")),
        "{with:#?}"
    );
    let without = rows_with(source, &["subject"], None, &["NDEBUG"]).remove(0);
    assert_eq!(without.verdict, "Unknown", "{without:#?}");
    assert!(
        without
            .assumptions
            .iter()
            .all(|a| !a.starts_with("asserted:"))
    );
}

#[test]
fn a_definite_null_dereference_on_the_entry_prefix_is_unsafe() {
    if !clang_available() {
        return;
    }
    let row = subject_row("int subject(void) { struct S *p = 0; return p->a; }");
    assert_eq!(row.verdict, "Unsafe", "{row:#?}");
    // Behind a branch, the same is unresolved reachability, never Unsafe.
    let row = subject_row("int subject(int k) { struct S *p = 0; if (k) return p->a; return 0; }");
    assert_eq!(row.verdict, "Unknown", "{row:#?}");
}

#[test]
fn lookups_and_memory_reads_are_sources() {
    if !clang_available() {
        return;
    }
    let row = subject_row(
        "int subject(const char *s) { if (!s) return 0; char *c = nd_find(s, 1); return *c; }",
    );
    let last = row.checks.last().expect("the lookup's dereference");
    assert_eq!(last.sources, vec!["lookup:nd_find".to_owned()], "{row:#?}");
    // A call forgets a path fact; a local's fact survives it.
    let row = subject_row(
        "int subject(struct S *s) { if (!s || !s->next) return 0; nd_use(s); return s->next->a; }",
    );
    let next = row
        .checks
        .iter()
        .find(|c| c.subject == "s->next")
        .expect("s->next");
    assert_eq!(next.outcome, "candidate", "{row:#?}");
    let base = row.checks.iter().rfind(|c| c.subject == "s").expect("s");
    assert_eq!(base.outcome, "proved", "{row:#?}");
}

#[test]
fn qualified_formals_errno_and_generic_selections_are_read() {
    if !clang_available() {
        return;
    }
    // A `const` pointer formal is a pointer (`T *const`).
    let row = subject_row("int subject(struct S *const s) { if (!s) return 0; return s->a; }");
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    // `errno` is a modifiable lvalue (C11 7.5p2).
    let row = subject_row(
        "int *__errno_location(void);\nint subject(void) { return (*__errno_location ()); }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    // Only the selected association of a generic selection is evaluated.
    let row = subject_row(
        "int subject(struct S *s, int k) { if (!s) return 0; return _Generic(k, int: s->a, default: s->next->a); }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    assert!(
        row.checks.iter().all(|c| c.subject != "s->next"),
        "{row:#?}"
    );
}

#[test]
fn a_test_of_an_assignment_tests_its_left_operand() {
    if !clang_available() {
        return;
    }
    // `if (!(p = alloc()))`: the assignment's value is `p` (C11 6.5.16p3).
    let row = subject_row(
        "int subject(void) { struct S *p; if (!(p = nd_alloc(8))) return -1; return p->a; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *n; \
         if ((n = s->next) != 0) return n->a; return 0; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    // A contradicted arm is infeasible even when the tested value has no
    // place: `(e->next = NULL) != NULL` never takes its true arm.
    let row = subject_row(
        "int subject(struct S *s, struct S *e) { if (!s || !e) return 0; \
         if ((e->next = (struct S *)0) != 0) return e->next->a; return 1; }",
    );
    assert!(
        row.checks
            .iter()
            .all(|c| !c.outcome.starts_with("violation")),
        "{row:#?}"
    );
}

#[test]
fn a_constant_failing_assertion_ends_its_path() {
    if !clang_available() {
        return;
    }
    // librdkafka's `RD_BUG` is `rd_assert(!*"INTERNAL ERROR")`: the first
    // character of a non-empty literal is a nonzero constant, so only the
    // assertion's failing (no-return) arm is reachable.
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; \
         if (!p) { if (!*\"bug\") ; else __assert_fail(\"x\", \"f\", 1, \"g\"); } return p->a; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    // `__builtin_alloca` has no null result.
    let row = subject_row(
        "int subject(void) { struct S *p = __builtin_alloca(sizeof *p); p->a = 1; return p->a; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
}

#[test]
fn a_test_of_a_value_decides_its_copies() {
    if !clang_available() {
        return;
    }
    // monkey's mk_string_tolower: `op` is a copy of `out` taken before the
    // test of `out`; the test decides both.
    let row = subject_row(
        "char *subject(const char *in) { char *out = nd_find(in, 0); char *op = out; \
         if (!out) return 0; *op = 'x'; return out; }",
    );
    assert_eq!(row.verdict, "Safe", "{row:#?}");
    // A reassignment detaches the copy.
    let row = subject_row(
        "int subject(struct S *s) { if (!s) return 0; struct S *p = s->next; struct S *q = p; \
         p = s; if (!p) return 0; return q->a; }",
    );
    let q = row.checks.iter().find(|c| c.subject == "q").expect("q");
    assert_eq!(q.outcome, "candidate", "{row:#?}");
}
