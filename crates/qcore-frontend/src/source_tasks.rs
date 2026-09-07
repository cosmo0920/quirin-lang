//! Exact source lowering for bounded copy-scalar and owned-buffer scoped tasks.
//!
//! Structural task records are not admitted on their own: this pass consumes
//! complete statements, checks every declaration and resolves immutable slots.
//! The returned core IR still requires independent `check_task_module` validation.
use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};

/// Detect source task forms in every definition, including empty/nested scopes
/// and uncalled bodies. This selects a checker, never grants execution support.
pub fn has_source_tasks(source: &SourceProgram) -> bool {
    fn has_scope(block: &SourceBlock) -> bool {
        block.statements.iter().any(|s| {
            matches!(s.value, StatementKind::ThreadScope { .. })
                || s.value.nested_blocks().iter().any(|b| has_scope(b))
        })
    }
    source.lexical_tokens().iter().any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "qcore_buffer_new" || word == "qcore_buffer_read"))
        || !source.task_expressions().is_empty()
        || source
            .declarations
            .iter()
            .filter_map(|d| d.value.body.as_ref())
            .any(has_scope)
}

fn failure(span: Span, pass: SemanticPass, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: match pass {
            SemanticPass::NameResolution => "Q013",
            SemanticPass::OrdinaryTypes => "Q020",
            SemanticPass::TypedErrors => "Q300",
            SemanticPass::Concurrency => "Q600",
            _ => "Q500",
        },
        pass,
        span,
        message: message.into(),
    }
}

