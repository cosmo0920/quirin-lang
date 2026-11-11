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

//! Card QUANT-INDEX-CHECKS: the Rust decisions of `qcore_core::index_quant`
//! against the shared vectors of `proof/QCore/IndexQuant.lean`
//! (`admittedVectors`, `decideVectors`, `shiftVectors`), parsed from the Lean
//! text so neither side can change alone, plus the Rust-only controls the
//! vectors do not reach.

use qcore_core::index_extent::{Count, Decision, IndexFact, Refusal};
use qcore_core::index_quant::{
    Admitted, BoundTerm, QuantDecision, QuantPredicate, QuantSet, Relation, admitted,
    both_in_bounds, index_admitted, shifted_within,
};

const LEAN: &str = include_str!("../../../proof/QCore/IndexQuant.lean");

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
    let mut at = 1;
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

fn list(term: &Term) -> &[Term] {
    match term {
        Term::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn pair(term: &Term) -> (i128, i128) {
    match term {
        Term::Tuple(fields) if fields.len() == 2 => (int(&fields[0]), int(&fields[1])),
        other => panic!("not a pair: {other:?}"),
    }
}

/// Formals are `fN`; the vectors' valuation `ρ0` maps every one to 0.
fn bound(term: &Term) -> BoundTerm {
    match name(term) {
        (".mk", [c, formal]) => BoundTerm {
            constant: int(c),
            formal: match name(formal) {
                ("none", []) => None,
                ("some", [f]) => Some(format!("f{}", int(f))),
                other => panic!("formal {other:?}"),
            },
        },
        other => panic!("term {other:?}"),
    }
}

fn relation(term: &Term) -> Relation {
    match name(term).0 {
        ".eq" => Relation::Eq,
        ".lt" => Relation::Lt,
        ".gt" => Relation::Gt,
        ".le" => Relation::Le,
        ".ge" => Relation::Ge,
        other => panic!("relation {other}"),
    }
}

fn predicate(term: &Term) -> QuantPredicate {
    match name(term) {
        (".cmp", [r, t]) => QuantPredicate::Compare {
            relation: relation(r),
            bound: bound(t),
        },
        (".and", [p, q]) => QuantPredicate::And(Box::new(predicate(p)), Box::new(predicate(q))),
        (".or", [p, q]) => QuantPredicate::Or(Box::new(predicate(p)), Box::new(predicate(q))),
        other => panic!("predicate {other:?}"),
    }
}

/// The Lean model reports the hull of a bounded set.
fn admitted_hull(term: &Term) -> Result<(i128, i128), &'static str> {
    match name(term) {
        (".empty", []) => Err("empty"),
        (".unbounded", []) => Err("unbounded"),
        (".bounded", [lo, hi]) => Ok((int(lo), int(hi))),
        other => panic!("admitted {other:?}"),
    }
}

fn rust_hull(set: &Admitted) -> Result<(i128, i128), &'static str> {
    match set {
        Admitted::Empty => Err("empty"),
        Admitted::Unbounded => Err("unbounded"),
        Admitted::Bounded(_) => Ok(set.hull().expect("a bounded set has a hull")),
    }
}

