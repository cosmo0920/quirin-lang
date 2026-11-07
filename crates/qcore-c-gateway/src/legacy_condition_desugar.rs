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

use super::call_instantiation_route::CallRefusal;
use super::condition_acquisition_route as acquisition;
use super::{
    CfgBuilder, LegacyInstruction, LegacyNodeId, Parser, Statement, SwitchTargets, TYPE_OPERATORS,
    Token, contains_braces, descriptor_custody, is_identifier, matching,
};
use crate::call_instantiation::Decline;
use crate::condition_acquisition::{self as acquisition_rule, LeafRoute, Split};
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
    /// Card CG04-CONDITION-ACQUISITION (rule 10x): the per-leaf decision
    /// that replaced the whole-condition type-operator and increment
    /// refusals, and the carrier check of a split acquisition.
    LeafShape(acquisition_rule::Refusal),
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
            Self::LeafShape(refusal) => ("leaf-shape", Some(refusal.code().to_owned()), None),
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

/// The spelling classes of a `__builtin_expect` hint, in the order of
/// `QCore.BuiltinExpectCondition.HintSpelling`: card BUILTIN-EXPECT-NULL-TEST.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HintSpelling {
    /// One token of decimal digits (`0`, `1`).
    Decimal,
    /// One integer-literal token with a hex or octal prefix or an integer
    /// suffix (`0L`, `1UL`, `0x0`, `00`).
    SuffixedOrPrefixed,
    /// An identifier, a member or an element read.
    Name,
    /// A call, an assignment, an increment or decrement.
    Effectful,
    /// Anything else: a cast, an operator expression, a floating literal.
    Other,
}

impl HintSpelling {
    /// `QCore.BuiltinExpectCondition.admitted`: only an integer literal,
    /// which has no evaluation event (C11 6.5.1, 6.4.4.1).
    pub(crate) const fn admitted(self) -> bool {
        matches!(self, Self::Decimal | Self::SuffixedOrPrefixed)
    }

    /// Classify a hint's tokens, outer parentheses already stripped.
    pub(crate) fn of(tokens: &[&str]) -> Self {
        if let [literal] = tokens {
            if !literal.is_empty() && literal.bytes().all(|byte| byte.is_ascii_digit()) {
                return Self::Decimal;
            }
            if is_integer_literal(literal) {
                return Self::SuffixedOrPrefixed;
            }
            if is_identifier(literal) {
                return Self::Name;
            }
            return Self::Other;
        }
        let effect = tokens.windows(2).any(|pair| {
            matches!(pair, ["+", "+"] | ["-", "-"]) || (is_identifier(pair[0]) && pair[1] == "(")
        }) || (0..tokens.len()).any(|index| is_assignment_at(tokens, index));
        if effect {
            return Self::Effectful;
        }
        let read = tokens.first().is_some_and(|first| is_identifier(first))
            && tokens.iter().all(|text| {
                is_identifier(text)
                    || matches!(*text, "." | "[" | "]" | "-" | ">")
                    || text.bytes().all(|byte| byte.is_ascii_alphanumeric())
            });
        if read { Self::Name } else { Self::Other }
    }
}

/// One C integer-literal token (C11 6.4.4.1): a decimal, octal or hex digit
/// sequence and an optional integer suffix.
fn is_integer_literal(text: &str) -> bool {
    let (digits, rest) =
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            let end = hex
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(hex.len());
            (&hex[..end], &hex[end..])
        } else {
            let end = text
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(text.len());
            let digits = &text[..end];
            if digits.len() > 1
                && digits.starts_with('0')
                && !digits.bytes().all(|b| (b'0'..=b'7').contains(&b))
            {
                return false;
            }
            (digits, &text[end..])
        };
    !digits.is_empty()
        && matches!(
            rest,
            "" | "u"
                | "U"
                | "l"
                | "L"
                | "ll"
                | "LL"
                | "ul"
                | "uL"
                | "Ul"
                | "UL"
                | "lu"
                | "lU"
                | "Lu"
                | "LU"
                | "ull"
                | "uLL"
                | "Ull"
                | "ULL"
                | "llu"
                | "llU"
                | "LLu"
                | "LLU"
        )
}

