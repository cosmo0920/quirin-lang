//! Exact source lowering for the bounded extern-call profile
//! `source.extern-call.v1` (`docs/design/EXTERN_CALL_SEMANTICS_V1.md`).
//!
//! The admitted surface is fully enumerated below. A module is either entirely
//! inside it and lowers to both an [`ExternProgram`] the core checker validates
//! and a [`FunctionSemantics`] bundle every established sequential pass
//! validates, or it is refused with a precise diagnostic. There is no partially
//! lowered middle state and no optimistic fallback.
//!
//! ```text
//! module    ::= "import" "std.extern" ";" (error-decl | extern-decl | entry)*
//! extern-decl ::= "extern" "fn" NAME "(" [param {"," param}] ")" "->" ABI ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit" ["effects" "{" EFFECT {"," EFFECT} "}"]
//!               "{" stmt* "}"
//! stmt      ::= "let" NAME "=" CALLEE "(" [arg {"," arg}] ")"
//!                   "succeeds" ("true"|"false") "else" {disposition}
//!                   "raise" ERROR ";"
//!             | CALLEE "(" [arg {"," arg}] ")" ";"
//!             | "return" ";"
//! disposition ::= CALLEE "(" [arg {"," arg}] ")"   -- one release, error arm
//! arg       ::= ["-"] INTEGER            -- scalar data
//!             | NAME                     -- a lent model Buffer, or a live
//!                                        --   identity in declared owner
//!                                        --   position
//!             | "move" NAME              -- a consumed `own<Buffer>`, or a
//!                                        --   stored one: which it is comes
//!                                        --   from the declared mode, never
//!                                        --   from the call site
//!             | "out"                    -- the declared out slot
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! Card `EXTERN-F1` adds the last of those argument forms and one meaning to
//! each of the two before it. `out` marks the position of a declared
//! `resource_out(_, nullable)` slot; the `let` binder names the identity the
//! callee writes into it, and the mandatory `succeeds BOOL else raise ERROR`
//! adapter is what makes that binding materialize on the success arm only. A
//! `move NAME` at a declared `resource_stored(_, owner, nonnull)` parameter
//! removes the binding from this body exactly as a consumption does — every
//! later use is refused — and the identity's residual accounting beneath the
//! declared owner becomes a named premise on the certificate.
//!
//! Card `EXTERN-ERR-REL` adds the dispositions between `else` and `raise`. They
//! are the calls this acquisition's established cleanup edge performs —
//! `qcore_core::cleanup_lowering` releases every identity live before a failing
//! acquisition, in reverse acquisition order — written out, because an
//! extern-acquired identity is released only by calling a declared
//! `resource_consumed` callee and this profile cannot invent one. Their
//! arguments are resolved against the bindings live at the attempt and never
//! remove one: the arm is a path, not a place.
//!
//! The `succeeds BOOL` control is the fixture's literal outcome, exactly the
//! `qcore_acquire`/`qcore_check` philosophy: these are controlled model
//! operations, not a claim about when a production allocator fails. Both arms
//! of a nullable owned return are exercisable, and the emitted C compares the
//! real result against the declared control rather than assuming it.
//!
//! What this lowering does **not** establish: that any callee's C
//! implementation satisfies its declared contract, how many bytes a callee
//! touches (C04 pointers are extentless `u8` projections and this profile
//! transports no extent), or which object file supplies a symbol. All three are
//! premises recorded on every certificate this route produces.
use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::whole_program::{CallSite, FunctionSemantics, FunctionSummary, ParameterOwnership};
use qcore_core::cleanup_lowering::{SourceOp, SourceProgram as LoweringProgram};
use qcore_core::extern_call::{
    CallParameter, ExternAdapter, ExternArgument, ExternCall, ExternCallee, ExternCustody,
    ExternEvent, ExternProgram, derived_custody, derived_effects,
};
use qcore_core::ownership::{Footprint, OwnershipOp, ResourceId};
use qcore_core::resource_flow::{Flow, Quantity, ResourceBounds};
use qcore_core::typed_errors::{Effect, EffectRow, ErrorLedger, ErrorOp};
use qcore_core::usage::UsageExpr;

/// A complete lowering of one entry function. Neither field is evidence on its
/// own: the core checker still has to validate `program`, and every
/// established sequential pass still has to validate `semantics`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredExternModule {
    pub entry: String,
    pub program: ExternProgram,
    /// Source span of the event at the same index. Every event has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for a refusal that names no event.
    pub scope_span: Span,
    pub semantics: FunctionSemantics,
    /// Declared summaries of the called extern symbols, for the two-way call
    /// dispatch of `check_whole_program_with_externs`.
    pub extern_summaries: BTreeMap<SymbolId, FunctionSummary>,
    /// The row the entry declares, which must admit the derived one.
    pub declared_effects: EffectRow,
}

