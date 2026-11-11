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

//! Card AUDIT-COMPOSITION: the laws of the integration semigroup
//! (`qcore_schemas::composition`), on generated vectors, and the shared
//! vectors decided by `proof/QCore/AuditComposition.lean` (`vectors_hold`),
//! parsed from the Lean file and checked against this implementation.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::composition::{
    COMPOSITION_SCHEMA_VERSION, CheckerIdentity, Component, ComponentSidecar, Contribution,
    Provenance, ReasonLeaf, RowKey, RuleSet, UnitRef, VerdictTuple,
};
use qcore_schemas::reason::ReasonCategory;
use qcore_schemas::verdict::Verdict;
use serde_json::json;

const LEAN: &str = include_str!("../../../proof/QCore/AuditComposition.lean");

fn row_key(row: u64) -> RowKey {
    RowKey {
        unit: UnitRef {
            library: "lib".into(),
            file: format!("u{row}.c"),
        },
        row: "unit#0".into(),
        property: "resource-custody.v1".into(),
        profile: "lib".into(),
    }
}

fn provenance(prov: u64) -> Provenance {
    Provenance {
        analyzer_revision: format!("analyzer-{prov}"),
        rule_set: RuleSet {
            registry: "registry".into(),
            off: BTreeSet::new(),
        },
        contract_bundles: BTreeMap::from([("resource-contracts".into(), "ab".repeat(32))]),
        external_checker: None,
    }
}

/// The Rust component a Lean `⟨row, prov, body, verdict, assumptions⟩` names.
fn component(row: u64, prov: u64, body: u64, verdict: Verdict, assumptions: &[u64]) -> Component {
    let key = row_key(row);
    Component {
        unit: key.unit,
        row: key.row,
        property: key.property,
        profile: key.profile,
        provenance: provenance(prov),
        verdict: VerdictTuple {
            verdict,
            scope: "scope".into(),
            bounds: "bounds".into(),
            assumptions: assumptions.iter().map(|a| format!("p{a}")).collect(),
            evidence_version: "v".into(),
        },
        reasons: Vec::new(),
        detail: json!({ "body": body }),
    }
}

fn contribution(components: &[Component]) -> Contribution {
    let mut contribution = Contribution::new();
    for component in components {
        contribution.insert(component.clone());
    }
    contribution
}

// ---- the shared vectors ---------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Term {
    Tuple(Vec<Term>),
    List(Vec<Term>),
    Atom(String),
}

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut atom = String::new();
    for ch in text.chars() {
        if matches!(ch, '⟨' | '⟩' | '[' | ']' | ',') || ch.is_whitespace() {
            if !atom.is_empty() {
                out.push(std::mem::take(&mut atom));
            }
            if !ch.is_whitespace() {
                out.push(ch.to_string());
            }
        } else {
            atom.push(ch);
        }
    }
    if !atom.is_empty() {
        out.push(atom);
    }
    out
}

/// One term. Inside a sequence, juxtaposed terms (`some .safe`) form a tuple
/// of their parts, which the interpretation below reads.
fn parse(tokens: &[String], at: &mut usize) -> Term {
    let token = tokens[*at].as_str();
    *at += 1;
    let close = match token {
        "⟨" => "⟩",
        "[" => "]",
        atom => return Term::Atom(atom.to_owned()),
    };
    let mut items = Vec::new();
    let mut item = Vec::new();
    while tokens[*at] != close {
        if tokens[*at] == "," {
            *at += 1;
            items.push(std::mem::take(&mut item));
            continue;
        }
        item.push(parse(tokens, at));
    }
    *at += 1;
    if !item.is_empty() {
        items.push(item);
    }
    let items = items
        .into_iter()
        .map(|mut parts| {
            if parts.len() == 1 {
                parts.pop().unwrap()
            } else {
                Term::Tuple(parts)
            }
        })
        .collect();
    if token == "⟨" {
        Term::Tuple(items)
    } else {
        Term::List(items)
    }
}

fn verdict(term: &Term) -> Verdict {
    match term {
        Term::Atom(name) => match name.as_str() {
            ".safe" => Verdict::Safe,
            ".pending" => Verdict::Pending,
            ".unknown" => Verdict::Unknown,
            ".provisionalUnsafe" => Verdict::Unsafe,
            other => panic!("not a verdict: {other}"),
        },
        other => panic!("not a verdict: {other:?}"),
    }
}

