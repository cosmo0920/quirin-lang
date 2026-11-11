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

//! Closed, dimension-checked definitional indices in quantitative types.
//! `where n := e` introduces a fresh type-local index, never a runtime write.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::StaticEnvironment;
use crate::frontend::{Located, SourceParseError};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::ast::{AxisMode, DimensionRef};
use qcore_core::index::IndexTerm;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NaturalExpression {
    Literal(u128),
    Name(String),
    Add(Box<Self>, Box<Self>),
    Scale(u128, Box<Self>),
    Min(Box<Self>, Box<Self>),
    Max(Box<Self>, Box<Self>),
    Monus(Box<Self>, Box<Self>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhereItem {
    Binding {
        name: String,
        expression: NaturalExpression,
    },
    Constraint {
        left: NaturalExpression,
        relation: Symbol,
        right: NaturalExpression,
    },
    True,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQuantIndexType {
    pub span: Span,
    pub mode: AxisMode,
    /// None is scalar shorthand; elaboration must select exactly one axis.
    pub fields: Vec<(Option<String>, NaturalExpression)>,
    pub where_items: Vec<Located<WhereItem>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedQuantIndexType {
    pub span: Span,
    pub mode: AxisMode,
    pub fields: BTreeMap<String, IndexTerm>,
    pub bindings: BTreeMap<String, IndexTerm>,
}

/// Returns the closing symbol matching a quant type opener at `cursor + 1`,
/// or `None` when the tokens at `cursor` do not begin a quant index type.
/// `quant<...>` and its readable alias `quant[|...|]` are the same type; a
/// mixed opener/closer pair is refused by the closer expectation, never
/// repaired.
fn quant_opener_close(tokens: &[Token], cursor: usize) -> Option<Symbol> {
    if tokens.get(cursor).map(|token| &token.kind) != Some(&TokenKind::Word("quant".into()))
        || tokens.get(cursor + 2).map(|token| &token.kind)
            == Some(&TokenKind::Word("access".into()))
    {
        return None;
    }
    match tokens.get(cursor + 1).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Less)) => Some(Symbol::Greater),
        Some(TokenKind::Symbol(Symbol::QuantOpen)) => Some(Symbol::QuantClose),
        _ => None,
    }
}

pub(crate) fn parse_quant_index_types(
    tokens: &[Token],
) -> Result<Vec<SourceQuantIndexType>, SourceParseError> {
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor + 1 < tokens.len() {
        if let Some(close) = quant_opener_close(tokens, cursor) {
            let mut parser = Parser {
                tokens,
                cursor: cursor + 2,
            };
            output.push(parser.quant_type(tokens[cursor].span.start, close)?);
            cursor = parser.cursor;
        } else {
            cursor += 1;
        }
    }
    Ok(output)
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
}

pub(crate) fn quant_type_end(
    tokens: &[Token],
    cursor: usize,
) -> Result<Option<usize>, SourceParseError> {
    let Some(close) = quant_opener_close(tokens, cursor) else {
        return Ok(None);
    };
    let mut parser = Parser {
        tokens,
        cursor: cursor + 2,
    };
    parser.quant_type(tokens[cursor].span.start, close)?;
    Ok(Some(parser.cursor))
}

impl Parser<'_> {
    fn error(&self, message: impl Into<String>) -> SourceParseError {
        SourceParseError {
            span: self
                .tokens
                .get(self.cursor)
                .or_else(|| self.tokens.last())
                .map_or(Span { start: 0, end: 0 }, |token| token.span),
            message: message.into(),
        }
    }
    fn take(&mut self, symbol: Symbol) -> bool {
        if self.tokens.get(self.cursor).map(|token| &token.kind) == Some(&TokenKind::Symbol(symbol))
        {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, symbol: Symbol) -> Result<(), SourceParseError> {
        if self.take(symbol) {
            Ok(())
        } else {
            Err(self.error(format!("expected {symbol:?} in quantitative type")))
        }
    }
    fn word(&mut self, word: &str) -> bool {
        if self.tokens.get(self.cursor).map(|token| &token.kind)
            == Some(&TokenKind::Word(word.into()))
        {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn name(&mut self) -> Result<String, SourceParseError> {
        let Some(Token {
            kind: TokenKind::Word(name),
            ..
        }) = self.tokens.get(self.cursor)
        else {
            return Err(self.error("expected index or axis name"));
        };
        let name = name.clone();
        self.cursor += 1;
        Ok(name)
    }
    fn quant_type(
        &mut self,
        start: usize,
        close: Symbol,
    ) -> Result<SourceQuantIndexType, SourceParseError> {
        let explicit_mode = if self.word("credit") {
            Some(AxisMode::Credit)
        } else if self.word("live") {
            Some(AxisMode::Live)
        } else if self.word("cost") {
            return Err(self.error("cost quantitative types require a future total/peak profile"));
        } else {
            None
        };
        let mut fields = Vec::new();
        if explicit_mode.is_some() {
            self.expect(Symbol::Comma)?;
            self.expect(Symbol::LeftBrace)?;
            loop {
                let name = self.name()?;
                self.expect(Symbol::Equal)?;
                fields.push((Some(name), self.expression(0)?));
                if !self.take(Symbol::Comma) {
                    break;
                }
            }
            self.expect(Symbol::RightBrace)?;
        } else {
            fields.push((None, self.expression(0)?));
        }
        let mut where_items = Vec::new();
        if self.word("where") {
            loop {
                let item_start = self
                    .tokens
                    .get(self.cursor)
                    .ok_or_else(|| self.error("empty where clause"))?
                    .span
                    .start;
                let item = if self.word("true") {
                    WhereItem::True
                } else if self.tokens.get(self.cursor + 1).map(|token| &token.kind)
                    == Some(&TokenKind::Symbol(Symbol::Define))
                {
                    let name = self.name()?;
                    self.expect(Symbol::Define)?;
                    WhereItem::Binding {
                        name,
                        expression: self.expression(0)?,
                    }
                } else {
                    let left = self.expression(0)?;
                    let relation = match self.tokens.get(self.cursor).map(|token| &token.kind) {
                        Some(TokenKind::Symbol(
                            relation @ (Symbol::EqualEqual
                            | Symbol::NotEqual
                            | Symbol::Less
                            | Symbol::LessEqual
                            | Symbol::Greater
                            | Symbol::GreaterEqual),
                        )) => *relation,
                        _ => {
                            return Err(
                                self.error("where requires an index definition or comparison")
                            );
                        }
                    };
                    self.cursor += 1;
                    WhereItem::Constraint {
                        left,
                        relation,
                        right: self.expression(0)?,
                    }
                };
                where_items.push(Located {
                    span: Span {
                        start: item_start,
                        end: self.tokens[self.cursor - 1].span.end,
                    },
                    value: item,
                });
                if !self.take(Symbol::Comma) && !self.word("and") {
                    break;
                }
            }
        }
        self.expect(close)?;
        Ok(SourceQuantIndexType {
            span: Span {
                start,
                end: self.tokens[self.cursor - 1].span.end,
            },
            mode: explicit_mode.unwrap_or(AxisMode::Credit),
            fields,
            where_items,
        })
    }
    fn expression(&mut self, depth: usize) -> Result<NaturalExpression, SourceParseError> {
        let mut value = self.product(depth)?;
        let mut terms = 0;
        while self.take(Symbol::Plus) {
            value = NaturalExpression::Add(Box::new(value), Box::new(self.product(depth)?));
            terms += 1;
            if terms > 64 {
                return Err(self.error("index expression exceeds term limit"));
            }
        }
        Ok(value)
    }
    fn product(&mut self, depth: usize) -> Result<NaturalExpression, SourceParseError> {
        let left = self.atom(depth)?;
        if self.take(Symbol::Star) {
            let right = self.atom(depth)?;
            match (left, right) {
                (NaturalExpression::Literal(n), value) | (value, NaturalExpression::Literal(n)) => {
                    Ok(NaturalExpression::Scale(n, Box::new(value)))
                }
                _ => Err(self.error("index multiplication requires a literal scale factor")),
            }
        } else {
            Ok(left)
        }
    }
    fn atom(&mut self, depth: usize) -> Result<NaturalExpression, SourceParseError> {
        if depth > 32 {
            return Err(self.error("index expression exceeds depth limit"));
        }
        if self.take(Symbol::LeftParen) {
            let result = self.expression(depth + 1)?;
            self.expect(Symbol::RightParen)?;
            return Ok(result);
        }
        let Some(token) = self.tokens.get(self.cursor) else {
            return Err(self.error("missing index expression"));
        };
        match &token.kind {
            TokenKind::Integer(value) => {
                let value = value
                    .parse()
                    .map_err(|_| self.error("index literal exceeds u128 or is not decimal"))?;
                self.cursor += 1;
                Ok(NaturalExpression::Literal(value))
            }
            TokenKind::Word(name) if matches!(name.as_str(), "min" | "max" | "monus") => {
                let name = name.clone();
                self.cursor += 1;
                self.expect(Symbol::LeftParen)?;
                let left = Box::new(self.expression(depth + 1)?);
                self.expect(Symbol::Comma)?;
                let right = Box::new(self.expression(depth + 1)?);
                self.expect(Symbol::RightParen)?;
                Ok(match name.as_str() {
                    "min" => NaturalExpression::Min(left, right),
                    "max" => NaturalExpression::Max(left, right),
                    _ => NaturalExpression::Monus(left, right),
                })
            }
            TokenKind::Word(_) => self.name().map(NaturalExpression::Name),
            _ => {
                Err(self.error("index expressions require natural constants and local index names"))
            }
        }
    }
}

pub fn resolve_quant_index_type(
    source: &SourceQuantIndexType,
    environment: &StaticEnvironment,
    forbidden_names: &BTreeSet<String>,
) -> Result<ResolvedQuantIndexType, String> {
    let mut definitions = BTreeMap::new();
    for item in &source.where_items {
        if let WhereItem::Binding { name, expression } = &item.value
            && (forbidden_names.contains(name)
                || definitions
                    .insert(name.clone(), expression.clone())
                    .is_some())
        {
            return Err(format!("index binding `{name}` is not fresh"));
        }
    }
    let mut fields = BTreeMap::new();
    let mut dimensions = BTreeMap::<String, DimensionRef>::new();
    let mut visiting = BTreeSet::new();
    for (axis, expression) in &source.fields {
        let axis = if let Some(axis) = axis {
            environment
                .axes
                .get(axis)
                .ok_or_else(|| format!("unknown quant axis `{axis}`"))?
        } else {
            let candidates = environment
                .axes
                .values()
                .filter(|axis| axis.mode == AxisMode::Credit)
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return Err("scalar quant shorthand requires exactly one credit axis".into());
            }
            candidates[0]
        };
        if axis.mode != source.mode {
            return Err(format!("quant mode does not match axis `{}`", axis.name));
        }
        infer_dimension(
            expression,
            &axis.dimension,
            &definitions,
            &mut dimensions,
            &mut visiting,
        )?;
        let value = evaluate(expression, &definitions, &mut BTreeSet::new(), 0)?;
        if fields
            .insert(
                axis.name.clone(),
                IndexTerm::literal(value, axis.dimension.clone()),
            )
            .is_some()
        {
            return Err(format!("duplicate quant field `{}`", axis.name));
        }
    }
    for item in &source.where_items {
        if let WhereItem::Constraint {
            left,
            relation,
            right,
        } = &item.value
        {
            let mut names = BTreeSet::new();
            collect_names(left, &mut names);
            collect_names(right, &mut names);
            let inferred = names
                .iter()
                .filter_map(|name| dimensions.get(name))
                .cloned()
                .collect::<BTreeSet<_>>();
            if inferred.len() > 1 {
                return Err("dimension mismatch in where comparison".into());
            }
            if let Some(dimension) = inferred.iter().next() {
                infer_dimension(
                    left,
                    dimension,
                    &definitions,
                    &mut dimensions,
                    &mut visiting,
                )?;
                infer_dimension(
                    right,
                    dimension,
                    &definitions,
                    &mut dimensions,
                    &mut visiting,
                )?;
            }
            let left = evaluate(left, &definitions, &mut BTreeSet::new(), 0)?;
            let right = evaluate(right, &definitions, &mut BTreeSet::new(), 0)?;
            let accepted = match relation {
                Symbol::EqualEqual => left == right,
                Symbol::NotEqual => left != right,
                Symbol::Less => left < right,
                Symbol::LessEqual => left <= right,
                Symbol::Greater => left > right,
                Symbol::GreaterEqual => left >= right,
                _ => false,
            };
            if !accepted {
                return Err(format!(
                    "false where constraint: {left} {relation:?} {right}"
                ));
            }
        }
    }
    let mut bindings = BTreeMap::new();
    for (name, expression) in &definitions {
        let dimension = dimensions
            .get(name)
            .ok_or_else(|| format!("cannot infer dimension of unused binding `{name}`"))?;
        let value = evaluate(expression, &definitions, &mut BTreeSet::new(), 0)?;
        bindings.insert(name.clone(), IndexTerm::literal(value, dimension.clone()));
    }
    Ok(ResolvedQuantIndexType {
        span: source.span,
        mode: source.mode,
        fields,
        bindings,
    })
}

fn collect_names(expression: &NaturalExpression, names: &mut BTreeSet<String>) {
    match expression {
        NaturalExpression::Name(name) => {
            names.insert(name.clone());
        }
        NaturalExpression::Literal(_) => {}
        NaturalExpression::Scale(_, value) => collect_names(value, names),
        NaturalExpression::Add(left, right)
        | NaturalExpression::Min(left, right)
        | NaturalExpression::Max(left, right)
        | NaturalExpression::Monus(left, right) => {
            collect_names(left, names);
            collect_names(right, names);
        }
    }
}

fn infer_dimension(
    expression: &NaturalExpression,
    dimension: &DimensionRef,
    definitions: &BTreeMap<String, NaturalExpression>,
    dimensions: &mut BTreeMap<String, DimensionRef>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    collect_names(expression, &mut names);
    for name in names {
        let definition = definitions
            .get(&name)
            .ok_or_else(|| format!("unbound or runtime index name `{name}`"))?;
        if !visiting.insert(name.clone()) {
            return Err(format!("cyclic index definition `{name}`"));
        }
        if visiting.len() > 64 {
            return Err("index definitions exceed dependency depth limit".into());
        }
        if let Some(existing) = dimensions.get(&name) {
            if existing != dimension {
                return Err(format!("index `{name}` is used at incompatible dimensions"));
            }
            visiting.remove(&name);
            continue;
        } else {
            dimensions.insert(name.clone(), dimension.clone());
        }
        infer_dimension(definition, dimension, definitions, dimensions, visiting)?;
        visiting.remove(&name);
    }
    Ok(())
}

fn evaluate(
    expression: &NaturalExpression,
    definitions: &BTreeMap<String, NaturalExpression>,
    visiting: &mut BTreeSet<String>,
    depth: usize,
) -> Result<u128, String> {
    evaluate_cached(
        expression,
        definitions,
        visiting,
        depth,
        &mut BTreeMap::new(),
    )
}

fn evaluate_cached(
    expression: &NaturalExpression,
    definitions: &BTreeMap<String, NaturalExpression>,
    visiting: &mut BTreeSet<String>,
    depth: usize,
    cache: &mut BTreeMap<String, u128>,
) -> Result<u128, String> {
    if depth > 128 {
        return Err("index evaluation exceeds depth limit".into());
    }
    Ok(match expression {
        NaturalExpression::Literal(value) => *value,
        NaturalExpression::Name(name) => {
            if let Some(value) = cache.get(name) {
                return Ok(*value);
            }
            let definition = definitions
                .get(name)
                .ok_or_else(|| format!("unbound or runtime index name `{name}`"))?;
            if !visiting.insert(name.clone()) {
                return Err(format!("cyclic index definition `{name}`"));
            }
            let value = evaluate_cached(definition, definitions, visiting, depth + 1, cache)?;
            visiting.remove(name);
            cache.insert(name.clone(), value);
            value
        }
        NaturalExpression::Scale(scale, value) => scale
            .checked_mul(evaluate_cached(
                value,
                definitions,
                visiting,
                depth + 1,
                cache,
            )?)
            .ok_or("index multiplication overflow")?,
        NaturalExpression::Add(left, right) => {
            evaluate_cached(left, definitions, visiting, depth + 1, cache)?
                .checked_add(evaluate_cached(
                    right,
                    definitions,
                    visiting,
                    depth + 1,
                    cache,
                )?)
                .ok_or("index addition overflow")?
        }
        NaturalExpression::Min(left, right) => {
            evaluate_cached(left, definitions, visiting, depth + 1, cache)?.min(evaluate_cached(
                right,
                definitions,
                visiting,
                depth + 1,
                cache,
            )?)
        }
        NaturalExpression::Max(left, right) => {
            evaluate_cached(left, definitions, visiting, depth + 1, cache)?.max(evaluate_cached(
                right,
                definitions,
                visiting,
                depth + 1,
                cache,
            )?)
        }
        NaturalExpression::Monus(left, right) => {
            evaluate_cached(left, definitions, visiting, depth + 1, cache)?
                .checked_sub(evaluate_cached(
                    right,
                    definitions,
                    visiting,
                    depth + 1,
                    cache,
                )?)
                .ok_or("checked monus requires right <= left")?
        }
    })
}
