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
struct op *c_op_req(int type);
struct item *c_item_make(int n);
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

// ---------------------------------------------------------------------------
// OUT-SLOT-MEMBER-THEN-FRESH ((a1)+(b)): the owned member, rule 96's move-out
// and rule 74l's delivers-or-leaves-unwritten mode
// ---------------------------------------------------------------------------
//
// `rd_kafka_assignment`/`rd_kafka_subscription` move an op's member into the
// caller's slot (`*out = rko->M; rko->M = NULL; rd_kafka_op_destroy(rko);`)
// and store a fresh list when that is null. `c_op_req` is the synthetic
// counterpart of `rd_kafka_op_req2`: a nullable result whose decided type 1
// leaves an owned payload (`resource_member_owned`). CHANGE PINS failed at the
// base (`docs/reports/out-slot-member-then-fresh/`).

fn kinds(source: &str, function: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        function,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = &summary.functions[function];
    (
        row.status,
        row.diagnostics.iter().map(|d| d.kind.clone()).collect(),
    )
}

fn unknown_alias(kinds: &[LegacyDiagnosticKind], name: &str) -> bool {
    kinds.iter().any(
        |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == name),
    )
}

fn leaks(kinds: &[LegacyDiagnosticKind]) -> bool {
    kinds
        .iter()
        .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
}

/// `c_op_req(type)` with its null test, then `BODY`.
fn requested(kind: i32, body: &str) -> String {
    format!(
        "void take(void) {{ struct op *rko; struct item *p; struct item *q; rko = c_op_req({kind}); \
         if (!rko) return; {body} }}"
    )
}

/// CHANGE PIN (rule 96, stated). The read the next statement nulls takes the
/// owned payload: releasing it is `Safe`, dropping it leaks.
#[test]
fn a_stated_owned_member_moves_out_to_the_reader() {
    if !clang_available() {
        return;
    }
    let moved = "p = rko->payload; rko->payload = 0; c_op_destroy(rko);";
    let (status, found) = kinds(&requested(1, &format!("{moved} c_free(p);")), "take");
    assert!(status == AnalysisStatus::Safe, "{status:?} {found:?}");
    let (status, found) = kinds(&requested(1, moved), "take");
    assert!(
        status == AnalysisStatus::Unsafe && leaks(&found),
        "{status:?} {found:?}"
    );
}

/// Rule 96's fallbacks: another decided type (no owned member), a second
/// move-out on the path (the state is spent), a read the next statement does
/// not null, an object handed to a call before the read, and a body that
/// stores into the member otherwise. Each keeps the read's own lowering, so
/// the release of the read value is never `Safe`.
#[test]
fn every_other_read_keeps_its_own_lowering() {
    if !clang_available() {
        return;
    }
    let moved = "p = rko->payload; rko->payload = 0;";
    for (kind, body, alias) in [
        (2, format!("{moved} c_op_destroy(rko); c_free(p);"), "p"),
        (
            1,
            format!(
                "{moved} q = rko->payload; rko->payload = 0; c_op_destroy(rko); c_free(p); c_free(q);"
            ),
            "q",
        ),
        (
            1,
            "p = rko->payload; c_op_destroy(rko); c_free(p);".to_owned(),
            "p",
        ),
        (
            1,
            format!("c_op_destroy2(rko); {moved} c_op_destroy(rko); c_free(p);"),
            "p",
        ),
        (
            1,
            format!("rko->payload = c_item_make(1); {moved} c_op_destroy(rko); c_free(p);"),
            "p",
        ),
    ] {
        let source = requested(kind, &body).replace(
            "void take(void)",
            "void c_op_destroy2(struct op *rko);\nvoid take(void)",
        );
        let (status, found) = kinds(&source, "take");
        assert!(
            status != AnalysisStatus::Safe && unknown_alias(&found, alias),
            "{body}: {status:?} {found:?}"
        );
    }
}

/// A member read out of an op whose declaration states no owned member
/// carries no custody, as at the base: a release of it is refused, and
/// dropping it discharges nothing because nothing was owed.
#[test]
fn an_ops_member_read_carries_no_custody() {
    if !clang_available() {
        return;
    }
    let moved = "struct op *rko; struct item *p; rko = c_op_new(1); p = rko->payload; \
                 rko->payload = 0; c_op_destroy(rko);";
    let (status, found) = kinds(&format!("void take(void) {{ {moved} c_free(p); }}"), "take");
    assert!(
        status == AnalysisStatus::Unknown && unknown_alias(&found, "p"),
        "{status:?} {found:?}"
    );
    let (status, found) = kinds(&format!("void take(void) {{ {moved} }}"), "take");
    assert!(status == AnalysisStatus::Safe, "{status:?} {found:?}");
}

