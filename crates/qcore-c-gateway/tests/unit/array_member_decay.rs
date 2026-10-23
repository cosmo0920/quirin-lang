//! Card W48-BU-SINGLE-LEAF-BODIES (rule 35, arm 2): the decision table, its
//! Lean binding and the element reader.

use serde_json::json;

use super::*;

fn element(code: &str) -> Element {
    match code {
        "scalar" => Element::Scalar,
        "pointer" => Element::Pointer,
        "record" => Element::Record,
        "array" => Element::Array,
        "other" => Element::Other,
        other => panic!("unknown element {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for &(member, class, qualified, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(member, element(class), qualified).code(),
            expected,
            "{member} {class} {qualified}"
        );
    }
}

fn kebab(lean: &str) -> String {
    let mut code = String::new();
    for character in lean.trim_start_matches('.').chars() {
        if character.is_ascii_uppercase() {
            code.push('-');
            code.push(character.to_ascii_lowercase());
        } else {
            code.push(character);
        }
    }
    code
}

#[test]
fn the_lean_decision_vectors_are_the_rust_table() {
    let lean =
        include_str!("../../../../proof/QCore/LiteralDecayReadings.lean").replace("\r\n", "\n");
    let block = lean
        .split("def decayVectors ")
        .nth(1)
        .expect("decayVectors in the Lean model");
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
            assert_eq!(fields.len(), 4, "{row}");
            (
                fields[0] == "true",
                kebab(fields[1]),
                fields[2] == "true",
                kebab(fields[3]),
            )
        })
        .collect::<Vec<_>>();
    let rust = DECISION_VECTORS
        .iter()
        .map(|&(m, e, q, d)| (m, e.to_owned(), q, d.to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_element_reader_names_each_class() {
    for (spelling, expected) in [
        ("unsigned int[4]", Element::Scalar),
        ("char[16]", Element::Scalar),
        ("const unsigned char[32]", Element::Scalar),
        ("unsigned long long[4]", Element::Scalar),
        ("char *[4]", Element::Pointer),
        ("int (*[2])(void)", Element::Pointer),
        ("struct item[8]", Element::Record),
        ("union u[2]", Element::Record),
        ("char[4][4]", Element::Array),
        ("enum e[3]", Element::Other),
        ("my_t[3]", Element::Other),
        ("int", Element::Other),
        ("int[", Element::Other),
    ] {
        assert_eq!(element_of(spelling), expected, "{spelling}");
    }
}

#[test]
fn only_a_decay_cast_names_an_operand() {
    let member = json!({"kind": "MemberExpr", "type": {"qualType": "unsigned int[4]"}});
    let decay = json!({"kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
        "inner": [{"kind": "ParenExpr", "inner": [member.clone()]}]});
    let operand = decayed_operand(&decay).expect("decayed operand");
    assert_eq!(operand, &member);
    assert_eq!(decide_operand(operand), Decision::Admit);
    let other = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "inner": [member.clone()]});
    assert!(decayed_operand(&other).is_none());
    let local = json!({"kind": "DeclRefExpr", "type": {"qualType": "unsigned int[4]"}});
    assert_eq!(decide_operand(&local), Decision::NotMember);
    let volatile = json!({"kind": "MemberExpr", "type": {"qualType": "volatile int[4]"}});
    assert_eq!(decide_operand(&volatile), Decision::Qualified);
}
