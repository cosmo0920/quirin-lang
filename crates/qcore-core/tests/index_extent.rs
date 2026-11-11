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

//! Card CG16-INDEX-AND-OFFSET-EXTENTS: the Rust decisions of
//! `qcore_core::index_extent` against the shared vectors of
//! `proof/QCore/IndexExtent.lean` (`elementVectors`, `byteVectors`,
//! `resolveVectors`, `verdictVectors`), parsed from the Lean text so neither
//! side can change alone, plus the Rust-only controls the vectors do not reach.

use qcore_core::index_extent::{
    BOUNDS_MODEL_VERSION, BOUNDS_PROPERTY, BoundsVerdict, Count, Decision, EvidenceIdentity,
    ExtentResolution, Facts, IndexFact, Judgement, Outcome, PointerFact, Refusal, Request,
    SameObject, ScalarFact, Standing, Unit, admit, bounds_verdict, byte_access, compose,
    element_access, formation, resolve_extent, same_object, scale,
};

const LEAN: &str = include_str!("../../../proof/QCore/IndexExtent.lean");

#[derive(Clone, Debug, PartialEq)]
enum Term {
    App(String, Vec<Term>),
    Tuple(Vec<Term>),
    List(Vec<Term>),
}

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if "()[],".contains(c) || c.is_whitespace() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            if !c.is_whitespace() {
                out.push(c.to_string());
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

fn parse_items(toks: &[String], at: &mut usize, close: &str) -> Vec<Term> {
    let mut items = Vec::new();
    loop {
        if toks[*at] == close {
            *at += 1;
            return items;
        }
        items.push(parse_term(toks, at));
        if toks[*at] == "," {
            *at += 1;
        }
    }
}

fn parse_arg(toks: &[String], at: &mut usize) -> Term {
    match toks[*at].as_str() {
        "(" => {
            *at += 1;
            let mut items = parse_items(toks, at, ")");
            if items.len() == 1 {
                items.remove(0)
            } else {
                Term::Tuple(items)
            }
        }
        "[" => {
            *at += 1;
            Term::List(parse_items(toks, at, "]"))
        }
        atom => {
            *at += 1;
            Term::App(atom.to_owned(), Vec::new())
        }
    }
}

fn parse_term(toks: &[String], at: &mut usize) -> Term {
    let head = parse_arg(toks, at);
    let Term::App(name, _) = &head else {
        return head;
    };
    let mut args = Vec::new();
    while !matches!(toks[*at].as_str(), "," | ")" | "]") {
        args.push(parse_arg(toks, at));
    }
    Term::App(name.clone(), args)
}

fn rows(header: &str) -> Vec<Vec<Term>> {
    let table = LEAN.split(header).nth(1).expect("vector table");
    let table = &table[..table.find("\n]").expect("end of table") + 2];
    let toks = tokens(&format!("[{table}"));
    let mut at = 1; // past the opening `[`
    parse_items(&toks, &mut at, "]")
        .into_iter()
        .map(|row| match row {
            Term::Tuple(fields) => fields,
            other => panic!("not a row: {other:?}"),
        })
        .collect()
}

fn name(term: &Term) -> (&str, &[Term]) {
    match term {
        Term::App(name, args) => (name.as_str(), args.as_slice()),
        other => panic!("not an application: {other:?}"),
    }
}

fn int(term: &Term) -> i128 {
    name(term).0.parse().expect("integer")
}

fn count(term: &Term) -> Count {
    match name(term) {
        (".const", [n]) => Count::Const(int(n) as u128),
        (".sym", [s]) => Count::Symbol(format!("s{}", int(s))),
        other => panic!("count {other:?}"),
    }
}

fn option<T>(term: &Term, inner: impl Fn(&Term) -> T) -> Option<T> {
    match name(term) {
        ("none", []) => None,
        ("some", [value]) => Some(inner(value)),
        other => panic!("option {other:?}"),
    }
}

fn index(term: &Term) -> IndexFact {
    match name(term) {
        (".const", [i]) => IndexFact::Const(int(i)),
        (".range", [lo, hi]) => IndexFact::Range {
            lo: int(lo),
            hi: count(hi),
        },
        (".unknown", []) => IndexFact::Unknown,
        other => panic!("index {other:?}"),
    }
}

fn refusal(term: &Term) -> Refusal {
    match name(term).0 {
        ".missingExtent" => Refusal::MissingExtent,
        ".unsupportedRange" => Refusal::UnsupportedRange,
        ".rangeNotContained" => Refusal::RangeNotContained,
        ".symbolMismatch" => Refusal::SymbolMismatch,
        ".overflow" => Refusal::Overflow,
        ".emptyWidth" => Refusal::EmptyWidth,
        ".emptyRange" => Refusal::EmptyRange,
        other => panic!("refusal {other}"),
    }
}

fn decision(term: &Term) -> Decision {
    match name(term) {
        (".inBounds", []) => Decision::InBounds,
        (".outOfBounds", []) => Decision::OutOfBounds,
        (".undecided", [r]) => Decision::Undecided(refusal(r)),
        other => panic!("decision {other:?}"),
    }
}

fn standing(term: &Term) -> Standing {
    match name(term).0 {
        ".source" => Standing::Source,
        ".resolved" => Standing::Resolved,
        ".held" => Standing::Held,
        other => panic!("standing {other}"),
    }
}

fn pair<A, B>(term: &Term, a: impl Fn(&Term) -> A, b: impl Fn(&Term) -> B) -> (A, B) {
    match term {
        Term::Tuple(fields) if fields.len() == 2 => (a(&fields[0]), b(&fields[1])),
        other => panic!("pair {other:?}"),
    }
}

fn outcome(term: &Term) -> Outcome {
    match name(term) {
        (".inBounds", []) => Outcome::InBounds,
        (".excluded", []) => Outcome::Excluded,
        (".violation", [flag]) => Outcome::Violation {
            reachable: name(flag).0 == "true",
        },
        (".held", [n]) => Outcome::Held(vec![format!("clause {}", int(n))]),
        (".unresolved", [n]) => Outcome::Unresolved(format!("reason {}", int(n))),
        other => panic!("outcome {other:?}"),
    }
}

fn verdict(term: &Term) -> BoundsVerdict {
    match name(term).0 {
        ".safe" => BoundsVerdict::Safe,
        ".pending" => BoundsVerdict::Pending,
        ".unknown" => BoundsVerdict::Unknown,
        ".«unsafe»" => BoundsVerdict::Unsafe,
        other => panic!("verdict {other}"),
    }
}

#[test]
fn cg16_element_vectors_match_the_lean_model() {
    let rows =
        rows("def elementVectors : List (Option Count × IndexFact × Decision × Decision) := [");
    assert_eq!(rows.len(), 16);
    for row in rows {
        let [n, i, access, formed] = row.as_slice() else {
            panic!("{row:?}")
        };
        let n = option(n, count);
        let i = index(i);
        assert_eq!(
            element_access(n.as_ref(), &i),
            decision(access),
            "{n:?} {i:?}"
        );
        assert_eq!(
            formation(n.as_ref(), &i),
            decision(formed),
            "{n:?} {i:?} (formation)"
        );
    }
}

#[test]
fn cg16_byte_vectors_match_the_lean_model() {
    let rows = rows("def byteVectors : List (Nat × Int × Nat × Nat × Decision) := [");
    assert_eq!(rows.len(), 7);
    for row in rows {
        let [extent, offset, width, limit, expected] = row.as_slice() else {
            panic!("{row:?}")
        };
        let got = byte_access(
            int(extent) as u128,
            int(offset),
            int(width) as u128,
            int(limit) as u128,
        );
        assert_eq!(got, decision(expected), "{row:?}");
    }
}

#[test]
fn cg16_resolve_vectors_match_the_lean_model() {
    let rows = rows(
        "def resolveVectors : List (Option Nat × Option (Count × Standing) × Option (Count × Standing)) := [",
    );
    assert_eq!(rows.len(), 7);
    for row in rows {
        let [source, declared, expected] = row.as_slice() else {
            panic!("{row:?}")
        };
        let source = option(source, |t| int(t) as u128);
        let declared = option(declared, |t| pair(t, count, standing));
        let expected = option(expected, |t| pair(t, count, standing));
        assert_eq!(
            resolve_extent(source, declared.clone()).in_force(),
            expected,
            "{row:?}"
        );
    }
}

#[test]
fn cg16_verdict_vectors_match_the_lean_model() {
    let rows = rows("def verdictVectors : List (List (Outcome × Outcome) × Nat × Verdict) := [");
    assert_eq!(rows.len(), 11);
    for row in rows {
        let [judgements, reasons, expected] = row.as_slice() else {
            panic!("{row:?}")
        };
        let Term::List(judgements) = judgements else {
            panic!("{row:?}")
        };
        let judgements = judgements
            .iter()
            .map(|j| {
                let (actual, hypothetical) = pair(j, outcome, outcome);
                Judgement {
                    actual,
                    hypothetical,
                }
            })
            .collect::<Vec<_>>();
        let reasons = (0..int(reasons))
            .map(|n| format!("function reason {n}"))
            .collect::<Vec<_>>();
        assert_eq!(
            bounds_verdict(&judgements, &reasons).0,
            verdict(expected),
            "{row:?}"
        );
    }
}

#[test]
fn cg16_scale_is_checked_against_the_target_limit() {
    let limit = i64::MAX as u128;
    assert_eq!(scale(3, 4, limit), Ok(12));
    assert_eq!(scale(-2, 8, limit), Ok(-16));
    assert_eq!(scale(1, 0, limit), Err(Refusal::EmptyWidth));
    assert_eq!(scale(i64::MAX as i128, 2, limit), Err(Refusal::Overflow));
    // Formed end past the limit refuses, never wraps into range.
    assert_eq!(
        byte_access(u128::MAX, i128::MAX, u128::MAX, limit),
        Decision::Undecided(Refusal::Overflow)
    );
}

#[test]
fn cg16_a_declared_extent_never_overrides_the_source() {
    assert_eq!(
        resolve_extent(Some(4), Some((Count::Const(8), Standing::Resolved))),
        ExtentResolution::Conflict {
            source: 4,
            declared: Count::Const(8)
        }
    );
    assert_eq!(
        resolve_extent(Some(4), Some((Count::Symbol("n".into()), Standing::Held))).in_force(),
        Some((Count::Const(4), Standing::Source))
    );
}

#[test]
fn cg16_a_write_withdraws_every_fact_that_names_the_variable() {
    let mut facts = Facts::default();
    facts.scalars.insert(
        "i".into(),
        ScalarFact {
            value: IndexFact::Range {
                lo: 0,
                hi: Count::Symbol("n".into()),
            },
            standing: Standing::Resolved,
        },
    );
    facts.scalars.insert(
        "k".into(),
        ScalarFact {
            value: IndexFact::Const(2),
            standing: Standing::Source,
        },
    );
    facts.pointers.insert(
        "p".into(),
        PointerFact {
            object: "p@entry".into(),
            count: Count::Symbol("n".into()),
            unit: Unit::Elements,
            offset: IndexFact::Const(0),
            standing: Standing::Resolved,
            source_object: false,
        },
    );
    facts.pointers.insert(
        "a".into(),
        PointerFact {
            object: "arr".into(),
            count: Count::Const(4),
            unit: Unit::Elements,
            offset: IndexFact::Const(1),
            standing: Standing::Source,
            source_object: true,
        },
    );
    let mut written = facts.clone();
    written.write("n");
    assert!(!written.scalars.contains_key("i"));
    assert!(!written.pointers.contains_key("p"));
    assert!(written.scalars.contains_key("k") && written.pointers.contains_key("a"));
    let mut called = facts.clone();
    called.call();
    assert!(
        !called.pointers.contains_key("p"),
        "a call may resize a declared object"
    );
    assert!(
        called.pointers.contains_key("a"),
        "no call resizes a source object"
    );
    assert_eq!(facts.join(&written), written);
}

#[test]
fn cg16_same_object_needs_one_identity() {
    let a = "arr".to_owned();
    let b = "brr".to_owned();
    assert_eq!(same_object(Some(&a), Some(&a), true), SameObject::Same);
    assert_eq!(same_object(Some(&a), Some(&b), true), SameObject::Different);
    assert_eq!(
        same_object(Some(&a), Some(&b), false),
        SameObject::Unresolved
    );
    assert_eq!(same_object(Some(&a), None, true), SameObject::Unresolved);
}

#[test]
fn cg16_composition_keeps_both_rows() {
    let both = Request {
        custody: true,
        bounds: true,
    };
    use BoundsVerdict::*;
    assert_eq!(compose(both, Some(Safe), Some(Unknown)), Unknown);
    assert_eq!(compose(both, Some(Safe), Some(Pending)), Pending);
    assert_eq!(compose(both, Some(Safe), Some(Unsafe)), Unsafe);
    assert_eq!(
        compose(both, Some(Safe), None),
        Unknown,
        "a requested row is never omitted"
    );
    let custody_only = Request {
        custody: true,
        bounds: false,
    };
    assert_eq!(compose(custody_only, Some(Safe), None), Safe);
}

#[test]
fn cg16_admission_binds_the_identity() {
    let identity = EvidenceIdentity {
        property: BOUNDS_PROPERTY.into(),
        model_version: BOUNDS_MODEL_VERSION.into(),
        source_sha256: "s".into(),
        target: "t".into(),
        contracts_sha256: "c".into(),
    };
    assert!(
        admit(
            (&identity, 3, BoundsVerdict::Safe),
            (&identity, 3, BoundsVerdict::Safe)
        )
        .is_ok()
    );
    let mut changed = identity.clone();
    changed.target = "other".into();
    assert!(
        admit(
            (&identity, 3, BoundsVerdict::Safe),
            (&changed, 3, BoundsVerdict::Safe)
        )
        .is_err()
    );
    assert!(
        admit(
            (&identity, 2, BoundsVerdict::Safe),
            (&identity, 3, BoundsVerdict::Safe)
        )
        .is_err()
    );
    assert!(
        admit(
            (&identity, 3, BoundsVerdict::Safe),
            (&identity, 3, BoundsVerdict::Unknown)
        )
        .is_err()
    );
    let mut other = identity.clone();
    other.property = "c.custody".into();
    assert!(
        admit(
            (&other, 3, BoundsVerdict::Safe),
            (&other, 3, BoundsVerdict::Safe)
        )
        .is_err()
    );
}
