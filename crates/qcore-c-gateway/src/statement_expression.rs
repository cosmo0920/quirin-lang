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

//! Card **CG07-GENERIC-AND-STATEMENT-EXPRESSIONS** (gateway rule 11g), gap
//! entry E19: GNU statement expressions `({ S1; …; Sn; e; })`.
//!
//! # What GNU C says
//!
//! A statement expression is a compound statement in parentheses used as an
//! expression. Its statements run in order, each full expression a sequence
//! point, and the value of the last expression statement is the value of the
//! whole (void when the last item is not an expression). Objects declared in
//! the block live until the block ends, which is before the value is used by
//! the enclosing expression. A jump out of the block is allowed; a jump into
//! it is not.
//!
//! # What this module decides (route independent, over Clang's AST)
//!
//! * **Positions** ([`POSITIONS`]). A statement expression is read only where
//!   the enclosing full expression puts nothing unsequenced beside it:
//!   **discarded** (an expression statement's operand, through parentheses,
//!   `__extension__`, casts to `void` and commas), the whole **initializer** of
//!   a one-declarator declaration, the whole right operand of an
//!   **assignment** to a named object, and the whole operand of a **return**.
//!   Implicit conversions, parentheses and `__extension__` may wrap it. In
//!   each of these C evaluates the block completely, then uses its value (or
//!   drops it), and the only other event of the position (the store or the
//!   exit) is sequenced after it, so lowering the block's statements first and
//!   then the position with the value expression in place of the block is the
//!   order C prescribes. Every other position (an operand of an operator, a
//!   call actual, a controlling expression, a member base, a value under a
//!   cast) is refused by name.
//! * **Hygiene.** The route lowers names flatly (no block scopes), so an
//!   inner declaration may not share its name with a formal, with a
//!   declaration outside every other statement expression, with any
//!   declaration enclosing it, with another declaration of the same
//!   statement expression, or with a referenced object, function or constant
//!   declared outside the statement expressions of the body. Disjoint
//!   statement expressions may reuse a name (a macro expanded twice): their
//!   scopes never overlap, exactly like sibling blocks.
//! * **Escape.** Where the value is used, no inner object's address may be
//!   taken anywhere in the block (`&t`, an inner array's decay, a member or
//!   element of either), so the value can carry no pointer to storage whose
//!   lifetime ends at the block (`({ int t; &t; })` is refused). A
//!   `static`, `extern` or thread-local inner declaration is refused.
//! * **Value.** Where the value is used, the last item must be an expression
//!   statement: that expression is the value.
//!
//! `QCore.StatementExpressions` states the semantics (`value_is_final`,
//! `lifetimes_end_at_block`, `lowering_exact`, `flat_names_exact`,
//! `admitted_no_escape`, `unlisted_position_refused`) and is bound to
//! [`POSITIONS`] and [`admit`] by shared vectors.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Where a statement expression stands in its full expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    Discarded,
    Initializer,
    Assigned,
    Returned,
}

impl Position {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Position::Discarded => "discarded",
            Position::Initializer => "initializer",
            Position::Assigned => "assigned",
            Position::Returned => "returned",
        }
    }

    /// Whether the position uses the value.
    pub(crate) fn uses_value(self) -> bool {
        !matches!(self, Position::Discarded)
    }
}

/// Every position the reader names, and whether it is admitted
/// (`QCore.StatementExpressions.positionTable`).
pub(crate) const POSITIONS: [(&str, bool); 9] = [
    ("discarded", true),
    ("initializer", true),
    ("assigned", true),
    ("returned", true),
    ("operand", false),
    ("call actual", false),
    ("condition", false),
    ("assigned path", false),
    ("cast value", false),
];

/// Whether a named position is admitted; an unlisted one is refused.
pub(crate) fn position_admitted(position: &str) -> bool {
    POSITIONS
        .iter()
        .any(|(name, admitted)| *name == position && *admitted)
}

/// The admission of one statement expression from its premises
/// (`QCore.StatementExpressions.admit`): the position is admitted, the names
/// are hygienic, no inner declaration has static storage, and, where the
/// value is used, no inner address is taken and the last item is a value.
pub(crate) fn admit(
    position_ok: bool,
    hygienic: bool,
    automatic: bool,
    uses_value: bool,
    address_taken: bool,
    last_is_value: bool,
) -> bool {
    position_ok && hygienic && automatic && (!uses_value || (!address_taken && last_is_value))
}

