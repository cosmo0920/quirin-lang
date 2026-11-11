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

//! A toy host for the package's own tests: tokens are strings and a literal
//! is spelled `LIT` (so the package is seen to take the spelling from the
//! host), and typed nodes are a small owned tree.

use qcore_gateway_rule_api::{Host, LexicalHost, TypedHost};

pub(crate) struct Toy;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Node {
    pub kind: &'static str,
    pub cast: Option<&'static str>,
    pub ty: Option<&'static str>,
    pub children: Vec<Node>,
}

impl Node {
    pub(crate) fn new(kind: &'static str) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub(crate) fn typed(mut self, ty: &'static str) -> Self {
        self.ty = Some(ty);
        self
    }

    pub(crate) fn cast(mut self, cast: &'static str) -> Self {
        self.cast = Some(cast);
        self
    }

    pub(crate) fn of(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }
}

impl Host for Toy {
    type Token = &'static str;
    type Decision = ();
    type Statement = ();
    type Condition = ();
    type Node = Node;

    fn token_text(token: &Self::Token) -> &str {
        token
    }
    fn token_line(_: &Self::Token) -> usize {
        1
    }
    fn refusal(_: &Self::Decision) -> Option<&str> {
        None
    }
}

impl LexicalHost for Toy {
    fn is_literal(token: &Self::Token) -> bool {
        *token == "LIT"
    }
}

impl TypedHost for Toy {
    fn node_kind(node: &Node) -> &str {
        node.kind
    }
    fn node_attribute<'n>(node: &'n Node, key: &str) -> Option<&'n str> {
        (key == "castKind").then_some(node.cast).flatten()
    }
    fn node_type(node: &Node) -> Option<&str> {
        node.ty
    }
    fn node_children(node: &Node) -> &[Node] {
        &node.children
    }
}

/// Split a line of space-separated tokens.
pub(crate) fn tokens(line: &'static str) -> Vec<&'static str> {
    line.split_whitespace().collect()
}
