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

use super::*;
use crate::resource_flow::Quantity;

fn q(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

#[test]
fn positive_and_negative_net_loops_have_checked_invariants() {
    let growing = Flow::acquire(q(2), q(2));
    let grown = summarize_general_loop(&growing, LoopBoundEvidence::Exact(3)).unwrap();
    assert_eq!(grown.summary.net().get("memory"), 6);
    assert_eq!(grown.summary.peak().get("memory"), 6);
    assert_eq!(grown.summary.total().get("memory"), 6);
    assert_eq!(grown.summary.required_entry().get("memory"), 0);
    check_loop_summary(&grown).unwrap();

    let shrinking = Flow::release(q(2), Quantity::zero());
    let shrunk = summarize_general_loop(&shrinking, LoopBoundEvidence::Exact(3)).unwrap();
    assert_eq!(shrunk.summary.net().get("memory"), -6);
    assert_eq!(shrunk.summary.required_entry().get("memory"), 6);
}

#[test]
fn unbounded_loop_refuses_a_finite_resource_claim() {
    assert_eq!(
        summarize_general_loop(&Flow::zero(), LoopBoundEvidence::Unbounded),
        Err(LoopSummaryError::Unbounded)
    );
}

#[test]
fn ledger_morphisms_compose_only_at_equal_endpoints() {
    let first = LedgerMorphism {
        name: "acquire".into(),
        source: Ledger::new(),
        target: BTreeMap::from([("memory".into(), 2)]),
        delta: BTreeMap::from([("memory".into(), 2)]),
    };
    let second = LedgerMorphism {
        name: "release".into(),
        source: first.target.clone(),
        target: Ledger::new(),
        delta: BTreeMap::from([("memory".into(), -2)]),
    };
    let composed = first.then(&second).unwrap();
    assert!(composed.source.is_empty());
    assert!(composed.target.is_empty());
    let mut broken = second;
    broken.source.insert("memory".into(), 3);
    broken.delta.insert("memory".into(), -3);
    assert!(matches!(
        first.then(&broken),
        Err(CategoryError::EndpointMismatch { .. })
    ));
}

#[test]
fn maximum_counts_cannot_forge_exact_final_custody() {
    for body in [Flow::acquire(q(1), q(1)), Flow::release(q(1), q(1))] {
        for bound in 0..=2 {
            for evidence in [
                LoopBoundEvidence::AtMost(bound),
                LoopBoundEvidence::Terminating(bound),
                LoopBoundEvidence::Widened(bound),
            ] {
                let result = summarize_general_loop(&body, evidence);
                if bound == 0 {
                    assert_eq!(result.unwrap().summary, Flow::zero());
                } else {
                    assert_eq!(result, Err(LoopSummaryError::NonExactCustody));
                }
            }
            let exact = summarize_general_loop(&body, LoopBoundEvidence::Exact(bound)).unwrap();
            check_loop_summary(&exact).unwrap();
            assert_eq!(exact.summary, body.repeat_exact(bound).unwrap());
            if bound != 0 {
                assert!(!Flow::zero().is_covered_by(&exact.summary));
                let mut forged = exact.clone();
                forged.evidence = LoopBoundEvidence::AtMost(bound);
                assert_eq!(
                    check_loop_summary(&forged),
                    Err(LoopSummaryError::NonExactCustody)
                );
                assert_eq!(
                    loop_ledger_morphism(&forged),
                    Err(CategoryError::InvalidLoopSummary)
                );
            }
        }
    }
}

#[test]
fn zero_net_maximum_covers_every_admitted_early_exit() {
    for body in [
        Flow::acquire(q(2), q(1))
            .then(&Flow::release(q(2), q(1)))
            .unwrap(),
        Flow::release(q(2), q(1))
            .then(&Flow::acquire(q(2), q(1)))
            .unwrap(),
    ] {
        for bound in 0..=2 {
            let certificate =
                summarize_general_loop(&body, LoopBoundEvidence::AtMost(bound)).unwrap();
            check_loop_summary(&certificate).unwrap();
            for actual in 0..=bound {
                assert!(
                    body.repeat_exact(actual)
                        .unwrap()
                        .is_covered_by(&certificate.summary)
                );
            }
            if bound == 0 {
                assert_eq!(certificate.summary, Flow::zero());
            }
            if bound == 2 {
                let mut forged = certificate;
                forged.summary = body.repeat_exact(1).unwrap();
                assert_eq!(
                    check_loop_summary(&forged),
                    Err(LoopSummaryError::InvalidCertificate)
                );
            }
        }
    }
    // Scalar balance does not prove which allocation identities survive;
    // the summary API deliberately carries no identity discharge claim.
}

/// R06 matrix bodies: positive, negative and both zero-net orderings.
/// Each entry is `(label, body, net change per iteration)`.
fn r06_matrix_bodies() -> Vec<(&'static str, Flow, i128)> {
    vec![
        ("positive-net", Flow::acquire(q(1), q(1)), 1),
        ("negative-net", Flow::release(q(1), q(1)), -1),
        (
            "zero-net-acquire-first",
            Flow::acquire(q(1), q(1))
                .then(&Flow::release(q(1), q(1)))
                .unwrap(),
            0,
        ),
        (
            "zero-net-release-first",
            Flow::release(q(1), q(1))
                .then(&Flow::acquire(q(1), q(1)))
                .unwrap(),
            0,
        ),
    ]
}

#[test]
fn exact_and_maximum_evidence_are_distinct_across_the_r06_matrix() {
    for (label, body, per_iteration) in r06_matrix_bodies() {
        for count in 0..=2u64 {
            // Exact evidence is admitted for every net sign and every count,
            // and its net custody change is exactly `count * body net`.
            let exact = summarize_general_loop(&body, LoopBoundEvidence::Exact(count))
                .unwrap_or_else(|_| {
                    panic!("exact evidence must be admitted for {label} at {count}")
                });
            check_loop_summary(&exact).unwrap();
            assert_eq!(exact.summary, body.repeat_exact(count).unwrap(), "{label}");
            assert_eq!(
                exact.summary.net().get("memory"),
                i128::from(count) * per_iteration,
                "{label}"
            );

            let maximum = summarize_general_loop(&body, LoopBoundEvidence::AtMost(count));
            if per_iteration != 0 && count != 0 {
                // A positive maximum over a nonzero-net body has no exact
                // final custody state and is refused, not approximated.
                assert_eq!(
                    maximum,
                    Err(LoopSummaryError::NonExactCustody),
                    "{label} at {count}"
                );
                // Early exit is not admitted under exact evidence either: no
                // shorter run is covered by the exact summary.
                for actual in 0..=2u64 {
                    let observed = body.repeat_exact(actual).unwrap();
                    assert_eq!(
                        observed.is_covered_by(&exact.summary),
                        actual == count,
                        "{label}: exact({count}) coverage of {actual}"
                    );
                }
                continue;
            }

            let maximum =
                maximum.unwrap_or_else(|_| panic!("zero-net maximum must be admitted for {label}"));
            check_loop_summary(&maximum).unwrap();
            assert!(maximum.summary.net().is_zero(), "{label}");
            if count == 0 {
                assert_eq!(maximum.summary, Flow::zero(), "{label}");
            }
            // Every admitted early exit, including zero iterations, is covered.
            for actual in 0..=count {
                assert!(
                    body.repeat_exact(actual)
                        .unwrap()
                        .is_covered_by(&maximum.summary),
                    "{label}: at-most({count}) coverage of {actual}"
                );
            }
        }
    }
}

#[test]
fn evidence_mutation_never_retains_an_exact_final_custody_claim() {
    for (label, body, per_iteration) in r06_matrix_bodies() {
        for count in 0..=2u64 {
            let exact = summarize_general_loop(&body, LoopBoundEvidence::Exact(count)).unwrap();
            let mut forged = exact.clone();
            forged.evidence = LoopBoundEvidence::AtMost(count);
            if per_iteration != 0 && count != 0 {
                // Independent replay refuses the mutated certificate; the
                // ledger interpretation refuses it too, so no downstream
                // consumer can read an exact final state off it.
                assert_eq!(
                    check_loop_summary(&forged),
                    Err(LoopSummaryError::NonExactCustody),
                    "{label} at {count}"
                );
                assert_eq!(
                    loop_ledger_morphism(&forged),
                    Err(CategoryError::InvalidLoopSummary),
                    "{label} at {count}"
                );
                continue;
            }
            // Where replay admits the mutation the two readings denote the
            // same ledger: the net change is zero under both, so there is no
            // exact custody claim left for the weaker evidence to retain.
            check_loop_summary(&forged).unwrap();
            assert!(forged.summary.net().is_zero(), "{label} at {count}");
            if count == 0 {
                // A zero-count loop denotes the empty flow under both readings,
                // whatever the body's net change is.
                assert_eq!(forged.summary, Flow::zero(), "{label}");
            } else {
                assert_eq!(
                    body.repeat_exact(count).unwrap(),
                    body.bounded_zero_net_loop(count).unwrap(),
                    "{label} at {count}"
                );
            }
            // The reverse mutation is admitted on exactly the same ground.
            let maximum = summarize_general_loop(&body, LoopBoundEvidence::AtMost(count)).unwrap();
            let mut strengthened = maximum;
            strengthened.evidence = LoopBoundEvidence::Exact(count);
            check_loop_summary(&strengthened).unwrap();
            assert_eq!(strengthened.summary, exact.summary, "{label} at {count}");
        }
    }
}

#[test]
fn loop_ledger_morphisms_separate_exact_and_maximum_deltas() {
    let acquiring =
        summarize_general_loop(&Flow::acquire(q(1), q(1)), LoopBoundEvidence::Exact(2)).unwrap();
    let grown = loop_ledger_morphism(&acquiring).unwrap();
    assert_eq!(grown.source.get("memory"), None);
    assert_eq!(grown.target.get("memory").copied(), Some(2));
    assert_eq!(grown.delta.get("memory").copied(), Some(2));

    let releasing =
        summarize_general_loop(&Flow::release(q(1), q(1)), LoopBoundEvidence::Exact(2)).unwrap();
    let shrunk = loop_ledger_morphism(&releasing).unwrap();
    assert_eq!(shrunk.source.get("memory").copied(), Some(2));
    assert_eq!(shrunk.target.get("memory").copied(), Some(0));
    assert_eq!(shrunk.delta.get("memory").copied(), Some(-2));

    let balanced = Flow::acquire(q(1), q(1))
        .then(&Flow::release(q(1), q(1)))
        .unwrap();
    let bounded = summarize_general_loop(&balanced, LoopBoundEvidence::AtMost(2)).unwrap();
    // A maximum count contributes no custody delta; only cost is scaled.
    assert!(loop_ledger_morphism(&bounded).unwrap().delta.is_empty());
    assert_eq!(bounded.summary.total().get("memory"), 4);
    // A summary swapped for a shorter exact run is refused by replay.
    let mut forged = bounded;
    forged.summary = balanced.repeat_exact(1).unwrap();
    assert_eq!(
        check_loop_summary(&forged),
        Err(LoopSummaryError::InvalidCertificate)
    );
    assert_eq!(
        loop_ledger_morphism(&forged),
        Err(CategoryError::InvalidLoopSummary)
    );
}
