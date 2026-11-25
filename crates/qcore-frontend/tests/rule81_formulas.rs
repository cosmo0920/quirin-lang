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

//! Card **K6-DIRECT-POSITION-ACCESS** (gateway rule 81): rule 81's admission
//! conditions and refusals, each a formula of the C04.18.9 typed rendering
//! over the K6 block of registered terms (`RENDERED_TERMS`, block `K6`;
//! sorts `k6_amp`, `k6_member`, `k6_place` in `RENDERED_VALUE_SORTS`), each
//! checked by the premise-formula validator `guarded_proposition` (every
//! free name a registered term in its rendered sort, the text its canonical
//! printing). Prose is refused. The table is the gateway's
//! `address_arrow_place::FORMULAS` and `QCore.AddressArrowPlace.formulaTable`,
//! row for row (bound by the gateway's unit test).

use qcore_frontend::premise_formula::{
    RENDERED_TERMS, RENDERED_VALUE_SORTS, RenderedSort, guarded_proposition, term,
};

/// Rule 81's formulas, `(id, formula)`: the admission conditions, then the
/// named refusals.
pub const FORMULAS: &[(&str, &str)] = &[
    (
        "k6_exempt_is_the_shape",
        "forall (#a: k6_amp) { (#k6_exempt(#a) => ((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a))) }",
    ),
    (
        "k6_the_shape_is_exempt",
        "forall (#a: k6_amp) { (((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a)) => #k6_exempt(#a)) }",
    ),
    (
        "k6_exempt_binds_as_the_plain_spelling",
        "forall (#a: k6_amp) { (#k6_exempt(#a) => (!#k6_binds(#a) && !#k6_plain_binds(#a))) }",
    ),
    (
        "k6_at_least_the_plain_reader",
        "forall (#a: k6_amp) { (#k6_plain_binds(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_only_removes_bindings",
        "forall (#a: k6_amp) { (#k6_binds(#a) => #k6_old_binds(#a)) }",
    ),
    (
        "k6_place_is_the_plain_place",
        "forall (#e: k6_member) { (#k6_over_member_address(#e) => (#k6_place(#e) == #k6_plain_place(#e) && #k6_root(#k6_place(#e)) == #k6_root(#k6_plain_place(#e)))) }",
    ),
    (
        "k6_place_only_adds",
        "forall (#e: k6_member) { (!#k6_old_place(#e) == #k6_refused => #k6_place(#e) == #k6_old_place(#e)) }",
    ),
    (
        "k6_refuse_outer_address",
        "forall (#a: k6_amp) { (#k6_outer_address(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_refuse_whole_address",
        "forall (#a: k6_amp) { (!#k6_name_arrow(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_refuse_operator_in_group",
        "forall (#a: k6_amp) { (!#k6_selection_chain(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_refuse_undereferenced_group",
        "forall (#a: k6_amp) { (!#k6_group_arrow(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_refuse_no_group",
        "forall (#a: k6_amp) { (!#k6_opens_group(#a) => #k6_binds(#a)) }",
    ),
    (
        "k6_refuse_whole_object_address",
        "forall (#e: k6_member) { ((#k6_over_address(#e) && !#k6_over_member_address(#e)) => #k6_place(#e) == #k6_refused) }",
    ),
];

#[test]
fn every_formula_is_a_validated_typed_rendering() {
    for (id, formula) in FORMULAS {
        guarded_proposition(id, formula).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut ids = FORMULAS.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    assert!(ids.iter().all(|id| id.starts_with("k6_")));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), FORMULAS.len());
}

#[test]
fn prose_is_refused_by_the_validator() {
    for prose in [
        "an exempted group binds nothing",
        "\"#k6_exempt(#a)\"",
        "forall (#a: k6_amp) { #k6_unregistered(#a) }",
        // A registered term in the wrong sort.
        "forall (#e: k6_member) { #k6_exempt(#e) }",
        // A free variable.
        "#k6_exempt(#a)",
        // Not the canonical printing.
        "forall (#a: k6_amp) { #k6_plain_binds(#a) => #k6_binds(#a) }",
    ] {
        assert!(guarded_proposition("k6_probe", prose).is_err(), "{prose}");
    }
}

#[test]
fn every_k6_term_is_used_and_typed() {
    let k6 = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "K6")
        .collect::<Vec<_>>();
    assert_eq!(k6.len(), 16);
    let registered = |s: &str| s == "obj" || RENDERED_VALUE_SORTS.iter().any(|(v, _)| *v == s);
    for t in &k6 {
        assert!(t.name.starts_with("k6_"), "#{}", t.name);
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
        .filter(|(s, _)| s.starts_with("k6_"))
        .count();
    assert_eq!(sorts, 3);
}
