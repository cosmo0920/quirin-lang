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

//! Exact entry roles for the existing bounded Boolean source model.
//! Caller conditions remain assumptions. Classification proves no C fact and
//! never creates hypothesis evidence or an ordinary call summary.
use qcore_schemas::quant_condition::{Definition, Selector};
use std::collections::BTreeMap;
pub const LIVE: &str = "#live(#env: environment, #modeled: predicate<environment,object>, #live: predicate<environment,object>) = forall (#o: object) { (#modeled(#env,#o) => #live(#env,#o)) }";
pub const EXCLUSIVE: &str = "#exclusive(#env: environment, #caller: actor, #modeled: predicate<environment,object>, #during: predicate<environment,event>, #access: predicate<environment,event,object>, #actor: function<event,actor>) = forall (#e: event) { forall (#o: object) { ((#modeled(#env,#o) && (#during(#env,#e) && #access(#env,#e,#o))) => #actor(#e) == #caller) } }";
pub fn domain(value: &str) -> String {
    format!("#domain(#{value}: integer) = ((#{value} == 0) || (#{value} == 1))")
}
fn same(candidate: &Definition, source: &str) -> Result<bool, String> {
    let expected = qcore_frontend::quant_condition::parse(Selector::Entry, source)?;
    Ok(
        candidate.proposition.parameters == expected.proposition.parameters
            && candidate.proposition.formula == expected.proposition.formula,
    )
}
pub fn entry_requirement(condition: &Definition, unit: bool) -> Result<&'static str, String> {
    qcore_frontend::quant_condition::validate(condition)?;
    if condition.selector != Selector::Entry {
        return Err("Boolean source condition is not an entry prerequisite".into());
    }
    if same(condition, LIVE)? {
        return Ok(if unit {
            "object_is_live"
        } else {
            "chunk_is_live"
        });
    }
    if same(condition, EXCLUSIVE)? {
        return Ok(if unit {
            "exclusive_object_access"
        } else {
            "exclusive_chunk_access"
        });
    }
    for (value, role) in if unit {
        vec![("old_field", "field_is_boolean")]
    } else {
        vec![
            ("old_busy", "busy_is_boolean"),
            ("old_lock", "lock_is_boolean"),
        ]
    } {
        if same(condition, &domain(value))? {
            return Ok(role);
        }
    }
    Err("unsupported Boolean source entry condition or binding signature".into())
}
/// Record assumed predicates with exact definition identities, never support them.
pub fn entry_assumptions(
    source: &str,
    function: &str,
    unit: bool,
) -> Result<BTreeMap<String, bool>, String> {
    let parsed =
        qcore_frontend::frontend::parse_source_program(source).map_err(|e| e.to_string())?;
    let selected = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == function)
        .collect::<Vec<_>>();
    let [declaration] = selected.as_slice() else {
        return Err("expected one Boolean source declaration".into());
    };
    let mut out = BTreeMap::new();
    for condition in crate::quant_condition::declaration(&declaration.value)? {
        if condition.selector != Selector::Entry {
            continue;
        }
        entry_requirement(&condition, unit)?;
        let digest =
            qcore_schemas::sha256_hex(&serde_json::to_vec(&condition).map_err(|e| e.to_string())?);
        out.insert(
            format!(
                "quant-condition-entry:{}:{digest}",
                condition.proposition.id
            ),
            true,
        );
    }
    Ok(out)
}
