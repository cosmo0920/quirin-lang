//! Cycle 12 source-file compiler orchestration.
//!
//! This is intentionally a conservative vertical slice.  It accepts the flat,
//! executable source forms for which the existing cleanup CFG has exact
//! semantics and rejects every other body form before C emission.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::cleanup_lowering::{SourceOp, SourceProgram as LoweringProgram, interpret_source};
use crate::frontend::{
    DeclarationKind, Located, SemanticPass, SourceDeclaration, SourceDiagnostic, SourceProgram,
    StatementKind, TypedSourceDeclaration, TypedSourceProgram, elaborate_source,
    parse_source_program,
};
use crate::lexer::{Span, Symbol, Token, TokenKind, lex};
use crate::ownership::{Footprint, OwnershipOp, ResourceId};
use crate::resource_flow::{Flow, Quantity, ResourceBounds};
use crate::typed_errors::{Effect, EffectRow, ErrorDeclarations, ErrorLedger, ErrorOp};
use crate::usage::UsageExpr;
use crate::whole_program::{
    CallSite, CheckedProgram, FunctionSemantics, FunctionSummary, ParameterOwnership,
    check_whole_program,
};
use crate::{ExecutionObservation, emit_c11};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub source: String,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub entry: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub line_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiagnostic {
    pub code: String,
    pub pass: SemanticPass,
    pub location: SourceLocation,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedSourceMap {
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub source: SourceLocation,
}

#[derive(Clone, Debug)]
pub struct CompiledModule {
    pub checked: CheckedProgram,
    pub entry: String,
    pub c_source: String,
    pub source_map: Vec<EmittedSourceMap>,
    pub reference_observation: ExecutionObservation,
}

#[derive(Debug)]
struct DriverError(String);

impl fmt::Display for DriverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DriverError {}

#[derive(Clone, Debug)]
struct FileRange {
    path: String,
    start: usize,
    end: usize,
    source: String,
}

/// Parses one or more source files as a module, checks the whole program, and
/// emits a standalone C11 translation unit for the selected entry function.
pub fn compile_files(
    files: &[SourceFile],
    options: &CompileOptions,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if files.is_empty() {
        return Err(vec![plain_diagnostic(
            "Q400",
            "no input files were provided",
        )]);
    }
    let (combined, ranges) = combine_files(files);
    let parsed = parse_source_program(&combined).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q000",
            SemanticPass::NameResolution,
            error.span,
            error.message,
            &ranges,
        )]
    })?;
    let typed = elaborate_source(&parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })?;
    let error_names = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ErrorType)
        .map(|declaration| declaration.value.name.clone())
        .collect::<BTreeSet<_>>();
    let error_names = if error_names.is_empty() {
        BTreeSet::from(["Failure".to_owned()])
    } else {
        error_names
    };
    let mut semantics = synthesize_semantics(&parsed, &typed, &error_names)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, &ranges)])?;
    mark_recursive_contracts(&mut semantics);
    let semantic_calls = semantics
        .iter()
        .map(|function| (function.symbol, function.calls.clone()))
        .collect::<BTreeMap<_, _>>();
    let checked = check_whole_program(
        &typed,
        &ErrorDeclarations::new(error_names.iter().cloned()),
        semantics,
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })?;

    let entry = select_entry(&typed, options).map_err(|diagnostic| vec![diagnostic])?;
    let symbol = typed.globals[&entry];
    let function = &checked.functions[&symbol];
    validate_emittable_calls(symbol, &semantic_calls, &checked)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, &ranges)])?;
    if !function.source.parameters.is_empty() {
        return Err(vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            function.source.span,
            "the emitted entry function must not have parameters".into(),
            &ranges,
        )]);
    }
    let reference_observation = crate::interpret_cfg(&function.cfg).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q202",
            SemanticPass::CleanupLowering,
            function.source.span,
            error.to_string(),
            &ranges,
        )]
    })?;
    let raw_c = emit_c11(&function.cfg).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q500",
            SemanticPass::CEmission,
            function.source.span,
            error.to_string(),
            &ranges,
        )]
    })?;
    let source_location = locate(function.source.span.start, &ranges);
    let (c_source, source_map) = add_source_mapping(raw_c, &entry, source_location);
    Ok(CompiledModule {
        checked,
        entry,
        c_source,
        source_map,
        reference_observation,
    })
}

