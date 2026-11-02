//! Card **CG06-VALUE-CONNECTIVES-AND-CONDITIONAL** (gateway rule 10z): value
//! `&&`, `||`, `?:` and GNU `?:` connected to the custody gateway (route G).
//!
//! Every hook here is a **fallback**, consulted only where every older route
//! of its position refused (hard invariant 3), and a declined answer leaves the
//! refusal byte for byte (the classifier's site and element counters are
//! restored). Each consultation is a parity row (`position:
//! "value-connective-statement"` or `"value-connective-return"`).
//!
//! * **Statements** (`expression_statement`'s last step) and **returns**
//!   (`return`'s last step). The full expression is matched to one Clang node
//!   by its line and exact tokens, and its value regions are read by
//!   [`crate::value_connective::regions`] with their order premise. The
//!   expression is then lowered by **branching**: the first region's deciding
//!   operand is an `if` condition lowered by the `if` statement's own routes
//!   (null, typed and call-bearing conditions, the condition CFG of rule 5h
//!   for a connective), and each edge carries the expression with the region
//!   replaced by the arm C evaluates on that edge (`1` / `0` for a
//!   connective, `X` again for `X ?: B`), lowered by the position's older
//!   routes as a statement or `return` of its own. Only the selected arm's
//!   events occur on an edge, and the flow joins the two edges' alias and
//!   custody states at the `if`'s join.
//! * Every leaf and every condition must be decided by an older route; one
//!   refusal declines the whole expression.
//!
//! Positions the card does not own: a connective in a controlling expression
//! is the condition CFG's (rule 5h), never read here.

use std::cell::Cell;

use serde_json::{Value, json};

use super::condition_desugar::Position;
use super::{LegacyInstruction, Parser, Statement, Token, contains_braces, is_identifier};
use crate::neutral_position::ExpressionRole;
use crate::typed_access_path::{StatementIndex, offset, record_row};
use crate::typed_c_ast::{children, kind};
use crate::value_connective::{self, Arm, Choice, Reading, Refusal, Region, Shape, Span};

thread_local! {
    /// Set while a leaf is lowered: a leaf is an older route's statement, and
    /// its regions were all substituted by the enclosing lowering.
    static LOWERING: Cell<bool> = const { Cell::new(false) };
}

/// Why route G declines, beside the region reading's own refusals.
#[derive(Debug)]
enum Decline {
    Reading(Refusal),
    Node,
    Condition,
    Leaf,
}

impl Decline {
    fn reason(&self) -> String {
        match self {
            Decline::Reading(refusal) => refusal.reason(),
            Decline::Node => "no unique Clang node".to_owned(),
            Decline::Condition => "a deciding operand no condition route decides".to_owned(),
            Decline::Leaf => "an arm's expression an older route refuses".to_owned(),
        }
    }
}

/// One piece of a full expression being lowered: tokens, or a region still to
/// branch on.
#[derive(Clone)]
enum Piece<'r, 'a> {
    Text(Vec<Token>),
    Hole(&'r Region<'a>),
}

/// The pieces of `tokens[span]` with `regions` (inside `span`, in order) as
/// holes.
fn pieces<'r, 'a>(tokens: &[Token], span: Span, regions: &'r [Region<'a>]) -> Vec<Piece<'r, 'a>> {
    let mut out = Vec::new();
    let mut at = span.0;
    for region in regions {
        if region.span.0 > at {
            out.push(Piece::Text(tokens[at..region.span.0].to_vec()));
        }
        out.push(Piece::Hole(region));
        at = region.span.1;
    }
    if span.1 > at {
        out.push(Piece::Text(tokens[at..span.1].to_vec()));
    }
    out
}

/// A connective or conditional is spelled somewhere in the run.
fn spells_connective(tokens: &[Token]) -> bool {
    tokens.iter().any(|token| token.text == "?")
        || tokens.windows(2).any(|pair| {
            (pair[0].text == "&" && pair[1].text == "&")
                || (pair[0].text == "|" && pair[1].text == "|")
        })
}

