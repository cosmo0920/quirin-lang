//! Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (gateway rules 12m-12o): the
//! positive and negative controls of each clause on route G, over the
//! synthetic `c_*` callees of `fixtures/contracts/op_callbacks.qv` (trusted
//! test assumptions: a `Safe` here is `Safe` over them and promotes nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
struct op { int type; int (*op_cb)(void *rk, void *rkq, struct op *rko); void *payload; };
struct item { int x; };
struct box { int type; struct item *payload; };
struct box *c_box_new(int n); struct item *c_item_new(int n);
void *c_alloc(unsigned long n); void c_free(void *p);
int c_serve(void *q, int (*cb)(void *, void *, void *), void *opaque);
void c_op_destroy(struct op *rko);
void c_op_free(struct box *rko);
void c_op_free_all(struct box *rko);
void c_op_free_partial(struct box *rko);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "op_callbacks.qv".to_owned(),
        include_str!("fixtures/contracts/op_callbacks.qv").to_owned(),
    )]))
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
            eprintln!("skipping Clang-dependent CG22-OP-CALLBACKS test: {reason}");
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
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status != AnalysisStatus::Safe && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// Every row of the unit `PRELUDE + source`, keyed by function.
fn rows(source: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{source}\n");
    let entry = source
        .lines()
        .rev()
        .find(|line| line.contains('('))
        .and_then(|line| line.split('(').next())
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule 12m: an object the callee supplies and hands the callback
// ---------------------------------------------------------------------------

const SERVE: &str = "\
static int take(void *q, void *op, void *opaque) { c_free(op); return 0; }
static int keep(void *q, void *op, void *opaque) { return 0; }
int serve_taking(void *q, void *opaque) { return c_serve(q, take, opaque); }
int serve_keeping(void *q, void *opaque) { return c_serve(q, keep, opaque); }
int serve_formal(void *q, int (*cb)(void *, void *, void *), void *opaque) { return c_serve(q, cb, opaque); }
";

#[test]
fn a_callback_that_takes_the_supplied_object_is_admitted() {
    if !clang_available() {
        return;
    }
    let rows = rows(SERVE);
    assert!(rows["serve_taking"].safe(), "{:?}", rows["serve_taking"]);
}

#[test]
fn a_callback_that_leaves_the_supplied_object_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(SERVE);
    assert!(
        rows["serve_keeping"].refused_with("callback-supplied-kept"),
        "{:?}",
        rows["serve_keeping"]
    );
    assert!(
        rows["serve_formal"].refused_with("callback-undecided"),
        "{:?}",
        rows["serve_formal"]
    );
}

// ---------------------------------------------------------------------------
// Rule 12n: a registration carried by the object
// ---------------------------------------------------------------------------

const MEMBER: &str = "\
static int handler(void *rk, void *rkq, struct op *rko) { return 0; }
void set_handler(struct op *o) { o->op_cb = handler; }
void clear_handler(struct op *o) { o->op_cb = NULL; }
void drop(void) { struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }
";

#[test]
fn a_closed_member_set_of_identities_decides() {
    if !clang_available() {
        return;
    }
    let rows = rows(MEMBER);
    assert!(rows["drop"].safe(), "{:?}", rows["drop"]);
}

