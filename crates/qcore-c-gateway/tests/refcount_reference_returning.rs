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

//! Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (gateway rules 13d, 13e):
//! route G's controls over the synthetic `rr_*` surface
//! (`fixtures/contracts/refcount_results.qv`). A `Safe` here is `Safe` over
//! those trusted test assumptions and promotes nothing.
//!
//! The controls: a returned reference balanced, leaked and over-dropped; a
//! result holder dropped or released first (`Safe`), returned or stored
//! (`retention-undecided`), handed a member it cannot name (refused at the
//! call), and its retained object released while held; a predefined
//! identifier retained (static storage); rule 13d's body check standing and
//! withdrawn; a declared drop wrapper taking over its caller's reference (the
//! transfer), after which a drop is an over-drop.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
void *rr_alloc(size_t n); void rr_free(void *p); void rr_use(void *p);
void *rr_lookup(int k); void *rr_make(void *owner, const char *name);
void *rr_handle(void *owner); void rr_handle_destroy(void *h);
void *rr_keep(void *p); void rr_drop(void *p);
void *rr_get(int k); void *rr_bad(void *p); void rr_release(void *p);
struct holder { void *owner; void *slot; };
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "refcount_results.qcore".to_owned(),
        include_str!("fixtures/contracts/refcount_results.qv").to_owned(),
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
            eprintln!("skipping Clang-dependent REFCOUNT-REFERENCE-RETURNING test: {reason}");
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

/// Every row of the unit `PRELUDE + source`, analysed from `entry`.
fn unit_rows(source: &str, entry: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
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
    let mut rows = unit_rows(source, name);
    rows.remove(name)
        .unwrap_or_else(|| panic!("no row `{name}` in {rows:?}"))
}

// ---------------------------------------------------------------------------
// Returned references (rule 13b's acquired family, over the declared lookups)
// ---------------------------------------------------------------------------

