//! Card **W44-BRANCH-CONDITION-CALLS**: desugaring a controlling expression
//! that carries modeled calls into a small CFG fragment.
//!
//! # What this module decides
//!
//! Cards B02 and B03 isolate **one** unconditionally evaluated contracted call
//! from a controlling expression ([`crate::legacy_condition_call`]). Every
//! other call-bearing condition — a short-circuit `&&`/`||`, a conditional
//! `?:`, two calls, a locally defined callee, `(c = f(x)) != EOF` — kept the
//! position's refusal (`unmodeled call, aggregate or indirect access in …`).
//!
//! This module lowers such a condition to what C evaluates:
//!
//! 1. The expression is read as a Boolean tree over `&&`, `||`, `!` and a
//!    top-level `?:` ([`parse`]). A **leaf** is a maximal operand that holds no
//!    top-level connective.
//! 2. Every call of a leaf becomes its own node, in C evaluation order, holding
//!    exactly the instruction the ordinary statement classifier gives the call
//!    as a statement (`Parser::classify_simple`). There is one call-effect
//!    model; this module adds none.
//! 3. The leaf then decides through one branch node: `NoOp` for a
//!    custody-neutral residual, or the existing null / validity / distinct /
//!    typed-read branch for a call-free leaf.
//! 4. The tree's connectives become **edges** ([`ConditionPlan`]): `A && B`
//!    evaluates `B` only on `A`'s true edge, `A || B` only on `A`'s false edge,
//!    `C ? A : B` evaluates `A` only on `C`'s true edge and `B` only on its
//!    false edge, and `!A` swaps `A`'s two exits. A call C skips is therefore
//!    never applied on a path where C skipped it, and a call C evaluates is
//!    applied on every path where C evaluates it — the evaluation-frequency
//!    theorem of `QCore.ConditionCallDesugar`.
//!
//! # Fresh temporaries
//!
//! Each call's value is conceptually a fresh temporary. An admitted call's
//! result is resource-free: its contract declares `resource_return_none()` or a
//! derived (non-owning) result, and the classifier itself refuses a discarded
//! owned result. A temporary that tracks nothing and is read by no refinement
//! binds nothing in the projection, so the node carries no destination — the
//! same lowering card B02 gives its one call. A custody result (owned,
//! reallocated, a descriptor) is **refused**, so `if ((p = malloc(n)) == NULL)`
//! keeps exactly the refusal it has today.
//!
//! The one written temporary is the C program's own: `(NAME = CALL(…))`, with
//! `NAME` a declared scalar or single-level pointer local that is no formal and
//! no descriptor name, lowers to the statement `NAME = CALL(…)` — the
//! classifier's `ContractCall` with destination `NAME`, binding whatever that
//! statement binds (an untracked value, or a derived pointer tracked by its
//! base) — and the leaf then decides on `NAME` exactly as a following
//! `if (NAME …)` would. `NAME` may not be read elsewhere in the leaf (C leaves
//! that unsequenced). An owned result assigned this way is custody and refuses.
//!
//! # Refusals (fail-closed)
//!
//! Any of the following keeps the whole condition refused, byte for byte:
//! braces, a type operator anywhere, an increment or decrement, the GNU `?:`
//! elvis operator, a call inside a connective nested below a leaf, a call
//! through a member or an expression, an uncontracted or refused call, a custody
//! result, a callee in the enclosing function's own recursive component (whose
//! contract may be a provisional assumption of the neutral route, constraint
//! 4), two calls in one leaf unless every one is the identity on the abstract
//! state (C leaves their order unspecified), a residual that is not
//! custody-neutral, a residual or call-free leaf reading a tracked descriptor
//! outside a validity test, and a switch expression that is not a single leaf.
//!
//! A declined plan restores `next_site` and the element-lowering counter, so a
//! condition this rule keeps refused keeps its site numbering and certificate
//! identity.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

use super::{
    CfgBuilder, LegacyInstruction, LegacyNodeId, Parser, Statement, SwitchTargets, TYPE_OPERATORS,
    Token, contains_braces, contains_call, descriptor_custody, is_identifier, matching,
};
use crate::neutral_position::ExpressionRole;

/// Whether the desugaring is consulted by the lowering. The census commits
/// (`2a4fe098`, `45d4c5ce`, `df5edb61`) shipped the recogniser with this
/// `false`, so the census answers are the rule's own while nothing was
/// admitted; the admission commit sets it.
pub(super) const ADMIT: bool = true;

/// The controlling-expression position a plan is built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Position {
    Branch,
    Loop,
    ForLoop,
    DoWhile,
    Switch,
}

impl Position {
    #[cfg(feature = "indirect-census")]
    fn label(self) -> &'static str {
        match self {
            Self::Branch => "if",
            Self::Loop => "while",
            Self::ForLoop => "for",
            Self::DoWhile => "do-while",
            Self::Switch => "switch",
        }
    }
}

/// Where one plan edge leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// The condition's true exit; for a switch, the dispatch to every case.
    True,
    /// The condition's false exit (the loop exit in a loop position).
    False,
    /// Another node of the same plan.
    Node(usize),
}

/// One node of a desugared condition.
#[derive(Clone, Debug)]
pub(super) struct PlanNode {
    pub(super) instruction: LegacyInstruction,
    pub(super) successors: Vec<Target>,
}

