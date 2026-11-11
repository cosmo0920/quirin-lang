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

//! Source-replayed symbolic metadata. No allocation, release or proof mutation.
pub use qcore_frontend::contract_lifecycle::{Entry, Operation, Target, is_clause};
use qcore_frontend::frontend::{SourceContractClauseKind, SourceDeclaration, parse_source_program};
use qcore_schemas::{hypothesis::Graph, premise_flow::View};
use std::collections::BTreeMap;

/// Validates metadata only; authoritative vocabulary loaders call this after
/// constructing their original graph. Hypothesis lifecycle targets do not mint
/// atoms: the proposition must separately be declared in that same graph.
pub fn declaration(
    decl: &SourceDeclaration,
    graph: &Graph,
) -> Result<BTreeMap<String, Entry>, String> {
    let mut entries = BTreeMap::new();
    for clause in decl.function_contract.iter().flat_map(|c| &c.clauses) {
        let Some((id, entry)) =
            qcore_frontend::contract_lifecycle::parse_clause(&clause.value.expression_source)?
        else {
            continue;
        };
        if clause.value.kind != SourceContractClauseKind::Requires {
            return Err("lifecycle: only requires admits symbolic metadata".into());
        }
        if let Target::Hypothesis(atom) = &entry.target
            && !graph.atoms.contains_key(atom)
        {
            return Err("lifecycle: hypothesis target is not a declared graph atom".into());
        }
        if entries.insert(id, entry).is_some() {
            return Err("lifecycle: duplicate entry ID".into());
        }
        if entries.len() > 256 {
            return Err("lifecycle: entry limit".into());
        }
    }
    Ok(entries)
}

/// An inspection result, deliberately outside the wire schema and evidence API.
/// Source identity is provenance, not proof or an independently checked receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub module: String,
    pub source_sha256: String,
    pub declaration: String,
    pub entries: BTreeMap<String, Entry>,
}

/// Runs the selected resource/bounds loader or existing declared-model graph
/// replay before retaining metadata. The model view claims no C-body semantics.
/// No metadata-only fallback can accept a malformed interpreted declaration.
pub fn replay(
    view: View,
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Document>, String> {
    let graphs = match view {
        View::Resource => crate::hypothesis::replay(sources)?,
        View::Bounds => crate::bounds_hypotheses::replay(sources)?,
        View::Model => crate::bounds_hypotheses::replay_model(sources)?,
    };
    let mut documents = BTreeMap::new();
    for (module, source) in sources {
        for decl in parse_source_program(source)
            .map_err(|e| e.to_string())?
            .declarations
        {
            if !decl
                .value
                .function_contract
                .iter()
                .flat_map(|c| &c.clauses)
                .any(|c| is_clause(&c.value.expression_source))
            {
                continue;
            }
            let graph = graphs
                .get(&decl.value.name)
                .ok_or("lifecycle: missing property-scoped declaration graph")?;
            let document = Document {
                module: module.clone(),
                source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
                declaration: decl.value.name.clone(),
                entries: declaration(&decl.value, graph)?,
            };
            if documents.insert(decl.value.name, document).is_some() {
                return Err("lifecycle: duplicate declaration".into());
            }
        }
    }
    Ok(documents)
}
