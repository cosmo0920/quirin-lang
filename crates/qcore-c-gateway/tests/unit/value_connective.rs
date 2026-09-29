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
    let store = json!({"kind": "BinaryOperator", "opcode": "=", "inner": [read("0x1"), read("0x2")]});
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