/// A desugared controlling expression: calls in evaluation order and one
/// branch node per leaf, wired by the Boolean connectives.
#[derive(Clone, Debug)]
pub(super) struct ConditionPlan {
    pub(super) nodes: Vec<PlanNode>,
    pub(super) entry: usize,
}

/// Why a call-bearing condition keeps its refusal. Census vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The expression carries no call; recorded by the census only.
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    NoCall,
    Braces,
    TypeOperator,
    IncrementOrDecrement,
    Elvis,
    Unparsed,
    SwitchShape,
    NestedConnective,
    IndirectCallee,
    AssignmentShape,
    AssignmentDestination(String),
    RecursiveLocalCallee(String),
    CallRefused(String, String),
    CustodyResult(String),
    UnsequencedEffects,
    ResidualRefused,
    DescriptorRead(String),
    CallFreeLeafRefused,
}

impl Refusal {
    #[cfg(feature = "indirect-census")]
    fn code(&self) -> (&'static str, Option<String>, Option<String>) {
        match self {
            Self::NoCall => ("no-call", None, None),
            Self::Braces => ("braces", None, None),
            Self::TypeOperator => ("type-operator", None, None),
            Self::IncrementOrDecrement => ("increment-or-decrement", None, None),
            Self::Elvis => ("elvis", None, None),
            Self::Unparsed => ("unparsed", None, None),
            Self::SwitchShape => ("switch-shape", None, None),
            Self::NestedConnective => ("nested-connective", None, None),
            Self::IndirectCallee => ("indirect-callee", None, None),
            Self::AssignmentShape => ("assignment-shape", None, None),
            Self::AssignmentDestination(name) => {
                ("assignment-destination", Some(name.clone()), None)
            }
            Self::RecursiveLocalCallee(name) => {
                ("recursive-local-callee", Some(name.clone()), None)
            }
            Self::CallRefused(name, why) => ("call-refused", Some(name.clone()), Some(why.clone())),
            Self::CustodyResult(name) => ("custody-result", Some(name.clone()), None),
            Self::UnsequencedEffects => ("unsequenced-effects", None, None),
            Self::ResidualRefused => ("residual-refused", None, None),
            Self::DescriptorRead(name) => ("descriptor-read", Some(name.clone()), None),
            Self::CallFreeLeafRefused => ("call-free-leaf-refused", None, None),
        }
    }
}

/// The Boolean structure of a controlling expression over token spans.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Tree {
    And(Box<Tree>, Box<Tree>),
    Or(Box<Tree>, Box<Tree>),
    Not(Box<Tree>),
    Choose(Box<Tree>, Box<Tree>, Box<Tree>),
    /// A half-open token span holding no top-level connective.
    Leaf(usize, usize),
}

/// The per-function state this rule keeps: the enclosing function's name and
/// a lazily read call graph of the translation unit, used only to decide
/// whether a locally defined callee lies in the function's own recursive
/// component.
#[derive(Clone, Debug, Default)]
pub(super) struct DesugarState {
    function: String,
    definitions: Option<BTreeSet<String>>,
    calls: BTreeMap<String, BTreeSet<String>>,
}

impl DesugarState {
    pub(super) fn new(function: &str) -> Self {
        Self {
            function: function.to_owned(),
            ..Self::default()
        }
    }
}

fn is_assignment_at(texts: &[&str], index: usize) -> bool {
    if texts[index] != "=" {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|i| texts.get(i)).copied();
    let next = texts.get(index + 1).copied();
    next != Some("=")
        && (!matches!(previous, Some("=" | "!" | "<" | ">"))
            || (matches!(previous, Some("<" | ">"))
                && index.checked_sub(2).and_then(|i| texts.get(i)).copied() == previous))
}

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip(texts: &[&str], mut start: usize, mut end: usize) -> (usize, usize) {
    while end > start + 1 && texts[start] == "(" && close_of(texts, start) == Some(end - 1) {
        start += 1;
        end -= 1;
    }
    (start, end)
}

