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

//! `qcore.obs-reordering.v1` against the model directly (card `OBS-REORDERING`).
//!
//! These exercise the relation itself, with hand-built declarations, so every
//! refusal arm is reached whether or not a source program can currently spell
//! it. The composed-route tests in `compiler/tests/obs_reordering.rs` exercise
//! the derivation and the native comparison.

use super::*;

fn parent(window: usize, text: &str) -> DeclaredRecord {
    DeclaredRecord {
        emitter: Emitter::Parent,
        window,
        text: text.to_owned(),
    }
}

fn worker(task: u64, window: usize, text: &str) -> DeclaredRecord {
    DeclaredRecord {
        emitter: Emitter::Worker(task),
        window,
        text: text.to_owned(),
    }
}

/// One barrier, one two-emitter window, one barrier — the shape every composed
/// program with a worker line has.
fn spawn_join_stream() -> AdmittedReordering {
    AdmittedReordering::declare(vec![
        parent(0, "before"),
        worker(1, 1, "w1a"),
        worker(1, 1, "w1b"),
        worker(2, 1, "w2a"),
        parent(1, "during"),
        parent(2, "after"),
    ])
    .expect("the declaration is well formed")
}

#[test]
fn the_declared_order_is_admitted() {
    let order = spawn_join_stream();
    let witness = order
        .compare(&["before", "w1a", "w1b", "w2a", "during", "after"])
        .expect("the declared order is admitted");
    assert!(witness.is_declared_order());
    assert_eq!(witness.declared_positions, vec![0, 1, 2, 3, 4, 5]);
}

/// The point of the capability: a stream that is **not** the declared order is
/// admitted, and the witness says which window moved.
#[test]
fn a_reordered_window_is_admitted_and_witnessed() {
    let order = spawn_join_stream();
    let witness = order
        .compare(&["before", "w2a", "w1a", "during", "w1b", "after"])
        .expect("a commuting reordering is admitted");
    assert!(!witness.is_declared_order());
    assert_eq!(witness.reordered_windows, vec![1]);
    // Each observed position names the declared record it carries.
    assert_eq!(witness.declared_positions, vec![0, 3, 1, 4, 2, 5]);
}

#[test]
fn one_emitters_own_lines_never_commute() {
    let order = spawn_join_stream();
    // `w1b` before `w1a` is worker 1 writing out of its own program order.
    let refusal = order
        .compare(&["before", "w1b", "w1a", "w2a", "during", "after"])
        .unwrap_err();
    assert_eq!(
        refusal,
        ReorderingRefusal::EmitterOrder {
            window: 1,
            emitter: Emitter::Worker(1)
        }
    );
    assert_eq!(refusal.code(), 5);
}

#[test]
fn a_barrier_commutes_with_nothing() {
    let order = spawn_join_stream();
    // `before` is alone in window 0 and `w1a` is in window 1: swapping them
    // crosses a barrier, which is a length-preserving change the window
    // decomposition refuses.
    let refusal = order
        .compare(&["w1a", "before", "w1b", "w2a", "during", "after"])
        .unwrap_err();
    assert_eq!(
        refusal,
        ReorderingRefusal::Residue {
            window: 0,
            index: 0,
            text: "w1a".to_owned()
        }
    );
}

#[test]
fn the_barriers_are_the_records_alone_in_their_window() {
    let order = spawn_join_stream();
    let barriers: Vec<&str> = order
        .barriers()
        .into_iter()
        .map(|record| record.text.as_str())
        .collect();
    assert_eq!(barriers, vec!["before", "after"]);
    assert_eq!(order.window_sizes(), vec![1, 4, 1]);
}

#[test]
fn a_residue_line_is_refused_rather_than_ignored() {
    let order = spawn_join_stream();
    let refusal = order
        .compare(&["before", "w1a", "w1b", "smuggled", "during", "after"])
        .unwrap_err();
    assert_eq!(
        refusal,
        ReorderingRefusal::Residue {
            window: 1,
            index: 3,
            text: "smuggled".to_owned()
        }
    );
    assert_eq!(refusal.code(), 4);
}

#[test]
fn a_dropped_line_is_a_length_mismatch() {
    let order = spawn_join_stream();
    assert_eq!(
        order
            .compare(&["before", "w1a", "w1b", "during", "after"])
            .unwrap_err(),
        ReorderingRefusal::LengthMismatch {
            declared: 6,
            observed: 5
        }
    );
}

/// A duplicated line inside a window has the right length and the right
/// alphabet, and is still refused: the duplicating emitter's projection is not
/// the declared one.
#[test]
fn a_duplicated_line_is_refused() {
    let order = spawn_join_stream();
    assert_eq!(
        order
            .compare(&["before", "w1a", "w1a", "w2a", "during", "after"])
            .unwrap_err(),
        ReorderingRefusal::EmitterOrder {
            window: 1,
            emitter: Emitter::Worker(1)
        }
    );
}