/// The position a full expression stands in.
#[derive(Clone, Copy)]
enum Site {
    Statement,
    Return,
}

impl Site {
    fn label(self) -> &'static str {
        match self {
            Site::Statement => "value-connective-statement",
            Site::Return => "value-connective-return",
        }
    }
}

/// The parts of an `if` condition, decided by the `if` statement's routes.
struct Decision {
    writes: super::RefusedWrites,
    condition_supported: bool,
    condition_call: Option<Box<LegacyInstruction>>,
    condition_access: Option<Box<LegacyInstruction>>,
    null_condition: Option<super::PointerNullCondition>,
    distinct_condition: Option<super::PointerDistinctCondition>,
    element_condition: Option<(String, crate::indexed_collection::IndexSpelling, bool)>,
    condition_plan: Option<Box<super::condition_desugar::ConditionPlan>>,
}

impl Parser<'_> {
    /// Hook: an expression or declaration statement every older route
    /// refused (`old` its refusal). `None` keeps the refusal.
    pub(super) fn connective_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        old: &str,
    ) -> Option<Statement> {
        self.connective(Site::Statement, tokens, line, old)
    }

    /// Hook: a `return` every older route refused. `None` keeps the refusal.
    pub(super) fn connective_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        self.connective(
            Site::Return,
            expression,
            line,
            "unmodeled call, aggregate or indirect access in return expression",
        )
    }

    fn connective(
        &mut self,
        site: Site,
        tokens: &[Token],
        line: usize,
        old: &str,
    ) -> Option<Statement> {
        if LOWERING.get()
            || tokens.is_empty()
            || !spells_connective(tokens)
            || contains_braces(tokens)
            || self.opaque_callee(tokens)
            || tokens.iter().any(|token| {
                is_identifier(&token.text) && self.descriptor_names.contains(&token.text)
            })
        {
            return None;
        }
        let index = self.typed_positions_index()?.clone();
        let saved = self.probe_state();
        let mut forms = Vec::new();
        let outcome = self.connective_lowering(&index, site, tokens, line, &mut forms);
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": site.label(),
            "old_route": "old_refusal",
            "old_refusal": old,
            "old_admitted": false,
            "regions": forms,
            "new_admitted": outcome.is_ok(),
            "new_refusal": outcome.as_ref().err().map(Decline::reason),
            "disagreement": outcome.is_ok(),
        }));
        match outcome {
            Ok(statement) => Some(statement),
            Err(_) => {
                self.restore_probe_state(&saved);
                None
            }
        }
    }

    fn connective_lowering(
        &mut self,
        index: &StatementIndex,
        site: Site,
        tokens: &[Token],
        line: usize,
        forms: &mut Vec<&'static str>,
    ) -> Result<Statement, Decline> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let root = match site {
            Site::Statement => Self::elaboration_node(index, tokens[0].line, &texts, &|node| {
                !kind(node).ends_with("Stmt") || kind(node) == "DeclStmt"
            })
            .ok_or(Decline::Node)?,
            Site::Return => {
                let mut wanted = vec!["return"];
                wanted.extend(texts.iter().copied());
                let statement = Self::elaboration_node(index, tokens[0].line, &wanted, &|node| {
                    kind(node) == "ReturnStmt"
                })
                .ok_or(Decline::Node)?;
                let [value] = children(statement) else {
                    return Err(Decline::Node);
                };
                value
            }
        };
        let aliasing = aliasing(&index.function);
        let unaliased = |decl: &str| aliasing.0.contains(decl) && !aliasing.1.contains(decl);
        let begin = offset(&root["range"]["begin"]).ok_or(Decline::Node)?;
        let locate = |node: &Value| locate(index, &texts, begin, node);
        // Rule 11m: a sibling's or the deciding operand's call is composed
        // by CG05's reading and footprints over this unit's contracts.
        let facts = self.nested_facts(index);
        let compose = |region: &Value, siblings: &[&Value]| {
            crate::connective_footprints::compose(region, siblings, &facts)
        };
        let regions = value_connective::regions(
            root,
            &Reading {
                locate: &locate,
                unaliased: &unaliased,
                compose: Some(&compose),
            },
        )
        .map_err(Decline::Reading)?;
        collect_forms(&regions, forms);
        let whole = pieces(tokens, (0, tokens.len()), &regions);
        let lowering = Lowering {
            site,
            tokens,
            root,
            index,
            line,
        };
        self.branch(&lowering, whole, &mut Vec::new())
    }

    /// Lower `pieces`: branch on the first hole, or hand the tokens to the
    /// position's older routes.
    fn branch<'r, 'a>(
        &mut self,
        lowering: &Lowering<'_, 'a>,
        pieces_: Vec<Piece<'r, 'a>>,
        choices: &mut Vec<(&'a Value, Choice<'a>)>,
    ) -> Result<Statement, Decline> {
        let (tokens, line) = (lowering.tokens, lowering.line);
        let Some(at) = pieces_
            .iter()
            .position(|piece| matches!(piece, Piece::Hole(_)))
        else {
            let run = pieces_
                .into_iter()
                .flat_map(|piece| match piece {
                    Piece::Text(tokens) => tokens,
                    Piece::Hole(_) => Vec::new(),
                })
                .collect::<Vec<_>>();
            return self.leaf(lowering, run, choices);
        };
        let Piece::Hole(region) = pieces_[at] else {
            unreachable!("a hole")
        };
        let literal = |text: &str| {
            let at_line = tokens[region.span.0].line;
            vec![Piece::Text(vec![Token {
                text: text.to_owned(),
                line: at_line,
            }])]
        };
        let arm = |arm: &'r Arm<'a>| pieces(tokens, arm.span, &arm.regions);
        let (condition, yes, no, choice) = match &region.shape {
            Shape::Boolean => (
                region.span,
                literal("1"),
                literal("0"),
                [Choice::Literal("1"), Choice::Literal("0")],
            ),
            Shape::Select { condition, arms } => (
                *condition,
                arm(&arms[0]),
                arm(&arms[1]),
                [Choice::Node(arms[0].node), Choice::Node(arms[1].node)],
            ),
            Shape::OmittedMiddle {
                middle,
                middle_node,
                otherwise,
            } => (
                *middle,
                vec![Piece::Text(tokens[middle.0..middle.1].to_vec())],
                arm(otherwise),
                [Choice::Node(middle_node), Choice::Node(otherwise.node)],
            ),
        };
        let edge = |arm: Vec<Piece<'r, 'a>>| {
            let mut edge = pieces_[..at].to_vec();
            edge.extend(arm);
            edge.extend(pieces_[at + 1..].iter().cloned());
            edge
        };
        let (yes, no) = (edge(yes), edge(no));
        // C's order: the deciding operand, then the arm of the edge taken.
        let deciding = match &region.shape {
            Shape::Boolean => region.node,
            _ => children(region.node).first().ok_or(Decline::Node)?,
        };
        let decision = value_connective::deciding(deciding, || {
            self.decision(line, &tokens[condition.0..condition.1])
        })?;
        choices.push((region.node, choice[0]));
        let then_body = self.branch(lowering, yes, choices);
        choices.pop();
        let then_body = then_body?;
        choices.push((region.node, choice[1]));
        let else_body = self.branch(lowering, no, choices);
        choices.pop();
        let else_body = else_body?;
        Ok(Statement::If {
            line,
            writes: decision.writes,
            condition_supported: decision.condition_supported,
            condition_call: decision.condition_call,
            condition_access: decision.condition_access,
            null_condition: decision.null_condition,
            distinct_condition: decision.distinct_condition,
            element_condition: decision.element_condition,
            condition_plan: decision.condition_plan,
            cell_test: None,
            then_body: vec![then_body],
            else_body: vec![else_body],
        })
    }

    /// The `if` statement's own routes over one deciding operand, in the
    /// order `parse_statement` asks them; `Err` when none decides it.
    fn decision(&mut self, line: usize, condition: &[Token]) -> Result<Decision, Decline> {
        let condition_supported = self.supported_expression(condition, ExpressionRole::Decides);
        let condition_call = (!condition_supported)
            .then(|| self.condition_call_instruction(condition))
            .flatten()
            .map(Box::new);
        let condition_access = (!condition_supported && condition_call.is_none())
            .then(|| self.condition_access(condition))
            .flatten();
        let null_condition = self.null_condition(condition);
        let element_condition = (!condition_supported
            && condition_call.is_none()
            && condition_access.is_none()
            && null_condition.is_none())
        .then(|| self.element_condition(condition))
        .flatten();
        if element_condition.is_some() {
            self.element_lowerings += 1;
        }
        let distinct_condition = self.distinct_condition(condition);
        let condition_plan = self.planned_condition(
            line,
            condition,
            Position::Branch,
            !condition_supported
                && condition_call.is_none()
                && condition_access.is_none()
                && element_condition.is_none()
                && null_condition.is_none()
                && distinct_condition.is_none(),
        );
        let decided = condition_supported
            || condition_call.is_some()
            || condition_access.is_some()
            || element_condition.is_some()
            || null_condition.is_some()
            || distinct_condition.is_some()
            || condition_plan.is_some();
        if !decided {
            return Err(Decline::Condition);
        }
        Ok(Decision {
            writes: self.expression_writes(condition),
            condition_supported,
            condition_call,
            condition_access,
            null_condition,
            distinct_condition,
            element_condition,
            condition_plan,
        })
    }

    /// One edge's expression, lowered by the position's older routes. A
    /// statement they refuse is then read from its Clang node with the
    /// edge's choices substituted, and lowered from its elaborated events as
    /// CG02 lowers a refused piece (rule 10s).
    fn leaf<'a>(
        &mut self,
        lowering: &Lowering<'_, 'a>,
        run: Vec<Token>,
        choices: &[(&'a Value, Choice<'a>)],
    ) -> Result<Statement, Decline> {
        let edge = choices
            .iter()
            .map(|(region, choice)| (*region, value_connective::chosen_child(region, *choice)))
            .collect::<Vec<_>>();
        // Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): the flag
        // is restored to its previous value on every exit, an unwind included.
        struct Restore(bool);
        impl Drop for Restore {
            fn drop(&mut self) {
                LOWERING.set(self.0);
            }
        }
        let _restore = Restore(LOWERING.replace(true));
        // The edge's own full expression, for rule 10y's consumers to read
        // where no source node carries the edge's tokens (rule 11m).
        let node = value_connective::substitute(lowering.root, choices);
        let root = node.clone();
        let lowered = value_connective::on_edge(&edge, || {
            value_connective::with_edge_root(root, || self.edge(lowering, &run, &node))
        });
        lowered.ok_or(Decline::Leaf)
    }

    /// One edge's statement or `return`, by the position's older routes.
    fn edge(
        &mut self,
        lowering: &Lowering<'_, '_>,
        run: &[Token],
        node: &Value,
    ) -> Option<Statement> {
        let line = lowering.line;
        let run = run.to_vec();
        match lowering.site {
            Site::Statement => {
                let saved = self.probe_state();
                let statement = self.expression_statement(&run, line, None);
                if !refuses(&statement) {
                    Some(statement)
                } else {
                    self.restore_probe_state(&saved);
                    self.leaf_events(lowering.index, node)
                        .ok()
                        .map(|instructions| {
                            Statement::Block(
                                instructions
                                    .into_iter()
                                    .map(|instruction| Statement::Instruction(instruction, line))
                                    .collect(),
                            )
                        })
                }
            }
            Site::Return => {
                // The `return` statement's own chain, in `parse_statement`'s
                // order (CG02, CG03, then CG05's nested-call return).
                let statement = self.return_statement(run.clone(), line);
                let refused = super::return_refused(&statement);
                let lowered = if let Some(elaborated) = refused
                    .then(|| self.elaborated_return(&run, line))
                    .flatten()
                {
                    Some(elaborated)
                } else if let Some(instantiated) = refused
                    .then(|| self.instantiated_return(&run, line))
                    .flatten()
                {
                    Some(instantiated)
                } else if let Some(nested) = self.nested_call_return(&run, line, &statement) {
                    Some(nested)
                } else {
                    (!refused).then_some(statement)
                };
                // An edge lowered to an explicit refusal (a refused call
                // kept as its own `Unknown` step) declines too.
                lowered.filter(|statement| !refuses(statement))
            }
        }
    }
}

