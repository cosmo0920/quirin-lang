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

//! Card SLOT-MOVE-AND-INTERIOR-BORROW (gateway rules 12x, 12y, 12z, 13a): the
//! brief's Rust controls on route G, over the synthetic `sm_*` surface
//! (`fixtures/contracts/slot_move_borrow.qv`). A `Safe` here is `Safe` over
//! those trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
void *sm_alloc(size_t n); void sm_free(void *p); size_t sm_len(const char *s);
void sm_touch(void *p);
void sm_cat(char **slot, const char *text);
void *sm_child(void *parent, const char *key);
void *sm_private(void *parent);
size_t sm_reader(void *slice, char **out);
__INT64_TYPE__ sm_strtol(const char *nptr, char **end, int base);
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "slot_move_borrow.qcore".to_owned(),
        include_str!("fixtures/contracts/slot_move_borrow.qv").to_owned(),
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
            eprintln!("skipping Clang-dependent SLOT-MOVE-AND-INTERIOR-BORROW test: {reason}");
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
fn rows(source: &str) -> BTreeMap<String, Row> {
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
    let mut rows = rows(source);
    rows.remove(name)
        .unwrap_or_else(|| panic!("no row `{name}` in {rows:?}"))
}

// ---------------------------------------------------------------------------
// Class 2: slot move (rule 12x)
// ---------------------------------------------------------------------------

#[test]
fn the_slot_keeps_exactly_one_object_across_a_move() {
    if !clang_available() {
        return;
    }
    // One release of the slot discharges it: the slot always owns one object.
    let row = one(
        "void f(void) { char *b = sm_alloc(8); sm_cat(&b, \"x\"); sm_len(b); sm_free(b); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // The failure arm: the original stays in the slot, so it is still owed.
    let row = one(
        "void f(void) { char *b = sm_alloc(8); sm_cat(&b, \"x\"); }",
        "f",
    );
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
}

#[test]
fn a_use_of_the_old_alias_after_a_slot_move_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void) { char *b = sm_alloc(8); char *old = b; sm_cat(&b, \"x\"); \
         sm_len(old); sm_free(b); }",
        "f",
    );
    assert!(row.refused() && row.mentions("slot-move-alias"), "{row:?}");
    // A release through the old alias is refused, never a release.
    let row = one(
        "void f(void) { char *b = sm_alloc(8); char *old = b; sm_cat(&b, \"x\"); \
         sm_free(old); sm_free(b); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
    assert!(!row.mentions("DoubleRelease"), "{row:?}");
}

#[test]
fn a_caller_slot_passed_through_is_safe_and_anything_else_is_undecided() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void g(char **buf) { sm_cat(buf, \"x\"); sm_cat(buf, \"y\"); }",
        "g",
    );
    assert!(row.safe(), "{row:?}");
    // The body reads the slot's object: it holds an alias the move invalidates.
    let row = one(
        "void g(char **buf) { char *p = *buf; sm_cat(buf, \"x\"); sm_len(p); }",
        "g",
    );
    assert!(!row.safe(), "{row:?}");
    // A slot holding no tracked identity: undecided.
    let row = one("void g(char *s) { sm_cat(&s, \"x\"); }", "g");
    assert!(
        row.refused() && row.mentions("slot-move-undecided"),
        "{row:?}"
    );
}

// ---------------------------------------------------------------------------
// Class 8: interior borrow (rule 12y)
// ---------------------------------------------------------------------------

#[test]
fn an_interior_use_after_the_parent_release_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void) { char *p = sm_alloc(8); char *c = sm_private(p); sm_free(p); sm_len(c); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("interior-borrow-ended"),
        "{row:?}"
    );
}

#[test]
fn an_interior_use_after_an_invalidating_mutation_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void *p) { char *c = sm_private(p); sm_touch(p); sm_len(c); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("interior-borrow-ended"),
        "{row:?}"
    );
    // A write through one borrow ends its sibling.
    let row = one(
        "void f(void *p) { char *a = sm_private(p); char *b = sm_child(p, \"k\"); \
         if (b) { sm_touch(b); sm_len(a); } }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
}