/// Whether this program calls one of its own declared extern functions, i.e.
/// whether the extern-call route is the one that models it. Declaring an
/// extern function is not calling one.
#[must_use]
pub fn has_source_extern(program: &SourceProgram) -> bool {
    !called_extern_names(program).is_empty()
}

/// Every declared extern function this program calls, with the span of the
/// first call. Used by the driver to require a contract for exactly the
/// callees a module uses, and by nothing else.
#[must_use]
pub fn called_extern_names(program: &SourceProgram) -> BTreeMap<String, Span> {
    let externs = program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ExternFunction)
        .map(|declaration| declaration.value.name.clone())
        .collect::<BTreeSet<_>>();
    let mut called = BTreeMap::new();
    if externs.is_empty() {
        return called;
    }
    let tokens = program.lexical_tokens();
    // Only the two *call positions* this profile can express are scanned, at
    // any nesting depth: the head of an expression statement, and the operand
    // immediately after `=` in a `let`. An extern name appearing anywhere else
    // — a `defer`, a `raise`, a `spawn` operand — is not an extern call in this
    // grammar; it keeps its existing refusal on the route that owns that form
    // and never turns a module into an extern-call module.
    fn scan(
        block: &SourceBlock,
        externs: &BTreeSet<String>,
        tokens: &[Token],
        called: &mut BTreeMap<String, Span>,
    ) {
        for statement in &block.statements {
            let span = statement.span;
            let window = tokens
                .iter()
                .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
                .collect::<Vec<_>>();
            let mut heads = match statement.value {
                StatementKind::Expression => vec![0],
                StatementKind::Let { .. } => window
                    .iter()
                    .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
                    .map(|index| vec![index + 1])
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            // Card `EXTERN-ERR-REL`: an adapter's error arm carries call heads
            // too. They are found by position, after the `else` word, so a
            // callee a module uses only to release custody on a failure path
            // still has its contract required rather than reported missing.
            if matches!(statement.value, StatementKind::Let { .. })
                && let Some(index) = window.iter().position(
                    |token| matches!(&token.kind, TokenKind::Word(word) if word == "else"),
                )
            {
                heads.extend(index + 1..window.len());
            }
            for head in heads {
                if let Some(TokenKind::Word(word)) = window.get(head).map(|token| &token.kind)
                    && externs.contains(word)
                    && window
                        .get(head + 1)
                        .is_some_and(|token| token.kind == TokenKind::Symbol(Symbol::LeftParen))
                {
                    called.entry(word.clone()).or_insert(span);
                }
            }
            for nested in statement.value.nested_blocks() {
                scan(nested, externs, tokens, called);
            }
        }
    }
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body {
            scan(body, &externs, tokens, &mut called);
        }
    }
    called
}

/// Signature of one declared extern function as written in the source file,
/// for comparison against the selected contract module's own declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceExternDeclaration {
    pub name: String,
    pub span: Span,
    pub parameter_names: Vec<String>,
    /// Normalized parameter type spellings, whitespace removed.
    pub parameter_types: Vec<String>,
    pub result_type: String,
    /// Whether the source declaration carries its own `contract { … }` block.
    pub has_inline_contract: bool,
    /// Card `UNSAFE-LINKAGE`: the declaration is spelled `unsafe extern fn`, so
    /// it waives the contract requirement and carries an unchecked-linkage
    /// premise instead. Read from the declaration's own leading token, which is
    /// the only place the waiver exists.
    pub unchecked: bool,
}

/// Collect the extern declarations of a program, normalized for comparison.
#[must_use]
pub fn source_extern_declarations(program: &SourceProgram) -> Vec<SourceExternDeclaration> {
    program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ExternFunction)
        .map(|declaration| SourceExternDeclaration {
            name: declaration.value.name.clone(),
            span: declaration.span,
            parameter_names: declaration
                .value
                .parameters
                .iter()
                .map(|parameter| parameter.name.value.clone())
                .collect(),
            parameter_types: declaration
                .value
                .parameters
                .iter()
                .map(|parameter| normalize_type(&parameter.type_source))
                .collect(),
            result_type: declaration
                .value
                .result_type
                .as_ref()
                .map_or_else(String::new, |result| normalize_type(&result.source)),
            has_inline_contract: declaration.value.function_contract.is_some(),
            unchecked: is_unchecked_declaration(&declaration.value),
        })
        .collect()
}

/// Whether one extern declaration is spelled `unsafe extern fn`. The parser
/// keeps both spellings on `DeclarationKind::ExternFunction`, so the waiver
/// lives in the retained token prefix and nowhere else.
fn is_unchecked_declaration(declaration: &SourceDeclaration) -> bool {
    matches!(
        declaration.tokens.first(),
        Some(TokenKind::Word(word)) if word == "unsafe"
    )
}

