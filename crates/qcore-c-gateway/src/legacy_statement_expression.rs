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

//! Card **CG07-GENERIC-AND-STATEMENT-EXPRESSIONS** (gateway rule 11g): GNU
//! statement expressions connected to the custody gateway (route G).
//!
//! Every hook here is a **fallback**, consulted only where every older route
//! of its position refused (hard invariant 3); a declined answer leaves the
//! refusal byte for byte (the classifier's site and element counters and the
//! stream cursor are restored). Each consultation is a parity row
//! (`position: "statement-expression-statement"` or `"-return"`).
//!
//! The full expression is matched to one Clang node by its line and exact
//! tokens, and [`crate::statement_expression`] decides its statement
//! expressions: the position, the name hygiene the flat lowering needs, static
//! storage, and, where the value is used, the escape premise and the value.
//! An admitted statement is then lowered in C's order, from the **parser's own
//! stream**, so every inner statement is read exactly as the same statement
//! written in an ordinary block would be (an `if`, a declaration, a call, a
//! nested statement expression through this hook again):
//!
//! * **discarded**: each comma operand in order; a statement expression is its
//!   block, parsed as a block; any other operand is a statement of its own;
//! * **initializer / assigned**: the block's statements before its last, then
//!   `T NAME = e` or `NAME = e`, with `e` the last expression, lowered by the
//!   statement's own routes;
//! * **returned**: the block's statements before its last, then `return e`
//!   through the `return` statement's own chain.
//!
//! An inner statement an older route refuses keeps its own refusal (a leaf
//! naming the construct it holds), exactly as it would in an ordinary block;
//! nothing inner is admitted that an ordinary block would refuse. The route
//! has no block scopes: an inner object's lifetime ends at the block, and the
//! hygiene premise makes the flat names exact (`flat_names_exact`); the escape
//! premise makes sure nothing that outlives the block points into it.

use serde_json::{Value, json};

use super::{Parser, Statement, Token};
use crate::statement_expression::{self as se, Body, Operand, Position, Refusal};
use crate::typed_access_path::{StatementIndex, offset, record_row};
use crate::typed_c_ast::{children, kind};

/// Why route G declines, beside the reading's own refusals.
#[derive(Debug)]
enum Decline {
    Reading(Refusal),
    Node,
    Span,
    Parse,
}

impl Decline {
    fn reason(&self) -> String {
        match self {
            Decline::Reading(refusal) => refusal.reason(),
            Decline::Node => "no unique Clang node".to_owned(),
            Decline::Span => "a statement expression not located in the stream".to_owned(),
            Decline::Parse => "an inner block the statement walker does not parse".to_owned(),
        }
    }
}

impl From<Refusal> for Decline {
    fn from(refusal: Refusal) -> Self {
        Decline::Reading(refusal)
    }
}

/// `( {` spelled somewhere in the run: a statement expression may be there.
fn spells_statement_expression(tokens: &[Token]) -> bool {
    tokens
        .windows(2)
        .any(|pair| pair[0].text == "(" && pair[1].text == "{")
}

