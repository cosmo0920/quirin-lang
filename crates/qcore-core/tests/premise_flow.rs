use qcore_core::{
    hypothesis::Disposition::{self, Open, Refuted, Supported},
    premise_flow,
};
fn agrees(s: Disposition, p: bool) -> bool {
    match s {
        Open => true,
        Supported => p,
        Refuted => !p,
    }
}
#[test]
fn every_partial_assignment_preserves_positive_and_negative_logical_soundness() {
    for a in [Open, Supported, Refuted] {
        for b in [Open, Supported, Refuted] {
            for p in [false, true] {
                for q in [false, true] {
                    if agrees(a, p) && agrees(b, q) {
                        assert!(agrees(premise_flow::and(a, b), p && q));
                        assert!(agrees(premise_flow::or(a, b), p || q));
                        assert!(agrees(premise_flow::except(a, b), p && !q));
                        let (s, t) = premise_flow::terminal(a, b);
                        assert!(agrees(s, p && q));
                        if t {
                            assert!(p && q)
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn missing_negation_and_terminal_obligations_never_succeed() {
    assert_eq!(premise_flow::except(Supported, Open), Open);
    assert_eq!(premise_flow::terminal(Supported, Open), (Open, false));
    assert_eq!(premise_flow::terminal(Supported, Refuted), (Refuted, false));
    assert_eq!(premise_flow::except(Supported, Refuted), Supported);
    assert_eq!(premise_flow::or(Supported, Open), Supported);
    assert_eq!(premise_flow::or(Refuted, Open), Open);
}
