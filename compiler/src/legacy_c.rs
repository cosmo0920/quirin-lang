//! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
//!
//! The parser intentionally accepts more C than the analyzer models.  Every
//! construct outside the supported structured subset becomes an `Unknown`
//! instruction, so an incomplete model can never produce a `Safe` verdict.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

pub type LegacyNodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceApiKind {
    Acquire,
    Release,
    Escape,
    /// A proven resource-neutral call.  Cycle 13 uses this only for an
    /// internal function whose own summary is retained in the program result.
    Neutral,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceContracts(BTreeMap<String, ResourceApiKind>);

impl ResourceContracts {
    #[must_use]
    pub fn new(contracts: impl IntoIterator<Item = (impl Into<String>, ResourceApiKind)>) -> Self {
        Self(
            contracts
                .into_iter()
                .map(|(name, kind)| (name.into(), kind))
                .collect(),
        )
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: ResourceApiKind) {
        self.0.insert(name.into(), kind);
    }

    fn kind(&self, name: &str) -> Option<ResourceApiKind> {
        self.0.get(name).copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyInstruction {
    Acquire { destination: String, site: usize },
    Alias { destination: String, source: String },
    Clear { destination: String },
    Release { source: String },
    Escape { source: String },
    NoOp,
    Unknown { description: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfgNode {
    pub instruction: LegacyInstruction,
    pub successors: Vec<LegacyNodeId>,
    pub line: usize,
    pub is_exit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfg {
    pub function: String,
    pub entry: LegacyNodeId,
    pub nodes: Vec<LegacyCfgNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegacyDiagnosticKind {
    Leak { acquisition_line: usize },
    DoubleRelease { acquisition_line: usize },
    ReleaseAfterEscape { acquisition_line: usize },
    UnknownAlias { variable: String },
    Unsupported { description: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyDiagnostic {
    pub function: String,
    pub line: usize,
    pub kind: LegacyDiagnosticKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyFunctionSummary {
    pub function: String,
    pub status: AnalysisStatus,
    pub diagnostics: Vec<LegacyDiagnostic>,
    /// Number of work-list transfers used to reach the CFG fixed point.
    pub fixed_point_steps: usize,
    pub cfg: LegacyCfg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyProgramSummary {
    pub entry: String,
    pub status: AnalysisStatus,
    pub functions: BTreeMap<String, LegacyFunctionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyCError {
    FunctionNotFound(String),
    MalformedFunction { function: String, line: usize },
    ProductionParserUnavailable(String),
    ProductionParserRejected(String),
}

impl fmt::Display for LegacyCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionNotFound(function) => write!(f, "C function `{function}` was not found"),
            Self::MalformedFunction { function, line } => {
                write!(
                    f,
                    "malformed body for C function `{function}` at line {line}"
                )
            }
            Self::ProductionParserUnavailable(message) => {
                write!(f, "production C parser is unavailable: {message}")
            }
            Self::ProductionParserRejected(message) => {
                write!(
                    f,
                    "production C parser rejected the translation unit: {message}"
                )
            }
        }
    }
}

impl std::error::Error for LegacyCError {}

/// Parse, import to a resource CFG, convert assignments to SSA-like strong
/// updates, and solve the finite may-alias/resource-state equations.
pub fn analyze_legacy_c(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let tokens = tokenize(source);
    let (body_start, line) = find_function_body(&tokens, function)
        .ok_or_else(|| LegacyCError::FunctionNotFound(function.to_owned()))?;
    let mut parser = Parser {
        tokens: &tokens,
        cursor: body_start,
        contracts,
        next_site: 0,
    };
    let statements =
        parser
            .parse_block()
            .map_err(|failure_line| LegacyCError::MalformedFunction {
                function: function.to_owned(),
                line: failure_line.max(line),
            })?;
    let cfg = CfgBuilder::build(function, &statements);
    Ok(analyze_cfg(cfg))
}

/// Gate the conservative importer with Clang's production C11 parser.  The
/// typed AST accepted by Clang is still deliberately projected into the small
/// resource language below; any projection gap remains `Unknown`.
pub fn analyze_legacy_c11(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let mut child = Command::new("clang")
        .args(["-x", "c", "-std=c11", "-fsyntax-only", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
        .write_all(source.as_bytes())
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    analyze_legacy_c(source, function, contracts)
}

/// Analyze every function definition in a translation unit and combine the
/// per-function results into a bounded interprocedural status summary.
/// Internal calls are resource-neutral at the call site, while the callee's
/// own resource behavior is checked separately and contributes to the result.
pub fn analyze_legacy_c_translation_unit(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyProgramSummary, LegacyCError> {
    let tokens = tokenize(source);
    let names = function_names(&tokens);
    if !names.iter().any(|name| name == entry) {
        return Err(LegacyCError::FunctionNotFound(entry.to_owned()));
    }
    let mut extended = contracts.clone();
    for name in &names {
        extended.insert(name, ResourceApiKind::Neutral);
    }
    let mut functions = BTreeMap::new();
    for name in names {
        functions.insert(name.clone(), analyze_legacy_c(source, &name, &extended)?);
    }
    let status = if functions
        .values()
        .any(|summary| summary.status == AnalysisStatus::Unsafe)
    {
        AnalysisStatus::Unsafe
    } else if functions
        .values()
        .any(|summary| summary.status == AnalysisStatus::Unknown)
    {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    };
    Ok(LegacyProgramSummary {
        entry: entry.to_owned(),
        status,
        functions,
    })
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    line: usize,
}

fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'\n' {
            line += 1;
            cursor += 1;
        } else if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                if bytes[cursor] == b'\n' {
                    line += 1;
                }
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'"' || byte == b'\'' {
            let quote = byte;
            let token_line = line;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    cursor += 1;
                    break;
                } else {
                    if bytes[cursor] == b'\n' {
                        line += 1;
                    }
                    cursor += 1;
                }
            }
            tokens.push(Token {
                text: "__qcore_literal__".to_owned(),
                line: token_line,
            });
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte.is_ascii_digit() {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte == b'#' {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else {
            tokens.push(Token {
                text: char::from(byte).to_string(),
                line,
            });
            cursor += 1;
        }
    }
    tokens
}

fn find_function_body(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) == Some("{") {
            return Some((close + 1, token.line));
        }
    }
    None
}

fn function_names(tokens: &[Token]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if !is_identifier(&token.text)
            || matches!(token.text.as_str(), "if" | "while" | "for" | "switch")
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        if let Some(close) = matching(tokens, index + 1, "(", ")")
            && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
        {
            names.insert(token.text.clone());
        }
    }
    names.into_iter().collect()
}

fn matching(tokens: &[Token], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
enum Statement {
    Instruction(LegacyInstruction, usize),
    Block(Vec<Statement>),
    If {
        line: usize,
        condition_supported: bool,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        line: usize,
        condition_supported: bool,
        body: Vec<Statement>,
    },
    Return {
        line: usize,
        expression_supported: bool,
    },
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    contracts: &'a ResourceContracts,
    next_site: usize,
}

impl Parser<'_> {
    fn parse_block(&mut self) -> Result<Vec<Statement>, usize> {
        let opening_line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("{") {
            return Err(opening_line);
        }
        self.cursor += 1;
        let mut statements = Vec::new();
        while self.cursor < self.tokens.len() && self.text() != Some("}") {
            statements.push(self.parse_statement()?);
        }
        if self.text() != Some("}") {
            return Err(self.tokens.last().map_or(opening_line, |token| token.line));
        }
        self.cursor += 1;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        match self.text() {
            Some("if") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let then_body = self.parse_statement_body()?;
                let else_body = if self.text() == Some("else") {
                    self.cursor += 1;
                    self.parse_statement_body()?
                } else {
                    Vec::new()
                };
                Ok(Statement::If {
                    line,
                    condition_supported: !contains_call(&condition),
                    then_body,
                    else_body,
                })
            }
            Some("while") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let body = self.parse_statement_body()?;
                Ok(Statement::While {
                    line,
                    condition_supported: !contains_call(&condition),
                    body,
                })
            }
            Some("return") => {
                let expression = self.consume_to_semicolon()?;
                Ok(Statement::Return {
                    line,
                    expression_supported: !contains_call(&expression),
                })
            }
            Some("{") => Ok(Statement::Block(self.parse_block()?)),
            Some("for" | "do" | "switch" | "goto" | "break" | "continue") => {
                self.consume_unsupported()?;
                Ok(Statement::Instruction(
                    LegacyInstruction::Unknown {
                        description: "unsupported control flow".to_owned(),
                    },
                    line,
                ))
            }
            Some(_) => {
                let tokens = self.consume_to_semicolon()?;
                Ok(Statement::Instruction(self.classify_simple(&tokens), line))
            }
            None => Err(line),
        }
    }

    fn parse_statement_body(&mut self) -> Result<Vec<Statement>, usize> {
        if self.text() == Some("{") {
            self.parse_block()
        } else {
            Ok(vec![self.parse_statement()?])
        }
    }

    fn take_parenthesized(&mut self) -> Result<Vec<Token>, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("(") {
            return Err(line);
        }
        let close = matching(self.tokens, self.cursor, "(", ")").ok_or(line)?;
        let result = self.tokens[self.cursor + 1..close].to_vec();
        self.cursor = close + 1;
        Ok(result)
    }

    fn consume_to_semicolon(&mut self) -> Result<Vec<Token>, usize> {
        let start = self.cursor;
        let line = self.tokens.get(start).map_or(1, |token| token.line);
        let mut parentheses = 0usize;
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => parentheses += 1,
                Some(")") => parentheses = parentheses.saturating_sub(1),
                Some(";") if parentheses == 0 => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
                Some("{") | Some("}") if parentheses == 0 => return Err(line),
                _ => {}
            }
            self.cursor += 1;
        }
        Err(line)
    }

    fn consume_unsupported(&mut self) -> Result<(), usize> {
        if matches!(self.text(), Some("for" | "switch")) {
            self.cursor += 1;
            self.take_parenthesized()?;
        } else {
            self.cursor += 1;
        }
        if self.text() == Some("{") {
            let line = self.tokens[self.cursor].line;
            self.cursor = matching(self.tokens, self.cursor, "{", "}")
                .map(|index| index + 1)
                .ok_or(line)?;
            Ok(())
        } else {
            self.consume_to_semicolon().map(|_| ())
        }
    }

    fn classify_simple(&mut self, tokens: &[Token]) -> LegacyInstruction {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let assignment = texts.iter().position(|text| *text == "=");
        let call = find_call(&texts);

        if let Some((call_index, function)) = call {
            if let Some(kind) = self.contracts.kind(function) {
                let argument = first_call_argument(&texts, call_index);
                return match kind {
                    ResourceApiKind::Acquire => {
                        let Some(equal) = assignment else {
                            return LegacyInstruction::Unknown {
                                description: format!(
                                    "discarded result of resource API `{function}`"
                                ),
                            };
                        };
                        let Some(destination) = direct_assignment_destination(&texts[..equal])
                        else {
                            return LegacyInstruction::Unknown {
                                description: "unrecognized acquisition destination".to_owned(),
                            };
                        };
                        let site = self.next_site;
                        self.next_site += 1;
                        LegacyInstruction::Acquire {
                            destination: destination.to_owned(),
                            site,
                        }
                    }
                    ResourceApiKind::Release => argument.map_or_else(
                        || LegacyInstruction::Unknown {
                            description: format!("unrecognized argument to `{function}`"),
                        },
                        |source| LegacyInstruction::Release {
                            source: source.to_owned(),
                        },
                    ),
                    ResourceApiKind::Escape => argument.map_or_else(
                        || LegacyInstruction::Unknown {
                            description: format!("unrecognized argument to `{function}`"),
                        },
                        |source| LegacyInstruction::Escape {
                            source: source.to_owned(),
                        },
                    ),
                    ResourceApiKind::Neutral => LegacyInstruction::NoOp,
                };
            }
            return LegacyInstruction::Unknown {
                description: format!("call to uncontracted function `{function}`"),
            };
        }

        if let Some(equal) = assignment {
            let Some(destination) = direct_assignment_destination(&texts[..equal]) else {
                return LegacyInstruction::Unknown {
                    description: "unrecognized assignment destination".to_owned(),
                };
            };
            let right = &texts[equal + 1..];
            if right.len() == 1 && matches!(right[0], "NULL" | "nullptr" | "0") {
                return LegacyInstruction::Clear {
                    destination: destination.to_owned(),
                };
            }
            if let Some(source) = canonical_place(right) {
                return LegacyInstruction::Alias {
                    destination: destination.to_owned(),
                    source,
                };
            }
            return LegacyInstruction::Unknown {
                description: "unsupported assignment expression".to_owned(),
            };
        }

        LegacyInstruction::NoOp
    }

    fn text(&self) -> Option<&str> {
        self.tokens
            .get(self.cursor)
            .map(|token| token.text.as_str())
    }
}

fn find_call<'a>(tokens: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tokens.windows(2).enumerate().find_map(|(index, pair)| {
        (is_identifier(pair[0]) && pair[1] == "(").then_some((index, pair[0]))
    })
}