/// Read `texts[start..end]` as a Boolean tree. `Err` is a refusal.
fn parse(texts: &[&str], start: usize, end: usize) -> Result<Tree, Refusal> {
    let (start, end) = strip(texts, start, end);
    if start >= end {
        return Err(Refusal::Unparsed);
    }
    // `__builtin_expect(E, K)` with a literal `K` has the value of `E`; both
    // are evaluated and `K` is a constant, so the call is transparent to the
    // Boolean reading (the `likely`/`unlikely` macros).
    if texts[start] == "__builtin_expect"
        && texts.get(start + 1) == Some(&"(")
        && close_of(texts, start + 1) == Some(end - 1)
    {
        let mut depth = 0usize;
        let mut comma = None;
        for index in start + 2..end - 1 {
            match texts[index] {
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.saturating_sub(1),
                "," if depth == 0 => {
                    if comma.replace(index).is_some() {
                        return Err(Refusal::Unparsed);
                    }
                }
                _ => {}
            }
        }
        let comma = comma.ok_or(Refusal::Unparsed)?;
        let (constant_start, constant_end) = strip(texts, comma + 1, end - 1);
        let constant = &texts[constant_start..constant_end];
        if !matches!(constant, [literal] if literal.bytes().all(|byte| byte.is_ascii_digit())) {
            return Err(Refusal::Unparsed);
        }
        return parse(texts, start + 2, comma);
    }
    let mut depth = 0usize;
    let mut question = None;
    let mut or = None;
    let mut and = None;
    let mut leaf_only = false;
    let mut index = start;
    while index < end {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1).ok_or(Refusal::Unparsed)?,
            _ if depth > 0 => {}
            "," => leaf_only = true,
            "=" if is_assignment_at(texts, index) => leaf_only = true,
            "?" => {
                if question.is_none() {
                    question = Some(index);
                }
            }
            "|" if texts.get(index + 1) == Some(&"|") && index + 1 < end => {
                if or.is_none() {
                    or = Some(index);
                }
                index += 1;
            }
            "&" if texts.get(index + 1) == Some(&"&") && index + 1 < end => {
                if and.is_none() {
                    and = Some(index);
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    if depth != 0 {
        return Err(Refusal::Unparsed);
    }
    if leaf_only {
        return Ok(Tree::Leaf(start, end));
    }
    if let Some(question) = question {
        let colon = matching_colon(texts, question, end).ok_or(Refusal::Unparsed)?;
        return Ok(Tree::Choose(
            Box::new(parse(texts, start, question)?),
            Box::new(parse(texts, question + 1, colon)?),
            Box::new(parse(texts, colon + 1, end)?),
        ));
    }
    if let Some(or) = or {
        return Ok(Tree::Or(
            Box::new(parse(texts, start, or)?),
            Box::new(parse(texts, or + 2, end)?),
        ));
    }
    if let Some(and) = and {
        return Ok(Tree::And(
            Box::new(parse(texts, start, and)?),
            Box::new(parse(texts, and + 2, end)?),
        ));
    }
    if texts[start] == "!" {
        let (inner_start, inner_end) = strip(texts, start + 1, end);
        let grouped = inner_start > start + 1 || texts.get(start + 1) == Some(&"!");
        let negated_call = texts.get(start + 2) == Some(&"(")
            && is_identifier(texts[start + 1])
            && close_of(texts, start + 2) == Some(end - 1);
        if grouped || negated_call {
            return Ok(Tree::Not(Box::new(parse(texts, inner_start, inner_end)?)));
        }
    }
    Ok(Tree::Leaf(start, end))
}

/// The `:` matching the `?` at `question`, at the same parenthesis depth.
fn matching_colon(texts: &[&str], question: usize, end: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut pending = 0usize;
    for (index, text) in texts.iter().enumerate().take(end).skip(question + 1) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "?" if depth == 0 => pending += 1,
            ":" if depth == 0 => {
                if pending == 0 {
                    return Some(index);
                }
                pending -= 1;
            }
            _ => {}
        }
    }
    None
}

fn leaves(tree: &Tree, out: &mut Vec<(usize, usize)>) {
    match tree {
        Tree::And(left, right) | Tree::Or(left, right) => {
            leaves(left, out);
            leaves(right, out);
        }
        Tree::Not(inner) => leaves(inner, out),
        Tree::Choose(condition, yes, no) => {
            leaves(condition, out);
            leaves(yes, out);
            leaves(no, out);
        }
        Tree::Leaf(start, end) => out.push((*start, *end)),
    }
}

/// One leaf, lowered: its calls in evaluation order and its branch.
struct LeafPlan {
    calls: Vec<LegacyInstruction>,
    decision: LegacyInstruction,
}

/// Whether `instruction` is the identity on the abstract state: a contracted
/// call whose every position borrows, whose result owns nothing, which
/// produces no site and carries no member read or address that could take
/// custody.
fn is_identity_call(instruction: &LegacyInstruction) -> bool {
    let LegacyInstruction::ContractCall {
        contract,
        produced_sites,
        aggregate_consumed,
        aggregate_addresses,
        nested,
        destination: None,
        ..
    } = instruction
    else {
        return false;
    };
    contract
        .parameters
        .iter()
        .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
        && matches!(
            contract.result,
            ResourceReturnMode::None | ResourceReturnMode::Derived { .. }
        )
        && produced_sites.is_empty()
        && aggregate_consumed.is_empty()
        && aggregate_addresses.is_empty()
        && nested.iter().all(|call| {
            call.contract
                .parameters
                .iter()
                .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                && call.contract.result == ResourceReturnMode::None
        })
}

/// A call span of a leaf: `[callee, close]` inclusive of the closing `)`.
struct CallSpan {
    callee: usize,
    close: usize,
}

/// The one `NAME = CALL(...)` of a leaf, in leaf token indices.
struct Assignment {
    /// The span the residual replaces by `( NAME )`.
    start: usize,
    end: usize,
    /// Where the statement `NAME = CALL(...)` starts.
    statement: usize,
    /// The callee token of the assigned call.
    call: usize,
    name: String,
}

