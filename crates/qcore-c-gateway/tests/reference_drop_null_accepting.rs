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

//! Card REFERENCE-DROP-NULL-ACCEPTING (gateway rule 13b, contract
//! specification C04.17): `resource_reference_dropped(p, nullable)` on route
//! G, over the synthetic `rc_*` surface (`fixtures/contracts/refcount.qv`,
//! `fixtures/contracts/refcount_nullable.qv`). A `Safe` here is `Safe` over
//! those trusted test assumptions and promotes nothing.
//!
//! The NULL edge of a NULL-accepting drop discharges nothing and creates
//! nothing; its non-NULL edge is exactly rule 13b's drop. The negative
//! controls: a drop **not** marked nullable still refuses a possibly-NULL
//! actual, and a NULL-accepting drop of a non-NULL reference the function
//! does not hold still refuses. The shared vectors bind
//! `QCore.NullableReferenceDrop`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_c_gateway::reference_count::{
    Binding, DropActual, DropDecision, Held, NULLABLE_DROP_VECTORS, NullableDrop,
    decide_drop_actual,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
void *rc_new(size_t n); void rc_use(void *p); void *rc_keep(void *p); void rc_drop(void *p);
void rc_drop_null(void *p); void unknown_call(void *p);
struct holder { void *ref; };
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "refcount.qcore".to_owned(),
            include_str!("fixtures/contracts/refcount.qv").to_owned(),
        ),
        (
            "refcount_nullable.qcore".to_owned(),
            include_str!("fixtures/contracts/refcount_nullable.qv").to_owned(),
        ),
    ]))
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
            eprintln!("skipping Clang-dependent REFERENCE-DROP-NULL-ACCEPTING test: {reason}");
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

