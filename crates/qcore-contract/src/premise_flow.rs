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

//! Source-replayed proposed logic. No result admits a C/source safety claim.
use crate::hypothesis;
use qcore_frontend::frontend::{SourceContractClauseKind, SourceDeclaration, parse_source_program};
use qcore_schemas::hypothesis::{Context, Disposition, Graph, Warrant};
pub use qcore_schemas::premise_flow::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn is_clause(source: &str) -> bool {
    qcore_frontend::premise_flow::is_clause(source)
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    Ok(qcore_schemas::sha256_hex(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
pub fn document_digest(document: &Document) -> Result<String, String> {
    digest(document)
}

/// Structural boundary from serialization-free frontend syntax to wire records.
fn source_expr(expr: qcore_frontend::premise_flow::Expr) -> Expr {
    use qcore_frontend::premise_flow::Expr as Source;
    match expr {
        Source::Atom { atom } => Expr::Atom { atom },
        Source::Flow { flow } => Expr::Flow { flow },
        Source::And { left, right } => Expr::And {
            left: Box::new(source_expr(*left)),
            right: Box::new(source_expr(*right)),
        },
        Source::Or { left, right } => Expr::Or {
            left: Box::new(source_expr(*left)),
            right: Box::new(source_expr(*right)),
        },
        Source::Except { left, right } => Expr::Except {
            left: Box::new(source_expr(*left)),
            right: Box::new(source_expr(*right)),
        },
        Source::Terminal { left, right } => Expr::Terminal {
            left: Box::new(source_expr(*left)),
            right: Box::new(source_expr(*right)),
        },
    }
}

/// Source syntax plus complete reference/terminal validation. No model/evidence
/// is needed merely to load metadata; interpreted clauses keep their own loader.
pub fn declaration(
    decl: &SourceDeclaration,
    graph: &Graph,
) -> Result<BTreeMap<String, Expr>, String> {
    let mut entries = BTreeMap::new();
    for clause in decl.function_contract.iter().flat_map(|c| &c.clauses) {
        let Some((id, expr)) =
            qcore_frontend::premise_flow::parse_clause(&clause.value.expression_source)?
        else {
            continue;
        };
        if clause.value.kind != SourceContractClauseKind::Requires {
            return Err("flow: only a requires clause admits premise flow".into());
        }
        if entries.insert(id, source_expr(expr)).is_some() {
            return Err("flow: duplicate entry ID".into());
        }
    }
    dependencies(&entries, graph)?;
    Ok(entries)
}
/// Includes every logical alternative, not only a successful branch.
pub fn dependencies(
    entries: &BTreeMap<String, Expr>,
    graph: &Graph,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    if entries.len() > 256 {
        return Err("flow: entry limit".into());
    }
    let mut out = BTreeMap::new();
    let mut budget = 0;
    for (id, expr) in entries {
        if !qcore_frontend::premise_flow::identifier(id) {
            return Err("flow: invalid entry ID".into());
        }
        let mut atoms = BTreeSet::new();
        let mut active = BTreeSet::from([id.clone()]);
        walk(
            expr,
            entries,
            graph,
            &mut active,
            &mut atoms,
            true,
            (0, &mut budget),
        )?;
        out.insert(id.clone(), atoms.into_iter().collect());
    }
    Ok(out)
}
fn walk(
    expr: &Expr,
    entries: &BTreeMap<String, Expr>,
    graph: &Graph,
    active: &mut BTreeSet<String>,
    atoms: &mut BTreeSet<String>,
    root: bool,
    state: (usize, &mut usize),
) -> Result<(), String> {
    let (depth, budget) = state;
    *budget += 1;
    if depth > 128 || *budget > 4096 {
        return Err("flow: expanded expression/reference limit".into());
    }
    match expr {
        Expr::Atom { atom } => {
            if !graph.atoms.contains_key(atom) {
                return Err("flow: missing graph atom".into());
            }
            atoms.insert(atom.clone());
        }
        Expr::Flow { flow } => {
            if !active.insert(flow.clone()) {
                return Err("flow: cyclic reference".into());
            }
            let target = entries.get(flow).ok_or("flow: missing entry reference")?;
            walk(
                target,
                entries,
                graph,
                active,
                atoms,
                root,
                (depth + 1, &mut *budget),
            )?;
            active.remove(flow);
        }
        Expr::And { left, right }
        | Expr::Or { left, right }
        | Expr::Except { left, right }
        | Expr::Terminal { left, right } => {
            if matches!(expr, Expr::Terminal { .. }) && !root {
                return Err(
                    "flow: terminal cannot have a subsequent continuation or nested consumer"
                        .into(),
                );
            }
            walk(
                left,
                entries,
                graph,
                active,
                atoms,
                false,
                (depth + 1, &mut *budget),
            )?;
            walk(
                right,
                entries,
                graph,
                active,
                atoms,
                false,
                (depth + 1, &mut *budget),
            )?;
        }
    }
    Ok(())
}
fn check_model(
    model: &Model,
    graph: &Graph,
    used: &BTreeSet<String>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let env = &model.environment;
    if env.id.trim().is_empty()
        || env.variables.is_empty()
        || env
            .variables
            .iter()
            .any(|(k, v)| k.trim().is_empty() || v.trim().is_empty())
        || model.atoms.keys().cloned().collect::<BTreeSet<_>>() != *used
    {
        return Err("flow: incomplete/excess independently held model/environment domain".into());
    }
    for (id, binding) in &model.atoms {
        if binding.atom_sha256 != digest(&graph.atoms[id])?
            || binding.environment != env.id
            || binding.proposition.trim().is_empty()
            || binding.quantifiers.is_empty()
            || binding.quantifiers.iter().any(|q| q.trim().is_empty())
            || binding.prerequisites != vec![env.id.clone()]
            || binding.conclusion.trim().is_empty()
        {
            return Err("flow: model/atom/proposition/prerequisite mismatch".into());
        }
        for reference in [&binding.model, &binding.audit] {
            if reference.path.is_empty()
                || reference.path.starts_with('/')
                || reference.path.contains('\\')
                || reference
                    .path
                    .split('/')
                    .any(|p| matches!(p, "" | "." | ".."))
                || !sha(&reference.sha256)
                || artifacts
                    .get(&reference.path)
                    .map(|b| qcore_schemas::sha256_hex(b))
                    != Some(reference.sha256.clone())
            {
                return Err("flow: missing or changed explicit model/audit bytes".into());
            }
        }
    }
    Ok(())
}
pub fn compile(
    sources: &BTreeMap<String, String>,
    graphs: &BTreeMap<String, Graph>,
    models: &BTreeMap<String, Model>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Document>, String> {
    compile_view(View::Resource, sources, graphs, models, artifacts)
}
pub fn compile_bounds(
    sources: &BTreeMap<String, String>,
    graphs: &BTreeMap<String, Graph>,
    models: &BTreeMap<String, Model>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Document>, String> {
    compile_view(View::Bounds, sources, graphs, models, artifacts)
}
pub fn compile_model(
    sources: &BTreeMap<String, String>,
    graphs: &BTreeMap<String, Graph>,
    models: &BTreeMap<String, Model>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Document>, String> {
    compile_view(View::Model, sources, graphs, models, artifacts)
}
pub fn compile_view(
    view: View,
    sources: &BTreeMap<String, String>,
    graphs: &BTreeMap<String, Graph>,
    models: &BTreeMap<String, Model>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Document>, String> {
    match view {
        View::Resource => hypothesis::check_replay(sources, graphs)?,
        View::Bounds => crate::bounds_hypotheses::check_replay(sources, graphs)?,
        View::Model => {
            if crate::bounds_hypotheses::replay_model(sources)? != *graphs {
                return Err("flow: independently held model graph differs".into());
            }
        }
    }
    let mut documents = BTreeMap::new();
    for (module, source) in sources {
        for decl in parse_source_program(source)
            .map_err(|e| e.to_string())?
            .declarations
        {
            let has_flow = decl
                .value
                .function_contract
                .iter()
                .flat_map(|c| &c.clauses)
                .any(|c| is_clause(&c.value.expression_source));
            if !has_flow {
                continue;
            }
            let graph = graphs
                .get(&decl.value.name)
                .ok_or("flow: missing property declaration graph")?;
            let entries = declaration(&decl.value, graph)?;
            let deps = dependencies(&entries, graph)?;
            let used = deps.into_values().flatten().collect::<BTreeSet<_>>();
            let model = models
                .get(&decl.value.name)
                .ok_or("flow: missing independently held model")?;
            check_model(model, graph, &used, artifacts)?;
            let document = Document {
                version: 1,
                view,
                source_module: module.clone(),
                declaration: decl.value.name.clone(),
                source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
                graph_sha256: hypothesis::digest(graph)?,
                model_sha256: digest(model)?,
                entries,
            };
            if documents.insert(decl.value.name, document).is_some() {
                return Err("flow: duplicate declaration".into());
            }
        }
    }
    if documents.keys().collect::<Vec<_>>() != models.keys().collect::<Vec<_>>() {
        return Err("flow: excess independently held declaration models".into());
    }
    Ok(documents)
}
pub struct Expected<'a> {
    pub sources: &'a BTreeMap<String, String>,
    pub graphs: &'a BTreeMap<String, Graph>,
    pub models: &'a BTreeMap<String, Model>,
    pub artifacts: &'a BTreeMap<String, Vec<u8>>,
    pub document: &'a Document,
    pub context: &'a Context,
}
fn complete(c: &Context) -> bool {
    [
        &c.artifact,
        &c.property,
        &c.semantic_profile,
        &c.scope,
        &c.bounds,
        &c.assumptions,
        &c.source_identity,
        &c.build_identity,
        &c.evidence_version,
    ]
    .iter()
    .all(|s| !s.is_empty())
}
/// Conditional proposed logic only. A source adapter may wrap this ONLY after
/// independently replaying its private-constructor checked scoped receipt.
pub fn evaluate_proposed(
    packet: &FlowInstance,
    held: Expected<'_>,
    entry: &str,
) -> Result<ProposedEvaluation, String> {
    if !complete(held.context) || packet.hypotheses.context != *held.context {
        return Err("flow: incomplete or foreign Context".into());
    }
    if packet.document.version != 1
        || packet.document != *held.document
        || packet.document_sha256 != document_digest(held.document)?
    {
        return Err("flow: stale document/version".into());
    }
    let replayed = compile_view(
        held.document.view,
        held.sources,
        held.graphs,
        held.models,
        held.artifacts,
    )?;
    if replayed.get(&held.document.declaration) != Some(held.document) {
        return Err("flow: stale source/model replay".into());
    }
    let graph = &held.graphs[&held.document.declaration];
    if packet.hypotheses.graph != *graph
        || packet.hypotheses.graph_sha256 != hypothesis::digest(graph)?
    {
        return Err("flow: stale graph".into());
    }
    let model = &held.models[&held.document.declaration];
    let mut values = BTreeMap::new();
    let mut warrants = BTreeMap::new();
    for e in &packet.hypotheses.evidence {
        if !model.atoms.contains_key(&e.atom)
            || e.context != *held.context
            || e.reference.trim().is_empty()
            || e.atom_sha256 != digest(&graph.atoms[&e.atom])?
            || !matches!(
                e.warrant,
                Warrant::Observed | Warrant::Declared | Warrant::HumanReviewed
            )
            || values.insert(e.atom.clone(), e.disposition).is_some()
        {
            return Err(
                "flow: duplicate/conflicting, stale, foreign or unsupported evidence".into(),
            );
        }
        warrants.insert(e.atom.clone(), e.warrant);
    }
    let expr = held
        .document
        .entries
        .get(entry)
        .ok_or("flow: missing selected entry")?;
    let (status, terminated) = eval(expr, &held.document.entries, &values);
    Ok(ProposedEvaluation {
        entry: entry.into(),
        disposition: status,
        terminated,
        atom_dependencies: dependencies(&held.document.entries, graph)?[entry].clone(),
        warrants,
        context: held.context.clone(),
    })
}
fn eval(
    expr: &Expr,
    entries: &BTreeMap<String, Expr>,
    values: &BTreeMap<String, Disposition>,
) -> (Disposition, bool) {
    use qcore_core::{hypothesis::Disposition as K, premise_flow as core};
    let to = |s| match s {
        Disposition::Open => K::Open,
        Disposition::Supported => K::Supported,
        Disposition::Refuted => K::Refuted,
    };
    let from = |s| match s {
        K::Open => Disposition::Open,
        K::Supported => Disposition::Supported,
        K::Refuted => Disposition::Refuted,
    };
    match expr {
        Expr::Atom { atom } => (
            values.get(atom).copied().unwrap_or(Disposition::Open),
            false,
        ),
        Expr::Flow { flow } => eval(&entries[flow], entries, values),
        Expr::And { left, right }
        | Expr::Or { left, right }
        | Expr::Except { left, right }
        | Expr::Terminal { left, right } => {
            let l = to(eval(left, entries, values).0);
            let r = to(eval(right, entries, values).0);
            match expr {
                Expr::And { .. } => (from(core::and(l, r)), false),
                Expr::Or { .. } => (from(core::or(l, r)), false),
                Expr::Except { .. } => (from(core::except(l, r)), false),
                _ => {
                    let (s, t) = core::terminal(l, r);
                    (from(s), t)
                }
            }
        }
    }
}
/// Strict versioned wire data. Duplicate keys/alternate encodings cannot hide
/// a different entry, graph, source or model behind normalization.
pub fn decode(bytes: &[u8]) -> Result<FlowInstance, String> {
    let packet: FlowInstance = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.document.version != 1
        || serde_json::to_vec(&packet).map_err(|e| e.to_string())? != bytes
    {
        return Err("flow: unsupported/noncanonical wire packet".into());
    }
    Ok(packet)
}