/// The span of the first `unsafe extern fn` declaration in this program, if any.
/// Card `UNSAFE-LINKAGE`: the import gate is on the *declaration*, because
/// declaring an uncontracted symbol is itself the trust act — a file that never
/// calls it has still linked it.
#[must_use]
pub fn unchecked_extern_declaration_span(program: &SourceProgram) -> Option<Span> {
    program
        .declarations
        .iter()
        .find(|declaration| {
            declaration.value.kind == DeclarationKind::ExternFunction
                && is_unchecked_declaration(&declaration.value)
        })
        .map(|declaration| declaration.span)
}

/// Whitespace-insensitive type spelling; the loader's own vocabulary is a
/// closed list, so this never has to canonicalize anything else.
#[must_use]
pub fn normalize_type(source: &str) -> String {
    source.split_whitespace().collect()
}

fn diagnostic(
    code: &'static str,
    pass: SemanticPass,
    span: Span,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass,
        span,
        message: message.into(),
    }
}

fn refused_context(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q026", SemanticPass::ExternCall, span, message)
}

fn bridging(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q024", SemanticPass::ExternCall, span, message)
}

fn adapter(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q025", SemanticPass::ExternCall, span, message)
}

fn unsupported(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q500", SemanticPass::CEmission, span, message)
}

/// The tokens of one statement, read from the program's own token stream.
///
/// This used to re-lex `source[span]` and shift the resulting spans. Card
/// `PIPE-OPERATOR` replaced that with a containment filter over the tokens the
/// parse produced, for a correctness reason and not a tidiness one: re-lexing
/// the original bytes bypasses the pipe desugaring, so a pipeline reaching this
/// route would have been read back in its written form and refused. For a
/// program with no pipe the two agree token for token — the parse lexed the
/// same bytes — and every existing extern fixture is unmoved.
fn statement_tokens(program: &SourceProgram, span: Span) -> Result<Vec<Token>, SourceDiagnostic> {
    let tokens: Vec<Token> = program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .cloned()
        .collect();
    if tokens.is_empty() {
        return Err(unsupported(span, "this statement has no tokens"));
    }
    Ok(tokens)
}

fn word(token: Option<&Token>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Symbol(found)) if *found == expected)
}

/// The declared effect row of the entry, read from its `effects { … }` suffix.
/// An absent suffix is the empty row, which admits no extern call at all: the
/// derived row always contains `Io`, so the opt-in stays visible.
fn declared_effect_row(
    declaration: &SourceDeclaration,
    span: Span,
) -> Result<EffectRow, SourceDiagnostic> {
    let tokens = &declaration.tokens;
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok(EffectRow::default());
    };
    if !matches!(
        tokens.get(index + 1),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            span,
            "an `effects` row requires a block",
        ));
    }
    let mut effects = BTreeSet::new();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name)) => {
                let effect = match name.as_str() {
                    "io" => Effect::Io,
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    other => {
                        return Err(diagnostic(
                            "Q020",
                            SemanticPass::OrdinaryTypes,
                            span,
                            format!(
                                "`{other}` is not an effect the extern-call profile models; use io, alloc or mut"
                            ),
                        ));
                    }
                };
                effects.insert(effect);
                cursor += 1;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            _ => {
                return Err(diagnostic(
                    "Q020",
                    SemanticPass::OrdinaryTypes,
                    span,
                    "the `effects` row is not a comma-separated list of effect names",
                ));
            }
        }
    }
    Ok(EffectRow::new(effects))
}

/// One resolved argument list: the checked arguments, the caller identities the
/// call reads in declaration order, and every `move NAME` argument with the mode
/// it bridged to. Card `EXTERN-MODES` makes the third component's mode
/// load-bearing: a reallocation subject is written `move` like the other three
/// departing modes but leaves the ledger *after* the acquisition rather than
/// before it, so the two can no longer be told apart by position alone.
type ResolvedArguments = (
    Vec<ExternArgument>,
    Vec<ResourceId>,
    Vec<(String, ResourceId, CallParameter)>,
);

struct Lowering<'a> {
    callees: &'a BTreeMap<String, ExternCallee>,
    globals: &'a BTreeMap<String, SymbolId>,
    errors: &'a BTreeSet<String>,
    bindings: BTreeMap<String, ResourceId>,
    next_resource: ResourceId,
    diagnostics: Vec<SourceDiagnostic>,
}