fn contains_call(tokens: &[Token]) -> bool {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    find_call(&texts).is_some()
}

fn first_call_argument(tokens: &[&str], call_index: usize) -> Option<String> {
    let close = tokens[call_index + 2..]
        .iter()
        .position(|token| *token == ")")?
        + call_index
        + 2;
    let argument = &tokens[call_index + 2..close];
    (!argument.contains(&",")).then(|| canonical_place(argument))?
}

fn direct_assignment_destination(tokens: &[&str]) -> Option<String> {
    if tokens.first() == Some(&"*") {
        return None;
    }
    (0..tokens.len()).find_map(|start| canonical_place(&tokens[start..]))
}

fn canonical_place(tokens: &[&str]) -> Option<String> {
    let first = tokens.first().copied()?;
    if !is_identifier(first) || matches!(first, "NULL" | "nullptr") {
        return None;
    }
    let mut place = first.to_owned();
    let mut cursor = 1;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "." if tokens
                .get(cursor + 1)
                .is_some_and(|field| is_identifier(field)) =>
            {
                place.push('.');
                place.push_str(tokens[cursor + 1]);
                cursor += 2;
            }
            "-" if tokens.get(cursor + 1) == Some(&">")
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|field| is_identifier(field)) =>
            {
                place.push_str("->");
                place.push_str(tokens[cursor + 2]);
                cursor += 3;
            }
            "[" if tokens
                .get(cursor + 1)
                .is_some_and(|index| index.bytes().all(|byte| byte.is_ascii_digit()))
                && tokens.get(cursor + 2) == Some(&"]") =>
            {
                place.push('[');
                place.push_str(tokens[cursor + 1]);
                place.push(']');
                cursor += 3;
            }
            _ => return None,
        }
    }
    Some(place)
}

