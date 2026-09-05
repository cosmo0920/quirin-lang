//! Cycle 11 whole-program semantic integration.
//!
//! Each source function supplies the reduced IR already consumed by the
//! established usage, ownership, resource, typed-error, region, and lowering
//! passes.  This module checks those passes as one transaction, resolves calls
//! by Cycle 10 symbol identity, computes summaries over the call graph, and
//! validates recursive contracts as post-fixed points.  It deliberately does
//! not emit C: source-to-C orchestration remains Cycle 12.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ResourceId;
use crate::cfg::{TypedCfg, interpret_cfg};
use crate::cleanup_lowering::{
    SourceProgram as LoweringProgram, interpret_source, lower_cleanup_cfg,
};
use crate::error_region::{RegionExitMap, RegionOp, analyze_error_regions};
use crate::frontend::{
    DeclarationKind, SemanticPass, SourceDiagnostic, SourceTypeKind, SymbolId,
    TypedSourceDeclaration, TypedSourceProgram, source_diagnostic,
};
use crate::lexer::Span;
use crate::ownership::{OwnershipOp, OwnershipState, check_ownership};
use crate::resource_flow::{Flow, ResourceBounds};
use crate::typed_errors::{
    EffectRow, ErrorDeclarations, ErrorLedger, ErrorOp, ExitMap, analyze_typed_errors,
};
use crate::usage::{DemandContext, Usage, UsageExpr, check_usage};

/// Ownership behavior of one formal parameter at a successful call boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterOwnership {
    /// A lexical loan; the caller keeps the same identity.
    Borrowed,
    /// Ownership leaves the caller through the callee.
    Consumed,
    /// Ownership crosses the call and returns with the same identity.
    Returned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub callee: SymbolId,
    pub arguments: Vec<ResourceId>,
    pub span: Span,
}

/// The interprocedural contract consumed at call sites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSummary {
    pub effects: EffectRow,
    pub flow: Flow,
    pub parameters: Vec<ParameterOwnership>,
}