impl Lowering<'_> {
    /// Resolve one call's arguments against the live bindings.
    ///
    /// `consume` is false on an adapter's error arm: that arm is a *path*, so
    /// its calls read the bindings this body has at the failing call and never
    /// remove them from the success path. The custody the arm discharges is
    /// accounted by `check_error_arm` over the checked event list, not by
    /// deleting a binding the success path still owns.
    fn arguments(
        &mut self,
        tokens: &[Token],
        span: Span,
        callee: &ExternCallee,
        consume: bool,
    ) -> Option<ResolvedArguments> {
        // Split on commas; nothing in this grammar nests, so a nested opener is
        // a refusal rather than something to descend into.
        let mut groups: Vec<Vec<&Token>> = vec![Vec::new()];
        for token in tokens {
            match &token.kind {
                TokenKind::Symbol(Symbol::Comma) => groups.push(Vec::new()),
                TokenKind::Symbol(Symbol::LeftParen | Symbol::RightParen) => {
                    self.diagnostics.push(bridging(
                        span,
                        "nested call arguments are not in this profile",
                    ));
                    return None;
                }
                _ => groups
                    .last_mut()
                    .expect("one group always exists")
                    .push(token),
            }
        }
        if groups.len() == 1 && groups[0].is_empty() {
            groups.clear();
        }
        if groups.len() != callee.parameters.len() {
            self.diagnostics.push(bridging(
                span,
                format!(
                    "`{}` declares {} parameters; the call passes {} arguments",
                    callee.name,
                    callee.parameters.len(),
                    groups.len()
                ),
            ));
            return None;
        }
        let mut arguments = Vec::new();
        let mut resources = Vec::new();
        let mut moved = Vec::new();
        for (index, group) in groups.iter().enumerate() {
            let declared = callee.parameters[index];
            let argument = match (group.as_slice(), declared) {
                ([token], CallParameter::Data(_))
                    if matches!(token.kind, TokenKind::Integer(_)) =>
                {
                    let TokenKind::Integer(value) = &token.kind else {
                        unreachable!("matched above")
                    };
                    match value.parse::<i64>() {
                        Ok(value) => ExternArgument::Data(value),
                        Err(_) => {
                            self.diagnostics.push(bridging(
                                span,
                                format!("scalar literal `{value}` does not fit the model"),
                            ));
                            return None;
                        }
                    }
                }
                ([minus, token], CallParameter::Data(_))
                    if minus.kind == TokenKind::Symbol(Symbol::Minus)
                        && matches!(token.kind, TokenKind::Integer(_)) =>
                {
                    let TokenKind::Integer(value) = &token.kind else {
                        unreachable!("matched above")
                    };
                    match format!("-{value}").parse::<i64>() {
                        Ok(value) => ExternArgument::Data(value),
                        Err(_) => {
                            self.diagnostics.push(bridging(
                                span,
                                format!("scalar literal `-{value}` does not fit the model"),
                            ));
                            return None;
                        }
                    }
                }
                ([token], CallParameter::Borrowed(_) | CallParameter::Owner) => {
                    let Some(name) = word(Some(token)) else {
                        self.diagnostics
                            .push(bridging(span, "a lent argument must be a resource binding"));
                        return None;
                    };
                    let Some(resource) = self.bindings.get(name).copied() else {
                        self.diagnostics.push(bridging(
                            span,
                            format!("`{name}` is not a live resource binding in this body"),
                        ));
                        return None;
                    };
                    resources.push(resource);
                    if declared == CallParameter::Owner {
                        // A declared store owner is a loan with a liveness
                        // precondition: the caller keeps the identity, so the
                        // binding is read, never removed.
                        ExternArgument::Owner(resource)
                    } else {
                        ExternArgument::Borrow(resource)
                    }
                }
                (
                    [keyword, token],
                    CallParameter::Consumed
                    | CallParameter::Stored { .. }
                    | CallParameter::Released
                    | CallParameter::ReallocSubject,
                ) if word(Some(keyword)) == Some("move") => {
                    let Some(name) = word(Some(token)) else {
                        self.diagnostics.push(bridging(
                            span,
                            "a moved argument must be a resource binding",
                        ));
                        return None;
                    };
                    let Some(resource) = (if consume {
                        self.bindings.remove(name)
                    } else {
                        self.bindings.get(name).copied()
                    }) else {
                        self.diagnostics.push(bridging(
                            span,
                            format!("`{name}` is not a live resource binding in this body"),
                        ));
                        return None;
                    };
                    resources.push(resource);
                    moved.push((name.to_owned(), resource, declared));
                    // Which of the four departing modes this is comes from the
                    // declaration, never from the call site, so admitting two
                    // more of them adds no source syntax and no reserved word.
                    match declared {
                        CallParameter::Stored { .. } => ExternArgument::Store(resource),
                        CallParameter::Released => ExternArgument::Release(resource),
                        CallParameter::ReallocSubject => ExternArgument::Realloc(resource),
                        _ => ExternArgument::Consume(resource),
                    }
                }
                ([token], CallParameter::OutSlot) if word(Some(token)) == Some("out") => {
                    ExternArgument::OutSlot
                }
                _ => {
                    self.diagnostics.push(bridging(
                        span,
                        format!(
                            "argument {index} does not bridge to the declared mode {declared:?}"
                        ),
                    ));
                    return None;
                }
            };
            arguments.push(argument);
        }
        Some((arguments, resources, moved))
    }
}

