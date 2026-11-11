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

//! Rule 36's source readings: the lexical candidate, the declaration's
//! matching pointer formals, and the typed source conjunction.
//!
//! The host supplies its own vocabulary: the lexical reader's formals and
//! body tokens, its declaration reader's types, and the reads of its typed
//! (Clang) AST ([`SourceHost`], [`FormalReturnHost`]). The decisions are
//! here. A missing fact declines: every reading below answers `None` (or
//! `false`, or the empty set), never a guess.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_gateway_rule_api::Host;

/// The typed-node reads rule 36's source conjunction needs. Each read is
/// total: an absent kind is empty, an absent attribute, field or type is
/// `None`, an absent child list is empty, so a malformed node is declined,
/// never trusted.
///
/// Rule 35's package (card RULE35-PILOT, unmerged at this card's base) adds
/// the same reads minus [`node_field`](Self::node_field) to the rule API as
/// `TypedHost`; the two are one capability once both land (follow-up
/// RULE36-PILOT-TYPED-HOST-UNIFY).
pub trait SourceHost: Host {
    /// The node's kind (`ParmVarDecl`, `DeclRefExpr`, ...).
    fn node_kind(node: &Self::Node) -> &str;
    /// A string attribute of the node (`name`, `id`, `opcode`, `castKind`).
    fn node_attribute<'n>(node: &'n Self::Node, key: &str) -> Option<&'n str>;
    /// An object-valued field of the node (`referencedDecl`).
    fn node_field<'n>(node: &'n Self::Node, key: &str) -> Option<&'n Self::Node>;
    /// The node's type spelling: the desugared spelling when the AST gives
    /// one, else the written one.
    fn node_type(node: &Self::Node) -> Option<&str>;
    /// The node's children, in order.
    fn node_children(node: &Self::Node) -> &[Self::Node];
    /// The host's positive object-pointer type test on a spelling (no union,
    /// volatile, `_Atomic` or function pointer).
    fn pointer_type(text: &str) -> bool;
}

/// The host's readings of the function being lowered that the source
/// conjunction asks per formal and per `return` node.
pub trait FormalReturnHost: SourceHost {
    /// The typed root reader answered for formal `name` and reads it as never
    /// bound in the body (rules 10l/10o), asked in that order.
    fn formal_unbound(&self, name: &str) -> bool;
    /// The source tokens a node's complete span covers, or `None` when the
    /// span is incomplete or not read once.
    fn node_tokens(&self, node: &Self::Node) -> Option<Arc<Vec<String>>>;
    /// The source line a node begins on.
    fn node_line(&self, node: &Self::Node) -> Option<usize>;
}

/// The function a source conjunction reads.
pub struct FunctionReading<'f, H: Host + ?Sized> {
    /// The lexical reader's formal names, by position (`None`: unnamed).
    pub formals: &'f [Option<String>],
    /// The positions [`pointer_formals`] admits; never empty.
    pub pointer_formals: &'f BTreeSet<usize>,
    /// The typed function definition.
    pub function: &'f H::Node,
    /// The body's tokens between its braces, or `None` when the lexical
    /// reader cannot delimit the body.
    pub body: Option<&'f [H::Token]>,
}

/// The formal positions whose declared type positively and unambiguously
/// matches the result's, when the result is a depth-one object pointer
/// (`ptr<mut, u8>` or `ptr<const, u8>`). An unreadable result, an
/// unspecified parameter list or any other result type admits none.
#[must_use]
pub fn pointer_formals<'a>(
    unspecified: bool,
    result: Option<&BTreeSet<&'a str>>,
    parameters: &[Option<BTreeSet<&'a str>>],
) -> BTreeSet<usize> {
    let Some(result) = result else {
        return BTreeSet::new();
    };
    if unspecified
        || result.len() != 1
        || !result
            .iter()
            .all(|ty| matches!(*ty, "ptr<mut, u8>" | "ptr<const, u8>"))
    {
        return BTreeSet::new();
    }
    parameters
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| (ty.as_ref() == Some(result)).then_some(index))
        .collect()
}