fn combine_files(files: &[SourceFile]) -> (String, Vec<FileRange>) {
    let mut combined = String::new();
    let mut ranges = Vec::new();
    for file in files {
        let start = combined.len();
        combined.push_str(&file.source);
        let end = combined.len();
        ranges.push(FileRange {
            path: file.path.clone(),
            start,
            end,
            source: file.source.clone(),
        });
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
    }
    (combined, ranges)
}

fn synthesize_semantics(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    errors: &BTreeSet<String>,
) -> Result<Vec<FunctionSemantics>, SourceDiagnostic> {
    parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .map(|declaration| {
            let typed_declaration = typed
                .declarations
                .iter()
                .find(|candidate| candidate.name == declaration.value.name)
                .expect("elaboration preserves declarations");
            synthesize_function(parsed, declaration, typed_declaration, typed, errors)
        })
        .collect()
}

fn synthesize_function(
    parsed: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed_declaration: &TypedSourceDeclaration,
    typed: &TypedSourceProgram,
    errors: &BTreeSet<String>,
) -> Result<FunctionSemantics, SourceDiagnostic> {
    let Some(body) = &declaration.value.body else {
        return Err(diagnostic(
            SemanticPass::CleanupLowering,
            declaration.span,
            "source function requires a body",
        ));
    };
    let mut resources = Vec::new();
    let mut owners = BTreeMap::<String, ResourceId>::new();
    let mut lowering = Vec::new();
    let mut ownership = Vec::new();
    let mut error_operations = Vec::new();
    let mut calls = Vec::new();
    let mut resource_segments = Vec::new();
    let mut segment = Flow::zero();
    let mut peak_live = 0_u64;
    let mut live = 0_u64;
    let mut total = 0_u64;

    for statement in &body.statements {
        if !statement.value.nested_blocks().is_empty() {
            return Err(diagnostic(
                SemanticPass::CEmission,
                statement.span,
                "structured control flow is not in the Cycle 12 executable subset",
            ));
        }
        let tokens = statement_tokens(parsed.source(), statement.span)?;
        match &statement.value {
            StatementKind::Let {
                binding: Some(binding),
            } => {
                let (resource, succeeds, error) = parse_acquire(&tokens).ok_or_else(|| {
                    diagnostic(SemanticPass::CEmission, statement.span, "`let` must use `qcore_acquire(resource_id, succeeds, ErrorType)` in the executable subset")
                })?;
                if !errors.contains(&error) {
                    return Err(diagnostic(
                        SemanticPass::TypedErrors,
                        statement.span,
                        format!("unknown error type `{error}`"),
                    ));
                }
                resources.push(resource);
                owners.insert(binding.clone(), resource);
                lowering.push(SourceOp::Acquire {
                    resource,
                    succeeds,
                    error,
                });
                ownership.push(OwnershipOp::Acquire {
                    id: resource,
                    owner: binding.clone(),
                    resource_type: "Resource".into(),
                    footprint: Footprint::single("resources", 1),
                });
                error_operations.push(ErrorOp::Acquire(resource));
                segment = segment
                    .then(&Flow::acquire(
                        Quantity::single("resources", 1),
                        Quantity::single("allocations", 1),
                    ))
                    .map_err(|error| {
                        diagnostic(
                            SemanticPass::ResourceFlow,
                            statement.span,
                            error.to_string(),
                        )
                    })?;
                live = live.saturating_add(1);
                peak_live = peak_live.max(live);
                total = total.saturating_add(1);
            }
            StatementKind::Expression => {
                if let Some((passes, error)) = parse_check(&tokens) {
                    if !errors.contains(&error) {
                        return Err(diagnostic(
                            SemanticPass::TypedErrors,
                            statement.span,
                            format!("unknown error type `{error}`"),
                        ));
                    }
                    lowering.push(SourceOp::Check { passes, error });
                } else if let Some(callee_name) = parse_zero_argument_call(&tokens) {
                    let Some(&callee) = typed.globals.get(&callee_name) else {
                        return Err(diagnostic(
                            SemanticPass::NameResolution,
                            statement.span,
                            format!("unknown function `{callee_name}`"),
                        ));
                    };
                    let callee_declaration =
                        typed.declarations.iter().find(|item| item.symbol == callee);
                    if !callee_declaration
                        .is_some_and(|item| item.kind == DeclarationKind::Function)
                    {
                        return Err(diagnostic(
                            SemanticPass::OrdinaryTypes,
                            statement.span,
                            "only source functions may be called in the executable subset",
                        ));
                    }
                    if callee_declaration.is_some_and(|item| !item.parameters.is_empty()) {
                        return Err(diagnostic(
                            SemanticPass::OrdinaryTypes,
                            statement.span,
                            "executable source calls must have zero arguments",
                        ));
                    }
                    resource_segments.push(segment);
                    segment = Flow::zero();
                    calls.push(CallSite {
                        callee,
                        arguments: Vec::new(),
                        span: statement.span,
                    });
                } else {
                    return Err(diagnostic(
                        SemanticPass::CEmission,
                        statement.span,
                        "unsupported expression statement in the Cycle 12 executable subset",
                    ));
                }
            }
            StatementKind::Drop { place: Some(place) } => {
                let resource = *owners.get(place).ok_or_else(|| {
                    diagnostic(
                        SemanticPass::Ownership,
                        statement.span,
                        format!("unknown resource binding `{place}`"),
                    )
                })?;
                lowering.push(SourceOp::Drop(resource));
                ownership.push(OwnershipOp::Drop(place.clone()));
                error_operations.push(ErrorOp::Drop(resource));
                segment = segment
                    .then(&Flow::release(
                        Quantity::single("resources", 1),
                        Quantity::zero(),
                    ))
                    .map_err(|error| {
                        diagnostic(
                            SemanticPass::ResourceFlow,
                            statement.span,
                            error.to_string(),
                        )
                    })?;
                live = live.saturating_sub(1);
            }
            StatementKind::Move { place: Some(place) } => {
                let resource = *owners.get(place).ok_or_else(|| {
                    diagnostic(
                        SemanticPass::Ownership,
                        statement.span,
                        format!("unknown resource binding `{place}`"),
                    )
                })?;
                lowering.push(SourceOp::TransferToResult(resource));
                ownership.push(OwnershipOp::TransferOut(place.clone()));
                error_operations.push(ErrorOp::Drop(resource));
                segment = segment
                    .then(&Flow::release(
                        Quantity::single("resources", 1),
                        Quantity::zero(),
                    ))
                    .map_err(|error| {
                        diagnostic(
                            SemanticPass::ResourceFlow,
                            statement.span,
                            error.to_string(),
                        )
                    })?;
                live = live.saturating_sub(1);
            }
            StatementKind::Return => {
                let value = parse_return(&tokens).ok_or_else(|| {
                    diagnostic(
                        SemanticPass::OrdinaryTypes,
                        statement.span,
                        "return value must be a signed 32-bit integer literal",
                    )
                })?;
                lowering.push(SourceOp::Return(value));
            }
            _ => {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    statement.span,
                    "statement is not in the Cycle 12 executable subset",
                ));
            }
        }
    }
    resource_segments.push(segment);
    let lowering = LoweringProgram::new(
        declaration.value.name.clone(),
        resources.clone(),
        errors.iter().cloned(),
        lowering,
    )
    .map_err(|error| {
        diagnostic(
            SemanticPass::CleanupLowering,
            declaration.span,
            error.to_string(),
        )
    })?;
    // Exercise the source interpreter at the synthesis boundary as well as the
    // whole-program translation validator.
    interpret_source(&lowering).map_err(|error| {
        diagnostic(
            SemanticPass::CleanupLowering,
            declaration.span,
            error.to_string(),
        )
    })?;
    let parameters = typed_declaration
        .parameters
        .iter()
        .filter_map(|parameter| match parameter.type_kind {
            crate::SourceTypeKind::Borrow => Some(ParameterOwnership::Borrowed),
            crate::SourceTypeKind::Owned => Some(ParameterOwnership::Consumed),
            _ => None,
        })
        .collect();
    Ok(FunctionSemantics {
        symbol: typed_declaration.symbol,
        usage_declarations: BTreeMap::new(),
        usage_expression: UsageExpr::Unit,
        ownership_operations: ownership,
        resource_segments,
        resource_bounds: ResourceBounds::new(
            Quantity::single("resources", peak_live),
            Quantity::single("allocations", total),
        ),
        allowed_effects: EffectRow::new([Effect::Alloc]),
        initial_error_ledger: ErrorLedger::default(),
        error_operations,
        region_operations: Vec::new(),
        lowering,
        calls,
        identity_entry: Vec::new(),
        identity_exit: Vec::new(),
        parameters,
        recursive_contract: None,
    })
}

