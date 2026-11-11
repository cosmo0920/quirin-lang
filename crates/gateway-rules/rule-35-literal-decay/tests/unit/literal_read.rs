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

//! Rule 35, arm 1: the decision table, its Lean binding and the token
//! reading through a toy host (the C gateway's own lexer is exercised by the
//! gateway's host tests).

use qcore_gateway_rule_api::Answer;
use qcore_gateway_rule_api::traits::{ClassificationArm, RulePackage, TokenQuery};

use super::*;
use crate::toy::{Toy, tokens};

fn follow_of(store: bool) -> Follow {
    if store { Follow::Store } else { Follow::Read }
}

#[test]
fn the_decision_vectors_hold() {
    for &(literal, store, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(literal, follow_of(store)).code(),
            expected,
            "{literal} {store}"
        );
    }
}

#[test]
fn the_lean_decision_vectors_are_the_rust_table() {
    let lean =
        include_str!("../../../../../proof/QCore/LiteralDecayReadings.lean").replace("\r\n", "\n");
    let block = lean
        .split("def literalVectors ")
        .nth(1)
        .expect("literalVectors in the Lean model");
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
                    ".literal" => "literal",
                    ".store" => "store",
                    ".notLiteral" => "not-literal",
                    other => panic!("unknown decision {other}"),
                },
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, DECISION_VECTORS.to_vec());
}

/// The package's answer at every `*` of a space-separated line.
fn star_exempt(line: &'static str) -> Vec<bool> {
    let tokens = tokens(line);
    (0..tokens.len())
        .filter(|&at| tokens[at] == "*")
        .map(|at| {
            let query = TokenQuery::<Toy> {
                tokens: &tokens,
                at,
            };
            let answer = ClassificationArm::ask(&LiteralRead, &Toy, &query);
            assert_eq!(answer == Answer::admit(()), exempt::<Toy>(&tokens, at));
            answer == Answer::admit(())
        })
        .collect()
}

#[test]
fn a_read_of_a_literal_is_exempt_and_every_other_star_is_not() {
    assert_eq!(star_exempt("! * LIT"), [true]);
    assert_eq!(star_exempt("c = * LIT LIT + 1"), [true]);
    assert_eq!(star_exempt("* LIT = = LIT"), [true]);
    assert_eq!(star_exempt("! * p"), [false]);
    assert_eq!(star_exempt("* ( char * ) LIT"), [false, false]);
    assert_eq!(star_exempt("* * LIT"), [false, true]);
    assert_eq!(
        star_exempt("* \"a\""),
        [false],
        "the host, not the package, spells literals"
    );
}

#[test]
fn a_store_or_increment_of_a_literal_is_not_exempt() {
    assert_eq!(star_exempt("* LIT = 1"), [false]);
    assert_eq!(star_exempt("* LIT + = 1"), [false]);
    assert_eq!(star_exempt("* LIT < < = 1"), [false]);
    assert_eq!(star_exempt("* LIT + +"), [false]);
    assert_eq!(star_exempt("* LIT = = 1"), [true]);
}

#[test]
fn only_a_star_is_ever_exempt() {
    let tokens = tokens("x LIT");
    assert!(!exempt::<Toy>(&tokens, 0));
    assert!(!exempt::<Toy>(&tokens, 5), "past the end");
    assert_eq!(LiteralRead.id(), "35");
}
