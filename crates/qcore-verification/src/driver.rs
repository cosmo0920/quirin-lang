//! Source-file orchestration for the sequential cleanup and scoped task
//! profiles. Each path produces its own checked evidence and rejects unsupported
//! forms before C emission. Task source is never represented by an erased
//! sequential CFG.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use qcore_core::cleanup_lowering::{SourceOp, SourceProgram as LoweringProgram, interpret_source};
use qcore_core::ownership::{Footprint, OwnershipOp, ResourceId};
use qcore_core::resource_flow::{Flow, Quantity, ResourceBounds};
use qcore_core::typed_errors::{Effect, EffectRow, ErrorDeclarations, ErrorLedger, ErrorOp};
use qcore_core::usage::{Usage, UsageExpr};
use qcore_core::{ExecutionObservation, emit_c11};
use qcore_frontend::frontend::{
    DeclarationKind, ExecutableResultType, Located, SemanticPass, SourceDeclaration,
    SourceDiagnostic, SourceModuleMode, SourceProgram, StatementKind, TypedSourceDeclaration,
    TypedSourceProgram, elaborate_source, parse_source_program,
};
use qcore_frontend::lexer::{Span, Symbol, Token, TokenKind, lex};
use qcore_frontend::whole_program::{
    CallSite, CheckedProgram, FunctionSemantics, FunctionSummary, ParameterOwnership,
    check_whole_program,
};
use qcore_verifier_host::VerifierRegistry;

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

/// Result typing records the sequential scalar/identity split separately from
/// checked task-owned Buffer results. General owned/error payload types remain
/// outside these bounded profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultProfile {
    /// Body definitions only; external contracts retain structural annotations.
    pub scalar_results: BTreeMap<String, ExecutableResultType>,
    /// Functions whose checked task result carries one owned Buffer identity.
    pub owned_buffer_results: std::collections::BTreeSet<String>,
    pub custody_payloads: CustodyPayloadProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyPayloadProfile {
    /// `move` appends a live acquired identity to a separate completion ledger.
    /// The ownership/cleanup model checks that ledger, but the scalar result
    /// annotation does not type its identities as `own<T>` payloads.
    SeparateIdentityLedger,
    /// The scoped copy profile rejects every resource-bearing source form.
    NoResourcePayloads,
    /// Typed owned Buffer capture/result custody, checked against task slots.
    TypedTaskBuffers,
}

/// Checked evidence is explicit about the executed semantic profile. Task
/// modules never carry a sequential CFG with their task operations erased.
#[derive(Clone, Debug)]
pub enum CheckedSourceModule {
    Sequential(Box<CheckedProgram>),
    ScopedTasks(Box<qcore_core::task_execution::CheckedTaskModule>),
}

impl CheckedSourceModule {
    pub fn as_sequential(&self) -> Option<&CheckedProgram> {
        match self {
            Self::Sequential(program) => Some(program),
            Self::ScopedTasks(_) => None,
        }
    }

    pub fn as_scoped_tasks(&self) -> Option<&qcore_core::task_execution::CheckedTaskModule> {
        match self {
            Self::Sequential(_) => None,
            Self::ScopedTasks(program) => Some(program),
        }
    }

