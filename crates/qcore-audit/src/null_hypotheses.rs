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

//! Property-specific, dependency-selected null graph metadata. No solver or
//! verdict reads this module. Every original legacy graph node is retained.
use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};
use qcore_schemas::hypothesis::{BindingRequirement, Graph, Refinement};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub struct View {
    graphs: BTreeMap<String, Graph>,
    facts: BTreeMap<String, BTreeSet<String>>,
    requirements: Vec<BindingRequirement>,
}
impl View {
    /// Reconstruct through the bounds loader's exact already-selected texts.
    pub fn new(
        environment: &BoundsContracts,
        sources: &BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let hashes = sources
            .iter()
            .map(|(key, text)| (key.clone(), qcore_schemas::sha256_hex(text.as_bytes())))
            .collect::<BTreeMap<_, _>>();
        if hashes != environment.sources {
            return Err("null hypothesis: wrong property source identities".into());
        }
        let graphs = qcore_contract::bounds_hypotheses::replay(sources)?;
        let requirements = qcore_contract::hypothesis_bindings::requirements(sources, &graphs)?;
        let mut facts = BTreeMap::<String, BTreeSet<String>>::new();
        for (name, contract) in environment.null_declarations() {
            for entry in &contract.clauses {
                let fact = match &entry.clause {
                    BoundsClause::NonnullMember { tag, field } => format!("member:{tag}.{field}"),
                    BoundsClause::NonnullMemberStable { tag, field } => {
                        format!("stable:{tag}.{field}")
                    }
                    BoundsClause::NonnullMemberScope { .. } => name.clone(),
                    _ => continue,
                };
                facts.entry(fact).or_default().insert(name.clone());
            }
        }
        Ok(Self {
            graphs,
            facts,
            requirements,
        })
    }
    /// Attach only through the row's actual member-fact consultation records.
    /// Declaration-order legacy IDs are retargeted by exact erasure identity.
    pub fn attach(
        &self,
        row: &Value,
        graph: &mut Graph,
    ) -> Result<Vec<BindingRequirement>, String> {
        let mut checked = graph.clone();
        let requirements = self.attach_checked(row, &mut checked)?;
        *graph = checked;
        Ok(requirements)
    }

    fn attach_checked(
        &self,
        row: &Value,
        graph: &mut Graph,
    ) -> Result<Vec<BindingRequirement>, String> {
        if row["null"]["property"].as_str() != Some(qcore_c_gateway::null_deref::NULL_PROPERTY) {
            return Err("null hypothesis: wrong or missing row property".into());
        }
        qcore_contract::hypothesis::validate(graph)?;
        let original = graph.clone();
        let mut used = BTreeSet::new();
        let dependencies = row["null"]["contract_dependencies"].as_array();
        for record in row["null"]["premises"].as_array().into_iter().flatten() {
            let name = record["name"]
                .as_str()
                .ok_or("null hypothesis: missing legacy name")?;
            let statement = record["statement"]
                .as_str()
                .ok_or("null hypothesis: missing legacy statement")?;
            for fact in record["through"].as_array().into_iter().flatten() {
                let fact = fact.as_str().ok_or("null hypothesis: nontext dependency")?;
                if !dependencies.is_some_and(|deps| deps.iter().any(|d| d.as_str() == Some(fact))) {
                    return Err(
                        "null hypothesis: through fact absent from actual row dependencies".into(),
                    );
                }
                for declaration in self.facts.get(fact).into_iter().flatten() {
                    let source = self
                        .graphs
                        .get(declaration)
                        .ok_or("null hypothesis: missing selected declaration graph")?;
                    let Some(refinement) = source.refinements.get(name) else {
                        continue;
                    };
                    let target = &source.premises[&refinement.premise];
                    if target.erasure.name != name
                        || target.erasure.statement != statement
                        || target.erasure.origin != "declared"
                    {
                        return Err("null hypothesis: source legacy erasure differs from actual fact premise".into());
                    }
                    let targets = graph
                        .premises
                        .iter()
                        .filter(|(_, p)| p.erasure == target.erasure)
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>();
                    if targets.len() != 1 {
                        return Err("null hypothesis: ambiguous row legacy target".into());
                    }
                    let attached = Refinement {
                        premise: targets[0].clone(),
                        children: refinement.children.clone(),
                    };
                    if graph
                        .refinements
                        .get(name)
                        .is_some_and(|old| old != &attached)
                    {
                        return Err("null hypothesis: conflicting property refinement".into());
                    }
                    for id in &refinement.children {
                        let atom = &source.atoms[id];
                        if graph.atoms.get(id).is_some_and(|old| old != atom) {
                            return Err("null hypothesis: conflicting property atom".into());
                        }
                        graph.atoms.insert(id.clone(), atom.clone());
                    }
                    graph.refinements.insert(name.to_owned(), attached);
                    graph.version = qcore_contract::hypothesis::expected_version(graph);
                    used.insert((declaration.clone(), name.to_owned()));
                }
            }
        }
        // Check exact original node payloads, not merely erasure counts.
        if graph.premises != original.premises
            || original
                .atoms
                .iter()
                .any(|(id, atom)| graph.atoms.get(id) != Some(atom))
        {
            return Err("null hypothesis: publication altered original legacy nodes".into());
        }
        qcore_contract::hypothesis::validate(graph)?;
        Ok(self
            .requirements
            .iter()
            .filter(|r| used.contains(&(r.declaration.clone(), r.premise.clone())))
            .cloned()
            .collect())
    }
}