fn number(term: &Term) -> u64 {
    match term {
        Term::Atom(text) => text.parse().expect("a natural number"),
        other => panic!("not a number: {other:?}"),
    }
}

fn numbers(term: &Term) -> Vec<u64> {
    match term {
        Term::List(items) => items.iter().map(number).collect(),
        other => panic!("not a list: {other:?}"),
    }
}

struct SharedVector {
    components: Vec<Component>,
    row: u64,
    expected: Option<Verdict>,
    conflict: bool,
    assumptions: BTreeSet<String>,
}

fn shared_vectors() -> Vec<SharedVector> {
    let header = "def vectors : List Vector := [";
    let start = LEAN.find(header).expect("the Lean vector list") + header.len();
    let body = &LEAN[start..];
    let body = &body[..body.find("\n]").expect("the list's end")];
    let mut vectors = Vec::new();
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let line = line.trim_end_matches(',');
        let tokens = tokens(line);
        let mut at = 0;
        let Term::Tuple(fields) = parse(&tokens, &mut at) else {
            panic!("a vector is an anonymous constructor: {line}");
        };
        assert_eq!(at, tokens.len(), "{line}");
        assert_eq!(fields.len(), 5, "{line}");
        let Term::List(components) = &fields[0] else {
            panic!("components: {line}");
        };
        let components = components
            .iter()
            .map(|term| {
                let Term::Tuple(parts) = term else {
                    panic!("component: {line}");
                };
                component(
                    number(&parts[0]),
                    number(&parts[1]),
                    number(&parts[2]),
                    verdict(&parts[3]),
                    &numbers(&parts[4]),
                )
            })
            .collect();
        let expected = match &fields[2] {
            Term::Atom(none) if none == "none" => None,
            Term::Tuple(parts) if parts[0] == Term::Atom("some".into()) => Some(verdict(&parts[1])),
            other => panic!("expected: {other:?}"),
        };
        let conflict = match &fields[3] {
            Term::Atom(flag) => flag.parse().expect("a Bool"),
            other => panic!("conflict: {other:?}"),
        };
        vectors.push(SharedVector {
            components,
            row: number(&fields[1]),
            expected,
            conflict,
            assumptions: numbers(&fields[4])
                .into_iter()
                .map(|a| format!("p{a}"))
                .collect(),
        });
    }
    vectors
}

#[test]
fn the_shared_lean_vectors_hold_in_rust() {
    let vectors = shared_vectors();
    assert_eq!(vectors.len(), 14, "every Lean vector line was read");
    for (index, vector) in vectors.iter().enumerate() {
        let contribution = contribution(&vector.components);
        let derived = contribution.derive(&row_key(vector.row));
        assert_eq!(
            derived.as_ref().map(|d| d.verdict),
            vector.expected,
            "vector {index}"
        );
        let conflict = derived.as_ref().is_some_and(|d| d.is_conflicted());
        assert_eq!(conflict, vector.conflict, "vector {index}");
        let assumptions = derived.map(|d| d.assumptions).unwrap_or_default();
        assert_eq!(assumptions, vector.assumptions, "vector {index}");
    }
}

// ---- the laws on generated vectors -----------------------------------------

/// Every component over two rows, two provenances, two bodies, four verdicts
/// and three assumption sets.
fn universe() -> Vec<Component> {
    let mut out = Vec::new();
    for row in 0..2 {
        for prov in 0..2 {
            for body in 0..2 {
                for verdict in Verdict::ALL {
                    for assumptions in [&[][..], &[1][..], &[2, 3][..]] {
                        out.push(component(row, prov, body, verdict, assumptions));
                    }
                }
            }
        }
    }
    out
}

/// A deterministic xorshift, so the generated vectors are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn contribution(&mut self, universe: &[Component]) -> Contribution {
        let size = (self.next() % 5) as usize;
        let picks = (0..size)
            .map(|_| universe[(self.next() % universe.len() as u64) as usize].clone())
            .collect::<Vec<_>>();
        contribution(&picks)
    }
}