/// The row of function `f` in the unit `PRELUDE + source`.
fn one(source: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "f",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = summary.functions.get("f").expect("a row for f");
    Row {
        status: row.status,
        kinds: row
            .diagnostics
            .iter()
            .map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

#[test]
fn the_declaration_loads_withheld_and_nullable() {
    let contracts = contracts();
    assert!(contracts.contract("rc_drop_null").is_none());
    let declared = contracts
        .retaining_contract("rc_drop_null")
        .expect("withheld with the retaining declarations");
    assert_eq!(declared.references.len(), 1);
    assert!(declared.references[0].nullable);
    // The plain drop is unchanged, and serializes without the field.
    let plain = contracts.retaining_contract("rc_drop").expect("withheld");
    assert!(!plain.references[0].nullable);
    let json = serde_json::to_string(&plain.references[0]).unwrap();
    assert!(!json.contains("nullable"), "{json}");
}

/// The non-NULL edge: a NULL-accepting drop of a reference the function
/// holds is exactly rule 13b's drop.
#[test]
fn the_non_null_edge_is_todays_drop() {
    if !clang_available() {
        return;
    }
    // After a NULL test: balanced.
    let row = one("void f(void) { void *r = rc_new(1); if (!r) return; rc_drop_null(r); }");
    assert!(row.safe(), "{row:?}");
    // A keep of a caller's object, dropped through the NULL-accepting drop.
    let row = one("void f(void *p) { rc_keep(p); rc_use(p); rc_drop_null(p); }");
    assert!(row.safe(), "{row:?}");
    // Two references held, one dropped: a leak candidate.
    let row =
        one("void f(void) { void *r = rc_new(1); if (!r) return; rc_keep(r); rc_drop_null(r); }");
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
    // Dropped twice: an over-drop of an acquired family.
    let row = one(
        "void f(void) {\n void *r = rc_new(1);\n if (!r) return;\n rc_drop_null(r);\n \
         rc_drop_null(r);\n}",
    );
    assert!(
        row.status == AnalysisStatus::Unsafe && row.mentions("DoubleRelease"),
        "{row:?}"
    );
    // Used after the last drop: refused, never accused.
    let row = one(
        "void f(void) {\n void *r = rc_new(1);\n if (!r) return;\n rc_drop_null(r);\n \
         rc_use(r);\n}",
    );
    assert!(
        row.refused() && row.mentions("reference-dropped"),
        "{row:?}"
    );
}

/// The NULL edge: nothing discharged, nothing created.
#[test]
fn the_null_edge_is_a_no_op() {
    if !clang_available() {
        return;
    }
    // A returned reference that may be NULL, dropped with no NULL test: its
    // NULL edge holds nothing, its non-NULL edge is the last drop.
    let row = one("void f(void) { void *r = rc_new(1); rc_drop_null(r); }");
    assert!(row.safe(), "{row:?}");
    // A definitely-NULL pointer: a no-op.
    let row = one("void f(void) { void *r = 0; rc_drop_null(r); }");
    assert!(row.safe(), "{row:?}");
    // Joined from a branch: one family or NULL.
    let row = one("void f(int c) { void *r = 0; if (c) { r = rc_new(1); } rc_drop_null(r); }");
    assert!(row.safe(), "{row:?}");
    // A NULL drop discharges nothing: the acquired reference is still owed.
    let row =
        one("void f(void) { void *n = 0; void *r = rc_new(1); if (!r) return; rc_drop_null(n); }");
    assert!(!row.safe() && row.mentions("Leak"), "{row:?}");
}

/// Negative controls.
#[test]
fn the_negative_controls_still_refuse() {
    if !clang_available() {
        return;
    }
    // A drop not marked nullable still refuses a possibly-NULL actual.
    let row = one("void f(void) { void *r = rc_new(1); rc_drop(r); }");
    assert!(!row.safe(), "{row:?}");
    let row = one("void f(void) { void *r = 0; rc_drop(r); }");
    assert!(
        row.refused() && row.mentions("reference-drop-unheld"),
        "{row:?}"
    );
    // A NULL-accepting drop of a non-NULL reference still needs the
    // reference: a caller's object, a member read.
    let row = one("void f(void *p) { rc_drop_null(p); }");
    assert!(
        row.refused() && row.mentions("reference-drop-unheld"),
        "{row:?}"
    );
    let row = one("void f(struct holder *h) { rc_drop_null(h->ref); }");
    assert!(row.refused() && !row.safe(), "{row:?}");
}

/// Rule 13c: a NULL-accepting drop defined in the unit stands only where its
/// body does nothing on NULL.
#[test]
fn a_body_that_dereferences_null_withdraws_the_nullable_drop() {
    if !clang_available() {
        return;
    }
    let row = one("struct obj { int refcnt; }; void rc_final(void *p);\n\
         void rc_drop_null(void *p) { struct obj *o = p; if (--o->refcnt == 0) rc_final(p); }\n\
         void f(void) { void *r = rc_new(1); rc_drop_null(r); }");
    assert!(
        row.refused() && row.mentions("reference-withdrawn"),
        "{row:?}"
    );
}

// ---------------------------------------------------------------------------
// The shared vectors (`QCore.NullableReferenceDrop`)
// ---------------------------------------------------------------------------

const LEAN: &str = include_str!("../../../proof/QCore/NullableReferenceDrop.lean");

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

fn lean_held(held: Held) -> String {
    match held {
        Held::Known(held) => format!("(.known {held})"),
        Held::Undecided => ".undecided".to_owned(),
    }
}

fn lean_binding(binding: Binding) -> String {
    match binding {
        Binding::Unbound => ".unbound".to_owned(),
        Binding::Family { held, handed } => format!("(.family {} {handed})", lean_held(held)),
        Binding::Invalid => ".invalid".to_owned(),
        Binding::Freed => ".freed".to_owned(),
        Binding::Other => ".other".to_owned(),
    }
}

fn lean_actual(actual: DropActual) -> String {
    match actual {
        DropActual::Plain(binding) => format!("(.plain {})", lean_binding(binding)),
        DropActual::Null => ".null".to_owned(),
        DropActual::OneOrNull { held, handed } => {
            format!("(.oneOrNull {} {handed})", lean_held(held))
        }
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

fn lean_decision(decision: NullableDrop) -> String {
    match decision {
        NullableDrop::NoOp => ".noOp".to_owned(),
        NullableDrop::Drop(drop) => format!(
            "(.drop {})",
            match drop {
                DropDecision::Decrement => ".decrement".to_owned(),
                DropDecision::Final => ".final".to_owned(),
                DropDecision::OverDrop => ".overDrop".to_owned(),
                DropDecision::Refuse(reason) => format!("(.refuse {})", lean_reason(reason)),
            }
        ),
    }
}

#[test]
fn the_nullable_drop_vectors_are_the_lean_table() {
    let rows = lean_rows("def nullableDropVectors : List (DropActual × Bool × NullableDrop) := [");
    let rust = NULLABLE_DROP_VECTORS
        .iter()
        .map(|(actual, nullable, decision)| {
            assert_eq!(decide_drop_actual(*actual, *nullable), *decision);
            format!(
                "({}, {nullable}, {})",
                lean_actual(*actual),
                lean_decision(*decision)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}