impl Parser<'_> {
    /// The hook the statement parser calls for every controlling expression.
    ///
    /// `refused` states that every pre-existing route refused the expression,
    /// so the lowering would otherwise be the position's refusal. Only then,
    /// and only for an expression carrying a call, is a plan built; a plan
    /// that is not used restores the classifier's counters.
    pub(super) fn planned_condition(
        &mut self,
        _line: usize,
        condition: &[Token],
        position: Position,
        refused: bool,
    ) -> Option<Box<ConditionPlan>> {
        if !refused {
            return None;
        }
        if !contains_call(condition) {
            // Census only: a refused call-free expression shares its leaf
            // with any call-bearing one on the same line and position.
            #[cfg(feature = "indirect-census")]
            if !self.admit_derived_returns {
                census(self, _line, position, condition, &Err(Refusal::NoCall));
            }
            return None;
        }
        let saved = (self.next_site, self.element_lowerings);
        let outcome = self.condition_plan(condition, position);
        // A derived-result validation parse re-reads a body the ordinary
        // parse already recorded.
        #[cfg(feature = "indirect-census")]
        if !self.admit_derived_returns {
            census(self, _line, position, condition, &outcome);
        }
        match outcome {
            Ok(plan) if ADMIT => Some(Box::new(plan)),
            _ => {
                (self.next_site, self.element_lowerings) = saved;
                None
            }
        }
    }

    /// The desugared plan of a refused call-bearing controlling expression.
    ///
    /// Called only where every pre-existing route refused the expression.
    /// Any refusal restores the classifier's site counter and element counter.
    pub(super) fn condition_plan(
        &mut self,
        condition: &[Token],
        position: Position,
    ) -> Result<ConditionPlan, Refusal> {
        let saved = (self.next_site, self.element_lowerings);
        let result = self.condition_plan_unguarded(condition, position);
        if result.is_err() {
            (self.next_site, self.element_lowerings) = saved;
        }
        result
    }

    fn condition_plan_unguarded(
        &mut self,
        condition: &[Token],
        position: Position,
    ) -> Result<ConditionPlan, Refusal> {
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if contains_braces(condition) {
            return Err(Refusal::Braces);
        }
        if texts.iter().any(|text| TYPE_OPERATORS.contains(text)) {
            return Err(Refusal::TypeOperator);
        }
        if texts
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        {
            return Err(Refusal::IncrementOrDecrement);
        }
        if texts.windows(2).any(|pair| pair == ["?", ":"]) {
            return Err(Refusal::Elvis);
        }
        let tree = parse(&texts, 0, texts.len())?;
        if position == Position::Switch && !matches!(tree, Tree::Leaf(..)) {
            return Err(Refusal::SwitchShape);
        }
        let mut spans = Vec::new();
        leaves(&tree, &mut spans);
        // Classified in textual order, which is C's evaluation order across
        // the sequence points `&&`, `||` and `?` introduce, so sites are
        // numbered in the order C creates them.
        let mut plans = Vec::with_capacity(spans.len());
        for &(start, end) in &spans {
            plans.push(Some(self.leaf_plan(&condition[start..end], position)?));
        }
        let mut counter = 0usize;
        let numbered = number(&tree, &mut counter);
        let mut nodes = Vec::new();
        let entry = assemble(
            &numbered,
            Target::True,
            Target::False,
            &mut plans,
            &mut nodes,
        );
        let Target::Node(entry) = entry else {
            return Err(Refusal::Unparsed);
        };
        Ok(ConditionPlan { nodes, entry })
    }

    fn leaf_plan(&mut self, tokens: &[Token], position: Position) -> Result<LeafPlan, Refusal> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !contains_call(tokens) {
            return self
                .call_free_decision(tokens, position)
                .ok_or(Refusal::CallFreeLeafRefused);
        }
        let calls = top_level_calls(&texts)?;
        // A connective below the leaf's top level would make a call's
        // evaluation conditional in a way this leaf's single branch cannot
        // express.
        let mut index = 0usize;
        let mut assignments = Vec::new();
        while index < texts.len() {
            if let Some(call) = calls.iter().find(|call| call.callee == index) {
                index = call.close + 1;
                continue;
            }
            match texts[index] {
                "?" => return Err(Refusal::NestedConnective),
                "&" if texts.get(index + 1) == Some(&"&") => {
                    return Err(Refusal::NestedConnective);
                }
                "|" if texts.get(index + 1) == Some(&"|") => {
                    return Err(Refusal::NestedConnective);
                }
                "=" if is_assignment_at(&texts, index) => assignments.push(index),
                _ => {}
            }
            index += 1;
        }
        // At most one assignment, of exactly the form `NAME = CALL(...)`,
        // either the whole leaf or one parenthesized group of it.
        let assigned = match assignments.as_slice() {
            [] => None,
            [at] => Some(self.assignment_group(&texts, &calls, *at)?),
            _ => return Err(Refusal::AssignmentShape),
        };
        let mut lowered = Vec::with_capacity(calls.len());
        for call in &calls {
            let callee = texts[call.callee];
            if let Some(body_owner) = self.recursive_local_callee(callee) {
                return Err(Refusal::RecursiveLocalCallee(body_owner));
            }
            let (statement, destination) = match &assigned {
                Some(group) if call.callee == group.call => (
                    &tokens[group.statement..=call.close],
                    Some(group.name.as_str()),
                ),
                _ => (&tokens[call.callee..=call.close], None),
            };
            let instruction = self.classify_simple(statement);
            match &instruction {
                LegacyInstruction::ContractCall {
                    destination: written,
                    contract,
                    ..
                } => {
                    if written.as_deref() != destination {
                        return Err(Refusal::CallRefused(
                            callee.to_owned(),
                            "unexpected destination".to_owned(),
                        ));
                    }
                    // An owned, reallocated or descriptor result is custody
                    // and refuses. A derived result owns nothing: discarded,
                    // it is exactly a discarded `None` (the classifier's own
                    // W43-LIBC-RESULT-ALIAS rule); written to the program's
                    // own pointer, that binding tracks it.
                    let resource_free = matches!(
                        contract.result,
                        ResourceReturnMode::None | ResourceReturnMode::Derived { .. }
                    );
                    if !resource_free {
                        return Err(Refusal::CustodyResult(callee.to_owned()));
                    }
                }
                LegacyInstruction::Unknown { description, .. } => {
                    return Err(Refusal::CallRefused(
                        callee.to_owned(),
                        census_reason(description),
                    ));
                }
                other => {
                    return Err(Refusal::CallRefused(
                        callee.to_owned(),
                        format!("lowered to {}", variant_name(other)),
                    ));
                }
            }
            lowered.push(instruction);
        }
        if lowered.len() > 1 {
            // C does not sequence two calls of one operand. Only calls that
            // are the identity on the abstract state commute; with one of
            // them written through, neither order may be chosen.
            if assigned.is_some() || !lowered.iter().all(is_identity_call) {
                return Err(Refusal::UnsequencedEffects);
            }
        }
        // The residual: every call replaced by a neutral literal, the one
        // assignment group by its destination.
        let mut residual = Vec::with_capacity(tokens.len());
        let mut index = 0usize;
        while index < tokens.len() {
            if let Some(group) = &assigned
                && index == group.start
            {
                residual.push(synthetic("(", &tokens[index]));
                residual.push(synthetic(&group.name, &tokens[index]));
                residual.push(synthetic(")", &tokens[index]));
                index = group.end;
                continue;
            }
            if let Some(call) = calls.iter().find(|call| call.callee == index) {
                residual.push(synthetic("0", &tokens[index]));
                index = call.close + 1;
                continue;
            }
            residual.push(tokens[index].clone());
            index += 1;
        }
        let residual_texts = residual
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if let Some(name) =
            descriptor_custody::value_carries_descriptor(&residual_texts, &self.descriptor_names)
        {
            return Err(Refusal::DescriptorRead(name.to_owned()));
        }
        let decision = if assigned.is_some() {
            match self.null_condition(&residual) {
                Some(test) if test.validity.is_none() => test.branch(),
                Some(_) => return Err(Refusal::ResidualRefused),
                None if self.supported_expression(&residual, ExpressionRole::Decides) => {
                    LegacyInstruction::NoOp
                }
                None => return Err(Refusal::ResidualRefused),
            }
        } else if self.supported_expression(&residual, ExpressionRole::Decides) {
            LegacyInstruction::NoOp
        } else {
            return Err(Refusal::ResidualRefused);
        };
        if position == Position::Switch && decision != LegacyInstruction::NoOp {
            return Err(Refusal::SwitchShape);
        }
        Ok(LeafPlan {
            calls: lowered,
            decision,
        })
    }

    /// The branch a call-free leaf decides through: the same routes, in the
    /// same order, the whole-condition lowering consults.
    fn call_free_decision(&self, tokens: &[Token], position: Position) -> Option<LeafPlan> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let decision = if let Some(test) = self.null_condition(tokens) {
            test.branch()
        } else if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names)
            .is_some()
        {
            return None;
        } else if position == Position::Branch
            && let Some(distinct) = self.distinct_condition(tokens)
        {
            LegacyInstruction::BranchOnDistinct {
                left: distinct.left,
                right: distinct.right,
                true_is_distinct: distinct.true_is_distinct,
            }
        } else if self.supported_expression(tokens, ExpressionRole::Decides) {
            LegacyInstruction::NoOp
        } else {
            *self.condition_access(tokens)?
        };
        if position == Position::Switch && decision != LegacyInstruction::NoOp {
            return None;
        }
        Some(LeafPlan {
            calls: Vec::new(),
            decision,
        })
    }

    /// The assignment group holding the `=` at `at`: exactly
    /// `NAME = CALL(...)`, either the whole leaf or one parenthesized group
    /// `( NAME = CALL(...) )` of it.
    fn assignment_group(
        &self,
        texts: &[&str],
        calls: &[CallSpan],
        at: usize,
    ) -> Result<Assignment, Refusal> {
        let name_index = at.checked_sub(1).ok_or(Refusal::AssignmentShape)?;
        let name = texts[name_index];
        let call = calls
            .iter()
            .find(|call| call.callee == at + 1)
            .ok_or(Refusal::AssignmentShape)?;
        if !is_identifier(name) {
            return Err(Refusal::AssignmentShape);
        }
        let (start, end, statement) = if name_index == 0 && call.close + 1 == texts.len() {
            (0, texts.len(), 0)
        } else {
            let open = name_index.checked_sub(1).ok_or(Refusal::AssignmentShape)?;
            if texts[open] != "(" || texts.get(call.close + 1) != Some(&")") {
                return Err(Refusal::AssignmentShape);
            }
            (open, call.close + 2, name_index)
        };
        // A declared scalar, or a single-level pointer the reader committed
        // to; the classifier decides what the call's result binds to it.
        let declared = self.typed_types.declared_is_scalar(name)
            || self.typed_types.pointer_kind(name).is_some();
        if !declared
            || self.descriptor_names.contains(name)
            || self.formal_parameters.contains(name)
        {
            return Err(Refusal::AssignmentDestination(name.to_owned()));
        }
        // `NAME` read elsewhere in the leaf is unsequenced with the write.
        if texts
            .iter()
            .enumerate()
            .any(|(index, text)| !(start..end).contains(&index) && *text == name)
        {
            return Err(Refusal::AssignmentDestination(name.to_owned()));
        }
        Ok(Assignment {
            start,
            end,
            statement,
            call: call.callee,
            name: name.to_owned(),
        })
    }

    /// `Some(callee)` when `callee` has a body in this translation unit and
    /// lies in the enclosing function's recursive component: it reaches the
    /// function back through the unit's direct-call graph. Such a callee's
    /// contract in force may be the neutral route's provisional `N(m)`
    /// (constraint 4), so it is not read as admission evidence.
    fn recursive_local_callee(&mut self, callee: &str) -> Option<String> {
        let tokens = self.tokens;
        let state = &mut self.desugar;
        if state.function.is_empty() {
            return Some(callee.to_owned());
        }
        let definitions = state
            .definitions
            .get_or_insert_with(|| unit_definitions(tokens));
        if !definitions.contains(callee) {
            return None;
        }
        if callee == state.function {
            return Some(callee.to_owned());
        }
        let definitions = definitions.clone();
        let mut seen = BTreeSet::from([callee.to_owned()]);
        let mut work = vec![callee.to_owned()];
        while let Some(name) = work.pop() {
            let called = state
                .calls
                .entry(name.clone())
                .or_insert_with(|| super::direct_calls(tokens, &name, &definitions))
                .clone();
            for next in called {
                if next == state.function {
                    return Some(callee.to_owned());
                }
                if seen.insert(next.clone()) {
                    work.push(next);
                }
            }
        }
        None
    }
}

