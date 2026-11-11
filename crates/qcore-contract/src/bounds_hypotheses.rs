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

//! Fresh property-specific bounds hypothesis replay. No solver consumes graphs.
//! Same-name metadata retains every original interpreted bounds/null clause.
use crate::bounds_contract::{BoundsClause, load_bounds_contract_sources};
use crate::contract_loader::{DeclaredPremise, PremiseOrigin};
use qcore_frontend::frontend::{DeclarationKind, SourceModuleMode, parse_source_program};
use qcore_schemas::hypothesis::Graph;
use std::collections::BTreeMap;

/// Full bounds parsing first: wrong vocabulary or ordinary malformed clauses
/// never become metadata-only successes. Original interpreted clauses remain
/// the authoritative facts, modes, preconditions and canonical semantic hash.
pub fn replay(sources: &BTreeMap<String, String>) -> Result<BTreeMap<String, Graph>, String> {
    if sources.is_empty() {
        return Ok(BTreeMap::new());
    }
    let loaded = load_bounds_contract_sources(sources)?;
    let mut out = BTreeMap::new();
    for (module, source) in sources {
        let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
        if parsed.mode != SourceModuleMode::Contract {
            return Err("bounds hypothesis: wrong source mode".into());
        }
        for declaration in parsed.declarations {
            let decl = declaration.value;
            let bounds = loaded
                .functions
                .get(&decl.name)
                .ok_or("bounds hypothesis: missing original declaration")?;
            if bounds.module != *module {
                return Err("bounds hypothesis: source/declaration mismatch".into());
            }
            let legacy = bounds
                .clauses
                .iter()
                .filter_map(|entry| match &entry.clause {
                    BoundsClause::Premise { name, statement } => Some(DeclaredPremise {
                        name: name.clone(),
                        statement: statement.clone(),
                        origin: PremiseOrigin::Declared,
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let graph = crate::hypothesis::declaration(&decl, &legacy)?;
            require_neutral_erasure(&graph, &legacy)?;
            if out.insert(decl.name, graph).is_some() {
                return Err("bounds hypothesis: duplicate property-scoped declaration".into());
            }
        }
    }
    Ok(out)
}

/// Bounds-v1 admits only same-name proposed attachments: v1 `premise_all`
/// additions would introduce a new premise and must remain refused here.
pub(crate) fn require_neutral_erasure(
    graph: &Graph,
    legacy: &[DeclaredPremise],
) -> Result<(), String> {
    let mut erased = crate::hypothesis::erasure(graph)?;
    let mut expected = legacy
        .iter()
        .map(|p| qcore_schemas::hypothesis::ErasedPremise {
            name: p.name.clone(),
            statement: p.statement.clone(),
            origin: p.origin.as_str().into(),
        })
        .collect::<Vec<_>>();
    let order = |p: &qcore_schemas::hypothesis::ErasedPremise| {
        (p.name.clone(), p.statement.clone(), p.origin.clone())
    };
    erased.sort_by_key(order);
    expected.sort_by_key(order);
    if erased != expected {
        return Err("bounds hypothesis: metadata cannot add or alter legacy premises".into());
    }
    Ok(())
}

pub fn check_replay(
    sources: &BTreeMap<String, String>,
    independently_expected: &BTreeMap<String, Graph>,
) -> Result<(), String> {
    if replay(sources)? != *independently_expected {
        return Err(
            "bounds hypothesis: independently held graph differs from actual property source"
                .into(),
        );
    }
    Ok(())
}

/// Model-only declarations stay outside both resource and bounds source views.
/// This validates the actual contract-mode graph without claiming a C ABI/body.
pub fn replay_model(sources: &BTreeMap<String, String>) -> Result<BTreeMap<String, Graph>, String> {
    let mut out = BTreeMap::new();
    for source in sources.values() {
        let named = crate::contract_loader::module_premises(source)?;
        let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
        if parsed.mode != SourceModuleMode::Contract
            || !parsed
                .declarations
                .iter()
                .any(|d| d.value.kind == DeclarationKind::ResourceType)
        {
            return Err(
                "model hypothesis: expected a declared resource-type model source view".into(),
            );
        }
        for declaration in parsed.declarations {
            let decl = declaration.value;
            if decl.kind != DeclarationKind::ExternFunction {
                continue;
            }
            let legacy = named.get(&decl.name).cloned().unwrap_or_default();
            let graph = crate::hypothesis::declaration(&decl, &legacy)?;
            require_neutral_erasure(&graph, &legacy)?;
            if out.insert(decl.name, graph).is_some() {
                return Err("model hypothesis: duplicate declaration".into());
            }
        }
    }
    Ok(out)
}