/// `ADMISSION_VECTORS` = `QCore.StatementExpressions.admissionVectors`:
/// `(position_ok, hygienic, automatic, uses_value, address_taken,
/// last_is_value, admitted)`.
#[cfg(test)]
pub(crate) const ADMISSION_VECTORS: [(bool, bool, bool, bool, bool, bool, bool); 10] = [
    (true, true, true, false, false, false, true),
    (true, true, true, false, true, false, true),
    (true, true, true, true, false, true, true),
    (true, true, true, true, true, true, false),
    (true, true, true, true, false, false, false),
    (false, true, true, false, false, true, false),
    (true, false, true, false, false, true, false),
    (true, true, false, false, false, true, false),
    (true, false, true, true, false, true, false),
    (false, true, true, true, false, true, false),
];

/// Why a statement expression is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A position the table does not admit.
    Position(&'static str),
    /// An inner name the flat lowering could confuse with another.
    Shadow(String),
    /// A `static`, `extern` or thread-local inner declaration.
    Storage(String),
    /// An inner object's address is taken where the value is used.
    Escape(String),
    /// The value is used but the last item is not an expression.
    NoValue,
    /// The node is not what the route matched.
    Shape(&'static str),
}

impl Refusal {
    pub(crate) fn reason(&self) -> String {
        match self {
            Refusal::Position(position) => {
                format!("statement expression in the {position} position (CG07)")
            }
            Refusal::Shadow(name) => {
                format!("statement expression declares `{name}`, a name the body also uses (CG07)")
            }
            Refusal::Storage(name) => {
                format!("statement expression declares `{name}` with static storage (CG07)")
            }
            Refusal::Escape(name) => {
                format!("statement expression takes the address of its own object `{name}` (CG07)")
            }
            Refusal::NoValue => {
                "statement expression whose value is not an expression (CG07)".into()
            }
            Refusal::Shape(what) => format!("statement expression: {what}"),
        }
    }
}

/// `node` with parentheses, `__extension__` and implicit conversions removed.
pub(crate) fn strip_transparent(node: &Value) -> &Value {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "FullExpr" | "ConstantExpr" => {}
            "UnaryOperator" if node["opcode"].as_str() == Some("__extension__") => {}
            _ => return node,
        }
        match children(node) {
            [inner] => node = inner,
            _ => return node,
        }
    }
}

/// Whether `node`'s subtree holds a statement expression.
pub(crate) fn holds(node: &Value) -> bool {
    kind(node) == "StmtExpr" || children(node).iter().any(holds)
}

/// The statement expression that is the whole of a value position's operand,
/// through the transparent wrappers.
pub(crate) fn whole_value(node: &Value) -> Option<&Value> {
    let bare = strip_transparent(node);
    (kind(bare) == "StmtExpr").then_some(bare)
}

/// One operand of a discarded full expression, in C's order.
pub(crate) enum Operand<'a> {
    /// A statement expression whose value is dropped.
    Block(&'a Value),
    /// An expression holding no statement expression.
    Plain(&'a Value),
}

/// The operands of a discarded full expression: through parentheses,
/// `__extension__`, implicit conversions, casts to `void` and commas (the
/// comma sequences its left operand before its right). An operand holding a
/// statement expression anywhere else refuses.
pub(crate) fn discarded_operands(node: &Value) -> Result<Vec<Operand<'_>>, Refusal> {
    let bare = strip_transparent(node);
    match kind(bare) {
        "StmtExpr" => Ok(vec![Operand::Block(bare)]),
        "BinaryOperator" if bare["opcode"].as_str() == Some(",") => {
            let [left, right] = children(bare) else {
                return Err(Refusal::Shape("comma arity"));
            };
            let mut operands = discarded_operands(left)?;
            operands.extend(discarded_operands(right)?);
            Ok(operands)
        }
        "CStyleCastExpr" if bare["castKind"].as_str() == Some("ToVoid") => match children(bare) {
            [inner] => discarded_operands(inner),
            _ => Err(Refusal::Shape("cast arity")),
        },
        _ if holds(bare) => Err(Refusal::Position(position_of(bare))),
        _ => Ok(vec![Operand::Plain(bare)]),
    }
}

/// The name of the refused position a statement expression under `node`
/// stands in.
pub(crate) fn position_of(node: &Value) -> &'static str {
    let bare = strip_transparent(node);
    match kind(bare) {
        "CallExpr" => "call actual",
        "CStyleCastExpr" => "cast value",
        "ConditionalOperator" | "BinaryConditionalOperator" => "condition",
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => "assigned path",
        _ => "operand",
    }
}