/// Every name `find_function_body` would find a body for: an identifier
/// followed by a parenthesized group followed by `{`, anywhere in the unit.
/// A superset of the derivation's candidates, so a component it forms is
/// contained in one this graph forms.
fn unit_definitions(tokens: &[Token]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index + 1].text != "(" || !is_identifier(&tokens[index].text) {
            continue;
        }
        if let Some(close) = matching(tokens, index + 1, "(", ")")
            && tokens.get(close + 1).is_some_and(|token| token.text == "{")
        {
            names.insert(tokens[index].text.clone());
        }
    }
    names
}

fn synthetic(text: &str, at: &Token) -> Token {
    Token {
        text: text.to_owned(),
        line: at.line,
    }
}

/// The calls of a leaf that are not inside another call's argument list.
/// A call through a member (`p->f(…)`, `s.f(…)`) or an expression
/// (`(*fp)(…)`) refuses: its callee is not a declared name.
fn top_level_calls(texts: &[&str]) -> Result<Vec<CallSpan>, Refusal> {
    let mut calls = Vec::new();
    let mut index = 0usize;
    while index + 1 < texts.len() {
        if texts[index] == ")" && texts[index + 1] == "(" {
            return Err(Refusal::IndirectCallee);
        }
        if is_identifier(texts[index]) && texts[index + 1] == "(" {
            let member = index >= 1
                && (texts[index - 1] == "."
                    || (texts[index - 1] == ">" && index >= 2 && texts[index - 2] == "-"));
            if member {
                return Err(Refusal::IndirectCallee);
            }
            let close = close_of(texts, index + 1).ok_or(Refusal::Unparsed)?;
            calls.push(CallSpan {
                callee: index,
                close,
            });
            index = close + 1;
            continue;
        }
        index += 1;
    }
    Ok(calls)
}