fn tokens_for(source: &SourceProgram, span: Span) -> Vec<Token> {
    source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect()
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

fn matching(tokens: &[Token], start: usize, open: Symbol, close: Symbol) -> Option<usize> {
    if !symbol(tokens.get(start), open) {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if symbol(Some(token), open) {
            depth += 1;
        }
        if symbol(Some(token), close) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
struct ParsedEffects {
    alloc: bool,
    spawn: bool,
    join: bool,
    may_block: bool,
    errors: BTreeSet<String>,
    /// Card T06. The declared subset of `errors` whose variants carry exactly
    /// one `own<Buffer>`. It is derived from the error type declaration, never
    /// from a use site, so one tag cannot carry a payload in one function and
    /// not in another.
    error_payloads: BTreeSet<String>,
}

/// A declared nominal error type: its complete variant set and the subset that
/// carries one owned Buffer.
#[derive(Clone, Debug, Default)]
struct ErrorType {
    variants: BTreeSet<String>,
    payloads: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    OwnedBuffer,
    /// `borrow<Buffer>`: a scoped read loan. Only a worker parameter can have
    /// this type, and only a `read … share FULL` capture can produce one.
    ReadLoan,
    I32,
    Unit,
}

fn scalar(tokens: &[Token]) -> Option<Scalar> {
    match tokens {
        [own, lt, buffer, gt]
            if word(Some(own), "own")
                && symbol(Some(lt), Symbol::Less)
                && word(Some(buffer), "Buffer")
                && symbol(Some(gt), Symbol::Greater) =>
        {
            Some(Scalar::OwnedBuffer)
        }
        [borrow, lt, buffer, gt]
            if word(Some(borrow), "borrow")
                && symbol(Some(lt), Symbol::Less)
                && word(Some(buffer), "Buffer")
                && symbol(Some(gt), Symbol::Greater) =>
        {
            Some(Scalar::ReadLoan)
        }
        [t] if word(Some(t), "i32") => Some(Scalar::I32),
        [t] if word(Some(t), "unit") => Some(Scalar::Unit),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct Signature {
    id: SymbolId,
    name: String,
    parameters: Vec<(SymbolId, String, Scalar)>,
    result: Scalar,
    effects: ParsedEffects,
}

fn error_declarations(
    source: &SourceProgram,
) -> Result<BTreeMap<String, ErrorType>, SourceDiagnostic> {
    let builtin = |tag: &str| ErrorType {
        variants: BTreeSet::from([tag.to_owned()]),
        // Both are raised by the runtime after a failure that acquired
        // nothing, so neither can ever hold a payload.
        payloads: BTreeSet::new(),
    };
    let mut errors = BTreeMap::from([
        ("AllocError".to_owned(), builtin("AllocError")),
        ("SpawnError".to_owned(), builtin("SpawnError")),
    ]);
    for declaration in &source.declarations {
        if declaration.value.kind != DeclarationKind::ErrorType {
            continue;
        }
        let tokens = tokens_for(source, declaration.span);
        let bad = || {
            failure(
                declaration.span,
                SemanticPass::TypedErrors,
                "task execution supports nominal error variants that are payload-free or carry exactly one own<Buffer>",
            )
        };
        if tokens.len() < 6
            || !word(tokens.first(), "error")
            || !symbol(tokens.get(2), Symbol::LeftBrace)
            || !symbol(tokens.last(), Symbol::RightBrace)
            || declaration.value.name == "SpawnError"
        {
            return Err(bad());
        }
        let mut declared = ErrorType::default();
        let inner = &tokens[3..tokens.len() - 1];
        let mut cursor = 0;
        while cursor < inner.len() {
            let Some(variant) = name(inner.get(cursor)) else {
                return Err(bad());
            };
            let tag = format!("{}::{variant}", declaration.value.name);
            // `V;` is payload-free; `V(own<Buffer>);` carries exactly one owned
            // Buffer. Nothing else is admitted, so a scalar, a borrow, several
            // fields and an empty list each reject rather than degrade.
            let payload = if symbol(inner.get(cursor + 1), Symbol::Semicolon) {
                cursor += 2;
                false
            } else if symbol(inner.get(cursor + 1), Symbol::LeftParen)
                && inner.len() > cursor + 7
                && scalar(&inner[cursor + 2..cursor + 6]) == Some(Scalar::OwnedBuffer)
                && symbol(inner.get(cursor + 6), Symbol::RightParen)
                && symbol(inner.get(cursor + 7), Symbol::Semicolon)
            {
                cursor += 8;
                true
            } else {
                return Err(bad());
            };
            if !declared.variants.insert(tag.clone()) {
                return Err(bad());
            }
            if payload {
                declared.payloads.insert(tag);
            }
        }
        if declared.variants.is_empty()
            || errors
                .insert(declaration.value.name.clone(), declared)
                .is_some()
        {
            return Err(bad());
        }
    }
    Ok(errors)
}

fn signature(
    source: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed: &TypedSourceDeclaration,
    errors: &BTreeMap<String, ErrorType>,
) -> Result<Signature, SourceDiagnostic> {
    let bad = |message| failure(declaration.span, SemanticPass::OrdinaryTypes, message);
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| bad("task functions require source bodies"))?;
    if declaration.value.function_contract.is_some() {
        return Err(bad(
            "source contracts require an implemented consumer for task execution",
        ));
    }
    let header_span = Span {
        start: declaration.span.start,
        end: body.span.start,
    };
    let tokens = tokens_for(source, header_span);
    if !word(tokens.first(), "fn")
        || name(tokens.get(1)) != Some(declaration.value.name.as_str())
        || !symbol(tokens.get(2), Symbol::LeftParen)
    {
        return Err(bad(
            "generic/unsupported function headers are outside task execution",
        ));
    }
    let close = matching(&tokens, 2, Symbol::LeftParen, Symbol::RightParen)
        .ok_or_else(|| bad("invalid task parameter list"))?;
    let groups = if close == 3 {
        Vec::new()
    } else {
        tokens[3..close]
            .split(|token| symbol(Some(token), Symbol::Comma))
            .collect::<Vec<_>>()
    };
    if groups.len() != typed.parameters.len() {
        return Err(bad("task parameter metadata disagrees with the source"));
    }
    let mut parameters = Vec::new();
    for (group, parameter) in groups.iter().zip(&typed.parameters) {
        if name(group.first()) != Some(parameter.name.as_str())
            || !symbol(group.get(1), Symbol::Colon)
        {
            return Err(bad("task parameters require explicit types"));
        }
        let kind = if group.len() == 3 && word(group.get(2), "i32") && parameter.usage.is_none() {
            Scalar::I32
        } else if group.len() == 8
            && scalar(&group[2..6]) == Some(Scalar::OwnedBuffer)
            && word(group.get(6), "use")
            && word(group.get(7), "one")
            && parameter
                .usage
                .as_ref()
                .is_some_and(|usage| usage.value == qcore_core::usage::Usage::ONE)
        {
            Scalar::OwnedBuffer
        } else if group.len() == 6
            && scalar(&group[2..6]) == Some(Scalar::ReadLoan)
            && parameter.usage.is_none()
        {
            // A read loan carries no usage grade in this profile: the borrower
            // may read it any number of times and may do nothing else with it.
            // Grading loan reads is deliberately left open.
            Scalar::ReadLoan
        } else {
            return Err(bad(
                "task parameters require i32, own<Buffer> use one, or an ungraded borrow<Buffer>",
            ));
        };
        parameters.push((parameter.symbol, parameter.name.clone(), kind));
    }
    if !symbol(tokens.get(close + 1), Symbol::Arrow) {
        return Err(bad("task functions require an explicit result type"));
    }
    let result_end = tokens
        .iter()
        .enumerate()
        .skip(close + 2)
        .find_map(|(i, t)| word(Some(t), "effects").then_some(i))
        .unwrap_or(tokens.len());
    let result = scalar(&tokens[close + 2..result_end])
        .filter(|kind| *kind != Scalar::ReadLoan)
        .ok_or_else(|| bad("task functions support i32, unit and own<Buffer> results"))?;
    let mut cursor = result_end;
    let mut effects = ParsedEffects::default();
    if cursor < tokens.len() {
        if !word(tokens.get(cursor), "effects")
            || !symbol(tokens.get(cursor + 1), Symbol::LeftBrace)
        {
            return Err(bad("unsupported function annotation in task execution"));
        }
        let last = matching(&tokens, cursor + 1, Symbol::LeftBrace, Symbol::RightBrace)
            .ok_or_else(|| bad("invalid task effect row"))?;
        if last + 1 != tokens.len() {
            return Err(bad(
                "trailing function annotations are not supported by task execution",
            ));
        }
        cursor += 2;
        let mut seen = BTreeSet::new();
        while cursor < last {
            let effect = name(tokens.get(cursor)).ok_or_else(|| bad("invalid task effect"))?;
            let key;
            match effect {
                "alloc" => {
                    effects.alloc = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "spawn" => {
                    effects.spawn = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "join" => {
                    effects.join = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "may_block" => {
                    effects.may_block = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "throws" => {
                    if !symbol(tokens.get(cursor + 1), Symbol::Less)
                        || !symbol(tokens.get(cursor + 3), Symbol::Greater)
                    {
                        return Err(bad("throws requires one declared nominal error type"));
                    }
                    let error = name(tokens.get(cursor + 2))
                        .ok_or_else(|| bad("throws requires a nominal error type"))?;
                    let declared = errors
                        .get(error)
                        .ok_or_else(|| bad("throws references an unknown error type"))?;
                    effects.errors.extend(declared.variants.iter().cloned());
                    effects
                        .error_payloads
                        .extend(declared.payloads.iter().cloned());
                    key = format!("throws<{error}>");
                    cursor += 4;
                }
                _ => return Err(bad("unsupported effect in task execution")),
            }
            if !seen.insert(key) {
                return Err(bad("duplicate task effect"));
            }
            if cursor == last {
                break;
            }
            if !symbol(tokens.get(cursor), Symbol::Comma) {
                return Err(bad("task effects must be separated by commas"));
            }
            cursor += 1;
            if cursor == last {
                return Err(bad("missing effect after comma"));
            }
        }
    }
    // Card T06's bound on the *declared row*, not on a path. When some declared
    // variant carries a payload, every one of them must, so a joined error
    // always holds exactly one Buffer and the caller's take is total. A mixed
    // row would make the payload's presence depend on which variant occurred,
    // which this profile does not represent. It is a refusal, not a silent
    // widening: the core would still reject the take on the payload-free path.
    if !effects.error_payloads.is_empty() && effects.error_payloads != effects.errors {
        return Err(failure(
            declaration.span,
            SemanticPass::TypedErrors,
            "a declared error row must carry an owned payload on every variant or on none",
        ));
    }
    Ok(Signature {
        id: typed.symbol,
        name: typed.name.clone(),
        parameters,
        result,
        effects,
    })
}

use qcore_core::task_execution::{
    ErrorOperand, ScalarOperand, ScalarType, TaskBlock, TaskEffects, TaskFunction, TaskInstruction,
    TaskInstructionKind, TaskModule, TaskSpan, TaskTerminator, TaskTerminatorKind,
};

impl Scalar {
    fn core(self) -> ScalarType {
        match self {
            Self::OwnedBuffer => ScalarType::OwnedBuffer,
            Self::ReadLoan => ScalarType::ReadLoan,
            Self::I32 => ScalarType::I32,
            Self::Unit => ScalarType::Unit,
        }
    }
}

fn core_span(span: Span) -> TaskSpan {
    TaskSpan {
        start: span.start,
        end: span.end,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Scalar(Scalar),
    Handle(SymbolId),
    Result {
        value: Scalar,
        errors: BTreeSet<String>,
        /// True when some variant this Result can carry declares an owned
        /// Buffer payload. Such a Result cannot be propagated with `?`: its
        /// payload has to be taken on an explicit error arm.
        error_payload: bool,
    },
    Scope,
    KnownError(String),
    Error {
        errors: BTreeSet<String>,
        /// True until `qcore_error_payload` takes the Buffer out. While it is
        /// true the binding cannot be re-raised or silently dropped.
        payload: bool,
    },
}

/// Lower complete source syntax into task execution IR. This checks the source
/// import/type boundary; the resulting IR requires `check_task_module` before
/// emission or a source-execution claim.
pub fn lower_source_tasks(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<TaskModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry).map_err(|error| vec![error])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<TaskModule, SourceDiagnostic> {
    let span = Span {
        start: 0,
        end: source.source().len(),
    };
    if source.mode != SourceModuleMode::Executable {
        return Err(failure(
            span,
            SemanticPass::CEmission,
            "entry-free task execution checking is not implemented",
        ));
    }
    let reparsed = parse_source_program(source.source())
        .map_err(|error| failure(error.span, SemanticPass::NameResolution, error.message))?;
    if reparsed != *source {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task source metadata differs from independent parsing",
        ));
    }
    let retyped = elaborate_source(source).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .expect("elaboration rejection has diagnostics")
    })?;
    if retyped != *typed {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task symbol metadata differs from independent elaboration",
        ));
    }
    if source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::ConcurrencyProfile)
        .count()
        != 1
    {
        return Err(failure(
            span,
            SemanticPass::Concurrency,
            "task execution requires exactly one `concurrency checked;` marker",
        ));
    }
    for declaration in &source.declarations {
        if matches!(
            declaration.value.name.as_str(),
            "qcore_buffer_new" | "qcore_buffer_read" | "qcore_error_payload" | "AllocError"
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::NameResolution,
                "reserved owned-buffer intrinsic or error name",
            ));
        }
        if !matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile
                | DeclarationKind::Function
                | DeclarationKind::ErrorType
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::CEmission,
                "declaration is outside the copy-scalar task execution profile",
            ));
        }
    }
    let errors = error_declarations(source)?;
    let mut signatures = BTreeMap::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::Function)
    {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .expect("independent elaboration preserved functions");
        let signature = signature(source, declaration, resolved, &errors)?;
        signatures.insert(signature.id, signature);
    }
    let entry = typed
        .globals
        .get(entry)
        .copied()
        .filter(|symbol| signatures.contains_key(symbol))
        .ok_or_else(|| {
            failure(
                span,
                SemanticPass::NameResolution,
                "task entry must name a source function",
            )
        })?;
    if !signatures[&entry].parameters.is_empty() {
        return Err(failure(
            span,
            SemanticPass::OrdinaryTypes,
            "task execution entry must have no parameters",
        ));
    }
    let mut functions = BTreeMap::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::Function)
    {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .unwrap();
        let signature = &signatures[&resolved.symbol];
        let body = declaration.value.body.as_ref().unwrap();
        let mut builder = Builder::new(
            source,
            &signatures,
            signature,
            &errors,
            next_temporary(resolved)?,
        );
        if !builder.block(body, &resolved.statements)? {
            return Err(failure(
                body.span,
                SemanticPass::OrdinaryTypes,
                "task functions require an explicit return or raise on every path",
            ));
        }
        functions.insert(
            resolved.symbol.0 as usize,
            TaskFunction {
                name: signature.name.clone(),
                parameters: signature
                    .parameters
                    .iter()
                    .map(|(symbol, _, kind)| (symbol.0 as usize, kind.core()))
                    .collect(),
                result: signature.result.core(),
                errors: signature.effects.errors.clone(),
                error_payloads: signature.effects.error_payloads.clone(),
                declared_effects: TaskEffects {
                    alloc: signature.effects.alloc,
                    spawn: signature.effects.spawn,
                    join: signature.effects.join,
                    may_block: signature.effects.may_block,
                },
                entry: 0,
                blocks: builder.blocks,
                span: core_span(declaration.span),
            },
        );
    }
    // A worker cannot recursively or transitively create tasks in this first
    // executable profile. All source functions were still lowered above.
    let workers = functions
        .values()
        .flat_map(|function| {
            function
                .blocks
                .values()
                .filter_map(|block| match block.terminator.kind {
                    TaskTerminatorKind::TrySpawn { worker, .. } => Some(worker),
                    _ => None,
                })
        })
        .collect::<BTreeSet<_>>();
    for worker in workers {
        if functions[&worker]
            .blocks
            .values()
            .any(|block| matches!(block.terminator.kind, TaskTerminatorKind::TrySpawn { .. }))
        {
            let span = functions[&worker].span;
            return Err(failure(
                Span {
                    start: span.start,
                    end: span.end,
                },
                SemanticPass::Concurrency,
                "spawned workers must be first-order leaf functions in this task profile",
            ));
        }
    }
    Ok(TaskModule {
        entry: entry.0 as usize,
        functions,
        source_len: source.source().len(),
    })
}