/// CHANGE PIN (rule 96, linked). Reading back a member the frame itself
/// linked, then nulling it, hands the child to the reader: releasing it is
/// `Safe`, dropping it leaks (as at the base, a true leak of the fixture).
#[test]
fn a_moved_out_linked_member_goes_to_the_reader() {
    if !clang_available() {
        return;
    }
    let linked = "struct op *rko; struct item *it; struct item *p; it = c_item_new(n); \
                  if (!it) { return; } rko = c_op_new(1); rko->payload = it; p = rko->payload; \
                  rko->payload = 0; c_op_destroy(rko);";
    let (status, found) = kinds(
        &format!("void take(int n) {{ {linked} c_free(p); }}"),
        "take",
    );
    assert!(status == AnalysisStatus::Safe, "{status:?} {found:?}");
    let (status, found) = kinds(&format!("void take(int n) {{ {linked} }}"), "take");
    assert!(
        status == AnalysisStatus::Unsafe && leaks(&found),
        "{status:?} {found:?}"
    );
    // Without the null store the destroy's arm frees the child the reader
    // holds: no move-out, and the release is never `Safe`.
    let unnulled = "void take(int n) { struct op *rko; struct item *it; struct item *p; \
                    it = c_item_new(n); if (!it) { return; } rko = c_op_new(1); \
                    rko->payload = it; p = rko->payload; c_op_destroy(rko); c_free(p); }";
    let (status, found) = kinds(unnulled, "take");
    assert!(status != AnalysisStatus::Safe, "{status:?} {found:?}");
}

/// The callee of rule 74l's mode (b): early exits that leave the slot as the
/// caller had it, a status (`err`) that does not partition them, and the
/// moved member or a fresh object stored into the slot after the destroy.
const FILL: &str = "int fill(int c, struct item **out) { struct op *rko; struct item *p; int err;
 if (c) return -1;
 rko = c_op_req(1);
 if (!rko) return -2;
 err = rko->type;
 p = rko->payload;
 rko->payload = 0;
 c_op_destroy(rko);
 if (p) { *out = p; return err; }
 *out = c_item_make(0);
 return err; }";

fn subject(declaration: &str) -> String {
    format!("{FILL}\nvoid subject(void) {{ {declaration} int e; e = fill(0, &p); c_free(p); }}")
}

/// CHANGE PIN (rule 74l (b)). A caller whose slot is known null takes the
/// delivery, the moved member or the fresh object: releasing it is `Safe`,
/// dropping it leaks.
#[test]
fn a_known_null_slot_takes_the_delivery() {
    if !clang_available() {
        return;
    }
    let (status, found) = kinds(&subject("struct item *p = 0;"), "subject");
    assert!(status == AnalysisStatus::Safe, "{status:?} {found:?}");
    let dropped =
        format!("{FILL}\nvoid subject(void) {{ struct item *p = 0; int e; e = fill(0, &p); }}");
    let (status, found) = kinds(&dropped, "subject");
    assert!(
        status == AnalysisStatus::Unsafe && leaks(&found),
        "{status:?} {found:?}"
    );
}

/// CHANGE PIN (text). Every other caller is refused by name: a slot holding
/// the caller's live object (the callee would overwrite it), an
/// uninitialised one, a possibly-null one.
#[test]
fn a_slot_not_known_null_is_refused() {
    if !clang_available() {
        return;
    }
    for declaration in [
        "struct item *p = c_item_make(1);",
        "struct item *p;",
        "struct item *p = c_item_new(1);",
    ] {
        let (status, found) = kinds(&subject(declaration), "subject");
        assert!(
            status == AnalysisStatus::Unknown,
            "{declaration}: {status:?} {found:?}"
        );
        assert!(
            found.iter().any(|kind| matches!(
                kind,
                LegacyDiagnosticKind::Unsupported { description } if description
                    == "call to `fill`, whose summary delivers its slot or leaves it unwritten, \
                        with a slot actual not known null"
            )),
            "{declaration}: {found:?}"
        );
    }
}

