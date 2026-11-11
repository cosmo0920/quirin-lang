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

//! Card **CG06-VALUE-CONNECTIVES-AND-CONDITIONAL** (gateway rule 10z): value
//! `&&`, `||`, `C ? A : B` and GNU `X ?: B` outside condition position, read
//! from Clang's typed AST.
//!
//! A full expression `F` (an expression statement, a one-declarator
//! declaration, or a `return` value) may hold **value regions**: a connective
//! or conditional whose value `F` consumes. [`regions`] finds each region `R`
//! that C evaluates as part of `F` (not inside `sizeof`), with the token spans
//! of its deciding operand and its arms, and the regions nested inside each
//! arm. A route then lowers `F[R]` to what C evaluates:
//!
//! ```text
//! decide(cond(R)); on non-zero: F[yes(R)]   on zero: F[no(R)]
//! ```
//!
//! * `A && B`, `A || B` ([`Shape::Boolean`]): the whole region is the deciding
//!   condition (the condition CFG evaluates `B` only on the edge C does), and
//!   the value is the `int` `1` or `0`.
//! * `C ? A : B` ([`Shape::Select`]): `C` decides; only the selected arm is
//!   evaluated, in `F`'s place of `R`.
//! * `X ?: B` ([`Shape::OmittedMiddle`]): `X` decides and is the value on the
//!   non-zero edge. The lowering reads `X` a second time on that edge, so it
//!   is admitted only when `X`'s evaluation is **reads only**: a second read
//!   of an unchanged place observes the same value, and a read changes no
//!   state (`QCore.ValueConnectives.elvis_reread`). An `X` with a
//!   call, a store, an increment or a volatile access refuses; C evaluates
//!   it once.
//!
//! The two arms' results are never merged here: each edge carries its own
//! alias and custody state to the join of the CFG the route builds, where the
//! flow's existing join merges them (two identities become a binding with two
//! targets, which no release through it resolves to one).
//!
//! # The order premise
//!
//! The lowering evaluates `cond(R)` before everything else of `F`. C sequences
//! `cond(R)` before the arms, and before every consumer of `R`'s value on the
//! path from `F`'s root to `R` (an operator, a call, the store), but leaves it
//! unsequenced with every **sibling**: a subexpression hanging off that path.
//! [`admit`] therefore requires (1) every sibling is free of effects (no call,
//! store, increment, statement expression or volatile access), so no sibling
//! event conflicts with `cond(R)`'s reads; and (2) when `cond(R)` itself has
//! effects (a call), every sibling is **inert**: a literal, a designation, or
//! a read of an automatic object whose address is never taken, which no call
//! can reach. The arms are evaluated inside `F[arm]`, whose own order the
//! older route that lowers it decides. No left-to-right order C does not
//! guarantee is chosen.
//!
//! Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m) asks those two refusals
//! of [`Reading::compose`] instead ([`admit_composed`]): the region and its
//! siblings composed by CG05's contract footprints
//! ([`crate::connective_footprints`]), admitted only when every unordered pair
//! commutes on both arms; every other refusal of [`admit`] stands.
//!
//! # Positions
//!
//! `R` is found through the positions [`TRANSPARENT`] lists (parentheses,
//! implicit and scalar casts, unary and binary operators, a call's actuals, a
//! member or subscript base, the root's `=` / `op=` operands, a declaration's
//! initializer). A connective under any other node an evaluation reaches (a
//! comma, a nested assignment, a statement expression, a compound literal, an
//! initializer list, `_Generic`, an indirect callee) refuses by name; one under
//! `sizeof` is unevaluated and is not a region. A connective in a region's
//! deciding operand is the condition CFG's, not a region of its own.
//!
//! The Lean model is `QCore.ValueConnectives`, bound to [`ADMISSION_VECTORS`]
//! and [`TRANSPARENT`] by the shared-vector test in
//! `tests/unit/value_connective.rs`.

use serde_json::Value;

use crate::connective_footprints::Composed;
use crate::typed_c_ast::{children, kind};