fn is_identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
}

struct CfgBuilder {
    nodes: Vec<LegacyCfgNode>,
}

impl CfgBuilder {
    fn build(function: &str, statements: &[Statement]) -> LegacyCfg {
        let mut builder = Self { nodes: Vec::new() };
        let exit = builder.push(LegacyInstruction::NoOp, Vec::new(), 0, true);
        let entry = builder.compile_sequence(statements, exit);
        LegacyCfg {
            function: function.to_owned(),
            entry,
            nodes: builder.nodes,
        }
    }

    fn compile_sequence(
        &mut self,
        statements: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        statements
            .iter()
            .rev()
            .fold(continuation, |next, statement| {
                self.compile(statement, next)
            })
    }

    fn compile(&mut self, statement: &Statement, continuation: LegacyNodeId) -> LegacyNodeId {
        match statement {
            Statement::Instruction(instruction, line) => {
                self.push(instruction.clone(), vec![continuation], *line, false)
            }
            Statement::Block(statements) => self.compile_sequence(statements, continuation),
            Statement::Return {
                line,
                expression_supported,
            } => {
                let instruction = if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in return expression".to_owned(),
                    }
                };
                self.push(instruction, Vec::new(), *line, true)
            }
            Statement::If {
                line,
                condition_supported,
                then_body,
                else_body,
            } => {
                let then_entry = self.compile_sequence(then_body, continuation);
                let else_entry = self.compile_sequence(else_body, continuation);
                let instruction = if *condition_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in branch condition".to_owned(),
                    }
                };
                self.push(instruction, vec![then_entry, else_entry], *line, false)
            }
            Statement::While {
                line,
                condition_supported,
                body,
            } => {
                let instruction = if *condition_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in loop condition".to_owned(),
                    }
                };
                let header = self.push(instruction, Vec::new(), *line, false);
                let body_entry = self.compile_sequence(body, header);
                self.nodes[header].successors = vec![body_entry, continuation];
                header
            }
        }
    }

    fn push(
        &mut self,
        instruction: LegacyInstruction,
        successors: Vec<LegacyNodeId>,
        line: usize,
        is_exit: bool,
    ) -> LegacyNodeId {
        let id = self.nodes.len();
        self.nodes.push(LegacyCfgNode {
            instruction,
            successors,
            line,
            is_exit,
        });
        id
    }
}

