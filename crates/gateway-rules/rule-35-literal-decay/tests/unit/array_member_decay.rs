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

//! Rule 35, arm 2: the decision table, its Lean binding, the element reader
//! and the package's answers through a toy host.

use qcore_gateway_rule_api::traits::{RulePackage, TypedReaderArm};
use qcore_gateway_rule_api::{Answer, TypedReading, Witness};

use super::*;
use crate::toy::{Node, Toy};

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
        include_str!("../../../../../proof/QCore/LiteralDecayReadings.lean").replace("\r\n", "\n");
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

fn decay(operand: Node) -> Node {
    Node::new("ImplicitCastExpr")
        .cast("ArrayToPointerDecay")
        .of(operand)
}

#[test]
fn only_a_decay_cast_names_an_operand() {
    let member = Node::new("MemberExpr").typed("unsigned int[4]");
    let cast = decay(Node::new("ParenExpr").of(member.clone()));
    let operand = decayed_operand::<Toy>(&cast).expect("decayed operand");
    assert_eq!(operand, &member);
    assert_eq!(decide_operand::<Toy>(operand), Decision::Admit);
    let other = Node::new("ImplicitCastExpr")
        .cast("LValueToRValue")
        .of(member.clone());
    assert!(decayed_operand::<Toy>(&other).is_none());
    assert!(
        decayed_operand::<Toy>(&Node::new("ImplicitCastExpr").cast("ArrayToPointerDecay"))
            .is_none()
    );
    let local = Node::new("DeclRefExpr").typed("unsigned int[4]");
    assert_eq!(decide_operand::<Toy>(&local), Decision::NotMember);
    let volatile = Node::new("MemberExpr").typed("volatile int[4]");
    assert_eq!(decide_operand::<Toy>(&volatile), Decision::Qualified);
}

#[test]
fn the_package_reads_decays_admits_scalar_members_and_witnesses_its_decision() {
    let arm = ArrayMemberDecay;
    assert_eq!(arm.id(), "35");
    let member = Node::new("MemberExpr").typed("unsigned int[4]");
    let cast = decay(member.clone());
    assert!(arm.reads(&Toy, &cast));
    assert_eq!(
        arm.ask(&Toy, &cast),
        Answer::admit(TypedReading::ElementAddress(&member))
    );
    assert_eq!(
        arm.witness(&Toy, &cast),
        Some(Witness {
            subject: "decayed_member",
            fields: vec![
                ("kind", Some("MemberExpr")),
                ("type", Some("unsigned int[4]")),
                ("element", Some("scalar")),
            ],
            admitted: true,
            refusal: None,
        })
    );

    let records = decay(Node::new("MemberExpr").typed("struct item[8]"));
    assert!(arm.reads(&Toy, &records));
    assert_eq!(arm.ask(&Toy, &records), Answer::Refuse);
    let witness = arm.witness(&Toy, &records).expect("decided");
    assert_eq!(
        (witness.admitted, witness.refusal),
        (false, Some("element"))
    );

    let untyped = decay(Node::new("MemberExpr"));
    assert_eq!(arm.ask(&Toy, &untyped), Answer::Refuse);
    let witness = arm.witness(&Toy, &untyped).expect("decided");
    assert_eq!(witness.fields[1], ("type", None));
    assert_eq!(witness.fields[2], ("element", Some("other")));

    let not_a_decay = Node::new("MemberExpr").typed("unsigned int[4]");
    assert!(!arm.reads(&Toy, &not_a_decay));
    assert_eq!(arm.ask(&Toy, &not_a_decay), Answer::Refuse);
    assert_eq!(arm.witness(&Toy, &not_a_decay), None);
}