fn next_temporary(function: &TypedSourceDeclaration) -> Result<usize, SourceDiagnostic> {
    fn statement_max(statements: &[TypedStatement]) -> usize {
        statements
            .iter()
            .map(|s| {
                s.binding
                    .into_iter()
                    .chain(s.body_binding)
                    .chain(
                        s.result_match
                            .iter()
                            .flat_map(|m| m.arms.iter().filter_map(|a| a.binding)),
                    )
                    .map(|id| id.0 as usize)
                    .chain(std::iter::once(statement_max(&s.nested)))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }
    function
        .parameters
        .iter()
        .map(|p| p.symbol.0 as usize)
        .chain(std::iter::once(statement_max(&function.statements)))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| {
            failure(
                function.span,
                SemanticPass::CEmission,
                "task slot identity space exhausted",
            )
        })
}

struct Builder<'a> {
    source: &'a SourceProgram,
    signatures: &'a BTreeMap<SymbolId, Signature>,
    signature: &'a Signature,
    errors: &'a BTreeMap<String, ErrorType>,
    environment: Vec<BTreeMap<String, (SymbolId, ValueType)>>,
    scopes: Vec<u64>,
    blocks: BTreeMap<usize, TaskBlock>,
    current: usize,
    next_block: usize,
    next_temporary: usize,
    instructions: Vec<TaskInstruction>,
}

impl<'a> Builder<'a> {
    fn new(
        source: &'a SourceProgram,
        signatures: &'a BTreeMap<SymbolId, Signature>,
        signature: &'a Signature,
        errors: &'a BTreeMap<String, ErrorType>,
        next_temporary: usize,
    ) -> Self {
        Self {
            source,
            signatures,
            signature,
            errors,
            environment: vec![
                signature
                    .parameters
                    .iter()
                    .map(|(symbol, name, kind)| (name.clone(), (*symbol, ValueType::Scalar(*kind))))
                    .collect(),
            ],
            scopes: Vec::new(),
            blocks: BTreeMap::new(),
            current: 0,
            next_block: 1,
            next_temporary,
            instructions: Vec::new(),
        }
    }

