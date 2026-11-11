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

//! Explicit guarded obligations. This proposed model is not a verdict consumer.
pub use qcore_frontend::guarded_decomposition::Clause;
use qcore_schemas::{
    hypothesis::{Context, Disposition, Graph, Warrant},
    premise_flow::View,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Environment,
    Prerequisite,
    Operation,
    Condition,
    Case,
    Postcondition,
}

/// All propositions are proposed. Dependencies identify existing graph atoms;
/// they assert neither equivalence nor implication from those atoms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub role: Role,
    pub proposition: String,
    pub dependencies: Vec<String>,
}

/// Sealed by fresh authoritative source replay and complete structural checking.
/// There is deliberately no Deserialize implementation or truth-bearing field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lowered {
    module: String,
    declaration: String,
    source_sha256: String,
    graph_sha256: String,
    readable_sha256: String,
    view: View,
    bindings: BTreeMap<String, Binding>,
    guards: Vec<Clause>,
    alternatives: Vec<Clause>,
    terminal_postconditions: Vec<Clause>,
}
impl Lowered {
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn graph_sha256(&self) -> &str {
        &self.graph_sha256
    }
    pub fn guards(&self) -> &[Clause] {
        &self.guards
    }
    pub fn alternatives(&self) -> &[Clause] {
        &self.alternatives
    }
    pub fn terminal_postconditions(&self) -> &[Clause] {
        &self.terminal_postconditions
    }
    pub fn bindings(&self) -> &BTreeMap<String, Binding> {
        &self.bindings
    }
    /// Binds every source/model identity, symbol role and proposition, including
    /// alternatives for which no evidence is presented.
    pub fn symbol_sha256(&self, symbol: &str) -> Result<String, String> {
        let binding = self
            .bindings
            .get(symbol)
            .ok_or("guarded-decomposition: unknown symbol")?;
        let bytes = serde_json::to_vec(&(
            1,
            &self.module,
            &self.declaration,
            &self.source_sha256,
            &self.graph_sha256,
            &self.readable_sha256,
            self.view,
            &self.bindings,
            symbol,
            binding,
        ))
        .map_err(|e| e.to_string())?;
        Ok(qcore_schemas::sha256_hex(&bytes))
    }
}