/// **The fail-closed premise, refused at declaration time.** Two emitters that
/// would write the same bytes in one window make a native line's writer a
/// non-function of its bytes; the relation refuses rather than weakening
/// silently to a multiset comparison.
#[test]
fn two_emitters_of_one_window_may_not_share_a_line() {
    let refusal = AdmittedReordering::declare(vec![
        parent(0, "open"),
        worker(1, 1, "[INFO] 42"),
        worker(2, 1, "[INFO] 42"),
    ])
    .unwrap_err();
    assert_eq!(
        refusal,
        ReorderingRefusal::AmbiguousAlphabet {
            window: 1,
            text: "[INFO] 42".to_owned()
        }
    );
    assert_eq!(refusal.code(), 2);
}

/// The same bytes in two *different* windows are fine: they are separated by a
/// barrier, so no attribution question arises.
#[test]
fn the_same_line_in_two_windows_is_admitted() {
    let order = AdmittedReordering::declare(vec![
        worker(1, 0, "[INFO] 42"),
        parent(1, "barrier"),
        worker(2, 2, "[INFO] 42"),
    ])
    .expect("distinct windows are independent alphabets");
    order
        .compare(&["[INFO] 42", "barrier", "[INFO] 42"])
        .expect("the declared order is admitted");
}

/// One emitter may repeat a line: attribution is unambiguous, and the order of
/// its own repetitions is its own program order.
#[test]
fn one_emitter_may_repeat_a_line() {
    let order = AdmittedReordering::declare(vec![
        worker(1, 0, "tick"),
        worker(1, 0, "tick"),
        worker(2, 0, "tock"),
    ])
    .expect("one emitter repeating itself is not ambiguous");
    order
        .compare(&["tock", "tick", "tick"])
        .expect("worker 2 first is admitted");
}

#[test]
fn a_declaration_whose_windows_go_backwards_is_refused() {
    assert_eq!(
        AdmittedReordering::declare(vec![parent(3, "a"), parent(1, "b")]).unwrap_err(),
        ReorderingRefusal::WindowsNotMonotone { index: 1 }
    );
    assert_eq!(ReorderingRefusal::WindowsNotMonotone { index: 1 }.code(), 1);
}

/// **The degenerate case, and the reason every existing comparison is unchanged
/// by this card.** A relation whose windows each have at most one emitter admits
/// exactly one stream: its own. `is_strict` reports it and `compare` behaves as
/// byte equality.
#[test]
fn a_single_emitter_relation_is_byte_equality() {
    let order = AdmittedReordering::declare(vec![
        parent(0, "QCORE_TASK_SPAWN 1 0"),
        parent(1, "QCORE_TASK_RESULT 1 I32 42"),
        parent(2, "QCORE_TASK_JOIN 1"),
    ])
    .unwrap();
    assert!(order.is_strict());
    assert_eq!(order.admitted_streams(64).unwrap().len(), 1);
    assert_eq!(
        order.admitted_streams(64).unwrap()[0],
        order.declared_text()
    );
    order
        .compare_text(&order.declared_text())
        .expect("its own text is admitted");
    assert!(
        order
            .compare(&[
                "QCORE_TASK_RESULT 1 I32 42",
                "QCORE_TASK_SPAWN 1 0",
                "QCORE_TASK_JOIN 1"
            ])
            .is_err(),
        "a strict relation admits no transposition"
    );
}

/// A window with many records but one emitter is strict too — being concurrent
/// with nobody is what makes an order total, not being short.
#[test]
fn a_many_record_single_emitter_window_is_strict() {
    let order = AdmittedReordering::declare(vec![
        worker(1, 0, "a"),
        worker(1, 0, "b"),
        worker(1, 0, "c"),
    ])
    .unwrap();
    assert!(order.is_strict());
    assert_eq!(order.admitted_streams(64).unwrap().len(), 1);
}

#[test]
fn the_admitted_streams_are_bounded_on_request() {
    let order = spawn_join_stream();
    // Window 1 has three emitters with 2, 1 and 1 lines: 4!/(2!·1!·1!) = 12.
    assert_eq!(order.admitted_streams(64).unwrap().len(), 12);
    assert_eq!(order.admitted_streams(4), None);
}

/// Every stream the relation says it admits is a stream the comparison accepts,
/// and the comparison accepts nothing else of that length. Enumerated rather
/// than argued, on this relation.
#[test]
fn the_enumeration_and_the_comparison_agree() {
    let order = spawn_join_stream();
    let admitted = order.admitted_streams(1024).unwrap();
    for stream in &admitted {
        order
            .compare_text(stream)
            .unwrap_or_else(|e| panic!("{stream:?} was enumerated but refused: {e}"));
    }
    // Every permutation of the declared lines that is not in the enumeration is
    // refused. 6 records, 720 permutations, 12 admitted.
    let lines: Vec<&str> = order.records().iter().map(|r| r.text.as_str()).collect();
    let mut accepted = 0usize;
    permutations(&lines, &mut |candidate| {
        if order.compare(candidate).is_ok() {
            accepted += 1;
            let text: String = candidate.iter().map(|l| format!("{l}\n")).collect();
            assert!(
                admitted.contains(&text),
                "{candidate:?} is accepted but not enumerated"
            );
        }
    });
    assert_eq!(accepted, admitted.len());
}