    pub fn function_count(&self) -> usize {
        match self {
            Self::Sequential(program) => program.functions.len(),
            Self::ScopedTasks(program) => program.module().functions.len(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExecutionProfile {
    SequentialCleanupV1,
    ScopedCopyTasksV1,
    ScopedOwnedTasksV1,
}

#[derive(Clone, Debug)]
pub struct CompiledModule {
    pub checked: CheckedSourceModule,
    pub execution_profile: SourceExecutionProfile,
    pub declaration_count: usize,
    pub mode: SourceModuleMode,
    pub result_profile: SourceResultProfile,
    pub entry: Option<String>,
    pub c_source: Option<String>,
    pub source_map: Vec<EmittedSourceMap>,
    pub reference_observation: Option<ExecutionObservation>,
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

/// Compiles with an explicit verifier registry. Source-level `verify scope`
/// declarations are resolved (`Q700` on failure) and then dispatched: the core
/// derives an obligation inventory, requests witnesses, and rechecks them
/// behind the firewall. A scope whose obligations are not all discharged by
/// core-checked evidence is `Q701`; conflicting provider evidence is `Q702`
/// and an unvalidated provider allegation is `Q703`. Use
/// `VerifierRegistry::resolve_source_scopes` for configuration validation
/// alone.
pub fn compile_files_with_registry(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if files.is_empty() {
        return Err(vec![plain_diagnostic(
            "Q400",
            "no input files were provided",
        )]);
    }
    let (_, ranges) = combine_files(files);
    let mut stripped = Vec::new();
    let mut modes = Vec::new();
    for (file, range) in files.iter().zip(&ranges) {
        let parsed = parse_source_program(&file.source).map_err(|error| {
            vec![map_raw_diagnostic(
                "Q000",
                SemanticPass::NameResolution,
                Span {
                    start: error.span.start + range.start,
                    end: error.span.end + range.start,
                },
                error.message,
                &ranges,
            )]
        })?;
        modes.push(parsed.mode);
        let mut source = file.source.clone();
        if let Some(span) = parsed.mode_span {
            let blank: String = source.as_bytes()[span.start..span.end]
                .iter()
                .map(|byte| if *byte == b'\n' { '\n' } else { ' ' })
                .collect();
            source.replace_range(span.start..span.end, &blank);
        }
        stripped.push(SourceFile::new(&file.path, source));
    }
    // Contract declarations can accompany either a library or an executable.
    // An unmarked file keeps the existing executable compilation behavior.
    let mode = if modes.contains(&SourceModuleMode::Executable) {
        SourceModuleMode::Executable
    } else if modes.contains(&SourceModuleMode::Library) {
        SourceModuleMode::Library
    } else {
        SourceModuleMode::Contract
    };
    if mode != SourceModuleMode::Executable && options.entry.is_some() {
        return Err(vec![plain_diagnostic(
            "Q403",
            "--entry is incompatible with library/contract mode; include an executable module to link an entry point",
        )]);
    }
    let (combined, _) = combine_files(&stripped);
    let mut parsed = parse_source_program(&combined).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q000",
            SemanticPass::NameResolution,
            error.span,
            error.message,
            &ranges,
        )]
    })?;
    parsed.mode = mode;
    let scopes = registry.resolve_source_scopes(&parsed).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = parsed
                    .declarations
                    .iter()
                    .find(|declaration| declaration.value.name == error.scope)
                    .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
                map_raw_diagnostic(
                    "Q700",
                    SemanticPass::PluginVerification,
                    span,
                    error.to_string(),
                    &ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    if !scopes.is_empty() {
        // W3: resolved scopes drive real verification. The core derives the
        // obligation inventory, dispatches requests, and fails closed; a scope
        // is verified only when every core-derived obligation is discharged by
        // core-checked evidence.
        let unit = stripped
            .first()
            .map_or("<unit>", |file| file.path.as_str())
            .to_owned();
        let report = crate::verify_dispatch::dispatch_scopes(&parsed, &unit, &scopes, registry);
        let diagnostics = report.diagnostics();
        if !diagnostics.is_empty() {
            return Err(diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let span = parsed
                        .declarations
                        .iter()
                        .find(|declaration| declaration.value.name == diagnostic.scope)
                        .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
                    map_raw_diagnostic(
                        diagnostic.code,
                        SemanticPass::PluginVerification,
                        span,
                        diagnostic.message,
                        &ranges,
                    )
                })
                .collect());
        }
    }
    let typed = elaborate_source(&parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })?;
    if qcore_frontend::source_tasks::has_source_tasks(&parsed) {
        return compile_task_source(&parsed, &typed, mode, options, &ranges);
    }
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

    let result_profile = SourceResultProfile {
        owned_buffer_results: std::collections::BTreeSet::new(),
        scalar_results: typed
            .declarations
            .iter()
            .filter(|declaration| declaration.kind == DeclarationKind::Function)
            .map(|declaration| {
                (
                    declaration.name.clone(),
                    declaration
                        .result_type
                        .as_ref()
                        .and_then(|annotation| annotation.executable_type())
                        .expect("function synthesis validated every scalar result"),
                )
            })
            .collect(),
        custody_payloads: CustodyPayloadProfile::SeparateIdentityLedger,
    };
    if mode != SourceModuleMode::Executable {
        return Ok(CompiledModule {
            checked: CheckedSourceModule::Sequential(Box::new(checked)),
            execution_profile: SourceExecutionProfile::SequentialCleanupV1,
            declaration_count: typed.declarations.len(),
            mode,
            result_profile,
            entry: None,
            c_source: None,
            source_map: Vec::new(),
            reference_observation: None,
        });
    }
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
    let reference_observation = qcore_core::interpret_cfg(&function.cfg).map_err(|error| {
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
        checked: CheckedSourceModule::Sequential(Box::new(checked)),
        execution_profile: SourceExecutionProfile::SequentialCleanupV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(c_source),
        source_map,
        reference_observation: Some(reference_observation),
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
    let annotation = typed_declaration.result_type.as_ref().ok_or_else(|| {
        diagnostic(
            SemanticPass::OrdinaryTypes,
            declaration.span,
            "missing declared result type",
        )
    })?;
    let result_type = annotation.executable_type().ok_or_else(|| {
        diagnostic(SemanticPass::OrdinaryTypes, annotation.span,
            "the executable result profile supports only i32 and unit; other result types require an implemented typed ABI")
    })?;
    let Some(body) = &declaration.value.body else {
        return Err(diagnostic(
            SemanticPass::CleanupLowering,
            declaration.span,
            "source function requires a body",
        ));
    };
    let mut resources = Vec::new();
    let mut owners = BTreeMap::<String, ResourceId>::new();
    let mut usage_declarations = typed_declaration
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                parameter
                    .usage
                    .as_ref()
                    .map_or(Usage::MANY, |grade| grade.value),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut local_usage_names = BTreeMap::<String, String>::new();
    let mut runtime_demands = Vec::new();
    let mut usage_reachable = true;
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
                // Local usage annotations are not in this executable slice.
                // Reject them instead of silently ignoring a contract.
                if tokens
                    .iter()
                    .take_while(|token| token.kind != TokenKind::Symbol(Symbol::Equal))
                    .any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "use"))
                {
                    return Err(diagnostic(
                        SemanticPass::Usage,
                        statement.span,
                        "local usage annotations are not supported in the executable subset",
                    ));
                }
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
                // Distinguish a local from a shadowed parameter. Ownership and
                // implicit error cleanup remain the ownership pass's concern.
                let usage_name = format!("$local:{}:{binding}", statement.span.start);
                usage_declarations.insert(usage_name.clone(), Usage::MANY);
                local_usage_names.insert(binding.clone(), usage_name);
                usage_reachable &= succeeds;
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
                    usage_reachable &= passes;
                } else if let Some(callee_name) = parse_zero_argument_call(&tokens) {
                    if local_usage_names.contains_key(&callee_name)
                        || typed_declaration
                            .parameters
                            .iter()
                            .any(|parameter| parameter.name == callee_name)
                    {
                        return Err(diagnostic(
                            SemanticPass::OrdinaryTypes,
                            statement.span,
                            "indirect calls are not supported in the executable subset",
                        ));
                    }
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
                    let usage_name = format!("$function:{}", callee.0);
                    usage_declarations.insert(usage_name.clone(), Usage::MANY);
                    if usage_reachable {
                        runtime_demands.push((
                            UsageExpr::Apply {
                                function: Box::new(UsageExpr::Variable(usage_name)),
                                parameter_usages: Vec::new(),
                                arguments: Vec::new(),
                            },
                            true,
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
                if usage_reachable {
                    runtime_demands
                        .push((UsageExpr::Variable(local_usage_names[place].clone()), false));
                }
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
                if usage_reachable {
                    runtime_demands
                        .push((UsageExpr::Variable(local_usage_names[place].clone()), false));
                }
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
                let value = parse_return(&tokens, result_type).ok_or_else(|| {
                    diagnostic(
                        SemanticPass::OrdinaryTypes,
                        statement.span,
                        match result_type {
                            ExecutableResultType::I32 => {
                                "an i32 result requires a signed 32-bit integer literal return"
                            }
                            ExecutableResultType::Unit => "a unit result requires a bare return",
                        },
                    )
                })?;
                lowering.push(SourceOp::Return(value));
                usage_reachable = false;
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
            qcore_frontend::frontend::SourceTypeKind::Borrow => Some(ParameterOwnership::Borrowed),
            qcore_frontend::frontend::SourceTypeKind::Owned => Some(ParameterOwnership::Consumed),
            _ => None,
        })
        .collect();
    // Callees may exit before their continuation. Without a proved normal-exit
    // summary, zero is a possible demand on every subsequent source use.
    let mut reversed_tail = Vec::new();
    for (expression, may_exit) in runtime_demands.into_iter().rev() {
        if may_exit && !reversed_tail.is_empty() {
            reversed_tail.reverse();
            let continuation = UsageExpr::Branch {
                condition: Box::new(UsageExpr::Unit),
                then_expression: Box::new(UsageExpr::Sequence(reversed_tail)),
                else_expression: Box::new(UsageExpr::Unit),
            };
            reversed_tail = vec![continuation];
        }
        reversed_tail.push(expression);
    }
    reversed_tail.reverse();
    let usage_expression = UsageExpr::Sequence(reversed_tail);
    Ok(FunctionSemantics {
        symbol: typed_declaration.symbol,
        usage_declarations,
        usage_expression,
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
    start: qcore_frontend::frontend::SymbolId,
    target: qcore_frontend::frontend::SymbolId,
    graph: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<qcore_frontend::frontend::SymbolId>>,
    visited: &mut BTreeSet<qcore_frontend::frontend::SymbolId>,
) -> bool {
    graph.get(&start).is_some_and(|next| {
        next.iter().any(|symbol| {
            *symbol == target
                || (visited.insert(*symbol) && reaches(*symbol, target, graph, visited))
        })
    })
}

fn validate_emittable_calls(
    entry: qcore_frontend::frontend::SymbolId,
    calls: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<CallSite>>,
    checked: &CheckedProgram,
) -> Result<(), SourceDiagnostic> {
    fn visit(
        symbol: qcore_frontend::frontend::SymbolId,
        calls: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<CallSite>>,
        checked: &CheckedProgram,
        visited: &mut BTreeSet<qcore_frontend::frontend::SymbolId>,
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
            let observation = qcore_core::interpret_cfg(&callee.cfg).map_err(|error| {
                diagnostic(SemanticPass::CleanupLowering, call.span, error.to_string())
            })?;
            let observationally_pure = observation.trace.is_empty()
                && matches!(
                    observation.outcome,
                    qcore_core::ExecutionOutcome::Ok { payload, .. } if payload.is_empty()
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

fn parse_return(tokens: &[Token], result_type: ExecutableResultType) -> Option<i32> {
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
        ] if word == "return" && result_type == ExecutableResultType::Unit => Some(0),
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
        ] if word == "return" && result_type == ExecutableResultType::I32 => value.parse().ok(),
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
        ] if word == "return" && result_type == ExecutableResultType::I32 => {
            // Parse the final signed spelling: the minimum's magnitude is not an i32.
            format!("-{value}").parse().ok()
        }
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

fn compile_task_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free scoped task modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let task_module = qcore_frontend::source_tasks::lower_source_tasks(parsed, typed, &entry)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let checked =
        qcore_core::task_execution::check_task_module(&task_module).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    map_raw_diagnostic(
                        "Q600",
                        SemanticPass::Concurrency,
                        Span {
                            start: error.span.start,
                            end: error.span.end,
                        },
                        error.message,
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    if !cfg!(target_os = "linux") {
        return Err(vec![plain_diagnostic(
            "Q500",
            "native scoped task emission currently requires a Linux C11 threads target",
        )]);
    }
    let emission = qcore_core::task_c_emit::emit_task_c11(&checked);
    use qcore_core::task_execution::{ScalarType, TaskTerminatorKind};
    let owned_profile = checked.module().functions.values().any(|function| {
        function.result == ScalarType::OwnedBuffer
            || function
                .parameters
                .iter()
                .any(|(_, kind)| *kind == ScalarType::OwnedBuffer)
            || function.blocks.values().any(|block| {
                matches!(
                    block.terminator.kind,
                    TaskTerminatorKind::TryAllocateBuffer { .. }
                )
            })
    });
    let result_profile = SourceResultProfile {
        scalar_results: checked
            .module()
            .functions
            .values()
            .filter_map(|function| {
                let scalar = match function.result {
                    ScalarType::I32 => ExecutableResultType::I32,
                    ScalarType::Unit => ExecutableResultType::Unit,
                    ScalarType::OwnedBuffer => return None,
                };
                Some((function.name.clone(), scalar))
            })
            .collect(),
        owned_buffer_results: checked
            .module()
            .functions
            .values()
            .filter(|function| function.result == ScalarType::OwnedBuffer)
            .map(|function| function.name.clone())
            .collect(),
        custody_payloads: if owned_profile {
            CustodyPayloadProfile::TypedTaskBuffers
        } else {
            CustodyPayloadProfile::NoResourcePayloads
        },
    };
    let source_map = emission
        .source_map
        .into_iter()
        .map(|mapping| EmittedSourceMap {
            generated_start_line: mapping.generated_start_line,
            generated_end_line: mapping.generated_end_line,
            source: locate(mapping.span.start, ranges),
        })
        .collect();
    Ok(CompiledModule {
        checked: CheckedSourceModule::ScopedTasks(Box::new(checked)),
        execution_profile: if owned_profile {
            SourceExecutionProfile::ScopedOwnedTasksV1
        } else {
            SourceExecutionProfile::ScopedCopyTasksV1
        },
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map,
        // Creation is dynamic. A single chosen reference execution is not the
        // evidence that all creation and completion alternatives are checked.
        reference_observation: None,
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
    qcore_frontend::frontend::source_diagnostic(pass, span, &error)
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
    // `FileRange::end` is the *exclusive* end of a file in the combined buffer,
    // and `combine_files` starts the next file at exactly that offset whenever
    // the previous file already ended with a newline. Testing `offset <=
    // range.end` therefore let byte 0 of file n+1 satisfy file n's predicate,
    // so any diagnostic at the very start of a non-first input file was blamed
    // on the previous file at a line one past its end. The owning file is now
    // the one that actually contains the offset. An offset belonging to no
    // file — the separator newline `combine_files` appends after a file that
    // did not end with one, or the very end of the combined buffer — falls
    // back to the last file starting at or before it, so an end-of-input span
    // still resolves.
    let index = ranges
        .iter()
        .position(|range| range.start <= offset && offset < range.end)
        .or_else(|| ranges.iter().rposition(|range| range.start <= offset))
        .unwrap_or(0);
    let range = &ranges[index];
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