/// The family a statement expression belongs to, for the census: glibc's
/// `assert`, a `typeof` temporary, or its first tokens.
fn family(texts: &[&str], span: (usize, usize)) -> String {
    let inner = &texts[span.0..span.1];
    if inner.contains(&"__assert_fail") {
        return "glibc assert".to_owned();
    }
    if inner
        .iter()
        .any(|text| matches!(*text, "__typeof__" | "typeof" | "__typeof"))
    {
        return "typeof temporary".to_owned();
    }
    inner
        .iter()
        .skip(2)
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A node's token span within the run starting at byte `begin`, when its
/// tokens are exactly that run's (card CG06's reading, rule 10z).
fn locate(
    index: &StatementIndex,
    texts: &[&str],
    begin: usize,
    node: &Value,
) -> Option<(usize, usize)> {
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

/// What one admitted statement lowers to.
enum Plan<'a> {
    /// Each operand in order.
    Discarded(Vec<Operand<'a>>),
    /// The statement expression that is the whole value, and the span of
    /// that value in the run.
    Value {
        position: Position,
        block: &'a Value,
        value_span: (usize, usize),
    },
}

impl Parser<'_> {
    /// Hook: a statement every older route refused (`old` its refusal),
    /// whose run starts at stream index `origin`. `None` keeps the refusal.
    pub(super) fn statement_expression_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        origin: Option<usize>,
        old: &str,
    ) -> Option<Statement> {
        let origin = origin?;
        if tokens.is_empty()
            || !spells_statement_expression(tokens)
            || !self.in_stream(tokens, origin)
        {
            return None;
        }
        self.statement_expression(false, tokens, line, origin, old)
    }

    /// Hook: a `return` every older route refused, whose expression starts at
    /// stream index `origin`. `None` keeps the refusal.
    pub(super) fn statement_expression_return(
        &mut self,
        expression: &[Token],
        line: usize,
        origin: usize,
    ) -> Option<Statement> {
        if expression.is_empty()
            || !spells_statement_expression(expression)
            || !self.in_stream(expression, origin)
        {
            return None;
        }
        self.statement_expression(
            true,
            expression,
            line,
            origin,
            "unmodeled call, aggregate or indirect access in return expression",
        )
    }

    /// Whether `tokens` are the stream's own tokens at `origin`.
    fn in_stream(&self, tokens: &[Token], origin: usize) -> bool {
        self.tokens
            .get(origin..origin + tokens.len())
            .is_some_and(|stream| {
                stream
                    .iter()
                    .zip(tokens)
                    .all(|(left, right)| left.text == right.text && left.line == right.line)
            })
    }

    fn statement_expression(
        &mut self,
        returned: bool,
        tokens: &[Token],
        line: usize,
        origin: usize,
        old: &str,
    ) -> Option<Statement> {
        let index = self.typed_positions_index()?.clone();
        let saved = (self.next_site, self.element_lowerings, self.cursor);
        let mut placements = Vec::new();
        let outcome = self.statement_expression_lowering(
            &index,
            returned,
            tokens,
            line,
            origin,
            &mut placements,
        );
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": if returned { "statement-expression-return" } else { "statement-expression-statement" },
            "old_route": "old_refusal",
            "old_refusal": old,
            "old_admitted": false,
            "statement_expressions": placements,
            "new_admitted": outcome.is_ok(),
            "new_refusal": outcome.as_ref().err().map(Decline::reason),
            "disagreement": outcome.is_ok(),
        }));
        match outcome {
            Ok(statement) => {
                self.cursor = saved.2;
                Some(statement)
            }
            Err(_) => {
                (self.next_site, self.element_lowerings, self.cursor) = saved;
                None
            }
        }
    }

    fn statement_expression_lowering(
        &mut self,
        index: &StatementIndex,
        returned: bool,
        tokens: &[Token],
        line: usize,
        origin: usize,
        placements: &mut Vec<Value>,
    ) -> Result<Statement, Decline> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (root, begin) = if returned {
            let mut wanted = vec!["return"];
            wanted.extend(texts.iter().copied());
            let statement = Self::elaboration_node(index, tokens[0].line, &wanted, &|node| {
                kind(node) == "ReturnStmt"
            })
            .ok_or(Decline::Node)?;
            let [value] = children(statement) else {
                return Err(Decline::Node);
            };
            (
                value,
                offset(&value["range"]["begin"]).ok_or(Decline::Node)?,
            )
        } else {
            let node = Self::elaboration_node(index, tokens[0].line, &texts, &|node| {
                !kind(node).ends_with("Stmt") || kind(node) == "DeclStmt"
            })
            .ok_or(Decline::Node)?;
            (node, offset(&node["range"]["begin"]).ok_or(Decline::Node)?)
        };
        let locate = |node: &Value| locate(index, &texts, begin, node);
        let body = Body::read(&index.function);
        let mut place = |block: &Value, position: &str, outcome: Result<(), &Refusal>| {
            let span = locate(block);
            placements.push(json!({
                "position": position,
                "family": span.map(|span| family(&texts, span)),
                "admitted": outcome.is_ok(),
                "refusal": outcome.err().map(Refusal::reason),
            }));
        };
        // The plan: which position each statement expression stands in, and
        // an assignment's named destination.
        let mut destination: Option<String> = None;
        let plan = if returned {
            match se::whole_value(root) {
                Some(block) => Plan::Value {
                    position: Position::Returned,
                    block,
                    value_span: locate(root).ok_or(Decline::Span)?,
                },
                None => {
                    let refusal = Refusal::Position(se::position_of(root));
                    return Err(refusal.into());
                }
            }
        } else if kind(root) == "DeclStmt" {
            let [declaration] = children(root) else {
                return Err(Refusal::Position("operand").into());
            };
            let [initializer] = children(declaration) else {
                return Err(Refusal::Position("operand").into());
            };
            match se::whole_value(initializer) {
                Some(block) if kind(declaration) == "VarDecl" => Plan::Value {
                    position: Position::Initializer,
                    block,
                    value_span: locate(initializer).ok_or(Decline::Span)?,
                },
                _ => return Err(Refusal::Position(se::position_of(initializer)).into()),
            }
        } else {
            let bare = se::strip_transparent(root);
            if kind(bare) == "BinaryOperator"
                && bare["opcode"].as_str() == Some("=")
                && let [left, right] = children(bare)
                && se::holds(right)
            {
                match se::whole_value(right) {
                    Some(block) if !se::holds(left) && se::named_destination(left).is_some() => {
                        destination = se::named_destination(left).map(str::to_owned);
                        Plan::Value {
                            position: Position::Assigned,
                            block,
                            value_span: locate(right).ok_or(Decline::Span)?,
                        }
                    }
                    Some(_) => return Err(Refusal::Position("assigned path").into()),
                    None => return Err(Refusal::Position(se::position_of(right)).into()),
                }
            } else {
                Plan::Discarded(se::discarded_operands(root)?)
            }
        };
        // Decide every statement expression before anything is lowered.
        match &plan {
            Plan::Discarded(operands) => {
                for operand in operands {
                    if let Operand::Block(block) = operand {
                        let outcome = se::decide(&body, block, Position::Discarded);
                        place(
                            block,
                            Position::Discarded.label(),
                            outcome.as_ref().map(|_| ()),
                        );
                        outcome?;
                    }
                }
            }
            Plan::Value {
                position, block, ..
            } => {
                let outcome = se::decide(&body, block, *position);
                place(block, position.label(), outcome.as_ref().map(|_| ()));
                outcome?;
            }
        }
        // Lower, in C's order, from the stream.
        let mut statements = Vec::new();
        match plan {
            Plan::Discarded(operands) => {
                // Every span is located before anything is lowered, so a
                // decline never follows a partly parsed block.
                let mut spans = Vec::with_capacity(operands.len());
                for operand in &operands {
                    spans.push(match operand {
                        Operand::Block(block) => (true, self.block_span(&texts, &locate, block)?),
                        Operand::Plain(node) => (false, locate(node).ok_or(Decline::Span)?),
                    });
                }
                for (block, (first, last)) in spans {
                    if block {
                        statements.push(Statement::Block(
                            self.inner_block(origin + first + 1, origin + last - 2)?,
                        ));
                    } else if first < last {
                        statements.push(self.expression_statement(
                            &tokens[first..last],
                            line,
                            None,
                        ));
                    }
                }
            }
            Plan::Value {
                position,
                block,
                value_span,
            } => {
                let (first, last) = self.block_span(&texts, &locate, block)?;
                let items = se::items(block)?;
                let value = items.last().ok_or(Decline::Span)?;
                let (value_first, value_last) = locate(value).ok_or(Decline::Span)?;
                // The block's value lies inside it; an initializer or a
                // returned value is the rest of the run (an assignment's
                // destination is re-spelled below).
                if value_first <= first + 1
                    || value_last > last - 2
                    || (position != Position::Assigned && value_span.1 != texts.len())
                {
                    return Err(Decline::Span);
                }
                statements.extend(self.inner_items(origin + first + 2, origin + value_first)?);
                let value_tokens = &tokens[value_first..value_last];
                statements.push(match position {
                    Position::Returned => self.statement_expression_returned(
                        value_tokens.to_vec(),
                        line,
                        origin + value_first,
                    ),
                    Position::Assigned => {
                        // `NAME = e`: the destination is the named object the
                        // left operand designates.
                        let name = destination.clone().ok_or(Decline::Node)?;
                        let at = value_tokens.first().map_or(line, |token| token.line);
                        let mut run = vec![
                            Token {
                                text: name,
                                line: at,
                            },
                            Token {
                                text: "=".to_owned(),
                                line: at,
                            },
                        ];
                        run.extend_from_slice(value_tokens);
                        self.expression_statement(&run, line, None)
                    }
                    _ => {
                        let mut run = tokens[..value_span.0].to_vec();
                        run.extend_from_slice(value_tokens);
                        self.expression_statement(&run, line, None)
                    }
                });
            }
        }
        Ok(Statement::Block(statements))
    }

    /// A statement expression's `( { … } )` span in the run.
    fn block_span(
        &self,
        texts: &[&str],
        locate: &dyn Fn(&Value) -> Option<(usize, usize)>,
        block: &Value,
    ) -> Result<(usize, usize), Decline> {
        let (first, last) = locate(block).ok_or(Decline::Span)?;
        (last >= first + 4
            && texts[first] == "("
            && texts[first + 1] == "{"
            && texts[last - 2] == "}"
            && texts[last - 1] == ")")
            .then_some((first, last))
            .ok_or(Decline::Span)
    }

    /// The block whose `{` is at stream index `open` and whose `}` is at
    /// `close`, parsed by the statement walker as an ordinary block.
    fn inner_block(&mut self, open: usize, close: usize) -> Result<Vec<Statement>, Decline> {
        let saved = self.cursor;
        self.cursor = open;
        let parsed = self.parse_block();
        let ended = self.cursor;
        self.cursor = saved;
        match parsed {
            Ok(statements) if ended == close + 1 => Ok(statements),
            _ => Err(Decline::Parse),
        }
    }

    /// The statements from stream index `start` up to (not including) `end`.
    fn inner_items(&mut self, start: usize, end: usize) -> Result<Vec<Statement>, Decline> {
        let saved = self.cursor;
        self.cursor = start;
        let mut statements = Vec::new();
        while self.cursor < end {
            match self.parse_statement() {
                Ok(statement) => statements.push(statement),
                Err(_) => {
                    self.cursor = saved;
                    return Err(Decline::Parse);
                }
            }
        }
        let ended = self.cursor;
        self.cursor = saved;
        (ended == end).then_some(statements).ok_or(Decline::Parse)
    }

    /// `return e`, through the `return` statement's own chain in
    /// `parse_statement`'s order, `e` at stream index `origin`.
    fn statement_expression_returned(
        &mut self,
        expression: Vec<Token>,
        line: usize,
        origin: usize,
    ) -> Statement {
        if let Some(tested) = self.pointer_test_return(&expression, line) {
            return tested;
        }
        let statement = self.return_statement(expression.clone(), line);
        if !super::return_refused(&statement) {
            return statement;
        }
        if let Some(elaborated) = self.elaborated_return(&expression, line) {
            return elaborated;
        }
        if let Some(instantiated) = self.instantiated_return(&expression, line) {
            return instantiated;
        }
        if let Some(nested) = self.nested_call_return(&expression, line, &statement) {
            return nested;
        }
        if let Some(branched) = self.connective_return(&expression, line) {
            return branched;
        }
        if let Some(inner) = self.statement_expression_return(&expression, line, origin) {
            return inner;
        }
        statement
    }
}
