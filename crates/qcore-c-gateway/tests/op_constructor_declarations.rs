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

//! Card OP-CONSTRUCTOR-DECLARATIONS (gateway rules 12o, 13w, 14e): the
//! positive and negative controls of a constructor's stated result state on
//! route G, over the synthetic `c_*` callees of
//! `fixtures/contracts/op_constructors.qv` (trusted test assumptions: a `Safe`
//! here is `Safe` over them and promotes nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
struct item { int x; };
typedef int (*op_cb_t)(void *rk, void *rkq, void *rko);
struct pair { int a; int b; };
struct op { int type; op_cb_t op_cb; struct item *payload; struct pair u; };
struct item *c_item_new(int n);
void c_free(void *p);
void *calloc(unsigned long n, unsigned long size);
void *malloc(unsigned long size);
struct op *c_op_new(int type);
struct op *c_op_new_cb(int type, op_cb_t cb);
void c_op_destroy(struct op *rko);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "op_constructors.qv".to_owned(),
        include_str!("fixtures/contracts/op_constructors.qv").to_owned(),
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
            eprintln!("skipping Clang-dependent OP-CONSTRUCTOR test: {reason}");
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

/// A body that constructs an op of `type` with `make`, links a fresh item
/// at its payload, and destroys the op.
fn linking(make: &str) -> String {
    format!(
        "void link_and_destroy(int n) {{ struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) {{ return; }} o = {make}; o->payload = it; c_op_destroy(o); }}"
    )
}

// ---------------------------------------------------------------------------
// Rule 12o: a decided discriminant selects exactly its arm
// ---------------------------------------------------------------------------

#[test]
fn a_decided_discriminant_selects_exactly_its_arm() {
    if !clang_available() {
        return;
    }
    // Type 1: its arm frees the payload, and the op's callback is absent.
    let one = rows(&linking("c_op_new(1)"));
    assert!(
        one["link_and_destroy"].safe(),
        "{:?}",
        one["link_and_destroy"]
    );
    // Type 2: `otherwise` frees nothing of it; the payload is never released.
    let two = rows(&linking("c_op_new(2)"));
    assert!(
        !two["link_and_destroy"].safe(),
        "{:?}",
        two["link_and_destroy"]
    );
    // The stated flag (256) is cleared by the release's mask.
    let flagged = format!(
        "static int handler(void *rk, void *rkq, void *rko) {{ return 0; }}\n{}",
        linking("c_op_new_cb(1, handler)")
    );
    let flagged = rows(&flagged);
    assert!(
        flagged["link_and_destroy"].safe(),
        "{:?}",
        flagged["link_and_destroy"]
    );
}

