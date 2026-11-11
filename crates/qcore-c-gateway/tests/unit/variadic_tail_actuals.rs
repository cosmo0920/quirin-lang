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

//! Unit tests of card CG26-TAIL-RESIDUAL-ACTUALS (gateway rule 12g): the
//! reading of a trailing actual from its Clang node, and its shared vectors
//! with `QCore.VariadicTailActuals`.

use super::*;
use serde_json::json;

const LEAN: &str = include_str!("../../../../proof/QCore/VariadicTailActuals.lean");

/// A tiny reader of the Lean spelling of `Expr`, `Reading` and `List Nat`.
struct Lean<'s> {
    tokens: Vec<&'s str>,
    at: usize,
}

impl<'s> Lean<'s> {
    fn new(text: &'s str) -> Self {
        let mut tokens = Vec::new();
        let mut start = None;
        for (index, ch) in text.char_indices() {
            if matches!(ch, '(' | ')' | '[' | ']' | ',') || ch.is_whitespace() {
                if let Some(begin) = start.take() {
                    tokens.push(&text[begin..index]);
                }
                if !ch.is_whitespace() {
                    tokens.push(&text[index..index + ch.len_utf8()]);
                }
            } else if start.is_none() {
                start = Some(index);
            }
        }
        if let Some(begin) = start {
            tokens.push(&text[begin..]);
        }
        Self { tokens, at: 0 }
    }
    fn next(&mut self) -> &'s str {
        let token = self.tokens[self.at];
        self.at += 1;
        token
    }
    fn expect(&mut self, wanted: &str) {
        assert_eq!(self.next(), wanted);
    }
    fn number(&mut self) -> usize {
        self.next().parse().expect("a Nat")
    }
    fn boolean(&mut self) -> bool {
        match self.next() {
            "true" => true,
            "false" => false,
            other => panic!("not a Bool: {other}"),
        }
    }
    fn expr(&mut self) -> TailExpr {
        match self.next() {
            ".lit" => TailExpr::Literal,
            ".other" => TailExpr::Other,
            "(" => {
                let head = self.next();
                let expression = match head {
                    ".name" => {
                        let name = format!("v{}", self.number());
                        TailExpr::Name {
                            name,
                            scalar: self.boolean(),
                        }
                    }
                    ".unary" => TailExpr::Unary(Box::new(self.expr())),
                    ".binary" => TailExpr::Binary {
                        pointer: self.boolean(),
                        left: Box::new(self.expr()),
                        right: Box::new(self.expr()),
                    },
                    ".cond" => TailExpr::Conditional(
                        Box::new(self.expr()),
                        Box::new(self.expr()),
                        Box::new(self.expr()),
                    ),
                    ".cast" => TailExpr::Cast(Box::new(self.expr())),
                    other => panic!("not an Expr: {other}"),
                };
                self.expect(")");
                expression
            }
            other => panic!("not an Expr: {other}"),
        }
    }
    fn reading(&mut self) -> TailReading {
        match self.next() {
            ".data" => TailReading::Data,
            "(" => {
                let reading = match self.next() {
                    ".borrowed" => TailReading::Borrowed(format!("v{}", self.number())),
                    ".refused" => TailReading::Refused(match self.next() {
                        ".cast" => TailRefusal::Cast,
                        ".pointerArithmetic" => TailRefusal::PointerArithmetic,
                        ".identityArm" => TailRefusal::IdentityArm,
                        ".identityOperand" => TailRefusal::IdentityOperand,
                        ".identityUnderDataTail" => TailRefusal::IdentityUnderDataTail,
                        ".integerIdentity" => TailRefusal::IntegerIdentity,
                        ".unansweredRoot" => TailRefusal::UnansweredRoot,
                        ".unread" => TailRefusal::Unread,
                        other => panic!("not a Refusal: {other}"),
                    }),
                    other => panic!("not a Reading: {other}"),
                };
                self.expect(")");
                reading
            }
            other => panic!("not a Reading: {other}"),
        }
    }
    fn names(&mut self) -> Vec<usize> {
        self.expect("[");
        let mut out = Vec::new();
        loop {
            match self.next() {
                "]" => return out,
                "," => {}
                number => out.push(number.parse().expect("a Nat")),
            }
        }
    }
}

fn run(expression: &TailExpr, carrying: &[usize], answered: &[usize], tail: bool) -> TailReading {
    let has = |list: &[usize], name: &str| list.iter().any(|n| format!("v{n}") == name);
    read(
        expression,
        &NameFacts {
            carries: &|name, _| has(carrying, name),
            answered: &|name| has(answered, name),
        },
        tail,
    )
}

/// The Rust reader agrees with every shared vector, and the Lean
/// `readVectors` are `READ_VECTORS`, row by row.
#[test]
fn cg26r_read_vectors_are_the_lean_vectors() {
    let body = LEAN
        .split("def readVectors : List (Expr × List Nat × List Nat × Bool × Reading) := [")
        .nth(1)
        .and_then(|rest| rest.split("\n]").next())
        .expect("readVectors");
    let rows = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), READ_VECTORS.len());
    for (row, (expression, carrying, answered, tail, expected)) in rows.iter().zip(READ_VECTORS) {
        let mut lean = Lean::new(row);
        lean.expect("(");
        let lean_expression = lean.expr();
        lean.expect(",");
        let lean_carrying = lean.names();
        lean.expect(",");
        let lean_answered = lean.names();
        lean.expect(",");
        let lean_tail = lean.boolean();
        lean.expect(",");
        let lean_expected = lean.reading();
        assert_eq!(lean_expression, Lean::new(expression).expr(), "{row}");
        assert_eq!(lean_carrying, *carrying, "{row}");
        assert_eq!(lean_answered, *answered, "{row}");
        assert_eq!(lean_tail, *tail, "{row}");
        assert_eq!(lean_expected, Lean::new(expected).reading(), "{row}");
        assert_eq!(
            run(&lean_expression, carrying, answered, *tail),
            lean_expected,
            "{row}"
        );
    }
}

