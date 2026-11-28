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

//! Card K2-BUILTIN-BIT-COUNTS (rule 94): the builtin table, the guard
//! reading, the decision, and their binding to `QCore.BitCountBuiltins`.

use crate::bit_count_builtins::{
    BUILTINS, Cond, DECISION_VECTORS, Guard, PROOF_VECTORS, RULE, Reason, Site, SiteFacts, code,
    decide, is_bit_count, matched, neutral_contract, proves, vector_cond, vector_facts,
};
use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

const LEAN: &str = include_str!("../../../../proof/QCore/BitCountBuiltins.lean");

#[test]
fn the_table_is_the_six_bit_counts_and_lean_s() {
    assert_eq!(RULE, "94");
    let names = BUILTINS
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def builtinNames : List String := [{names}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    for (name, _) in BUILTINS {
        assert!(is_bit_count(name));
    }
    for other in [
        "__builtin_popcount",
        "__builtin_ffs",
        "__builtin_clrsb",
        "clz",
        "__builtin_expect",
    ] {
        assert!(!is_bit_count(other), "{other}");
    }
}

#[test]
fn the_neutral_contract_borrows_its_one_scalar_and_returns_no_custody() {
    let contract = neutral_contract();
    assert_eq!(contract.parameters, vec![ResourceParameterMode::Borrowed]);
    assert_eq!(contract.result, ResourceReturnMode::None);
    assert!(!contract.has_container_effect());
    assert!(contract.parameter_kinds.is_empty());
    assert!(contract.link_members.is_empty());
}

#[test]
fn the_proof_vectors_hold_and_are_lean_s() {
    for (shape, edge, expected) in PROOF_VECTORS {
        assert_eq!(
            proves(&vector_cond(shape), edge),
            expected,
            "{shape} on {edge}"
        );
    }
    let rows = PROOF_VECTORS
        .iter()
        .map(|(s, e, x)| format!("(\"{s}\", {e}, {x})"))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def proofVectors : List (String × Bool × Bool) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn the_decision_vectors_hold_and_are_lean_s() {
    for (shape, answer) in DECISION_VECTORS {
        assert_eq!(code(decide(&vector_facts(shape))), answer, "{shape}");
    }
    let rows = DECISION_VECTORS
        .iter()
        .map(|(s, a)| format!("(\"{s}\", \"{a}\")"))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def decisionVectors : List (String × String) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn a_condition_that_says_nothing_of_x_proves_nothing() {
    for edge in [true, false] {
        assert!(!proves(&Cond::Other, edge));
        assert!(!proves(&Cond::Zero, edge));
        // `x < 0` is not read: always false for an unsigned `x`, and the
        // rule claims nothing about signed ranges.
        let lt = Cond::Compare("<", Box::new(Cond::Read), Box::new(Cond::Zero));
        assert!(!proves(&lt, edge));
    }
    // `x & 1` is no read of `x`; `x - 1` neither.
    assert!(!proves(
        &Cond::Compare("&", Box::new(Cond::Read), Box::new(Cond::Zero)),
        true
    ));
}

#[test]
fn every_refusal_has_its_key_and_no_guard_admits_an_unstable_object() {
    let unstable = SiteFacts {
        named_operand: true,
        stable: false,
        structured: true,
        guards: vec![Guard::Expression {
            cond: vector_cond("x"),
            edge: true,
        }],
    };
    assert_eq!(decide(&unstable), Err(Reason::Unstable));
    assert_eq!(Reason::Unmatched.key(), "bit-count-sites-unmatched");
}

#[test]
fn sites_match_by_line_and_callee_multiset() {
    let site = |line, callee: &str| Site {
        callee: callee.to_owned(),
        line,
        decision: Ok(()),
    };
    let tokens = vec![
        (3, "__builtin_clz".to_owned()),
        (3, "__builtin_clz".to_owned()),
    ];
    assert!(matched(
        &tokens,
        &[site(3, "__builtin_clz"), site(3, "__builtin_clz")]
    ));
    assert!(!matched(&tokens, &[site(3, "__builtin_clz")]));
    assert!(!matched(
        &tokens,
        &[site(3, "__builtin_clz"), site(4, "__builtin_clz")]
    ));
    assert!(!matched(&tokens[..1], &[site(3, "__builtin_ctz")]));
}
