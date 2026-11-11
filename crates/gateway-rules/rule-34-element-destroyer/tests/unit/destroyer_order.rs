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

//! Rule 34's transfer arm: the decision table, its Lean binding and the
//! arm through the rule API's consultation (the gateway's host is exercised by
//! the gateway's `element_destroyer.rs` and `rule34_switch.rs`).

use qcore_gateway_rule_api::traits::RulePackage;
use qcore_gateway_rule_api::{Answer, DeclaredEffect, Phase, TransferArm, TransferQuery, consult};

use super::*;

const fn effect(unlinks: bool, releases: bool) -> DeclaredEffect {
    DeclaredEffect { unlinks, releases }
}

#[test]
fn the_decision_vectors_hold() {
    for &(unlinks, releases, expected) in DECISION_VECTORS {
        assert_eq!(
            code(phase(effect(unlinks, releases))),
            expected,
            "{unlinks} {releases}"
        );
    }
    assert_eq!(DECISION_VECTORS.len(), 4, "every effect shape");
}

#[test]
fn the_lean_phase_vectors_are_the_rust_table() {
    let lean = include_str!("../../../../../proof/QCore/ElementDestroyerSwitch.lean")
        .replace("\r\n", "\n");
    let block = lean
        .split("def phaseVectors ")
        .nth(1)
        .expect("phaseVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let fields = row
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 3, "{row}");
            (
                fields[0] == "true",
                fields[1] == "true",
                match fields[2] {
                    ".beforeCall" => "before",
                    ".afterCall" => "after",
                    other => panic!("unknown phase {other}"),
                },
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, DECISION_VECTORS.to_vec());
}

#[test]
fn a_call_without_a_destroyer_is_not_rule_34s_shape() {
    assert_eq!(order(&[]), Answer::Refuse);
    assert_eq!(
        order(&[
            effect(false, false),
            effect(true, false),
            effect(false, true)
        ]),
        Answer::Refuse,
        "a publication, an unlink beside a borrow, a released link"
    );
}

#[test]
fn a_destroyers_unlink_moves_before_the_call_and_nothing_else_does() {
    let effects = [
        effect(false, false),
        effect(true, true),
        effect(true, false),
    ];
    assert_eq!(
        order(&effects),
        Answer::admit(vec![Phase::AfterCall, Phase::BeforeCall, Phase::AfterCall])
    );
    let Answer::Admit(admission) = order(&effects) else {
        panic!("admitted");
    };
    assert!(admission.obligations.is_empty());
    assert_eq!(admission.lowering.len(), effects.len());
}

#[test]
fn the_package_answers_through_the_rule_api_under_its_registry_id() {
    let effects = [effect(true, true)];
    let query = TransferQuery {
        callee: "mk_http_thread_destroy",
        effects: &effects,
    };
    assert_eq!(ElementDestroyer.id(), "34");
    assert_eq!(crate::ID, "34");
    let packages = [ElementDestroyer];
    let on = consult(&packages, |_| true, |_, package| package.ask(&query))
        .expect("rule 34 orders the destroyer");
    assert_eq!(on.admission.lowering, [Phase::BeforeCall]);
    assert!(
        consult(
            &packages,
            |package| package.id() != "34",
            |_, package| package.ask(&query)
        )
        .is_none(),
        "switched off, the arm is not asked and the host decides"
    );
}
