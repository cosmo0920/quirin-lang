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

//! Canonical conjunction-only graph validation and retained-text replay.
use crate::contract_loader::DeclaredPremise;
use qcore_frontend::frontend::{SourceContractClauseKind, SourceDeclaration};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::hypothesis::*;
use std::collections::{BTreeMap, BTreeSet};

fn identifier(s: &str) -> bool {
    s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn plain(s: &str) -> bool {
    !s.trim().is_empty() && !s.contains('\\') && !s.chars().any(char::is_control)
}
pub fn is_clause(s: &str) -> bool {
    s.split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .is_some_and(|s| {
            matches!(
                s,
                "resource_hypothesis" | "resource_premise_all" | "resource_premise_decomposition"
            )
        })
}
/// Reads a declaration-local graph, including legacy metadata without rewriting its bytes.
pub fn declaration(decl: &SourceDeclaration, legacy: &[DeclaredPremise]) -> Result<Graph, String> {
    let mut graph = Graph {
        version: 1,
        atoms: BTreeMap::new(),
        premises: BTreeMap::new(),
        refinements: BTreeMap::new(),
        requirements: Vec::new(),
    };
    for premise in legacy {
        append_legacy(
            &mut graph,
            ErasedPremise {
                name: premise.name.clone(),
                statement: premise.statement.clone(),
                origin: premise.origin.as_str().into(),
            },
        )?;
    }
    if decl
        .function_contract
        .iter()
        .flat_map(|c| &c.clauses)
        .any(|c| matches!(c.value.kind, SourceContractClauseKind::QuantCondition(_)))
    {
        crate::quant_condition::declaration(decl)?;
    }
    let mut attachments = Vec::new();
    for clause in decl.function_contract.iter().flat_map(|c| &c.clauses) {
        let source = &clause.value.expression_source;
        if let SourceContractClauseKind::QuantCondition(selector) = &clause.value.kind {
            let condition = qcore_frontend::quant_condition::parse(selector.clone(), source)?;
            if condition.selector == qcore_schemas::quant_condition::Selector::Entry {
                let definition = condition.proposition;
                let id = definition.id.clone();
                if graph
                    .atoms
                    .insert(
                        id.clone(),
                        Atom {
                            id,
                            proposition: Proposition::EntryCondition { definition },
                        },
                    )
                    .is_some()
                {
                    return Err("hypothesis: duplicate formal atom ID".into());
                }
            }
            // Exit assertions are retained by quant_condition::declaration.
            // They never become hypotheses or acquire a truth assignment here.
            continue;
        }
        if clause.value.kind == SourceContractClauseKind::QuantProposition {
            let definition = qcore_frontend::quant_proposition::parse(source)?;
            let id = definition.id.clone();
            let atom = Atom {
                id: id.clone(),
                proposition: Proposition::Formula { definition },
            };
            if graph.atoms.insert(id, atom).is_some() {
                return Err("hypothesis: duplicate formal atom ID".into());
            }
            continue;
        }

        if !is_clause(source) {
            continue;
        }
        if clause.value.kind != SourceContractClauseKind::Requires {
            return Err("hypothesis: graph clauses must be requires".into());
        }
        let ts = lex(source)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|t| t.kind)
            .filter(|t| *t != TokenKind::Eof)
            .collect::<Vec<_>>();
        let malformed = || format!("hypothesis: malformed graph clause `{source}`");
        let [
            TokenKind::Word(head),
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(id),
            TokenKind::Symbol(Symbol::Comma),
            rest @ ..,
        ] = ts.as_slice()
        else {
            return Err(malformed());
        };
        if !identifier(id) || id.starts_with("legacy_") {
            return Err(malformed());
        }
        if head == "resource_hypothesis" {
            let [
                TokenKind::StringLiteral(statement),
                TokenKind::Symbol(Symbol::RightParen),
            ] = rest
            else {
                return Err(malformed());
            };
            if !plain(statement) {
                return Err(malformed());
            }
            let atom = Atom {
                id: id.clone(),
                proposition: Proposition::Opaque {
                    statement: statement.clone(),
                },
            };
            if graph.atoms.get(id).is_some_and(|old| *old != atom) {
                return Err("hypothesis: conflicting atom ID".into());
            }
            graph.atoms.insert(id.clone(), atom);
        } else {
            if head == "resource_premise_all"
                && (graph.premises.contains_key(id) || legacy.iter().any(|p| p.name == *id))
            {
                return Err("hypothesis: conflicting premise ID".into());
            }
            let mut children = Vec::new();
            for (i, t) in rest.iter().enumerate() {
                if i.is_multiple_of(2) {
                    let TokenKind::Word(child) = t else {
                        return Err(malformed());
                    };
                    children.push(child.clone());
                } else if *t
                    != TokenKind::Symbol(if i == rest.len() - 1 {
                        Symbol::RightParen
                    } else {
                        Symbol::Comma
                    })
                {
                    return Err(malformed());
                }
            }
            if !rest.len().is_multiple_of(2) {
                return Err(malformed());
            }
            if head == "resource_premise_decomposition" {
                attachments.push((id.clone(), children));
                continue;
            }
            graph.premises.insert(
                id.clone(),
                Conjunction {
                    id: id.clone(),
                    erasure: ErasedPremise {
                        name: id.clone(),
                        statement: format!("all({})", children.join(",")),
                        origin: "declared".into(),
                    },
                    children,
                },
            );
        }
    }
    for (name, children) in attachments {
        let targets = graph
            .premises
            .iter()
            .filter(|(id, p)| {
                id.starts_with("legacy_premise_")
                    && p.erasure.name == name
                    && p.erasure.origin == "declared"
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if targets.len() != 1 || graph.refinements.contains_key(&name) {
            return Err(
                "hypothesis: decomposition needs one existing declared premise and one attachment"
                    .into(),
            );
        }
        graph.refinements.insert(
            name,
            Refinement {
                premise: targets[0].clone(),
                children,
            },
        );
    }
    let grouped = graph
        .premises
        .values()
        .flat_map(|p| p.children.iter())
        .chain(graph.refinements.values().flat_map(|p| p.children.iter()))
        .collect::<BTreeSet<_>>();
    graph.requirements = graph
        .atoms
        .iter()
        .filter(|(id, a)| {
            matches!(a.proposition, Proposition::EntryCondition { .. })
                || (matches!(a.proposition, Proposition::Formula { .. }) && !grouped.contains(id))
        })
        .map(|(id, _)| id.clone())
        .collect();
    graph.version = expected_version(&graph);
    validate(&graph)?;
    Ok(graph)
}
/// Canonical encoding version for the actual node forms. This does not
/// validate roots, references, typing or erasure; callers must still validate.
pub fn expected_version(graph: &Graph) -> u32 {
    if graph
        .atoms
        .values()
        .any(|a| matches!(a.proposition, Proposition::EntryCondition { .. }))
    {
        4
    } else if graph
        .atoms
        .values()
        .any(|a| matches!(a.proposition, Proposition::Formula { .. }))
    {
        3
    } else if !graph.refinements.is_empty() {
        2
    } else {
        1
    }
}
pub fn validate(graph: &Graph) -> Result<(), String> {
    if graph.version != expected_version(graph) {
        return Err("hypothesis: unsupported or noncanonical graph version".into());
    }
    let mut used = BTreeSet::new();
    for (id, atom) in &graph.atoms {
        if id != &atom.id || !identifier(id) {
            return Err("hypothesis: invalid atom ID".into());
        }
        match &atom.proposition {
            Proposition::Opaque { statement } if plain(statement) => (),
            Proposition::Formula { definition } | Proposition::EntryCondition { definition } => {
                if definition.id != *id {
                    return Err("hypothesis: formal atom ID mismatch".into());
                }
                qcore_frontend::quant_proposition::validate(definition)?;
            }
            Proposition::Typed { .. } => {
                return Err("hypothesis: unknown typed predicate (registry is empty in v1)".into());
            }
            _ => return Err("hypothesis: invalid opaque statement".into()),
        }
    }
    for (id, premise) in &graph.premises {
        if id != &premise.id
            || !identifier(id)
            || premise.children.is_empty()
            || premise.children.windows(2).any(|w| w[0] >= w[1])
        {
            return Err("hypothesis: noncanonical conjunction".into());
        }
        if graph.atoms.contains_key(id) {
            return Err("hypothesis: overlapping node IDs".into());
        }
        if !id.starts_with("legacy_")
            && (premise.erasure.name != *id
                || premise.erasure.origin != "declared"
                || premise.erasure.statement != format!("all({})", premise.children.join(",")))
        {
            return Err("hypothesis: invalid structured erasure or overlapping IDs".into());
        }
        if id.starts_with("legacy_")
            && (premise.children != vec![id.replacen("legacy_premise_", "legacy_atom_", 1)]
                || !id.starts_with("legacy_premise_")
                || !matches!(graph.atoms.get(&id.replacen("legacy_premise_", "legacy_atom_", 1)).map(|a| &a.proposition), Some(Proposition::Opaque {statement}) if *statement == premise.erasure.statement))
        {
            return Err("hypothesis: invalid legacy erasure".into());
        }
        for child in &premise.children {
            if !graph.atoms.contains_key(child) {
                return Err("hypothesis: missing atom or cyclic premise edge".into());
            }
            used.insert(child.clone());
        }
    }
    for (name, refinement) in &graph.refinements {
        let target = graph
            .premises
            .get(&refinement.premise)
            .ok_or("hypothesis: missing refinement target")?;
        if !identifier(name)
            || name.starts_with("legacy_")
            || !refinement.premise.starts_with("legacy_premise_")
            || target.erasure.name != *name
            || target.erasure.origin != "declared"
            || refinement.children.is_empty()
            || refinement.children.windows(2).any(|w| w[0] >= w[1])
        {
            return Err("hypothesis: invalid proposed decomposition".into());
        }
        for child in &refinement.children {
            if child.starts_with("legacy_") || !graph.atoms.contains_key(child) {
                return Err("hypothesis: missing or legacy refinement atom".into());
            }
            used.insert(child.clone());
        }
    }
    let expected_requirements = graph
        .atoms
        .iter()
        .filter(|(id, a)| {
            matches!(a.proposition, Proposition::EntryCondition { .. })
                || (matches!(a.proposition, Proposition::Formula { .. }) && !used.contains(*id))
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if graph.requirements != expected_requirements
        || (graph.version < 3 && !graph.requirements.is_empty())
    {
        return Err("hypothesis: noncanonical or missing formal entry obligations".into());
    }
    used.extend(graph.requirements.iter().cloned());
    if used.len() != graph.atoms.len() {
        return Err("hypothesis: unreferenced atom".into());
    }
    Ok(())
}
pub fn digest(graph: &Graph) -> Result<String, String> {
    validate(graph)?;
    Ok(qcore_schemas::sha256_hex(
        &serde_json::to_vec(graph).map_err(|e| e.to_string())?,
    ))
}
pub fn erasure(graph: &Graph) -> Result<Vec<ErasedPremise>, String> {
    validate(graph)?;
    Ok(graph.premises.values().map(|p| p.erasure.clone()).collect())
}
/// Freshly replayed declaration graphs, using the loader's inclusion/inheritance path.
pub fn replay(sources: &BTreeMap<String, String>) -> Result<BTreeMap<String, Graph>, String> {
    crate::contract_loader::declaration_hypotheses(sources)
}
pub fn check_replay(
    sources: &BTreeMap<String, String>,
    expected: &BTreeMap<String, Graph>,
) -> Result<(), String> {
    if replay(sources)? != *expected {
        return Err("hypothesis: retained-text graph replay mismatch".into());
    }
    Ok(())
}
/// Claims are evidence only, including supported/derived claims. No certificate authority is returned.
/// Both expected context and expected graph must be independently held (the
/// latter freshly replayed from retained sources), never copied from the claim.
pub fn compose(
    instance: &Instance,
    expected: &Context,
    expected_graph: &Graph,
    premise: &str,
) -> Result<Disposition, String> {
    let node = instance
        .graph
        .premises
        .get(premise)
        .ok_or("hypothesis: missing premise")?;
    compose_children(instance, expected, expected_graph, &node.children)
}
/// Compose only the proposed conjunction. Its disposition is never transported
/// to the legacy proposition, even when every proposed atom is supported.
pub fn compose_refinement(
    instance: &Instance,
    expected: &Context,
    expected_graph: &Graph,
    name: &str,
) -> Result<Disposition, String> {
    let refinement = instance
        .graph
        .refinements
        .get(name)
        .ok_or("hypothesis: missing proposed decomposition")?;
    compose_children(instance, expected, expected_graph, &refinement.children)
}
fn compose_children(
    instance: &Instance,
    expected: &Context,
    expected_graph: &Graph,
    children: &[String],
) -> Result<Disposition, String> {
    if &instance.context != expected
        || [
            &expected.artifact,
            &expected.property,
            &expected.semantic_profile,
            &expected.scope,
            &expected.bounds,
            &expected.assumptions,
            &expected.source_identity,
            &expected.build_identity,
            &expected.evidence_version,
        ]
        .iter()
        .any(|s| s.is_empty())
    {
        return Err("hypothesis: incomplete or incompatible context".into());
    }
    if digest(&instance.graph)? != instance.graph_sha256
        || instance.graph_sha256 != digest(expected_graph)?
        || instance.graph != *expected_graph
    {
        return Err("hypothesis: stale graph digest".into());
    }
    let mut evidence = BTreeMap::new();
    for e in &instance.evidence {
        let atom = instance
            .graph
            .atoms
            .get(&e.atom)
            .ok_or("hypothesis: evidence for missing atom")?;
        let hash = qcore_schemas::sha256_hex(&serde_json::to_vec(atom).map_err(|e| e.to_string())?);
        if e.context != *expected
            || e.atom_sha256 != hash
            || e.reference.is_empty()
            || evidence.insert(&e.atom, e).is_some()
        {
            return Err("hypothesis: conflicting, stale or incompatible evidence".into());
        }
    }
    use qcore_core::hypothesis as kernel;
    let statuses = children
        .iter()
        .map(|id| {
            (
                true,
                match evidence.get(id).map(|e| e.disposition) {
                    Some(Disposition::Supported) => kernel::Disposition::Supported,
                    Some(Disposition::Refuted) => kernel::Disposition::Refuted,
                    _ => kernel::Disposition::Open,
                },
            )
        })
        .collect::<Vec<_>>();
    Ok(match kernel::conjunction(&statuses) {
        kernel::Disposition::Open => Disposition::Open,
        kernel::Disposition::Supported => Disposition::Supported,
        kernel::Disposition::Refuted => Disposition::Refuted,
    })
}

/// Strict wire loader: rejects duplicate map keys, reordered fields/children, whitespace
/// variants and unknown fields instead of normalizing a noncanonical claim.
pub fn decode(bytes: &[u8]) -> Result<Graph, String> {
    let graph: Graph = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    validate(&graph)?;
    if serde_json::to_vec(&graph).map_err(|e| e.to_string())? != bytes {
        return Err("hypothesis: noncanonical wire encoding".into());
    }
    Ok(graph)
}

/// Compatibility elaboration for already validated premise records (including
/// bounds/member and rule-owned metadata). Keeps their original order exactly.
pub fn legacy_records(records: &[ErasedPremise]) -> Result<Graph, String> {
    let mut graph = Graph {
        version: 1,
        atoms: BTreeMap::new(),
        premises: BTreeMap::new(),
        refinements: BTreeMap::new(),
        requirements: Vec::new(),
    };
    for record in records {
        append_legacy(&mut graph, record.clone())?;
    }
    validate(&graph)?;
    Ok(graph)
}

/// Append one legacy record using disjoint, reserved atom and premise symbols.
pub fn append_legacy(graph: &mut Graph, record: ErasedPremise) -> Result<(), String> {
    let index = graph
        .premises
        .keys()
        .filter(|id| id.starts_with("legacy_"))
        .count();
    let atom_id = format!("legacy_atom_{index:08}");
    let premise_id = format!("legacy_premise_{index:08}");
    if graph.atoms.contains_key(&atom_id) || graph.premises.contains_key(&premise_id) {
        return Err("hypothesis: conflicting generated legacy ID".into());
    }
    graph.atoms.insert(
        atom_id.clone(),
        Atom {
            id: atom_id.clone(),
            proposition: Proposition::Opaque {
                statement: record.statement.clone(),
            },
        },
    );
    graph.premises.insert(
        premise_id.clone(),
        Conjunction {
            id: premise_id,
            children: vec![atom_id],
            erasure: record,
        },
    );
    validate(graph)
}
