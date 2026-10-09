//! Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME** (gateway rules 13s, 13t): one
//! function's typed readings computed once per index and answered again from
//! it, so a statement hook no longer re-walks the whole body.
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
//! of them and tokenized the same span again for each node and each statement.
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
//! * **Rule 13s.** [`call_resolutions`]: every callee name's
//!   [`Resolution`], from one walk that gathers what `call_exits::resolve`
//!   gathers per callee (the body's object names, its `noreturn` function
//!   declarations, each name's references in walk order). A name with no
//!   entry is `Unresolved`, as `resolve` answers for a name with no
//!   reference.
//! * **Rule 13t.** [`LineTree`]: each node's begin and end line, in the
//!   function's own child order, so a walk that reads lines from it visits
//!   exactly the nodes a walk computing them visits; and each source span's
//!   tokens, tokenized once ([`FunctionReuse::span_tokens`]).
//!
//! Nothing here is consulted for a decision the uncached readers would not
//! make: `QCore.DeepAstTransports` states the table's and the tree's
//! equivalence, and the unit tests compare every name and every node.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use serde_json::Value;

use crate::call_exits::Resolution;
use crate::typed_c_ast::{children, kind};

/// A source span's tokens, or the absent answer, shared.
pub(crate) type SpanTokens = Option<Arc<Vec<String>>>;

/// One function's reusable readings (see the module documentation).
#[derive(Default)]
pub(crate) struct FunctionReuse {
    calls: OnceLock<BTreeMap<String, Resolution>>,
    lines: OnceLock<LineTree>,
    spans: Mutex<HashMap<(usize, usize), SpanTokens>>,
}

/// A node's begin and end line (`None` where the node carries no offset or
/// one beyond the source), and its children's, in `inner` order.
pub(crate) struct LineTree {
    pub(crate) begin: Option<usize>,
    pub(crate) end: Option<usize>,
    pub(crate) children: Vec<LineTree>,
}

impl LineTree {
    fn build(
        node: &Value,
        begin: &dyn Fn(&Value) -> Option<usize>,
        end: &dyn Fn(&Value) -> Option<usize>,
    ) -> Self {
        Self {
            begin: begin(node),
            end: end(node),
            children: children(node)
                .iter()
                .map(|child| Self::build(child, begin, end))
                .collect(),
        }
    }
}

impl FunctionReuse {
    /// Rule 13s: `call_exits::resolve(function, callee, no_return)`, from the
    /// function's table.
    pub(crate) fn resolution(
        &self,
        function: &Value,
        callee: &str,
        no_return_declarations: &BTreeSet<String>,
    ) -> Resolution {
        self.calls
            .get_or_init(|| call_resolutions(function, no_return_declarations))
            .get(callee)
            .copied()
            .unwrap_or(Resolution::Unresolved)
    }

    /// Rule 13t: the function's line tree, built on first use with the
    /// index's own line readers.
    pub(crate) fn line_tree(
        &self,
        function: &Value,
        begin: &dyn Fn(&Value) -> Option<usize>,
        end: &dyn Fn(&Value) -> Option<usize>,
    ) -> &LineTree {
        self.lines
            .get_or_init(|| LineTree::build(function, begin, end))
    }

    /// Rule 13t: the tokens of the source span `start..end`, computed by
    /// `tokenize` once per span (the absent answer included).
    pub(crate) fn span_tokens(
        &self,
        span: (usize, usize),
        tokenize: impl FnOnce() -> Option<Vec<String>>,
    ) -> SpanTokens {
        if let Some(found) = self
            .spans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&span)
        {
            return found.clone();
        }
        // Tokenized outside the lock; a concurrent reader computes the same
        // pure answer, and the first one stored stands.
        let found = tokenize().map(Arc::new);
        self.spans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(span)
            .or_insert(found)
            .clone()
    }
}