/// Replay exact property-specific source bytes first, then lower the separately
/// supplied document. `expected_graph` is independently held by the caller.
/// This API never strips unknown clauses to rescue a failed vocabulary loader.
pub fn replay(
    view: View,
    sources: &BTreeMap<String, String>,
    module: &str,
    declaration: &str,
    expected_graph: &Graph,
    bindings: &BTreeMap<String, Binding>,
    readable: &str,
) -> Result<Lowered, String> {
    let graphs = match view {
        View::Resource => crate::hypothesis::replay(sources)?,
        View::Bounds => crate::bounds_hypotheses::replay(sources)?,
        View::Model => crate::bounds_hypotheses::replay_model(sources)?,
    };
    let graph = graphs
        .get(declaration)
        .ok_or("guarded-decomposition: missing declaration graph")?;
    if graph != expected_graph {
        return Err("guarded-decomposition: expected graph mismatch".into());
    }
    let source = sources
        .get(module)
        .ok_or("guarded-decomposition: missing exact module")?;
    // The graph map is keyed by declaration, so establish that it belongs to
    // the selected module rather than another source with an equal atom set.
    let program =
        qcore_frontend::frontend::parse_source_program(source).map_err(|e| e.to_string())?;
    if !program
        .declarations
        .iter()
        .any(|d| d.value.name == declaration)
    {
        return Err("guarded-decomposition: declaration belongs to another module".into());
    }
    lower(view, module, declaration, source, graph, bindings, readable)
}
fn lower(
    view: View,
    module: &str,
    declaration: &str,
    source: &str,
    graph: &Graph,
    bindings: &BTreeMap<String, Binding>,
    readable: &str,
) -> Result<Lowered, String> {
    let clauses = qcore_frontend::guarded_decomposition::parse(readable)?;
    if bindings.len() > 256 {
        return Err("guarded-decomposition: symbol limit".into());
    }
    for (id, binding) in bindings {
        if id.len() > 128
            || !qcore_frontend::premise_flow::identifier(id)
            || binding.proposition.trim().is_empty()
            || binding.proposition.len() > 4096
            || binding.proposition.chars().any(char::is_control)
            || binding.dependencies.len() > 256
            || binding.dependencies.windows(2).any(|w| w[0] >= w[1])
            || binding
                .dependencies
                .iter()
                .any(|a| !graph.atoms.contains_key(a))
        {
            return Err("guarded-decomposition: malformed binding or graph dependency".into());
        }
    }
    let mut used = BTreeSet::new();
    let mut require = |id: &str, role| -> Result<(), String> {
        if bindings.get(id).map(|b| b.role) != Some(role) {
            return Err(format!(
                "guarded-decomposition: wrong or missing role for {id}"
            ));
        }
        used.insert(id.to_string());
        Ok(())
    };
    let mut enabled = BTreeSet::new();
    let mut branched = BTreeSet::new();
    let mut cases = BTreeSet::new();
    let mut terminated = BTreeSet::new();
    let mut forbidden = BTreeSet::new();
    let mut guards = Vec::new();
    let mut alternatives = Vec::new();
    let mut terminal_postconditions = Vec::new();
    for clause in clauses {
        match &clause {
            Clause::Enable {
                environment,
                prerequisites,
                operation,
            } => {
                require(environment, Role::Environment)?;
                let mut distinct = BTreeSet::new();
                for p in prerequisites {
                    require(p, Role::Prerequisite)?;
                    if !distinct.insert(p) {
                        return Err("guarded-decomposition: duplicate prerequisite".into());
                    }
                }
                require(operation, Role::Operation)?;
                if !enabled.insert(operation.clone()) {
                    return Err("guarded-decomposition: duplicate operation guard".into());
                }
                guards.push(clause);
            }
            Clause::Forbid {
                condition,
                operation,
            } => {
                require(condition, Role::Condition)?;
                require(operation, Role::Operation)?;
                if !forbidden.insert((condition.clone(), operation.clone())) {
                    return Err("guarded-decomposition: duplicate forbidden obligation".into());
                }
                guards.push(clause);
            }
            Clause::Alternatives {
                operation,
                cases: branches,
            } => {
                require(operation, Role::Operation)?;
                if !branched.insert(operation.clone()) {
                    return Err("guarded-decomposition: duplicate alternatives".into());
                }
                for case in branches {
                    require(case, Role::Case)?;
                    if !cases.insert(case.clone()) {
                        return Err("guarded-decomposition: duplicate or shared case".into());
                    }
                }
                alternatives.push(clause);
            }
            Clause::Terminal {
                case,
                postcondition,
            } => {
                require(case, Role::Case)?;
                require(postcondition, Role::Postcondition)?;
                if !terminated.insert(case.clone()) {
                    return Err("guarded-decomposition: duplicate terminal case".into());
                }
                terminal_postconditions.push(clause);
            }
        }
    }
    if enabled.is_empty()
        || enabled != branched
        || cases != terminated
        || forbidden.iter().any(|(_, op)| !enabled.contains(op))
        || used.len() != bindings.len()
    {
        return Err("guarded-decomposition: incomplete operations/cases or unused symbols".into());
    }
    Ok(Lowered {
        module: module.into(),
        declaration: declaration.into(),
        source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
        graph_sha256: qcore_schemas::sha256_hex(
            &serde_json::to_vec(graph).map_err(|e| e.to_string())?,
        ),
        readable_sha256: qcore_schemas::sha256_hex(readable.as_bytes()),
        view,
        bindings: bindings.clone(),
        guards,
        alternatives,
        terminal_postconditions,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub symbol: String,
    pub symbol_sha256: String,
    pub context: Context,
    pub disposition: Disposition,
    pub warrant: Warrant,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Open,
    Inactive,
    Satisfied,
    Refuted,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedEvaluation {
    pub guards: Vec<Status>,
    pub alternatives: Vec<Status>,
    pub terminal_postconditions: Vec<Status>,
    pub dependencies: Vec<String>,
}

fn conjunction(values: impl Iterator<Item = Disposition>) -> Disposition {
    let mut open = false;
    for value in values {
        match value {
            Disposition::Refuted => return Disposition::Refuted,
            Disposition::Open => open = true,
            Disposition::Supported => (),
        }
    }
    if open {
        Disposition::Open
    } else {
        Disposition::Supported
    }
}
fn implication(guard: Disposition, conclusion: Disposition) -> Status {
    match (guard, conclusion) {
        (Disposition::Refuted, _) => Status::Inactive,
        (Disposition::Open, _) => Status::Open,
        (Disposition::Supported, Disposition::Open) => Status::Open,
        (Disposition::Supported, Disposition::Supported) => Status::Satisfied,
        (Disposition::Supported, Disposition::Refuted) => Status::Refuted,
    }
}

/// Evaluate explicitly presented proposed assignments only. This cannot prove a
/// source proposition, execute an operation, mint evidence, mutate graph truth,
/// establish branch exhaustiveness, classify standing or construct Unsafe.
pub fn evaluate_proposed(
    document: &Lowered,
    expected_context: &Context,
    presented_context: &Context,
    assignments: &[Assignment],
) -> Result<ProposedEvaluation, String> {
    if expected_context != presented_context {
        return Err("guarded-decomposition: context mismatch".into());
    }
    let mut evidence = BTreeMap::new();
    for a in assignments {
        if &a.context != expected_context
            || a.symbol_sha256 != document.symbol_sha256(&a.symbol)?
            || matches!(a.warrant, Warrant::Derived | Warrant::ProviderProposed)
            || evidence.insert(a.symbol.clone(), a.disposition).is_some()
        {
            return Err("guarded-decomposition: invalid/duplicate proposed assignment".into());
        }
    }
    let value = |id: &str| evidence.get(id).copied().unwrap_or(Disposition::Open);
    let guards = document
        .guards
        .iter()
        .map(|clause| match clause {
            Clause::Enable {
                environment,
                prerequisites,
                operation,
            } => implication(
                value(operation),
                conjunction(
                    std::iter::once(value(environment))
                        .chain(prerequisites.iter().map(|p| value(p))),
                ),
            ),
            Clause::Forbid {
                condition,
                operation,
            } => implication(
                value(condition),
                match value(operation) {
                    Disposition::Supported => Disposition::Refuted,
                    Disposition::Refuted => Disposition::Supported,
                    Disposition::Open => Disposition::Open,
                },
            ),
            _ => unreachable!(),
        })
        .collect();
    let alternatives = document
        .alternatives
        .iter()
        .map(|clause| {
            let Clause::Alternatives { operation, cases } = clause else {
                unreachable!()
            };
            let dispositions: Vec<_> = cases.iter().map(|c| value(c)).collect();
            let coverage = if dispositions.contains(&Disposition::Supported) {
                Disposition::Supported
            } else if dispositions.iter().all(|d| *d == Disposition::Refuted) {
                Disposition::Refuted
            } else {
                Disposition::Open
            };
            implication(value(operation), coverage)
        })
        .collect();
    let terminal_postconditions = document
        .terminal_postconditions
        .iter()
        .map(|clause| {
            let Clause::Terminal {
                case,
                postcondition,
            } = clause
            else {
                unreachable!()
            };
            implication(value(case), value(postcondition))
        })
        .collect();
    Ok(ProposedEvaluation {
        guards,
        alternatives,
        terminal_postconditions,
        dependencies: document.bindings.keys().cloned().collect(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Impact {
    pub symbols_requiring_recheck: Vec<String>,
    pub guard_indices: Vec<usize>,
    pub alternative_indices: Vec<usize>,
    pub terminal_indices: Vec<usize>,
}
fn symbols(clause: &Clause) -> Vec<&str> {
    match clause {
        Clause::Enable {
            environment,
            prerequisites,
            operation,
        } => std::iter::once(environment.as_str())
            .chain(prerequisites.iter().map(String::as_str))
            .chain(std::iter::once(operation.as_str()))
            .collect(),
        Clause::Forbid {
            condition,
            operation,
        } => vec![condition, operation],
        Clause::Alternatives { operation, cases } => std::iter::once(operation.as_str())
            .chain(cases.iter().map(String::as_str))
            .collect(),
        Clause::Terminal {
            case,
            postcondition,
        } => vec![case, postcondition],
    }
}
/// Conservative recheck propagation, never automatic truth/refutation updates.
/// An environment/prerequisite change reaches every declared alternative and
/// terminal; a terminal change cannot retroactively establish an entry guard.
pub fn affected_obligations(
    document: &Lowered,
    expected_context: &Context,
    presented_context: &Context,
    changed_symbols: &[String],
) -> Result<Impact, String> {
    if expected_context != presented_context {
        return Err("guarded-decomposition: impact context mismatch".into());
    }
    let mut dirty = BTreeSet::new();
    for id in changed_symbols {
        if !document.bindings.contains_key(id) || !dirty.insert(id.clone()) {
            return Err("guarded-decomposition: invalid/duplicate changed symbol".into());
        }
    }
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for clause in document
        .guards
        .iter()
        .chain(&document.alternatives)
        .chain(&document.terminal_postconditions)
    {
        let (from, to) = match clause {
            Clause::Enable {
                environment,
                prerequisites,
                operation,
            } => (
                std::iter::once(environment.clone())
                    .chain(prerequisites.iter().cloned())
                    .collect::<Vec<_>>(),
                vec![operation.clone()],
            ),
            Clause::Forbid {
                condition,
                operation,
            } => (vec![condition.clone()], vec![operation.clone()]),
            Clause::Alternatives { operation, cases } => (vec![operation.clone()], cases.clone()),
            Clause::Terminal {
                case,
                postcondition,
            } => (vec![case.clone()], vec![postcondition.clone()]),
        };
        for source in from {
            edges.entry(source).or_default().extend(to.clone());
        }
    }
    loop {
        let before = dirty.len();
        let reached: Vec<_> = dirty
            .iter()
            .flat_map(|s| edges.get(s).into_iter().flatten().cloned())
            .collect();
        dirty.extend(reached);
        if dirty.len() == before {
            break;
        }
    }
    let affected = |clauses: &[Clause]| {
        clauses
            .iter()
            .enumerate()
            .filter_map(|(i, c)| symbols(c).iter().any(|s| dirty.contains(*s)).then_some(i))
            .collect()
    };
    Ok(Impact {
        guard_indices: affected(&document.guards),
        alternative_indices: affected(&document.alternatives),
        terminal_indices: affected(&document.terminal_postconditions),
        symbols_requiring_recheck: dirty.into_iter().collect(),
    })
}

/// Whole declaration metadata. Serialization cannot construct checked lowering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inline {
    pub entry: String,
    pub bindings: BTreeMap<String, Binding>,
    pub readable: String,
}
pub fn is_source_clause(source: &str) -> bool {
    qcore_frontend::guarded_decomposition::is_source_clause(source)
}
/// Validate every source symbol and obligation before a property loader treats
/// it as proposed metadata. Nothing is inferred from a familiar head.
pub fn declaration(
    decl: &qcore_frontend::frontend::SourceDeclaration,
    graph: &Graph,
) -> Result<Option<Inline>, String> {
    use qcore_frontend::{
        frontend::SourceContractClauseKind,
        guarded_decomposition::{SourceClause, parse_source_clause},
    };
    let mut bindings = BTreeMap::new();
    let mut document = None;
    for clause in decl.function_contract.iter().flat_map(|c| &c.clauses) {
        let Some(parsed) = parse_source_clause(&clause.value.expression_source)? else {
            continue;
        };
        if clause.value.kind != SourceContractClauseKind::Requires {
            return Err("guarded source: only requires metadata admitted".into());
        }
        match parsed {
            SourceClause::Symbol {
                id,
                role,
                proposition,
                dependencies,
            } => {
                let role = match role.as_str() {
                    "environment" => Role::Environment,
                    "prerequisite" => Role::Prerequisite,
                    "operation" => Role::Operation,
                    "condition" => Role::Condition,
                    "case" => Role::Case,
                    "postcondition" => Role::Postcondition,
                    _ => return Err("guarded source: unknown role".into()),
                };
                if bindings
                    .insert(
                        id,
                        Binding {
                            role,
                            proposition,
                            dependencies,
                        },
                    )
                    .is_some()
                {
                    return Err("guarded source: duplicate symbol".into());
                }
            }
            SourceClause::Document { id, readable } => {
                if document.replace((id, readable)).is_some() {
                    return Err("guarded source: exactly one document per declaration".into());
                }
            }
        }
    }
    let Some((entry, readable)) = document else {
        return if bindings.is_empty() {
            Ok(None)
        } else {
            Err("guarded source: symbols without document".into())
        };
    };
    // Local validation does not publish the synthetic identities; full replay
    // below seals the actual whole-module/property identities afresh.
    lower(
        View::Resource,
        "",
        &decl.name,
        "",
        graph,
        &bindings,
        &readable,
    )?;
    Ok(Some(Inline {
        entry,
        bindings,
        readable,
    }))
}
/// Replay the complete original property source first, then compare the entire
/// actual source metadata with the independently expected document and graph.
pub fn replay_inline(
    view: View,
    sources: &BTreeMap<String, String>,
    module: &str,
    name: &str,
    graph: &Graph,
    expected: &Inline,
) -> Result<Lowered, String> {
    let checked = replay(
        view,
        sources,
        module,
        name,
        graph,
        &expected.bindings,
        &expected.readable,
    )?;
    let parsed = qcore_frontend::frontend::parse_source_program(
        sources.get(module).ok_or("guarded source: module absent")?,
    )
    .map_err(|e| e.to_string())?;
    let matching = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == name)
        .collect::<Vec<_>>();
    if matching.len() != 1 || declaration(&matching[0].value, graph)?.as_ref() != Some(expected) {
        return Err("guarded source: complete independently held metadata mismatch".into());
    }
    Ok(checked)
}