#[test]
fn a_release_of_an_interior_pointer_is_refused() {
    if !clang_available() {
        return;
    }
    let row = one(
        "void f(void *p) { char *c = sm_private(p); sm_free(c); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
    assert!(!row.mentions("InvalidRelease"), "{row:?}");
    let row = one(
        "void f(void) { char *p = sm_alloc(8); char *c = sm_private(p); sm_free(c); sm_free(p); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
}

#[test]
fn a_lazily_created_child_is_owned_by_the_parent() {
    if !clang_available() {
        return;
    }
    // No obligation for the caller; a write through it reaches the parent.
    let row = one(
        "void f(void *p) { char *c = sm_private(p); sm_touch(c); sm_len(c); *c = 0; }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    let row = one(
        "void f(void) { char *p = sm_alloc(8); char *c = sm_private(p); sm_len(c); sm_free(p); }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
}

#[test]
fn a_forwarded_borrow_keeps_its_ending_rules() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void *w(void *p) { return sm_private(p); }\n\
         void *wc(const void *p) { return sm_private((void *)p); }\n\
         void x(void *p) { char *c = w(p); sm_len(c); }\n\
         void y(void *p) { char *c = w(p); sm_touch(p); sm_len(c); }",
    );
    assert!(rows["w"].safe(), "{:?}", rows["w"]);
    assert!(rows["wc"].safe(), "{:?}", rows["wc"]);
    assert!(rows["x"].safe(), "{:?}", rows["x"]);
    assert!(rows["y"].refused(), "{:?}", rows["y"]);
}

// ---------------------------------------------------------------------------
// The derived out slot (rule 12z)
// ---------------------------------------------------------------------------

#[test]
fn a_derived_out_slot_owns_nothing() {
    if !clang_available() {
        return;
    }
    let row = one(
        "size_t f(void *slice) { char *q; size_t n = sm_reader(slice, &q); return n; }",
        "f",
    );
    assert!(row.safe(), "{row:?}");
    // Its value released: refused.
    let row = one(
        "void f(void *slice) { char *q; sm_reader(slice, &q); sm_free(q); }",
        "f",
    );
    assert!(row.refused(), "{row:?}");
    // The slot would overwrite a tracked identity: refused.
    let row = one(
        "void f(void *slice) { char *q = sm_alloc(1); sm_reader(slice, &q); sm_free(q); }",
        "f",
    );
    assert!(
        row.refused() && row.mentions("out-derived-undecided"),
        "{row:?}"
    );
    // A caller's slot passed through, and a NULL slot under `nullable`.
    let rows = rows(
        "size_t r(void *slice, char **p) { return sm_reader(slice, p); }\n\
         __INT64_TYPE__ s(const char *text) { return sm_strtol(text, 0, 10); }\n\
         __INT64_TYPE__ t(const char *text) { char *end; __INT64_TYPE__ v = sm_strtol(text, &end, 10); return v; }",
    );
    assert!(rows["r"].safe(), "{:?}", rows["r"]);
    assert!(rows["s"].safe(), "{:?}", rows["s"]);
    assert!(rows["t"].safe(), "{:?}", rows["t"]);
}

// ---------------------------------------------------------------------------
// Body checks (rule 13a)
// ---------------------------------------------------------------------------

#[test]
fn a_declaration_its_own_body_does_not_validate_is_withdrawn_in_that_unit() {
    if !clang_available() {
        return;
    }
    // A slot-moving body that forwards its slot validates; one that stores
    // into the slot does not, and its callers then see an uncontracted callee.
    let rows = rows(
        "void sm_cat(char **slot, const char *text) { *slot = 0; sm_len(text); }\n\
         void caller(void) { char *b = sm_alloc(8); sm_cat(&b, \"x\"); sm_free(b); }",
    );
    assert!(
        rows["caller"].mentions("call to uncontracted function `sm_cat`"),
        "{:?}",
        rows["caller"]
    );
    let rows = rows_of_borrowed_bodies();
    assert!(
        rows.0.safe(),
        "a child reached from the parent: {:?}",
        rows.0
    );
    // The withdrawn declaration no longer governs: the body's own summary
    // (a fresh owned object) does, and the caller that drops it leaks.
    assert!(
        !rows.1.safe() && rows.1.mentions("Leak"),
        "a fresh object returned as a borrow: {:?}",
        rows.1
    );
}

fn rows_of_borrowed_bodies() -> (Row, Row) {
    let good = one(
        "struct node { struct node *child; struct node *next; void *priv; };\n\
         void *sm_private(void *parent) { struct node *n = parent; void *p; \
         if (!(p = n->priv)) { p = sm_alloc(4); n->priv = p; } return p; }\n\
         void caller(void *parent) { char *c = sm_private(parent); sm_len(c); }",
        "caller",
    );
    let bad = one(
        "void *sm_private(void *parent) { void *p = sm_alloc(4); sm_len(parent); return p; }\n\
         void caller(void *parent) { char *c = sm_private(parent); sm_len(c); }",
        "caller",
    );
    (good, bad)
}
