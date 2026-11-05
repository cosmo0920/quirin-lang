//! Card NULL-WITNESS-GUARD-CORRELATION (gateway rule 39): a lookup's miss is
//! the NULL event of a witness only when it is feasible on the path by facts
//! the search reads. Each control runs on a real Clang AST.
//!
//! The fixture contracts are trusted test assumptions, not library facts:
//! `q_first` (a one-read accessor, the shape of `rd_kafka_msgq_first`),
//! `q_find` (a declared lookup the unit has no body for) and `tab_alloc` (an
//! arena allocator with its own NULL returns, the shape of
//! `rd_tmpabuf_alloc0`) return a nullable pointer borrowed from their first
//! argument.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, analyze_null_c11};
use qcore_contract::bounds_contract::BoundsContracts;
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct m { int v; struct m *next; };
struct q { struct m *first; int cnt; };
struct tab { unsigned long size; unsigned long of; char *buf; int failed; };
struct x { int a; };
static inline struct m *q_first(const struct q *q) { return q->first; }
struct m *q_find(struct q *q, int key);
static void *tab_alloc(struct tab *t, unsigned long n) {
    void *p;
    if (t->failed) return 0;
    if (t->of + n > t->size) return 0;
    p = t->buf + t->of;
    t->of += n;
    return p;
}
static void tab_new(struct tab *t, unsigned long n) { t->size = n; t->of = 0; t->buf = 0; t->failed = 0; }
void nd_use(void *p);
";

const RESOURCE: &str = "module contract;
extern fn q_first(q: ptr<const, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_return_borrowed_from(q, nullable);
};
extern fn q_find(q: ptr<mut, u8>, key: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(key);
    ensures normal resource_return_borrowed_from(q, nullable);
};
extern fn tab_alloc(t: ptr<mut, u8>, n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_borrowed_from(t, nullable);
};
";

fn resource() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("nl.qv".to_owned(), RESOURCE.to_owned())]))
        .expect("the fixture resource contracts load")
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

fn subject(source: &str) -> NullFunctionRow {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    analyze_null_c11(
        &unit,
        &["subject".to_owned()],
        &resource(),
        &BoundsContracts::default(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error}\n{text}"))
    .functions
    .remove(0)
}

fn statuses(row: &NullFunctionRow) -> Vec<(&str, Option<&str>)> {
    row.triage.iter().map(|t| (t.status, t.class)).collect()
}

fn reason(row: &NullFunctionRow) -> &str {
    assert_eq!(row.triage.len(), 1, "{row:#?}");
    &row.triage[0].reason
}

/// `rd_kafka_msgq_allow_wakeup_at`'s shape: the lookup sits on the arm where
/// the queue's head was tested non-null, and the body ties its result to it.
#[test]
fn a_guard_the_body_ties_to_the_result_refuses_the_witness() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct q *q) {\n if (q->first == 0) { return 0; }\n \
         else { struct m *m = q_first(q);\n return m->v; } }",
    );
    // Before the card: a finding ("the lookup `q_first` ... misses").
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    let reason = reason(&row);
    assert!(reason.contains("rule 39"), "{reason}");
    assert!(
        reason.contains("its body returns `q->first`, which the path already fixed non-null"),
        "{reason}"
    );
    assert_eq!(row.verdict, "Unknown", "{row:#?}");
    // A refusal never removes the candidate.
    assert!(
        row.checks.iter().any(|c| c.outcome == "candidate"),
        "{row:#?}"
    );
}

/// True positive: the lookup is used on the arm where the queue is empty.
#[test]
fn a_lookup_used_on_the_empty_arm_stays_a_finding() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct q *q) {\n if (q->first == 0) { struct m *m = q_first(q);\n \
         return m->v; }\n return 1; }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    let reason = reason(&row);
    assert!(
        reason.starts_with("the lookup `q_first` at line")
            && reason.contains("returns `q->first` (its body), which is NULL on this path"),
        "{reason}"
    );
    assert_eq!(row.verdict, "Unsafe", "{row:#?}");
}