fn mark_recursive_contracts(functions: &mut [FunctionSemantics]) {
    let graph = functions
        .iter()
        .map(|function| {
            (
                function.symbol,
                function
                    .calls
                    .iter()
                    .map(|call| call.callee)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for function in functions {
        if reaches(
            function.symbol,
            function.symbol,
            &graph,
            &mut BTreeSet::new(),
        ) {
            function.recursive_contract = Some(FunctionSummary {
                effects: EffectRow::new([Effect::Alloc]),
                flow: Flow::zero(),
                parameters: function.parameters.clone(),
            });
        }
    }
}

fn reaches(
    start: crate::SourceSymbolId,
    target: crate::SourceSymbolId,
    graph: &BTreeMap<crate::SourceSymbolId, Vec<crate::SourceSymbolId>>,
    visited: &mut BTreeSet<crate::SourceSymbolId>,
) -> bool {
    graph.get(&start).is_some_and(|next| {
        next.iter().any(|symbol| {
            *symbol == target
                || (visited.insert(*symbol) && reaches(*symbol, target, graph, visited))
        })
    })
}

fn validate_emittable_calls(
    entry: crate::SourceSymbolId,
    calls: &BTreeMap<crate::SourceSymbolId, Vec<CallSite>>,
    checked: &CheckedProgram,
) -> Result<(), SourceDiagnostic> {
    fn visit(
        symbol: crate::SourceSymbolId,
        calls: &BTreeMap<crate::SourceSymbolId, Vec<CallSite>>,
        checked: &CheckedProgram,
        visited: &mut BTreeSet<crate::SourceSymbolId>,
    ) -> Result<(), SourceDiagnostic> {
        if !visited.insert(symbol) {
            return Ok(());
        }
        for call in calls.get(&symbol).into_iter().flatten() {
            if checked.recursive_functions.contains(&call.callee) {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    call.span,
                    "recursive calls are not in the Cycle 12 executable C subset",
                ));
            }
            let callee = &checked.functions[&call.callee];
            let observation = crate::interpret_cfg(&callee.cfg).map_err(|error| {
                diagnostic(SemanticPass::CleanupLowering, call.span, error.to_string())
            })?;
            let observationally_pure = observation.trace.is_empty()
                && matches!(
                    observation.outcome,
                    crate::ExecutionOutcome::Ok { payload, .. } if payload.is_empty()
                );
            if !observationally_pure {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    call.span,
                    "only observationally pure source calls can be emitted in Cycle 12",
                ));
            }
            visit(call.callee, calls, checked, visited)?;
        }
        Ok(())
    }
    visit(entry, calls, checked, &mut BTreeSet::new())
}