#[test]
fn an_open_member_set_refuses() {
    if !clang_available() {
        return;
    }
    // A store of a formal of non-function-pointer type opens the member.
    let open = format!("{MEMBER}void set_any(struct op *o, void *f) {{ o->op_cb = f; }}\n");
    let rows_open = rows(&open);
    assert!(
        rows_open["drop"].refused_with("callback-member-open"),
        "{:?}",
        rows_open["drop"]
    );
    // A function-pointer formal whose function's address is taken opens it.
    let escaped = format!(
        "{MEMBER}void set_cb(struct op *o, int (*f)(void *, void *, struct op *)) {{ o->op_cb = f; }}\n\
         void (*registrar)(struct op *, int (*)(void *, void *, struct op *)) = set_cb;\n\
         void drop2(void) {{ struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }}\n"
    );
    let rows_escaped = rows(&escaped);
    assert!(
        rows_escaped["drop2"].refused_with("callback-member-open"),
        "{:?}",
        rows_escaped["drop2"]
    );
    // No store in the unit: nothing the unit can decide.
    let none = "void drop(void) { struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }";
    let rows_none = rows(none);
    assert!(
        rows_none["drop"].refused_with("callback-member-open"),
        "{:?}",
        rows_none["drop"]
    );
    // A closed set with an effectful candidate.
    let effectful = "\
static int freeing(void *rk, void *rkq, struct op *rko) { c_free(rk); return 0; }
void set_freeing(struct op *o) { o->op_cb = freeing; }
void drop(void) { struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }";
    let rows_effectful = rows(effectful);
    assert!(
        rows_effectful["drop"].refused_with("callback-effectful"),
        "{:?}",
        rows_effectful["drop"]
    );
}

#[test]
fn a_formal_registrar_closes_over_its_callers() {
    if !clang_available() {
        return;
    }
    let source = "\
static int handler(void *rk, void *rkq, struct op *rko) { return 0; }
static void set_cb(struct op *o, int (*f)(void *, void *, struct op *)) { o->op_cb = f; }
void install(struct op *o) { set_cb(o, handler); }
void drop(void) { struct op *o = c_alloc(sizeof(struct op)); if (!o) return; c_op_destroy(o); }";
    let rows = rows(source);
    assert!(rows["drop"].safe(), "{:?}", rows["drop"]);
}

// ---------------------------------------------------------------------------
// Rule 12o: a type-switched release
// ---------------------------------------------------------------------------

fn linking(callee: &str) -> String {
    format!(
        "void link_and_free(int n) {{ struct item *it; struct box *b; it = c_item_new(n); \
         if (!it) {{ return; }} b = c_box_new(n); if (!b) {{ c_free(it); return; }} \
         b->payload = it; {callee}(b); }}"
    )
}

#[test]
fn an_undecided_discriminant_takes_the_cautious_join() {
    if !clang_available() {
        return;
    }
    // Every arm frees the payload: the linked object is released.
    let every = rows(&linking("c_op_free_all"));
    assert!(
        every["link_and_free"].safe(),
        "{:?}",
        every["link_and_free"]
    );
    // Only arm 1 frees it: undecided.
    let some = rows(&linking("c_op_free"));
    assert!(
        some["link_and_free"].refused_with("arm-undecided"),
        "{:?}",
        some["link_and_free"]
    );
}

#[test]
fn a_value_with_no_declared_arm_is_never_neutral() {
    if !clang_available() {
        return;
    }
    let probe = rows("void drop(struct box *o) { c_op_free_all(o); }");
    eprintln!("PROBE {:?}", probe["drop"]);
    let rows = rows("void drop(struct box *o) { c_op_free_partial(o); }");
    assert!(
        rows["drop"].refused_with("arm-undeclared"),
        "{:?}",
        rows["drop"]
    );
}

#[test]
fn the_arms_stand_where_the_body_validates_them() {
    if !clang_available() {
        return;
    }
    let matching = "\
void c_op_free(struct box *rko) { switch (rko->type) { case 1: c_free(rko->payload); break; default: break; } c_free(rko); }
void use_free(struct box *o) { c_op_free(o); }";
    let rows_matching = rows(matching);
    assert!(
        !rows_matching["use_free"].refused_with("arm-withdrawn"),
        "{:?}",
        rows_matching["use_free"]
    );
    let mismatching = "\
void c_op_free(struct box *rko) { switch (rko->type) { case 2: c_free(rko->payload); break; default: break; } c_free(rko); }
void use_free(struct box *o) { c_op_free(o); }";
    let rows_mismatching = rows(mismatching);
    assert!(
        rows_mismatching["use_free"].refused_with("arm-withdrawn"),
        "{:?}",
        rows_mismatching["use_free"]
    );
}
