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

fn q(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

fn acquire(amount: u64) -> Flow {
    Flow::acquire(q(amount), q(amount))
}

fn release(amount: u64) -> Flow {
    Flow::release(q(amount), Quantity::zero())
}

fn compose(flows: &[Flow]) -> Result<Flow, FlowError> {
    flows
        .iter()
        .try_fold(Flow::zero(), |summary, flow| summary.then(flow))
}

#[test]
fn sequential_acquisitions_add_live_and_total() {
    let summary = acquire(8).then(&acquire(5)).unwrap();
    assert_eq!(summary.net().get("memory"), 13);
    assert_eq!(summary.peak().get("memory"), 13);
    assert_eq!(summary.total().get("memory"), 13);
}

#[test]
fn release_then_reacquire_uses_maximum_peak_not_sum() {
    let summary = compose(&[acquire(10), release(10), acquire(7)]).unwrap();
    assert_eq!(summary.net().get("memory"), 7);
    assert_eq!(summary.peak().get("memory"), 10);
    assert_eq!(summary.total().get("memory"), 17);
}

#[test]
fn mutually_exclusive_branches_join_componentwise() {
    let left = acquire(4).then(&release(4)).unwrap();
    let right = acquire(9).then(&release(9)).unwrap();
    let joined = left.join_branches(&right).unwrap();
    assert_eq!(joined.net().get("memory"), 0);
    assert_eq!(joined.peak().get("memory"), 9);
    assert_eq!(joined.total().get("memory"), 9);
    let bounds = ResourceBounds::new(joined.peak.clone(), joined.total.clone());
    assert!(bounds.accepts(&interpret_flow_trace(&[left]).unwrap()));
    assert!(bounds.accepts(&interpret_flow_trace(&[right]).unwrap()));
}

#[test]
fn branch_join_rejects_different_post_live_states() {
    assert_eq!(
        acquire(1).join_branches(&Flow::zero()),
        Err(FlowError::BranchNetMismatch)
    );
}

#[test]
fn bounded_zero_net_loop_scales_only_total() {
    let body = acquire(3).then(&release(3)).unwrap();
    let loop_flow = body.bounded_zero_net_loop(12).unwrap();
    assert_eq!(loop_flow.net().get("memory"), 0);
    assert_eq!(loop_flow.peak().get("memory"), 3);
    assert_eq!(loop_flow.total().get("memory"), 36);
    let bounds = ResourceBounds::new(loop_flow.peak.clone(), loop_flow.total.clone());
    for iterations in 0..=12 {
        let trace = vec![body.clone(); iterations];
        assert!(bounds.accepts(&interpret_flow_trace(&trace).unwrap()));
    }
    assert_eq!(
        acquire(1).bounded_zero_net_loop(2),
        Err(FlowError::NonZeroLoopNet)
    );
}

#[test]
fn bounded_loop_alias_is_exact_repetition_not_a_maximum_count() {
    // `bounded_loop` is a historical spelling for exact repetition. Reading it
    // as a maximum count would admit a nonzero-net body that the maximum-count
    // summary refuses, so the two must stay distinguishable here.
    let growing = acquire(1);
    for iterations in 0..=3 {
        assert_eq!(
            growing.bounded_loop(iterations).unwrap(),
            growing.repeat_exact(iterations).unwrap()
        );
    }
    assert_eq!(growing.bounded_loop(2).unwrap().net().get("memory"), 2);
    assert_eq!(
        growing.bounded_zero_net_loop(2),
        Err(FlowError::NonZeroLoopNet)
    );
    let balanced = acquire(3).then(&release(3)).unwrap();
    assert_eq!(
        balanced.bounded_loop(4).unwrap(),
        balanced.bounded_zero_net_loop(4).unwrap()
    );
}

#[test]
fn function_summary_composes_at_call_site() {
    let callee = FunctionSummary::new(acquire(6).then(&release(6)).unwrap());
    let caller = acquire(2)
        .then(&callee.instantiate())
        .unwrap()
        .then(&release(2))
        .unwrap();
    assert_eq!(caller.net().get("memory"), 0);
    assert_eq!(caller.peak().get("memory"), 8);
    assert_eq!(caller.total().get("memory"), 8);

    let observed = interpret_flow_trace(&[acquire(2), callee.instantiate(), release(2)]).unwrap();
    assert_eq!(observed, caller);
}

#[test]
fn bounds_are_checked_on_every_axis() {
    let footprint =
        Quantity::from_axes([("memory".into(), 16), ("descriptors".into(), 1)]).unwrap();
    let flow = Flow::acquire(footprint.clone(), footprint);
    assert!(
        ResourceBounds::new(
            Quantity::from_axes([("memory".into(), 16), ("descriptors".into(), 1)]).unwrap(),
            Quantity::from_axes([("memory".into(), 32), ("descriptors".into(), 1)]).unwrap(),
        )
        .accepts(&flow)
    );
    assert!(!ResourceBounds::new(q(15), q(32)).accepts(&flow));
}

#[test]
fn generated_valid_traces_match_composed_summaries() {
    // Exhaust every valid length-six trace over acquire(1), acquire(2),
    // and release(1). This catches associativity/offset mistakes in peak.
    fn visit(prefix: &mut Vec<Flow>, live: u64, depth: usize) {
        if depth == 6 {
            let static_flow = compose(prefix).unwrap();
            let observed = interpret_flow_trace(prefix).unwrap();
            assert_eq!(static_flow, observed);
            assert!(
                ResourceBounds::new(static_flow.peak().clone(), static_flow.total().clone())
                    .accepts(&observed)
            );
            return;
        }
        for amount in [1, 2] {
            prefix.push(acquire(amount));
            visit(prefix, live + amount, depth + 1);
            prefix.pop();
        }
        if live > 0 {
            prefix.push(release(1));
            visit(prefix, live - 1, depth + 1);
            prefix.pop();
        }
    }
    visit(&mut Vec::new(), 0, 0);
}

#[test]
fn arithmetic_overflow_is_explicit() {
    assert_eq!(
        acquire(u64::MAX).then(&acquire(1)),
        Err(FlowError::Overflow)
    );
    let body = Flow {
        net: NetChange::zero(),
        required: Quantity::zero(),
        peak: q(1),
        total: q(u64::MAX),
    };
    assert_eq!(body.bounded_zero_net_loop(2), Err(FlowError::Overflow));
}
