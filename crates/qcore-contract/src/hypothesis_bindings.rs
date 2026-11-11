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

//! Explicit content-addressed reference integrity. No filesystem lookup, typed
//! predicate, source correspondence, premise truth or certificate authority.
use crate::hypothesis;
use qcore_schemas::hypothesis::{BindingManifest, BindingRequirement, Graph};
use std::collections::BTreeMap;

const MARKER: &str = "// qcore-premise-binding-manifest-v1 ";
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.starts_with('/')
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}
/// Declared requirements only. They remain unresolved until all caller-supplied
/// expected data, manifest bytes and artifact bytes pass `resolve`.
pub fn requirements(
    sources: &BTreeMap<String, String>,
    graphs: &BTreeMap<String, Graph>,
) -> Result<Vec<BindingRequirement>, String> {
    let mut out = BTreeMap::new();
    for (module, source) in sources {
        for line in source.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix(MARKER) else {
                if line.starts_with("// qcore-premise-binding-manifest-") {
                    return Err("binding: unsupported or malformed manifest marker".into());
                }
                continue;
            };
            let fields = rest.split_whitespace().collect::<Vec<_>>();
            if fields.len() != 3 || !sha(fields[2]) {
                return Err("binding: malformed manifest pin".into());
            }
            let graph = graphs
                .get(fields[0])
                .ok_or("binding: missing declaration graph")?;
            if !graph.refinements.contains_key(fields[1]) {
                return Err("binding: missing proposed decomposition".into());
            }
            let requirement = BindingRequirement {
                source_module: module.clone(),
                declaration: fields[0].into(),
                premise: fields[1].into(),
                manifest_sha256: fields[2].into(),
                graph_sha256: hypothesis::digest(graph)?,
            };
            if out
                .insert((fields[0].to_owned(), fields[1].to_owned()), requirement)
                .is_some()
            {
                return Err("binding: duplicate manifest pin".into());
            }
        }
    }
    Ok(out.into_values().collect())
}

/// All expectations and artifact bytes are independently held by the caller.
/// Matching model/audit bytes establishes references, never the source binding
/// or satisfaction of any formal antecedent. No evidence disposition is minted.
pub fn resolve(
    sources: &BTreeMap<String, String>,
    expected_graphs: &BTreeMap<String, Graph>,
    expected_manifests: &BTreeMap<String, BindingManifest>,
    presented_manifests: &BTreeMap<String, Vec<u8>>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, BindingManifest>, String> {
    hypothesis::check_replay(sources, expected_graphs)?;
    resolve_replayed(
        sources,
        expected_graphs,
        expected_manifests,
        presented_manifests,
        artifacts,
    )
}

/// Explicit bounds/null view replay. Never merges resource names or Contexts.
pub fn resolve_bounds(
    sources: &BTreeMap<String, String>,
    expected_graphs: &BTreeMap<String, Graph>,
    expected_manifests: &BTreeMap<String, BindingManifest>,
    presented_manifests: &BTreeMap<String, Vec<u8>>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, BindingManifest>, String> {
    crate::bounds_hypotheses::check_replay(sources, expected_graphs)?;
    resolve_replayed(
        sources,
        expected_graphs,
        expected_manifests,
        presented_manifests,
        artifacts,
    )
}

/// Explicit model-only view. Reference integrity does not supply a C ABI or body.
pub fn resolve_model(
    sources: &BTreeMap<String, String>,
    expected_graphs: &BTreeMap<String, Graph>,
    expected_manifests: &BTreeMap<String, BindingManifest>,
    presented_manifests: &BTreeMap<String, Vec<u8>>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, BindingManifest>, String> {
    if crate::bounds_hypotheses::replay_model(sources)? != *expected_graphs {
        return Err(
            "model binding: independently held graph differs from actual model source".into(),
        );
    }
    resolve_replayed(
        sources,
        expected_graphs,
        expected_manifests,
        presented_manifests,
        artifacts,
    )
}

fn resolve_replayed(
    sources: &BTreeMap<String, String>,
    expected_graphs: &BTreeMap<String, Graph>,
    expected_manifests: &BTreeMap<String, BindingManifest>,
    presented_manifests: &BTreeMap<String, Vec<u8>>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, BindingManifest>, String> {
    let requirements = requirements(sources, expected_graphs)?;
    let required_domain = requirements
        .iter()
        .map(|r| (r.declaration.clone(), r.premise.clone()))
        .collect::<std::collections::BTreeSet<_>>();
    let graph_domain = expected_graphs
        .iter()
        .flat_map(|(declaration, graph)| {
            graph
                .refinements
                .keys()
                .map(move |name| (declaration.clone(), name.clone()))
        })
        .collect::<std::collections::BTreeSet<_>>();
    let manifest_domain = requirements
        .iter()
        .map(|r| r.manifest_sha256.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if required_domain != graph_domain
        || expected_manifests
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != manifest_domain
        || presented_manifests
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != manifest_domain
    {
        return Err("binding: incomplete or excess explicit binding domain".into());
    }
    let mut resolved = BTreeMap::new();
    for required in requirements {
        let bytes = presented_manifests
            .get(&required.manifest_sha256)
            .ok_or("binding: required explicit manifest input missing")?;
        if qcore_schemas::sha256_hex(bytes) != required.manifest_sha256 {
            return Err("binding: manifest content digest mismatch".into());
        }
        let manifest: BindingManifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if serde_json::to_vec(&manifest).map_err(|e| e.to_string())? != *bytes
            || expected_manifests.get(&required.manifest_sha256) != Some(&manifest)
        {
            return Err("binding: noncanonical or independently unexpected manifest".into());
        }
        let graph = &expected_graphs[&required.declaration];
        let refinement = &graph.refinements[&required.premise];
        if manifest.version != 1
            || manifest.declaration != required.declaration
            || manifest.premise != required.premise
            || manifest.graph_sha256 != required.graph_sha256
            || manifest.source_binding != "open"
            || manifest.environment.id.is_empty()
            || manifest.environment.variables.is_empty()
            || manifest
                .environment
                .variables
                .iter()
                .any(|(k, v)| k.is_empty() || v.is_empty())
            || manifest.atoms.keys().cloned().collect::<Vec<_>>() != refinement.children
        {
            return Err("binding: incompatible manifest domain/environment/source status".into());
        }
        for (id, binding) in &manifest.atoms {
            let atom = &graph.atoms[id];
            if binding.atom_sha256
                != qcore_schemas::sha256_hex(&serde_json::to_vec(atom).map_err(|e| e.to_string())?)
                || binding.environment != manifest.environment.id
                || binding.proposition.is_empty()
                || binding.quantifiers.is_empty()
                || binding.quantifiers.iter().any(String::is_empty)
                || binding.prerequisites != vec![manifest.environment.id.clone()]
                || binding.conclusion.is_empty()
            {
                return Err(
                    "binding: stale atom or explicit proposition/environment/prerequisite mismatch"
                        .into(),
                );
            }
            for reference in [&binding.model, &binding.audit] {
                if !path(&reference.path)
                    || !sha(&reference.sha256)
                    || artifacts
                        .get(&reference.path)
                        .map(|bytes| qcore_schemas::sha256_hex(bytes))
                        != Some(reference.sha256.clone())
                {
                    return Err(
                        "binding: missing or mismatched independently supplied artifact".into(),
                    );
                }
            }
        }
        resolved.insert(required.manifest_sha256, manifest);
    }
    Ok(resolved)
}