    fn bad(&self, span: Span, message: impl Into<String>) -> SourceDiagnostic {
        failure(span, SemanticPass::CEmission, message)
    }
    fn temporary(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let slot = self.next_temporary;
        self.next_temporary = slot
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task slot identity space exhausted"))?;
        Ok(slot)
    }
    fn block_id(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let block = self.next_block;
        self.next_block = block
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task block identity space exhausted"))?;
        Ok(block)
    }
    fn instruction(&mut self, span: Span, kind: TaskInstructionKind) {
        self.instructions.push(TaskInstruction {
            span: core_span(span),
            kind,
        });
    }
    fn finish(&mut self, span: Span, kind: TaskTerminatorKind) {
        self.blocks.insert(
            self.current,
            TaskBlock {
                instructions: std::mem::take(&mut self.instructions),
                terminator: TaskTerminator {
                    span: core_span(span),
                    kind,
                },
            },
        );
    }
    fn exit_instructions(&self, span: Span) -> Vec<TaskInstruction> {
        self.scopes
            .iter()
            .rev()
            .map(|scope| TaskInstruction {
                span: core_span(span),
                kind: TaskInstructionKind::ExitScope { scope: *scope },
            })
            .collect()
    }
    fn error_block(&mut self, span: Span, error: ErrorOperand) -> Result<usize, SourceDiagnostic> {
        let block = self.block_id(span)?;
        self.blocks.insert(
            block,
            TaskBlock {
                instructions: self.exit_instructions(span),
                terminator: TaskTerminator {
                    span: core_span(span),
                    // Every synthesized raise is payload-free: the two builtin
                    // failures hold nothing, and `?` is refused for a Result
                    // whose error can carry a payload.
                    kind: TaskTerminatorKind::Raise {
                        error,
                        payload: None,
                    },
                },
            },
        );
        Ok(block)
    }
    fn lookup(&self, name: &str, span: Span) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        self.environment
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::NameResolution,
                    "value is not a live local or parameter in this task function",
                )
            })
    }
    fn lookup_place(
        &self,
        place: &ResolvedTaskPlace,
    ) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        let SourcePlace::Name(name) = &place.syntax.value else {
            return Err(self.bad(
                place.syntax.span,
                "task execution supports direct local/parameter places only",
            ));
        };
        let found = self.lookup(&name.value, name.span)?;
        if found.0 != place.root_symbol {
            return Err(self.bad(
                name.span,
                "task place identity disagrees with the lexical value binding",
            ));
        }
        Ok(found)
    }

    fn allocation_input(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<Option<ScalarOperand>, SourceDiagnostic> {
        if !word(tokens.first(), "qcore_buffer_new") {
            return Ok(None);
        }
        if !self.signature.effects.alloc {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "buffer allocation requires alloc effect",
            ));
        }
        if !symbol(tokens.get(1), Symbol::LeftParen)
            || matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
                != Some(tokens.len().saturating_sub(1))
        {
            return Err(self.bad(
                span,
                "buffer constructor requires one complete i32 argument",
            ));
        }
        let (initial, kind) = self.scalar_operand(&tokens[2..tokens.len() - 1], span)?;
        if kind != Scalar::I32 {
            return Err(self.bad(span, "buffer constructor requires i32"));
        }
        Ok(Some(initial))
    }

    fn buffer_initializer(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        if word(tokens.first(), "qcore_buffer_new") {
            if !symbol(tokens.last(), Symbol::Question) {
                return Err(self.bad(
                    span,
                    "buffer allocation requires ? or explicit Result match",
                ));
            }
            let initial = self
                .allocation_input(&tokens[..tokens.len() - 1], span)?
                .unwrap();
            if !self.signature.effects.errors.contains("AllocError") {
                return Err(failure(
                    span,
                    SemanticPass::TypedErrors,
                    "allocation propagation requires throws<AllocError>",
                ));
            }
            let success = self.block_id(span)?;
            let allocation_error =
                self.error_block(span, ErrorOperand::Tag("AllocError".into()))?;
            self.finish(
                span,
                TaskTerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot: slot,
                    success,
                    allocation_error,
                },
            );
            self.current = success;
            return Ok(Some(ValueType::Scalar(Scalar::OwnedBuffer)));
        }
        if word(tokens.first(), "qcore_buffer_read") {
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(span, "buffer read requires one exact owned-buffer name"));
            }
            let buffer =
                name(tokens.get(2)).ok_or_else(|| self.bad(span, "buffer read requires a name"))?;
            let (id, kind) = self.lookup(buffer, span)?;
            // A read is the one operation a loan admits, and the one operation
            // an owner keeps while its buffer is lent out.
            if !matches!(
                kind,
                ValueType::Scalar(Scalar::OwnedBuffer | Scalar::ReadLoan)
            ) {
                return Err(self.bad(span, "buffer read requires own<Buffer> or borrow<Buffer>"));
            }
            self.instruction(
                span,
                TaskInstructionKind::ReadBuffer {
                    buffer: id.0 as usize,
                    value_slot: slot,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::I32)));
        }
        // Card T06. `qcore_error_payload(e)` moves the owned Buffer out of a
        // bound typed error and into a new owned binding. It is the only way a
        // payload leaves an error, it works at most once per binding, and the
        // error keeps its tag afterwards so it can still be inspected.
        if word(tokens.first(), "qcore_error_payload") {
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(
                    span,
                    "taking an error payload requires one exact bound error name",
                ));
            }
            let binding = name(tokens.get(2))
                .ok_or_else(|| self.bad(span, "taking an error payload requires a name"))?;
            let (id, kind) = self.lookup(binding, span)?;
            let ValueType::Error {
                errors,
                payload: true,
            } = kind
            else {
                return Err(self.bad(
                    span,
                    "qcore_error_payload requires a bound error that still holds an owned payload",
                ));
            };
            self.instruction(
                span,
                TaskInstructionKind::TakeErrorPayload {
                    error_slot: id.0 as usize,
                    buffer_slot: slot,
                },
            );
            // The binding now names a payload-free error. Rebinding it here is
            // what makes a second take, and a silent re-raise, impossible.
            self.rebind(
                binding,
                ValueType::Error {
                    errors,
                    payload: false,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::OwnedBuffer)));
        }
        Ok(None)
    }

    /// Replace the innermost binding of `name` in place. Only the payload
    /// state of a typed error changes this way, and only from held to taken.
    fn rebind(&mut self, name: &str, kind: ValueType) {
        for scope in self.environment.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                entry.1 = kind;
                return;
            }
        }
        unreachable!("rebinding follows a successful lookup of the same name")
    }

    fn scalar_operand(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<(ScalarOperand, Scalar), SourceDiagnostic> {
        match tokens {
            [
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => value
                .parse::<i32>()
                .map(|value| (ScalarOperand::I32(value), Scalar::I32))
                .map_err(|_| self.bad(span, "integer literal is outside i32")),
            [
                Token {
                    kind: TokenKind::Symbol(Symbol::Minus),
                    ..
                },
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => {
                let magnitude = value
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n <= 2147483648)
                    .ok_or_else(|| self.bad(span, "negative integer literal is outside i32"))?;
                Ok((
                    ScalarOperand::I32((-(magnitude as i64)) as i32),
                    Scalar::I32,
                ))
            }
            [
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
            ] => {
                let (symbol, kind) = self.lookup(name, span)?;
                let ValueType::Scalar(scalar) = kind else {
                    return Err(self.bad(
                        span,
                        "a scalar expression cannot copy a handle, result, or scope",
                    ));
                };
                if scalar == Scalar::OwnedBuffer {
                    return Err(self.bad(
                        span,
                        "owned buffers require explicit move or a checked buffer read",
                    ));
                }
                if scalar == Scalar::ReadLoan {
                    return Err(self.bad(
                        span,
                        "a read loan cannot be copied; it admits only a checked buffer read",
                    ));
                }
                Ok((ScalarOperand::Slot(symbol.0 as usize), scalar))
            }
            _ => Err(self.bad(
                span,
                "task scalar expressions require an i32 literal or immutable scalar binding",
            )),
        }
    }

    fn propagate(
        &mut self,
        span: Span,
        result_slot: usize,
        value_slot: usize,
        value: Scalar,
        errors: &BTreeSet<String>,
        error_payload: bool,
    ) -> Result<ValueType, SourceDiagnostic> {
        if !errors.is_subset(&self.signature.effects.errors) {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "propagated task errors are absent from the function's throws effects",
            ));
        }
        // `?` has no place to name the payload, so it could only discard or
        // silently forward it. Both are refused: the payload has to be taken
        // on an explicit error arm.
        if error_payload {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "an error that carries an owned payload requires an explicit Result match, not `?`",
            ));
        }
        if errors.is_empty() {
            self.instruction(
                span,
                TaskInstructionKind::UnwrapInfallible {
                    result_slot,
                    value_slot,
                },
            );
            return Ok(ValueType::Scalar(value));
        }
        let success = self.block_id(span)?;
        let error_slot = self.temporary(span)?;
        let error = self.error_block(span, ErrorOperand::Slot(error_slot))?;
        self.finish(
            span,
            TaskTerminatorKind::TryOutcome {
                result_slot,
                value_slot,
                error_slot,
                success,
                error,
            },
        );
        self.current = success;
        Ok(ValueType::Scalar(value))
    }

    fn spawn_inputs(
        &self,
        span: Span,
        scope: SymbolId,
        worker: SymbolId,
        captures: &[ResolvedTaskCapture],
    ) -> Result<(u64, Vec<ScalarOperand>), SourceDiagnostic> {
        if !self.signature.effects.spawn {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "spawn requires its declared effect",
            ));
        }
        let scope_id = u64::from(scope.0);
        if self.scopes.last() != Some(&scope_id) {
            return Err(self.bad(span, "spawn requires the innermost active thread scope"));
        }
        let signature = self
            .signatures
            .get(&worker)
            .ok_or_else(|| self.bad(span, "unknown source worker"))?;
        if captures.len() != signature.parameters.len() {
            return Err(self.bad(span, "worker capture arity mismatch"));
        }
        let mut arguments = Vec::new();
        let mut loans = 0usize;
        for (capture, (_, _, parameter_type)) in captures.iter().zip(&signature.parameters) {
            let (symbol, kind) = self.lookup_place(&capture.place)?;
            // A read loan is the one capture whose place type and parameter
            // type differ on purpose: the lender keeps `own<Buffer>` and the
            // borrower receives `borrow<Buffer>`.
            let place_type = if *parameter_type == Scalar::ReadLoan {
                Scalar::OwnedBuffer
            } else {
                *parameter_type
            };
            if kind != ValueType::Scalar(place_type) {
                return Err(failure(
                    capture.span,
                    SemanticPass::OrdinaryTypes,
                    "capture type does not match worker parameter",
                ));
            }
            let operand = match (&capture.mode, parameter_type) {
                (SourceCaptureMode::Copy, Scalar::I32) => ScalarOperand::Slot(symbol.0 as usize),
                (SourceCaptureMode::Move, Scalar::OwnedBuffer) => {
                    ScalarOperand::Move(symbol.0 as usize)
                }
                (SourceCaptureMode::Read { share }, Scalar::ReadLoan) => {
                    // Only the whole read authority is admitted. A fractional
                    // or named share is a multi-reader split, which this
                    // profile refuses rather than approximates.
                    if share.value != SourceShareExpression::Full {
                        return Err(self.bad(
                            share.span,
                            "a read loan requires `share FULL`; fractional read shares are outside this profile",
                        ));
                    }
                    loans += 1;
                    ScalarOperand::Loan(symbol.0 as usize)
                }
                _ => {
                    return Err(self.bad(
                        capture.span,
                        "only copy i32, move own<Buffer> and read own<Buffer> share FULL captures are executable",
                    ));
                }
            };
            arguments.push(operand);
        }
        if loans > 1 {
            return Err(self.bad(
                span,
                "this profile admits at most one read loan per task creation",
            ));
        }
        Ok((scope_id, arguments))
    }

    fn result_match(
        &mut self,
        statement: &TypedStatement,
        tokens: &[Token],
    ) -> Result<bool, SourceDiagnostic> {
        let span = statement.span;
        let resolved = statement
            .result_match
            .as_ref()
            .ok_or_else(|| self.bad(span, "missing resolved Result match metadata"))?;
        let syntax = &resolved.syntax;
        if syntax.span != span
            || resolved.arms.len() != 2
            || syntax.arms.len() != 2
            || !word(tokens.first(), "match")
            || tokens.get(1).map(|t| t.span.start) != Some(syntax.scrutinee_span.start)
        {
            return Err(self.bad(
                span,
                "Result match requires one complete scrutinee and exactly two canonical arms",
            ));
        }
        let open = tokens
            .iter()
            .position(|t| t.span.start >= syntax.scrutinee_span.end)
            .ok_or_else(|| self.bad(span, "missing Result match body"))?;
        if open < 2
            || tokens[open - 1].span.end != syntax.scrutinee_span.end
            || matching(tokens, open, Symbol::LeftBrace, Symbol::RightBrace)
                != Some(tokens.len() - 1)
        {
            return Err(self.bad(
                span,
                "Result match must consume its complete expression and outer body",
            ));
        }
        let mut cursor = open + 1;
        let mut ok = None;
        let mut err = None;
        for (index, arm) in resolved.arms.iter().enumerate() {
            if arm.syntax != syntax.arms[index] {
                return Err(self.bad(span, "Result arm metadata disagrees with source"));
            }
            let pattern = arm.syntax.pattern.as_ref().ok_or_else(|| self.bad(arm.syntax.pattern_span, "only exact Result::Ok { value: name } and Result::Err { error: name } patterns are supported"))?;
            let binder = arm.binding.ok_or_else(|| {
                self.bad(
                    arm.syntax.pattern_span,
                    "missing resolved Result arm binder",
                )
            })?;
            let end = tokens
                .iter()
                .position(|t| t.span.end == arm.syntax.pattern_span.end)
                .ok_or_else(|| self.bad(span, "Result pattern boundary is unavailable"))?;
            if tokens.get(cursor).map(|t| t.span.start) != Some(arm.syntax.pattern_span.start)
                || end < cursor
                || !symbol(tokens.get(end + 1), Symbol::FatArrow)
                || tokens.get(end + 2).map(|t| t.span.start) != Some(arm.syntax.body.span.start)
            {
                return Err(self.bad(span, "unsupported tokens around Result match arm"));
            }
            let body_end = matching(tokens, end + 2, Symbol::LeftBrace, Symbol::RightBrace)
                .ok_or_else(|| self.bad(span, "invalid Result arm body"))?;
            if tokens[body_end].span.end != arm.syntax.body.span.end {
                return Err(self.bad(span, "Result arm body boundary mismatch"));
            }
            cursor = body_end + 1;
            let slot = match pattern.kind {
                ResultPatternKind::Ok => &mut ok,
                ResultPatternKind::Err => &mut err,
            };
            if slot.replace((index, binder)).is_some() {
                return Err(self.bad(arm.syntax.pattern_span, "duplicate Result match variant"));
            }
        }
        if cursor != tokens.len() - 1 {
            return Err(self.bad(span, "unsupported trailing Result match tokens"));
        }
        let (ok_index, ok_binder) =
            ok.ok_or_else(|| self.bad(span, "Result match lacks Ok arm"))?;
        let (err_index, err_binder) =
            err.ok_or_else(|| self.bad(span, "Result match lacks Err arm"))?;
        let success = self.block_id(span)?;
        let error = self.block_id(span)?;
        let (ok_type, err_type) = match resolved.scrutinee.as_ref() {
            Some(ResolvedResultMatchScrutinee::Task(task)) => {
                if task.syntax.propagates_error || task.syntax.span != syntax.scrutinee_span {
                    return Err(self.bad(
                        span,
                        "Result match requires the whole unpropagated task expression",
                    ));
                }
                match &task.kind {
                    ResolvedTaskExpressionKind::Spawn {
                        scope,
                        worker,
                        captures,
                    } => {
                        let (scope, arguments) =
                            self.spawn_inputs(span, *scope, *worker, captures)?;
                        self.finish(
                            span,
                            TaskTerminatorKind::TrySpawn {
                                scope,
                                worker: worker.0 as usize,
                                arguments,
                                handle_slot: ok_binder.0 as usize,
                                success,
                                spawn_error: error,
                            },
                        );
                        (
                            ValueType::Handle(*worker),
                            ValueType::KnownError("SpawnError".into()),
                        )
                    }
                    ResolvedTaskExpressionKind::Join { .. } => {
                        let result_slot = self.temporary(span)?;
                        let mut scrutinee_statement = statement.clone();
                        scrutinee_statement.task_expressions = vec![(**task).clone()];
                        let kind = self
                            .task_expression(
                                &scrutinee_statement,
                                &tokens[1..open],
                                Some(result_slot),
                            )?
                            .ok_or_else(|| self.bad(span, "join match requires a Result value"))?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(self.bad(span, "join match scrutinee is not a Result"));
                        };
                        self.finish(
                            span,
                            TaskTerminatorKind::TryOutcome {
                                result_slot,
                                value_slot: ok_binder.0 as usize,
                                error_slot: err_binder.0 as usize,
                                success,
                                error,
                            },
                        );
                        (
                            ValueType::Scalar(value),
                            ValueType::Error {
                                errors,
                                payload: error_payload,
                            },
                        )
                    }
                }
            }
            Some(ResolvedResultMatchScrutinee::Value(symbol_id)) => {
                let [token] = &tokens[1..open] else {
                    return Err(
                        self.bad(span, "Result match requires one immutable Result binding")
                    );
                };
                let spelling =
                    name(Some(token)).ok_or_else(|| self.bad(span, "invalid Result binding"))?;
                let (found, kind) = self.lookup(spelling, syntax.scrutinee_span)?;
                if found != *symbol_id {
                    return Err(self.bad(span, "Result binding identity mismatch"));
                }
                let ValueType::Result {
                    value,
                    errors,
                    error_payload,
                } = kind
                else {
                    return Err(self.bad(span, "match requires an initialized task Result"));
                };
                self.finish(
                    span,
                    TaskTerminatorKind::TryOutcome {
                        result_slot: found.0 as usize,
                        value_slot: ok_binder.0 as usize,
                        error_slot: err_binder.0 as usize,
                        success,
                        error,
                    },
                );
                (
                    ValueType::Scalar(value),
                    ValueType::Error {
                        errors,
                        payload: error_payload,
                    },
                )
            }
            None => {
                let scrutinee = tokens_for(self.source, syntax.scrutinee_span);
                let initial = self
                    .allocation_input(&scrutinee, span)?
                    .ok_or_else(|| self.bad(span, "unsupported Result match scrutinee"))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TryAllocateBuffer {
                        initial,
                        value_slot: ok_binder.0 as usize,
                        success,
                        allocation_error: error,
                    },
                );
                (
                    ValueType::Scalar(Scalar::OwnedBuffer),
                    ValueType::KnownError("AllocError".into()),
                )
            }
        };
        let outer_environment = self.environment.clone();
        let outer_scopes = self.scopes.clone();
        let mut continuation = None;
        for (index, arm) in resolved.arms.iter().enumerate() {
            let (target, binder, kind) = if index == ok_index {
                (success, ok_binder, ok_type.clone())
            } else {
                debug_assert_eq!(index, err_index);
                (error, err_binder, err_type.clone())
            };
            self.current = target;
            self.environment = outer_environment.clone();
            self.scopes = outer_scopes.clone();
            self.environment.push(BTreeMap::from([(
                arm.syntax.pattern.as_ref().unwrap().binding.value.clone(),
                (binder, kind),
            )]));
            let arm_statements = statement
                .nested
                .get(arm.statement_range.clone())
                .ok_or_else(|| self.bad(span, "Result arm statement range is invalid"))?;
            let terminated = self.block(&arm.syntax.body, arm_statements)?;
            self.environment.pop();
            if !terminated {
                let join = match continuation {
                    Some(id) => id,
                    None => {
                        let id = self.block_id(span)?;
                        continuation = Some(id);
                        id
                    }
                };
                self.finish(span, TaskTerminatorKind::Goto { target: join });
            }
        }
        self.environment = outer_environment;
        self.scopes = outer_scopes;
        if let Some(block) = continuation {
            self.current = block;
        }
        Ok(continuation.is_none())
    }

    fn task_expression(
        &mut self,
        statement: &TypedStatement,
        rhs: &[Token],
        slot: Option<usize>,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let span = statement.span;
        if statement.task_expressions.len() != 1 {
            return Err(self.bad(
                span,
                "task execution requires exactly one complete task expression",
            ));
        }
        let task = &statement.task_expressions[0];
        if rhs.first().map(|t| t.span.start) != Some(task.syntax.span.start)
            || rhs.last().map(|t| t.span.end) != Some(task.syntax.span.end)
        {
            return Err(self.bad(span, "task expression must consume its complete statement or initializer; suffixes and surrounding operations are unsupported"));
        }
        match &task.kind {
            ResolvedTaskExpressionKind::Spawn {
                scope,
                worker,
                captures,
            } => {
                let Some(slot) = slot else {
                    return Err(self.bad(span, "spawn requires a fresh handle binding"));
                };
                if !task.syntax.propagates_error {
                    return Err(self.bad(
                        span,
                        "spawn requires immediate error discrimination with `?` in this profile",
                    ));
                }
                if !self.signature.effects.spawn
                    || !self.signature.effects.errors.contains("SpawnError")
                {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "spawn requires spawn and throws<SpawnError> effects",
                    ));
                }
                let (scope_id, arguments) = self.spawn_inputs(span, *scope, *worker, captures)?;
                let success = self.block_id(span)?;
                let spawn_error = self.error_block(span, ErrorOperand::Tag("SpawnError".into()))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TrySpawn {
                        scope: scope_id,
                        worker: worker.0 as usize,
                        arguments,
                        handle_slot: slot,
                        success,
                        spawn_error,
                    },
                );
                self.current = success;
                Ok(Some(ValueType::Handle(*worker)))
            }
            ResolvedTaskExpressionKind::Join { handle } => {
                if !self.signature.effects.join || !self.signature.effects.may_block {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "join requires join and may_block effects",
                    ));
                }
                let (handle, kind) = self.lookup_place(handle)?;
                let ValueType::Handle(worker) = kind else {
                    return Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        "join requires a task handle, not a scalar or undiscriminated result",
                    ));
                };
                let worker = &self.signatures[&worker];
                let value = worker.result;
                let errors = worker.effects.errors.clone();
                // The worker's own declared payload row, not the caller's: a
                // caller cannot decide that a joined error holds nothing.
                let error_payload = !worker.effects.error_payloads.is_empty();
                if task.syntax.propagates_error {
                    let result_slot = self.temporary(span)?;
                    let value_slot = match slot {
                        Some(slot) => slot,
                        None => self.temporary(span)?,
                    };
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: Some(result_slot),
                        },
                    );
                    let kind = self.propagate(
                        span,
                        result_slot,
                        value_slot,
                        value,
                        &errors,
                        error_payload,
                    )?;
                    if slot.is_none() && value != Scalar::Unit {
                        return Err(self.bad(span, "discarded join result must be unit"));
                    }
                    Ok(slot.map(|_| kind))
                } else {
                    if slot.is_none() && (value != Scalar::Unit || !errors.is_empty()) {
                        return Err(self.bad(span, "discarded join result must be infallible unit"));
                    }
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: slot,
                        },
                    );
                    Ok(slot.map(|_| ValueType::Result {
                        value,
                        errors,
                        error_payload,
                    }))
                }
            }
        }
    }

    fn block(
        &mut self,
        block: &SourceBlock,
        typed: &[TypedStatement],
    ) -> Result<bool, SourceDiagnostic> {
        if block.statements.len() != typed.len() {
            return Err(self.bad(block.span, "task block metadata mismatch"));
        }
        let mut terminated = false;
        for (statement, typed) in block.statements.iter().zip(typed) {
            if terminated {
                return Err(self.bad(
                    statement.span,
                    "unreachable statements after return/raise are not admitted by task execution",
                ));
            }
            if statement.span != typed.span {
                return Err(self.bad(statement.span, "task statement metadata mismatch"));
            }
            let tokens = tokens_for(self.source, statement.span);
            match &statement.value {
                StatementKind::Match { .. } => {
                    terminated = self.result_match(typed, &tokens)?;
                }
                StatementKind::ThreadScope { name, body } => {
                    if tokens.len() < 5
                        || !word(tokens.first(), "thread")
                        || !word(tokens.get(1), "scope")
                        || name != super_name(tokens.get(2)).unwrap_or("")
                        || !symbol(tokens.get(3), Symbol::LeftBrace)
                        || tokens[3].span.start != body.span.start
                        || matching(&tokens, 3, Symbol::LeftBrace, Symbol::RightBrace)
                            != Some(tokens.len() - 1)
                        || tokens.last().map(|token| token.span.end) != Some(body.span.end)
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "thread scope header must be consumed exactly",
                        ));
                    }
                    let scope = typed.body_binding.ok_or_else(|| {
                        self.bad(statement.span, "missing typed thread-scope binder")
                    })?;
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::EnterScope {
                            scope: u64::from(scope.0),
                        },
                    );
                    self.scopes.push(u64::from(scope.0));
                    self.environment
                        .push(BTreeMap::from([(name.clone(), (scope, ValueType::Scope))]));
                    terminated = self.block(body, &typed.nested)?;
                    self.environment.pop();
                    self.scopes.pop();
                    if !terminated {
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::ExitScope {
                                scope: u64::from(scope.0),
                            },
                        );
                    }
                }
                StatementKind::Let {
                    binding: Some(binding),
                } => {
                    if !word(tokens.first(), "let")
                        || name(tokens.get(1)) != Some(binding.as_str())
                        || !symbol(tokens.last(), Symbol::Semicolon)
                    {
                        return Err(self.bad(
                            statement.span,
                            "only a single immutable name binding is supported",
                        ));
                    }
                    let equal = tokens
                        .iter()
                        .position(|token| symbol(Some(token), Symbol::Equal))
                        .ok_or_else(|| {
                            self.bad(statement.span, "task locals require initialized values")
                        })?;
                    let annotation = if equal == 2 {
                        None
                    } else if equal > 3 && symbol(tokens.get(2), Symbol::Colon) {
                        Some(scalar(&tokens[3..equal]).ok_or_else(|| {
                            self.bad(statement.span, "unsupported task local type")
                        })?)
                    } else {
                        return Err(self.bad(
                            statement.span,
                            "unsupported binding pattern/type annotation",
                        ));
                    };
                    let rhs = &tokens[equal + 1..tokens.len() - 1];
                    let id = typed
                        .binding
                        .ok_or_else(|| self.bad(statement.span, "missing typed local binder"))?;
                    let kind = if !typed.task_expressions.is_empty() {
                        self.task_expression(typed, rhs, Some(id.0 as usize))?
                            .unwrap()
                    } else if let Some(kind) =
                        self.buffer_initializer(rhs, statement.span, id.0 as usize)?
                    {
                        kind
                    } else if rhs.len() == 2 && symbol(rhs.get(1), Symbol::Question) {
                        let source_name = name(rhs.first()).ok_or_else(|| {
                            self.bad(
                                statement.span,
                                "result propagation requires an immutable result binding",
                            )
                        })?;
                        let (result, kind) = self.lookup(source_name, statement.span)?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(
                                self.bad(statement.span, "`?` requires a collected task result")
                            );
                        };
                        self.propagate(
                            statement.span,
                            result.0 as usize,
                            id.0 as usize,
                            value,
                            &errors,
                            error_payload,
                        )?
                    } else {
                        let (value, scalar) = self.scalar_operand(rhs, statement.span)?;
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::BindScalar {
                                slot: id.0 as usize,
                                value,
                            },
                        );
                        ValueType::Scalar(scalar)
                    };
                    if annotation.is_some_and(|annotation| kind != ValueType::Scalar(annotation)) {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "local result does not match its declared scalar type",
                        ));
                    }
                    self.environment
                        .last_mut()
                        .unwrap()
                        .insert(binding.clone(), (id, kind));
                }
                StatementKind::Drop { place: Some(place) } => {
                    if tokens.len() != 3
                        || !word(tokens.first(), "drop")
                        || name(tokens.get(1)) != Some(place.as_str())
                        || !symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        return Err(self.bad(statement.span, "buffer drop requires one exact name"));
                    }
                    let (id, kind) = self.lookup(place, statement.span)?;
                    if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                        return Err(self.bad(statement.span, "drop requires an owned buffer"));
                    }
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::DropBuffer {
                            buffer: id.0 as usize,
                        },
                    );
                }
                StatementKind::Expression => {
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "unsupported expression statement in task execution",
                        ));
                    }
                    self.task_expression(typed, &tokens[..tokens.len() - 1], None)?;
                }
                StatementKind::Return => {
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || !word(tokens.first(), "return")
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(statement.span, "unsupported task return expression"));
                    }
                    let (value, kind) = if tokens.len() == 4 && word(tokens.get(1), "move") {
                        let buffer = name(tokens.get(2)).ok_or_else(|| {
                            self.bad(statement.span, "owned return requires a name")
                        })?;
                        let (id, kind) = self.lookup(buffer, statement.span)?;
                        if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                            return Err(
                                self.bad(statement.span, "move return requires own<Buffer>")
                            );
                        }
                        (ScalarOperand::Move(id.0 as usize), Scalar::OwnedBuffer)
                    } else if tokens.len() == 2 {
                        (ScalarOperand::Unit, Scalar::Unit)
                    } else {
                        self.scalar_operand(&tokens[1..tokens.len() - 1], statement.span)?
                    };
                    if kind != self.signature.result {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "return value does not match the declared task function result",
                        ));
                    }
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Return { value });
                    terminated = true;
                }
                StatementKind::Raise => {
                    let (error, payload) = if tokens.len() == 3
                        && word(tokens.first(), "raise")
                        && symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        let binding = name(tokens.get(1)).ok_or_else(|| {
                            self.bad(statement.span, "raise requires a bound typed error")
                        })?;
                        let (id, kind) = self.lookup(binding, statement.span)?;
                        let error = match kind {
                            ValueType::KnownError(tag) => {
                                if !self.signature.effects.errors.contains(&tag) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error is absent from throws effects",
                                    ));
                                }
                                ErrorOperand::Tag(tag)
                            }
                            ValueType::Error { errors, payload } => {
                                if !errors.is_subset(&self.signature.effects.errors) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error variants are absent from throws effects",
                                    ));
                                }
                                // Re-raising a bound error keeps its tag, and a
                                // tag alone cannot carry a Buffer onward: take
                                // the payload first, then decide what to do with
                                // it and which tag to raise.
                                if payload {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "take the owned payload with qcore_error_payload before re-raising this error",
                                    ));
                                }
                                ErrorOperand::Slot(id.0 as usize)
                            }
                            _ => {
                                return Err(self.bad(
                                    statement.span,
                                    "raise requires a typed error arm binding",
                                ));
                            }
                        };
                        (error, None)
                    } else {
                        // `raise E::V;` is payload-free; `raise E::V(b);` moves
                        // exactly one owned Buffer into the raised error. The
                        // declaration decides which spelling is required.
                        let carried = tokens.len() == 8
                            && symbol(tokens.get(4), Symbol::LeftParen)
                            && symbol(tokens.get(6), Symbol::RightParen);
                        if (tokens.len() != 5 && !carried)
                            || !word(tokens.first(), "raise")
                            || !symbol(tokens.get(2), Symbol::ColonColon)
                            || !symbol(tokens.last(), Symbol::Semicolon)
                        {
                            return Err(self.bad(
                                statement.span,
                                "task raise requires a bound error, Error::Variant, or Error::Variant(buffer)",
                            ));
                        }
                        let error_type = name(tokens.get(1))
                            .ok_or_else(|| self.bad(statement.span, "invalid error type"))?;
                        let variant = name(tokens.get(3))
                            .ok_or_else(|| self.bad(statement.span, "invalid error variant"))?;
                        let tag = format!("{error_type}::{variant}");
                        if !self
                            .errors
                            .get(error_type)
                            .is_some_and(|declared| declared.variants.contains(&tag))
                            || !self.signature.effects.errors.contains(&tag)
                        {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised variant must be declared and covered by throws effects",
                            ));
                        }
                        if self.signature.effects.error_payloads.contains(&tag) != carried {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised payload disagrees with the declared error variant",
                            ));
                        }
                        let payload = if carried {
                            let buffer = name(tokens.get(5)).ok_or_else(|| {
                                self.bad(statement.span, "an error payload requires one name")
                            })?;
                            let (id, kind) = self.lookup(buffer, statement.span)?;
                            // A loan, a scalar and a Result each fail here: the
                            // payload has to be an owned Buffer this frame may
                            // give away, and the raise consumes it.
                            if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                                return Err(self
                                    .bad(statement.span, "an error payload requires own<Buffer>"));
                            }
                            Some(ScalarOperand::Move(id.0 as usize))
                        } else {
                            None
                        };
                        (ErrorOperand::Tag(tag), payload)
                    };
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Raise { error, payload });
                    terminated = true;
                }
                _ => {
                    return Err(self.bad(
                        statement.span,
                        "statement is outside the copy-scalar task execution profile",
                    ));
                }
            }
        }
        Ok(terminated)
    }
}

// Avoid a pattern binding named `name` obscuring the token helper in scope arms.
fn super_name(token: Option<&Token>) -> Option<&str> {
    name(token)
}
