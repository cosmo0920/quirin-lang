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
    /// The identity a call that returns fresh custody binds in the caller's
    /// ledger (card `source.extern-call.v1`, design §3.4). `None` for every
    /// call whose callee returns no custody, which is every source call in the
    /// established sequential subset.
    pub result: Option<ResourceId>,
    pub span: Span,
    /// The call sits inside a branch arm, so it runs on some paths through the
    /// caller and not on others (card `IF-ARMS`, `source.branch.v1`).
    ///
    /// The call sequence this vector belongs to is **flat and positional**:
    /// [`unfold_summary`] composes `segment · callee · segment · …` in order,
    /// which applies every callee's flow on every path. That is exact for an
    /// unconditional call and it is *not* a model of a conditional one, so a
    /// conditional call is admitted only when composing its callee changes
    /// nothing at all — a neutral callee summary flow. Anything else is
    /// refused rather than approximated; follow-on `IF-ARM-CALL-FLOW`.
    pub conditional: bool,
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
    /// The region operations the model above was asked about, retained beside
    /// its answer (card `QUANT-REGION-SOURCE`). A consumer that wants to know
    /// *what* was checked — which region, which catch clause, whether a
    /// `quant open` reached the kernel at all — reads them here rather than
    /// re-deriving them from the source. Empty for every function that writes
    /// no `error region`.
    pub region_operations: Vec<RegionOp>,
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
///
/// Every call target must be a checked source function. Card
/// `source.extern-call.v1` adds the second dispatch arm through
/// [`check_whole_program_with_externs`]; this entry point supplies no extern
/// environment, so its behavior is exactly what it always was.
pub fn check_whole_program(
    source: &TypedSourceProgram,
    errors: &ErrorDeclarations,
    semantics: Vec<FunctionSemantics>,
) -> Result<CheckedProgram, Vec<SourceDiagnostic>> {
    check_whole_program_with_externs(source, errors, semantics, &BTreeMap::new())
}