fn statement_tokens(source: &str, span: Span) -> Result<Vec<Token>, SourceDiagnostic> {
    let slice = &source[span.start..span.end];
    let mut tokens =
        lex(slice).map_err(|error| diagnostic(SemanticPass::CEmission, span, error.to_string()))?;
    tokens.pop();
    Ok(tokens)
}

fn parse_acquire(tokens: &[Token]) -> Option<(ResourceId, bool, String)> {
    let equal = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))?;
    let args = &tokens[equal + 1..];
    match args {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Integer(id),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(succeeds),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(error),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if name == "qcore_acquire" => {
            Some((id.parse().ok()?, parse_bool(succeeds)?, error.clone()))
        }
        _ => None,
    }
}

fn parse_check(tokens: &[Token]) -> Option<(bool, String)> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Word(passes),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(error),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if name == "qcore_check" => Some((parse_bool(passes)?, error.clone())),
        _ => None,
    }
}

fn parse_zero_argument_call(tokens: &[Token]) -> Option<String> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] => Some(name.clone()),
        _ => None,
    }
}

fn parse_return(tokens: &[Token]) -> Option<i32> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" => Some(0),
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Integer(value),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" => value.parse().ok(),
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Minus),
                ..
            },
            Token {
                kind: TokenKind::Integer(value),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" => value.parse::<i32>().ok()?.checked_neg(),
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn select_entry(
    typed: &TypedSourceProgram,
    options: &CompileOptions,
) -> Result<String, FileDiagnostic> {
    let functions = typed
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == DeclarationKind::Function)
        .collect::<Vec<_>>();
    let requested = options
        .entry
        .as_deref()
        .or_else(|| typed.globals.contains_key("main").then_some("main"));
    let selected = match requested {
        Some(name) => functions
            .iter()
            .find(|function| function.name == name)
            .copied(),
        None if functions.len() == 1 => functions.first().copied(),
        None => None,
    };
    selected
        .map(|function| function.name.clone())
        .ok_or_else(|| {
            plain_diagnostic(
                "Q401",
                match requested {
                    Some(name) => format!("entry function `{name}` was not found"),
                    None => "module has no unambiguous entry function; pass `--entry NAME`".into(),
                },
            )
        })
}