/// One full expression's lowering.
struct Lowering<'t, 'a> {
    site: Site,
    tokens: &'t [Token],
    root: &'a Value,
    index: &'a StatementIndex,
    line: usize,
}

fn collect_forms(regions: &[Region<'_>], forms: &mut Vec<&'static str>) {
    for region in regions {
        forms.push(region.form());
        match &region.shape {
            Shape::Boolean => {}
            Shape::Select { arms, .. } => {
                for arm in arms.iter() {
                    collect_forms(&arm.regions, forms);
                }
            }
            Shape::OmittedMiddle { otherwise, .. } => collect_forms(&otherwise.regions, forms),
        }
    }
}

/// A node's token span within the full expression's run starting at byte
/// `begin`, when its tokens are exactly that run's.
fn locate(index: &StatementIndex, texts: &[&str], begin: usize, node: &Value) -> Option<Span> {
    let start = offset(&node["range"]["begin"])?;
    let prefix = index.source.get(begin..start)?;
    let first = super::tokenize(prefix).len();
    let own = index.node_tokens(node)?;
    let last = first.checked_add(own.len())?;
    (last <= texts.len()
        && texts[first..last]
            .iter()
            .copied()
            .eq(own.iter().map(String::as_str)))
    .then_some((first, last))
}

/// The function's automatic objects and the objects whose address a `&`
/// takes (CG02's aliasing reading).
fn aliasing(
    function: &Value,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
) {
    fn walk(
        node: &Value,
        automatic: &mut std::collections::BTreeSet<String>,
        taken: &mut std::collections::BTreeSet<String>,
    ) {
        match kind(node) {
            "ParmVarDecl" | "VarDecl" if node["storageClass"].as_str().is_none() => {
                if let Some(id) = node["id"].as_str() {
                    automatic.insert(id.to_owned());
                }
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                if let Some(operand) = children(node).first() {
                    let operand = crate::elaboration::strip_parens(operand);
                    if kind(operand) == "DeclRefExpr"
                        && let Some(id) = operand["referencedDecl"]["id"].as_str()
                    {
                        taken.insert(id.to_owned());
                    }
                }
            }
            _ => {}
        }
        for child in children(node) {
            walk(child, automatic, taken);
        }
    }
    let mut automatic = std::collections::BTreeSet::new();
    let mut taken = std::collections::BTreeSet::new();
    walk(function, &mut automatic, &mut taken);
    (automatic, taken)
}

/// Whether a lowered statement is a refusal (anywhere inside a block).
fn refuses(statement: &Statement) -> bool {
    match statement {
        Statement::Instruction(LegacyInstruction::Unknown { .. }, _) => true,
        Statement::Return {
            returned_evaluation: Some(evaluation),
            ..
        } => matches!(**evaluation, LegacyInstruction::Unknown { .. }),
        Statement::Block(statements) => statements.iter().any(refuses),
        _ => false,
    }
}