fn count(term: &Term) -> Count {
    match name(term) {
        (".const", [n]) => Count::Const(int(n) as u128),
        (".sym", [s]) => Count::Symbol(format!("v{}", int(s))),
        other => panic!("count {other:?}"),
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

fn set(term: &Term) -> QuantSet {
    match name(term) {
        (".constant", [intervals]) => {
            QuantSet::Constant(list(intervals).iter().map(pair).collect())
        }
        (".pre", [s]) => QuantSet::Prefix(format!("v{}", int(s))),
        other => panic!("set {other:?}"),
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

fn decision(term: &Term) -> QuantDecision {
    match name(term) {
        (".inside", []) => QuantDecision::Inside,
        (".outside", []) => QuantDecision::Outside,
        (".undecided", [r]) => QuantDecision::Undecided(refusal(r)),
        other => panic!("decision {other:?}"),
    }
}

fn zero(_: &str) -> Option<i128> {
    Some(0)
}

#[test]
fn the_admitted_vectors_agree_with_lean() {
    let vectors = rows("def admittedVectors : List (Pred × Admitted) := [");
    assert_eq!(vectors.len(), 15, "admitted_vectors_count");
    for row in vectors {
        let p = predicate(&row[0]);
        let reading = p.reading(&zero).expect("a reading");
        assert_eq!(
            rust_hull(&admitted(&reading)),
            admitted_hull(&row[1]),
            "{p:?}"
        );
    }
}

#[test]
fn the_decide_vectors_agree_with_lean() {
    let vectors = rows("def decideVectors : List (QSet × IndexFact × QDecision) := [");
    assert_eq!(vectors.len(), 23, "decide_vectors_count");
    for row in vectors {
        let (s, f) = (set(&row[0]), index(&row[1]));
        assert_eq!(index_admitted(&s, &f), decision(&row[2]), "{s:?} {f:?}");
    }
}

#[test]
fn the_shift_vectors_agree_with_lean() {
    let vectors = rows("def shiftVectors : List (QSet × List (Int × Int) × Int × QDecision) := [");
    assert_eq!(vectors.len(), 7, "shift_vectors_count");
    for row in vectors {
        let s = set(&row[0]);
        let inner = list(&row[1]).iter().map(pair).collect::<Vec<_>>();
        let k = int(&row[2]);
        assert_eq!(
            shifted_within(&s, &inner, k),
            decision(&row[3]),
            "{s:?} {inner:?} {k}"
        );
    }
}

fn cmp(relation: Relation, constant: i128) -> QuantPredicate {
    QuantPredicate::Compare {
        relation,
        bound: BoundTerm {
            constant,
            formal: None,
        },
    }
}

fn and(p: QuantPredicate, q: QuantPredicate) -> QuantPredicate {
    QuantPredicate::And(Box::new(p), Box::new(q))
}

fn or(p: QuantPredicate, q: QuantPredicate) -> QuantPredicate {
    QuantPredicate::Or(Box::new(p), Box::new(q))
}

/// Rust-only control of `mem_reading`: over a window, an index is in the
/// reading exactly when the predicate holds, for every operator, `&&`, `||`
/// and a formal under several values.
#[test]
fn the_reading_is_exact_over_a_window() {
    let len = |constant| QuantPredicate::Compare {
        relation: Relation::Lt,
        bound: BoundTerm {
            constant,
            formal: Some("len".into()),
        },
    };
    let predicates = [
        and(cmp(Relation::Ge, 0), cmp(Relation::Lt, 24)),
        and(cmp(Relation::Gt, 21), cmp(Relation::Le, 23)),
        cmp(Relation::Eq, 5),
        or(
            and(cmp(Relation::Ge, 0), cmp(Relation::Lt, 4)),
            and(cmp(Relation::Ge, 8), cmp(Relation::Lt, 12)),
        ),
        or(cmp(Relation::Lt, 24), cmp(Relation::Gt, 0)),
        and(cmp(Relation::Lt, 0), cmp(Relation::Gt, 5)),
        and(cmp(Relation::Ge, 0), len(0)),
        and(cmp(Relation::Ge, 0), len(-1)),
        or(cmp(Relation::Eq, -3), and(cmp(Relation::Ge, 2), len(2))),
    ];
    for value in [0, 3, 10] {
        let valuation = |formal: &str| (formal == "len").then_some(value);
        for p in &predicates {
            let reading = p.reading(&valuation).expect("a reading");
            for i in -40..40 {
                assert_eq!(
                    reading.iter().any(|v| v.contains(i)),
                    p.holds(&valuation, i).expect("holds"),
                    "{p:?} at {i} with len = {value}"
                );
            }
        }
    }
}

#[test]
fn only_the_prefix_form_is_symbolic() {
    let formal = |relation, constant| QuantPredicate::Compare {
        relation,
        bound: BoundTerm {
            constant,
            formal: Some("len".into()),
        },
    };
    assert_eq!(
        and(cmp(Relation::Ge, 0), formal(Relation::Lt, 0)).symbolic_prefix(),
        Some("len".into())
    );
    assert_eq!(
        and(cmp(Relation::Ge, 0), formal(Relation::Le, -1)).symbolic_prefix(),
        Some("len".into())
    );
    assert_eq!(
        and(cmp(Relation::Gt, -1), formal(Relation::Lt, 0)).symbolic_prefix(),
        Some("len".into())
    );
    for p in [
        and(cmp(Relation::Ge, 0), formal(Relation::Lt, 1)),
        and(cmp(Relation::Ge, 1), formal(Relation::Lt, 0)),
        and(cmp(Relation::Ge, -1), formal(Relation::Lt, 0)),
        and(
            and(cmp(Relation::Ge, 0), formal(Relation::Lt, 0)),
            cmp(Relation::Lt, 24),
        ),
        or(
            and(cmp(Relation::Ge, 0), formal(Relation::Lt, 0)),
            cmp(Relation::Eq, 3),
        ),
        and(formal(Relation::Ge, 0), cmp(Relation::Lt, 4)),
        and(cmp(Relation::Ge, 0), cmp(Relation::Lt, 4)),
    ] {
        assert_eq!(p.symbolic_prefix(), None, "{p:?}");
    }
}

#[test]
fn the_hull_ends_are_joined_conservatively() {
    use Decision::{InBounds, OutOfBounds, Undecided};
    let u = Undecided(Refusal::MissingExtent);
    assert_eq!(both_in_bounds(&InBounds, &InBounds), InBounds);
    assert_eq!(both_in_bounds(&InBounds, &OutOfBounds), OutOfBounds);
    assert_eq!(both_in_bounds(&OutOfBounds, &u), OutOfBounds);
    assert_eq!(both_in_bounds(&InBounds, &u), u);
    assert_eq!(both_in_bounds(&u, &InBounds), u);
}

#[test]
fn bounded_sets_are_merged_and_unbounded_ones_never_bounded() {
    let reading = or(
        and(cmp(Relation::Ge, 0), cmp(Relation::Le, 3)),
        and(cmp(Relation::Ge, 4), cmp(Relation::Le, 7)),
    )
    .reading(&zero)
    .unwrap();
    assert_eq!(admitted(&reading), Admitted::Bounded(vec![(0, 7)]));
    // An empty disjunct beside an unbounded one: still unbounded.
    let reading = or(
        and(cmp(Relation::Lt, 0), cmp(Relation::Gt, 5)),
        cmp(Relation::Ge, 0),
    )
    .reading(&zero)
    .unwrap();
    assert_eq!(admitted(&reading), Admitted::Unbounded);
    // An undecidable index is never inside.
    assert_eq!(
        index_admitted(&QuantSet::Constant(vec![(0, 23)]), &IndexFact::Unknown),
        QuantDecision::Undecided(Refusal::UnsupportedRange)
    );
}