#[test]
fn a_returned_reference_is_the_callers_to_drop() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(int k) { void *p = rr_lookup(k); if (!p) return; rr_use(p); rr_drop(p); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    let row = one(
        "void f(int k) { void *p = rr_lookup(k); if (!p) return; rr_use(p); }",
        "f",
    );
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    let row = one(
        "void f(int k) {\n void *p = rr_lookup(k);\n if (!p) return;\n rr_drop(p);\n rr_drop(p);\n}",
        "f",
    );
    assert!(row.mentions("DoubleRelease"), "{row:?}");
    // Handed on to the caller.
    let row = one(
        "void *f(int k) { void *p = rr_lookup(k); if (!p) return 0; return p; }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

// ---------------------------------------------------------------------------
// Result holders (rule 13e)
// ---------------------------------------------------------------------------

#[test]
fn a_result_holder_dropped_or_released_first_is_safe() {
    if !clang_available() {
        return;
    }
    // The caller's object outlives the queue: the last reference this
    // function holds on it is dropped before the function returns.
    let row = one(
        "void f(void *owner) { void *q = rr_make(owner, \"name\"); rr_use(q); rr_drop(q); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // A heap handle released first.
    let row = one(
        "void f(void *owner) { void *h = rr_handle(owner); rr_use(h); rr_handle_destroy(h); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

#[test]
fn a_result_holder_that_outlives_the_frame_is_undecided_never_safe() {
    if !clang_available() {
        return;
    }
    // Returned: the caller of this function decides the handle's lifetime.
    let row = one(
        "void *f(void *owner) { void *h = rr_handle(owner); return h; }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("retention-undecided"),
        "{row:?}"
    );
    let row = one("void *f(void *owner) { return rr_handle(owner); }", "f");
    assert!(row.refused(), "{row:?}");
    // A queue reference handed on to the caller still holds `owner`.
    let row = one(
        "void *f(void *owner) { void *q = rr_make(owner, \"name\"); return q; }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("retention-undecided"),
        "{row:?}"
    );
    // An extra reference kept: the drop is not this function's last.
    let row = one(
        "void f(void *owner) {\n void *q = rr_make(owner, \"name\");\n rr_keep(q);\n rr_drop(q);\n}",
        "f",
    );
    assert!(!row.safe(), "{row:?}");
}

#[test]
fn a_retained_actual_the_route_cannot_name_refuses_at_the_call() {
    if !clang_available() {
        return;
    }
    // The address of a frame object: not a name rule 12t reads. (A plain
    // local copy, `void *alias = owner;`, and a pointer cast of the formal are
    // read through to `owner`, and are decided like the formal.)
    let row = one(
        "void f(void) { struct holder h; h.owner = 0; void *q = rr_make(&h, \"name\"); rr_drop(q); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("retention-undecided"),
        "{row:?}"
    );
    // A member actual: refused (the access gate reads the member first), and
    // the result is then no acquired family, so its drop is unheld too.
    let row = one(
        "void f(struct holder *o) { void *q = rr_make(o->owner, \"name\"); rr_drop(q); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
    // The result into a place the route does not track.
    let row = one(
        "void f(struct holder *o, void *owner) { o->slot = rr_handle(owner); }",
        "f",
    );
    assert!(!row.safe(), "{row:?}");
}

#[test]
fn a_retained_object_released_while_the_handle_holds_it_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void) {\n void *owner = rr_alloc(8);\n void *h = rr_handle(owner);\n rr_free(owner);\n rr_handle_destroy(h);\n}",
        "f",
    );
    assert!(
        row.refused() && row.mentions("retention-released-while-held"),
        "{row:?}"
    );
    // Released after the handle: Safe.
    let row = one(
        "void f(void) {\n void *owner = rr_alloc(8);\n void *h = rr_handle(owner);\n rr_handle_destroy(h);\n rr_free(owner);\n}",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

#[test]
fn a_predefined_identifier_is_static_storage() {
    if !clang_available() {
        return;
    }
    for name in ["__func__", "__FUNCTION__", "__PRETTY_FUNCTION__"] {
        let row = one(
            &format!("void f(void *owner) {{ void *q = rr_make(owner, {name}); rr_drop(q); }}"),
            "f",
        );
        assert!(row.safe(), "{name}: {row:?}");
    }
    // A frame buffer is not.
    let row = one(
        "void f(void *owner) { char buf[8]; buf[0] = 0; void *q = rr_make(owner, buf); rr_drop(q); }",
        "f",
    );
    assert!(!row.safe(), "{row:?}");
}

// ---------------------------------------------------------------------------
// Rule 13d's body check, and the transfer
// ---------------------------------------------------------------------------

#[test]
fn a_returned_reference_whose_body_validates_stands_in_its_unit() {
    if !clang_available() {
        return;
    }
    let rows = unit_rows(
        "void *rr_get(int k) { void *p = rr_lookup(k); return p; }\n\
         void g(int k) { void *p = rr_get(k); if (!p) return; rr_use(p); rr_drop(p); }",
        "g",
    );
    assert!(rows["g"].safe(), "{rows:?}");
}

#[test]
fn a_returned_reference_whose_body_does_not_validate_is_withdrawn() {
    if !clang_available() {
        return;
    }
    // `rr_bad` hands back its argument: no reference is taken.
    let rows = unit_rows(
        "void *rr_bad(void *p) { return p; }\n\
         void g(void *p) { void *q = rr_bad(p); if (!q) return; rr_drop(q); }",
        "g",
    );
    assert!(
        rows["g"].refused() && rows["g"].mentions("reference-withdrawn"),
        "{rows:?}"
    );
}

#[test]
fn a_declared_drop_takes_over_the_callers_reference() {
    if !clang_available() {
        return;
    }
    // `rr_release` forwards to a declared drop (rule 13c validates it): the
    // caller's reference leaves its family.
    let rows = unit_rows(
        "void rr_release(void *p) { if (!p) return; rr_drop(p); }\n\
         void g(int k) { void *p = rr_lookup(k); if (!p) return; rr_release(p); }",
        "g",
    );
    assert!(rows["g"].safe(), "{rows:?}");
    // A drop after the transfer is an over-drop.
    let rows = unit_rows(
        "void rr_release(void *p) { if (!p) return; rr_drop(p); }\n\
         void g(int k) {\n void *p = rr_lookup(k);\n if (!p) return;\n rr_release(p);\n rr_drop(p);\n}",
        "g",
    );
    assert!(rows["g"].mentions("DoubleRelease"), "{rows:?}");
}