const ROUNDS: usize = 3000;

fn rows() -> [RowKey; 2] {
    [row_key(0), row_key(1)]
}

#[test]
fn union_is_associative_commutative_idempotent_with_the_empty_identity() {
    let universe = universe();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let empty = Contribution::new();
    for _ in 0..ROUNDS {
        let (a, b, c) = (
            rng.contribution(&universe),
            rng.contribution(&universe),
            rng.contribution(&universe),
        );
        assert_eq!(a.union(&b).union(&c), a.union(&b.union(&c)));
        assert_eq!(a.union(&b), b.union(&a));
        assert_eq!(a.union(&empty), a);
        assert_eq!(empty.union(&a), a);
        assert_eq!(a.union(&a), a);
        // Erases nothing.
        let ab = a.union(&b);
        assert!(a.components().chain(b.components()).all(|x| ab.contains(x)));
        assert_eq!(
            ab.len(),
            a.len() + b.components().filter(|x| !a.contains(x)).count()
        );
        // The derived rows read only the set.
        for row in rows() {
            assert_eq!(
                ab.union(&c).derive(&row),
                a.union(&b.union(&c)).derive(&row)
            );
        }
    }
}

#[test]
fn a_byte_identical_component_is_idempotent_and_never_a_conflict() {
    for x in universe() {
        let once = Contribution::singleton(x.clone());
        let mut twice = once.clone();
        twice.insert(x.clone());
        assert_eq!(once, twice);
        assert_eq!(twice.len(), 1);
        assert_eq!(twice.conflicts().count(), 0);
        let derived = twice.derive(&x.row_key()).unwrap();
        assert_eq!(derived.verdict, x.verdict.verdict);
        assert!(!derived.is_conflicted());
    }
}

#[test]
fn the_join_never_lowers_and_adding_never_removes_an_assumption() {
    let universe = universe();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for _ in 0..ROUNDS {
        let (a, b) = (rng.contribution(&universe), rng.contribution(&universe));
        let ab = a.union(&b);
        for row in rows() {
            let Some(before) = a.derive(&row) else {
                assert!(a.components().all(|x| x.row_key() != row));
                continue;
            };
            let after = ab.derive(&row).expect("a present row stays present");
            assert!(after.verdict >= before.verdict, "the join lowered a row");
            assert!(before.assumptions.is_subset(&after.assumptions));
            assert!(before.reasons.is_subset(&after.reasons));
            for x in a.components().filter(|x| x.row_key() == row) {
                assert!(x.verdict.verdict <= before.verdict);
                assert!(x.verdict.assumptions.is_subset(&before.assumptions));
            }
        }
    }
}

#[test]
fn a_conflict_is_unknown_keeps_both_and_names_its_category() {
    let universe = universe();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut seen = 0;
    for _ in 0..ROUNDS {
        let a = rng.contribution(&universe);
        for row in rows() {
            let Some(derived) = a.derive(&row) else {
                continue;
            };
            let members = a
                .components()
                .filter(|x| x.row_key() == row)
                .collect::<Vec<_>>();
            let expected_conflict = members.iter().any(|x| {
                members
                    .iter()
                    .any(|y| x.provenance == y.provenance && x.bytes() != y.bytes())
            });
            assert_eq!(derived.is_conflicted(), expected_conflict);
            if !expected_conflict {
                continue;
            }
            seen += 1;
            assert!(derived.verdict >= Verdict::Unknown);
            if members
                .iter()
                .all(|x| x.verdict.verdict <= Verdict::Unknown)
            {
                assert_eq!(derived.verdict, Verdict::Unknown);
            }
            assert!(
                derived
                    .reasons
                    .iter()
                    .any(|leaf| leaf.category == Some(ReasonCategory::CompositionConflict))
            );
            // Both kept: every member is still a component of the row.
            assert_eq!(derived.components.len(), members.len());
        }
    }
    assert!(seen > 100, "the generator reaches conflicts ({seen})");
}

