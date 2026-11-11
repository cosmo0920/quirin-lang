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

//! Result-match syntax and arm-local names. Pattern recognition does not prove
//! exhaustiveness, result typing, task custody, or executable support.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultPatternKind {
    Ok,
    Err,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultPattern {
    pub kind: ResultPatternKind,
    pub binding: Located<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultMatchArm {
    pub pattern_span: Span,
    pub pattern_source: String,
    pub pattern_tokens: Vec<TokenKind>,
    /// Only the exact builtin record pattern is recognized. A missing value
    /// retains the full pattern for a downstream consumer to reject or support.
    pub pattern: Option<SourceResultPattern>,
    pub body: SourceBlock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultMatch {
    pub span: Span,
    pub scrutinee_span: Span,
    pub scrutinee_source: String,
    pub scrutinee_tokens: Vec<TokenKind>,
    pub arms: Vec<SourceResultMatchArm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedResultMatchScrutinee {
    Task(Box<ResolvedTaskExpression>),
    Value(SymbolId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedResultMatchArm {
    pub syntax: SourceResultMatchArm,
    pub binding: Option<SymbolId>,
    /// Range into the enclosing `TypedStatement::nested`, which owns each
    /// resolved child exactly once. Consumers must validate the range.
    pub statement_range: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedResultMatch {
    pub syntax: SourceResultMatch,
    /// Name resolution only. Compound or propagated scrutinees are retained
    /// in syntax but have no executable result-match classification here.
    pub scrutinee: Option<ResolvedResultMatchScrutinee>,
    pub arms: Vec<ResolvedResultMatchArm>,
}

pub(super) fn parse_inventory(
    parser: &SourceParser<'_>,
    declarations: &[Located<SourceDeclaration>],
) -> Result<Vec<SourceResultMatch>, SourceParseError> {
    fn block(
        parser: &SourceParser<'_>,
        source: &SourceBlock,
        output: &mut Vec<SourceResultMatch>,
    ) -> Result<(), SourceParseError> {
        for statement in &source.statements {
            if matches!(statement.value, StatementKind::Match { .. }) {
                let start = parser
                    .tokens
                    .iter()
                    .position(|token| token.span.start == statement.span.start)
                    .ok_or_else(|| {
                        SourceParser::error_at(statement.span, "missing match tokens")
                    })?;
                let limit = parser
                    .tokens
                    .iter()
                    .position(|token| token.span.start >= statement.span.end)
                    .unwrap_or(parser.tokens.len());
                let open = parser.last_top_level_block(start, limit)?;
                let close = parser.close(open)?;
                if open <= start + 1 {
                    return Err(parser.error(start, "match requires a scrutinee expression"));
                }
                let scrutinee_span = parser.range_span(start + 1, open);
                let mut arms = Vec::new();
                let mut cursor = open + 1;
                while cursor < close {
                    let arrow = parser
                        .top_level_symbol(cursor, close, Symbol::FatArrow)
                        .ok_or_else(|| parser.error(cursor, "match arm requires =>"))?;
                    if arrow == cursor {
                        return Err(parser.error(cursor, "match arm requires a pattern"));
                    }
                    let pattern_span = parser.range_span(cursor, arrow);
                    let pattern_tokens = parser.tokens[cursor..arrow]
                        .iter()
                        .map(|t| t.kind.clone())
                        .collect();
                    let pattern = canonical_pattern(parser, cursor, arrow);
                    let body = parser.block(arrow + 1)?;
                    arms.push(SourceResultMatchArm {
                        pattern_span,
                        pattern_source: parser.source[pattern_span.start..pattern_span.end]
                            .to_owned(),
                        pattern_tokens,
                        pattern,
                        body,
                    });
                    cursor = parser.close(arrow + 1)? + 1;
                }
                output.push(SourceResultMatch {
                    span: statement.span,
                    scrutinee_span,
                    scrutinee_source: parser.source[scrutinee_span.start..scrutinee_span.end]
                        .to_owned(),
                    scrutinee_tokens: parser.tokens[start + 1..open]
                        .iter()
                        .map(|t| t.kind.clone())
                        .collect(),
                    arms,
                });
            }
            for nested in statement.value.nested_blocks() {
                block(parser, nested, output)?;
            }
        }
        Ok(())
    }
    let mut result = Vec::new();
    for declaration in declarations {
        if let Some(body) = &declaration.value.body {
            block(parser, body, &mut result)?;
        }
    }
    result.sort_by_key(|syntax| syntax.span.start);
    Ok(result)
}

fn canonical_pattern(
    parser: &SourceParser<'_>,
    start: usize,
    end: usize,
) -> Option<SourceResultPattern> {
    if end - start != 8
        || !parser.word(start, "Result")
        || !parser.symbol(start + 1, Symbol::ColonColon)
        || !parser.symbol(start + 3, Symbol::LeftBrace)
        || !parser.symbol(start + 5, Symbol::Colon)
        || !parser.symbol(start + 7, Symbol::RightBrace)
    {
        return None;
    }
    let kind = if parser.word(start + 2, "Ok") && parser.word(start + 4, "value") {
        ResultPatternKind::Ok
    } else if parser.word(start + 2, "Err") && parser.word(start + 4, "error") {
        ResultPatternKind::Err
    } else {
        return None;
    };
    let name = parser.word_value(start + 6)?;
    if name == "_" || is_reserved_source_word(name) {
        return None;
    }
    Some(SourceResultPattern {
        kind,
        binding: Located {
            span: parser.tokens[start + 6].span,
            value: name.to_owned(),
        },
    })
}

pub(super) fn resolve_scrutinee(
    syntax: &SourceResultMatch,
    tasks: &[ResolvedTaskExpression],
    scopes: &[BTreeMap<String, SymbolId>],
    globals: &BTreeMap<String, SymbolId>,
) -> Option<ResolvedResultMatchScrutinee> {
    if let [task] = tasks
        && task.syntax.span == syntax.scrutinee_span
        && !task.syntax.propagates_error
    {
        return Some(ResolvedResultMatchScrutinee::Task(Box::new(task.clone())));
    }
    if let [TokenKind::Word(name)] = syntax.scrutinee_tokens.as_slice() {
        let symbol = scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .or_else(|| globals.get(name))
            .copied()?;
        return Some(ResolvedResultMatchScrutinee::Value(symbol));
    }
    None
}