/// Inputs for all semantic engines attached to one Cycle 10 function symbol.
/// Resource segments occur around calls and therefore have length
/// `calls.len() + 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSemantics {
    pub symbol: SymbolId,
    pub usage_declarations: BTreeMap<String, Usage>,
    pub usage_expression: UsageExpr,
    pub ownership_operations: Vec<OwnershipOp>,
    pub resource_segments: Vec<Flow>,
    pub resource_bounds: ResourceBounds,
    pub allowed_effects: EffectRow,
    pub initial_error_ledger: ErrorLedger,
    pub error_operations: Vec<ErrorOp>,
    pub region_operations: Vec<RegionOp>,
    pub lowering: LoweringProgram,
    pub calls: Vec<CallSite>,
    pub identity_entry: Vec<ResourceId>,
    pub identity_exit: Vec<ResourceId>,
    pub parameters: Vec<ParameterOwnership>,
    /// Mandatory exactly for functions in a recursive call-graph SCC.
    pub recursive_contract: Option<FunctionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFunction {
    pub source: TypedSourceDeclaration,
    pub usage: DemandContext,
    pub ownership: OwnershipState,
    pub typed_errors: ExitMap,
    pub regions: RegionExitMap,
    pub summary: FunctionSummary,
    pub cfg: TypedCfg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedProgram {
    pub source: TypedSourceProgram,
    pub functions: BTreeMap<SymbolId, CheckedFunction>,
    pub recursive_functions: BTreeSet<SymbolId>,
}

#[derive(Debug)]
struct InterfaceError(String);

impl fmt::Display for InterfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InterfaceError {}

struct LocalChecked {
    source: TypedSourceDeclaration,
    semantics: FunctionSemantics,
    usage: DemandContext,
    ownership: OwnershipState,
    typed_errors: ExitMap,
    regions: RegionExitMap,
    local_effects: EffectRow,
    cfg: TypedCfg,
}

/// Runs every established sequential pass and resolves all function summaries.
/// Any failure is returned at the source function or call span that owns it.
pub fn check_whole_program(
    source: &TypedSourceProgram,
    errors: &ErrorDeclarations,
    semantics: Vec<FunctionSemantics>,
) -> Result<CheckedProgram, Vec<SourceDiagnostic>> {
    let source_functions = source
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == DeclarationKind::Function)
        .map(|declaration| (declaration.symbol, declaration.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut supplied = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for function in semantics {
        let span = source_functions
            .get(&function.symbol)
            .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
        if supplied.insert(function.symbol, function).is_some() {
            diagnostics.push(diagnostic(
                SemanticPass::NameResolution,
                span,
                "duplicate semantic bundle for one source function",
            ));
        }
    }
    for (symbol, declaration) in &source_functions {
        if !supplied.contains_key(symbol) {
            diagnostics.push(diagnostic(
                SemanticPass::NameResolution,
                declaration.span,
                "source function has no Cycle 11 semantic bundle",
            ));
        }
    }
    for (symbol, function) in &supplied {
        if !source_functions.contains_key(symbol) {
            diagnostics.push(diagnostic(
                SemanticPass::NameResolution,
                Span { start: 0, end: 0 },
                "semantic bundle does not name a source function",
            ));
        }
        for call in &function.calls {
            if !source_functions.contains_key(&call.callee) {
                diagnostics.push(diagnostic(
                    SemanticPass::NameResolution,
                    call.span,
                    "call target is not a source function",
                ));
            }
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let graph = supplied
        .iter()
        .map(|(symbol, function)| {
            (
                *symbol,
                function.calls.iter().map(|call| call.callee).collect(),
            )
        })
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    let recursive_functions = graph
        .keys()
        .copied()
        .filter(|symbol| reaches(*symbol, *symbol, &graph, &mut BTreeSet::new()))
        .collect::<BTreeSet<_>>();

    let mut local = BTreeMap::new();
    for (symbol, function) in supplied {
        let declaration = source_functions[&symbol].clone();
        match check_local(declaration, function, errors) {
            Ok(checked) => {
                local.insert(symbol, checked);
            }
            Err(error) => diagnostics.push(error),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    for symbol in &recursive_functions {
        let checked = &local[symbol];
        if checked.semantics.recursive_contract.is_none() {
            diagnostics.push(diagnostic(
                SemanticPass::ResourceFlow,
                checked.source.span,
                "recursive function requires an explicit summary contract",
            ));
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // Recursive contracts seed the fixed point.  Acyclic summaries are then
    // evaluated once their callees are available.
    let mut summaries = recursive_functions
        .iter()
        .map(|symbol| {
            (
                *symbol,
                local[symbol]
                    .semantics
                    .recursive_contract
                    .clone()
                    .expect("recursive contracts checked above"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    while summaries.len() < local.len() {
        let ready = local
            .iter()
            .filter(|(symbol, _)| !summaries.contains_key(symbol))
            .find(|(_, function)| {
                function
                    .semantics
                    .calls
                    .iter()
                    .all(|call| summaries.contains_key(&call.callee))
            })
            .map(|(symbol, _)| *symbol);
        let Some(symbol) = ready else {
            diagnostics.push(diagnostic(
                SemanticPass::ResourceFlow,
                Span { start: 0, end: 0 },
                "call-summary fixed point could not be resolved",
            ));
            return Err(diagnostics);
        };
        match unfold_summary(&local[&symbol], &summaries) {
            Ok(summary) => {
                summaries.insert(symbol, summary);
            }
            Err(error) => diagnostics.push(error),
        }
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
    }

    // One unfolding of every recursive body must be contained in its seed;
    // this is the executable post-fixed-point rule.
    for symbol in &recursive_functions {
        let unfolded = match unfold_summary(&local[symbol], &summaries) {
            Ok(summary) => summary,
            Err(error) => {
                diagnostics.push(error);
                continue;
            }
        };
        let declared = &summaries[symbol];
        if !unfolded.flow.is_covered_by(&declared.flow)
            || !unfolded.effects.is_subset_of(&declared.effects)
            || unfolded.parameters != declared.parameters
        {
            diagnostics.push(diagnostic(
                SemanticPass::ResourceFlow,
                local[symbol].source.span,
                "recursive summary is not a post-fixed point of its body",
            ));
        }
    }

    for (symbol, function) in &local {
        let summary = &summaries[symbol];
        if !function.semantics.resource_bounds.accepts(&summary.flow) {
            diagnostics.push(diagnostic(
                SemanticPass::ResourceFlow,
                function.source.span,
                "whole-function summary exceeds its declared resource bounds",
            ));
        }
        if !summary
            .effects
            .is_subset_of(&function.semantics.allowed_effects)
        {
            diagnostics.push(diagnostic(
                SemanticPass::TypedErrors,
                function.source.span,
                "callee effects exceed the caller's declared effect row",
            ));
        }
        if let Err(error) = check_identity_transfer(function, &summaries) {
            diagnostics.push(error);
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let functions = local
        .into_iter()
        .map(|(symbol, function)| {
            (
                symbol,
                CheckedFunction {
                    source: function.source,
                    usage: function.usage,
                    ownership: function.ownership,
                    typed_errors: function.typed_errors,
                    regions: function.regions,
                    summary: summaries[&symbol].clone(),
                    cfg: function.cfg,
                },
            )
        })
        .collect();
    Ok(CheckedProgram {
        source: source.clone(),
        functions,
        recursive_functions,
    })
}

fn check_local(
    source: TypedSourceDeclaration,
    semantics: FunctionSemantics,
    errors: &ErrorDeclarations,
) -> Result<LocalChecked, SourceDiagnostic> {
    if semantics.resource_segments.len() != semantics.calls.len() + 1 {
        return Err(diagnostic(
            SemanticPass::ResourceFlow,
            source.span,
            "resource segments must occur before, between, and after every call",
        ));
    }
    let ownership_parameters = source
        .parameters
        .iter()
        .filter(|parameter| {
            matches!(
                parameter.type_kind,
                SourceTypeKind::Owned | SourceTypeKind::Borrow
            )
        })
        .collect::<Vec<_>>();
    if ownership_parameters.len() != semantics.parameters.len() {
        return Err(diagnostic(
            SemanticPass::OrdinaryTypes,
            source.span,
            "ownership summary arity does not match the source parameter types",
        ));
    }
    if ownership_parameters
        .iter()
        .zip(&semantics.parameters)
        .any(|(parameter, mode)| {
            parameter.type_kind == SourceTypeKind::Borrow && *mode != ParameterOwnership::Borrowed
        })
    {
        return Err(diagnostic(
            SemanticPass::Ownership,
            source.span,
            "borrow parameters require a borrowed call-summary mode",
        ));
    }
    let references = referenced_symbols(&source);
    for call in &semantics.calls {
        if call.span.start < source.span.start || call.span.end > source.span.end {
            return Err(diagnostic(
                SemanticPass::NameResolution,
                call.span,
                "call span lies outside its source function",
            ));
        }
        if !references.contains(&call.callee) {
            return Err(diagnostic(
                SemanticPass::NameResolution,
                call.span,
                "semantic call is not present in the elaborated source references",
            ));
        }
    }

    let usage = check_usage(&semantics.usage_declarations, &semantics.usage_expression)
        .map_err(|error| source_diagnostic(SemanticPass::Usage, source.span, &error))?;
    let ownership = check_ownership(&semantics.ownership_operations)
        .map_err(|error| source_diagnostic(SemanticPass::Ownership, source.span, &error))?;
    let typed_errors = analyze_typed_errors(
        errors,
        &semantics.allowed_effects,
        semantics.initial_error_ledger.clone(),
        &semantics.error_operations,
    )
    .map_err(|error| source_diagnostic(SemanticPass::TypedErrors, source.span, &error))?;
    let local_effects = typed_errors.inferred_effects();
    let regions = analyze_error_regions(
        errors,
        &semantics.allowed_effects,
        semantics.initial_error_ledger.clone(),
        &semantics.region_operations,
    )
    .map_err(|error| source_diagnostic(SemanticPass::ErrorRegion, source.span, &error))?;
    let cfg = lower_cleanup_cfg(&semantics.lowering)
        .map_err(|error| source_diagnostic(SemanticPass::CleanupLowering, source.span, &error))?;
    let source_observation = interpret_source(&semantics.lowering)
        .map_err(|error| source_diagnostic(SemanticPass::CleanupLowering, source.span, &error))?;
    let cfg_observation = interpret_cfg(&cfg)
        .map_err(|error| source_diagnostic(SemanticPass::CleanupLowering, source.span, &error))?;
    if source_observation != cfg_observation {
        return Err(diagnostic(
            SemanticPass::CleanupLowering,
            source.span,
            "checked CFG does not preserve the source observation",
        ));
    }
    let universe = semantics
        .lowering
        .resources()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut pass_identities = semantics
        .initial_error_ledger
        .ids()
        .collect::<BTreeSet<_>>();
    collect_ownership_identities(&semantics.ownership_operations, &mut pass_identities);
    collect_error_identities(&semantics.error_operations, &mut pass_identities);
    collect_region_identities(&semantics.region_operations, &mut pass_identities);
    if !pass_identities.is_subset(&universe) {
        return Err(diagnostic(
            SemanticPass::Ownership,
            source.span,
            "a semantic pass references an identity absent from the lowering identity domain",
        ));
    }
    for identity in semantics
        .identity_entry
        .iter()
        .chain(&semantics.identity_exit)
        .chain(
            semantics
                .calls
                .iter()
                .flat_map(|call| call.arguments.iter()),
        )
    {
        if !universe.contains(identity) {
            return Err(diagnostic(
                SemanticPass::Ownership,
                source.span,
                "call-interface identity is absent from the lowering identity domain",
            ));
        }
    }
    Ok(LocalChecked {
        source,
        semantics,
        usage,
        ownership,
        typed_errors,
        regions,
        local_effects,
        cfg,
    })
}

fn unfold_summary(
    function: &LocalChecked,
    summaries: &BTreeMap<SymbolId, FunctionSummary>,
) -> Result<FunctionSummary, SourceDiagnostic> {
    let mut flow = function.semantics.resource_segments[0].clone();
    let mut effects = function.local_effects.clone();
    for (index, call) in function.semantics.calls.iter().enumerate() {
        let callee = &summaries[&call.callee];
        if call.arguments.len() != callee.parameters.len() {
            return Err(diagnostic(
                SemanticPass::OrdinaryTypes,
                call.span,
                "call ownership arity does not match the callee summary",
            ));
        }
        flow = flow
            .then(&callee.flow)
            .and_then(|with_call| with_call.then(&function.semantics.resource_segments[index + 1]))
            .map_err(|error| source_diagnostic(SemanticPass::ResourceFlow, call.span, &error))?;
        effects = EffectRow::new(effects.iter().chain(callee.effects.iter()).cloned());
    }
    Ok(FunctionSummary {
        effects,
        flow,
        parameters: function.semantics.parameters.clone(),
    })
}

fn check_identity_transfer(
    function: &LocalChecked,
    summaries: &BTreeMap<SymbolId, FunctionSummary>,
) -> Result<(), SourceDiagnostic> {
    let mut live = unique_identities(&function.semantics.identity_entry).ok_or_else(|| {
        diagnostic(
            SemanticPass::Ownership,
            function.source.span,
            "function entry contains a duplicated resource identity",
        )
    })?;
    for call in &function.semantics.calls {
        let callee = &summaries[&call.callee];
        let mut used = BTreeMap::<ResourceId, ParameterOwnership>::new();
        for (identity, mode) in call.arguments.iter().copied().zip(&callee.parameters) {
            if !live.contains(&identity) {
                return Err(diagnostic(
                    SemanticPass::Ownership,
                    call.span,
                    "call uses a resource identity not owned by the caller",
                ));
            }
            if let Some(previous) = used.insert(identity, *mode)
                && (previous != ParameterOwnership::Borrowed
                    || *mode != ParameterOwnership::Borrowed)
            {
                return Err(diagnostic(
                    SemanticPass::Ownership,
                    call.span,
                    "one resource identity aliases an owned call parameter",
                ));
            }
        }
        for (identity, mode) in call.arguments.iter().copied().zip(&callee.parameters) {
            if *mode == ParameterOwnership::Consumed {
                live.remove(&identity);
            }
        }
    }
    let expected = unique_identities(&function.semantics.identity_exit).ok_or_else(|| {
        diagnostic(
            SemanticPass::Ownership,
            function.source.span,
            "function exit contains a duplicated resource identity",
        )
    })?;
    if live != expected {
        return Err(diagnostic(
            SemanticPass::Ownership,
            function.source.span,
            "cross-function ownership transfer does not match the declared exit identities",
        ));
    }
    Ok(())
}

fn unique_identities(identities: &[ResourceId]) -> Option<BTreeSet<ResourceId>> {
    let set = identities.iter().copied().collect::<BTreeSet<_>>();
    (set.len() == identities.len()).then_some(set)
}

fn collect_ownership_identities(operations: &[OwnershipOp], result: &mut BTreeSet<ResourceId>) {
    for operation in operations {
        match operation {
            OwnershipOp::Acquire { id, .. } => {
                result.insert(*id);
            }
            OwnershipOp::BorrowShared { body, .. } | OwnershipOp::BorrowMutable { body, .. } => {
                collect_ownership_identities(body, result);
            }
            OwnershipOp::Branch { then_ops, else_ops } => {
                collect_ownership_identities(then_ops, result);
                collect_ownership_identities(else_ops, result);
            }
            OwnershipOp::Read(_)
            | OwnershipOp::Write(_)
            | OwnershipOp::Move { .. }
            | OwnershipOp::Copy(_)
            | OwnershipOp::Drop(_)
            | OwnershipOp::TransferOut(_) => {}
        }
    }
}

fn collect_error_identities(operations: &[ErrorOp], result: &mut BTreeSet<ResourceId>) {
    for operation in operations {
        match operation {
            ErrorOp::Acquire(identity) | ErrorOp::Drop(identity) => {
                result.insert(*identity);
            }
            ErrorOp::Branch { then_ops, else_ops } => {
                collect_error_identities(then_ops, result);
                collect_error_identities(else_ops, result);
            }
            ErrorOp::Catch { body, clauses } => {
                collect_error_identities(body, result);
                for clause in clauses {
                    collect_error_identities(&clause.handler, result);
                }
            }
            ErrorOp::Perform(_) | ErrorOp::Raise(_) | ErrorOp::Try(_) => {}
        }
    }
}

fn collect_region_identities(operations: &[RegionOp], result: &mut BTreeSet<ResourceId>) {
    for operation in operations {
        match operation {
            RegionOp::Acquire { resource, .. }
            | RegionOp::Drop(resource)
            | RegionOp::MoveToErrorPayload(resource)
            | RegionOp::DropPayload(resource) => {
                result.insert(*resource);
            }
            RegionOp::Branch { then_ops, else_ops } => {
                collect_region_identities(then_ops, result);
                collect_region_identities(else_ops, result);
            }
            RegionOp::ErrorRegion { body, catches, .. } => {
                collect_region_identities(body, result);
                for clause in catches {
                    collect_region_identities(&clause.handler, result);
                }
            }
            RegionOp::AddStrictCredit(_)
            | RegionOp::SpendStrictCredit(_)
            | RegionOp::Defer { .. }
            | RegionOp::Raise(_)
            | RegionOp::QuantOpen { .. }
            | RegionOp::Discharge(_) => {}
        }
    }
}

fn referenced_symbols(source: &TypedSourceDeclaration) -> BTreeSet<SymbolId> {
    fn collect(statements: &[crate::frontend::TypedStatement], result: &mut BTreeSet<SymbolId>) {
        for statement in statements {
            result.extend(statement.referenced_symbols.iter().copied());
            collect(&statement.nested, result);
        }
    }
    let mut result = BTreeSet::new();
    collect(&source.statements, &mut result);
    result
}

fn reaches(
    start: SymbolId,
    target: SymbolId,
    graph: &BTreeMap<SymbolId, BTreeSet<SymbolId>>,
    visited: &mut BTreeSet<SymbolId>,
) -> bool {
    for next in &graph[&start] {
        if *next == target {
            return true;
        }
        if visited.insert(*next) && reaches(*next, target, graph, visited) {
            return true;
        }
    }
    false
}

fn diagnostic(pass: SemanticPass, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    source_diagnostic(pass, span, &InterfaceError(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup_lowering::SourceOp;
    use crate::frontend::{elaborate_source, parse_source_program};
    use crate::ownership::Footprint;
    use crate::resource_flow::Quantity;

    fn zero_summary(parameters: Vec<ParameterOwnership>) -> FunctionSummary {
        FunctionSummary {
            effects: EffectRow::default(),
            flow: Flow::zero(),
            parameters,
        }
    }

    fn lowering(name: &str, resources: Vec<ResourceId>) -> LoweringProgram {
        LoweringProgram::new(
            name,
            resources,
            ["Failure".into()],
            vec![SourceOp::Return(0)],
        )
        .unwrap()
    }

    fn semantics(
        symbol: SymbolId,
        name: &str,
        resources: Vec<ResourceId>,
        calls: Vec<CallSite>,
    ) -> FunctionSemantics {
        FunctionSemantics {
            symbol,
            usage_declarations: BTreeMap::new(),
            usage_expression: UsageExpr::Unit,
            ownership_operations: Vec::new(),
            resource_segments: vec![Flow::zero(); calls.len() + 1],
            resource_bounds: ResourceBounds::new(Quantity::zero(), Quantity::zero()),
            allowed_effects: EffectRow::default(),
            initial_error_ledger: ErrorLedger::default(),
            error_operations: Vec::new(),
            region_operations: Vec::new(),
            lowering: lowering(name, resources),
            calls,
            identity_entry: Vec::new(),
            identity_exit: Vec::new(),
            parameters: Vec::new(),
            recursive_contract: None,
        }
    }

    fn typed(source: &str) -> TypedSourceProgram {
        elaborate_source(&parse_source_program(source).unwrap()).unwrap()
    }

    fn call_span(program: &TypedSourceProgram, caller: &str) -> Span {
        program
            .declarations
            .iter()
            .find(|declaration| declaration.name == caller)
            .unwrap()
            .statements[0]
            .span
    }

    #[test]
    fn checks_an_acyclic_whole_program_and_preserves_cfg_observation() {
        let source = typed("fn leaf() -> unit { return; } fn root() -> unit { leaf(); return; }");
        let leaf = source.globals["leaf"];
        let root = source.globals["root"];
        let checked = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![
                semantics(leaf, "leaf", Vec::new(), Vec::new()),
                semantics(
                    root,
                    "root",
                    Vec::new(),
                    vec![CallSite {
                        callee: leaf,
                        arguments: Vec::new(),
                        span: call_span(&source, "root"),
                    }],
                ),
            ],
        )
        .unwrap();
        assert_eq!(checked.functions.len(), 2);
        assert!(checked.recursive_functions.is_empty());
        assert_eq!(
            interpret_cfg(&checked.functions[&root].cfg).unwrap(),
            interpret_source(&lowering("root", Vec::new())).unwrap()
        );
    }

    #[test]
    fn recursive_scc_requires_an_explicit_contract() {
        let source = typed("fn looped() -> unit { looped(); return; }");
        let symbol = source.globals["looped"];
        let error = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![semantics(
                symbol,
                "looped",
                Vec::new(),
                vec![CallSite {
                    callee: symbol,
                    arguments: Vec::new(),
                    span: call_span(&source, "looped"),
                }],
            )],
        )
        .unwrap_err();
        assert!(error[0].message.contains("explicit summary"));
    }

    #[test]
    fn recursive_post_fixed_summary_is_accepted() {
        let source = typed("fn looped() -> unit { looped(); return; }");
        let symbol = source.globals["looped"];
        let mut body = semantics(
            symbol,
            "looped",
            Vec::new(),
            vec![CallSite {
                callee: symbol,
                arguments: Vec::new(),
                span: call_span(&source, "looped"),
            }],
        );
        body.recursive_contract = Some(zero_summary(Vec::new()));
        let checked = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![body],
        )
        .unwrap();
        assert!(checked.recursive_functions.contains(&symbol));
        assert_eq!(checked.functions[&symbol].summary, zero_summary(Vec::new()));
    }

    #[test]
    fn recursive_summary_must_cover_one_unfolding() {
        let source = typed("fn looped() -> unit { looped(); return; }");
        let symbol = source.globals["looped"];
        let mut body = semantics(
            symbol,
            "looped",
            Vec::new(),
            vec![CallSite {
                callee: symbol,
                arguments: Vec::new(),
                span: call_span(&source, "looped"),
            }],
        );
        body.resource_segments[0] = Flow::acquire(Quantity::single("memory", 1), Quantity::zero());
        body.recursive_contract = Some(zero_summary(Vec::new()));
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![body],
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("post-fixed"))
        );
    }

    #[test]
    fn consumed_parameter_transfers_the_same_identity_across_a_call() {
        let source = typed(
            "fn sink(x: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { sink(move x); return; }",
        );
        let sink = source.globals["sink"];
        let root = source.globals["root"];
        let mut sink_body = semantics(sink, "sink", vec![7], Vec::new());
        sink_body.parameters = vec![ParameterOwnership::Consumed];
        let mut root_body = semantics(
            root,
            "root",
            vec![7],
            vec![CallSite {
                callee: sink,
                arguments: vec![7],
                span: call_span(&source, "root"),
            }],
        );
        root_body.identity_entry = vec![7];
        root_body.parameters = vec![ParameterOwnership::Consumed];
        let checked = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![sink_body, root_body],
        )
        .unwrap();
        assert_eq!(
            checked.functions[&sink].summary.parameters,
            vec![ParameterOwnership::Consumed]
        );
    }

    #[test]
    fn returned_parameter_preserves_identity_for_the_caller() {
        let source = typed(
            "fn inspect(x: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { inspect(move x); return; }",
        );
        let inspect = source.globals["inspect"];
        let root = source.globals["root"];
        let mut inspect_body = semantics(inspect, "inspect", vec![9], Vec::new());
        inspect_body.parameters = vec![ParameterOwnership::Returned];
        let mut root_body = semantics(
            root,
            "root",
            vec![9],
            vec![CallSite {
                callee: inspect,
                arguments: vec![9],
                span: call_span(&source, "root"),
            }],
        );
        root_body.identity_entry = vec![9];
        root_body.identity_exit = vec![9];
        root_body.parameters = vec![ParameterOwnership::Returned];
        check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![inspect_body, root_body],
        )
        .unwrap();
    }

    #[test]
    fn owned_call_parameters_cannot_alias_one_identity() {
        let source = typed(
            "fn pair(a: own<File>, b: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { pair(move x, move x); return; }",
        );
        let pair = source.globals["pair"];
        let root = source.globals["root"];
        let mut pair_body = semantics(pair, "pair", vec![3], Vec::new());
        pair_body.parameters = vec![ParameterOwnership::Consumed, ParameterOwnership::Returned];
        let mut root_body = semantics(
            root,
            "root",
            vec![3],
            vec![CallSite {
                callee: pair,
                arguments: vec![3, 3],
                span: call_span(&source, "root"),
            }],
        );
        root_body.identity_entry = vec![3];
        root_body.parameters = vec![ParameterOwnership::Consumed];
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![pair_body, root_body],
        )
        .unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("aliases")));
    }

    #[test]
    fn local_ownership_failure_is_reported_at_the_function_span() {
        let source = typed("fn bad() -> unit { return; }");
        let symbol = source.globals["bad"];
        let mut body = semantics(symbol, "bad", vec![1], Vec::new());
        body.ownership_operations = vec![OwnershipOp::Acquire {
            id: 1,
            owner: "leak".into(),
            resource_type: "File".into(),
            footprint: Footprint::single("files", 1),
        }];
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![body],
        )
        .unwrap_err();
        assert_eq!(errors[0].pass, SemanticPass::Ownership);
        assert_eq!(errors[0].span, source.declarations[0].span);
    }

    #[test]
    fn callee_effects_must_fit_the_callers_declared_row() {
        let source = typed("fn io() -> unit { return; } fn root() -> unit { io(); return; }");
        let io = source.globals["io"];
        let root = source.globals["root"];
        let mut io_body = semantics(io, "io", Vec::new(), Vec::new());
        io_body.allowed_effects = EffectRow::new([crate::Effect::Io]);
        io_body.error_operations = vec![ErrorOp::Perform(crate::Effect::Io)];
        let root_body = semantics(
            root,
            "root",
            Vec::new(),
            vec![CallSite {
                callee: io,
                arguments: Vec::new(),
                span: call_span(&source, "root"),
            }],
        );
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![io_body, root_body],
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.pass == SemanticPass::TypedErrors
                    && error.message.contains("callee effects"))
        );
    }
}