#[test]
fn an_undecided_discriminant_takes_the_cautious_join() {
    if !clang_available() {
        return;
    }
    let formal = rows(&linking("c_op_new(n)"));
    assert!(
        formal["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        formal["link_and_destroy"]
    );
}

#[test]
fn an_unclassified_store_withdraws_the_decided_arm() {
    if !clang_available() {
        return;
    }
    // An unguarded store into the discriminant of an existing object.
    let unguarded = format!(
        "void retype(struct op *o) {{ o->type = 1; }}\n{}",
        linking("c_op_new(1)")
    );
    let unguarded = rows(&unguarded);
    assert!(
        unguarded["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        unguarded["link_and_destroy"]
    );
    // A store in the creating function itself.
    let own = "void link_and_destroy(int n) { struct item *it; struct op *o; it = c_item_new(n); \
               if (!it) { return; } o = c_op_new(1); o->type = 1; o->payload = it; c_op_destroy(o); }";
    let own = rows(own);
    assert!(
        own["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        own["link_and_destroy"]
    );
}

#[test]
fn a_guarded_rewrite_keeps_or_joins_the_arm() {
    if !clang_available() {
        return;
    }
    // 3 -> 4: both `otherwise`; the decided 3 still selects `otherwise`, which
    // frees nothing: the payload is lost, never released by a guess.
    let within = format!(
        "void serve(struct op *o) {{ switch (o->type) {{ case 3: o->type = 4; break; default: break; }} }}\n{}",
        linking("c_op_new(3)")
    );
    let within = rows(&within);
    assert!(
        !within["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        within["link_and_destroy"]
    );
    // 3 -> 1: the decided 3 may become 1, whose arm frees the payload: the
    // two arms join to undecided.
    let across = format!(
        "void serve(struct op *o) {{ switch (o->type) {{ case 3: o->type = 1; break; default: break; }} }}\n{}",
        linking("c_op_new(3)")
    );
    let across = rows(&across);
    assert!(
        across["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        across["link_and_destroy"]
    );
    // A rewrite whose object is later set to NULL still reads the guarded
    // object (a null rebinding reaches no other object): 3 -> 1 joins.
    let nulled = format!(
        "void serve(struct op *o) {{ switch (o->type) {{ case 3: o->type = 1; o = NULL; break; default: break; }} }}\n{}",
        linking("c_op_new(3)")
    );
    let nulled = rows(&nulled);
    assert!(
        nulled["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        nulled["link_and_destroy"]
    );
    // A rewrite of a rebound object is unclassified: even a decided 1 joins.
    let rebound = format!(
        "void serve(struct op *o, struct op *p) {{ switch (o->type) {{ case 3: o = p; o->type = 4; break; default: break; }} }}\n{}",
        linking("c_op_new(1)")
    );
    let rebound = rows(&rebound);
    assert!(
        rebound["link_and_destroy"].refused_with("arm-undecided"),
        "{:?}",
        rebound["link_and_destroy"]
    );
}

#[test]
fn a_sub_object_write_keeps_and_a_whole_object_write_voids_the_state() {
    if !clang_available() {
        return;
    }
    // Copying a sub-object writes its members only: the decided 1 stands.
    let sub = format!(
        "void copy(struct op *a, struct op *b) {{ a->u = b->u; }}\n{}",
        linking("c_op_new(1)")
    );
    let sub = rows(&sub);
    assert!(
        sub["link_and_destroy"].safe(),
        "{:?}",
        sub["link_and_destroy"]
    );
    // A whole-object assignment may write the member and the discriminant:
    // both states are void, and the unit's closure (no store) refuses.
    let whole = format!(
        "void copy(struct op *a, struct op *b) {{ *a = *b; }}\n{}",
        linking("c_op_new(1)")
    );
    let whole = rows(&whole);
    assert!(
        whole["link_and_destroy"].refused_with("callback-member-open"),
        "{:?}",
        whole["link_and_destroy"]
    );
}

// ---------------------------------------------------------------------------
// Rules 13w and 14e: the member's per-path state
// ---------------------------------------------------------------------------

#[test]
fn a_registration_then_destroy_invokes_only_the_registered_candidate() {
    if !clang_available() {
        return;
    }
    let identity = "\
static int handler(void *rk, void *rkq, void *rko) { return 0; }
void make_and_destroy(void) { struct op *o = c_op_new_cb(2, handler); c_op_destroy(o); }";
    let identity = rows(identity);
    assert!(
        identity["make_and_destroy"].safe(),
        "{:?}",
        identity["make_and_destroy"]
    );
    // The registered candidate frees its first actual: effectful, refused,
    // and named; an identity candidate stored elsewhere in the unit is not
    // read in its place.
    let effectful = "\
static int handler(void *rk, void *rkq, void *rko) { return 0; }
static int freeing(void *rk, void *rkq, void *rko) { c_free(rk); return 0; }
void make_and_destroy(void) { struct op *o = c_op_new_cb(2, freeing); c_op_destroy(o); }";
    let effectful = rows(effectful);
    assert!(
        effectful["make_and_destroy"].refused_with("callback-effectful"),
        "{:?}",
        effectful["make_and_destroy"]
    );
}

/// A registered candidate defined after its caller is summarised first (the
/// derivation edge of rule 13w's per-path registration).
#[test]
fn a_registered_candidate_precedes_its_caller() {
    if !clang_available() {
        return;
    }
    let later = "\
static int handler(void *rk, void *rkq, void *rko);
void make_and_destroy(void) { struct op *o = c_op_new_cb(2, handler); c_op_destroy(o); }
static int handler(void *rk, void *rkq, void *rko) { return 0; }
void tail(void) { }";
    let later = rows(later);
    assert!(
        later["make_and_destroy"].safe(),
        "{:?}",
        later["make_and_destroy"]
    );
}

#[test]
fn an_absent_member_invokes_no_candidate() {
    if !clang_available() {
        return;
    }
    let absent = "void make_and_destroy(void) { struct op *o = c_op_new(2); c_op_destroy(o); }";
    let absent = rows(absent);
    assert!(
        absent["make_and_destroy"].safe(),
        "{:?}",
        absent["make_and_destroy"]
    );
}

#[test]
fn an_unclassified_store_voids_the_absent_state() {
    if !clang_available() {
        return;
    }
    // A store of a formal into an existing object's member.
    let other = "\
void set_any(struct op *o, void *f) { o->op_cb = f; }
void make_and_destroy(void) { struct op *o = c_op_new(2); c_op_destroy(o); }";
    let other = rows(other);
    assert!(
        other["make_and_destroy"].refused_with("callback-member-open"),
        "{:?}",
        other["make_and_destroy"]
    );
    // A null store into an existing object only removes a candidate: kept.
    let null = "\
void clear(struct op *o) { o->op_cb = NULL; }
void make_and_destroy(void) { struct op *o = c_op_new(2); c_op_destroy(o); }";
    let null = rows(null);
    assert!(
        null["make_and_destroy"].safe(),
        "{:?}",
        null["make_and_destroy"]
    );
    // A store into the member in the creating function: the state is open
    // there, and the unit's closure decides.
    let own = "\
static int freeing(void *rk, void *rkq, void *rko) { c_free(rk); return 0; }
void make_and_destroy(void) { struct op *o = c_op_new(2); o->op_cb = freeing; c_op_destroy(o); }";
    let own = rows(own);
    assert!(
        own["make_and_destroy"].refused_with("callback-effectful"),
        "{:?}",
        own["make_and_destroy"]
    );
}

#[test]
fn the_absent_state_stands_where_the_body_validates_it() {
    if !clang_available() {
        return;
    }
    let zeroed = "\
struct op *c_op_new(int type) { struct op *o; o = calloc(1, sizeof(*o)); o->type = type; return o; }
void make_and_destroy(void) { struct op *o = c_op_new(2); c_op_destroy(o); }";
    let zeroed = rows(zeroed);
    assert!(
        zeroed["make_and_destroy"].safe(),
        "{:?}",
        zeroed["make_and_destroy"]
    );
    // Not zeroed: the state is withdrawn in this unit, and the unit's
    // closure (no store) does not decide.
    let unzeroed = "\
struct op *c_op_new(int type) { struct op *o; o = malloc(sizeof(*o)); o->type = type; return o; }
void make_and_destroy(void) { struct op *o = c_op_new(2); c_op_destroy(o); }";
    let unzeroed = rows(unzeroed);
    assert!(
        unzeroed["make_and_destroy"].refused_with("callback-member-open"),
        "{:?}",
        unzeroed["make_and_destroy"]
    );
}
