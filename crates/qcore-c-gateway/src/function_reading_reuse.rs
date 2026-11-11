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

//! Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME** (gateway rules 13s, 13t): one
//! function's typed readings computed once per index and answered again from
//! it, so a statement hook no longer re-walks the whole body for an answer it
//! already has.
//!
//! # The cost
//!
//! Measured on librdkafka's `rdhttp.c` (132.5 s of the corpus arbiter's
//! 544.5 s, run last): `call_exits::resolve` walked the whole function once
//! per uncontracted call statement (33 % of the unit's samples, through the
//! parity-only call-exit census), and `Parser::elaboration_node` walked from
//! the function root per statement hook, computing every node's begin and end
//! line and tokenizing every candidate node's source span (44 %). The four
//! definitions calling `curl_easy_setopt` expand its GCC type-check macro into
//! subtrees whose every node lies on the call's line, so each walk visited all
//! of them and tokenized the same span again for each node and each statement;
//! their 908 calls name only 44 distinct warning functions.
//!
//! # What this module holds
//!
//! [`FunctionReuse`] belongs to one function of one unit index
//! (`typed_access_path`'s function entry, beside its line index): the index is
//! built from one Clang document of one launch identity (the full analysed
//! text, frontend request, dialect and Clang resolution environment,
//! `clang_reuse::UnitKey`), so every reading here keys on that document by
//! construction, and is dropped with it. Each reading is a pure function of
//! the function's own `Value`, the unit's source text and line starts, and
//! the unit's `noreturn` declaration ids, none of which changes while the
//! index lives:
//!
//! * **Rule 13s.** [`FunctionReuse::resolution`]: `call_exits::resolve` of a
//!   callee, computed by `resolve` itself on the first ask for that name and
//!   answered from the memo after ([`Memo`]).
//! * **Rule 13t.** [`LineTree`]: each node's begin and end line, read on the
//!   walk's first descent into the node and kept in the function's own child
//!   order, so a walk that reads lines from it visits exactly the nodes a walk
//!   computing them visits; and each source span's tokens, tokenized once
//!   ([`FunctionReuse::span_tokens`], a [`Memo`] too).
//!
//! A memo stores only what its own computation answered for the key, and a
//! key is answered from the memo only when that very key was stored: nothing
//! here is consulted for a decision the uncached readers would not make.
//! `QCore.DeepAstTransports` states the memo and the line tree's equivalence;
//! the unit tests compare every name and every node.

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use serde_json::Value;

use crate::call_exits::Resolution;
use crate::typed_c_ast::children;

/// A source span's tokens, or the absent answer, shared.
pub(crate) type SpanTokens = Option<Arc<Vec<String>>>;

/// A pure function's answers by key: the first computation for a key is
/// stored and every later ask of that key returns it.
pub(crate) struct Memo<K, V> {
    answers: Mutex<HashMap<K, V>>,
}

impl<K, V> Default for Memo<K, V> {
    fn default() -> Self {
        Self {
            answers: Mutex::new(HashMap::new()),
        }
    }
}

impl<K: Eq + Hash, V: Clone> Memo<K, V> {
    /// The stored answer for `key`, or `compute`'s, stored. `compute` runs
    /// outside the lock; a concurrent reader computes the same pure answer,
    /// and the first one stored stands.
    pub(crate) fn get_or_compute(&self, key: K, compute: impl FnOnce() -> V) -> V {
        if let Some(found) = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return found.clone();
        }
        let found = compute();
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key)
            .or_insert(found)
            .clone()
    }
}

/// One function's reusable readings (see the module documentation).
#[derive(Default)]
pub(crate) struct FunctionReuse {
    calls: Memo<String, Resolution>,
    lines: OnceLock<LineTree>,
    spans: Memo<(usize, usize), SpanTokens>,
}

/// A node's begin and end line (`None` where the node carries no offset or
/// one beyond the source), and its children's, in `inner` order. The children
/// are read on first descent, so a walk never computes a line the uncached
/// walk would not have computed, and a later walk reads it again from here.
/// A tree is only ever read beside the node it was built from (the walk pairs
/// each child with its own entry).
pub(crate) struct LineTree {
    pub(crate) begin: Option<usize>,
    pub(crate) end: Option<usize>,
    children: OnceLock<Vec<LineTree>>,
}