/// The named object an assignment's left operand designates: `NAME`, or
/// `*&NAME` (librdkafka's `RD_KAFKAP_STR_DUPA (&dst, src)` spells
/// `*((&dst)) = …`), through parentheses. Designating a named object
/// evaluates nothing, so nothing of the left operand is unsequenced with the
/// right one.
pub(crate) fn named_destination(left: &Value) -> Option<&str> {
    let bare = crate::elaboration::strip_parens(left);
    let object = match kind(bare) {
        "DeclRefExpr" => bare,
        "UnaryOperator" if bare["opcode"].as_str() == Some("*") => {
            let address = crate::elaboration::strip_parens(children(bare).first()?);
            if kind(address) != "UnaryOperator" || address["opcode"].as_str() != Some("&") {
                return None;
            }
            let object = crate::elaboration::strip_parens(children(address).first()?);
            (kind(object) == "DeclRefExpr").then_some(object)?
        }
        _ => return None,
    };
    matches!(
        object["referencedDecl"]["kind"].as_str(),
        Some("VarDecl" | "ParmVarDecl")
    )
    .then(|| object["referencedDecl"]["name"].as_str())
    .flatten()
}

/// The items of a statement expression's block.
pub(crate) fn items(statement_expression: &Value) -> Result<&[Value], Refusal> {
    match children(statement_expression) {
        [compound] if kind(compound) == "CompoundStmt" => Ok(children(compound)),
        _ => Err(Refusal::Shape("no compound statement")),
    }
}

/// Whether an item is an expression statement (the value of a last item).
pub(crate) fn is_expression(item: &Value) -> bool {
    let kind = kind(item);
    !kind.is_empty() && !kind.ends_with("Stmt") && !kind.ends_with("Decl")
}

/// Every `VarDecl` of a subtree: `(id, name, storage class)`.
fn declarations(node: &Value, out: &mut Vec<(String, String, Option<String>)>) {
    if kind(node) == "VarDecl"
        && let (Some(id), Some(name)) = (node["id"].as_str(), node["name"].as_str())
    {
        out.push((
            id.to_owned(),
            name.to_owned(),
            node["storageClass"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| node["tls"].as_str().map(str::to_owned)),
        ));
    }
    for child in children(node) {
        declarations(child, out);
    }
}

/// The function's name facts, read once per body.
pub(crate) struct Body {
    /// Every `VarDecl` id with its name and the ids of the statement
    /// expressions enclosing it, innermost last.
    declared: Vec<(String, String, Vec<usize>)>,
    /// `ParmVarDecl` names.
    formals: BTreeSet<String>,
    /// Every referenced declaration: name to the ids it resolves to.
    references: BTreeMap<String, BTreeSet<String>>,
    /// Each statement expression (by node address) with the statement
    /// expressions enclosing it, outermost first.
    ancestors: BTreeMap<usize, Vec<usize>>,
}

fn address(node: &Value) -> usize {
    std::ptr::from_ref(node) as usize
}

impl Body {
    pub(crate) fn read(function: &Value) -> Self {
        fn walk(node: &Value, stack: &mut Vec<usize>, body: &mut Body) {
            let entered = kind(node) == "StmtExpr";
            if entered {
                body.ancestors.insert(address(node), stack.clone());
                stack.push(address(node));
            }
            match kind(node) {
                "VarDecl" => {
                    if let (Some(id), Some(name)) = (node["id"].as_str(), node["name"].as_str()) {
                        body.declared
                            .push((id.to_owned(), name.to_owned(), stack.clone()));
                    }
                }
                "ParmVarDecl" => {
                    if let Some(name) = node["name"].as_str() {
                        body.formals.insert(name.to_owned());
                    }
                }
                "DeclRefExpr" => {
                    let reference = &node["referencedDecl"];
                    if let (Some(id), Some(name)) =
                        (reference["id"].as_str(), reference["name"].as_str())
                    {
                        body.references
                            .entry(name.to_owned())
                            .or_default()
                            .insert(id.to_owned());
                    }
                }
                _ => {}
            }
            for child in children(node) {
                walk(child, stack, body);
            }
            if entered {
                stack.pop();
            }
        }
        let mut body = Body {
            declared: Vec::new(),
            formals: BTreeSet::new(),
            references: BTreeMap::new(),
            ancestors: BTreeMap::new(),
        };
        walk(function, &mut Vec::new(), &mut body);
        body
    }