/// Two-way call resolution: a callee is a checked source function, or a
/// declared extern function whose summary the caller derived from an admitted
/// contract and supplies here (card `source.extern-call.v1`).
///
/// An extern summary is a **declared assumption**, not a checked body: this
/// function consumes it exactly like a source summary at call sites, and every
/// verdict downstream is valid only relative to that declaration. Extern
/// symbols are leaves — they contribute no call-graph edge, are never
/// candidates for the recursive post-fixed-point rule, and never receive a
/// `CheckedFunction`, because there is no body to check.
pub fn check_whole_program_with_externs(
    source: &TypedSourceProgram,
    errors: &ErrorDeclarations,
    semantics: Vec<FunctionSemantics>,
    externs: &BTreeMap<SymbolId, FunctionSummary>,
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
        if let Some(contract) = &declaration.function_contract {
            for clause in &contract.clauses {
                diagnostics.push(SourceDiagnostic {
                    code: "Q021",
                    pass: SemanticPass::OrdinaryTypes,
                    span: clause.span,
                    message: "source function contract clauses are not yet enforced; refusing unchecked contract".into(),
                });
            }
        }
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
            if !source_functions.contains_key(&call.callee) && !externs.contains_key(&call.callee) {
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

    // Extern leaves carry no body and therefore no edge: including them would
    // make `reaches` index a graph node that does not exist, and would let a
    // declared assumption participate in the recursive post-fixed-point rule.
    let graph = supplied
        .iter()
        .map(|(symbol, function)| {
            (
                *symbol,
                function
                    .calls
                    .iter()
                    .map(|call| call.callee)
                    .filter(|callee| !externs.contains_key(callee))
                    .collect(),
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
    // Declared extern summaries seed the fixed point beside recursive
    // contracts: they are given, never computed from a body.
    summaries.extend(externs.iter().map(|(symbol, s)| (*symbol, s.clone())));
    while summaries.len() < local.len() + externs.len() {
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
                    region_operations: function.semantics.region_operations.clone(),
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

    for parameter in &source.parameters {
        if let Some(annotation) = &parameter.usage
            && semantics.usage_declarations.get(&parameter.name) != Some(&annotation.value)
        {
            return Err(diagnostic(
                SemanticPass::Usage,
                annotation.span,
                format!(
                    "parameter `{}` usage declaration does not match its source annotation",
                    parameter.name
                ),
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
    // Card `QUANT-REGION-SOURCE`: the error-region model's refusals carry the
    // finer `Q300` and `Q303`–`Q309` codes the `ErrorRegion` pass **already**
    // allocates for exactly those conditions — the same allocation
    // `qcore_core::region_task_credit::RegionTaskDiagnostic::code` reads — with
    // `Q302` still the generic presentation code of every other condition.
    // Nothing is allocated here. Before this card `region_operations` was empty
    // on every path that reaches this line, so no existing diagnostic moves.
    .map_err(|error| SourceDiagnostic {
        code: crate::source_region::error_region_code(&error),
        pass: SemanticPass::ErrorRegion,
        span: source.span,
        message: crate::source_region::error_region_message(&error),
    })?;
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
                .flat_map(|call| call.arguments.iter().chain(call.result.iter())),
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
        // Card `IF-ARMS`. A call inside a branch arm runs on one path and not
        // the other, but this composition is flat: it applies the callee's flow
        // on every path. That is faithful only when applying it is the identity,
        // so a conditional call demands a **neutral** callee summary — refused,
        // never approximated, and never silently joined into one of the arms.
        if call.conditional && callee.flow != Flow::zero() {
            return Err(diagnostic(
                SemanticPass::Branch,
                call.span,
                "`source.branch.v1` admits a call inside a branch arm only when the callee's \
                 summary flow is neutral: the whole-program call sequence is flat, so a \
                 conditional call would otherwise compose its callee's resource flow on paths \
                 that never perform it",
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
        // A call that returns fresh custody binds one new identity here. It is
        // introduced after the consumptions of the same call, so a callee that
        // both consumes and produces cannot silently reuse an argument's
        // identity for its result.
        if let Some(result) = call.result
            && !live.insert(result)
        {
            return Err(diagnostic(
                SemanticPass::Ownership,
                call.span,
                "a returned resource identity is already live in the caller",
            ));
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
            OwnershipOp::BorrowShared { body, .. }
            | OwnershipOp::BorrowMutable { body, .. }
            | OwnershipOp::LendRange { body, .. } => {
                collect_ownership_identities(body, result);
            }
            OwnershipOp::Branch { then_ops, else_ops } => {
                collect_ownership_identities(then_ops, result);
                collect_ownership_identities(else_ops, result);
            }
            OwnershipOp::Read(_)
            | OwnershipOp::Write(_)
            | OwnershipOp::ReadRange { .. }
            | OwnershipOp::WriteRange { .. }
            | OwnershipOp::Move { .. }
            | OwnershipOp::Copy(_)
            | OwnershipOp::Drop(_)
            | OwnershipOp::TransferOut(_)
            // Card `AGGREGATE-OWNED-FIELD`: the aggregate operations move,
            // access and release identities an `Acquire` already introduced;
            // none introduces one.
            | OwnershipOp::Compose { .. }
            | OwnershipOp::ReadField(_)
            | OwnershipOp::WriteField(_)
            | OwnershipOp::MoveOut { .. }
            | OwnershipOp::DropField(_)
            // Card `W48-QR-USABLE-RECORDS`: the nested composition, the refill
            // and the live-record check likewise introduce no identity.
            | OwnershipOp::ComposeNested { .. }
            | OwnershipOp::RefillField { .. }
            | OwnershipOp::AccessRecord(_) => {}
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
#[path = "../tests/unit/whole_program.rs"]
mod tests;