/// Lower one module. `callees` holds exactly the admitted contracts the driver
/// selected; a call to anything else never reaches here.
pub fn lower_source_extern(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    callees: &BTreeMap<String, ExternCallee>,
) -> Result<LoweredExternModule, Vec<SourceDiagnostic>> {
    let mut diagnostics = Vec::new();
    let errors = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ErrorType)
        .map(|declaration| declaration.value.name.clone())
        .collect::<BTreeSet<_>>();
    let errors = if errors.is_empty() {
        BTreeSet::from(["Failure".to_owned()])
    } else {
        errors
    };
    let entry_declaration = parsed
        .declarations
        .iter()
        .find(|declaration| {
            declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
        })
        .ok_or_else(|| {
            vec![unsupported(
                Span { start: 0, end: 0 },
                format!("entry function `{entry}` was not found"),
            )]
        })?;
    for declaration in &parsed.declarations {
        if declaration.value.kind == DeclarationKind::Function && declaration.value.name != entry {
            diagnostics.push(refused_context(
                declaration.span,
                "the extern-call profile models exactly one entry function; other source functions are not lowered on this route",
            ));
        }
    }
    let scope_span = entry_declaration.span;
    let declared_effects = match declared_effect_row(&entry_declaration.value, scope_span) {
        Ok(row) => row,
        Err(error) => {
            diagnostics.push(error);
            EffectRow::default()
        }
    };
    let Some(body) = &entry_declaration.value.body else {
        diagnostics.push(unsupported(
            scope_span,
            "the entry function requires a body",
        ));
        return Err(diagnostics);
    };
    if !entry_declaration.value.parameters.is_empty() {
        diagnostics.push(refused_context(
            scope_span,
            "an extern-calling entry function takes no parameters in this profile",
        ));
    }

    let mut lowering = Lowering {
        callees,
        globals: &typed.globals,
        errors: &errors,
        bindings: BTreeMap::new(),
        next_resource: 1,
        diagnostics: Vec::new(),
    };
    let mut events = Vec::new();
    let mut event_spans = Vec::new();
    let mut calls = Vec::new();
    let mut resources = Vec::new();
    let mut ownership = Vec::new();
    let mut error_operations = Vec::new();
    let mut source_operations = Vec::new();
    let mut extern_summaries = BTreeMap::new();
    let mut live = 0_u64;
    let mut peak = 0_u64;
    let mut total = 0_u64;

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            lowering.diagnostics.push(refused_context(
                span,
                "extern calls are admitted in a flat sequential entry body only; nested blocks, task workers, `with lock` bodies and error regions keep their existing refusals",
            ));
            continue;
        }
        let tokens = match statement_tokens(parsed, span) {
            Ok(tokens) => tokens,
            Err(error) => {
                lowering.diagnostics.push(error);
                continue;
            }
        };
        match &statement.value {
            StatementKind::Return => {
                if tokens.len() != 2 || word(tokens.first()) != Some("return") {
                    lowering
                        .diagnostics
                        .push(unsupported(span, "a unit result requires a bare `return;`"));
                    continue;
                }
                source_operations.push(SourceOp::Return(0));
                events.push(ExternEvent::Return);
                event_spans.push(span);
            }
            StatementKind::Let {
                binding: Some(name),
            } => {
                let Some(parsed_call) = parse_adapter_statement(&tokens) else {
                    lowering.diagnostics.push(adapter(
                        span,
                        "an extern acquisition must read `let NAME = CALLEE(ARGS) succeeds BOOL else raise ERROR;`",
                    ));
                    continue;
                };
                let Some(callee) = lowering.callees.get(&parsed_call.callee).cloned() else {
                    lowering.diagnostics.push(diagnostic(
                        "Q023",
                        SemanticPass::ExternCall,
                        span,
                        format!(
                            "`{}` has no admitted declared contract in the selected extern contract environment",
                            parsed_call.callee
                        ),
                    ));
                    continue;
                };
                if !callee.acquires() {
                    lowering.diagnostics.push(adapter(
                        span,
                        format!(
                            "`{}` establishes no fresh custody, so it takes no adapter and binds no resource",
                            callee.name
                        ),
                    ));
                    continue;
                }
                if !lowering.errors.contains(&parsed_call.error) {
                    lowering.diagnostics.push(adapter(
                        span,
                        format!(
                            "the adapter raises undeclared error type `{}`",
                            parsed_call.error
                        ),
                    ));
                    continue;
                }
                // The error arm is resolved *before* the call's own arguments,
                // against the bindings this body holds when the call is
                // attempted: the arm is the path taken because the call failed,
                // so it sees exactly what was live at the attempt.
                let mut releases = Vec::new();
                let mut arm_refused = false;
                for (name, release_tokens) in &parsed_call.releases {
                    let Some(released) = lowering.callees.get(name).cloned() else {
                        lowering.diagnostics.push(diagnostic(
                            "Q023",
                            SemanticPass::ExternCall,
                            span,
                            format!(
                                "the error-arm disposition calls `{name}`, which has no admitted declared contract in the selected extern contract environment"
                            ),
                        ));
                        arm_refused = true;
                        continue;
                    };
                    if lowering.globals.get(&released.name).is_none() {
                        lowering.diagnostics.push(diagnostic(
                            "Q027",
                            SemanticPass::ExternCall,
                            span,
                            format!(
                                "`{}` has a selected contract but no `extern fn` declaration in this module",
                                released.name
                            ),
                        ));
                        arm_refused = true;
                        continue;
                    }
                    // The arm's calls are calls this module makes, so the
                    // entry's declared row must admit them; an error path is
                    // not an exemption from the effect discipline.
                    if !derived_effects(&released).is_subset_of(&declared_effects) {
                        lowering.diagnostics.push(adapter(
                            span,
                            format!(
                                "the error-arm disposition calls `{}`, whose derived effect row the entry's declared row does not admit",
                                released.name
                            ),
                        ));
                        arm_refused = true;
                        continue;
                    }
                    let Some((release_arguments, ..)) =
                        lowering.arguments(release_tokens, span, &released, false)
                    else {
                        arm_refused = true;
                        continue;
                    };
                    releases.push(qcore_core::extern_call::ExternRelease {
                        callee: released.name.clone(),
                        arguments: release_arguments,
                    });
                }
                if arm_refused {
                    continue;
                }
                let Some((arguments, argument_resources, moved)) =
                    lowering.arguments(&parsed_call.arguments, span, &callee, true)
                else {
                    continue;
                };
                // An acquiring call may also carry departing custody. Those
                // departures are recorded before the acquisition, in the order
                // the ledger sees them; the core checker still refuses the
                // adapter's error arm while anything remains live.
                //
                // A reallocation subject is the exception, and it is recorded
                // *after* the acquisition on purpose. `cleanup_lowering` gives a
                // failed acquisition the cleanup edge over everything live at
                // that point, and contract spec §C04.3 says the subject is
                // exactly that: preserved on the null arm, replaced on the
                // success arm. Dropping it before the acquire would tell the
                // sequential passes it was gone on a path where it is not.
                for (name, resource, declared) in &moved {
                    if matches!(declared, CallParameter::ReallocSubject) {
                        continue;
                    }
                    ownership.push(OwnershipOp::TransferOut(name.clone()));
                    error_operations.push(ErrorOp::Drop(*resource));
                    source_operations.push(SourceOp::Drop(*resource));
                    live = live.saturating_sub(1);
                }
                let resource = lowering.next_resource;
                lowering.next_resource += 1;
                lowering.bindings.insert(name.clone(), resource);
                resources.push(resource);
                record_call(
                    &mut extern_summaries,
                    lowering.globals,
                    &callee,
                    &mut calls,
                    span,
                    argument_resources,
                    Some(resource),
                    &mut lowering.diagnostics,
                );
                ownership.push(OwnershipOp::Acquire {
                    id: resource,
                    owner: name.clone(),
                    resource_type: "Resource".into(),
                    footprint: Footprint::single("resources", 1),
                });
                error_operations.push(ErrorOp::Acquire(resource));
                source_operations.push(SourceOp::Acquire {
                    resource,
                    succeeds: parsed_call.succeeds,
                    error: parsed_call.error.clone(),
                });
                live += 1;
                peak = peak.max(live);
                total += 1;
                // The replacement happens exactly once and only on the success
                // path, which is precisely where a `SourceOp::Drop` placed after
                // a `SourceOp::Acquire` sits: a failed acquisition diverts to
                // the cleanup edge before reaching it. The declared peak is an
                // upper bound, so counting the replaced and the established
                // identity as briefly coexisting is conservative.
                for (name, subject, declared) in &moved {
                    if matches!(declared, CallParameter::ReallocSubject) {
                        ownership.push(OwnershipOp::TransferOut(name.clone()));
                        error_operations.push(ErrorOp::Drop(*subject));
                        source_operations.push(SourceOp::Drop(*subject));
                        live = live.saturating_sub(1);
                    }
                }
                events.push(ExternEvent::Call(ExternCall {
                    callee: callee.name.clone(),
                    arguments,
                    adapter: Some(ExternAdapter {
                        resource,
                        succeeds: parsed_call.succeeds,
                        releases,
                        error: parsed_call.error,
                    }),
                }));
                event_spans.push(span);
            }
            StatementKind::Expression => {
                let Some(parsed_call) = parse_plain_call(&tokens) else {
                    lowering.diagnostics.push(unsupported(
                        span,
                        "the extern-call profile admits `CALLEE(ARGS);`, an acquisition `let` and a bare `return;`",
                    ));
                    continue;
                };
                let Some(callee) = lowering.callees.get(&parsed_call.callee).cloned() else {
                    lowering.diagnostics.push(diagnostic(
                        "Q023",
                        SemanticPass::ExternCall,
                        span,
                        format!(
                            "`{}` has no admitted declared contract in the selected extern contract environment",
                            parsed_call.callee
                        ),
                    ));
                    continue;
                };
                if callee.acquires() {
                    lowering.diagnostics.push(adapter(
                        span,
                        format!(
                            "`{}` establishes fresh custody; its failure arm must be branched with `let NAME = … succeeds BOOL else raise ERROR;`",
                            callee.name
                        ),
                    ));
                    continue;
                }
                let Some((arguments, argument_resources, moved)) =
                    lowering.arguments(&parsed_call.arguments, span, &callee, true)
                else {
                    continue;
                };
                // Consumption discharges the caller's obligation, which is
                // exactly what `resource_consumed` declares. It is not a claim
                // that the callee destroys the resource (contract spec §C04.2).
                // A stored argument has the same caller-side delta; where its
                // custody went is declared, recorded and premised, not checked.
                // A `resource_released` argument has the same delta again, and
                // is the one of the three that does claim destruction — which is
                // why it is premised separately rather than folded in here. A
                // reallocation subject cannot occur on this statement form: its
                // callee acquires, so it is refused above without an adapter.
                for (name, resource, _) in &moved {
                    ownership.push(OwnershipOp::TransferOut(name.clone()));
                    error_operations.push(ErrorOp::Drop(*resource));
                    source_operations.push(SourceOp::Drop(*resource));
                    live = live.saturating_sub(1);
                }
                record_call(
                    &mut extern_summaries,
                    lowering.globals,
                    &callee,
                    &mut calls,
                    span,
                    argument_resources,
                    None,
                    &mut lowering.diagnostics,
                );
                events.push(ExternEvent::Call(ExternCall {
                    callee: callee.name.clone(),
                    arguments,
                    adapter: None,
                }));
                event_spans.push(span);
            }
            _ => {
                lowering.diagnostics.push(unsupported(
                    span,
                    "statement is not in the extern-call executable subset",
                ));
            }
        }
    }
    diagnostics.extend(lowering.diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // The call sites are checked before any semantics are assembled, so an
    // extern-specific refusal is reported under its own code instead of
    // surfacing later as a generic lowering failure.
    let program = ExternProgram {
        entry: entry.to_owned(),
        callees: callees.clone(),
        events,
        errors: errors.clone(),
    };
    if let Err(errors) = qcore_core::extern_call::check_extern_program(&program) {
        return Err(errors
            .into_iter()
            .map(|error| {
                let span = event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(scope_span);
                diagnostic(
                    error.code(),
                    SemanticPass::ExternCall,
                    span,
                    error.to_string(),
                )
            })
            .collect());
    }
    let lowering_program = LoweringProgram::new(
        entry.to_owned(),
        resources.clone(),
        errors.iter().cloned(),
        source_operations,
    )
    .map_err(|error| {
        vec![diagnostic(
            "Q202",
            SemanticPass::CleanupLowering,
            scope_span,
            error.to_string(),
        )]
    })?;
    // One flow segment before, between and after every call; the acquisitions
    // and releases live in the callees' declared summaries, not here.
    let resource_segments = vec![Flow::zero(); calls.len() + 1];
    let semantics = FunctionSemantics {
        symbol: typed.globals[entry],
        usage_declarations: BTreeMap::new(),
        usage_expression: UsageExpr::Sequence(Vec::new()),
        ownership_operations: ownership,
        resource_segments,
        resource_bounds: ResourceBounds::new(
            Quantity::single("resources", peak),
            Quantity::single("allocations", total),
        ),
        allowed_effects: declared_effects.clone(),
        initial_error_ledger: ErrorLedger::default(),
        error_operations,
        region_operations: Vec::new(),
        lowering: lowering_program,
        calls,
        identity_entry: Vec::new(),
        identity_exit: Vec::new(),
        parameters: Vec::new(),
        recursive_contract: None,
    };
    Ok(LoweredExternModule {
        entry: entry.to_owned(),
        program,
        event_spans,
        scope_span,
        semantics,
        extern_summaries,
        declared_effects,
    })
}