fn add_source_mapping(
    mut c: String,
    entry: &str,
    source: SourceLocation,
) -> (String, Vec<EmittedSourceMap>) {
    let marker = format!("static struct qcore_result qcore_fn_{entry}(void)");
    let marker_offset = c.find(&marker).unwrap_or(0);
    let generated_start_line = c[..marker_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 2;
    let function_lines = c[marker_offset..].find("\n}\n\nint main").map_or(1, |end| {
        c[marker_offset..marker_offset + end + 2].lines().count()
    });
    let directive = format!(
        "#line {} \"{}\"\n",
        source.line,
        escape_c_path(&source.path)
    );
    if let Some(offset) = c.find(&marker) {
        c.insert_str(offset, &directive);
    }
    let generated_end_line = generated_start_line + function_lines.saturating_sub(1);
    (
        c,
        vec![EmittedSourceMap {
            generated_start_line,
            generated_end_line,
            source,
        }],
    )
}

fn escape_c_path(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

fn diagnostic(pass: SemanticPass, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    let error = DriverError(message.into());
    crate::source_diagnostic(pass, span, &error)
}

fn map_diagnostic(diagnostic: SourceDiagnostic, ranges: &[FileRange]) -> FileDiagnostic {
    map_raw_diagnostic(
        diagnostic.code,
        diagnostic.pass,
        diagnostic.span,
        diagnostic.message,
        ranges,
    )
}

fn map_raw_diagnostic(
    code: &str,
    pass: SemanticPass,
    span: Span,
    message: String,
    ranges: &[FileRange],
) -> FileDiagnostic {
    FileDiagnostic {
        code: code.into(),
        pass,
        location: locate(span.start, ranges),
        message,
    }
}

fn locate(offset: usize, ranges: &[FileRange]) -> SourceLocation {
    let range = ranges
        .iter()
        .find(|range| range.start <= offset && offset <= range.end)
        .unwrap_or(&ranges[0]);
    let local = offset.saturating_sub(range.start).min(range.source.len());
    let before = &range.source[..local];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    let line_source = range.source[local.saturating_sub(local - line_start)..]
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();
    SourceLocation {
        path: range.path.clone(),
        line,
        column,
        line_source,
    }
}

fn plain_diagnostic(code: &str, message: impl Into<String>) -> FileDiagnostic {
    FileDiagnostic {
        code: code.into(),
        pass: SemanticPass::NameResolution,
        location: SourceLocation {
            path: "<command-line>".into(),
            line: 1,
            column: 1,
            line_source: String::new(),
        },
        message: message.into(),
    }
}

pub fn format_diagnostic(diagnostic: &FileDiagnostic) -> String {
    let mut output = format!(
        "{}:{}:{}: {}: {}",
        diagnostic.location.path,
        diagnostic.location.line,
        diagnostic.location.column,
        diagnostic.code,
        diagnostic.message
    );
    if !diagnostic.location.line_source.is_empty() {
        output.push('\n');
        output.push_str(&diagnostic.location.line_source);
        output.push('\n');
        output.push_str(&" ".repeat(diagnostic.location.column.saturating_sub(1)));
        output.push('^');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionOutcome, TraceEvent, parse_c_observation};
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn compile(source: &str) -> CompiledModule {
        compile_files(
            &[SourceFile::new("main.qc", source)],
            &CompileOptions::default(),
        )
        .unwrap()
    }

    #[test]
    fn compiles_values_and_maps_emitted_source() {
        let module = compile("fn main() -> i32 { return 42; }");
        assert_eq!(
            module.reference_observation.outcome,
            ExecutionOutcome::Ok {
                value: 42,
                payload: Vec::new()
            }
        );
        assert!(module.c_source.contains("#line 1 \"main.qc\""));
        assert_eq!(module.source_map[0].source.path, "main.qc");
    }

    #[test]
    fn combines_module_files_and_checks_calls() {
        let files = [
            SourceFile::new("leaf.qc", "fn leaf() -> i32 { return 1; }"),
            SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
        ];
        let module = compile_files(&files, &CompileOptions::default()).unwrap();
        assert_eq!(module.checked.functions.len(), 2);
        assert_eq!(module.entry, "main");
    }

    #[test]
    fn rejects_observable_calls_that_the_reduced_c_cfg_cannot_encode() {
        let files = [
            SourceFile::new(
                "leaf.qc",
                "error Failure { Failed; } fn leaf() -> i32 { qcore_check(false, Failure); return 1; }",
            ),
            SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
        ];
        let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
        assert_eq!(errors[0].code, "Q500");
        assert_eq!(errors[0].location.path, "main.qc");
    }

    #[test]
    fn synthesized_quantitative_bound_covers_the_resource_trace() {
        let module = compile(
            "error Failure { Failed; } fn main() -> i32 { let a = qcore_acquire(3, true, Failure); let b = qcore_acquire(4, true, Failure); drop b; drop a; return 0; }",
        );
        let summary =
            &module.checked.functions[&module.checked.source.globals[&module.entry]].summary;
        assert_eq!(summary.flow.peak().get("resources"), 2);
        assert_eq!(summary.flow.total().get("allocations"), 2);
    }

    #[test]
    fn preserves_error_cleanup_and_ownership_transfer_observations() {
        let failed = compile(
            "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(7, true, Failure); qcore_check(false, Failure); drop r; return 1; }",
        );
        assert_eq!(
            failed.reference_observation.outcome,
            ExecutionOutcome::Error {
                name: "Failure".into(),
                tag: 1
            }
        );
        assert_eq!(
            failed.reference_observation.trace,
            vec![TraceEvent::Acquire(7), TraceEvent::Release(7)]
        );
        let moved = compile(
            "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(9, true, Failure); move r; return -3; }",
        );
        assert_eq!(
            moved.reference_observation.outcome,
            ExecutionOutcome::Ok {
                value: -3,
                payload: vec![9]
            }
        );
        assert_eq!(
            moved.reference_observation.trace,
            vec![TraceEvent::Acquire(9), TraceEvent::Transfer(9)]
        );
    }

    #[test]
    fn reports_unsupported_source_at_the_owning_file() {
        let errors = compile_files(
            &[SourceFile::new(
                "bad.qc",
                "fn main() -> i32 { if (true) { return 1; } }",
            )],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].location.path, "bad.qc");
        assert_eq!(errors[0].code, "Q500");
    }

    #[test]
    fn emitted_c_matches_reference_for_value_error_trace_and_transfer() {
        if Command::new("cc").arg("--version").output().is_err() {
            return;
        }
        for source in [
            "fn main() -> i32 { return 17; }",
            "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(1, true, Failure); qcore_check(false, Failure); drop r; return 0; }",
            "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(2, true, Failure); move r; return 5; }",
        ] {
            let module = compile(source);
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let directory = std::env::temp_dir()
                .join(format!("qcore-cycle12-{}-{sequence}", std::process::id()));
            fs::create_dir_all(&directory).unwrap();
            let c_path = directory.join("program.c");
            let exe_path = directory.join("program");
            fs::write(&c_path, &module.c_source).unwrap();
            let status = Command::new("cc")
                .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
                .arg(&c_path)
                .arg("-o")
                .arg(&exe_path)
                .status()
                .unwrap();
            assert!(status.success());
            let output = Command::new(&exe_path).output().unwrap();
            assert!(output.status.success());
            assert_eq!(
                parse_c_observation(
                    std::str::from_utf8(&output.stdout).unwrap(),
                    module.checked.functions[&module.checked.source.globals[&module.entry]]
                        .cfg
                        .error_abi()
                )
                .unwrap(),
                module.reference_observation
            );
            fs::remove_dir_all(directory).unwrap();
        }
    }
}