const LIVE: u8 = 1;
const RELEASED: u8 = 2;
const ESCAPED: u8 = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractPointer {
    targets: BTreeSet<usize>,
    unknown: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractState {
    pointers: BTreeMap<String, AbstractPointer>,
    resources: BTreeMap<usize, u8>,
    widened: bool,
}

const MAX_ALIAS_TARGETS: usize = 8;

impl AbstractState {
    fn join(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        let variables = self
            .pointers
            .keys()
            .chain(incoming.pointers.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for variable in variables {
            match (
                self.pointers.get(&variable).cloned(),
                incoming.pointers.get(&variable),
            ) {
                (Some(mut current), Some(other)) => {
                    current.targets.extend(other.targets.iter().copied());
                    current.unknown |= other.unknown;
                    if current.targets.len() > MAX_ALIAS_TARGETS {
                        current.targets.clear();
                        current.unknown = true;
                        self.widened = true;
                    }
                    self.pointers.insert(variable, current);
                }
                (Some(mut current), None) => {
                    current.unknown = true;
                    self.pointers.insert(variable, current);
                }
                (None, Some(other)) => {
                    let mut joined = other.clone();
                    joined.unknown = true;
                    self.pointers.insert(variable, joined);
                }
                (None, None) => {}
            }
        }
        for (&site, &status) in &incoming.resources {
            self.resources
                .entry(site)
                .and_modify(|current| *current |= status)
                .or_insert(status);
        }
        self.widened |= incoming.widened;
        *self != before
    }
}

fn analyze_cfg(cfg: LegacyCfg) -> LegacyFunctionSummary {
    let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(AbstractState::default());
    let mut work = VecDeque::from([cfg.entry]);
    let mut diagnostics = BTreeSet::new();
    let mut steps = 0usize;
    let acquisition_lines = cfg
        .nodes
        .iter()
        .filter_map(|node| match node.instruction {
            LegacyInstruction::Acquire { site, .. } => Some((site, node.line)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();

    while let Some(node_id) = work.pop_front() {
        steps += 1;
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &mut diagnostics,
        );
        if node.is_exit {
            report_leaks(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &mut diagnostics,
            );
        }
        for &successor in &node.successors {
            let changed = match &mut incoming[successor] {
                Some(current) => current.join(&output),
                slot @ None => {
                    *slot = Some(output.clone());
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }

    if incoming.iter().flatten().any(|state| state.widened) {
        diagnostics.insert(LegacyDiagnostic {
            function: cfg.function.clone(),
            line: 0,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "alias set exceeded {MAX_ALIAS_TARGETS} targets and was widened"
                ),
            },
        });
    }

    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let unsafe_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::Leak { .. }
                | LegacyDiagnosticKind::DoubleRelease { .. }
                | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
        )
    });
    let unknown_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { .. } | LegacyDiagnosticKind::Unsupported { .. }
        )
    });
    let status = if unsafe_found {
        AnalysisStatus::Unsafe
    } else if unknown_found {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    };
    LegacyFunctionSummary {
        function: cfg.function.clone(),
        status,
        diagnostics,
        fixed_point_steps: steps,
        cfg,
    }
}