/// What one walk of a body says of one name, in `call_exits::resolve`'s
/// terms.
#[derive(Default)]
struct NameFacts {
    /// A `VarDecl` or `ParmVarDecl` declares the name.
    shadowed: bool,
    /// Every `DeclRefExpr` referring to the name, in walk order.
    references: Vec<Value>,
}

/// Rule 13s: [`Resolution`] of every name the body refers to or declares as
/// an object, exactly as `call_exits::resolve` answers it per name.
pub(crate) fn call_resolutions(
    function: &Value,
    no_return_declarations: &BTreeSet<String>,
) -> BTreeMap<String, Resolution> {
    let mut names: BTreeMap<String, NameFacts> = BTreeMap::new();
    let mut local = BTreeSet::new();
    walk(function, &mut |node| match kind(node) {
        "VarDecl" | "ParmVarDecl" => {
            if let Some(name) = node["name"].as_str() {
                names.entry(name.to_owned()).or_default().shadowed = true;
            }
        }
        "FunctionDecl" if crate::call_exits::declares_no_return(node) => {
            if let Some(id) = node["id"].as_str() {
                local.insert(id.to_owned());
            }
        }
        "DeclRefExpr" => {
            if let Some(name) = node["referencedDecl"]["name"].as_str() {
                names
                    .entry(name.to_owned())
                    .or_default()
                    .references
                    .push(node["referencedDecl"].clone());
            }
        }
        _ => {}
    });
    names
        .into_iter()
        .map(|(name, facts)| {
            let resolution = if facts.shadowed || facts.references.is_empty() {
                Resolution::Unresolved
            } else {
                crate::call_exits::resolution_of(&facts.references, no_return_declarations, &local)
            };
            (name, resolution)
        })
        .collect()
}

fn walk<'a>(node: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(node);
    for child in children(node) {
        walk(child, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    /// Rule 13s: the table answers every name exactly as `resolve` does.
    #[test]
    fn the_table_is_resolve_for_every_name() {
        let no_return = BTreeSet::from(["0x3".to_owned()]);
        for body in bodies() {
            let table = call_resolutions(&body, &no_return);
            let reuse = FunctionReuse::default();
            for name in names(&body) {
                let expected = crate::call_exits::resolve(&body, &name, &no_return);
                assert_eq!(
                    table.get(&name).copied().unwrap_or(Resolution::Unresolved),
                    expected,
                    "{name}"
                );
                assert_eq!(
                    reuse.resolution(&body, &name, &no_return),
                    expected,
                    "{name}"
                );
            }
        }
        // The fixture reaches every answer.
        let table = call_resolutions(&bodies()[0], &no_return);
        assert_eq!(table["die"], Resolution::NoReturn);
        assert_eq!(table["held"], Resolution::NoReturn);
        assert_eq!(table["local"], Resolution::NoReturn);
        assert_eq!(table["go"], Resolution::Returns);
        assert_eq!(table["mixed"], Resolution::Unresolved);
        assert_eq!(table["object"], Resolution::Unresolved);
        assert_eq!(table["shadow"], Resolution::Unresolved);
    }

    /// Rule 13t: the tree holds every node's lines in child order.
    #[test]
    fn the_line_tree_mirrors_the_function() {
        let body = bodies().remove(0);
        let begin = |node: &Value| node["name"].as_str().map(str::len);
        let end = |node: &Value| Some(children(node).len());
        let tree = LineTree::build(&body, &begin, &end);
        fn check(
            node: &Value,
            tree: &LineTree,
            begin: &dyn Fn(&Value) -> Option<usize>,
            end: &dyn Fn(&Value) -> Option<usize>,
        ) -> usize {
            assert_eq!(tree.begin, begin(node));
            assert_eq!(tree.end, end(node));
            assert_eq!(tree.children.len(), children(node).len());
            1 + children(node)
                .iter()
                .zip(&tree.children)
                .map(|(child, sub)| check(child, sub, begin, end))
                .sum::<usize>()
        }
        assert!(check(&body, &tree, &begin, &end) > 20);
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
