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