/// The callee's own row keeps rule 74l's refusal: the summary is what its
/// callers read, not a certificate of the body (KEEP-OR-DELIVER-OWN-ROW).
#[test]
fn the_null_only_callees_own_row_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let (status, found) = kinds(FILL, "fill");
    assert!(status == AnalysisStatus::Unknown, "{status:?} {found:?}");
}

/// The upstream order: the member stored into the slot *before* the destroy,
/// which takes the op and so may write caller memory: rule 11r(8) poisons
/// the bound slot there, no summary validates, and the caller keeps the
/// uncontracted call. So does the `!*out && !err` test.
#[test]
fn a_slot_bound_before_the_destroy_stays_refused() {
    if !clang_available() {
        return;
    }
    const UPSTREAM: &str = "int fill(int c, struct item **out) { struct op *rko; int err;
 if (c) return -1;
 rko = c_op_req(1);
 if (!rko) return -2;
 err = rko->type;
 *out = rko->payload;
 rko->payload = 0;
 c_op_destroy(rko);
 if (!*out && !err) { *out = c_item_make(0); }
 return err; }";
    let caller = format!(
        "{UPSTREAM}\nvoid subject(void) {{ struct item *p = 0; int e; e = fill(0, &p); c_free(p); }}"
    );
    let (status, found) = kinds(&caller, "subject");
    assert!(status == AnalysisStatus::Unknown, "{status:?} {found:?}");
    assert!(
        found.iter().any(|kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { description }
                if description == "call to uncontracted function `fill`"
        )),
        "{found:?}"
    );
}

/// What the vocabulary still refuses: a result state on a nullable result
/// with no owned member, the liveness part on a nullable result, a member of
/// a result mapped to anything but `#absent` or a function, an owned member
/// with no discriminant or under another field, and a disposition whose
/// classes leave a returned value without a mode.
#[test]
fn the_vocabulary_refuses_every_other_statement() {
    let refusal = |clauses: &str, ret: &str| {
        load_resource_contract_sources(&BTreeMap::from([(
            "member_then_fresh.qv".to_owned(),
            format!(
                "module contract;\nextern fn c_op_req(type: i32) -> ptr<mut, u8>\ncontract {{\n    \
                 requires resource_contract_v1;\n    ensures normal resource_borrowed(type);\n    \
                 {clauses}\n    ensures normal {ret};\n}};\n"
            ),
        )]))
        .expect_err("the statement is refused")
    };
    let nullable = refusal(
        "ensures normal resource_discriminant_set(result, op.type, type, 0);",
        "resource_return_owned(nullable)",
    );
    assert!(nullable.contains("quant-live-unbacked"), "{nullable}");
    let live = refusal(
        "ensures normal resource_discriminant_set(result, op.type, type, 0);\n    \
         ensures normal resource_member_owned(result, op.payload) when type == 1;\n    \
         quant live #result;",
        "resource_return_owned(nullable)",
    );
    assert!(live.contains("quant-live-unbacked"), "{live}");
    for value in ["#moved", "#kept", "#released"] {
        let member = refusal(
            &format!("quant live #result: op.payload |-> {value};"),
            "resource_return_owned(nonnull)",
        );
        assert!(member.contains("quant-live-value"), "{value}: {member}");
    }
    let undecided = refusal(
        "ensures normal resource_member_owned(result, op.payload) when type == 1;",
        "resource_return_owned(nullable)",
    );
    assert!(undecided.contains("decided discriminant"), "{undecided}");
    let other_field = refusal(
        "ensures normal resource_discriminant_set(result, op.type, type, 0);\n    \
         ensures normal resource_member_owned(result, op.payload) when kind == 1;",
        "resource_return_owned(nullable)",
    );
    assert!(other_field.contains("guard names"), "{other_field}");
    let partial = load_resource_contract_sources(&BTreeMap::from([(
        "member_then_fresh.qv".to_owned(),
        "module contract;\nextern fn c_fill(out: ptr<mut, ptr<mut, u8>>) -> i32\ncontract {\n    \
         requires resource_contract_v1;\n    \
         ensures normal resource_on_return(eq, 0, resource_out(out, nonnull));\n    \
         ensures normal resource_return_none();\n};\n"
            .to_owned(),
    )]))
    .expect_err("an uncovered class is refused");
    assert!(
        partial.contains("do not cover every returned value"),
        "{partial}"
    );
}