/// The census's short name for a classifier refusal: backticked operands
/// collapsed, as the blocker census normalises them.
fn census_reason(description: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for ch in description.chars() {
        if ch == '`' {
            if !inside {
                out.push_str("`_`");
            }
            inside = !inside;
        } else if !inside {
            out.push(ch);
        }
    }
    out
}

fn variant_name(instruction: &LegacyInstruction) -> String {
    let debug = format!("{instruction:?}");
    debug
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The tree with each leaf replaced by its textual index, the index of its
/// plan in the order [`leaves`] lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Numbered {
    And(Box<Numbered>, Box<Numbered>),
    Or(Box<Numbered>, Box<Numbered>),
    Not(Box<Numbered>),
    Choose(Box<Numbered>, Box<Numbered>, Box<Numbered>),
    Leaf(usize),
}

fn number(tree: &Tree, counter: &mut usize) -> Numbered {
    match tree {
        Tree::And(left, right) => {
            let left = number(left, counter);
            Numbered::And(Box::new(left), Box::new(number(right, counter)))
        }
        Tree::Or(left, right) => {
            let left = number(left, counter);
            Numbered::Or(Box::new(left), Box::new(number(right, counter)))
        }
        Tree::Not(inner) => Numbered::Not(Box::new(number(inner, counter))),
        Tree::Choose(condition, yes, no) => {
            let condition = number(condition, counter);
            let yes = number(yes, counter);
            Numbered::Choose(
                Box::new(condition),
                Box::new(yes),
                Box::new(number(no, counter)),
            )
        }
        Tree::Leaf(..) => {
            *counter += 1;
            Numbered::Leaf(*counter - 1)
        }
    }
}

