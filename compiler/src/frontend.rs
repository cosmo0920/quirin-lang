//! Cycle 10 complete source-front-end skeleton.
//!
//! The earlier cycles intentionally exposed their checked IRs as Rust APIs and
//! parsed only dimensions and quantitative axes.  This module is the common
//! source boundary: it recognizes every declaration and statement family in
//! the revision 0.1 grammar, retains byte spans, resolves bindings, and
//! classifies the resulting nodes for the existing semantic passes.  The
//! revision 0.2 concurrency forms are parsed as well, but are deliberately not
//! certified by the sequential Cycle 10 elaborator.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::lexer::{LexError, Span, Symbol, Token, TokenKind, lex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<T> {
    pub span: Span,
    pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclarationKind {
    ConcurrencyProfile,
    Dimension,
    Axis,
    TypeAlias,
    ResourceType,
    ErrorType,
    Constant,
    ExternFunction,
    Function,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDeclaration {
    pub kind: DeclarationKind,
    pub name: String,
    pub parameters: Vec<SourceParameter>,
    pub body: Option<SourceBlock>,
    pub tokens: Vec<TokenKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTypeKind {
    Atomic,
    Named,
    Tuple,
    Array,
    Slice,
    Pointer,
    Owned,
    Borrow,
    Quant,
    AccessQuant,
    Result,
    Function,
    Never,
    Task,
    Mutex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceParameter {
    pub name: Located<String>,
    pub type_kind: SourceTypeKind,
    pub type_span: Span,
    pub type_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBlock {
    pub span: Span,
    pub statements: Vec<Located<StatementKind>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Let {
        binding: Option<String>,
    },
    LetIndex {
        binding: String,
    },
    Assignment {
        place: Option<String>,
    },
    Expression,
    Drop {
        place: Option<String>,
    },
    Move {
        place: Option<String>,
    },
    Discharge {
        binding: Option<String>,
    },
    Guard,
    If {
        branches: Vec<SourceBlock>,
    },
    Match {
        arms: Vec<SourceBlock>,
    },
    While {
        body: SourceBlock,
    },
    Return,
    Raise,
    Break,
    Continue,
    Defer,
    ErrorRegion {
        name: String,
        blocks: Vec<SourceBlock>,
    },
    Unsafe {
        body: SourceBlock,
    },
    ThreadScope {
        name: String,
        body: SourceBlock,
    },
    Footprint {
        name: String,
    },
    WithLock {
        guard: String,
        body: SourceBlock,
    },
}

impl StatementKind {
    pub fn nested_blocks(&self) -> Vec<&SourceBlock> {
        match self {
            Self::If { branches } | Self::Match { arms: branches } => branches.iter().collect(),
            Self::While { body }
            | Self::Unsafe { body }
            | Self::ThreadScope { body, .. }
            | Self::WithLock { body, .. } => vec![body],
            Self::ErrorRegion { blocks, .. } => blocks.iter().collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    source: String,
    lexical_tokens: Vec<Token>,
    tokens: Vec<TokenKind>,
    pub declarations: Vec<Located<SourceDeclaration>>,
}

impl SourceProgram {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn token_kinds(&self) -> &[TokenKind] {
        &self.tokens
    }

    pub fn lexical_tokens(&self) -> &[Token] {
        &self.lexical_tokens
    }

    /// Round trips compare lexical structure rather than byte offsets.  This
    /// permits callers to format or indent a program without invalidating the
    /// source locations from the original parse.
    pub fn syntax_equivalent(&self, other: &Self) -> bool {
        self.tokens == other.tokens
            && self
                .declarations
                .iter()
                .map(|decl| (decl.value.kind, decl.value.name.as_str()))
                .eq(other
                    .declarations
                    .iter()
                    .map(|decl| (decl.value.kind, decl.value.name.as_str())))
    }
}

impl fmt::Display for SourceProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.source.trim())?;
        if !self.source.trim().is_empty() {
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceParseError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for SourceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SourceParseError {}

impl From<LexError> for SourceParseError {
    fn from(error: LexError) -> Self {
        Self {
            span: error.span,
            message: error.message,
        }
    }
}

pub fn parse_source_program(source: &str) -> Result<SourceProgram, SourceParseError> {
    SourceParser::new(source)?.parse()
}

struct SourceParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    matching: BTreeMap<usize, usize>,
}

impl<'a> SourceParser<'a> {
    fn new(source: &'a str) -> Result<Self, SourceParseError> {
        let mut tokens = lex(source)?;
        tokens.pop();
        let mut stack = Vec::<(Symbol, usize)>::new();
        let mut matching = BTreeMap::new();
        for (index, token) in tokens.iter().enumerate() {
            let symbol = match token.kind {
                TokenKind::Symbol(symbol) => symbol,
                _ => continue,
            };
            match symbol {
                Symbol::LeftParen | Symbol::LeftBrace | Symbol::LeftBracket => {
                    stack.push((symbol, index));
                }
                Symbol::RightParen | Symbol::RightBrace | Symbol::RightBracket => {
                    let Some((open, start)) = stack.pop() else {
                        return Err(Self::error_at(token.span, "unmatched closing delimiter"));
                    };
                    if !matches!(
                        (open, symbol),
                        (Symbol::LeftParen, Symbol::RightParen)
                            | (Symbol::LeftBrace, Symbol::RightBrace)
                            | (Symbol::LeftBracket, Symbol::RightBracket)
                    ) {
                        return Err(Self::error_at(token.span, "mismatched closing delimiter"));
                    }
                    matching.insert(start, index);
                    matching.insert(index, start);
                }
                _ => {}
            }
        }
        if let Some((_, start)) = stack.pop() {
            return Err(Self::error_at(
                tokens[start].span,
                "unclosed source delimiter",
            ));
        }
        Ok(Self {
            source,
            tokens,
            matching,
        })
    }

    fn parse(self) -> Result<SourceProgram, SourceParseError> {
        let mut declarations = Vec::new();
        let mut cursor = 0;
        while cursor < self.tokens.len() {
            let (declaration, next) = self.declaration(cursor)?;
            declarations.push(declaration);
            cursor = next;
        }
        Ok(SourceProgram {
            source: self.source.to_owned(),
            lexical_tokens: self.tokens.clone(),
            tokens: self.tokens.iter().map(|token| token.kind.clone()).collect(),
            declarations,
        })
    }

    fn declaration(
        &self,
        start: usize,
    ) -> Result<(Located<SourceDeclaration>, usize), SourceParseError> {
        let (kind, name_index) = if self.words(start, &["concurrency", "checked"]) {
            (DeclarationKind::ConcurrencyProfile, None)
        } else if self.word(start, "dimension") {
            (DeclarationKind::Dimension, Some(start + 1))
        } else if self.words(start, &["quant", "axis"]) {
            (DeclarationKind::Axis, Some(start + 2))
        } else if self.words(start, &["resource", "type"]) {
            (DeclarationKind::ResourceType, Some(start + 2))
        } else if self.word(start, "type") {
            (DeclarationKind::TypeAlias, Some(start + 1))
        } else if self.word(start, "error") {
            (DeclarationKind::ErrorType, Some(start + 1))
        } else if self.word(start, "const") {
            (DeclarationKind::Constant, Some(start + 1))
        } else if self.words(start, &["extern", "fn"]) {
            (DeclarationKind::ExternFunction, Some(start + 2))
        } else if self.word(start, "fn") {
            (DeclarationKind::Function, Some(start + 1))
        } else {
            return Err(self.error(start, "expected a QCore declaration"));
        };

        let name = match name_index {
            Some(index) => self.identifier(index)?.to_owned(),
            None => "checked-concurrency-0.2".to_owned(),
        };
        let (end, body_open) = match kind {
            DeclarationKind::ResourceType | DeclarationKind::ErrorType => {
                let open = self.find_symbol(start, Symbol::LeftBrace)?;
                (self.close(open)? + 1, None)
            }
            DeclarationKind::Function => {
                let open = self.function_body(start)?;
                (self.close(open)? + 1, Some(open))
            }
            _ => (self.semicolon(start)? + 1, None),
        };
        let body = body_open.map(|open| self.block(open)).transpose()?;
        self.validate_declaration(kind, start, end)?;
        let parameters = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            self.function_parameters(start, end)?
        } else {
            Vec::new()
        };
        let span = self.range_span(start, end);
        Ok((
            Located {
                span,
                value: SourceDeclaration {
                    kind,
                    name,
                    parameters,
                    body,
                    tokens: self.tokens[start..end]
                        .iter()
                        .map(|token| token.kind.clone())
                        .collect(),
                },
            },
            end,
        ))
    }

    fn function_body(&self, start: usize) -> Result<usize, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let mut cursor = self.close(params)? + 1;
        while cursor < self.tokens.len() {
            if self.symbol(cursor, Symbol::LeftBrace) {
                let close = self.close(cursor)?;
                let next = close + 1;
                if next == self.tokens.len() || self.is_declaration_start(next) {
                    return Ok(cursor);
                }
                cursor = next;
            } else if self.symbol(cursor, Symbol::Semicolon) {
                return Err(self.error(cursor, "function declaration requires a body"));
            } else {
                cursor += 1;
            }
        }
        Err(self.error(start, "function declaration requires a body"))
    }

    fn function_parameters(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<SourceParameter>, SourceParseError> {
        let open = self.find_symbol(start, Symbol::LeftParen)?;
        let close = self.close(open)?;
        if close >= end {
            return Err(self.error(open, "invalid function parameter list"));
        }
        let mut parameters = Vec::new();
        let mut segment = open + 1;
        let mut cursor = segment;
        let mut angle_depth = 0_u32;
        while cursor <= close {
            let separator =
                cursor == close || (angle_depth == 0 && self.symbol(cursor, Symbol::Comma));
            if separator {
                if segment < cursor {
                    parameters.push(self.source_parameter(segment, cursor)?);
                }
                segment = cursor + 1;
                cursor += 1;
                continue;
            }
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
                cursor += 1;
            } else if self.symbol(cursor, Symbol::Greater) && angle_depth > 0 {
                angle_depth -= 1;
                cursor += 1;
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        Ok(parameters)
    }

    fn source_parameter(
        &self,
        start: usize,
        end: usize,
    ) -> Result<SourceParameter, SourceParseError> {
        let name = self.identifier(start)?.to_owned();
        if !self.symbol(start + 1, Symbol::Colon) {
            return Err(self.error(start + 1, "parameter requires a type annotation"));
        }
        let type_start = start + 2;
        let mut type_end = end;
        let mut cursor = type_start;
        let mut angle_depth = 0_u32;
        while cursor < end {
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
            } else if self.symbol(cursor, Symbol::Greater) && angle_depth > 0 {
                angle_depth -= 1;
            } else if angle_depth == 0 && (self.word(cursor, "as") || self.word(cursor, "use")) {
                type_end = cursor;
                break;
            }
            cursor = self.skip_group(cursor);
        }
        if type_start >= type_end {
            return Err(self.error(type_start, "parameter type cannot be empty"));
        }
        let type_span = self.range_span(type_start, type_end);
        Ok(SourceParameter {
            name: Located {
                span: self.tokens[start].span,
                value: name,
            },
            type_kind: self.type_kind(type_start),
            type_span,
            type_source: self.source[type_span.start..type_span.end].to_owned(),
        })
    }

    fn type_kind(&self, start: usize) -> SourceTypeKind {
        if self.symbol(start, Symbol::LeftParen) {
            return SourceTypeKind::Tuple;
        }
        match self.word_value(start) {
            Some(
                "unit" | "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
                | "isize" | "usize" | "f32" | "f64",
            ) => SourceTypeKind::Atomic,
            Some("array") => SourceTypeKind::Array,
            Some("slice") => SourceTypeKind::Slice,
            Some("ptr") => SourceTypeKind::Pointer,
            Some("own") => SourceTypeKind::Owned,
            Some("borrow") => SourceTypeKind::Borrow,
            Some("quant") if self.word(start + 2, "access") => SourceTypeKind::AccessQuant,
            Some("quant") => SourceTypeKind::Quant,
            Some("result") => SourceTypeKind::Result,
            Some("fn") => SourceTypeKind::Function,
            Some("never") => SourceTypeKind::Never,
            Some("task") => SourceTypeKind::Task,
            Some("mutex") => SourceTypeKind::Mutex,
            _ => SourceTypeKind::Named,
        }
    }

    fn validate_declaration(
        &self,
        kind: DeclarationKind,
        start: usize,
        end: usize,
    ) -> Result<(), SourceParseError> {
        match kind {
            DeclarationKind::ConcurrencyProfile => {
                if end - start != 3 || !self.symbol(end - 1, Symbol::Semicolon) {
                    return Err(self.error(start, "expected `concurrency checked;`"));
                }
            }
            DeclarationKind::Dimension => {
                if end - start != 7
                    || !self.symbol(start + 2, Symbol::Colon)
                    || !self.word(start + 3, "Idx")
                    || !self.symbol(start + 4, Symbol::Plus)
                    || !self.word(start + 5, "Ordered")
                {
                    return Err(self.error(start, "malformed dimension declaration"));
                }
            }
            DeclarationKind::Axis => {
                if !self.symbol(start + 3, Symbol::Colon)
                    || self
                        .top_level_symbol(start + 4, end, Symbol::Less)
                        .is_none()
                    || !self.symbol(end - 1, Symbol::Semicolon)
                {
                    return Err(self.error(start, "malformed quantitative-axis declaration"));
                }
            }
            DeclarationKind::TypeAlias => {
                if self
                    .top_level_symbol(start + 2, end, Symbol::Equal)
                    .is_none()
                {
                    return Err(self.error(start, "type alias requires `=` and a type"));
                }
            }
            DeclarationKind::Constant => {
                if self
                    .top_level_symbol(start + 2, end, Symbol::Colon)
                    .is_none()
                    || self
                        .top_level_symbol(start + 2, end, Symbol::Equal)
                        .is_none()
                {
                    return Err(self.error(start, "constant requires a type and initializer"));
                }
            }
            DeclarationKind::ExternFunction | DeclarationKind::Function => {
                let params = self.find_symbol(start, Symbol::LeftParen)?;
                let after = self.close(params)? + 1;
                if !self.symbol(after, Symbol::Arrow) {
                    return Err(self.error(after, "function requires `->` return type"));
                }
            }
            DeclarationKind::ResourceType => {
                for member in ["repr", "footprint", "destructor"] {
                    if self.find_word_between(start, end, member).is_none() {
                        return Err(
                            self.error(start, format!("resource type requires `{member}` member"))
                        );
                    }
                }
            }
            DeclarationKind::ErrorType => {}
        }
        Ok(())
    }

    fn block(&self, open: usize) -> Result<SourceBlock, SourceParseError> {
        let close = self.close(open)?;
        let mut cursor = open + 1;
        let mut statements = Vec::new();
        while cursor < close {
            let (statement, next) = self.statement(cursor, close)?;
            if next <= cursor {
                return Err(self.error(cursor, "parser made no progress in block"));
            }
            statements.push(statement);
            cursor = next;
        }
        Ok(SourceBlock {
            span: self.range_span(open, close + 1),
            statements,
        })
    }

    fn statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        if self.word(start, "if") {
            return self.if_statement(start);
        }
        if self.word(start, "match") {
            return self.match_statement(start, limit);
        }
        if self.word(start, "while") {
            return self.single_body_statement(start, limit, |body| StatementKind::While { body });
        }
        if self.words(start, &["error", "region"]) {
            return self.error_region_statement(start, limit);
        }
        if self.word(start, "unsafe") {
            return self.single_body_statement(start, limit, |body| StatementKind::Unsafe { body });
        }
        if self.words(start, &["thread", "scope"]) {
            let name = self.identifier(start + 2)?.to_owned();
            return self.single_body_statement(start, limit, |body| StatementKind::ThreadScope {
                name,
                body,
            });
        }
        if self.words(start, &["with", "lock"]) {
            let body_open = self.last_top_level_block(start, limit)?;
            let guard = self
                .find_word_between(start + 2, body_open, "as")
                .and_then(|index| self.word_value(index + 1))
                .ok_or_else(|| self.error(start, "with lock requires `as` guard binding"))?
                .to_owned();
            let body = self.block(body_open)?;
            let next = self.close(body_open)? + 1;
            return Ok(self.located_statement(
                start,
                next,
                StatementKind::WithLock { guard, body },
            ));
        }

        let semicolon = self.semicolon_before(start, limit)?;
        let next = semicolon + 1;
        let kind = if self.words(start, &["let", "idx"]) {
            StatementKind::LetIndex {
                binding: self.identifier(start + 2)?.to_owned(),
            }
        } else if self.word(start, "let") {
            StatementKind::Let {
                binding: self.pattern_binding(start + 1, semicolon),
            }
        } else if self.word(start, "drop") {
            StatementKind::Drop {
                place: self.root_name(start + 1, semicolon),
            }
        } else if self.word(start, "move") {
            StatementKind::Move {
                place: self.root_name(start + 1, semicolon),
            }
        } else if self.word(start, "discharge") {
            StatementKind::Discharge {
                binding: self.word_value(start + 1).map(ToOwned::to_owned),
            }
        } else if self.word(start, "guard") {
            StatementKind::Guard
        } else if self.word(start, "return") {
            StatementKind::Return
        } else if self.word(start, "raise") {
            StatementKind::Raise
        } else if self.word(start, "break") {
            StatementKind::Break
        } else if self.word(start, "continue") {
            StatementKind::Continue
        } else if self.word(start, "defer") {
            StatementKind::Defer
        } else if self.word(start, "footprint") {
            StatementKind::Footprint {
                name: self.identifier(start + 1)?.to_owned(),
            }
        } else if let Some(equal) = self.top_level_symbol(start, semicolon, Symbol::Equal) {
            StatementKind::Assignment {
                place: self.root_name(start, equal),
            }
        } else {
            StatementKind::Expression
        };
        Ok(self.located_statement(start, next, kind))
    }

    fn if_statement(
        &self,
        start: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let condition = self.find_symbol(start, Symbol::LeftParen)?;
        let then_open = self.close(condition)? + 1;
        if !self.symbol(then_open, Symbol::LeftBrace) {
            return Err(self.error(then_open, "if condition must be followed by a block"));
        }
        let mut branches = vec![self.block(then_open)?];
        let mut next = self.close(then_open)? + 1;
        if self.word(next, "else") {
            if self.symbol(next + 1, Symbol::LeftBrace) {
                branches.push(self.block(next + 1)?);
                next = self.close(next + 1)? + 1;
            } else if self.word(next + 1, "if") {
                let (nested, nested_next) = self.if_statement(next + 1)?;
                if let StatementKind::If {
                    branches: nested_branches,
                } = nested.value
                {
                    branches.extend(nested_branches);
                }
                next = nested_next;
            } else {
                return Err(self.error(next + 1, "else must be followed by a block or if"));
            }
        }
        Ok(self.located_statement(start, next, StatementKind::If { branches }))
    }

    fn match_statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let open = self.last_top_level_block(start, limit)?;
        let close = self.close(open)?;
        let mut arms = Vec::new();
        let mut cursor = open + 1;
        while cursor < close {
            let arrow = self
                .top_level_symbol(cursor, close, Symbol::FatArrow)
                .ok_or_else(|| self.error(cursor, "match arm requires `=>`"))?;
            let body_open = arrow + 1;
            if !self.symbol(body_open, Symbol::LeftBrace) {
                return Err(self.error(body_open, "match arm requires a block"));
            }
            arms.push(self.block(body_open)?);
            cursor = self.close(body_open)? + 1;
        }
        Ok(self.located_statement(start, close + 1, StatementKind::Match { arms }))
    }

    fn single_body_statement<F>(
        &self,
        start: usize,
        limit: usize,
        constructor: F,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError>
    where
        F: FnOnce(SourceBlock) -> StatementKind,
    {
        let open = self.last_top_level_block(start, limit)?;
        let body = self.block(open)?;
        let next = self.close(open)? + 1;
        Ok(self.located_statement(start, next, constructor(body)))
    }

    fn error_region_statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let name = self.identifier(start + 2)?.to_owned();
        let body_open = self.find_symbol(start + 3, Symbol::LeftBrace)?;
        let mut blocks = vec![self.block(body_open)?];
        let mut next = self.close(body_open)? + 1;
        while next < limit && self.word(next, "catch") {
            let pattern_open = next + 1;
            if !self.symbol(pattern_open, Symbol::LeftParen) {
                return Err(self.error(pattern_open, "catch requires a parenthesized pattern"));
            }
            let mut block_open = self.close(pattern_open)? + 1;
            if self.words(block_open, &["quant", "open"]) {
                block_open += 5;
            }
            if !self.symbol(block_open, Symbol::LeftBrace) {
                return Err(self.error(block_open, "catch requires a block"));
            }
            blocks.push(self.block(block_open)?);
            next = self.close(block_open)? + 1;
        }
        if blocks.len() == 1 {
            return Err(self.error(next, "error region requires at least one catch"));
        }
        Ok(self.located_statement(start, next, StatementKind::ErrorRegion { name, blocks }))
    }

    fn located_statement(
        &self,
        start: usize,
        end: usize,
        value: StatementKind,
    ) -> (Located<StatementKind>, usize) {
        (
            Located {
                span: self.range_span(start, end),
                value,
            },
            end,
        )
    }

    fn last_top_level_block(&self, start: usize, limit: usize) -> Result<usize, SourceParseError> {
        let mut cursor = start;
        let mut candidate = None;
        while cursor < limit {
            if self.symbol(cursor, Symbol::LeftBrace) {
                candidate = Some(cursor);
                cursor = self.close(cursor)? + 1;
                if cursor == limit || self.is_statement_start(cursor) {
                    break;
                }
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        candidate.ok_or_else(|| self.error(start, "structured statement requires a block"))
    }

    fn semicolon(&self, start: usize) -> Result<usize, SourceParseError> {
        self.semicolon_before(start, self.tokens.len())
    }

    fn semicolon_before(&self, start: usize, limit: usize) -> Result<usize, SourceParseError> {
        let mut cursor = start;
        while cursor < limit {
            if self.symbol(cursor, Symbol::Semicolon) {
                return Ok(cursor);
            }
            cursor = self.skip_group(cursor);
        }
        Err(self.error(start, "statement or declaration requires `;`"))
    }

    fn top_level_symbol(&self, start: usize, limit: usize, expected: Symbol) -> Option<usize> {
        let mut cursor = start;
        while cursor < limit {
            if self.symbol(cursor, expected) {
                return Some(cursor);
            }
            cursor = self.skip_group(cursor);
        }
        None
    }

    fn find_symbol(&self, start: usize, expected: Symbol) -> Result<usize, SourceParseError> {
        (start..self.tokens.len())
            .find(|&index| self.symbol(index, expected))
            .ok_or_else(|| self.error(start, format!("expected {expected:?}")))
    }

    fn find_word_between(&self, start: usize, end: usize, expected: &str) -> Option<usize> {
        (start..end).find(|&index| self.word(index, expected))
    }

    fn close(&self, open: usize) -> Result<usize, SourceParseError> {
        self.matching
            .get(&open)
            .copied()
            .ok_or_else(|| self.error(open, "expected a matched delimiter"))
    }

    fn skip_group(&self, cursor: usize) -> usize {
        if matches!(
            self.tokens.get(cursor).map(|token| &token.kind),
            Some(TokenKind::Symbol(
                Symbol::LeftParen | Symbol::LeftBrace | Symbol::LeftBracket
            ))
        ) {
            self.matching
                .get(&cursor)
                .map_or(cursor + 1, |close| close + 1)
        } else {
            cursor + 1
        }
    }

    fn pattern_binding(&self, start: usize, limit: usize) -> Option<String> {
        (start..limit).find_map(|index| {
            self.word_value(index)
                .and_then(|word| (!matches!(word, "_" | "mut")).then(|| word.to_owned()))
        })
    }

    fn root_name(&self, start: usize, limit: usize) -> Option<String> {
        (start..limit).find_map(|index| self.word_value(index).map(ToOwned::to_owned))
    }

    fn range_span(&self, start: usize, end: usize) -> Span {
        Span {
            start: self.tokens[start].span.start,
            end: self.tokens[end - 1].span.end,
        }
    }

    fn identifier(&self, index: usize) -> Result<&str, SourceParseError> {
        self.word_value(index)
            .ok_or_else(|| self.error(index, "expected identifier"))
    }

    fn word_value(&self, index: usize) -> Option<&str> {
        match self.tokens.get(index).map(|token| &token.kind) {
            Some(TokenKind::Word(word)) => Some(word),
            _ => None,
        }
    }

    fn word(&self, index: usize, expected: &str) -> bool {
        self.word_value(index) == Some(expected)
    }

    fn words(&self, start: usize, expected: &[&str]) -> bool {
        expected
            .iter()
            .enumerate()
            .all(|(offset, word)| self.word(start + offset, word))
    }

    fn symbol(&self, index: usize, expected: Symbol) -> bool {
        matches!(
            self.tokens.get(index).map(|token| &token.kind),
            Some(TokenKind::Symbol(symbol)) if *symbol == expected
        )
    }

    fn is_declaration_start(&self, index: usize) -> bool {
        self.word(index, "dimension")
            || self.word(index, "quant")
            || self.word(index, "type")
            || self.word(index, "resource")
            || self.word(index, "error")
            || self.word(index, "const")
            || self.word(index, "extern")
            || self.word(index, "fn")
            || self.word(index, "concurrency")
    }

    fn is_statement_start(&self, index: usize) -> bool {
        matches!(
            self.word_value(index),
            Some(
                "let"
                    | "drop"
                    | "move"
                    | "discharge"
                    | "guard"
                    | "if"
                    | "match"
                    | "while"
                    | "return"
                    | "raise"
                    | "break"
                    | "continue"
                    | "defer"
                    | "error"
                    | "unsafe"
                    | "thread"
                    | "footprint"
                    | "with"
            )
        )
    }

    fn error(&self, index: usize, message: impl Into<String>) -> SourceParseError {
        let span = self.tokens.get(index).map_or(
            Span {
                start: self.source.len(),
                end: self.source.len(),
            },
            |token| token.span,
        );
        Self::error_at(span, message)
    }

    fn error_at(span: Span, message: impl Into<String>) -> SourceParseError {
        SourceParseError {
            span,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticPass {
    NameResolution,
    OrdinaryTypes,
    Index,
    Usage,
    Ownership,
    ResourceFlow,
    TypedErrors,
    ErrorRegion,
    CleanupLowering,
    CEmission,
    Concurrency,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDiagnostic {
    pub code: &'static str,
    pub pass: SemanticPass,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} at bytes {}..{}",
            self.code, self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SourceDiagnostic {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedBinding {
    pub symbol: SymbolId,
    pub name: String,
    pub span: Span,
    pub type_kind: SourceTypeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedStatement {
    pub span: Span,
    pub pass: SemanticPass,
    pub referenced_symbols: Vec<SymbolId>,
    pub nested: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedSourceDeclaration {
    pub symbol: SymbolId,
    pub kind: DeclarationKind,
    pub name: String,
    pub span: Span,
    pub parameters: Vec<TypedBinding>,
    pub statements: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedSourceProgram {
    pub declarations: Vec<TypedSourceDeclaration>,
    pub globals: BTreeMap<String, SymbolId>,
    pub origin: String,
}

pub fn elaborate_source(
    program: &SourceProgram,
) -> Result<TypedSourceProgram, Vec<SourceDiagnostic>> {
    let mut diagnostics = Vec::new();
    let mut globals = BTreeMap::new();
    let mut next_symbol = 0_u32;
    for declaration in &program.declarations {
        if declaration.value.kind == DeclarationKind::ConcurrencyProfile {
            continue;
        }
        if globals.contains_key(&declaration.value.name) {
            diagnostics.push(SourceDiagnostic {
                code: "Q010",
                pass: SemanticPass::NameResolution,
                span: declaration.span,
                message: format!("duplicate declaration `{}`", declaration.value.name),
            });
        } else {
            globals.insert(declaration.value.name.clone(), SymbolId(next_symbol));
            next_symbol += 1;
        }
    }

    let mut declarations = Vec::new();
    for declaration in &program.declarations {
        if declaration.value.kind == DeclarationKind::ConcurrencyProfile {
            continue;
        }
        let symbol = globals[&declaration.value.name];
        let mut scopes = vec![BTreeMap::<String, SymbolId>::new()];
        let mut parameters = Vec::new();
        for parameter in &declaration.value.parameters {
            if scopes[0].contains_key(&parameter.name.value) {
                diagnostics.push(SourceDiagnostic {
                    code: "Q011",
                    pass: SemanticPass::NameResolution,
                    span: parameter.name.span,
                    message: format!("duplicate parameter `{}`", parameter.name.value),
                });
                continue;
            }
            let parameter_symbol = SymbolId(next_symbol);
            next_symbol += 1;
            scopes[0].insert(parameter.name.value.clone(), parameter_symbol);
            parameters.push(TypedBinding {
                symbol: parameter_symbol,
                name: parameter.name.value.clone(),
                span: parameter.name.span,
                type_kind: parameter.type_kind,
            });
        }
        let statements = declaration
            .value
            .body
            .as_ref()
            .map_or_else(Vec::new, |body| {
                elaborate_block(
                    body,
                    &mut scopes,
                    &globals,
                    &program.lexical_tokens,
                    &mut next_symbol,
                    &mut diagnostics,
                )
            });
        declarations.push(TypedSourceDeclaration {
            symbol,
            kind: declaration.value.kind,
            name: declaration.value.name.clone(),
            span: declaration.span,
            parameters,
            statements,
        });
    }
    if diagnostics.is_empty() {
        Ok(TypedSourceProgram {
            declarations,
            globals,
            origin: program.source.clone(),
        })
    } else {
        Err(diagnostics)
    }
}

fn elaborate_block(
    block: &SourceBlock,
    scopes: &mut Vec<BTreeMap<String, SymbolId>>,
    globals: &BTreeMap<String, SymbolId>,
    lexical_tokens: &[Token],
    next_symbol: &mut u32,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Vec<TypedStatement> {
    scopes.push(BTreeMap::new());
    let mut typed = Vec::new();
    for statement in &block.statements {
        let (binding, reference, pass) = statement_metadata(&statement.value);
        let mut references = lexical_references(statement, lexical_tokens);
        if let Some(reference) = reference {
            references.insert(reference.to_owned());
        }
        let mut referenced_symbols = Vec::new();
        for name in references {
            let resolved = scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&name))
                .or_else(|| globals.get(&name))
                .copied();
            if let Some(symbol) = resolved {
                referenced_symbols.push(symbol);
            } else {
                diagnostics.push(SourceDiagnostic {
                    code: "Q013",
                    pass: SemanticPass::NameResolution,
                    span: statement.span,
                    message: format!("unknown name `{name}`"),
                });
            }
        }
        if let Some(binding) = binding {
            let scope = scopes.last_mut().expect("block scope exists");
            if scope.contains_key(binding) {
                diagnostics.push(SourceDiagnostic {
                    code: "Q012",
                    pass: SemanticPass::NameResolution,
                    span: statement.span,
                    message: format!("duplicate local binding `{binding}`"),
                });
            } else {
                scope.insert(binding.to_owned(), SymbolId(*next_symbol));
                *next_symbol += 1;
            }
        }
        let mut nested = Vec::new();
        for child in statement.value.nested_blocks() {
            nested.extend(elaborate_block(
                child,
                scopes,
                globals,
                lexical_tokens,
                next_symbol,
                diagnostics,
            ));
        }
        typed.push(TypedStatement {
            span: statement.span,
            pass,
            referenced_symbols,
            nested,
        });
    }
    scopes.pop();
    typed
}

fn lexical_references(
    statement: &Located<StatementKind>,
    lexical_tokens: &[Token],
) -> BTreeSet<String> {
    let mut tokens = lexical_tokens
        .iter()
        .filter(|token| {
            statement.span.start <= token.span.start && token.span.end <= statement.span.end
        })
        .collect::<Vec<_>>();
    if matches!(
        statement.value,
        StatementKind::If { .. }
            | StatementKind::Match { .. }
            | StatementKind::While { .. }
            | StatementKind::ErrorRegion { .. }
            | StatementKind::Unsafe { .. }
            | StatementKind::ThreadScope { .. }
            | StatementKind::WithLock { .. }
    ) && let Some(body) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        tokens.truncate(body);
    }
    if matches!(
        statement.value,
        StatementKind::Let { .. } | StatementKind::LetIndex { .. }
    ) && let Some(equal) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
    {
        tokens.drain(..=equal);
    }
    if matches!(statement.value, StatementKind::Assignment { .. })
        && let Some(equal) = tokens
            .iter()
            .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
    {
        tokens.drain(..=equal);
    }

    let mut result = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        let TokenKind::Word(word) = &token.kind else {
            continue;
        };
        let previous = index.checked_sub(1).and_then(|index| tokens.get(index));
        let next = tokens.get(index + 1);
        if is_reserved_source_word(word)
            || matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Symbol(
                    Symbol::Dot | Symbol::ColonColon | Symbol::Apostrophe
                ))
            )
            || matches!(
                next.map(|token| &token.kind),
                Some(TokenKind::Symbol(Symbol::Colon))
            )
        {
            continue;
        }
        result.insert(word.clone());
    }
    result
}

fn is_reserved_source_word(word: &str) -> bool {
    matches!(
        word,
        "let"
            | "idx"
            | "drop"
            | "move"
            | "discharge"
            | "guard"
            | "else"
            | "raise"
            | "if"
            | "match"
            | "while"
            | "invariant"
            | "quant"
            | "bound"
            | "return"
            | "break"
            | "continue"
            | "defer"
            | "error"
            | "region"
            | "catch"
            | "open"
            | "as"
            | "unsafe"
            | "thread"
            | "scope"
            | "footprint"
            | "range"
            | "with"
            | "lock"
            | "spawn"
            | "join"
            | "copy"
            | "read"
            | "write"
            | "share"
            | "mut"
            | "const"
            | "true"
            | "false"
            | "zero"
            | "one"
            | "affine"
            | "relevant"
            | "many"
            | "inf"
            | "FULL"
            | "qcore_acquire"
            | "qcore_check"
    )
}

fn statement_metadata(statement: &StatementKind) -> (Option<&str>, Option<&str>, SemanticPass) {
    match statement {
        StatementKind::Let { binding } => (binding.as_deref(), None, SemanticPass::OrdinaryTypes),
        StatementKind::LetIndex { binding } => (Some(binding), None, SemanticPass::Index),
        StatementKind::Assignment { place } => (None, place.as_deref(), SemanticPass::Ownership),
        StatementKind::Expression => (None, None, SemanticPass::Usage),
        StatementKind::Drop { place } => (None, place.as_deref(), SemanticPass::ResourceFlow),
        StatementKind::Move { place } => (None, place.as_deref(), SemanticPass::Ownership),
        StatementKind::Discharge { binding } => {
            (None, binding.as_deref(), SemanticPass::ErrorRegion)
        }
        StatementKind::Guard => (None, None, SemanticPass::Index),
        StatementKind::If { .. } | StatementKind::Match { .. } | StatementKind::While { .. } => {
            (None, None, SemanticPass::Usage)
        }
        StatementKind::Return => (None, None, SemanticPass::OrdinaryTypes),
        StatementKind::Raise => (None, None, SemanticPass::TypedErrors),
        StatementKind::Break | StatementKind::Continue => (None, None, SemanticPass::ResourceFlow),
        StatementKind::Defer => (None, None, SemanticPass::CleanupLowering),
        StatementKind::ErrorRegion { .. } => (None, None, SemanticPass::ErrorRegion),
        StatementKind::Unsafe { .. } => (None, None, SemanticPass::CEmission),
        StatementKind::ThreadScope { .. }
        | StatementKind::Footprint { .. }
        | StatementKind::WithLock { .. } => (None, None, SemanticPass::Concurrency),
    }
}

/// Converts an error from any established semantic API into a stable,
/// source-located diagnostic.  The individual passes retain their detailed
/// error types; this is the shared Cycle 10 presentation boundary.
pub fn source_diagnostic(
    pass: SemanticPass,
    span: Span,
    error: &(impl std::error::Error + ?Sized),
) -> SourceDiagnostic {
    let code = match pass {
        SemanticPass::NameResolution => "Q010",
        SemanticPass::OrdinaryTypes => "Q020",
        SemanticPass::Index => "Q001",
        SemanticPass::Usage => "Q100",
        SemanticPass::Ownership => "Q103",
        SemanticPass::ResourceFlow => "Q200",
        SemanticPass::TypedErrors => "Q300",
        SemanticPass::ErrorRegion => "Q302",
        SemanticPass::CleanupLowering => "Q202",
        SemanticPass::CEmission => "Q500",
        SemanticPass::Concurrency => "Q600",
    };
    SourceDiagnostic {
        code,
        pass,
        span,
        message: error.to_string(),
    }
}

pub fn covered_semantic_passes(program: &TypedSourceProgram) -> BTreeSet<SemanticPass> {
    fn collect(statement: &TypedStatement, result: &mut BTreeSet<SemanticPass>) {
        result.insert(statement.pass);
        for nested in &statement.nested {
            collect(nested, result);
        }
    }
    let mut result = BTreeSet::new();
    for declaration in &program.declarations {
        result.insert(SemanticPass::NameResolution);
        for statement in &declaration.statements {
            collect(statement, &mut result);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str = r#"dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
type Buffer = own<array<u8, 16>>;
resource type File { repr: i32; state: Open; footprint: { files = 1 }; destructor: close noexcept; copy: forbidden; move: allowed; }
error Io { Failed { code: i32 }; }
const limit: usize = 16;
extern fn close(file: own<File> use one) -> unit effects { unsafe } contract { requires true; };
fn work[file_count: Idx<Count<File>>](file: own<File> use one, flag: bool) -> result<i32, {Io}> effects { throws<Io> } {
  let idx n = limit;
  let alias: borrow<'scope, const, File> = &file;
  guard n <= limit else raise Io::Failed { code: 1 };
  if (flag) { move alias; } else { defer close(move file); }
  match flag { true => { return 1; } false => { raise Io::Failed { code: 2 }; } }
  while (flag) invariant true quant invariant zero == zero bound 1 { continue; }
  error region cleanup { let temp = move file; raise Io::Failed { code: 3 }; } catch (Io::Failed { code }) quant open cleanup as pack { drop temp; discharge pack; }
  unsafe { return 0; }
}
concurrency checked;
fn future(shared: i32) -> unit { footprint cell = shared; thread scope s { let task = spawn s work(copy shared); join move task; } with lock shared as guard { guard.value = 1; } }
"#;

    #[test]
    fn parses_complete_golden_surface_and_preserves_exact_text() {
        let program = parse_source_program(GOLDEN).unwrap();
        assert_eq!(program.declarations.len(), 10);
        assert_eq!(program.to_string(), GOLDEN);
        let reparsed = parse_source_program(&program.to_string()).unwrap();
        assert!(program.syntax_equivalent(&reparsed));
        assert!(
            program
                .declarations
                .iter()
                .all(|declaration| declaration.span.start < declaration.span.end)
        );
    }

    #[test]
    fn classifies_all_sequential_statement_families() {
        let program = parse_source_program(GOLDEN).unwrap();
        let kinds = program
            .declarations
            .iter()
            .filter_map(|declaration| declaration.value.body.as_ref())
            .flat_map(|body| body.statements.iter().map(|statement| &statement.value))
            .collect::<Vec<_>>();
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::LetIndex { .. }))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::Guard))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::If { .. }))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::Match { .. }))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::While { .. }))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::ErrorRegion { .. }))
        );
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, StatementKind::Unsafe { .. }))
        );
    }

    #[test]
    fn classifies_parameter_types_for_typed_source_ir() {
        let source = "fn types(a: i32, b: own<File>, c: borrow<'s, const, File>, d: quant<access, p, FULL>, e: result<i32, {Io}>, f: task<'s, i32, {Io}>, g: mutex<File>) -> unit { return; }";
        let program = parse_source_program(source).unwrap();
        let parameters = &program.declarations[0].value.parameters;
        assert_eq!(
            parameters
                .iter()
                .map(|parameter| parameter.type_kind)
                .collect::<Vec<_>>(),
            vec![
                SourceTypeKind::Atomic,
                SourceTypeKind::Owned,
                SourceTypeKind::Borrow,
                SourceTypeKind::AccessQuant,
                SourceTypeKind::Result,
                SourceTypeKind::Task,
                SourceTypeKind::Mutex,
            ]
        );
        let typed = elaborate_source(&program).unwrap();
        assert_eq!(
            typed.declarations[0].parameters[1].type_kind,
            SourceTypeKind::Owned
        );
    }

    #[test]
    fn elaborates_names_and_attributes_statements_to_passes() {
        let source = "error Problem { Failed; } extern fn cleanup() -> unit; fn consume(x: i32) -> unit { let y: i32 = x; move y; drop x; raise Problem::Failed; defer cleanup(); unsafe { return; } }";
        let program = parse_source_program(source).unwrap();
        let typed = elaborate_source(&program).unwrap();
        let passes = covered_semantic_passes(&typed);
        assert!(passes.contains(&SemanticPass::NameResolution));
        assert!(passes.contains(&SemanticPass::OrdinaryTypes));
        assert!(passes.contains(&SemanticPass::Ownership));
        assert!(passes.contains(&SemanticPass::ResourceFlow));
        assert!(passes.contains(&SemanticPass::TypedErrors));
        assert!(passes.contains(&SemanticPass::CleanupLowering));
        assert!(passes.contains(&SemanticPass::CEmission));
    }

    #[test]
    fn reports_duplicate_and_unknown_names_at_source_spans() {
        let source = "fn bad(x: i32, x: i32) -> unit { move missing; }";
        let program = parse_source_program(source).unwrap();
        let errors = elaborate_source(&program).unwrap_err();
        assert!(errors.iter().any(|error| error.code == "Q011"));
        assert!(errors.iter().any(|error| error.code == "Q013"));
        assert!(errors.iter().all(|error| error.span.end <= source.len()));
    }

    #[test]
    fn rejects_unbalanced_and_unterminated_syntax() {
        assert!(parse_source_program("fn bad() -> unit { return;").is_err());
        assert!(parse_source_program("const x: i32 = 1").is_err());
        assert!(parse_source_program("dimension Bad: Idx;").is_err());
        assert!(parse_source_program("const x = 1;").is_err());
        assert!(parse_source_program("fn missing_return_type() { return; }").is_err());
    }

    #[test]
    fn maps_every_existing_semantic_boundary_to_a_source_diagnostic() {
        #[derive(Debug)]
        struct TestError;
        impl fmt::Display for TestError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("test failure")
            }
        }
        impl std::error::Error for TestError {}

        let passes = [
            SemanticPass::NameResolution,
            SemanticPass::OrdinaryTypes,
            SemanticPass::Index,
            SemanticPass::Usage,
            SemanticPass::Ownership,
            SemanticPass::ResourceFlow,
            SemanticPass::TypedErrors,
            SemanticPass::ErrorRegion,
            SemanticPass::CleanupLowering,
            SemanticPass::CEmission,
            SemanticPass::Concurrency,
        ];
        for pass in passes {
            let diagnostic = source_diagnostic(pass, Span { start: 2, end: 4 }, &TestError);
            assert_eq!(diagnostic.pass, pass);
            assert_eq!(diagnostic.span, Span { start: 2, end: 4 });
            assert!(!diagnostic.code.is_empty());
        }
    }
}
