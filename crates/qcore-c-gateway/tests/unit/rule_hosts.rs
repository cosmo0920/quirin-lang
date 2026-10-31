//! Card RULE35-PILOT: the gateway hosts rule 35's package. Its own decision
//! tables and Lean binding are the package's tests
//! (`crates/gateway-rules/rule-35-literal-decay`); these check the host: the
//! gateway's lexer and Clang JSON reach the package through the capabilities,
//! the registry switch gates both arms, and the parity row keeps the shape
//! rule 35 recorded before the move.

use serde_json::json;

use super::*;
use crate::gateway_rules::test_override::with_rules_off;

fn star_exempt(line: &str) -> Vec<bool> {
    let owned = crate::legacy_c::tokenize(line)
        .into_iter()
        .map(|token| token.text)
        .collect::<Vec<_>>();
    let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
    texts
        .iter()
        .enumerate()
        .filter(|(_, text)| **text == "*")
        .map(|(index, _)| literal_read(&texts, index))
        .collect()
}

#[test]
fn the_lexers_literal_reaches_the_literal_read_arm() {
    assert_eq!(star_exempt("!*\"fd not found\""), [true]);
    assert_eq!(star_exempt("c = *\"a\" \"b\" + 1"), [true]);
    assert_eq!(star_exempt("*\"a\" == 'a'"), [true]);
    assert_eq!(star_exempt("!*p"), [false]);
    assert_eq!(star_exempt("*(char *)\"a\""), [false, false]);
    assert_eq!(star_exempt("**\"a\""), [false, true]);
    assert_eq!(star_exempt("*\"a\" = 1"), [false]);
    assert_eq!(star_exempt("*\"a\" += 1"), [false]);
    assert_eq!(star_exempt("*\"a\" <<= 1"), [false]);
    assert_eq!(star_exempt("*\"a\"++"), [false]);
    assert_eq!(star_exempt("*\"a\" == 1"), [true]);
    with_rules_off("35", || {
        assert_eq!(star_exempt("!*\"fd not found\""), [false]);
    });
}

#[test]
fn the_clang_cast_reaches_the_decay_arm_and_the_row_keeps_its_shape() {
    let member = json!({"kind": "MemberExpr", "type": {"qualType": "unsigned int[4]"}});
    let decay = json!({"kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
        "inner": [{"kind": "ParenExpr", "inner": [member.clone()]}]});
    assert!(decay_reads(&decay));
    assert_eq!(
        decay_reading(&decay, None),
        Some(TypedReading::ElementAddress(&member))
    );
    let other = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "inner": [member.clone()]});
    assert!(!decay_reads(&other));
    let desugared = json!({"kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
        "inner": [{"kind": "MemberExpr",
            "type": {"qualType": "U32[4]", "desugaredQualType": "volatile unsigned int[4]"}}]});
    assert!(decay_reads(&desugared));
    assert_eq!(
        decay_reading(&desugared, None),
        None,
        "the desugared type decides"
    );

    let witness = rule_35_literal_decay::ArrayMemberDecay
        .witness(&ArmHost::HOST, &desugared)
        .expect("decided");
    assert_eq!(
        witness_row("35", &witness),
        json!({
            "rule": "35",
            "decayed_member": {
                "kind": "MemberExpr",
                "type": "volatile unsigned int[4]",
                "element": "other",
            },
            "admitted": false,
            "refusal": "qualified",
        })
    );
    let untyped = json!({"kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
        "inner": [{"kind": "DeclRefExpr"}]});
    let witness = rule_35_literal_decay::ArrayMemberDecay
        .witness(&ArmHost::HOST, &untyped)
        .expect("decided");
    assert_eq!(
        witness_row("35", &witness),
        json!({
            "rule": "35",
            "decayed_member": {"kind": "DeclRefExpr", "type": null, "element": "other"},
            "admitted": false,
            "refusal": "not-member",
        })
    );

    with_rules_off("35", || {
        assert!(!decay_reads(&decay));
        assert_eq!(decay_reading(&decay, None), None);
    });
}

/// Card ARM-OBLIGATIONS: a hosted arm's admission with obligations is
/// carried into the open unit frame, and refused with none open.
#[test]
fn an_admission_with_obligations_is_carried_or_refused() {
    use crate::arm_obligations::{Frame, carry_answer};
    use qcore_gateway_rule_api::{Admission, Answer, Obligation};
    let premised = || {
        Answer::Admit(Admission {
            lowering: 1,
            obligations: vec![Obligation::Premise("p".into())],
        })
    };
    assert_eq!(carry_answer("35", Answer::admit(1)), Some(1));
    assert_eq!(carry_answer("35", Answer::<u8>::Refuse), None);
    assert_eq!(carry_answer("35", premised()), None, "no frame: refused");
    let frame = Frame::open();
    assert_eq!(carry_answer("35", premised()), Some(1));
    assert_eq!(frame.close().premises.len(), 1);
}