fn permutations<'a>(items: &[&'a str], visit: &mut impl FnMut(&[&'a str])) {
    fn walk<'a>(
        items: &[&'a str],
        used: &mut Vec<bool>,
        prefix: &mut Vec<&'a str>,
        visit: &mut impl FnMut(&[&'a str]),
    ) {
        if prefix.len() == items.len() {
            visit(prefix);
            return;
        }
        for index in 0..items.len() {
            if used[index] {
                continue;
            }
            used[index] = true;
            prefix.push(items[index]);
            walk(items, used, prefix, visit);
            prefix.pop();
            used[index] = false;
        }
    }
    walk(items, &mut vec![false; items.len()], &mut Vec::new(), visit);
}

/// The equational characterisation: two observed streams are `R`-equivalent
/// exactly when their canonical forms agree.
#[test]
fn equivalence_is_equality_of_canonical_forms() {
    let order = spawn_join_stream();
    let a = ["before", "w1a", "w2a", "w1b", "during", "after"];
    let b = ["before", "w2a", "w1a", "during", "w1b", "after"];
    assert_eq!(
        canonical_of_observed(&order, &a).unwrap(),
        canonical_of_observed(&order, &b).unwrap()
    );
    assert_eq!(
        canonical_of_observed(&order, &a).unwrap(),
        order.canonical()
    );
}

/// `≡` is an equivalence relation, so "these two runs agree" is a property of
/// the pair and not of which one was called the reference.
#[test]
fn equivalence_is_reflexive_symmetric_and_transitive() {
    let order = spawn_join_stream();
    let admitted = order.admitted_streams(1024).unwrap();
    let canon = |text: &str| {
        let lines: Vec<&str> = text.lines().collect();
        canonical_of_observed(&order, &lines).unwrap()
    };
    for a in &admitted {
        assert_eq!(canon(a), canon(a), "reflexive");
        for b in &admitted {
            assert_eq!(canon(a) == canon(b), canon(b) == canon(a), "symmetric");
            for c in &admitted {
                if canon(a) == canon(b) && canon(b) == canon(c) {
                    assert_eq!(canon(a), canon(c), "transitive");
                }
            }
        }
    }
}

/// Soundness 1: an accepted stream carries the same whole-stream projection for
/// every emitter. This is what an accepted native run has actually been shown to
/// satisfy about each thread.
#[test]
fn an_accepted_stream_preserves_every_emitters_own_order() {
    let order = spawn_join_stream();
    let declared_projection = |emitter: Emitter| -> Vec<&str> {
        order
            .records()
            .iter()
            .filter(|record| record.emitter == emitter)
            .map(|record| record.text.as_str())
            .collect()
    };
    for stream in order.admitted_streams(1024).unwrap() {
        let lines: Vec<&str> = stream.lines().collect();
        let witness = order.compare(&lines).unwrap();
        for emitter in order.emitters() {
            let observed: Vec<&str> = lines
                .iter()
                .zip(&witness.declared_positions)
                .filter(|&(_, &position)| order.records()[position].emitter == emitter)
                .map(|(line, _)| *line)
                .collect();
            assert_eq!(observed, declared_projection(emitter));
        }
    }
}

/// Soundness 2: an accepted stream carries the barriers in the declared places.
#[test]
fn an_accepted_stream_preserves_the_barrier_structure() {
    let order = spawn_join_stream();
    let barrier_positions: Vec<usize> = order
        .window_sizes()
        .iter()
        .scan(0usize, |start, &len| {
            let at = *start;
            *start += len;
            Some((at, len))
        })
        .filter(|(_, len)| *len == 1)
        .map(|(at, _)| at)
        .collect();
    for stream in order.admitted_streams(1024).unwrap() {
        let lines: Vec<&str> = stream.lines().collect();
        for &position in &barrier_positions {
            assert_eq!(lines[position], order.records()[position].text);
        }
    }
}

#[test]
fn the_relation_identity_is_pinned() {
    assert_eq!(OBS_REORDERING_IDENTITY, "qcore.obs-reordering.v1");
}

#[test]
fn an_empty_stream_is_admitted_and_strict() {
    let order = AdmittedReordering::declare(Vec::new()).unwrap();
    assert!(order.is_strict());
    assert_eq!(order.window_sizes(), Vec::<usize>::new());
    order.compare(&[]).unwrap();
    assert_eq!(
        order.compare(&["unexpected"]).unwrap_err(),
        ReorderingRefusal::LengthMismatch {
            declared: 0,
            observed: 1
        }
    );
}

#[test]
fn a_refusal_renders_what_it_refused() {
    let refusal = ReorderingRefusal::Residue {
        window: 1,
        index: 3,
        text: "smuggled".to_owned(),
    };
    let rendered = refusal.to_string();
    assert!(rendered.contains("window 1"), "{rendered}");
    assert!(rendered.contains("smuggled"), "{rendered}");
    assert_eq!(Emitter::Worker(2).to_string(), "worker 2");
    assert_eq!(Emitter::Parent.to_string(), "parent");
}
