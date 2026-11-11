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

//! Rule 36's source readings over a toy typed AST: the conjunction admits
//! exactly the confirmed original formal, and each missing or contradicting
//! fact declines. The gateway's own tests drive the same readings over
//! Clang's JSON (`crates/qcore-c-gateway/src/legacy_exact_formal_return.rs`
//! and `tests/derived_result_returned.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_gateway_rule_api::Host;

use super::{
    FormalReturnHost, FunctionReading, SourceHost, candidate, exact_formal_returns, inspect_body,
    original_formal, pointer_formals,
};

#[derive(Clone, Debug, Default)]
struct Node {
    kind: &'static str,
    attributes: Vec<(&'static str, &'static str)>,
    fields: Vec<(&'static str, Node)>,
    ty: Option<&'static str>,
    children: Vec<Node>,
    spelled: Option<Vec<&'static str>>,
    line: Option<usize>,
}

fn node(kind: &'static str) -> Node {
    Node {
        kind,
        ..Node::default()
    }
}

impl Node {
    fn with(mut self, key: &'static str, value: &'static str) -> Self {
        self.attributes.push((key, value));
        self
    }
    fn typed(mut self, ty: &'static str) -> Self {
        self.ty = Some(ty);
        self
    }
    fn field(mut self, key: &'static str, value: Node) -> Self {
        self.fields.push((key, value));
        self
    }
    fn child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }
    fn spelled(mut self, tokens: &[&'static str], line: usize) -> Self {
        self.spelled = Some(tokens.to_vec());
        self.line = Some(line);
        self
    }
}

struct Toy {
    unbound: bool,
}

impl Host for Toy {
    type Token = (&'static str, usize);
    type Decision = ();
    type Statement = ();
    type Condition = ();
    type Node = Node;

    fn token_text(token: &Self::Token) -> &str {
        token.0
    }
    fn token_line(token: &Self::Token) -> usize {
        token.1
    }
    fn refusal(_: &Self::Decision) -> Option<&str> {
        None
    }
}

impl SourceHost for Toy {
    fn node_kind(node: &Node) -> &str {
        node.kind
    }
    fn node_attribute<'n>(node: &'n Node, key: &str) -> Option<&'n str> {
        node.attributes
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| *value)
    }
    fn node_field<'n>(node: &'n Node, key: &str) -> Option<&'n Node> {
        node.fields
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value)
    }
    fn node_type(node: &Node) -> Option<&str> {
        node.ty
    }
    fn node_children(node: &Node) -> &[Node] {
        &node.children
    }
    fn pointer_type(text: &str) -> bool {
        text.trim_end().ends_with('*')
            && !text.contains("volatile")
            && !text.contains("_Atomic")
            && !text.contains('(')
    }
}

impl FormalReturnHost for Toy {
    fn formal_unbound(&self, _: &str) -> bool {
        self.unbound
    }
    fn node_tokens(&self, node: &Node) -> Option<Arc<Vec<String>>> {
        node.spelled
            .as_ref()
            .map(|tokens| Arc::new(tokens.iter().map(|t| (*t).to_owned()).collect()))
    }
    fn node_line(&self, node: &Node) -> Option<usize> {
        node.line
    }
}

fn formal_reference(id: &'static str) -> Node {
    node("DeclRefExpr").typed("char *").field(
        "referencedDecl",
        node("ParmVarDecl")
            .with("kind", "ParmVarDecl")
            .with("name", "p")
            .with("id", id),
    )
}

/// `char *identity(int n, char *p) { return p; }` as the toy AST.
fn function(returned: Node) -> Node {
    node("FunctionDecl")
        .child(
            node("ParmVarDecl")
                .with("name", "n")
                .with("id", "n-1")
                .typed("int"),
        )
        .child(
            node("ParmVarDecl")
                .with("name", "p")
                .with("id", "p-1")
                .typed("char *"),
        )
        .child(
            node("CompoundStmt").child(
                node("ReturnStmt").spelled(&["return", "p"], 1).child(
                    node("ImplicitCastExpr")
                        .typed("char *")
                        .with("castKind", "LValueToRValue")
                        .child(returned),
                ),
            ),
        )
}

const BODY: [(&str, usize); 3] = [("return", 1), ("p", 1), (";", 1)];

