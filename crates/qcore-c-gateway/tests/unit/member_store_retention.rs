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

//! Card CG10-MEMBER-STORE-RETAINED-BY (rule 70): the flow decision table and
//! its mechanical binding to `QCore.MemberStoreRetention`.

use super::*;

fn owner(code: &str) -> OwnerClass {
    [
        OwnerClass::Untracked,
        OwnerClass::Tracked,
        OwnerClass::Unresolved,
    ]
    .into_iter()
    .find(|class| class.code() == code)
    .expect("owner class")
}

#[test]
fn rule_70_decision_vectors_hold() {
    for &(carries, owner_code, decision) in DECISION_VECTORS {
        assert_eq!(
            decide(carries, owner(owner_code)).code(),
            decision,
            "{carries} {owner_code}"
        );
    }
}

/// The mechanical binding of `proof/QCore/MemberStoreRetention.lean`: its
/// decision vectors are the Rust ones, row by row.
#[test]
fn rule_70_lean_vectors_match_the_rust_table() {
    let lean = include_str!("../../../../proof/QCore/MemberStoreRetention.lean");
    let table = lean
        .split("def decisionVectors : List (Bool × OwnerClass × Decision) := [")
        .nth(1)
        .expect("decisionVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let lean_code = |name: &str| -> String {
        let name = name.trim().trim_start_matches('.');
        let mut code = String::new();
        for character in name.chars() {
            if character.is_ascii_uppercase() {
                code.push('-');
                code.push(character.to_ascii_lowercase());
            } else {
                code.push(character);
            }
        }
        code
    };
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(|line| {
            let line = line.trim().trim_end_matches(',');
            let fields = line[1..line.len() - 1]
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>();
            (fields[0] == "true", lean_code(fields[1]), lean_code(fields[2]))
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), DECISION_VECTORS.len());
    for (row, vector) in rows.iter().zip(DECISION_VECTORS) {
        assert_eq!(row.0, vector.0);
        assert_eq!(row.1, vector.1);
        assert_eq!(row.2, vector.2);
    }
}

/// A rule-70 decision never admits what rule 10v's table refuses for a
/// reason other than the missing owner premise: a tracked or unresolved
/// owner, or a value that carries an identity.
#[test]
fn rule_70_admits_only_an_untracked_owner_and_an_identity_free_value() {
    for carries in [false, true] {
        for owner_class in [
            OwnerClass::Untracked,
            OwnerClass::Tracked,
            OwnerClass::Unresolved,
        ] {
            let admitted = decide(carries, owner_class) == Decision::Admit;
            assert_eq!(
                admitted,
                !carries && owner_class == OwnerClass::Untracked,
                "{carries} {owner_class:?}"
            );
        }
    }
}