impl LineTree {
    fn of(
        node: &Value,
        begin: &dyn Fn(&Value) -> Option<usize>,
        end: &dyn Fn(&Value) -> Option<usize>,
    ) -> Self {
        Self {
            begin: begin(node),
            end: end(node),
            children: OnceLock::new(),
        }
    }

    /// The lines of `node`'s children (`node` is the node this tree was built
    /// from), in `inner` order.
    pub(crate) fn children(
        &self,
        node: &Value,
        begin: &dyn Fn(&Value) -> Option<usize>,
        end: &dyn Fn(&Value) -> Option<usize>,
    ) -> &[LineTree] {
        self.children.get_or_init(|| {
            children(node)
                .iter()
                .map(|child| Self::of(child, begin, end))
                .collect()
        })
    }
}

impl FunctionReuse {
    /// Rule 13s: `call_exits::resolve(function, callee, no_return)`, once
    /// per callee name of this function.
    pub(crate) fn resolution(
        &self,
        function: &Value,
        callee: &str,
        no_return_declarations: &BTreeSet<String>,
    ) -> Resolution {
        self.calls.get_or_compute(callee.to_owned(), || {
            crate::call_exits::resolve(function, callee, no_return_declarations)
        })
    }

    /// Rule 13t: the function's line tree (its root's lines; the rest on
    /// descent), with the index's own line readers.
    pub(crate) fn line_tree(
        &self,
        function: &Value,
        begin: &dyn Fn(&Value) -> Option<usize>,
        end: &dyn Fn(&Value) -> Option<usize>,
    ) -> &LineTree {
        self.lines
            .get_or_init(|| LineTree::of(function, begin, end))
    }

