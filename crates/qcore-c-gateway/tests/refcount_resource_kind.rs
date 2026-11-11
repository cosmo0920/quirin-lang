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

//! Card REFCOUNT-RESOURCE-KIND (gateway rules 13b, 13c): the brief's Rust
//! controls on route G, over the synthetic `rc_*` surface
//! (`fixtures/contracts/refcount.qv`). A `Safe` here is `Safe` over those
//! trusted test assumptions and promotes nothing.
//!
//! The controls: a balanced keep and drop; a missing drop; a double drop; a
//! keep returned to the caller; a keep stored into a tracked owner; a
//! statistics counter beside a reference count on the same atomic primitive;
//! a use after a possibly-final drop. Then: the neutral reading is not
//! admitted, a withdrawn declaration keeps refusing, and the shared vectors
//! bind `QCore.ReferenceCount`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_c_gateway::reference_count::{
    Binding, DropDecision, Held, KeepDecision, REFERENCE_TRACE_VECTORS, REFERENCE_VECTORS,
    RefEvent, RefOutcome, decide_drop, decide_keep, ref_replay,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
void *rc_alloc(size_t n); void rc_free(void *p); void rc_use(void *p); void *rc_new(size_t n);
void *rc_keep(void *p); void rc_keep_void(void *p); void rc_drop(void *p);
int rc_atomic_add(int *ra, int v); void unknown_call(void *p);
struct obj { int refcnt; int stats; };
struct holder { struct obj *ref; };
static struct obj *obj_keep(struct obj *o) { rc_atomic_add(&o->refcnt, 1); return o; }
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "refcount.qcore".to_owned(),
        include_str!("fixtures/contracts/refcount.qv").to_owned(),
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
            eprintln!("skipping Clang-dependent REFCOUNT-RESOURCE-KIND test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    kinds: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    /// Not `Safe`, and no violation candidate: a refusal, never an accusation.
    fn refused(&self) -> bool {
        self.status == AnalysisStatus::Unknown
    }
    fn mentions(&self, key: &str) -> bool {
        self.kinds.iter().any(|kind| kind.contains(key))
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
fn rows_with(source: &str, contracts: &ResourceContracts) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{source}\n");
    let entry = source
        .split('(')
        .next()
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
        contracts,
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
                    kinds: row
                        .diagnostics
                        .iter()
                        .map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                description.clone()
                            }
                            other => format!("{other:?}"),
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

fn one(source: &str, name: &str) -> Row {
    let mut rows = rows_with(source, &contracts());
    rows.remove(name)
        .unwrap_or_else(|| panic!("no row `{name}` in {rows:?}"))
}

// ---------------------------------------------------------------------------
// The brief's controls
// ---------------------------------------------------------------------------

#[test]
fn a_balanced_keep_and_drop_is_safe() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void *p) { rc_keep(p); rc_use(p); rc_drop(p); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // Through the kept result, a void keep, and two references held at once.
    let row = one(
        "void f(void *p) { void *q = rc_keep(p); rc_use(q); rc_drop(q); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    let row = one("void f(void *p) { rc_keep_void(p); rc_drop(p); }", "f");
    assert!(row.safe(), "{row:?}");
    let row = one(
        "void f(void *p) { rc_keep(p); rc_keep(p); rc_drop(p); rc_use(p); rc_drop(p); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // Balanced on both arms of a branch.
    let row = one(
        "void f(void *p, int c) { rc_keep(p); if (c) { rc_use(p); } rc_drop(p); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

#[test]
fn a_missing_drop_is_a_leak_candidate_and_never_safe() {
    if !clang_available() {
        return;
    }
    let row = one("void f(void *p) { rc_keep(p); rc_use(p); }", "f");
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    // Two keeps, one drop.
    let row = one(
        "void f(void *p) { rc_keep(p); rc_keep(p); rc_drop(p); }",
        "f",
    );
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    // A drop on one arm only.
    let row = one(
        "void f(void *p, int c) { rc_keep(p); if (c) rc_drop(p); }",
        "f",
    );
    assert!(!row.safe(), "{row:?}");
    // On a path that also refuses, the missing drop is refused, not accused.
    // (One statement per line: a refused statement withdraws every identity
    // its line reads, as for any other resource.)
    let row = one(
        "void f(void *p, void *q) {\n rc_keep(p);\n rc_drop(q);\n}",
        "f",
    );
    assert!(row.refused() && !row.mentions("Leak"), "{row:?}");
    assert!(row.mentions("reference-unbalanced"), "{row:?}");
    // An uncontracted call may have discharged the reference, as it may a
    // heap allocation: refused, not accused.
    let row = one(
        "void f(void *p, void *q) { rc_keep(p); unknown_call(q); }",
        "f",
    );
    assert!(row.refused() && !row.mentions("Leak"), "{row:?}");
    // A refused statement that may have handed the reference on withdraws
    // it: refused, not accused.
    let row = one("void f(void *p) { rc_keep(p); unknown_call(p); }", "f");
    assert!(row.refused() && !row.mentions("Leak"), "{row:?}");
}

#[test]
fn a_double_drop_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void *p) { rc_keep(p); rc_drop(p); rc_drop(p); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-dropped"),
        "{row:?}"
    );
    // A drop of a reference the function never took consumes the caller's:
    // refused, never Safe.
    let row = one("void f(void *p) { rc_drop(p); }", "f");
    assert!(
        row.refused() && row.mentions("reference-drop-unheld"),
        "{row:?}"
    );
    // A drop of a member read: refused.
    let row = one("void f(struct holder *h) { rc_drop(h->ref); }", "f");
    assert!(row.refused(), "{row:?}");
}

#[test]
fn a_keep_returned_to_the_caller_is_balanced() {
    if !clang_available() {
        return;
    }
    let row = one("void *f(void *p) { return rc_keep(p); }", "f");
    assert!(row.safe(), "{row:?}");
    let row = one("void *f(void *p) { void *q = rc_keep(p); return q; }", "f");
    assert!(row.safe(), "{row:?}");
    // Through a pointer cast of the actual (`rd_kafka_toppar_list_copy`).
    let row = one(
        "void *f(const void *elem) { return rc_keep((struct obj *)elem); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // Two keeps, one returned: one still owed.
    let row = one("void *f(void *p) { rc_keep(p); return rc_keep(p); }", "f");
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
}

#[test]
fn a_keep_stored_into_a_tracked_owner_is_handed_off() {
    if !clang_available() {
        return;
    }
    let row = one(
        "struct holder *f(struct obj *p) { struct holder *o; struct obj *r; \
         o = rc_alloc(8); r = rc_keep(p); o->ref = r; return o; }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // An owner freed while it holds the reference: the reference is still
    // owed (a field link hands nothing off unless the owner escapes).
    let row = one(
        "void f(struct obj *p) { struct holder *o; struct obj *r; \
         o = rc_alloc(8); r = rc_keep(p); o->ref = r; rc_free(o); }",
        "f",
    );
    assert!(!row.safe(), "{row:?}");
    // A drop of the reference the owner holds: refused.
    let row = one(
        "struct holder *f(struct obj *p) { struct holder *o; struct obj *r; \
         o = rc_alloc(8); r = rc_keep(p); o->ref = r; rc_drop(r); return o; }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-drop-unheld"),
        "{row:?}"
    );
    // A keep in a loop, dropped in the same iteration: balanced.
    let row = one(
        "void f(struct obj **v, int n) {\n int i;\n for (i = 0; i < n; i++) {\n \
         struct obj *o = v[i];\n rc_keep(o);\n rc_use(o);\n rc_drop(o);\n }\n}",
        "f",
    );
    assert!(!row.mentions("Leak"), "{row:?}");
    // Kept straight into a member: a place this route does not track.
    let row = one(
        "void f(struct holder *h, void *p) { h->ref = rc_keep(p); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
}

#[test]
fn a_statistics_counter_is_never_read_as_a_reference() {
    if !clang_available() {
        return;
    }
    // The declared keep wrapper over the primitive is a reference.
    let row = one("void f(struct obj *o) { obj_keep(o); rc_drop(o); }", "f");
    assert!(row.safe(), "{row:?}");
    let row = one("void f(struct obj *o) { obj_keep(o); }", "f");
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    // The same primitive on a statistics counter: never a keep (no leak), and
    // never neutral (not Safe): the primitive is not declared.
    let row = one(
        "void f(struct obj *o) { rc_atomic_add(&o->stats, 1); }",
        "f",
    );
    assert!(row.refused() && row.mentions("rc_atomic_add"), "{row:?}");
    // A reference keep spelled through the primitive is not read either.
    let row = one(
        "void f(struct obj *o) { rc_atomic_add(&o->refcnt, 1); }",
        "f",
    );
    assert!(row.refused() && !row.mentions("Leak"), "{row:?}");
    // Both beside each other: the counter leaves the reference's balance alone
    // and the function is refused for the counter, not accused.
    let row = one(
        "void f(struct obj *o) { obj_keep(o); rc_atomic_add(&o->stats, 1); rc_drop(o); }",
        "f",
    );
    assert!(row.refused() && !row.mentions("Leak"), "{row:?}");
}

#[test]
fn a_use_after_a_possibly_final_drop_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void *p) { void *q = rc_keep(p); rc_drop(q); rc_use(p); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-dropped"),
        "{row:?}"
    );
    let row = one(
        "void f(void *p) { rc_keep(p); void *a = p; rc_drop(p); rc_use(a); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-dropped"),
        "{row:?}"
    );
    // A keep after the last drop: refused too.
    let row = one(
        "void f(void *p) { rc_keep(p); rc_drop(p); rc_keep(p); rc_drop(p); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
}

#[test]
fn a_returned_reference_is_owned_by_the_caller() {
    if !clang_available() {
        return;
    }
    // A reference-returning call: the caller owes one drop.
    let row = one(
        "void f(void) { void *r = rc_new(1); if (!r) return; rc_use(r); rc_drop(r); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    let row = one(
        "void f(void) { void *r = rc_new(1); if (!r) return; rc_use(r); }",
        "f",
    );
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    // Kept once more: two drops balance it.
    let row = one(
        "void f(void) { void *r = rc_new(1); if (!r) return; rc_keep(r); rc_drop(r); \
         rc_use(r); rc_drop(r); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // Handed on to the caller.
    let row = one(
        "void *f(void) { void *r = rc_new(1); if (!r) return 0; return r; }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

#[test]
fn an_over_drop_of_an_acquired_reference_is_a_violation_candidate() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void) {\n void *r = rc_new(1);\n if (!r) return;\n rc_drop(r);\n rc_drop(r);\n}",
        "f",
    );
    assert!(
        row.status == AnalysisStatus::Unsafe && row.mentions("DoubleRelease"),
        "{row:?}"
    );
    // On a path that already refused something it is refused, not accused.
    let row = one(
        "void f(void *q) {\n void *r = rc_new(1);\n if (!r) return;\n rc_drop(q);\n \
         rc_drop(r);\n rc_drop(r);\n}",
        "f",
    );
    assert!(row.refused() && !row.mentions("DoubleRelease"), "{row:?}");
    // A use after the last drop: refused, never accused.
    let row = one(
        "void f(void) {\n void *r = rc_new(1);\n if (!r) return;\n rc_drop(r);\n rc_use(r);\n}",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-dropped"),
        "{row:?}"
    );
}

// ---------------------------------------------------------------------------
// The neutral reading and the body check (rule 13c)
// ---------------------------------------------------------------------------

#[test]
fn a_reference_declaration_is_withheld_from_the_function_map() {
    let contracts = contracts();
    // A reference-returning call is an ordinary owned result, in the map.
    assert!(contracts.contract("rc_new").is_some());
    for name in ["rc_keep", "rc_keep_void", "rc_drop", "obj_keep"] {
        assert!(contracts.contract(name).is_none(), "{name}");
        assert!(
            contracts
                .retaining_contract(name)
                .is_some_and(|declared| !declared.references.is_empty()),
            "{name}"
        );
    }
}

#[test]
fn a_body_that_does_not_validate_withdraws_the_declaration_in_its_unit() {
    if !clang_available() {
        return;
    }
    // `rc_keep` defined here stores its argument: withdrawn, every call
    // refuses, and no inference reads the body in the declaration's place.
    let row = one(
        "void *stash; void *rc_keep(void *p) { stash = p; return p; } \
         void f(void *p) { rc_keep(p); rc_drop(p); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("reference-withdrawn"),
        "{row:?}"
    );
    let row = one(
        "void *stash; void *rc_keep(void *p) { stash = p; return p; } \
         void f(void *p) { rc_keep(p); }",
        "f",
    );
    assert!(row.refused() && !row.safe(), "{row:?}");
}

#[test]
fn a_heap_release_of_a_reference_is_a_kind_mismatch() {
    if !clang_available() {
        return;
    }
    let row = one("void f(void *p) { void *q = rc_keep(p); rc_free(q); }", "f");
    assert!(!row.safe(), "{row:?}");
}

// ---------------------------------------------------------------------------
// The shared vectors (`QCore.ReferenceCount`)
// ---------------------------------------------------------------------------

const LEAN: &str = include_str!("../../../proof/QCore/ReferenceCount.lean");

/// The rows of the Lean list `header` opens, one string per row.
fn lean_rows(header: &str) -> Vec<String> {
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the Lean list closes");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

fn lean_binding(binding: Binding) -> String {
    match binding {
        Binding::Unbound => ".unbound".to_owned(),
        Binding::Family {
            held: Held::Known(held),
            handed,
        } => format!("(.family (.known {held}) {handed})"),
        Binding::Family {
            held: Held::Undecided,
            handed,
        } => format!("(.family .undecided {handed})"),
        Binding::Invalid => ".invalid".to_owned(),
        Binding::Freed => ".freed".to_owned(),
        Binding::Other => ".other".to_owned(),
    }
}

fn lean_reason(reason: &str) -> &'static str {
    match reason {
        "reference-dropped" => ".dropped",
        "reference-drop-unheld" => ".unheld",
        "reference-undecided" => ".undecided",
        other => panic!("no Lean reason for {other}"),
    }
}

fn lean_keep(decision: KeepDecision) -> String {
    match decision {
        KeepDecision::Mint => ".mint".to_owned(),
        KeepDecision::Add => ".add".to_owned(),
        KeepDecision::Refuse(reason) => format!("(.refuse {})", lean_reason(reason)),
    }
}

fn lean_drop(decision: DropDecision) -> String {
    match decision {
        DropDecision::Decrement => ".decrement".to_owned(),
        DropDecision::Final => ".final".to_owned(),
        DropDecision::OverDrop => ".overDrop".to_owned(),
        DropDecision::Refuse(reason) => format!("(.refuse {})", lean_reason(reason)),
    }
}

fn lean_event(event: RefEvent) -> String {
    match event {
        RefEvent::Acquire(n) => format!(".acquire {n}"),
        RefEvent::Keep(n) => format!(".keep {n}"),
        RefEvent::KeepTo(d, n) => format!(".keepTo {d} {n}"),
        RefEvent::Alias(d, n) => format!(".alias {d} {n}"),
        RefEvent::Drop(n) => format!(".drop {n}"),
        RefEvent::Use(n) => format!(".use {n}"),
        RefEvent::Handoff(n) => format!(".handoff {n}"),
        RefEvent::Counter(n) => format!(".counter {n}"),
        RefEvent::Refuse => ".refuse".to_owned(),
        RefEvent::Withdrawn(n) => format!(".withdrawn {n}"),
    }
}

fn lean_outcome(outcome: RefOutcome) -> &'static str {
    match outcome {
        RefOutcome::Safe => ".safe",
        RefOutcome::Leak => ".leak",
        RefOutcome::Refused => ".refused",
        RefOutcome::Violation => ".violation",
    }
}

#[test]
fn the_decision_vectors_are_the_lean_table() {
    let rows =
        lean_rows("def referenceVectors : List (Binding × KeepDecision × DropDecision) := [");
    let rust = REFERENCE_VECTORS
        .iter()
        .map(|(binding, keep, drop)| {
            assert_eq!(decide_keep(*binding), *keep);
            assert_eq!(decide_drop(*binding), *drop);
            format!(
                "({}, {}, {})",
                lean_binding(*binding),
                lean_keep(*keep),
                lean_drop(*drop)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_trace_vectors_are_the_lean_table() {
    let rows = lean_rows("def referenceTraceVectors : List (List Event × Outcome) := [");
    let rust = REFERENCE_TRACE_VECTORS
        .iter()
        .map(|(path, outcome)| {
            assert_eq!(ref_replay(path).outcome(), *outcome, "{path:?}");
            format!(
                "([{}], {})",
                path.iter()
                    .map(|event| lean_event(*event))
                    .collect::<Vec<_>>()
                    .join(", "),
                lean_outcome(*outcome)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}