#[allow(clippy::too_many_arguments)]
fn record_call(
    extern_summaries: &mut BTreeMap<SymbolId, FunctionSummary>,
    globals: &BTreeMap<String, SymbolId>,
    callee: &ExternCallee,
    calls: &mut Vec<CallSite>,
    span: Span,
    arguments: Vec<ResourceId>,
    result: Option<ResourceId>,
    diagnostics: &mut Vec<SourceDiagnostic>,
) {
    let Some(symbol) = globals.get(&callee.name).copied() else {
        diagnostics.push(diagnostic(
            "Q027",
            SemanticPass::ExternCall,
            span,
            format!(
                "`{}` has a selected contract but no `extern fn` declaration in this module",
                callee.name
            ),
        ));
        return;
    };
    extern_summaries
        .entry(symbol)
        .or_insert_with(|| extern_summary(callee));
    calls.push(CallSite {
        callee: symbol,
        arguments,
        result,
        span,
    });
}

/// The interprocedural summary of an admitted extern callee, derived from the
/// declared contract alone. This is the declared assumption the caller's passes
/// consume; it is never inferred from a body, because there is no body.
#[must_use]
pub fn extern_summary(callee: &ExternCallee) -> FunctionSummary {
    let parameters = derived_custody(callee)
        .into_iter()
        .map(|custody| match custody {
            // A declared store owner is a loan: the caller supplies live
            // custody and keeps the identity.
            ExternCustody::Borrowed | ExternCustody::Owner => ParameterOwnership::Borrowed,
            // Every departing mode has the same *caller-side* delta, so the
            // interprocedural summary the sequential passes consume is the same
            // for all four. What differs between them is what the declaration
            // claims about where the custody went, which is carried on the
            // extern certificate's own fields and premises, not here.
            ExternCustody::Consumed
            | ExternCustody::Stored
            | ExternCustody::Released
            | ExternCustody::Reallocated => ParameterOwnership::Consumed,
        })
        .collect::<Vec<_>>();
    let consumed = parameters
        .iter()
        .filter(|mode| **mode == ParameterOwnership::Consumed)
        .count() as u64;
    let mut flow = Flow::zero();
    if consumed > 0 {
        flow = flow
            .then(&Flow::release(
                Quantity::single("resources", consumed),
                Quantity::zero(),
            ))
            .expect("a release composes with the zero flow");
    }
    if callee.acquires() {
        flow = flow
            .then(&Flow::acquire(
                Quantity::single("resources", 1),
                Quantity::single("allocations", 1),
            ))
            .expect("an acquisition composes with a release");
    }
    FunctionSummary {
        effects: derived_effects(callee),
        flow,
        parameters,
    }
}