fn read(
    host: &Toy,
    function: &Node,
    body: Option<&[(&'static str, usize)]>,
) -> Option<BTreeMap<(usize, String), usize>> {
    let formals = [Some("n".to_owned()), Some("p".to_owned())];
    let pointer = BTreeSet::from([1]);
    exact_formal_returns(
        host,
        &FunctionReading::<Toy> {
            formals: &formals,
            pointer_formals: &pointer,
            function,
            body,
        },
    )
}

#[test]
fn the_conjunction_confirms_the_original_formal_and_declines_missing_facts() {
    let host = Toy { unbound: true };
    let confirmed = BTreeMap::from([((1, "p".to_owned()), 1)]);
    assert_eq!(
        read(&host, &function(formal_reference("p-1")), Some(&BODY)),
        Some(confirmed)
    );
    // A second identity for the formal's name declines the whole reading.
    assert_eq!(
        read(&host, &function(formal_reference("other")), Some(&BODY)),
        None
    );
    // A returned value that is not the original object pointer confirms no
    // return.
    assert_eq!(
        read(
            &host,
            &function(formal_reference("p-1").typed("int")),
            Some(&BODY)
        ),
        Some(BTreeMap::new())
    );
    // The typed root reader does not answer, or reads the formal as bound.
    assert_eq!(
        read(
            &Toy { unbound: false },
            &function(formal_reference("p-1")),
            Some(&BODY)
        ),
        None
    );
    // The lexical reader cannot delimit the body.
    assert_eq!(read(&host, &function(formal_reference("p-1")), None), None);
    // A lexical return the AST does not hold.
    let twice = [
        ("return", 1),
        ("p", 1),
        (";", 1),
        ("return", 2),
        ("p", 2),
        (";", 2),
    ];
    assert_eq!(
        read(&host, &function(formal_reference("p-1")), Some(&twice)),
        Some(BTreeMap::from([((1, "p".to_owned()), 1)]))
    );
    // A written formal is not eligible.
    let mut written = function(formal_reference("p-1"));
    written.children[2].children.insert(
        0,
        node("BinaryOperator")
            .with("opcode", "=")
            .child(formal_reference("p-1"))
            .child(formal_reference("p-1")),
    );
    assert_eq!(read(&host, &written, Some(&BODY)), Some(BTreeMap::new()));
    // A return node whose span is incomplete declines the whole reading.
    let mut unspelled = function(formal_reference("p-1"));
    unspelled.children[2].children[0].spelled = None;
    assert_eq!(read(&host, &unspelled, Some(&BODY)), None);
}

#[test]
fn only_value_preserving_casts_of_object_pointers_reach_the_formal() {
    assert!(original_formal::<Toy>(&formal_reference("p-1"), "p", "p-1"));
    assert!(!original_formal::<Toy>(
        &formal_reference("p-1"),
        "p",
        "other"
    ));
    assert!(!original_formal::<Toy>(
        &formal_reference("p-1"),
        "q",
        "p-1"
    ));
    for ty in ["int", "char *volatile", "_Atomic(char *)", "char (*)(void)"] {
        assert!(
            !original_formal::<Toy>(&formal_reference("p-1").typed(ty), "p", "p-1"),
            "{ty}"
        );
    }
    assert!(original_formal::<Toy>(
        &formal_reference("p-1").typed("char *const"),
        "p",
        "p-1"
    ));
    for cast in [
        "FunctionToPointerDecay",
        "NullToPointer",
        "IntegralToPointer",
    ] {
        let wrapped = node("ImplicitCastExpr")
            .typed("char *")
            .with("castKind", cast)
            .child(formal_reference("p-1"));
        assert!(!original_formal::<Toy>(&wrapped, "p", "p-1"), "{cast}");
    }
    let mut unnamed = formal_reference("p-1");
    unnamed.fields.clear();
    assert!(!original_formal::<Toy>(&unnamed, "p", "p-1"));
}

#[test]
fn a_shadowing_value_declines_and_writes_are_recorded() {
    let mut names = BTreeMap::from([("p".to_owned(), "p-1".to_owned())]);
    let shadow = node("EnumConstantDecl")
        .with("name", "p")
        .with("id", "p-enum");
    assert!(inspect_body::<Toy>(&shadow, &mut names, &mut BTreeSet::new()).is_none());
    let mut written = BTreeSet::new();
    let address = node("UnaryOperator")
        .with("opcode", "&")
        .child(formal_reference("p-1"));
    assert!(inspect_body::<Toy>(&address, &mut BTreeMap::new(), &mut written).is_some());
    assert!(written.contains("p-1"));
}

#[test]
fn pointer_formals_need_a_positive_matching_depth_one_pointer_result() {
    let pointer = BTreeSet::from(["ptr<mut, u8>"]);
    let int = BTreeSet::from(["i32"]);
    let parameters = [Some(int.clone()), Some(pointer.clone()), None];
    assert_eq!(
        pointer_formals(false, Some(&pointer), &parameters),
        BTreeSet::from([1])
    );
    assert!(pointer_formals(true, Some(&pointer), &parameters).is_empty());
    assert!(pointer_formals(false, None, &parameters).is_empty());
    assert!(pointer_formals(false, Some(&int), &parameters).is_empty());
    let two = BTreeSet::from(["ptr<mut, u8>", "ptr<const, u8>"]);
    assert!(pointer_formals(false, Some(&two), &parameters).is_empty());
}

#[test]
fn the_candidate_needs_a_bare_return_of_a_pointer_formal() {
    let formals = [Some("n".to_owned()), Some("p".to_owned())];
    let body = [("return", 1), ("p", 1), (";", 1)];
    assert!(candidate::<Toy>(&formals, &body, || BTreeSet::from([1])));
    assert!(!candidate::<Toy>(&formals, &body, || BTreeSet::from([0])));
    let scalar = [("return", 1), ("n", 1), (";", 1)];
    assert!(!candidate::<Toy>(&formals, &scalar, || BTreeSet::from([1])));
    let expression = [("return", 1), ("p", 1), ("+", 1), ("1", 1), (";", 1)];
    assert!(!candidate::<Toy>(&formals, &expression, || panic!(
        "not read"
    )));
}
