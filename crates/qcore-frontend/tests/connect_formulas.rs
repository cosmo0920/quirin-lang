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

//! Card **QUIRIN-AUDIT-CONNECT**: every condition and refusal of the chained
//! measurement (Part A), the bundle composition (Part B) and the declared
//! contract dependencies (Part C), `qcore_schemas::connect::FORMULAS`, is a
//! formula of the C04.18.9 typed rendering over the CX block of registered
//! terms (`RENDERED_TERMS`, block `CX`; ten `cx_` sorts in
//! `RENDERED_VALUE_SORTS`), checked by the premise-formula validator
//! `guarded_proposition` (every free name a registered term in its rendered
//! sort, the text its canonical printing). Prose is refused. The Lean model
//! `QCore.MeasureConnect` states one theorem per formula, `formula_<id>`,
//! bound here by name.

use qcore_frontend::premise_formula::{
    RENDERED_TERMS, RENDERED_VALUE_SORTS, RenderedSort, guarded_proposition, term,
};
use qcore_schemas::connect::FORMULAS;

#[test]
fn every_formula_is_a_validated_typed_rendering() {
    for (id, formula) in FORMULAS {
        guarded_proposition(id, formula).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut ids = FORMULAS.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    assert!(ids.iter().all(|id| id.starts_with("cx_")));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), FORMULAS.len());
}

#[test]
fn prose_is_refused_by_the_validator() {
    for prose in [
        "a refused link is never bridged",
        "\"#cx_joined(#s,#t)\"",
        "forall (#s: cx_step) { #cx_unregistered(#s) }",
        // A registered term in the wrong sort.
        "forall (#w: cx_row) { #cx_member(#w) }",
        // A free variable.
        "#cx_member(#s)",
        // Not the canonical printing.
        "forall (#s: cx_step) { #cx_member(#s) => #cx_member(#s) }",
    ] {
        assert!(guarded_proposition("cx_probe", prose).is_err(), "{prose}");
    }
}

#[test]
fn every_cx_term_is_used_typed_and_bound_to_lean() {
    let cx = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "CX")
        .collect::<Vec<_>>();
    assert_eq!(cx.len(), 94);
    let registered =
        |s: &str| s == "obj" || s == "integer" || RENDERED_VALUE_SORTS.iter().any(|(v, _)| *v == s);
    for t in &cx {
        assert!(t.name.starts_with("cx_"), "#{}", t.name);
        assert!(term(t.name).is_none(), "#{} is a premise term", t.name);
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1,
            "#{}",
            t.name
        );
        let sorts: Vec<&str> = match t.sort {
            RenderedSort::Value(v) => vec![v],
            RenderedSort::Predicate(a) => a.to_vec(),
            RenderedSort::Function(a, r) => a.iter().copied().chain([r]).collect(),
        };
        assert!(sorts.iter().all(|s| registered(s)), "#{}", t.name);
        let spelled = format!("#{}", t.name);
        assert!(
            FORMULAS.iter().any(|(_, f)| f
                .split(|c: char| !(c == '#' || c == '_' || c.is_ascii_alphanumeric()))
                .any(|w| w == spelled)),
            "#{} is named by no formula",
            t.name
        );
    }
    let sorts = RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with("cx_"))
        .count();
    assert_eq!(sorts, 10);
    // The CX block is contiguous, followed only by the G1 block (card
    // G1-CLOSURE-FORMULAS) and then the OTP block (card OTEL-PROTO-AUDIT),
    // never reordered.
    let blocks: Vec<_> = RENDERED_TERMS.iter().map(|t| t.block).collect();
    let first = blocks.iter().position(|b| *b == "CX").unwrap();
    let count = blocks.iter().filter(|b| **b == "CX").count();
    assert!(blocks[first..first + count].iter().all(|b| *b == "CX"));
    assert!(
        blocks[first + count..]
            .iter()
            .all(|b| *b == "G1" || *b == "OTP")
    );
    assert_eq!(blocks[first - 1], "K6");
    // Every formula has its Lean theorem, every term its Lean definition.
    let lean = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../proof/QCore/MeasureConnect.lean"),
    )
    .expect("QCore.MeasureConnect");
    for (id, formula) in FORMULAS {
        assert!(
            lean.contains(&format!("theorem formula_{id} ")),
            "no Lean theorem formula_{id}"
        );
        assert!(
            lean.contains(&format!("\"{formula}\"")),
            "the Lean formula table does not state {id} as written"
        );
    }
    for t in &cx {
        assert!(
            lean.contains(&format!("(\"{}\"", t.name)),
            "no Lean cxTerms entry for #{}",
            t.name
        );
    }
}