/// A half-open token span of the full expression's token run.
pub(crate) type Span = (usize, usize);

/// Rule 11m: the composition of a region (its node) with its siblings.
pub(crate) type Composer<'a> = dyn Fn(&Value, &[&Value]) -> Result<(), Composed> + 'a;

/// One row of [`COMPOSED_VECTORS`].
#[cfg(test)]
pub(crate) type ComposedVector = (bool, bool, bool, Option<bool>, Option<bool>, bool);

/// One value region.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Region<'a> {
    /// The region's own tokens.
    pub(crate) span: Span,
    /// The region's Clang node.
    pub(crate) node: &'a Value,
    pub(crate) shape: Shape<'a>,
}

/// One arm: its tokens, its node and the value regions C evaluates inside it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arm<'a> {
    pub(crate) span: Span,
    pub(crate) node: &'a Value,
    pub(crate) regions: Vec<Region<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Shape<'a> {
    /// `A && B` or `A || B`: the region decides; its value is `1` or `0`.
    Boolean,
    /// `C ? A : B`.
    Select {
        condition: Span,
        arms: Box<[Arm<'a>; 2]>,
    },
    /// GNU `X ?: B`, `X` reads only; `middle` is `X`, whose node is the
    /// value on the non-zero edge.
    OmittedMiddle {
        middle: Span,
        middle_node: &'a Value,
        otherwise: Box<Arm<'a>>,
    },
}

impl Region<'_> {
    /// The form's [`crate::elaboration::FORM_TABLE`] row.
    pub(crate) fn form(&self) -> &'static str {
        match self.shape {
            Shape::Boolean => "value connective",
            Shape::Select { .. } => "conditional",
            Shape::OmittedMiddle { .. } => "omitted-middle conditional",
        }
    }

    /// The number of leaves the lowering produces for this region.
    pub(crate) fn leaves(&self) -> usize {
        let arm = |arm: &Arm| arm.regions.iter().map(Region::leaves).product::<usize>();
        match &self.shape {
            Shape::Boolean => 2,
            Shape::Select { arms, .. } => arm(&arms[0]) + arm(&arms[1]),
            Shape::OmittedMiddle { otherwise, .. } => 1 + arm(otherwise),
        }
    }
}

/// Why a full expression's regions are refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The full expression holds no value region.
    NoRegion,
    /// A connective under a node the evaluation reaches that is not a
    /// transparent position (the node's kind).
    Position(String),
    /// A sibling of a region has an effect C leaves unsequenced with the
    /// region's deciding operand.
    SiblingEffect,
    /// The deciding operand has an effect and a sibling reads what it could
    /// reach.
    UnorderedDecision,
    /// `X ?: B` whose `X` is not reads only (C evaluates it once).
    OmittedMiddleEffect,
    /// A node whose tokens are not the matching run of the full expression's.
    Span,
    /// More leaves than [`MAX_LEAVES`].
    Leaves(usize),
    /// Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m): rule 10z's premise
    /// refusal (`SiblingEffect` or `UnorderedDecision`) that the composition
    /// with CG05's footprints did not discharge, and why.
    Composition(Box<Refusal>, Composed),
}

impl Refusal {
    pub(crate) fn reason(&self) -> String {
        match self {
            Refusal::NoRegion => "no value connective or conditional".to_owned(),
            Refusal::Position(kind) => {
                format!("a value connective or conditional under `{kind}`")
            }
            Refusal::SiblingEffect => {
                "an operand unsequenced with the region carries an effect".to_owned()
            }
            Refusal::UnorderedDecision => {
                "a deciding operand with an effect beside an operand it could reach".to_owned()
            }
            Refusal::OmittedMiddleEffect => {
                "an omitted-middle operand that is not reads only (evaluated once)".to_owned()
            }
            Refusal::Span => "a region's tokens are not a run of the expression".to_owned(),
            Refusal::Leaves(count) => format!("{count} leaves exceed {MAX_LEAVES}"),
            Refusal::Composition(premise, composed) => format!(
                "{}; composed with CG05's footprints: {}",
                premise.reason(),
                composed.reason()
            ),
        }
    }
}