/// The lexical pre-filter: the body returns some formal bare (`return P;`)
/// and one such formal is a [`pointer_formals`] position. `pointer_formals`
/// is read only when a bare formal return exists.
pub fn candidate<H: Host + ?Sized>(
    formals: &[Option<String>],
    body: &[H::Token],
    pointer_formals: impl FnOnce() -> BTreeSet<usize>,
) -> bool {
    let returned = body
        .windows(3)
        .filter_map(|run| {
            (H::token_text(&run[0]) == "return" && H::token_text(&run[2]) == ";")
                .then(|| {
                    formals
                        .iter()
                        .position(|name| name.as_deref() == Some(H::token_text(&run[1])))
                })
                .flatten()
        })
        .collect::<BTreeSet<_>>();
    if returned.is_empty() {
        return false;
    }
    !returned.is_disjoint(&pointer_formals())
}

/// The source conjunction: each `(line, name)` of a bare `return NAME;` the
/// typed reader confirms as the original, unwritten, unrebound pointer formal
/// at that position, mapped to the position. `None` when any fact is missing.
pub fn exact_formal_returns<H: FormalReturnHost + ?Sized>(
    host: &H,
    reading: &FunctionReading<'_, H>,
) -> Option<BTreeMap<(usize, String), usize>> {
    let ast_parameters = H::node_children(reading.function)
        .iter()
        .filter(|node| H::node_kind(node) == "ParmVarDecl")
        .collect::<Vec<_>>();
    if ast_parameters.len() != reading.formals.len() {
        return None;
    }
    let body = H::node_children(reading.function)
        .iter()
        .find(|node| H::node_kind(node) == "CompoundStmt")?;
    let mut names = BTreeMap::new();
    for node in &ast_parameters {
        value_name::<H>(node, &mut names)?;
    }
    let mut eligible = BTreeMap::new();
    for &parameter in reading.pointer_formals {
        let name = reading.formals.get(parameter)?.as_ref()?;
        let node = *ast_parameters.get(parameter)?;
        if H::node_attribute(node, "name") == Some(name.as_str())
            && object_pointer::<H>(node)
            && host.formal_unbound(name)
        {
            eligible.insert(
                name.clone(),
                (parameter, H::node_attribute(node, "id")?.to_owned()),
            );
        }
    }
    if eligible.is_empty() {
        return None;
    }
    let mut writes = BTreeSet::new();
    inspect_body::<H>(body, &mut names, &mut writes)?;
    eligible.retain(|_, (_, id)| !writes.contains(id));
    let tokens = reading.body?;
    let mut lexical = BTreeMap::<(usize, String), usize>::new();
    for run in tokens.windows(3) {
        if H::token_text(&run[0]) == "return"
            && H::token_text(&run[2]) == ";"
            && eligible.contains_key(H::token_text(&run[1]))
        {
            *lexical
                .entry((H::token_line(&run[0]), H::token_text(&run[1]).to_owned()))
                .or_default() += 1;
        }
    }
    let mut ast = BTreeMap::new();
    inspect_returns(host, body, &eligible, &mut ast)?;
    let mut answer = BTreeMap::new();
    for (key, count) in lexical {
        if ast.get(&key) == Some(&(count, true)) {
            answer.insert(key.clone(), eligible.get(&key.1)?.0);
        }
    }
    Some(answer)
}

/// A declaration, or a formal, of a depth-one object pointer type. Only
/// top-level `const` is removed; volatile, `_Atomic` and function-pointer
/// types still fail the host's positive pointer test, and pointee `const` is
/// kept.
pub fn object_pointer<H: SourceHost + ?Sized>(node: &H::Node) -> bool {
    H::node_type(node).is_some_and(|text| {
        let text = text.trim_end();
        let text = text.strip_suffix("const").map_or(text, str::trim_end);
        H::pointer_type(text)
    })
}

/// Records a value declaration's name and identity; one name bound to two
/// identities (a shadow, an enumerator, a function) or a missing name or
/// identity declines.
pub fn value_name<H: SourceHost + ?Sized>(
    node: &H::Node,
    names: &mut BTreeMap<String, String>,
) -> Option<()> {
    let name = H::node_attribute(node, "name")?;
    let id = H::node_attribute(node, "id")?;
    if name.is_empty() || id.is_empty() {
        return None;
    }
    if names.get(name).is_some_and(|old| old != id) {
        return None;
    }
    names.insert(name.to_owned(), id.to_owned());
    Some(())
}