fn transfer(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    match &node.instruction {
        LegacyInstruction::Acquire { destination, site } => {
            state.resources.insert(*site, LIVE);
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([*site]),
                    unknown: false,
                },
            );
        }
        LegacyInstruction::Alias {
            destination,
            source,
        } => {
            let pointer = state.pointers.get(source).cloned().unwrap_or_else(|| {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.clone(),
                    },
                });
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                }
            });
            state.pointers.insert(destination.clone(), pointer);
        }
        LegacyInstruction::Clear { destination } => {
            state
                .pointers
                .insert(destination.clone(), AbstractPointer::default());
        }
        LegacyInstruction::Release { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                false,
            );
        }
        LegacyInstruction::Escape { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                true,
            );
        }
        LegacyInstruction::Unknown { description } => {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: description.clone(),
                },
            });
            for pointer in state.pointers.values_mut() {
                pointer.unknown = true;
            }
        }
        LegacyInstruction::NoOp => {}
    }
    state
}

fn apply_resource_action(
    function: &str,
    line: usize,
    source: &str,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    escape: bool,
) {
    let pointer = state
        .pointers
        .get(source)
        .cloned()
        .unwrap_or(AbstractPointer {
            targets: BTreeSet::new(),
            unknown: true,
        });
    if pointer.unknown || pointer.targets.is_empty() {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    for site in pointer.targets {
        let status = state.resources.get(&site).copied().unwrap_or(0);
        if escape {
            state.resources.insert(site, (status & !LIVE) | ESCAPED);
        } else {
            if status & RELEASED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::DoubleRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            if status & ESCAPED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::ReleaseAfterEscape {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            state.resources.insert(site, (status & !LIVE) | RELEASED);
        }
    }
}

fn acquisition_line(acquisition_lines: &BTreeMap<usize, usize>, site: usize) -> usize {
    acquisition_lines.get(&site).copied().unwrap_or(0)
}

fn report_leaks(
    function: &str,
    line: usize,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &status) in &state.resources {
        if status & LIVE != 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Leak {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contracts() -> ResourceContracts {
        ResourceContracts::new([
            ("q_alloc", ResourceApiKind::Acquire),
            ("q_free", ResourceApiKind::Release),
            ("q_transfer", ResourceApiKind::Escape),
        ])
    }

    fn analyze(body: &str) -> LegacyFunctionSummary {
        analyze_legacy_c(body, "subject", &contracts()).unwrap()
    }

    fn has(summary: &LegacyFunctionSummary, expected: fn(&LegacyDiagnosticKind) -> bool) -> bool {
        summary
            .diagnostics
            .iter()
            .any(|diagnostic| expected(&diagnostic.kind))
    }

    #[test]
    fn safe_annotated_program_reaches_a_safe_fixed_point() {
        let summary =
            analyze("void subject(void) {\n void *p = q_alloc();\n q_free(p);\n return;\n}");
        assert_eq!(summary.status, AnalysisStatus::Safe);
        assert!(summary.fixed_point_steps >= 3);
    }

    #[test]
    fn finds_a_seeded_normal_exit_leak() {
        let summary = analyze("void subject(void) {\n void *p = q_alloc();\n return;\n}");
        assert_eq!(summary.status, AnalysisStatus::Unsafe);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )));
    }

    #[test]
    fn finds_a_seeded_double_free_through_an_alias() {
        let summary = analyze(
            "void subject(void) {\n void *p = q_alloc();\n void *q = p;\n q_free(p);\n q_free(q);\n}",
        );
        assert_eq!(summary.status, AnalysisStatus::Unsafe);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::DoubleRelease { .. }
        )));
    }

    #[test]
    fn finds_a_seeded_error_path_leak() {
        let summary = analyze(
            "int subject(int fail) {\n void *p = q_alloc();\n if (fail) { return -1; }\n q_free(p);\n return 0;\n}",
        );
        assert_eq!(summary.status, AnalysisStatus::Unsafe);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )));
    }

    #[test]
    fn branch_cleanup_on_both_paths_is_safe() {
        let summary = analyze(
            "int subject(int fail) {\n void *p = q_alloc();\n if (fail) { q_free(p); return -1; } else { q_free(p); return 0; }\n}",
        );
        assert_eq!(summary.status, AnalysisStatus::Safe);
    }

    #[test]
    fn loop_analysis_terminates_at_a_conservative_fixed_point() {
        let summary = analyze(
            "void subject(int again) {\n void *p = q_alloc();\n while (again) { q_free(p); }\n}",
        );
        assert_eq!(summary.status, AnalysisStatus::Unsafe);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::DoubleRelease { .. }
        )));
    }

    #[test]
    fn ownership_transfer_discharges_the_local_obligation() {
        let summary =
            analyze("void subject(void) {\n void *p = q_alloc();\n q_transfer(p);\n return;\n}");
        assert_eq!(summary.status, AnalysisStatus::Safe);
    }

    #[test]
    fn uncontracted_calls_report_unknown_instead_of_safe() {
        let summary = analyze("void subject(void) {\n void *p = mystery();\n return;\n}");
        assert_eq!(summary.status, AnalysisStatus::Unknown);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { .. }
        )));
    }

    #[test]
    fn unresolved_release_alias_reports_unknown() {
        let summary = analyze("void subject(void) {\n q_free(p);\n}");
        assert_eq!(summary.status, AnalysisStatus::Unknown);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
        )));
    }

    #[test]
    fn calls_in_conditions_and_returns_cannot_be_silently_accepted() {
        let condition = analyze("int subject(void) { if (mystery()) { return 1; } return 0; }");
        assert_eq!(condition.status, AnalysisStatus::Unknown);

        let returned = analyze("void *subject(void) { return mystery(); }");
        assert_eq!(returned.status, AnalysisStatus::Unknown);
    }

    #[test]
    fn indirect_pointer_expressions_cannot_be_silently_accepted() {
        let arithmetic =
            analyze("void subject(void) { void *p = q_alloc(); p = p + 1; q_free(p); return; }");
        assert_eq!(arithmetic.status, AnalysisStatus::Unknown);

        let field =
            analyze("void subject(void) { void *p = q_alloc(); q_free(p->field); return; }");
        assert_eq!(field.status, AnalysisStatus::Unsafe);
        assert!(has(&field, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p->field"
        )));
    }

    #[test]
    fn standalone_compound_blocks_are_imported() {
        let summary = analyze("void subject(void) { { void *p = q_alloc(); q_free(p); } return; }");
        assert_eq!(summary.status, AnalysisStatus::Safe);
    }

    #[test]
    fn malformed_and_missing_functions_are_parse_errors() {
        assert!(matches!(
            analyze_legacy_c("void other(void) {}", "subject", &contracts()),
            Err(LegacyCError::FunctionNotFound(_))
        ));
        assert!(matches!(
            analyze_legacy_c("void subject(void) { q_free(p);", "subject", &contracts()),
            Err(LegacyCError::MalformedFunction { .. })
        ));
    }

    #[test]
    fn imports_struct_fields_and_constant_array_elements() {
        let summary = analyze(
            "struct Pair { void *left; void *right[2]; };\nvoid subject(void) {\n struct Pair p;\n p.left = q_alloc();\n p.right[0] = q_alloc();\n q_free(p.right[0]);\n q_free(p.left);\n}",
        );
        assert_eq!(summary.status, AnalysisStatus::Safe);
    }

    #[test]
    fn translation_unit_summary_propagates_callee_defects() {
        let source = "void helper(void) { void *p = q_alloc(); return; }\nvoid subject(void) { helper(); return; }";
        let summary = analyze_legacy_c_translation_unit(source, "subject", &contracts()).unwrap();
        assert_eq!(summary.status, AnalysisStatus::Unsafe);
        assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
        assert_eq!(summary.functions["helper"].status, AnalysisStatus::Unsafe);
    }

    #[test]
    fn documented_alias_widening_can_only_lose_a_safe_result() {
        let mut source = String::from("void subject(int c) {\n");
        for index in 0..=8 {
            source.push_str(&format!(" void *p{index} = q_alloc();\n"));
        }
        source.push_str(" void *q = p0;\n");
        for index in 1..=8 {
            source.push_str(&format!(" if (c) q = p{index};\n"));
        }
        for index in 0..=8 {
            source.push_str(&format!(" q_free(p{index});\n"));
        }
        source.push_str("}\n");
        let summary = analyze(&source);
        assert_eq!(summary.status, AnalysisStatus::Unknown);
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { description }
                if description.contains("widened")
        )));
    }

    #[test]
    fn clang_gateway_rejects_invalid_c_before_projection() {
        let source = "void *q_alloc(void); void q_free(void *); void subject(void) { void *p = q_alloc(); q_free(p); }";
        match analyze_legacy_c11(source, "subject", &contracts()) {
            Ok(summary) => assert_eq!(summary.status, AnalysisStatus::Safe),
            Err(LegacyCError::ProductionParserUnavailable(_)) => return,
            Err(error) => panic!("unexpected production parser result: {error}"),
        }
        assert!(matches!(
            analyze_legacy_c11("void subject( {", "subject", &contracts()),
            Err(LegacyCError::ProductionParserRejected(_))
                | Err(LegacyCError::ProductionParserUnavailable(_))
        ));
    }
}