#[test]
fn removal_restores_the_prior_derivation() {
    let universe = universe();
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let mut disjoint = 0;
    for _ in 0..ROUNDS {
        let (a, b) = (rng.contribution(&universe), rng.contribution(&universe));
        let ab = a.union(&b);
        let removed = ab.remove(&b);
        if a.components().all(|x| !b.contains(x)) {
            disjoint += 1;
            assert_eq!(removed, a);
            for row in rows() {
                assert_eq!(removed.derive(&row), a.derive(&row));
            }
        }
        // Removal never keeps a removed component and never invents one.
        assert!(
            removed
                .components()
                .all(|x| !b.contains(x) && ab.contains(x))
        );
    }
    assert!(
        disjoint > 1000,
        "the generator reaches disjoint pairs ({disjoint})"
    );
}

#[test]
fn a_contribution_round_trips_through_its_wire_form_by_union() {
    let universe = universe();
    let mut rng = Rng(0x5851_F42D_4C95_7F2D);
    for _ in 0..200 {
        let a = rng.contribution(&universe);
        let wire = serde_json::to_string(&a).unwrap();
        let read: Contribution = serde_json::from_str(&wire).unwrap();
        assert_eq!(read, a);
    }
    // A listed duplicate collapses and a listed conflict is kept.
    let x = component(0, 0, 0, Verdict::Safe, &[1]);
    let y = component(0, 0, 1, Verdict::Safe, &[1]);
    let wire = json!({
        "schema_version": COMPOSITION_SCHEMA_VERSION,
        "components": [x, x, y],
    });
    let read: Contribution = serde_json::from_value(wire).unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(read.conflicts().count(), 1);
    // Another version is refused.
    let wire = json!({"schema_version": 2, "components": []});
    assert!(serde_json::from_value::<Contribution>(wire).is_err());
}

#[test]
fn an_external_checker_is_a_distinct_provenance_and_the_join_is_its_ceiling() {
    let host = component(0, 0, 0, Verdict::Safe, &[1]);
    let mut checker = component(0, 0, 0, Verdict::Pending, &[2]);
    checker.provenance.external_checker = Some(CheckerIdentity {
        id: "acme.pool_check".into(),
        version: "1".into(),
        digest: "cd".repeat(32),
    });
    checker.reasons = vec![ReasonLeaf {
        category: Some(ReasonCategory::PluginClaimUncertified),
        text: "plugin claim uncertified: ...".into(),
    }];
    let both = Contribution::singleton(host.clone()).union(&Contribution::singleton(checker));
    let derived = both.derive(&host.row_key()).unwrap();
    assert_eq!(derived.verdict, Verdict::Pending);
    assert!(!derived.is_conflicted());
    assert_eq!(
        derived.assumptions,
        BTreeSet::from(["p1".to_owned(), "p2".to_owned()])
    );
    // Removing the checker's contribution restores the host's row.
    let restored = both.remove(&Contribution::singleton(
        both.components().nth(1).unwrap().clone(),
    ));
    assert_eq!(restored, Contribution::singleton(host.clone()));
}

#[test]
fn a_sidecar_must_present_every_row_exactly_once() {
    let x = component(0, 0, 0, Verdict::Safe, &[]);
    let y = component(1, 0, 0, Verdict::Unknown, &[]);
    let contribution =
        Contribution::singleton(x.clone()).union(&Contribution::singleton(y.clone()));
    let sidecar = |order: Vec<RowKey>| ComponentSidecar {
        schema_version: COMPOSITION_SCHEMA_VERSION,
        view: "test".into(),
        order,
        contribution: contribution.clone(),
    };
    assert!(sidecar(vec![x.row_key(), y.row_key()]).validate().is_ok());
    assert!(sidecar(vec![y.row_key(), x.row_key()]).validate().is_ok());
    assert!(
        sidecar(vec![x.row_key()]).validate().is_err(),
        "unpresented"
    );
    assert!(
        sidecar(vec![x.row_key(), y.row_key(), x.row_key()])
            .validate()
            .is_err(),
        "presented twice"
    );
    assert!(
        sidecar(vec![x.row_key(), y.row_key(), row_key(7)])
            .validate()
            .is_err(),
        "presented without a component"
    );
    let mut versioned = sidecar(vec![x.row_key(), y.row_key()]);
    versioned.schema_version = 2;
    assert!(versioned.validate().is_err());
}