    /// The hygiene premise for the statement expression `node`: every inner
    /// name is fresh for the flat lowering.
    pub(crate) fn hygiene(&self, node: &Value) -> Result<(), Refusal> {
        let this = address(node);
        let inner = self
            .declared
            .iter()
            .filter(|(_, _, stack)| stack.contains(&this))
            .collect::<Vec<_>>();
        // Declarations inside any statement expression.
        let in_blocks = self
            .declared
            .iter()
            .filter(|(_, _, stack)| !stack.is_empty())
            .map(|(id, _, _)| id.as_str())
            .collect::<BTreeSet<_>>();
        for (id, name, _) in &inner {
            let shadow = || Err(Refusal::Shadow(name.clone()));
            if self.formals.contains(name) {
                return shadow();
            }
            for (other_id, other_name, stack) in &self.declared {
                if other_name != name || other_id == id {
                    continue;
                }
                // Another declaration of the name is admitted only inside a
                // statement expression disjoint from this one: its innermost
                // statement expression neither holds this one nor lies in it,
                // so the two scopes never overlap.
                let disjoint = stack.last().is_some_and(|innermost| {
                    !stack.contains(&this)
                        && *innermost != this
                        && !self
                            .ancestors
                            .get(&this)
                            .is_some_and(|chain| chain.contains(innermost))
                });
                if !disjoint {
                    return shadow();
                }
            }
            // Every reference to the name resolves to a declaration inside a
            // statement expression (never a global, function or constant).
            if self
                .references
                .get(name)
                .is_some_and(|ids| ids.iter().any(|id| !in_blocks.contains(id.as_str())))
            {
                return shadow();
            }
        }
        Ok(())
    }

    /// Whether the body holds `node` as one of its statement expressions.
    pub(crate) fn knows(&self, node: &Value) -> bool {
        self.ancestors.contains_key(&address(node))
    }
}

/// The storage premise: no inner declaration is `static`, `extern` or
/// thread-local.
pub(crate) fn automatic(node: &Value) -> Result<(), Refusal> {
    let mut inner = Vec::new();
    declarations(node, &mut inner);
    match inner.into_iter().find(|(_, _, storage)| storage.is_some()) {
        Some((_, name, _)) => Err(Refusal::Storage(name)),
        None => Ok(()),
    }
}

/// The first inner object whose address the block takes: `&t`, an inner
/// array's decay, or a member or element of either.
pub(crate) fn address_taken(node: &Value) -> Option<String> {
    let mut inner = Vec::new();
    declarations(node, &mut inner);
    let ids = inner
        .into_iter()
        .map(|(id, name, _)| (id, name))
        .collect::<BTreeMap<_, _>>();
    fn root(node: &Value) -> Option<&Value> {
        let bare = strip_transparent(node);
        match kind(bare) {
            "DeclRefExpr" => Some(bare),
            "MemberExpr" if bare["isArrow"].as_bool() != Some(true) => {
                children(bare).first().and_then(root)
            }
            "ArraySubscriptExpr" => children(bare).first().and_then(|base| {
                let base = strip_transparent(base);
                // `a[i]` of an array object designates part of it; of a
                // pointer, the pointed-to object.
                (kind(base) == "DeclRefExpr"
                    && base["type"]["qualType"]
                        .as_str()
                        .is_some_and(|ty| ty.contains('[')))
                .then_some(base)
            }),
            _ => None,
        }
    }
    fn walk(node: &Value, ids: &BTreeMap<String, String>) -> Option<String> {
        let taken = match kind(node) {
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                children(node).first().and_then(root)
            }
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("ArrayToPointerDecay") => {
                children(node).first().and_then(root)
            }
            _ => None,
        };
        if let Some(reference) = taken
            && let Some(name) = reference["referencedDecl"]["id"]
                .as_str()
                .and_then(|id| ids.get(id))
        {
            return Some(name.clone());
        }
        children(node).iter().find_map(|child| walk(child, ids))
    }
    walk(node, &ids)
}

/// Decide one statement expression in an admitted position.
pub(crate) fn decide(body: &Body, node: &Value, position: Position) -> Result<(), Refusal> {
    if !body.knows(node) {
        return Err(Refusal::Shape("not a statement expression of this body"));
    }
    let hygiene = body.hygiene(node);
    let storage = automatic(node);
    let taken = address_taken(node);
    let last_is_value = items(node)?.last().is_some_and(is_expression);
    if admit(
        position_admitted(position.label()),
        hygiene.is_ok(),
        storage.is_ok(),
        position.uses_value(),
        taken.is_some(),
        last_is_value,
    ) {
        return Ok(());
    }
    // The first premise that failed, by name.
    hygiene?;
    storage?;
    if let Some(name) = taken {
        return Err(Refusal::Escape(name));
    }
    if !last_is_value {
        return Err(Refusal::NoValue);
    }
    Err(Refusal::Position(position.label()))
}

#[cfg(test)]
#[path = "../tests/unit/statement_expression.rs"]
mod tests;
