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

//! Card CG24-ALIAS-EXTENT-FRAME-VOCABULARY: the Rust decisions of
//! `qcore_core::index_extent_frames` against the shared vectors of
//! `proof/QCore/IndexExtentFrames.lean` (`frameVectors`, `derivedVectors`,
//! `refuteVectors`), parsed from the Lean text so neither side can change
//! alone, plus the Rust-only controls the vectors do not reach.

use std::collections::BTreeSet;

use qcore_core::index_extent::{
    Count, Decision, Facts, IndexFact, PointerFact, Refusal, ScalarFact, Standing, Unit,
};
use qcore_core::index_extent_frames::{
    BodyEffect, CallFrame, FrameMode, FrameTouch, derived_within, frame_at_call, multi_base_view,
    refutes,
};

const LEAN: &str = include_str!("../../../proof/QCore/IndexExtentFrames.lean");

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

fn flag(term: &Term) -> bool {
    match name(term).0 {
        "true" => true,
        "false" => false,
        other => panic!("bool {other}"),
    }
}

fn list(term: &Term) -> &[Term] {
    match term {
        Term::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn option<T>(term: &Term, inner: impl Fn(&Term) -> T) -> Option<T> {
    match name(term) {
        ("none", []) => None,
        ("some", [value]) => Some(inner(value)),
        other => panic!("option {other:?}"),
    }
}

/// Variables are `vN`, so a symbol names the variable whose write withdraws it.
fn var(term: &Term) -> String {
    format!("v{}", int(term))
}

fn count(term: &Term) -> Count {
    match name(term) {
        (".const", [n]) => Count::Const(int(n) as u128),
        (".sym", [s]) => Count::Symbol(var(s)),
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

fn mode(term: &Term) -> FrameMode {
    match name(term).0 {
        ".reads" => FrameMode::Reads,
        ".writes" => FrameMode::Writes,
        ".moves" => FrameMode::Moves,
        other => panic!("mode {other}"),
    }
}

fn effect(term: &Term) -> BodyEffect {
    match name(term).0 {
        ".read" => BodyEffect::Read,
        ".written" => BodyEffect::Written,
        ".moved" => BodyEffect::Moved,
        other => panic!("effect {other}"),
    }
}

fn frame(term: &Term) -> CallFrame {
    match name(term) {
        (".unstated", []) => CallFrame::Unstated,
        (".preserves", []) => CallFrame::PreservesExtents,
        (".closed", [touches, held]) => CallFrame::Closed {
            touches: list(touches)
                .iter()
                .map(|touch| match name(touch) {
                    (".mk", [m, addr]) => FrameTouch {
                        mode: mode(m),
                        addr: option(addr, var),
                    },
                    other => panic!("touch {other:?}"),
                })
                .collect(),
            held: flag(held),
        },
        other => panic!("frame {other:?}"),
    }
}

fn facts(pointers: &Term, scalars: &Term) -> Facts {
    let mut facts = Facts::default();
    for pointer in list(pointers) {
        let (".mk", [v, source, mention, held]) = name(pointer) else {
            panic!("pointer fact {pointer:?}")
        };
        facts.pointers.insert(
            var(v),
            PointerFact {
                object: format!("object of {}", var(v)),
                count: option(mention, var).map_or(Count::Const(4), Count::Symbol),
                unit: Unit::Elements,
                offset: IndexFact::Const(0),
                standing: if flag(held) {
                    Standing::Held
                } else {
                    Standing::Resolved
                },
                source_object: flag(source),
            },
        );
    }
    for scalar in list(scalars) {
        let (".mk", [v, value]) = name(scalar) else {
            panic!("scalar fact {scalar:?}")
        };
        facts.scalars.insert(
            var(v),
            ScalarFact {
                value: index(value),
                standing: Standing::Source,
            },
        );
    }
    facts
}

fn vars(term: &Term) -> BTreeSet<String> {
    list(term).iter().map(var).collect()
}

#[test]
fn cg24_frame_vectors_match_the_lean_model() {
    let rows = rows(
        "def frameVectors : List (Frame × List PFact × List Fact × List Nat × List Nat × List Nat) := [",
    );
    assert_eq!(rows.len(), 11);
    for row in rows {
        let [
            frame_term,
            pointers,
            scalars,
            kept,
            kept_scalars,
            held_after,
        ] = row.as_slice()
        else {
            panic!("{row:?}")
        };
        let frame = frame(frame_term);
        let mut state = facts(pointers, scalars);
        let reported = state.call_frame(&frame);
        let got: BTreeSet<String> = state.pointers.keys().cloned().collect();
        assert_eq!(got, vars(kept), "{frame:?}: pointer facts kept");
        let got: BTreeSet<String> = state.scalars.keys().cloned().collect();
        assert_eq!(got, vars(kept_scalars), "{frame:?}: scalar facts kept");
        let held: BTreeSet<String> = state
            .pointers
            .iter()
            .filter(|(_, fact)| fact.standing == Standing::Held)
            .map(|(name, _)| name.clone())
            .collect();
        assert_eq!(held, vars(held_after), "{frame:?}: held after");
        assert_eq!(
            reported.into_iter().collect::<BTreeSet<_>>(),
            if matches!(frame, CallFrame::Closed { held: true, .. }) {
                vars(held_after)
            } else {
                BTreeSet::new()
            },
            "{frame:?}: reported holds"
        );
    }
}

#[test]
fn cg24_derived_vectors_match_the_lean_model() {
    let rows = rows("def derivedVectors : List (Count × IndexFact × Count × Decision) := [");
    assert_eq!(rows.len(), 11);
    for row in rows {
        let [n, start, len, expected] = row.as_slice() else {
            panic!("{row:?}")
        };
        let (n, start, len) = (count(n), index(start), count(len));
        assert_eq!(
            derived_within(&n, &start, &len),
            decision(expected),
            "{n:?} {start:?} {len:?}"
        );
    }
}

#[test]
fn cg24_refute_vectors_match_the_lean_model() {
    let rows = rows("def refuteVectors : List (Mode × BodyEffect × Bool) := [");
    assert_eq!(rows.len(), 9);
    for row in rows {
        let [m, e, expected] = row.as_slice() else {
            panic!("{row:?}")
        };
        assert_eq!(refutes(mode(m), effect(e)), flag(expected), "{row:?}");
    }
}

#[test]
fn cg24_a_refuted_or_absent_declaration_is_unstated() {
    let touches = vec![FrameTouch {
        mode: FrameMode::Reads,
        addr: None,
    }];
    assert_eq!(
        frame_at_call(Some(touches.clone()), true, false),
        CallFrame::Unstated
    );
    assert_eq!(frame_at_call(None, false, false), CallFrame::Unstated);
    assert_eq!(
        frame_at_call(Some(touches.clone()), false, true),
        CallFrame::Closed {
            touches,
            held: true
        }
    );
}

#[test]
fn cg24_one_contradicted_base_refuses_a_multi_base_view() {
    assert!(multi_base_view(&[
        Decision::InBounds,
        Decision::Undecided(Refusal::MissingExtent)
    ]));
    assert!(!multi_base_view(&[
        Decision::InBounds,
        Decision::OutOfBounds
    ]));
    assert!(multi_base_view(&[]));
}

#[test]
fn cg24_derived_containment_refuses_on_overflow_rather_than_wrapping() {
    // Lean's integers are unbounded; the Rust decision refuses instead of
    // wrapping (a stricter answer, never a looser one).
    assert_eq!(
        derived_within(
            &Count::Const(u128::MAX),
            &IndexFact::Const(1),
            &Count::Const(1)
        ),
        Decision::Undecided(Refusal::Overflow)
    );
    assert_eq!(
        derived_within(
            &Count::Const(8),
            &IndexFact::Const(i128::MAX),
            &Count::Const(1)
        ),
        Decision::Undecided(Refusal::Overflow)
    );
}