/// What replaces a region on one edge of the lowering.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Choice<'a> {
    /// The arm (or `X` of `X ?: B`) C evaluates on the edge.
    Node(&'a Value),
    /// The `int` value of a connective on the edge.
    Literal(&'static str),
}

/// `node` with every chosen region replaced by its choice on the edge: the
/// Clang node of the expression C evaluates on that path.
pub(crate) fn substitute(node: &Value, choices: &[(&Value, Choice<'_>)]) -> Value {
    if let Some((_, choice)) = choices
        .iter()
        .find(|(region, _)| std::ptr::eq(*region, node))
    {
        return match choice {
            Choice::Node(arm) => substitute(arm, choices),
            Choice::Literal(value) => serde_json::json!({
                "kind": "IntegerLiteral",
                "type": { "qualType": "int" },
                "valueCategory": "prvalue",
                "value": value,
            }),
        };
    }
    let mut copy = node.clone();
    if let Some(inner) = node["inner"].as_array() {
        copy["inner"] = Value::Array(
            inner
                .iter()
                .map(|child| substitute(child, choices))
                .collect(),
        );
    }
    copy
}

thread_local! {
    /// The regions chosen on the edge whose expression an older route is
    /// lowering: each region node's address and the child C evaluates there.
    static EDGE: std::cell::RefCell<Vec<(usize, usize)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn address(node: &Value) -> usize {
    std::ptr::from_ref(node) as usize
}

/// The child of `region` a choice names, when it names a node.
pub(crate) fn chosen_child(region: &Value, choice: Choice<'_>) -> Option<usize> {
    match choice {
        Choice::Node(arm) => children(region)
            .iter()
            .position(|child| std::ptr::eq(child, arm)),
        Choice::Literal(_) => None,
    }
}

/// Run `body` with `edge` (region node, chosen child) as the current edge.
pub(crate) fn on_edge<T>(edge: &[(&Value, Option<usize>)], body: impl FnOnce() -> T) -> T {
    let chosen = edge
        .iter()
        .filter_map(|(region, child)| child.map(|child| (address(region), child)))
        .collect::<Vec<_>>();
    // Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): restored on
    // every exit, an unwind included.
    struct Restore(Vec<(usize, usize)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = std::mem::take(&mut self.0);
            EDGE.with(|cell| *cell.borrow_mut() = previous);
        }
    }
    let _restore = Restore(EDGE.with(|cell| std::mem::replace(&mut *cell.borrow_mut(), chosen)));
    body()
}

/// The node C evaluates at `actual` on the current edge: through parentheses
/// and value-preserving implicit conversions (`NoOp`, `BitCast`,
/// `IntegralCast`), a region chosen on the edge is the arm C evaluates there.
/// `None` when no chosen region stands at `actual`. A call actual's typed
/// reader (rule 10k) asks this before matching the actual's tokens, so an
/// edge's call statement reads the arm the edge evaluates.
pub(crate) fn edge_node(actual: &Value) -> Option<&Value> {
    EDGE.with(|cell| {
        let edge = cell.borrow();
        if edge.is_empty() {
            return None;
        }
        let mut node = actual;
        let mut moved = false;
        loop {
            let mut bare = node;
            loop {
                match kind(bare) {
                    "ParenExpr" => {}
                    "ImplicitCastExpr"
                        if matches!(
                            bare["castKind"].as_str(),
                            Some("NoOp" | "BitCast" | "IntegralCast")
                        ) => {}
                    _ => break,
                }
                match children(bare).first() {
                    Some(inner) => bare = inner,
                    None => break,
                }
            }
            match edge.iter().find(|(region, _)| *region == address(bare)) {
                Some(&(_, child)) => {
                    node = children(bare).get(child)?;
                    moved = true;
                }
                None => break,
            }
        }
        moved.then_some(node)
    })
}

thread_local! {
    /// Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m): the full expression
    /// of the edge an older route is lowering, every region replaced by its
    /// choice ([`substitute`]).
    static EDGE_ROOT: std::cell::RefCell<Option<Value>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `body` with `root` (an edge's substituted full expression) as the
/// current edge's root.
pub(crate) fn with_edge_root<T>(root: Value, body: impl FnOnce() -> T) -> T {
    // Rule 12q: restored on every exit, an unwind included.
    struct Restore(Option<Value>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            EDGE_ROOT.with(|cell| *cell.borrow_mut() = previous);
        }
    }
    let _restore = Restore(EDGE_ROOT.with(|cell| cell.replace(Some(root))));
    body()
}

/// Rule 11m: the full expression C evaluates on the edge being lowered, when
/// one is. Rule 10y's consumers ask it where no Clang node carries the edge's
/// tokens (an edge's statement is spelled nowhere in the source): the edge's
/// node is the source node with each region replaced by the arm C evaluates
/// there, the node rule 10z's own `leaf_events` lowering reads.
pub(crate) fn edge_root() -> Option<Value> {
    EDGE_ROOT.with(|cell| cell.borrow().clone())
}

thread_local! {
    /// The deciding operand whose `if` condition routes are running.
    static DECIDING: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Run `body` while the condition routes decide `node`, a region's deciding
/// operand.
pub(crate) fn deciding<T>(node: &Value, body: impl FnOnce() -> T) -> T {
    // Rule 12q: restored on every exit, an unwind included.
    struct Restore(usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            DECIDING.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(DECIDING.with(|cell| cell.replace(address(node))));
    body()
}

/// Among several Clang nodes carrying an operand's line and tokens, the one
/// inside the deciding operand being lowered, when exactly one is. Asked only
/// after rule 10n's controlling-scope match declined.
pub(crate) fn deciding_candidate<'a>(
    root: &'a Value,
    candidates: &[&'a Value],
) -> Option<&'a Value> {
    let wanted = DECIDING.with(std::cell::Cell::get);
    if wanted == 0 {
        return None;
    }
    fn find(node: &Value, wanted: usize) -> Option<&Value> {
        if address(node) == wanted {
            return Some(node);
        }
        children(node).iter().find_map(|child| find(child, wanted))
    }
    fn inside(node: &Value, target: &Value) -> bool {
        std::ptr::eq(node, target) || children(node).iter().any(|child| inside(child, target))
    }
    let decided = find(root, wanted)?;
    let mut within = candidates
        .iter()
        .copied()
        .filter(|candidate| inside(decided, candidate));
    let one = within.next()?;
    within.next().is_none().then_some(one)
}

/// The most leaves one full expression is lowered into.
pub(crate) const MAX_LEAVES: usize = 8;

/// The nodes a region is found through, with the positions of their
/// children C evaluates as part of the node. `"root"` operands (`=`, `op=`)
/// are transparent only at the full expression's root.
pub(crate) const TRANSPARENT: [&str; 12] = [
    "ParenExpr",
    "ImplicitCastExpr",
    "CStyleCastExpr",
    "UnaryOperator",
    "BinaryOperator",
    "CallExpr actual",
    "MemberExpr",
    "ArraySubscriptExpr",
    "root =",
    "root op=",
    "root declaration",
    "root return",
];

/// The admission of one region by its premises
/// (`QCore.ValueConnectives.admit`).
pub(crate) fn admit(
    siblings_effect_free: bool,
    decision_effect_free: bool,
    siblings_inert: bool,
    omitted_middle_reads_only: Option<bool>,
) -> Result<(), Refusal> {
    if omitted_middle_reads_only == Some(false) {
        return Err(Refusal::OmittedMiddleEffect);
    }
    if !siblings_effect_free {
        return Err(Refusal::SiblingEffect);
    }
    if !decision_effect_free && !siblings_inert {
        return Err(Refusal::UnorderedDecision);
    }
    Ok(())
}

/// Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m): [`admit`], where its two
/// order refusals (`SiblingEffect`, `UnorderedDecision`) are asked of the
/// composition with CG05's footprints instead (`compose`, `None` when the
/// route supplies none: the refusal stays). An omitted-middle operand with an
/// effect is never composed: C evaluates it once and the lowering re-reads
/// it. `QCore.ValueConnectives.admitComposed`.
pub(crate) fn admit_composed(
    siblings_effect_free: bool,
    decision_effect_free: bool,
    siblings_inert: bool,
    omitted_middle_reads_only: Option<bool>,
    compose: Option<&dyn Fn() -> Result<(), Composed>>,
) -> Result<(), Refusal> {
    match admit(
        siblings_effect_free,
        decision_effect_free,
        siblings_inert,
        omitted_middle_reads_only,
    ) {
        Err(premise @ (Refusal::SiblingEffect | Refusal::UnorderedDecision)) => match compose {
            Some(compose) => {
                compose().map_err(|composed| Refusal::Composition(Box::new(premise), composed))
            }
            None => Err(premise),
        },
        other => other,
    }
}

/// `(siblings effect-free, decision effect-free, siblings inert, omitted
/// middle reads only, composition (`None`: none asked), admitted)`, the
/// shared vectors of `QCore.ValueConnectives.composedVectors`.
#[cfg(test)]
pub(crate) const COMPOSED_VECTORS: [ComposedVector; 10] = [
    (true, true, true, None, Some(false), true),
    (false, true, true, None, None, false),
    (false, true, true, None, Some(true), true),
    (false, true, true, None, Some(false), false),
    (true, false, false, None, Some(true), true),
    (true, false, false, None, Some(false), false),
    (false, false, false, None, Some(true), true),
    (true, true, true, Some(false), Some(true), false),
    (false, true, true, Some(false), Some(true), false),
    (false, true, true, Some(true), Some(true), true),
];

/// `(siblings effect-free, decision effect-free, siblings inert, omitted
/// middle reads only, admitted)`, the shared vectors of
/// `QCore.ValueConnectives.admissionVectors`.
#[cfg(test)]
pub(crate) const ADMISSION_VECTORS: [(bool, bool, bool, Option<bool>, bool); 8] = [
    (true, true, true, None, true),
    (true, true, false, None, true),
    (true, false, true, None, true),
    (true, false, false, None, false),
    (false, true, true, None, false),
    (true, true, true, Some(true), true),
    (true, true, true, Some(false), false),
    (false, false, false, Some(false), false),
];

fn is_connective(node: &Value) -> bool {
    match kind(node) {
        "ConditionalOperator" | "BinaryConditionalOperator" => true,
        "BinaryOperator" => matches!(node["opcode"].as_str(), Some("&&" | "||")),
        _ => false,
    }
}

/// Whether C evaluates a connective anywhere in `node` (never under `sizeof`).
pub(crate) fn holds_connective(node: &Value) -> bool {
    if kind(node) == "UnaryExprOrTypeTraitExpr" {
        return false;
    }
    is_connective(node) || children(node).iter().any(holds_connective)
}

fn volatile(node: &Value) -> bool {
    node["type"]["qualType"]
        .as_str()
        .is_some_and(|ty| ty.contains("volatile"))
}

/// Whether `node`'s evaluation is reads only: no call, store, increment,
/// statement expression or volatile access.
pub(crate) fn effect_free(node: &Value) -> bool {
    match kind(node) {
        "UnaryExprOrTypeTraitExpr" => return true,
        "CallExpr" | "StmtExpr" | "CompoundAssignOperator" => return false,
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => return false,
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => return false,
        "ImplicitCastExpr"
            if node["castKind"].as_str() == Some("LValueToRValue")
                && (volatile(node) || children(node).first().is_some_and(volatile)) =>
        {
            return false;
        }
        _ => {}
    }
    children(node).iter().all(effect_free)
}

/// Whether no call can change what `node` evaluates to or designates: a
/// literal, a designation (a named object, a function, a member or element
/// of a designation), or a read of an automatic object whose address is never
/// taken.
pub(crate) fn inert(node: &Value, unaliased: &dyn Fn(&str) -> bool) -> bool {
    match kind(node) {
        "IntegerLiteral"
        | "CharacterLiteral"
        | "FloatingLiteral"
        | "StringLiteral"
        | "UnaryExprOrTypeTraitExpr" => true,
        "DeclRefExpr" => true,
        "ImplicitCastExpr" if node["castKind"].as_str() == Some("LValueToRValue") => {
            let Some(inner) = children(node).first() else {
                return false;
            };
            let inner = crate::elaboration::strip_parens(inner);
            kind(inner) == "DeclRefExpr"
                && matches!(
                    inner["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                )
                && inner["referencedDecl"]["id"]
                    .as_str()
                    .is_some_and(unaliased)
                && !volatile(node)
        }
        "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "MemberExpr"
        | "ArraySubscriptExpr" => children(node).iter().all(|child| inert(child, unaliased)),
        "UnaryOperator" => {
            matches!(
                node["opcode"].as_str(),
                Some("-" | "+" | "~" | "!" | "*" | "&")
            ) && children(node).iter().all(|child| inert(child, unaliased))
        }
        "BinaryOperator" => {
            !matches!(node["opcode"].as_str(), Some("=" | "," | "&&" | "||"))
                && children(node).iter().all(|child| inert(child, unaliased))
        }
        _ => false,
    }
}

/// What [`regions`] asks of the route.
pub(crate) struct Reading<'a> {
    /// A node's token span within the full expression's run, `None` when its
    /// tokens are not that run's.
    pub(crate) locate: &'a dyn Fn(&Value) -> Option<Span>,
    pub(crate) unaliased: &'a dyn Fn(&str) -> bool,
    /// Card CG06-VALUE-CONNECTIVE-RESIDUALS (rule 11m): the composition of a
    /// region (its node) with its siblings by CG05's footprints, asked only
    /// where [`admit`] refuses a sibling's effect or a deciding operand's
    /// effect beside a sibling that is not inert. `None` keeps rule 10z's
    /// refusal.
    pub(crate) compose: Option<&'a Composer<'a>>,
}

/// The value regions of a full expression's root: a `DeclStmt`'s one
/// initializer, a statement's expression, or a `return` value.
pub(crate) fn regions<'a>(
    root: &'a Value,
    reading: &Reading<'_>,
) -> Result<Vec<Region<'a>>, Refusal> {
    let mut out = Vec::new();
    match kind(root) {
        "DeclStmt" => {
            let [declaration] = children(root) else {
                return Err(Refusal::Position("DeclStmt".into()));
            };
            if kind(declaration) != "VarDecl" || declaration["storageClass"].as_str().is_some() {
                return Err(Refusal::Position("DeclStmt".into()));
            }
            let [initializer] = children(declaration) else {
                return Err(Refusal::Position("VarDecl".into()));
            };
            walk(initializer, &[], reading, &mut out)?;
        }
        "BinaryOperator" if root["opcode"].as_str() == Some("=") => {
            descend(root, reading, &[], &mut out)?;
        }
        "CompoundAssignOperator" => descend(root, reading, &[], &mut out)?,
        _ => walk(root, &[], reading, &mut out)?,
    }
    if out.is_empty() {
        return Err(Refusal::NoRegion);
    }
    let leaves = out.iter().map(Region::leaves).product::<usize>();
    if leaves > MAX_LEAVES {
        return Err(Refusal::Leaves(leaves));
    }
    Ok(out)
}

/// Walk each child of a transparent node, the others its siblings.
fn descend<'a>(
    node: &'a Value,
    reading: &Reading<'_>,
    siblings: &[&'a Value],
    out: &mut Vec<Region<'a>>,
) -> Result<(), Refusal> {
    let all = children(node);
    for (at, child) in all.iter().enumerate() {
        let mut own = siblings.to_vec();
        own.extend(
            all.iter()
                .enumerate()
                .filter(|(other, _)| *other != at)
                .map(|(_, sibling)| sibling),
        );
        walk(child, &own, reading, out)?;
    }
    Ok(())
}

fn walk<'a>(
    node: &'a Value,
    siblings: &[&'a Value],
    reading: &Reading<'_>,
    out: &mut Vec<Region<'a>>,
) -> Result<(), Refusal> {
    if !holds_connective(node) {
        return Ok(());
    }
    if is_connective(node) {
        out.push(region(node, siblings, reading)?);
        return Ok(());
    }
    // The position the node is, in `TRANSPARENT`'s vocabulary: a nested `=`
    // or `,` is not an operator position (CG02 splits the sequenced forms).
    let position = match (kind(node), node["opcode"].as_str()) {
        ("BinaryOperator", Some(opcode @ ("=" | ","))) => format!("BinaryOperator {opcode}"),
        ("CallExpr", _) => {
            let callee = children(node)
                .first()
                .ok_or_else(|| Refusal::Position("CallExpr".into()))?;
            if holds_connective(callee) || crate::elaboration::direct_callee(callee).is_none() {
                return Err(Refusal::Position("indirect callee".into()));
            }
            "CallExpr actual".to_owned()
        }
        (other, _) => other.to_owned(),
    };
    if !TRANSPARENT.contains(&position.as_str()) {
        return Err(Refusal::Position(position));
    }
    descend(node, reading, siblings, out)
}

fn region<'a>(
    node: &'a Value,
    siblings: &[&'a Value],
    reading: &Reading<'_>,
) -> Result<Region<'a>, Refusal> {
    let span = (reading.locate)(node).ok_or(Refusal::Span)?;
    let decision = match kind(node) {
        "BinaryOperator" => node,
        _ => children(node).first().ok_or(Refusal::Span)?,
    };
    let omitted = (kind(node) == "BinaryConditionalOperator").then(|| effect_free(decision));
    // Rule 11m: where rule 10z's order premise refuses, the region is
    // composed with its siblings by CG05's footprints.
    let compose = reading
        .compose
        .map(|compose| move || compose(node, siblings));
    admit_composed(
        siblings.iter().all(|sibling| effect_free(sibling)),
        effect_free(decision),
        siblings
            .iter()
            .all(|sibling| inert(sibling, reading.unaliased)),
        omitted,
        compose.as_ref().map(|compose| compose as &dyn Fn() -> _),
    )?;
    // Nested regions of an arm keep the region's siblings; the arm's own
    // siblings are added as the walk descends into it.
    let arm = |arm: &'a Value| -> Result<Arm<'a>, Refusal> {
        // An unparenthesised comma middle operand (`C ? A, B : D`) would
        // re-parse differently once substituted into its position.
        let mut bare = arm;
        while kind(bare) == "ImplicitCastExpr" {
            bare = children(bare).first().ok_or(Refusal::Span)?;
        }
        if kind(bare) == "BinaryOperator" && bare["opcode"].as_str() == Some(",") {
            return Err(Refusal::Position("BinaryOperator ,".into()));
        }
        let span = (reading.locate)(arm).ok_or(Refusal::Span)?;
        let mut regions = Vec::new();
        walk(arm, siblings, reading, &mut regions)?;
        Ok(Arm {
            span,
            node: arm,
            regions,
        })
    };
    let shape = match kind(node) {
        "BinaryOperator" => Shape::Boolean,
        "ConditionalOperator" => {
            let [condition, yes, no] = children(node) else {
                return Err(Refusal::Span);
            };
            Shape::Select {
                condition: (reading.locate)(condition).ok_or(Refusal::Span)?,
                arms: Box::new([arm(yes)?, arm(no)?]),
            }
        }
        _ => {
            let [middle, _, _, no] = children(node) else {
                return Err(Refusal::Span);
            };
            Shape::OmittedMiddle {
                middle: (reading.locate)(middle).ok_or(Refusal::Span)?,
                middle_node: middle,
                otherwise: Box::new(arm(no)?),
            }
        }
    };
    Ok(Region { span, node, shape })
}

#[cfg(test)]
#[path = "../tests/unit/value_connective.rs"]
mod tests;
