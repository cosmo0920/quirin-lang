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

//! Card **VERDICT-BODY-STANDING-CLAUSE-SCOPE** (gateway rules 13i–13k): the
//! clauses each piece of body evidence covers, and the shared vectors binding
//! `proof/QCore/BodyClauseStanding.lean`.
//!
//! * `coverageVectors` = [`covers`], every evidence spelling against every
//!   clause kind.
//! * A real unit: a reference keep whose body rule 13c validates covers its
//!   reference clause only (its `resource_borrowed` formal and its alias
//!   result stay uncovered); a release whose body W4 validates covers every
//!   clause; a W4 summary of a declaration with a container clause covers the
//!   custody clauses and leaves the container clause uncovered.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::contract_clause_standing::{
    ClauseKind, EVIDENCE, covers, declaration_clauses, unit_coverage,
};
use qcore_c_gateway::contract_dependencies::{Recording, UnitRecord};
use qcore_c_gateway::legacy_c::{
    ResourceApiKind, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, analyze_legacy_c_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ContainerEffect;

const LEAN: &str = include_str!("../../../proof/QCore/BodyClauseStanding.lean");

/// The rows of the Lean list `header` opens: the first field and the quoted
/// strings of the nested list.
fn lean_coverage_rows() -> Vec<(String, BTreeSet<String>)> {
    let header = "def coverageVectors : List (String × List String) := [";
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the Lean list closes");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let (evidence, kinds) = line.split_once(", [").expect("a nested list");
            let evidence = evidence
                .trim_start_matches('(')
                .trim_matches('"')
                .to_owned();
            let kinds = kinds
                .split(']')
                .next()
                .expect("closed")
                .split(',')
                .map(|kind| kind.trim().trim_matches('"').to_owned())
                .filter(|kind| !kind.is_empty())
                .collect();
            (evidence, kinds)
        })
        .collect()
}

#[test]
fn lean_coverage_vectors_match_covers() {
    let rows = lean_coverage_rows();
    assert_eq!(
        rows.iter()
            .map(|(evidence, _)| evidence.as_str())
            .collect::<Vec<_>>(),
        EVIDENCE
    );
    for (evidence, kinds) in rows {
        for kind in ClauseKind::ALL {
            assert_eq!(
                covers(&evidence, kind),
                kinds.contains(kind.spelling()),
                "{evidence} {kind:?}"
            );
        }
    }
    // The Lean clause kinds are the Rust ones, in order.
    let header = "def allKinds : List ClauseKind :=";
    let start = LEAN.find(header).expect("allKinds") + header.len();
    let body = &LEAN[start..];
    let list = &body[..body.find(']').expect("closes")];
    let lean = list
        .split(',')
        .map(|kind| {
            kind.trim()
                .trim_start_matches('[')
                .trim()
                .trim_start_matches('.')
        })
        .filter(|kind| !kind.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        lean,
        ClauseKind::ALL
            .iter()
            .map(|kind| kind.spelling())
            .collect::<Vec<_>>()
    );
}

fn refcount_contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "refcount.qcore".to_owned(),
        include_str!("fixtures/contracts/refcount.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn covered(coverage: &BTreeMap<String, BTreeSet<String>>) -> Vec<&str> {
    coverage
        .iter()
        .filter(|(_, by)| !by.is_empty())
        .map(|(label, _)| label.as_str())
        .collect()
}

#[test]
fn a_reference_keep_validated_by_13c_covers_its_reference_clause_only() {
    let unit = "\
struct obj { int refcnt; };\n\
int rc_atomic_add(int *ra, int v);\n\
static struct obj *obj_keep(struct obj *o) { rc_atomic_add(&o->refcnt, 1); return o; }\n\
void f(struct obj *o) { obj_keep(o); }\n";
    let contracts = refcount_contracts();
    let recording = Recording::start();
    let _ = analyze_legacy_c_translation_unit(unit, "f", &contracts).expect("the unit analyses");
    let record = recording.finish();
    assert_eq!(
        record.validated.get("obj_keep"),
        Some(&BTreeSet::from(["13c".to_owned()])),
        "{record:?}"
    );
    let coverage = unit_coverage(&record, &contracts);
    let keep = &coverage["obj_keep"];
    assert_eq!(
        keep.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "resource_borrowed(#0)",
            "resource_reference_kept(#0)",
            "resource_return_alias(#0, nonnull)",
        ]
    );
    assert_eq!(covered(keep), ["resource_reference_kept(#0)"]);
}

const RELEASE_UNIT: &str = "\
void free(void *p);\n\
void released(void *p) { free(p); }\n";

#[test]
fn a_release_validated_by_w4_covers_every_clause() {
    let mut contracts = ResourceContracts::new([("free", ResourceApiKind::Release)]);
    contracts.insert("released", ResourceApiKind::Release);
    let recording = Recording::start();
    let _ = analyze_legacy_c_translation_unit(RELEASE_UNIT, "released", &contracts)
        .expect("the unit analyses");
    let record = recording.finish();
    assert!(
        record
            .validated
            .get("released")
            .is_some_and(|by| by.contains("W4")),
        "{record:?}"
    );
    let coverage = unit_coverage(&record, &contracts);
    let released = &coverage["released"];
    assert_eq!(
        covered(released),
        ["resource_released(#0, nonnull)", "resource_return_none()"]
    );
    assert_eq!(released.len(), 2);
}

#[test]
fn w4_leaves_a_container_clause_uncovered() {
    let mut contracts = ResourceContracts::default();
    let mut contract = ResourceFunctionContract::new(
        [
            ResourceParameterMode::Borrowed,
            ResourceParameterMode::Borrowed,
        ],
        ResourceReturnMode::None,
    );
    contract.container_effects.insert(
        0,
        ContainerEffect::Published {
            container_parameter: 1,
        },
    );
    contracts.insert_contract("link", contract);
    let mut record = UnitRecord::default();
    record.examined.insert("link".to_owned());
    record
        .validated
        .insert("link".to_owned(), BTreeSet::from(["W4".to_owned()]));
    let coverage = unit_coverage(&record, &contracts);
    let link = &coverage["link"];
    assert_eq!(
        covered(link),
        [
            "resource_borrowed(#0)",
            "resource_borrowed(#1)",
            "resource_return_none()"
        ]
    );
    assert!(link["resource_published(#0, #1)"].is_empty());
    // The clause list reads the declaration's tables, container included.
    assert_eq!(declaration_clauses(&contracts, "link").len(), 4);
    // A name no table declares has no clause, and nothing is listed for it.
    assert!(declaration_clauses(&contracts, "absent").is_empty());
}

#[test]
fn a_named_premise_is_an_uncovered_clause() {
    let module = "module contract;

extern fn q_wait(q: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_premise(served_queue, \"no callback takes the caller's reference\");
    ensures normal resource_borrowed(q);
    ensures normal resource_return_none();
};
";
    let contracts =
        load_resource_contract_sources(&BTreeMap::from([("q.qv".to_owned(), module.to_owned())]))
            .expect("the module loads");
    let mut record = UnitRecord::default();
    record
        .validated
        .insert("q_wait".to_owned(), BTreeSet::from(["W4".to_owned()]));
    let coverage = unit_coverage(&record, &contracts);
    let q_wait = &coverage["q_wait"];
    assert!(q_wait["resource_premise(served_queue)"].is_empty());
    assert_eq!(
        covered(q_wait),
        ["resource_borrowed(#0)", "resource_return_none()"]
    );
}
