//! Cycle 10 complete source-front-end skeleton.
//!
//! The earlier cycles intentionally exposed their checked IRs as Rust APIs and
//! parsed only dimensions and quantitative axes.  This module is the common
//! source boundary: it recognizes every declaration and statement family in
//! the revision 0.1 grammar, retains byte spans, resolves bindings, and
//! classifies the resulting nodes for the existing semantic passes.  The
//! revision 0.2 concurrency forms are parsed as well, but are deliberately not
//! certified by the sequential Cycle 10 elaborator.

#[path = "frontend_tasks.rs"]
mod tasks;
pub use tasks::{
    ResolvedTaskCapture, ResolvedTaskExpression, ResolvedTaskExpressionKind, ResolvedTaskPlace,
    SourceCaptureMode, SourceIndexExpression, SourcePlace, SourceShareExpression,
    SourceTaskCapture, SourceTaskExpression, SourceTaskExpressionKind,
};

#[path = "frontend_result_match.rs"]
mod result_match;
pub use result_match::{
    ResolvedResultMatch, ResolvedResultMatchArm, ResolvedResultMatchScrutinee, ResultPatternKind,
    SourceResultMatch, SourceResultMatchArm, SourceResultPattern,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::lexer::{LexError, Span, Symbol, Token, TokenKind, lex};
use qcore_core::usage::Usage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<T> {
    pub span: Span,
    pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclarationKind {
    ConcurrencyProfile,
    VerifierScope,
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
    pub result_type: Option<SourceResultType>,
    pub quant_index_types: Vec<crate::quant_index::SourceQuantIndexType>,
    pub body: Option<SourceBlock>,
    pub function_contract: Option<SourceFunctionContract>,
    pub verifier_scope: Option<VerifierScopeDeclaration>,
    pub tokens: Vec<TokenKind>,
}

/// A source-level function contract retained as structured syntax.  Semantic
/// passes can consume these clauses incrementally without reparsing declaration
/// text, while corpus tests can pin the contract surface independently of the
/// executable subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFunctionContract {
    pub span: Span,
    pub clauses: Vec<Located<SourceContractClause>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceContractClause {
    pub kind: SourceContractClauseKind,
    pub expression_span: Span,
    pub expression_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContractClauseKind {
    Requires,
    Ensures(SourceContractExitSelector),
    Quant(SourceContractExitSelector),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContractExitSelector {
    Normal,
    Error(String),
    Any,
}

/// A source-level selection of verifier providers. The scope is declarative:
/// it grants no authority and is resolved against the registry before a
/// provider is invoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierScopeDeclaration {
    pub providers: Vec<VerifierProviderUse>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierProviderUse {
    pub span: Span,
    pub plugin_id: String,
    pub version: Option<String>,
    pub profile: String,
    pub capabilities: BTreeSet<String>,
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

/// Exact declared result syntax. Retention is structural; only the explicitly
/// recognized executable profile below is admitted by the source driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultType {
    pub kind: SourceTypeKind,
    pub span: Span,
    pub source: String,
    pub tokens: Vec<TokenKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableResultType {
    I32,
    Unit,
}

impl SourceResultType {
    pub fn executable_type(&self) -> Option<ExecutableResultType> {
        match self.tokens.as_slice() {
            [TokenKind::Word(name)] if name == "i32" => Some(ExecutableResultType::I32),
            [TokenKind::Word(name)] if name == "unit" => Some(ExecutableResultType::Unit),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceParameter {
    pub name: Located<String>,
    pub type_kind: SourceTypeKind,
    pub type_span: Span,
    pub type_source: String,
    pub usage: Option<Located<Usage>>,
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

/// File/module compilation intent. Non-executable modes check declarations and
/// supported bodies without selecting or synthesizing a process entry point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceModuleMode {
    #[default]
    Executable,
    Library,
    Contract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    pub mode: SourceModuleMode,
    pub mode_span: Option<Span>,
    imports: Vec<Located<String>>,
    source: String,
    lexical_tokens: Vec<Token>,
    task_expressions: Vec<SourceTaskExpression>,
    result_matches: Vec<SourceResultMatch>,
    tokens: Vec<TokenKind>,
    pub declarations: Vec<Located<SourceDeclaration>>,
}

impl SourceProgram {
    /// Explicit, file-local bundled module imports with their full source spans.
    pub fn imports(&self) -> &[Located<String>] {
        &self.imports
    }

    /// Full match syntax with optional exact builtin Result-pattern recognition.
    pub fn result_matches(&self) -> &[SourceResultMatch] {
        &self.result_matches
    }

    /// Parsed task syntax only; executable checking remains a separate gate.
    pub fn task_expressions(&self) -> &[SourceTaskExpression] {
        &self.task_expressions
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn token_kinds(&self) -> &[TokenKind] {
        &self.tokens
    }

    pub fn lexical_tokens(&self) -> &[Token] {
        &self.lexical_tokens
    }

    pub fn verifier_scopes(&self) -> impl Iterator<Item = (&str, &VerifierScopeDeclaration)> {
        self.declarations.iter().filter_map(|declaration| {
            declaration
                .value
                .verifier_scope
                .as_ref()
                .map(|scope| (declaration.value.name.as_str(), scope))
        })
    }

    /// Round trips compare lexical structure rather than byte offsets.  This
    /// permits callers to format or indent a program without invalidating the
    /// source locations from the original parse.
    pub fn syntax_equivalent(&self, other: &Self) -> bool {
        self.mode == other.mode
            && self.tokens == other.tokens
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
        let mut imports = Vec::new();
        let mut cursor = 0;
        let mut mode = SourceModuleMode::Executable;
        let mut mode_span = None;
        if self.word(0, "module") {
            mode = if self.word(1, "contract") {
                SourceModuleMode::Contract
            } else if self.word(1, "library") {
                SourceModuleMode::Library
            } else if self.word(1, "executable") {
                SourceModuleMode::Executable
            } else {
                return Err(self.error(1, "expected module contract, library, or executable"));
            };
            if self.tokens.get(2).map(|token| &token.kind)
                != Some(&TokenKind::Symbol(Symbol::Semicolon))
            {
                return Err(self.error(2, "expected semicolon after module mode"));
            }
            mode_span = Some(self.range_span(0, 3));
            cursor = 3;
        }
        while cursor < self.tokens.len() {
            if self.word(cursor, "import") {
                let start = cursor;
                cursor += 1;
                let mut path = self.identifier(cursor)?.to_owned();
                cursor += 1;
                while self.tokens.get(cursor).map(|token| &token.kind)
                    == Some(&TokenKind::Symbol(Symbol::Dot))
                {
                    cursor += 1;
                    path.push('.');
                    path.push_str(self.identifier(cursor)?);
                    cursor += 1;
                }
                if self.tokens.get(cursor).map(|token| &token.kind)
                    != Some(&TokenKind::Symbol(Symbol::Semicolon))
                {
                    return Err(self.error(cursor, "expected semicolon after module import"));
                }
                cursor += 1;
                imports.push(Located {
                    span: self.range_span(start, cursor),
                    value: path,
                });
                continue;
            }
            let (declaration, next) = self.declaration(cursor)?;
            declarations.push(declaration);
            cursor = next;
        }
        let task_expressions = tasks::parse_inventory(&self, &declarations)?;
        let result_matches = result_match::parse_inventory(&self, &declarations)?;
        Ok(SourceProgram {
            imports,
            task_expressions,
            result_matches,
            mode,
            mode_span,
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
        } else if self.words(start, &["verify", "scope"]) {
            (DeclarationKind::VerifierScope, Some(start + 2))
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
            DeclarationKind::ResourceType
            | DeclarationKind::ErrorType
            | DeclarationKind::VerifierScope => {
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
        let function_contract = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            self.function_contract(start, body_open.unwrap_or(end))?
        } else {
            None
        };
        let verifier_scope = if kind == DeclarationKind::VerifierScope {
            let open = self.find_symbol(start, Symbol::LeftBrace)?;
            Some(self.verifier_scope(open)?)
        } else {
            None
        };
        self.validate_declaration(kind, start, end)?;
        let parameters = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            self.function_parameters(start, end)?
        } else {
            Vec::new()
        };
        let result_type = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            Some(self.function_result_type(start, body_open.unwrap_or(end - 1))?)
        } else {
            None
        };
        let span = self.range_span(start, end);
        let quant_index_types =
            crate::quant_index::parse_quant_index_types(&self.tokens[start..end])?;
        Ok((
            Located {
                span,
                value: SourceDeclaration {
                    kind,
                    name,
                    parameters,
                    result_type,
                    quant_index_types,
                    body,
                    function_contract,
                    verifier_scope,
                    tokens: self.tokens[start..end]
                        .iter()
                        .map(|token| token.kind.clone())
                        .collect(),
                },
            },
            end,
        ))
    }

    fn function_result_type(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<SourceResultType, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let type_start = self.close(params)? + 2; // skip the validated arrow
        let mut cursor = type_start;
        let quantitative_types =
            crate::quant_index::parse_quant_index_types(&self.tokens[type_start..limit])?;
        let mut angle_depth = 0_u32;
        while cursor < limit {
            // Quantitative predicates can contain comparison `>` tokens. Use
            // their parser's span instead of treating comparisons as brackets.
            if let Some(quantitative) = quantitative_types
                .iter()
                .find(|ty| ty.span.start == self.tokens[cursor].span.start)
            {
                while cursor < limit && self.tokens[cursor].span.start < quantitative.span.end {
                    cursor += 1;
                }
                continue;
            }
            if angle_depth == 0 && (self.word(cursor, "effects") || self.word(cursor, "contract")) {
                break;
            }
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
            } else if self.symbol(cursor, Symbol::Greater) {
                angle_depth = angle_depth
                    .checked_sub(1)
                    .ok_or_else(|| self.error(cursor, "unmatched result type delimiter"))?;
            }
            cursor = self.skip_group(cursor);
        }
        if cursor == type_start || angle_depth != 0 {
            return Err(self.error(
                type_start,
                "function requires a nonempty balanced result type",
            ));
        }
        let span = self.range_span(type_start, cursor);
        Ok(SourceResultType {
            kind: self.type_kind(type_start),
            span,
            source: self.source[span.start..span.end].to_owned(),
            tokens: self.tokens[type_start..cursor]
                .iter()
                .map(|token| token.kind.clone())
                .collect(),
        })
    }

    fn function_contract(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<Option<SourceFunctionContract>, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let mut cursor = self.close(params)? + 1;
        let mut contract = None;
        while cursor < limit {
            if self.word(cursor, "contract") {
                if contract.is_some() {
                    return Err(self.error(cursor, "function has more than one contract block"));
                }
                if !self.symbol(cursor + 1, Symbol::LeftBrace) {
                    return Err(self.error(cursor + 1, "function contract requires a block"));
                }
                let open = cursor + 1;
                let close = self.close(open)?;
                if close >= limit {
                    return Err(self.error(open, "function contract lies outside its signature"));
                }
                contract = Some(self.contract_block(cursor, open, close)?);
                cursor = close + 1;
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        Ok(contract)
    }

    fn contract_block(
        &self,
        contract_start: usize,
        open: usize,
        close: usize,
    ) -> Result<SourceFunctionContract, SourceParseError> {
        let mut clauses = Vec::new();
        let mut cursor = open + 1;
        while cursor < close {
            let clause_start = cursor;
            let (kind, expression_start) = if self.word(cursor, "requires") {
                (SourceContractClauseKind::Requires, cursor + 1)
            } else if self.word(cursor, "ensures") || self.word(cursor, "quant") {
                let is_quant = self.word(cursor, "quant");
                let (selector, next) = self.contract_exit_selector(cursor + 1, close)?;
                let kind = if is_quant {
                    SourceContractClauseKind::Quant(selector)
                } else {
                    SourceContractClauseKind::Ensures(selector)
                };
                (kind, next)
            } else {
                return Err(self.error(
                    cursor,
                    "contract clause must begin with `requires`, `ensures`, or `quant`",
                ));
            };
            let semicolon = self.semicolon_before(expression_start, close)?;
            if expression_start == semicolon {
                return Err(self.error(expression_start, "contract clause expression is empty"));
            }
            let expression_span = self.range_span(expression_start, semicolon);
            clauses.push(Located {
                span: self.range_span(clause_start, semicolon + 1),
                value: SourceContractClause {
                    kind,
                    expression_span,
                    expression_source: self.source[expression_span.start..expression_span.end]
                        .trim()
                        .to_owned(),
                },
            });
            cursor = semicolon + 1;
        }
        Ok(SourceFunctionContract {
            span: self.range_span(contract_start, close + 1),
            clauses,
        })
    }

    fn contract_exit_selector(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(SourceContractExitSelector, usize), SourceParseError> {
        if self.word(start, "normal") {
            return Ok((SourceContractExitSelector::Normal, start + 1));
        }
        if self.word(start, "any") {
            return Ok((SourceContractExitSelector::Any, start + 1));
        }
        if self.word(start, "error") {
            if !self.symbol(start + 1, Symbol::LeftParen) {
                return Err(self.error(start + 1, "error exit selector requires an error type"));
            }
            let close = self.close(start + 1)?;
            if close >= limit || close == start + 2 {
                return Err(self.error(start + 1, "error exit selector requires an error type"));
            }
            let error_span = self.range_span(start + 2, close);
            return Ok((
                SourceContractExitSelector::Error(
                    self.source[error_span.start..error_span.end]
                        .trim()
                        .to_owned(),
                ),
                close + 1,
            ));
        }
        Err(self.error(
            start,
            "contract clause requires `normal`, `error(Type)`, or `any` exit selector",
        ))
    }

    fn verifier_scope(&self, open: usize) -> Result<VerifierScopeDeclaration, SourceParseError> {
        let close = self.close(open)?;
        let mut cursor = open + 1;
        let mut providers = Vec::new();
        let mut seen = BTreeSet::new();
        while cursor < close {
            let entry_start = cursor;
            if !self.word(cursor, "plugin") {
                return Err(self.error(cursor, "verifier scope expects a `plugin` entry"));
            }
            cursor += 1;
            let mut components = Vec::new();
            components.push(self.identifier(cursor)?.to_owned());
            cursor += 1;
            while self.symbol(cursor, Symbol::Dot) {
                cursor += 1;
                components.push(self.identifier(cursor)?.to_owned());
                cursor += 1;
            }
            let plugin_id = components.join(".");
            if components.len() < 2
                || components.iter().any(|component| {
                    component.is_empty()
                        || !component
                            .chars()
                            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
                })
            {
                return Err(self.error(entry_start, "plugin ID must be a lowercase dotted name"));
            }
            let version = if self.word(cursor, "version") {
                cursor += 1;
                let value = match self.tokens.get(cursor).map(|token| &token.kind) {
                    Some(TokenKind::StringLiteral(value)) | Some(TokenKind::Word(value)) => {
                        value.clone()
                    }
                    _ => {
                        return Err(self.error(
                            cursor,
                            "plugin version must be an identifier or string literal",
                        ));
                    }
                };
                cursor += 1;
                Some(value)
            } else {
                None
            };
            if !self.word(cursor, "profile") {
                return Err(self.error(cursor, "plugin entry requires `profile`"));
            }
            cursor += 1;
            let profile = match self.tokens.get(cursor).map(|token| &token.kind) {
                Some(TokenKind::StringLiteral(value)) | Some(TokenKind::Word(value)) => {
                    value.clone()
                }
                _ => return Err(self.error(cursor, "plugin profile must be a name or string")),
            };
            cursor += 1;
            if !self.word(cursor, "capabilities") {
                return Err(self.error(cursor, "plugin entry requires `capabilities`"));
            }
            cursor += 1;
            if !self.symbol(cursor, Symbol::LeftBrace) {
                return Err(self.error(cursor, "plugin capabilities require a set"));
            }
            let capabilities_close = self.close(cursor)?;
            let mut capability_cursor = cursor + 1;
            let mut capabilities = BTreeSet::new();
            while capability_cursor < capabilities_close {
                let capability = self.identifier(capability_cursor)?.to_owned();
                if !capabilities.insert(capability) {
                    return Err(self.error(capability_cursor, "duplicate plugin capability"));
                }
                capability_cursor += 1;
                if capability_cursor < capabilities_close {
                    if !self.symbol(capability_cursor, Symbol::Comma) {
                        return Err(self.error(
                            capability_cursor,
                            "plugin capabilities must be comma-separated",
                        ));
                    }
                    capability_cursor += 1;
                }
            }
            if capabilities.is_empty() {
                return Err(self.error(cursor, "plugin capability set cannot be empty"));
            }
            cursor = capabilities_close + 1;
            if !self.symbol(cursor, Symbol::Semicolon) {
                return Err(self.error(cursor, "plugin entry requires `;`"));
            }
            cursor += 1;
            if !seen.insert((plugin_id.clone(), profile.clone())) {
                return Err(self.error(entry_start, "duplicate plugin/profile entry"));
            }
            providers.push(VerifierProviderUse {
                span: self.range_span(entry_start, cursor),
                plugin_id,
                version,
                profile,
                capabilities,
            });
        }
        if providers.is_empty() {
            return Err(self.error(open, "verifier scope must contain at least one plugin"));
        }
        Ok(VerifierScopeDeclaration { providers })
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
            if let Some(next) = crate::quant_index::quant_type_end(&self.tokens, cursor)? {
                cursor = next;
                continue;
            }
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
            if let Some(next) = crate::quant_index::quant_type_end(&self.tokens, cursor)? {
                cursor = next;
                continue;
            }
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
        let mut suffix = type_end;
        if self.word(suffix, "as") {
            if !self.word(suffix + 1, "idx") || suffix + 2 >= end {
                return Err(self.error(suffix, "index alias requires `as idx NAME`"));
            }
            self.identifier(suffix + 2)?;
            suffix += 3;
        }
        let usage = if suffix < end {
            if !self.word(suffix, "use") {
                return Err(self.error(
                    suffix,
                    "expected a usage annotation after the parameter type",
                ));
            }
            Some(Located {
                span: self.range_span(suffix, end),
                value: self.usage_grade(suffix + 1, end)?,
            })
        } else {
            None
        };
        Ok(SourceParameter {
            name: Located {
                span: self.tokens[start].span,
                value: name,
            },
            type_kind: self.type_kind(type_start),
            type_span,
            type_source: self.source[type_span.start..type_span.end].to_owned(),
            usage,
        })
    }

    fn usage_grade(&self, start: usize, end: usize) -> Result<Usage, SourceParseError> {
        let natural = |index: usize| -> Result<u64, SourceParseError> {
            match self.tokens.get(index).map(|token| &token.kind) {
                Some(TokenKind::Integer(value)) => value
                    .parse()
                    .map_err(|_| self.error(index, "usage bound exceeds the supported u64 range")),
                _ => Err(self.error(index, "usage bounds must be natural integer literals")),
            }
        };
        if end == start + 1 {
            return match self.word_value(start) {
                Some("zero") => Ok(Usage::ZERO),
                Some("one") => Ok(Usage::ONE),
                Some("affine") => Ok(Usage::AFFINE),
                Some("relevant") => Ok(Usage::RELEVANT),
                Some("many") => Ok(Usage::MANY),
                Some(_) => Err(self.error(start, "unknown or unsupported symbolic usage grade")),
                None => natural(start).map(|n| Usage::finite(n, n)),
            };
        }
        if end == start + 5
            && self.symbol(start, Symbol::LeftBracket)
            && self.symbol(start + 2, Symbol::Comma)
            && self.symbol(start + 4, Symbol::RightBracket)
        {
            let lower = natural(start + 1)?;
            if self.word(start + 3, "inf") {
                return Ok(Usage::unbounded(lower));
            }
            let upper = natural(start + 3)?;
            if lower > upper {
                return Err(self.error(start, "usage lower bound exceeds its upper bound"));
            }
            return Ok(Usage::finite(lower, upper));
        }
        Err(self.error(
            start,
            "expected a named grade, natural literal, or literal usage interval",
        ))
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
            DeclarationKind::VerifierScope => {}
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
            || self.word(index, "verify")
            || self.word(index, "import")
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
                    // Statement leaders of the checked mutex-guard profile.
                    // A structured statement's body ends where the next
                    // statement begins, so every leader must be listed here or
                    // `last_top_level_block` would run past an intervening
                    // statement and claim a later block as the body.
                    | "open"
                    | "close"
                    | "read"
                    | "write"
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
    /// Card `source.extern-call.v1`: admission and bridging of a call to a
    /// declared extern function. Owns `Q022`–`Q027` (spec §L21). It is a
    /// separate pass from `OrdinaryTypes` on purpose: `Q020`/`Q021` refuse
    /// *unenforced source contracts* and keep their exact behavior, while this
    /// pass consumes *explicitly selected declared* contracts.
    ExternCall,
    PluginVerification,
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
    pub usage: Option<Located<Usage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedStatement {
    /// Structured arm-local names; not evidence of result typing/exhaustiveness.
    pub result_match: Option<ResolvedResultMatch>,
    /// Identity introduced by this statement, after its initializer resolves.
    /// Assignments retain the existing identity and introduce no new binding.
    pub binding: Option<SymbolId>,
    /// Body-local scope/guard identity, including an empty body.
    pub body_binding: Option<SymbolId>,
    /// Role-specific name resolution; no task lifetime or capture authority claim.
    pub task_expressions: Vec<ResolvedTaskExpression>,
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
    pub result_type: Option<SourceResultType>,
    pub quant_index_types: Vec<crate::quant_index::ResolvedQuantIndexType>,
    pub function_contract: Option<SourceFunctionContract>,
    pub statements: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedSourceProgram {
    pub mode: SourceModuleMode,
    pub declarations: Vec<TypedSourceDeclaration>,
    pub globals: BTreeMap<String, SymbolId>,
    pub origin: String,
}

fn visible_local_names(block: &SourceBlock, position: usize, names: &mut BTreeSet<String>) {
    if position < block.span.start || position >= block.span.end {
        return;
    }
    for statement in &block.statements {
        if statement.span.start > position {
            break;
        }
        match &statement.value {
            StatementKind::Let {
                binding: Some(name),
            }
            | StatementKind::LetIndex { binding: name }
            | StatementKind::Discharge {
                binding: Some(name),
            } => {
                names.insert(name.clone());
            }
            StatementKind::WithLock { guard, body }
            | StatementKind::ThreadScope { name: guard, body }
                if position >= body.span.start && position < body.span.end =>
            {
                names.insert(guard.clone());
            }
            _ => {}
        }
        for child in statement.value.nested_blocks() {
            visible_local_names(child, position, names);
        }
    }
}

/// Placement of one verbatim declaration slice inside the reconstructed
/// dimension/axis source, together with that slice's span in the caller's own
/// source. Used only to attribute unconditional declaration diagnostics.
struct IndexSlice {
    start: usize,
    end: usize,
    origin: Span,
}

/// Maps a span in the reconstructed dimension/axis source back to the caller's
/// source. Offsets inside a slice map byte for byte; a span that starts past
/// every slice (end of the reconstructed buffer) is attributed to the last
/// slice, and an empty slice list keeps the previous degenerate span rather
/// than inventing a location.
fn map_index_span(span: Span, slices: &[IndexSlice]) -> Span {
    let Some(slice) = slices
        .iter()
        .find(|slice| slice.start <= span.start && span.start < slice.end)
        .or_else(|| slices.last())
    else {
        return Span { start: 0, end: 0 };
    };
    let start = slice.origin.start
        + span
            .start
            .saturating_sub(slice.start)
            .min(slice.end - slice.start);
    let end = slice.origin.start
        + span
            .end
            .saturating_sub(slice.start)
            .min(slice.end - slice.start);
    Span {
        start,
        end: end.max(start),
    }
}

pub fn elaborate_source(
    program: &SourceProgram,
) -> Result<TypedSourceProgram, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(program)?;
    let mut diagnostics = Vec::new();
    let mut globals = BTreeMap::new();
    let mut next_symbol = 0_u32;
    for declaration in &program.declarations {
        if matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile | DeclarationKind::VerifierScope
        ) {
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
    let mut index_environment = crate::check::StaticEnvironment::default();
    // Declaration validity is independent of whether a quantitative type uses it.
    {
        // Every slice is a verbatim copy of the original declaration text, so
        // each slice's placement in the reconstructed buffer maps back to the
        // original span byte for byte. Retaining that map is what lets a
        // failure here name the offending declaration instead of reporting the
        // degenerate `0..0` span (which rendered as line 1 of the first input
        // file, and in a multi-file build blamed whichever file was listed
        // first). The accepted/rejected set is unchanged.
        let index_declarations = program
            .declarations
            .iter()
            .filter(|declaration| {
                matches!(
                    declaration.value.kind,
                    DeclarationKind::Dimension | DeclarationKind::Axis
                )
            })
            .collect::<Vec<_>>();
        let mut axis_source = String::new();
        let mut slices: Vec<IndexSlice> = Vec::new();
        for declaration in &index_declarations {
            if !axis_source.is_empty() {
                axis_source.push('\n');
            }
            let start = axis_source.len();
            axis_source.push_str(&program.source[declaration.span.start..declaration.span.end]);
            slices.push(IndexSlice {
                start,
                end: axis_source.len(),
                origin: declaration.span,
            });
        }
        match crate::parser::parse_program(&axis_source) {
            Err(error) => {
                // The parser's offsets address the reconstructed buffer; report
                // the caller's own bytes instead.
                let span = map_index_span(error.span, &slices);
                diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span,
                    message: format!("{} at bytes {}..{}", error.message, span.start, span.end),
                });
            }
            Ok(parsed) => match crate::check::StaticEnvironment::check_located(&parsed) {
                Ok(environment) => index_environment = environment,
                Err((error, index)) => diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span: slices
                        .get(index)
                        .map_or(Span { start: 0, end: 0 }, |slice| slice.origin),
                    message: error.to_string(),
                }),
            },
        }
    }
    for declaration in &program.declarations {
        if matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile | DeclarationKind::VerifierScope
        ) {
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
                usage: parameter.usage.clone(),
            });
        }
        let forbidden_names: BTreeSet<String> = globals
            .keys()
            .cloned()
            .chain(
                declaration
                    .value
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.value.clone()),
            )
            .collect();
        let mut quant_index_types = Vec::new();
        for source_type in &declaration.value.quant_index_types {
            let mut forbidden_names = forbidden_names.clone();
            if let Some(body) = &declaration.value.body {
                visible_local_names(body, source_type.span.start, &mut forbidden_names);
                for syntax in program.result_matches() {
                    for arm in &syntax.arms {
                        if arm.body.span.start <= source_type.span.start
                            && source_type.span.start < arm.body.span.end
                            && let Some(pattern) = &arm.pattern
                        {
                            forbidden_names.insert(pattern.binding.value.clone());
                        }
                    }
                }
            }
            match crate::quant_index::resolve_quant_index_type(
                source_type,
                &index_environment,
                &forbidden_names,
            ) {
                Ok(resolved) => quant_index_types.push(resolved),
                Err(message) => diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span: source_type.span,
                    message,
                }),
            }
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
                    &mut tasks::ResolutionContext {
                        program,
                        live_scopes: BTreeSet::new(),
                    },
                    &mut next_symbol,
                    &mut diagnostics,
                    None,
                )
            });
        declarations.push(TypedSourceDeclaration {
            symbol,
            kind: declaration.value.kind,
            name: declaration.value.name.clone(),
            span: declaration.span,
            parameters,
            result_type: declaration.value.result_type.clone(),
            quant_index_types,
            function_contract: declaration.value.function_contract.clone(),
            statements,
        });
    }
    if diagnostics.is_empty() {
        Ok(TypedSourceProgram {
            mode: program.mode,
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
    context: &mut tasks::ResolutionContext<'_>,
    next_symbol: &mut u32,
    diagnostics: &mut Vec<SourceDiagnostic>,
    body_binding: Option<(&str, bool, SymbolId)>,
) -> Vec<TypedStatement> {
    let mut scope = BTreeMap::new();
    let mut introduced_scope = None;
    if let Some((name, thread_scope, symbol)) = body_binding {
        if thread_scope {
            context.live_scopes.insert(symbol);
            introduced_scope = Some(symbol);
        }
        scope.insert(name.to_owned(), symbol);
    }
    scopes.push(scope);
    let mut typed = Vec::new();
    for statement in &block.statements {
        let (binding, reference, pass) = statement_metadata(&statement.value);
        let mut references = lexical_references(statement, &context.program.lexical_tokens);
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
        let task_expressions =
            tasks::resolve_tasks(statement.span, scopes, globals, context, diagnostics);
        let mut binding_symbol = None;
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
                let symbol = SymbolId(*next_symbol);
                scope.insert(binding.to_owned(), symbol);
                binding_symbol = Some(symbol);
                *next_symbol += 1;
            }
        }
        let body_binding = match &statement.value {
            StatementKind::WithLock { guard, .. } => Some((guard.as_str(), false)),
            StatementKind::ThreadScope { name, .. } => Some((name.as_str(), true)),
            _ => None,
        }
        .map(|(name, thread_scope)| {
            let symbol = SymbolId(*next_symbol);
            *next_symbol += 1;
            (name, thread_scope, symbol)
        });
        let mut nested = Vec::new();
        let match_syntax = context
            .program
            .result_matches()
            .iter()
            .find(|syntax| syntax.span == statement.span)
            .cloned();
        let resolved_match = if let Some(syntax) = match_syntax {
            // Resolve the scrutinee in the enclosing environment before any arm
            // binder exists. Arm scopes and identities are mutually isolated.
            let scrutinee =
                result_match::resolve_scrutinee(&syntax, &task_expressions, scopes, globals);
            let mut arms = Vec::new();
            for arm in &syntax.arms {
                let arm_binding = arm.pattern.as_ref().map(|pattern| {
                    let symbol = SymbolId(*next_symbol);
                    *next_symbol += 1;
                    (pattern.binding.value.as_str(), false, symbol)
                });
                let statements = elaborate_block(
                    &arm.body,
                    scopes,
                    globals,
                    context,
                    next_symbol,
                    diagnostics,
                    arm_binding,
                );
                let start = nested.len();
                nested.extend(statements);
                let statement_range = start..nested.len();
                arms.push(ResolvedResultMatchArm {
                    syntax: arm.clone(),
                    binding: arm_binding.map(|(_, _, symbol)| symbol),
                    statement_range,
                });
            }
            Some(ResolvedResultMatch {
                syntax,
                scrutinee,
                arms,
            })
        } else {
            for child in statement.value.nested_blocks() {
                nested.extend(elaborate_block(
                    child,
                    scopes,
                    globals,
                    context,
                    next_symbol,
                    diagnostics,
                    body_binding,
                ));
            }
            None
        };
        typed.push(TypedStatement {
            result_match: resolved_match,
            binding: binding_symbol,
            body_binding: body_binding.map(|(_, _, symbol)| symbol),
            task_expressions,
            span: statement.span,
            pass,
            referenced_symbols,
            nested,
        });
    }
    scopes.pop();
    if let Some(symbol) = introduced_scope {
        context.live_scopes.remove(&symbol);
    }
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
        // Header binders are definitions in the body scope, not references.
        // Skip the binder token only: `with lock m as m` still resolves the
        // mutex expression against the enclosing scope.
        let body_binder = match &statement.value {
            StatementKind::ThreadScope { .. } => index == 2,
            StatementKind::WithLock { .. } => matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Word(keyword)) if keyword == "as"
            ),
            _ => false,
        };
        if body_binder
            || is_reserved_source_word(word)
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
            // Checked mutex-guard profile keywords. Reserving them keeps
            // `mutex`, `guards`, `protected`, `close` and `borrow` out of name
            // resolution, exactly as `lock`, `open`, `read` and `write` already
            // are.
            | "mutex"
            | "guards"
            | "protected"
            | "close"
            | "borrow"
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
            | "qcore_buffer_new"
            | "qcore_buffer_read"
            | "qcore_buffer_write"
            // Card T06's payload intrinsic. Reserving it keeps a local binding
            // from shadowing the take form, exactly as the buffer intrinsics
            // above are protected.
            | "qcore_error_payload"
            | "qcore_acquire"
            | "qcore_check"
            // Card `source.extern-call.v1`: the literal outcome control of the
            // nullable-owned-return adapter. Reserved so a local binding named
            // `succeeds` cannot shadow the adapter form, exactly as the
            // mutex-guard keywords above are protected.
            | "succeeds"
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
        // The generic presentation code of the extern-call pass. The specific
        // conditions Q023-Q027 are issued at their own emission sites.
        SemanticPass::ExternCall => "Q022",
        SemanticPass::PluginVerification => "Q700",
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
#[path = "../tests/unit/frontend.rs"]
mod tests;
