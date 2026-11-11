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

//! Unit tests of card CG06's value-region reading (gateway rule 10z) and the
//! mechanical binding to `proof/QCore/ValueConnectives.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/ValueConnectives.lean");

fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").unwrap()].to_owned()
}

#[test]
fn cg06_admission_table() {
    for (siblings, decision, inert, middle, admitted) in ADMISSION_VECTORS {
        assert_eq!(
            admit(siblings, decision, inert, middle).is_ok(),
            admitted,
            "{siblings} {decision} {inert} {middle:?}"
        );
    }
    // Each premise's own refusal.
    assert_eq!(
        admit(true, true, true, Some(false)),
        Err(Refusal::OmittedMiddleEffect)
    );
    assert_eq!(admit(false, true, true, None), Err(Refusal::SiblingEffect));
    assert_eq!(
        admit(true, false, false, None),
        Err(Refusal::UnorderedDecision)
    );
}

/// Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m): `admit_composed` on its
/// vectors, and the mechanical binding `composedVectors` = `COMPOSED_VECTORS`.
#[test]
fn cg06_composed_admission_table_and_lean_vectors() {
    use crate::connective_footprints::Composed;
    for (siblings, decision, inert, middle, composed, admitted) in COMPOSED_VECTORS {
        let compose = move || {
            if composed == Some(true) {
                Ok(())
            } else {
                Err(Composed::NonCallSibling)
            }
        };
        let outcome = admit_composed(
            siblings,
            decision,
            inert,
            middle,
            composed.is_some().then_some(&compose as &dyn Fn() -> _),
        );
        assert_eq!(
            outcome.is_ok(),
            admitted,
            "{siblings} {decision} {inert} {middle:?} {composed:?}: {outcome:?}"
        );
    }
    // A refusal the composition did not discharge names both.
    assert_eq!(
        admit_composed(
            false,
            true,
            true,
            None,
            Some(&|| Err(Composed::NonCallSibling))
        ),
        Err(Refusal::Composition(
            Box::new(Refusal::SiblingEffect),
            Composed::NonCallSibling
        ))
    );
    let option = |text: &str| match text.trim() {
        "none" => None,
        "some true" => Some(true),
        "some false" => Some(false),
        other => panic!("unknown Lean option {other}"),
    };
    let rows = block(
        "def composedVectors : List (Bool × Bool × Bool × Option Bool × Option Bool × Bool) := [",
    )
    .lines()
    .map(str::trim)
    .filter(|line| line.starts_with('('))
    .map(|line| {
        let fields = line
            .trim_end_matches(',')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(", ")
            .collect::<Vec<_>>();
        (
            fields[0] == "true",
            fields[1] == "true",
            fields[2] == "true",
            option(fields[3]),
            option(fields[4]),
            fields[5] == "true",
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(rows, COMPOSED_VECTORS.to_vec());
}

/// The mechanical binding: `admissionVectors` = `ADMISSION_VECTORS` and
/// `transparent` = `TRANSPARENT`.
#[test]
fn cg06_lean_vectors_match() {
    let option = |text: &str| match text.trim() {
        "none" => None,
        "some true" => Some(true),
        "some false" => Some(false),
        other => panic!("unknown Lean option {other}"),
    };
    let rows = block("def admissionVectors : List (Bool × Bool × Bool × Option Bool × Bool) := [")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let fields = line
                .trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(", ")
                .collect::<Vec<_>>();
            (
                fields[0] == "true",
                fields[1] == "true",
                fields[2] == "true",
                option(fields[3]),
                fields[4] == "true",
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, ADMISSION_VECTORS.to_vec());
    let positions = block("def transparent : List String := [")
        .lines()
        .filter_map(|line| line.split('"').nth(1))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(positions, TRANSPARENT.to_vec());
}

#[test]
fn cg06_effect_and_inert_readings() {
    use serde_json::json;
    let read = |id: &str| {
        json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue", "type": {"qualType": "int"},
               "inner": [{"kind": "DeclRefExpr", "referencedDecl": {"kind": "VarDecl", "id": id, "name": "v"}}]})
    };
    let call = json!({"kind": "CallExpr", "inner": []});
    let store =
        json!({"kind": "BinaryOperator", "opcode": "=", "inner": [read("0x1"), read("0x2")]});
    let unevaluated = json!({"kind": "UnaryExprOrTypeTraitExpr", "inner": [call.clone()]});
    assert!(effect_free(&read("0x1")));
    assert!(!effect_free(&call));
    assert!(!effect_free(&store));
    assert!(effect_free(&unevaluated));
    let unaliased = |id: &str| id == "0x1";
    assert!(inert(&read("0x1"), &unaliased));
    assert!(!inert(&read("0x2"), &unaliased));
    assert!(inert(
        &json!({"kind": "DeclRefExpr", "referencedDecl": {"kind": "VarDecl", "id": "0x9"}}),
        &unaliased
    ));
    let member = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "inner": [{"kind": "MemberExpr", "isArrow": true, "inner": [read("0x1")]}]});
    assert!(!inert(&member, &unaliased), "a read through a pointer");
}