    /// Rule 13t: the tokens of the source span `start..end`, computed by
    /// `tokenize` once per span (the absent answer included).
    pub(crate) fn span_tokens(
        &self,
        span: (usize, usize),
        tokenize: impl FnOnce() -> Option<Vec<String>>,
    ) -> SpanTokens {
        self.spans.get_or_compute(span, || tokenize().map(Arc::new))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn walk<'a>(node: &'a Value, visit: &mut impl FnMut(&'a Value)) {
        visit(node);
        for child in children(node) {
            walk(child, visit);
        }
    }

    /// Every name a body mentions, and some it does not.
    fn names(function: &Value) -> BTreeSet<String> {
        let mut found = BTreeSet::from(["absent".to_owned(), String::new()]);
        walk(function, &mut |node| {
            for name in [
                node["name"].as_str(),
                node["referencedDecl"]["name"].as_str(),
            ]
            .into_iter()
            .flatten()
            {
                found.insert(name.to_owned());
            }
        });
        found
    }

    fn reference(name: &str, kind: &str, id: &str, qual: &str) -> Value {
        json!({"kind": "DeclRefExpr", "referencedDecl": {
            "id": id, "kind": kind, "name": name, "type": {"qualType": qual}}})
    }

    fn bodies() -> Vec<Value> {
        let call = |inner: Vec<Value>| json!({"kind": "CallExpr", "inner": inner});
        vec![
            json!({"kind": "FunctionDecl", "name": "f", "inner": [
                {"kind": "CompoundStmt", "inner": [
                    call(vec![reference("die", "FunctionDecl", "0x1", "void (void) __attribute__((noreturn))")]),
                    call(vec![reference("go", "FunctionDecl", "0x2", "void (void)")]),
                    call(vec![reference("held", "FunctionDecl", "0x3", "void (void)")]),
                    call(vec![reference("mixed", "FunctionDecl", "0x4", "void (void)")]),
                    call(vec![reference("mixed", "FunctionDecl", "0x5", "void (void) __attribute__((noreturn))")]),
                    call(vec![reference("object", "VarDecl", "0x6", "int")]),
                    call(vec![reference("shadow", "FunctionDecl", "0x7", "void (void)")]),
                    {"kind": "DeclStmt", "inner": [{"kind": "VarDecl", "name": "shadow"}]},
                    {"kind": "DeclStmt", "inner": [{"kind": "FunctionDecl", "id": "0x8", "name": "local",
                        "inner": [{"kind": "NoReturnAttr"}]}]},
                    call(vec![reference("local", "FunctionDecl", "0x8", "void (void)")]),
                    {"kind": "DeclStmt", "inner": [{"kind": "ParmVarDecl", "name": "unused"}]}
                ]}
            ]}),
            json!({"kind": "FunctionDecl", "name": "g", "inner": [{"kind": "CompoundStmt"}]}),
        ]
    }

    /// Rule 13s: the memo answers every name, asked twice, exactly as
    /// `resolve` does, and the fixture reaches every answer.
    #[test]
    fn the_memo_is_resolve_for_every_name() {
        let no_return = BTreeSet::from(["0x3".to_owned()]);
        let mut seen = BTreeSet::new();
        for body in bodies() {
            let reuse = FunctionReuse::default();
            for _ in 0..2 {
                for name in names(&body) {
                    let expected = crate::call_exits::resolve(&body, &name, &no_return);
                    assert_eq!(
                        reuse.resolution(&body, &name, &no_return),
                        expected,
                        "{name}"
                    );
                    seen.insert(format!("{expected:?}"));
                }
            }
        }
        assert_eq!(seen.len(), 3, "{seen:?}");
    }

    /// The memo stores the first computation per key and never computes a
    /// stored key again.
    #[test]
    fn a_memo_computes_each_key_once() {
        let memo = Memo::<u8, u32>::default();
        let calls = std::cell::Cell::new(0);
        let compute = |value| {
            calls.set(calls.get() + 1);
            value
        };
        assert_eq!(memo.get_or_compute(1, || compute(10)), 10);
        assert_eq!(memo.get_or_compute(1, || unreachable!()), 10);
        assert_eq!(memo.get_or_compute(2, || compute(20)), 20);
        assert_eq!(calls.get(), 2);
    }

    /// Rule 13t: the tree holds every node's lines in child order.
    #[test]
    fn the_line_tree_mirrors_the_function() {
        let body = bodies().remove(0);
        let begin = |node: &Value| node["name"].as_str().map(str::len);
        let end = |node: &Value| Some(children(node).len());
        let reuse = FunctionReuse::default();
        fn check(
            node: &Value,
            tree: &LineTree,
            begin: &dyn Fn(&Value) -> Option<usize>,
            end: &dyn Fn(&Value) -> Option<usize>,
        ) -> usize {
            assert_eq!(tree.begin, begin(node));
            assert_eq!(tree.end, end(node));
            let subtrees = tree.children(node, begin, end);
            assert_eq!(subtrees.len(), children(node).len());
            1 + children(node)
                .iter()
                .zip(subtrees)
                .map(|(child, sub)| check(child, sub, begin, end))
                .sum::<usize>()
        }
        // Built on the first walk, read on the second: the same lines.
        let first = check(&body, reuse.line_tree(&body, &begin, &end), &begin, &end);
        let unused = |_: &Value| -> Option<usize> { unreachable!("already read") };
        let tree = reuse.line_tree(&body, &unused, &unused);
        assert_eq!(tree.begin, begin(&body));
        let second = check(&body, tree, &begin, &end);
        assert_eq!(first, second);
        assert!(first > 20);
    }

    /// Rule 13t: a span is tokenized once; its answer, absent included, is
    /// the first computation's.
    #[test]
    fn a_span_is_tokenized_once() {
        let reuse = FunctionReuse::default();
        let calls = std::cell::Cell::new(0);
        let tokens = || {
            calls.set(calls.get() + 1);
            Some(vec!["a".to_owned()])
        };
        assert_eq!(
            reuse.span_tokens((0, 1), tokens).as_deref(),
            Some(&vec!["a".to_owned()])
        );
        assert_eq!(
            reuse.span_tokens((0, 1), || unreachable!()).as_deref(),
            Some(&vec!["a".to_owned()])
        );
        assert_eq!(reuse.span_tokens((1, 9), || None), None);
        assert_eq!(reuse.span_tokens((1, 9), || unreachable!()), None);
        assert_eq!(calls.get(), 1);
    }
}