fn reference(name: &str, qual: &str) -> serde_json::Value {
    json!({"kind": "DeclRefExpr", "type": {"qualType": qual},
           "referencedDecl": {"kind": "VarDecl", "name": name}})
}

fn implicit(inner: serde_json::Value, qual: &str) -> serde_json::Value {
    json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
           "type": {"qualType": qual}, "inner": [inner]})
}

#[test]
fn cg26r_the_node_shapes() {
    let i = implicit(reference("i", "int"), "int");
    assert_eq!(
        TailExpr::from_node(&i),
        TailExpr::Name {
            name: "i".into(),
            scalar: true
        }
    );
    let buf = json!({"kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
                     "type": {"qualType": "char *"}, "inner": [reference("buf", "char[256]")]});
    assert_eq!(
        TailExpr::from_node(&buf),
        TailExpr::Name {
            name: "buf".into(),
            scalar: false
        }
    );
    let sum = json!({"kind": "BinaryOperator", "opcode": "+", "type": {"qualType": "int"},
                     "inner": [i.clone(), {"kind": "IntegerLiteral", "type": {"qualType": "int"}}]});
    assert!(matches!(
        TailExpr::from_node(&sum),
        TailExpr::Binary { pointer: false, .. }
    ));
    let offset = json!({"kind": "BinaryOperator", "opcode": "+", "type": {"qualType": "char *"},
                        "inner": [buf.clone(), i.clone()]});
    assert!(matches!(
        TailExpr::from_node(&offset),
        TailExpr::Binary { pointer: true, .. }
    ));
    // Card CAST-FORMALIZATION (rule 12h), a deliberate flip: a scalar
    // conversion is read through to its operand's own reading
    // (`cast_semantics::tail_transparent`); a pointer made an integer stays a
    // cast, refused.
    let cast = json!({"kind": "CStyleCastExpr", "castKind": "IntegralCast",
                      "type": {"qualType": "long"}, "inner": [i.clone()]});
    assert_eq!(
        TailExpr::from_node(&cast),
        TailExpr::Name {
            name: "i".into(),
            scalar: true
        }
    );
    let pointer = implicit(reference("p", "char *"), "char *");
    let integer = json!({"kind": "CStyleCastExpr", "castKind": "PointerToIntegral",
                         "type": {"qualType": "unsigned long"}, "inner": [pointer]});
    assert!(matches!(TailExpr::from_node(&integer), TailExpr::Cast(_)));
    let sizeof = json!({"kind": "UnaryExprOrTypeTraitExpr", "name": "sizeof",
                        "type": {"qualType": "unsigned long"}, "argType": {"qualType": "char[256]"}});
    assert_eq!(TailExpr::from_node(&sizeof), TailExpr::Literal);
    let vla = json!({"kind": "UnaryExprOrTypeTraitExpr", "name": "sizeof",
                     "type": {"qualType": "unsigned long"}, "argType": {"qualType": "char[n]"}});
    assert_eq!(TailExpr::from_node(&vla), TailExpr::Other);
    let address = json!({"kind": "UnaryOperator", "opcode": "&", "type": {"qualType": "int *"},
                         "inner": [reference("i", "int")]});
    assert_eq!(TailExpr::from_node(&address), TailExpr::Other);
    let increment = json!({"kind": "UnaryOperator", "opcode": "++", "isPostfix": true,
                           "type": {"qualType": "int"}, "inner": [reference("i", "int")]});
    assert_eq!(TailExpr::from_node(&increment), TailExpr::Other);
    let function = json!({"kind": "DeclRefExpr", "type": {"qualType": "int (int)"},
                          "referencedDecl": {"kind": "FunctionDecl", "name": "f"}});
    assert_eq!(TailExpr::from_node(&function), TailExpr::Other);
    let pointer_to_function = reference("cb", "int (*)(int)");
    assert_eq!(TailExpr::from_node(&pointer_to_function), TailExpr::Other);
    let record = reference("s", "struct s");
    assert_eq!(TailExpr::from_node(&record), TailExpr::Other);
    // A typedef of an anonymous enumeration has no desugared spelling; the
    // integral promotion Clang applied to it is the evidence of its type.
    let anonymous = json!({"kind": "ImplicitCastExpr", "castKind": "IntegralCast",
                           "type": {"qualType": "unsigned int"},
                           "inner": [implicit(reference("j", "rack_config_t"), "rack_config_t")]});
    assert_eq!(
        TailExpr::from_node(&anonymous),
        TailExpr::Name {
            name: "j".into(),
            scalar: true
        }
    );
    // Without that conversion the same spelling is not read.
    assert_eq!(
        TailExpr::from_node(&implicit(reference("j", "rack_config_t"), "rack_config_t")),
        TailExpr::Other
    );
    // A pointer-to-integer conversion is not an arithmetic source.
    let pointer = json!({"kind": "ImplicitCastExpr", "castKind": "PointerToIntegral",
                         "type": {"qualType": "long"},
                         "inner": [implicit(reference("q", "opaque_t"), "opaque_t")]});
    assert_eq!(TailExpr::from_node(&pointer), TailExpr::Other);
    let enumerator = json!({"kind": "DeclRefExpr", "type": {"qualType": "int"},
                            "referencedDecl": {"kind": "EnumConstantDecl", "name": "E"}});
    assert_eq!(TailExpr::from_node(&enumerator), TailExpr::Literal);
}
