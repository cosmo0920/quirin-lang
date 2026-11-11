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

//! Version-2 guarded metadata adoption: exact whole-source replay, no truth rule.
use crate::{
    guarded_decomposition::{self as guarded, Inline},
    hypothesis,
};
use qcore_schemas::{hypothesis::Graph, premise_flow::View};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub version: u32,
    pub module: String,
    pub declaration: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub graph: Graph,
    pub metadata: Inline,
}
/// Archival only: no Deserialize, unchecked constructor, or truth disposition.
/// ```compile_fail
/// let _: qcore_contract::guarded_metadata_bridge::Checked = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug, Serialize)]
pub struct Checked {
    version: u32,
    plan_sha256: String,
    before_sha256: String,
    after_sha256: String,
    graph_sha256: String,
    metadata_sha256: String,
    #[serde(skip)]
    before: BTreeMap<String, String>,
    #[serde(skip)]
    after: BTreeMap<String, String>,
}
fn sha(s: &str) -> String {
    qcore_schemas::sha256_hex(s.as_bytes())
}
fn literal(s: &str) -> Result<String, String> {
    if s.contains(['"', '\\', '\r']) {
        return Err("guarded bridge: unescaped LF source literal required".into());
    }
    Ok(format!("\"{s}\""))
}
/// Deterministic source form. Proposition bytes and dependency order are held
/// independently in the plan, not recovered by deleting arbitrary clauses.
pub fn render(metadata: &Inline) -> Result<String, String> {
    let mut out = String::new();
    for (id, binding) in &metadata.bindings {
        let role = match binding.role {
            guarded::Role::Environment => "environment",
            guarded::Role::Prerequisite => "prerequisite",
            guarded::Role::Operation => "operation",
            guarded::Role::Condition => "condition",
            guarded::Role::Case => "case",
            guarded::Role::Postcondition => "postcondition",
        };
        let deps = binding
            .dependencies
            .iter()
            .map(|d| format!(", {d}"))
            .collect::<String>();
        out.push_str(&format!(
            "    requires resource_guarded_symbol_v1({id}, {role}, {}{deps});\n",
            literal(&binding.proposition)?
        ));
    }
    out.push_str(&format!(
        "    requires resource_guarded_decomposition_v1({}, {});\n",
        metadata.entry,
        literal(&metadata.readable)?
    ));
    Ok(out)
}
pub fn check(
    plan_bytes: &[u8],
    expected_plan_sha256: &str,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Result<Checked, String> {
    if qcore_schemas::sha256_hex(plan_bytes) != expected_plan_sha256 {
        return Err("guarded bridge: independent plan digest mismatch".into());
    }
    let plan: Plan = serde_json::from_slice(plan_bytes).map_err(|e| e.to_string())?;
    if plan.version != 2
        || before.len() != 1
        || after.len() != 1
        || before.keys().ne(after.keys())
        || !before.contains_key(&plan.module)
        || sha(&before[&plan.module]) != plan.before_sha256
        || sha(&after[&plan.module]) != plan.after_sha256
    {
        return Err("guarded bridge: version or exact whole-source inventory mismatch".into());
    }
    let old_graphs = hypothesis::replay(before)?;
    let new_graphs = hypothesis::replay(after)?;
    if old_graphs != new_graphs || old_graphs.get(&plan.declaration) != Some(&plan.graph) {
        return Err("guarded bridge: complete original graph mismatch".into());
    }
    let parsed = qcore_frontend::frontend::parse_source_program(&before[&plan.module])
        .map_err(|e| e.to_string())?;
    let mut anchor = None;
    for decl in &parsed.declarations {
        if guarded::declaration(&decl.value, &old_graphs[&decl.value.name])?.is_some() {
            return Err("guarded bridge: preexisting guarded metadata refused".into());
        }
        if decl.value.name == plan.declaration {
            let last = decl
                .value
                .function_contract
                .as_ref()
                .and_then(|c| c.clauses.last())
                .ok_or("guarded bridge: empty target contract")?;
            let end = last.span.end;
            if !before[&plan.module][end..].starts_with('\n') || anchor.replace(end + 1).is_some() {
                return Err("guarded bridge: standalone LF target required".into());
            }
        }
    }
    let at = anchor.ok_or("guarded bridge: target absent")?;
    let addition = render(&plan.metadata)?;
    let old = &before[&plan.module];
    let generated = format!("{}{}{}", &old[..at], addition, &old[at..]);
    if generated != after[&plan.module] {
        return Err("guarded bridge: noncanonical addition or unrelated source edit".into());
    }
    guarded::replay_inline(
        View::Resource,
        after,
        &plan.module,
        &plan.declaration,
        &plan.graph,
        &plan.metadata,
    )?;
    let old_contracts = crate::contract_loader::load_resource_contract_sources(before)?;
    let mut new_contracts = crate::contract_loader::load_resource_contract_sources(after)?;
    // Exact reconstructive transport of the independently held original text.
    // Every other interpreted field must agree; no JSON field is deleted.
    new_contracts.set_source_assumptions(before.clone());
    if new_contracts != old_contracts
        || crate::contract_loader::declaration_premises(before)?
            != crate::contract_loader::declaration_premises(after)?
        || crate::contract_loader::declared_abi_signatures(before)?
            != crate::contract_loader::declared_abi_signatures(after)?
    {
        return Err("guarded bridge: original interpreted output or erasure differs".into());
    }
    Ok(Checked {
        version: 2,
        plan_sha256: expected_plan_sha256.into(),
        before_sha256: plan.before_sha256,
        after_sha256: plan.after_sha256,
        graph_sha256: hypothesis::digest(&plan.graph)?,
        metadata_sha256: qcore_schemas::sha256_hex(
            &serde_json::to_vec(&plan.metadata).map_err(|e| e.to_string())?,
        ),
        before: before.clone(),
        after: after.clone(),
    })
}
impl Checked {
    /// Whole sources only. Changed, missing, extra or foreign modules refuse.
    pub fn recover_before(
        &self,
        presented: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>, String> {
        if presented != &self.after {
            return Err("guarded bridge: changed complete presented source map".into());
        }
        Ok(self.before.clone())
    }
}