/// Every declaration a written operand references.
pub fn references<H: SourceHost + ?Sized>(
    node: &H::Node,
    written: &mut BTreeSet<String>,
) -> Option<()> {
    if H::node_kind(node) == "DeclRefExpr" {
        written.insert(H::node_attribute(H::node_field(node, "referencedDecl")?, "id")?.to_owned());
    }
    for child in H::node_children(node) {
        references::<H>(child, written)?;
    }
    Some(())
}

/// The body's value bindings (through [`value_name`]) and the declarations
/// it assigns, compound-assigns, increments, decrements or takes the address
/// of.
pub fn inspect_body<H: SourceHost + ?Sized>(
    node: &H::Node,
    names: &mut BTreeMap<String, String>,
    written: &mut BTreeSet<String>,
) -> Option<()> {
    match H::node_kind(node) {
        "VarDecl" | "EnumConstantDecl" | "FunctionDecl" => value_name::<H>(node, names)?,
        "DeclRefExpr" => {
            value_name::<H>(H::node_field(node, "referencedDecl")?, names)?;
        }
        "BinaryOperator" => {
            if H::node_attribute(node, "opcode")? == "=" {
                references::<H>(H::node_children(node).first()?, written)?;
            }
        }
        "CompoundAssignOperator" => references::<H>(H::node_children(node).first()?, written)?,
        "UnaryOperator" => {
            if matches!(H::node_attribute(node, "opcode")?, "&" | "++" | "--") {
                references::<H>(H::node_children(node).first()?, written)?;
            }
        }
        _ => {}
    }
    for child in H::node_children(node) {
        inspect_body::<H>(child, names, written)?;
    }
    Some(())
}

/// Whether `node` is the original reference to formal `name` with identity
/// `id`, through value-preserving implicit casts of object-pointer type only.
pub fn original_formal<H: SourceHost + ?Sized>(mut node: &H::Node, name: &str, id: &str) -> bool {
    loop {
        if !object_pointer::<H>(node) {
            return false;
        }
        if H::node_kind(node) != "ImplicitCastExpr" {
            break;
        }
        if !matches!(
            H::node_attribute(node, "castKind"),
            Some("LValueToRValue" | "NoOp" | "BitCast")
        ) {
            return false;
        }
        let [child] = H::node_children(node) else {
            return false;
        };
        node = child;
    }
    let declaration = H::node_field(node, "referencedDecl");
    let read = |key| declaration.and_then(|declaration| H::node_attribute(declaration, key));
    H::node_kind(node) == "DeclRefExpr"
        && read("kind") == Some("ParmVarDecl")
        && read("name") == Some(name)
        && read("id") == Some(id)
}

/// Counts, per `(line, name)`, the typed `return NAME` statements of an
/// eligible formal, and whether every one returns the original formal.
pub fn inspect_returns<H: FormalReturnHost + ?Sized>(
    host: &H,
    node: &H::Node,
    eligible: &BTreeMap<String, (usize, String)>,
    counts: &mut BTreeMap<(usize, String), (usize, bool)>,
) -> Option<()> {
    if H::node_kind(node) == "ReturnStmt" {
        let tokens = host.node_tokens(node)?;
        let line = host.node_line(node)?;
        if let [keyword, name] = tokens.as_slice()
            && keyword == "return"
            && let Some((_, id)) = eligible.get(name)
        {
            let valid = match H::node_children(node) {
                [value] => original_formal::<H>(value, name, id),
                _ => false,
            };
            let count = counts.entry((line, name.clone())).or_insert((0, true));
            count.0 += 1;
            count.1 &= valid;
        }
    }
    for child in H::node_children(node) {
        inspect_returns(host, child, eligible, counts)?;
    }
    Some(())
}

#[cfg(test)]
#[path = "../tests/unit/source.rs"]
mod tests;