struct ParsedCall {
    callee: String,
    arguments: Vec<Token>,
    succeeds: bool,
    /// The error arm's dispositions, in written order: `(callee, arguments)`.
    releases: Vec<(String, Vec<Token>)>,
    error: String,
}

fn call_head(tokens: &[Token], start: usize) -> Option<(String, Vec<Token>, usize)> {
    let name = word(tokens.get(start))?.to_owned();
    if !symbol(tokens.get(start + 1), Symbol::LeftParen) {
        return None;
    }
    let close = tokens
        .iter()
        .enumerate()
        .skip(start + 2)
        .find(|(_, token)| token.kind == TokenKind::Symbol(Symbol::RightParen))
        .map(|(index, _)| index)?;
    Some((name, tokens[start + 2..close].to_vec(), close + 1))
}

fn parse_adapter_statement(tokens: &[Token]) -> Option<ParsedCall> {
    if word(tokens.first()) != Some("let") || !symbol(tokens.get(2), Symbol::Equal) {
        return None;
    }
    let (callee, arguments, next) = call_head(tokens, 3)?;
    if word(tokens.get(next)) != Some("succeeds") {
        return None;
    }
    let succeeds = match word(tokens.get(next + 1)) {
        Some("true") => true,
        Some("false") => false,
        _ => return None,
    };
    if word(tokens.get(next + 2)) != Some("else") {
        return None;
    }
    // Card `EXTERN-ERR-REL`: between `else` and `raise` the handler names the
    // dispositions of every other live identity. Zero of them is the v1 shape.
    let mut cursor = next + 3;
    let mut releases = Vec::new();
    while word(tokens.get(cursor)) != Some("raise") {
        let (name, release_arguments, after) = call_head(tokens, cursor)?;
        releases.push((name, release_arguments));
        cursor = after;
    }
    let error = word(tokens.get(cursor + 1))?.to_owned();
    if !symbol(tokens.get(cursor + 2), Symbol::Semicolon) || tokens.len() != cursor + 3 {
        return None;
    }
    Some(ParsedCall {
        callee,
        arguments,
        succeeds,
        releases,
        error,
    })
}

fn parse_plain_call(tokens: &[Token]) -> Option<ParsedCall> {
    let (callee, arguments, next) = call_head(tokens, 0)?;
    if !symbol(tokens.get(next), Symbol::Semicolon) || tokens.len() != next + 1 {
        return None;
    }
    Some(ParsedCall {
        callee,
        arguments,
        succeeds: true,
        releases: Vec::new(),
        error: String::new(),
    })
}