/// `rd_kafka_handle_idempotent_Produce_success`'s shape: no decision; the
/// accessor's NULL is an entry memory value, never a NULL condition.
#[test]
fn an_accessor_over_entry_memory_is_not_a_null_event() {
    if !clang_available() {
        return;
    }
    let row = subject("int subject(struct q *q) {\n return q_first(q)->v; }");
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    assert!(
        reason(&row).contains("a memory value no condition on the path names NULL"),
        "{row:#?}"
    );
    assert_eq!(row.verdict, "Unknown", "{row:#?}");
}

/// The length test's shape without a tie: a test that reads through the
/// lookup's argument correlates its miss.
#[test]
fn a_test_through_the_argument_correlates_an_untied_lookup() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct q *q) {\n if (q->cnt > 0) return q_find(q, 1)->v;\n return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    let reason = reason(&row);
    assert!(
        reason.contains("it reads the state of `q`: the path tested `q->cnt`"),
        "{reason}"
    );
}

/// Control: an untied lookup over state the path left free is still a
/// named miss (rule 14r unchanged).
#[test]
fn an_untied_lookup_over_free_state_stays_a_finding() {
    if !clang_available() {
        return;
    }
    let row = subject("int subject(struct q *q, int key) {\n return q_find(q, key)->v; }");
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    assert!(reason(&row).contains("the lookup `q_find`"), "{row:#?}");
}

/// `rd_tmpabuf_alloc0`'s shape: an arena the function itself initialised
/// and sized on the path.
#[test]
fn a_local_arena_the_path_established_refuses_the_witness() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(void) {\n struct tab t; struct x *p;\n tab_new(&t, sizeof(struct x));\n \
         p = tab_alloc(&t, sizeof *p);\n p->a = 1;\n return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    let reason = reason(&row);
    assert!(
        reason.contains(
            "it reads the state of `&t`: `t` is a local of this function, whose value or object the path establishes"
        ),
        "{reason}"
    );
    assert_eq!(row.verdict, "Unknown", "{row:#?}");
}

/// True positive: an arena the function received and allocates from without
/// any size accounting on its path.
#[test]
fn an_arena_without_size_accounting_stays_a_finding() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct tab *t) {\n struct x *p = tab_alloc(t, sizeof *p);\n \
         p->a = 1;\n return 0; }",
    );
    assert_eq!(
        statuses(&row),
        vec![("finding", Some("null-on-normal-path"))],
        "{row:#?}"
    );
    assert!(reason(&row).contains("the lookup `tab_alloc`"), "{row:#?}");
}

/// A call that received the arena before the lookup correlates its miss.
#[test]
fn a_call_that_received_the_argument_correlates_the_lookup() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct tab *t) {\n struct x *p;\n nd_use(t);\n \
         p = tab_alloc(t, sizeof *p);\n p->a = 1;\n return 0; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    assert!(
        reason(&row).contains("it reads the state of `t`: the call `nd_use` at line"),
        "{row:#?}"
    );
}

/// The function's own NULL test of a refused lookup's result is inexact: its
/// null arm is explored but never a witness.
#[test]
fn a_test_of_a_refused_result_is_never_a_witness() {
    if !clang_available() {
        return;
    }
    let row = subject(
        "int subject(struct q *q) {\n struct m *m;\n if (q->first == 0) return 0;\n \
         m = q_first(q);\n if (m) nd_use(m);\n return m->v; }",
    );
    assert_eq!(statuses(&row), vec![("unresolved", None)], "{row:#?}");
    let reason = reason(&row);
    assert!(
        reason.starts_with("an inexact path reaches the dereference NULL")
            && reason.contains("rule 39"),
        "{reason}"
    );
    assert_eq!(row.verdict, "Unknown", "{row:#?}");
}