/// Whether a `__builtin_expect` hint is admitted: exactly one integer
/// literal (card BUILTIN-EXPECT-NULL-TEST). Its value is a hint only, and a
/// literal has no evaluation event, so the call has its operand's value and
/// its operand's events.
pub(crate) fn hint_is_integer_literal(tokens: &[&str]) -> bool {
    HintSpelling::of(tokens).admitted()
}

/// Read `texts[start..end]` as a Boolean tree. `Err` is a refusal.
fn parse(texts: &[&str], start: usize, end: usize) -> Result<Tree, Refusal> {
    let (start, end) = strip(texts, start, end);
    if start >= end {
        return Err(Refusal::Unparsed);
    }
    // `__builtin_expect(E, K)` with a literal `K` has the value of `E`; both
    // are evaluated and `K` is a constant, so the call is transparent to the
    // Boolean reading (the `likely`/`unlikely` macros). Card
    // BUILTIN-EXPECT-NULL-TEST (rule 5h amended): `K` is any one integer
    // literal ([`hint_is_integer_literal`]); every other hint refuses.
    if texts[start] == "__builtin_expect"
        && texts.get(start + 1) == Some(&"(")
        && close_of(texts, start + 1) == Some(end - 1)
    {
        let mut depth = 0usize;
        let mut comma = None;
        for (index, text) in texts.iter().enumerate().take(end - 1).skip(start + 2) {
            match *text {
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.saturating_sub(1),
                // The guard records the first top-level comma and refuses a
                // second: `replace` answers the comma it displaced.
                "," if depth == 0 && comma.replace(index).is_some() => {
                    return Err(Refusal::Unparsed);
                }
                _ => {}
            }
        }
        let comma = comma.ok_or(Refusal::Unparsed)?;
        let (constant_start, constant_end) = strip(texts, comma + 1, end - 1);
        if !hint_is_integer_literal(&texts[constant_start..constant_end]) {
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
pub(super) fn is_identity_call(instruction: &LegacyInstruction) -> bool {
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
        // Card W45-FB-RETRY-LIFETIME: a container effect is not the identity.
        && !contract.has_container_effect()
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
                && !call.contract.has_container_effect()
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
        line: usize,
        condition: &[Token],
        position: Position,
        refused: bool,
    ) -> Option<Box<ConditionPlan>> {
        let plan = self.planned_condition_routes(line, condition, position, refused);
        // Card CG02-ELABORATION-CORE (rule 10s): asked last, only where every
        // route above refused.
        if plan.is_none() && refused && !self.opaque_callee(condition) {
            let elaborated = self.elaborated_condition(condition, line, position);
            if elaborated.is_some() {
                return elaborated;
            }
            // Card VA-LOOP-AND-POINTER-ARG (rule 12c): asked after the
            // elaborator, only for an expression holding a `va_arg`.
            return self.va_arg_condition(condition, line, position);
        }
        plan
    }

    fn planned_condition_routes(
        &mut self,
        _line: usize,
        condition: &[Token],
        position: Position,
        refused: bool,
    ) -> Option<Box<ConditionPlan>> {
        // Card W45-PARENTHESISED-CALLEE: a callee that is not a name is never
        // desugared.
        if !refused || self.opaque_callee(condition) {
            return None;
        }
        // Card CG04-CONDITION-ACQUISITION (rule 10x): a type operator's
        // operand is not a call of the condition.
        if !acquisition::holds_real_call(condition) {
            // Census only: a refused call-free expression shares its leaf
            // with any call-bearing one on the same line and position.
            #[cfg(feature = "indirect-census")]
            if !self.admit_derived_returns {
                census(self, _line, position, condition, &Err(Refusal::NoCall));
            }
            // Card W47-ACCESS-PATH-POSITIONS (rule 10k): a refused call-free
            // expression is desugared only when an operand needed the typed
            // access-path reader; otherwise its refusal stays.
            return self.typed_call_free_plan(condition, position);
        }
        let saved = self.probe_state();
        self.condition_widening.set(0);
        let outcome = self.condition_plan(condition, position);
        // A derived-result validation parse re-reads a body the ordinary
        // parse already recorded.
        #[cfg(feature = "indirect-census")]
        if !self.admit_derived_returns {
            census(self, _line, position, condition, &outcome);
        }
        self.record_condition_desugar(
            condition,
            position,
            outcome.as_ref().map_err(|refusal| format!("{refusal:?}")),
            ADMIT,
        );
        match outcome {
            Ok(plan) if ADMIT => Some(Box::new(plan)),
            _ => {
                self.restore_probe_state(&saved);
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
        let saved = self.probe_state();
        let result = self.condition_plan_unguarded(condition, position);
        if result.is_err() {
            self.restore_probe_state(&saved);
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
        // Card CG04-CONDITION-ACQUISITION (rule 10x): a type operator and an
        // increment are decided per leaf (`leaf_plan`); only a type operand
        // whose extent is not read lexically keeps the whole refusal.
        if acquisition::type_operand_spans(&texts).is_none() {
            return Err(Refusal::TypeOperator);
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
            let leaf = &condition[start..end];
            let plan = match self.scalar_accessor_condition(leaf) {
                Some(plan) => plan,
                None => self.leaf_plan(leaf, position)?,
            };
            plans.push(Some(plan));
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

    /// Asked only for a controlling expression after every older whole-
    /// condition route refused. The exact scalar observation retains the
    /// external call and its ordinary checks; it names no tracked base and
    /// refines neither successor. Single-leaf return consumers are unchanged.
    fn scalar_accessor_condition(&mut self, tokens: &[Token]) -> Option<LeafPlan> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let at = crate::scalar_accessor_condition::callee(&texts)?;
        let name = texts[at];
        let contract = self.contracts.contract(name)?.clone();
        if !crate::scalar_accessor_condition::eligible(
            super::find_function_body(self.tokens, name).is_none(),
            self.typed_model.scalar_pointer_result(name),
            contract.parameters.is_empty(),
            contract.result == ResourceReturnMode::None,
            !contract.has_container_effect(),
        ) {
            return None;
        }
        self.probe(|parser| {
            let outcome = parser.instantiate_call(
                name,
                &tokens[at..at + 3],
                None,
                crate::call_instantiation::ResultUse::Tested,
            );
            match outcome {
                Ok(calls) if calls.last().is_some_and(is_identity_call) => {
                    if name == "__errno_location" {
                        use crate::reading_site_evidence::{Capture, Route, Span};
                        crate::reading_site_evidence::capture(Capture {
                            function: &parser.typed_path_function,
                            declaration: name,
                            route: Route::ScalarCondition,
                            full: parser.tokens,
                            statement: tokens,
                            call: Span {
                                start: at,
                                end: at + 3,
                            },
                            outer_argument: None,
                            siblings: Vec::new(),
                            contract: &contract,
                            environment: parser.contracts,
                            checked_predicates: [
                                "exact-zero-argument-scalar-condition",
                                "external-no-local-body",
                                "single-level-scalar-pointer-declaration",
                                "declared-empty-parameters",
                                "declared-no-result-or-container-effect",
                                "ordinary-instantiation-succeeded",
                                "instantiated-identity-call",
                            ]
                            .into_iter()
                            .map(|id| (id.to_owned(), true))
                            .collect(),
                        });
                    }
                    super::probe_state::Probe::Keep(Some(LeafPlan {
                        calls,
                        decision: LegacyInstruction::NoOp,
                    }))
                }
                _ => super::probe_state::Probe::Decline(None),
            }
        })
    }

    /// Card **CG03-CALL-SUMMARY-INSTANTIATION** (rule 10u): the plan of an
    /// expression that is exactly one leaf, read with the same premises as a
    /// controlling expression (braces, type operators, increments, the elvis
    /// operator and any connective refuse). The calls, in evaluation order,
    /// and the leaf's decision; the caller restores the counters on a
    /// refusal. A refusal is its census variant's text.
    pub(super) fn single_leaf_plan(
        &mut self,
        expression: &[Token],
        position: Position,
    ) -> Result<(Vec<LegacyInstruction>, LegacyInstruction), String> {
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let refuse = |refusal: Refusal| format!("{refusal:?}");
        if contains_braces(expression) {
            return Err(refuse(Refusal::Braces));
        }
        if texts.iter().any(|text| TYPE_OPERATORS.contains(text)) {
            return Err(refuse(Refusal::TypeOperator));
        }
        if texts
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        {
            return Err(refuse(Refusal::IncrementOrDecrement));
        }
        if texts.windows(2).any(|pair| pair == ["?", ":"]) {
            return Err(refuse(Refusal::Elvis));
        }
        // A negation evaluates its operand once and decides nothing more.
        let mut tree = parse(&texts, 0, texts.len()).map_err(refuse)?;
        while let Tree::Not(inner) = tree {
            tree = *inner;
        }
        let Tree::Leaf(start, end) = tree else {
            return Err("a connective (CG06)".to_owned());
        };
        let plan = self
            .leaf_plan(&expression[start..end], position)
            .map_err(refuse)?;
        Ok((plan.calls, plan.decision))
    }

    fn leaf_plan(&mut self, tokens: &[Token], position: Position) -> Result<LeafPlan, Refusal> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        // Card CG04-CONDITION-ACQUISITION (rule 10x): the leaf's route, from
        // its features with every type operand set apart.
        let features = acquisition::leaf_features(&texts).ok_or(Refusal::TypeOperator)?;
        let route = acquisition_rule::leaf_route(features).map_err(Refusal::LeafShape)?;
        if texts.iter().any(|text| TYPE_OPERATORS.contains(text)) {
            self.condition_widening
                .set(self.condition_widening.get() | acquisition::WIDENED_TYPE_OPERATOR);
        }
        if route == LeafRoute::CallFree {
            // An increment never reaches the older call-free routes, which
            // the whole-condition refusal kept it from before this card.
            if !features.increment {
                if let Some(plan) = self.call_free_decision(tokens, position) {
                    return Ok(plan);
                }
                // Card W47-ACCESS-PATH-POSITIONS: one `NAME = PATH` operand.
                if let Some((assignment, decision)) =
                    self.typed_assignment_operand(tokens, position)
                {
                    return Ok(LeafPlan {
                        calls: vec![assignment],
                        decision,
                    });
                }
            }
            // Card CG04-CONDITION-ACQUISITION (rule 10x): the elaborator
            // (rule 10s), asked last: the leaf's events, then a branch that
            // refines nothing.
            let events = self
                .elaborated_leaf(tokens)
                .ok_or(Refusal::CallFreeLeafRefused)?;
            self.condition_widening
                .set(self.condition_widening.get() | acquisition::WIDENED_ELABORATED_LEAF);
            return Ok(LeafPlan {
                calls: events,
                decision: LegacyInstruction::NoOp,
            });
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
        // Card CG04-CONDITION-ACQUISITION (rule 10x): the split the leaf
        // makes on the assigned destination, if any. A descriptor
        // destination is admitted only as a split acquisition (below).
        let split = assigned
            .as_ref()
            .and_then(|group| self.acquisition_split(&residual, &group.name));
        let descriptor_destination = assigned
            .as_ref()
            .filter(|group| self.descriptor_names.contains(&group.name));
        if let Some(group) = descriptor_destination
            && split != Some(Split::Sentinel)
        {
            return Err(Refusal::AssignmentDestination(group.name.clone()));
        }
        let mut lowered = Vec::with_capacity(calls.len());
        for call in &calls {
            let callee = texts[call.callee];
            let (statement, destination) = match &assigned {
                Some(group) if call.callee == group.call => (
                    &tokens[group.statement..=call.close],
                    Some(group.name.as_str()),
                ),
                _ => (&tokens[call.callee..=call.close], None),
            };
            // Card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u): the one
            // instantiation. A leaf's call value is tested, so an owned,
            // reallocated or descriptor result is custody and refuses; a
            // derived result owns nothing (discarded, it is a discarded
            // `None`; written to the program's own pointer, that binding
            // tracks it). A callee in the caller's recursive component
            // refuses before the classifier runs. The refusals below are the
            // desugaring's own, unchanged.
            // A call whose actuals the lexical gate refused arrives with its
            // actuals' typed reads before it (card W44-INDIRECT-CALL-
            // ARGUMENT-ACCESS, the statement position's own lowering).
            // Card CG04-CONDITION-ACQUISITION (rule 10x): the value of the
            // assigned call split on its destination is the NULL split use.
            let use_ = acquisition_rule::leaf_use(destination.is_some(), split);
            let sequence = match self.instantiate_call(callee, statement, destination, use_) {
                Ok(sequence) => sequence,
                Err(CallRefusal::Declined(Decline::RecursiveLocal)) => {
                    return Err(Refusal::RecursiveLocalCallee(callee.to_owned()));
                }
                Err(CallRefusal::Declined(_)) => {
                    return Err(Refusal::CustodyResult(callee.to_owned()));
                }
                Err(CallRefusal::UnexpectedDestination) => {
                    return Err(Refusal::CallRefused(
                        callee.to_owned(),
                        "unexpected destination".to_owned(),
                    ));
                }
                Err(CallRefusal::Unapplied(description)) => {
                    return Err(Refusal::CallRefused(
                        callee.to_owned(),
                        census_reason(&description),
                    ));
                }
                Err(CallRefusal::Lowered(other)) => {
                    return Err(Refusal::CallRefused(
                        callee.to_owned(),
                        format!("lowered to {}", variant_name(&other)),
                    ));
                }
            };
            if let (Some(split), Some(_)) = (split, destination)
                && let Some(LegacyInstruction::ContractCall { contract, .. }) = sequence.last()
            {
                let result = crate::call_instantiation::ResultClass::of(contract.result);
                acquisition_rule::carrier(result, contract.result_kind.is_integer(), split)
                    .map_err(Refusal::LeafShape)?;
                acquisition_rule::placement(
                    !result.resource_free(),
                    self.condition_on_cycle(position),
                    crate::loop_generations::SITES_ARE_GENERATION_INDEXED,
                )
                .map_err(Refusal::LeafShape)?;
                // A descriptor destination holds only its own acquisition.
                if descriptor_destination.is_some()
                    && (result.resource_free() || !contract.result_kind.is_integer())
                {
                    return Err(Refusal::AssignmentDestination(
                        destination.unwrap_or(callee).to_owned(),
                    ));
                }
                if !result.resource_free() {
                    self.condition_widening
                        .set(self.condition_widening.get() | acquisition::WIDENED_SPLIT);
                }
            }
            lowered.push(sequence);
        }
        if lowered.len() > 1 {
            // C does not sequence two calls of one operand. Only calls that
            // are the identity on the abstract state commute; with one of
            // them written through, neither order may be chosen. A call's
            // actual reads change no map, so they commute with an identity.
            if assigned.is_some()
                || !lowered
                    .iter()
                    .all(|sequence| sequence.last().is_some_and(is_identity_call))
            {
                return Err(Refusal::UnsequencedEffects);
            }
        }
        let lowered = lowered.into_iter().flatten().collect::<Vec<_>>();
        let residual_texts = residual
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if let Some(name) =
            descriptor_custody::value_carries_descriptor(&residual_texts, &self.descriptor_names)
        {
            // Card CG04-CONDITION-ACQUISITION (rule 10x): a descriptor
            // acquisition's destination is read only by its own validity
            // test, which the split below compiles; any other descriptor
            // name, or this one elsewhere, keeps the refusal.
            let own = split == Some(Split::Sentinel)
                && descriptor_destination.is_some_and(|group| {
                    residual_texts
                        .iter()
                        .all(|text| !self.descriptor_names.contains(text) || *text == group.name)
                });
            if !own {
                return Err(Refusal::DescriptorRead(name.to_owned()));
            }
        }
        let decision = if assigned.is_some() {
            match self.null_condition(&residual) {
                Some(test) if test.validity.is_none() => test.branch(),
                // Card CG04-CONDITION-ACQUISITION (rule 10x): the sentinel
                // split of a descriptor acquisition.
                Some(test) if split == Some(Split::Sentinel) => test.branch(),
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
        } else if let Some(access) = self.condition_access(tokens) {
            *access
        } else {
            // Card W47-ACCESS-PATH-POSITIONS (rule 10k): the typed operand
            // read, asked after every route above refused.
            let read = self.typed_condition_decision(tokens)?;
            // A read refines nothing, so it may also dispatch a switch.
            return Some(LeafPlan {
                calls: Vec::new(),
                decision: read,
            });
        };
        if position == Position::Switch && decision != LegacyInstruction::NoOp {
            return None;
        }
        Some(LeafPlan {
            calls: Vec::new(),
            decision,
        })
    }

    /// Card **W47-ACCESS-PATH-POSITIONS**: the plan of a refused call-free
    /// controlling expression, kept only when at least one operand was
    /// decided by the typed access-path reader. A plan that is not kept
    /// restores the classifier's counters.
    fn typed_call_free_plan(
        &mut self,
        condition: &[Token],
        position: Position,
    ) -> Option<Box<ConditionPlan>> {
        let saved = self.probe_state();
        let uses = self.typed_position_uses.get();
        self.condition_widening.set(0);
        let outcome = self.condition_plan(condition, position);
        let typed = self.typed_position_uses.get() > uses;
        self.typed_position_uses.set(uses);
        // Card CG04-CONDITION-ACQUISITION (rule 10x): a leaf the elaborator
        // decided keeps the plan as a typed operand does.
        let elaborated = self.condition_widening.get() & acquisition::WIDENED_ELABORATED_LEAF != 0;
        if elaborated {
            self.record_condition_desugar(
                condition,
                position,
                outcome.as_ref().map_err(|refusal| format!("{refusal:?}")),
                ADMIT,
            );
        }
        // W48-BOTTOM-UP-BATCH4: old typed readers also carry checked reads.
        // Keep their complete graph, with the read at its original leaf; no
        // leaf admission or transfer obligation changes. An incomplete plan
        // is still refused, and its probe state restored below.
        let checked_read = outcome.as_ref().is_ok_and(|plan| {
            plan.nodes.iter().any(|node| {
                matches!(
                    node.instruction,
                    LegacyInstruction::IndirectScalarAccess { .. }
                )
            })
        });
        match outcome {
            Ok(plan)
                if ADMIT
                    && crate::call_free_condition_reads::keep(typed, elaborated, checked_read) =>
            {
                Some(Box::new(plan))
            }
            _ => {
                self.restore_probe_state(&saved);
                None
            }
        }
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
        // Card CG04-CONDITION-ACQUISITION (rule 10x): or, where the lexical
        // reader does not type the name, one automatic local the Clang AST
        // types (rule 10l's declared-local premise of an assignment operand's
        // destination: a `void *`, a typedef'd pointer).
        let declared = self.position_declared_local(name);
        // Card CG04-CONDITION-ACQUISITION (rule 10x): a descriptor name is
        // decided by `leaf_plan`, which admits it only as the destination of
        // its own split acquisition.
        if !declared || self.formal_parameters.contains(name) {
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
    pub(super) fn recursive_local_callee(&mut self, callee: &str) -> Option<String> {
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
        // Card CG04-CONDITION-ACQUISITION (rule 10x): a type operator's
        // operand is not evaluated; `leaf_route` refused a call inside one.
        if TYPE_OPERATORS.contains(&texts[index]) && texts[index + 1] == "(" {
            index = close_of(texts, index + 1).ok_or(Refusal::Unparsed)? + 1;
            continue;
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
                ResourceReturnMode::Removed { .. } => "removed",
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

    /// Card BUILTIN-EXPECT-NULL-TEST: every integer-literal hint is admitted,
    /// every other hint refuses, and the classes bind
    /// `QCore.BuiltinExpectCondition.vectors` in constructor order.
    #[test]
    fn builtin_expect_hints_are_integer_literals() {
        for hint in [
            "0", "1", "( 0 )", "0L", "1UL", "0x0", "0X1", "00", "1ll", "0LLU",
        ] {
            assert!(
                tree(&format!("__builtin_expect ( ! p , {hint} )")).is_ok(),
                "{hint}"
            );
        }
        for hint in [
            "k",
            "s -> k",
            "a [ 0 ]",
            "f ( )",
            "k ++",
            "k = 1",
            "0 . 5",
            "- 1",
            "( long ) 0",
            "08",
            "0x",
            "1Lu2",
            "0lL",
        ] {
            assert!(
                tree(&format!("__builtin_expect ( ! p , {hint} )")).is_err(),
                "{hint}"
            );
        }
        let representatives: [&[&str]; 5] = [&["0"], &["0L"], &["k"], &["k", "++"], &["-", "1"]];
        let classes = representatives.map(HintSpelling::of);
        assert_eq!(
            classes,
            [
                HintSpelling::Decimal,
                HintSpelling::SuffixedOrPrefixed,
                HintSpelling::Name,
                HintSpelling::Effectful,
                HintSpelling::Other,
            ]
        );
        assert_eq!(
            classes.map(HintSpelling::admitted),
            [true, true, false, false, false]
        );
    }
}