/// Wire the leaves into nodes: `A && B` enters `B` from `A`'s true edge,
/// `A || B` from its false edge, `C ? A : B` enters `A` from `C`'s true edge
/// and `B` from its false edge, and `!A` swaps `A`'s exits. Returns the entry.
fn assemble(
    tree: &Numbered,
    when_true: Target,
    when_false: Target,
    plans: &mut [Option<LeafPlan>],
    nodes: &mut Vec<PlanNode>,
) -> Target {
    match tree {
        Numbered::Leaf(index) => {
            let plan = plans[*index].take().expect("each leaf is assembled once");
            let decision = nodes.len();
            nodes.push(PlanNode {
                instruction: plan.decision,
                successors: vec![when_true, when_false],
            });
            let mut entry = Target::Node(decision);
            for call in plan.calls.into_iter().rev() {
                let node = nodes.len();
                nodes.push(PlanNode {
                    instruction: call,
                    successors: vec![entry],
                });
                entry = Target::Node(node);
            }
            entry
        }
        Numbered::Not(inner) => assemble(inner, when_false, when_true, plans, nodes),
        Numbered::And(left, right) => {
            let right_entry = assemble(right, when_true, when_false, plans, nodes);
            assemble(left, right_entry, when_false, plans, nodes)
        }
        Numbered::Or(left, right) => {
            let right_entry = assemble(right, when_true, when_false, plans, nodes);
            assemble(left, when_true, right_entry, plans, nodes)
        }
        Numbered::Choose(condition, yes, no) => {
            let yes_entry = assemble(yes, when_true, when_false, plans, nodes);
            let no_entry = assemble(no, when_true, when_false, plans, nodes);
            assemble(condition, yes_entry, no_entry, plans, nodes)
        }
    }
}

impl CfgBuilder {
    /// Push every plan node with no successors yet; returns their ids.
    fn allocate_plan(&mut self, plan: &ConditionPlan, line: usize) -> Vec<LegacyNodeId> {
        plan.nodes
            .iter()
            .map(|node| self.push(node.instruction.clone(), Vec::new(), line, false))
            .collect()
    }

    /// Wire the allocated plan. `when_true` is the true exit (every case entry
    /// for a switch), `when_false` the false exit. In a loop, every edge to
    /// the false exit is recorded as a loop exit edge, which the value route
    /// may never remove (card SCALAR-ROUTE-EXIT-EDGE).
    fn connect_plan(
        &mut self,
        plan: &ConditionPlan,
        ids: &[LegacyNodeId],
        when_true: &[LegacyNodeId],
        when_false: Option<LegacyNodeId>,
        loop_exit: bool,
    ) {
        for (node, &id) in plan.nodes.iter().zip(ids) {
            let mut successors = Vec::new();
            let mut exit_edge = None;
            for target in &node.successors {
                match target {
                    Target::True => successors.extend_from_slice(when_true),
                    Target::False => {
                        if let Some(target) = when_false {
                            exit_edge.get_or_insert(successors.len());
                            successors.push(target);
                        }
                    }
                    Target::Node(index) => successors.push(ids[*index]),
                }
            }
            self.nodes[id].successors = successors;
            if loop_exit {
                self.nodes[id].loop_exit_edge = exit_edge;
            }
        }
    }

    /// `if` with a desugared condition.
    pub(super) fn compile_planned_if(
        &mut self,
        plan: &ConditionPlan,
        line: usize,
        then_entry: LegacyNodeId,
        else_entry: LegacyNodeId,
    ) -> LegacyNodeId {
        let ids = self.allocate_plan(plan, line);
        self.connect_plan(plan, &ids, &[then_entry], Some(else_entry), false);
        ids[plan.entry]
    }

    /// `while` with a desugared condition: the plan's entry is the header
    /// every back edge and `continue` revisits.
    pub(super) fn compile_planned_while(
        &mut self,
        plan: &ConditionPlan,
        line: usize,
        body: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        let ids = self.allocate_plan(plan, line);
        let header = ids[plan.entry];
        self.break_targets.push(continuation);
        self.continue_targets.push(header);
        let body_entry = self.compile_sequence(body, header);
        self.continue_targets.pop();
        self.break_targets.pop();
        self.connect_plan(plan, &ids, &[body_entry], Some(continuation), true);
        header
    }

    /// `for` with a desugared condition: the increment returns to the plan's
    /// entry, `continue` reaches the increment, and the init precedes it.
    pub(super) fn compile_planned_for(
        &mut self,
        plan: &ConditionPlan,
        line: usize,
        init: Option<&(LegacyInstruction, usize)>,
        increment: Option<&(LegacyInstruction, usize)>,
        body: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        let ids = self.allocate_plan(plan, line);
        let header = ids[plan.entry];
        let increment_entry = increment.map_or(header, |(instruction, line)| {
            self.push(instruction.clone(), vec![header], *line, false)
        });
        self.break_targets.push(continuation);
        self.continue_targets.push(increment_entry);
        let body_entry = self.compile_sequence(body, increment_entry);
        self.continue_targets.pop();
        self.break_targets.pop();
        self.connect_plan(plan, &ids, &[body_entry], Some(continuation), true);
        init.map_or(header, |(instruction, line)| {
            self.push(instruction.clone(), vec![header], *line, false)
        })
    }

    /// `do … while` with a desugared condition: the body runs first; the plan
    /// is where the body's end and `continue` arrive.
    pub(super) fn compile_planned_do_while(
        &mut self,
        plan: &ConditionPlan,
        line: usize,
        body: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        let ids = self.allocate_plan(plan, line);
        let header = ids[plan.entry];
        self.break_targets.push(continuation);
        self.continue_targets.push(header);
        let body_entry = self.compile_sequence(body, header);
        self.continue_targets.pop();
        self.break_targets.pop();
        self.connect_plan(plan, &ids, &[body_entry], Some(continuation), true);
        body_entry
    }

    /// `switch` with a desugared expression: the calls run once, then the
    /// dispatch node selects a case exactly as the undesugared switch does.
    pub(super) fn compile_planned_switch(
        &mut self,
        plan: &ConditionPlan,
        line: usize,
        body: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        self.break_targets.push(continuation);
        self.switch_cases.push(SwitchTargets::default());
        let _body_entry = self.compile_sequence(body, continuation);
        let targets = self.switch_cases.pop().expect("switch target scope");
        self.break_targets.pop();
        let mut successors = targets.entries;
        if !targets.has_default || successors.is_empty() {
            successors.push(continuation);
        }
        successors.sort_unstable();
        successors.dedup();
        let ids = self.allocate_plan(plan, line);
        self.connect_plan(plan, &ids, &successors, None, false);
        ids[plan.entry]
    }
}

/// Card census hook: record one refused call-bearing controlling expression
/// with this rule's answer. Compiled out with the feature off.
#[cfg(feature = "indirect-census")]
pub(super) fn census(
    parser: &Parser<'_>,
    line: usize,
    position: Position,
    condition: &[Token],
    outcome: &Result<ConditionPlan, Refusal>,
) {
    let texts = condition
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let mut callees = Vec::new();
    for pair in texts.windows(2) {
        if !is_identifier(pair[0]) || pair[1] != "(" || TYPE_OPERATORS.contains(&pair[0]) {
            continue;
        }
        let contract = parser.contracts.contract(pair[0]);
        callees.push(crate::condition_call_census::Callee {
            name: pair[0].to_owned(),
            contracted: contract.is_some(),
            local: super::find_function_body(parser.tokens, pair[0]).is_some(),
            result: contract.map(|contract| match contract.result {
                ResourceReturnMode::None => "none",
                ResourceReturnMode::Owned { .. } => "owned",
                ResourceReturnMode::Reallocated { .. } => "reallocated",
                ResourceReturnMode::Derived { .. } => "derived",
            }),
        });
    }
    let (code, subject, reason) = match outcome {
        Ok(_) => ("admitted", None, None),
        Err(refusal) => refusal.code(),
    };
    crate::condition_call_census::record(crate::condition_call_census::Record {
        unit_seq: crate::indirect_census::unit_seq(),
        function: parser.desugar.function.clone(),
        line,
        position: position.label(),
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        outcome: code,
        subject,
        reason,
        nodes: outcome.as_ref().map_or(0, |plan| plan.nodes.len()),
        callees,
    });
}

#[cfg(test)]
mod tests {
    use super::{Tree, parse};

    fn tree(text: &str) -> Result<Tree, super::Refusal> {
        let texts = text.split_whitespace().collect::<Vec<_>>();
        parse(&texts, 0, texts.len())
    }

    #[test]
    fn connectives_split_at_the_top_level_only() {
        assert_eq!(
            tree("a & & f ( b )").unwrap(),
            Tree::And(Box::new(Tree::Leaf(0, 1)), Box::new(Tree::Leaf(3, 7)))
        );
        assert_eq!(
            tree("( a | | b ) & & c").unwrap(),
            Tree::And(
                Box::new(Tree::Or(
                    Box::new(Tree::Leaf(1, 2)),
                    Box::new(Tree::Leaf(4, 5))
                )),
                Box::new(Tree::Leaf(8, 9))
            )
        );
        assert_eq!(tree("f ( a & & b )").unwrap(), Tree::Leaf(0, 7));
    }

    #[test]
    fn a_conditional_and_a_negation_are_read() {
        assert_eq!(
            tree("c ? a : b").unwrap(),
            Tree::Choose(
                Box::new(Tree::Leaf(0, 1)),
                Box::new(Tree::Leaf(2, 3)),
                Box::new(Tree::Leaf(4, 5))
            )
        );
        assert_eq!(
            tree("! ( a & & b )").unwrap(),
            Tree::Not(Box::new(Tree::And(
                Box::new(Tree::Leaf(2, 3)),
                Box::new(Tree::Leaf(5, 6))
            )))
        );
        assert_eq!(
            tree("! f ( a )").unwrap(),
            Tree::Not(Box::new(Tree::Leaf(1, 5)))
        );
    }

    #[test]
    fn an_assignment_or_a_comma_keeps_the_whole_operand_a_leaf() {
        assert_eq!(tree("x = a & & b").unwrap(), Tree::Leaf(0, 6));
        assert_eq!(tree("a , b & & c").unwrap(), Tree::Leaf(0, 6));
    }

    #[test]
    fn builtin_expect_is_transparent_only_with_a_literal_constant() {
        assert_eq!(
            tree("__builtin_expect ( ! ! ( a & & b ) , 0 )").unwrap(),
            Tree::Not(Box::new(Tree::Not(Box::new(Tree::And(
                Box::new(Tree::Leaf(5, 6)),
                Box::new(Tree::Leaf(8, 9))
            )))))
        );
        assert!(tree("__builtin_expect ( a , k )").is_err());
    }
}
