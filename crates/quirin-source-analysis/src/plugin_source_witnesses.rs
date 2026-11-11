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

//! Executable, finite plugin-loader witnesses. The compiler, host loader, real
//! header interpretation and maintained C/model correspondence are explicit
//! prerequisites, not attestation or a proof about arbitrary deployed plugins.
//! Native JSON is untrusted. Admission reruns the embedded producer against
//! separately held pinned sources, then checks the whole selected trace domain.
use qcore_schemas::{hypothesis::Context, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const INVENTORY: &[u8] =
    include_bytes!("../../../docs/reports/plugin-premise-source-witnesses/input-inventory.json");
const FORMAL_MODEL: &[u8] = include_bytes!("../../../proof/QCore/PluginSourceWitnesses.lean");
const FORMAL_AUDIT: &[u8] = include_bytes!("../../../proof/AuditPluginSourceWitnesses.lean");
const CHECKER: &[u8] = include_bytes!("plugin_source_witnesses.rs");
const PRODUCER: &str = include_str!("plugin_source_witnesses/observe.py");
const MODEL: &str = include_str!("plugin_source_witnesses/trace_model.json");
pub const BOUNDED_ATOM: &str = "rk_duplicate_fixture_no_loader_calls_v1";
pub const COMPLETION_ATOM: &str = "rk_duplicate_fixture_complete_v1";
pub const GO_COMPLETE_ATOM: &str = "go_creator_fixture_complete_v1";
pub const GO_IMPLIES_ATOM: &str = "go_creator_fixture_implies_registration_v1";
pub const PROFILE: &str = "plugin-loader-fixture-v1";
const BOUNDS: &str = "20-cases/single-thread/explicit-stubs";
const ASSUMPTIONS: &str = "local-clang-posix-loader/real-pinned-headers/explicit-allocation-list-log-stubs/no-deployed-plugin-claim";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub module: String,
    pub function: String,
    pub start: usize,
    pub end: usize,
    pub body: String,
    pub sha256: String,
}
/// Presentation only; deserialization supplies no checked receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub version: u32,
    pub inventory_sha256: String,
    pub compiler: String,
    pub compiler_sha256: String,
    pub target: String,
    pub projections: Vec<Projection>,
    pub asts: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, String>,
    pub symbols: BTreeMap<String, String>,
    pub layouts: BTreeMap<String, Vec<String>>,
    pub header_inputs: BTreeMap<String, String>,
    pub traces: BTreeMap<String, Vec<String>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuardFact {
    condition_calls: Vec<String>,
    condition_members: Vec<String>,
    condition_references: Vec<String>,
    returns_in_guarded_subtree: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceFacts {
    function: String,
    calls: Vec<String>,
    guards: Vec<GuardFact>,
    assigned_members: Vec<String>,
    ignored_top_level_calls: Vec<String>,
    /// Exact main-source AST range, independently matched to the projection.
    start: usize,
    end: usize,
}
/// Not deserializable and no public constructor. It describes exactly twenty
/// selected invocations, never the old seven atoms or either universal envelope.
#[derive(Debug, Serialize)]
pub struct CheckedWitness {
    context: Context,
    observation_sha256: String,
    model_sha256: String,
    producer_sha256: String,
    source_facts: BTreeMap<String, SourceFacts>,
    cases: BTreeMap<String, Vec<String>>,
    remaining_obligations: Vec<String>,
}
fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(sha256_hex(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
fn children(v: &Value) -> impl Iterator<Item = &Value> {
    v.get("inner")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
fn kind(v: &Value) -> &str {
    v.get("kind").and_then(Value::as_str).unwrap_or("")
}
fn collect<'a>(v: &'a Value, k: &str, out: &mut Vec<&'a Value>) {
    if kind(v) == k {
        out.push(v)
    }
    for c in children(v) {
        collect(c, k, out)
    }
}
fn nodes<'a>(v: &'a Value, k: &str) -> Vec<&'a Value> {
    let mut out = vec![];
    collect(v, k, &mut out);
    out
}
fn callee(v: &Value) -> Option<String> {
    let first = children(v).next()?;
    nodes(first, "DeclRefExpr")
        .first()?
        .get("referencedDecl")?
        .get("name")?
        .as_str()
        .map(str::to_owned)
}
fn calls(v: &Value) -> Vec<String> {
    nodes(v, "CallExpr")
        .into_iter()
        .filter_map(callee)
        .collect()
}
fn members(v: &Value) -> Vec<String> {
    nodes(v, "MemberExpr")
        .into_iter()
        .filter_map(|n| n.get("name")?.as_str().map(str::to_owned))
        .collect()
}
fn offset(v: &Value, edge: &str) -> Result<usize, String> {
    v.get("range")
        .and_then(|x| x.get(edge))
        .and_then(|x| x.get("offset"))
        .and_then(Value::as_u64)
        .and_then(|x| usize::try_from(x).ok())
        .ok_or_else(|| "original main-source AST range missing".into())
}
fn checked_projection_prefix(
    projection: &Projection,
    start: usize,
    end: usize,
) -> Result<(), String> {
    let prefix = start
        .checked_sub(projection.start)
        .and_then(|n| projection.body.get(..n))
        .ok_or("typed prefix outside source or UTF-8 boundary")?;
    if start > projection.end || start > end || end != projection.end || !prefix.trim().is_empty() {
        return Err("typed/source projection boundary mismatch".into());
    }
    Ok(())
}
fn source_facts(observation: &Observation) -> Result<BTreeMap<String, SourceFacts>, String> {
    let inventory: Value = serde_json::from_slice(INVENTORY).map_err(|e| e.to_string())?;
    let names: Vec<String> =
        serde_json::from_value(inventory["source_functions"].clone()).map_err(|e| e.to_string())?;
    if observation
        .asts
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        != names.iter().cloned().collect()
        || observation.projections.len() != names.len()
    {
        return Err("incomplete selected source domain".into());
    }
    let mut facts = BTreeMap::new();
    for projection in &observation.projections {
        let source = inventory["source_files"]
            .as_array()
            .ok_or("source inventory")?
            .iter()
            .find(|v| v["path"].as_str() == Some(&projection.module))
            .ok_or("unselected source module")?;
        let raw = fs::read(source["actual_path"].as_str().ok_or("source path")?)
            .map_err(|e| e.to_string())?;
        if sha256_hex(&raw) != source["sha256"].as_str().unwrap_or("")
            || raw.get(projection.start..projection.end) != Some(projection.body.as_bytes())
            || sha256_hex(projection.body.as_bytes()) != projection.sha256
        {
            return Err("source projection changed or range forged".into());
        }
        let text = observation
            .asts
            .get(&projection.function)
            .ok_or("missing original typed AST")?;
        let values: Vec<Value> = serde_json::Deserializer::from_str(text)
            .into_iter::<Value>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let definitions: Vec<_> = values
            .iter()
            .filter(|v| {
                kind(v) == "FunctionDecl"
                    && v["name"].as_str() == Some(&projection.function)
                    && children(v).any(|c| kind(c) == "CompoundStmt")
            })
            .collect();
        if definitions.len() != 1 {
            return Err("missing/ambiguous original typed definition".into());
        }
        let def = definitions[0];
        let start = offset(def, "begin")?;
        let end = offset(def, "end")?
            .checked_add(1)
            .ok_or("typed range end overflow")?;
        // Extraction may include initial whitespace; the AST must delimit the
        // complete same body, with no token before/after hidden by the producer.
        checked_projection_prefix(projection, start, end)?;
        let compound = children(def)
            .find(|c| kind(c) == "CompoundStmt")
            .ok_or("body missing")?;
        let guards = nodes(compound, "IfStmt")
            .into_iter()
            .map(|v| {
                let c = children(v).next().ok_or("missing if condition")?;
                Ok(GuardFact {
                    condition_calls: calls(c),
                    condition_members: members(c),
                    condition_references: nodes(c, "DeclRefExpr")
                        .into_iter()
                        .filter_map(|r| {
                            r.get("referencedDecl")?
                                .get("name")?
                                .as_str()
                                .map(str::to_owned)
                        })
                        .collect(),
                    returns_in_guarded_subtree: nodes(v, "ReturnStmt").len(),
                })
            })
            .collect::<Result<_, String>>()?;
        let assigned_members = nodes(compound, "BinaryOperator")
            .into_iter()
            .filter(|v| v["opcode"] == "=")
            .filter_map(|v| children(v).next())
            .flat_map(members)
            .collect();
        let ignored_top_level_calls = children(compound)
            .filter(|v| kind(v) == "CallExpr")
            .filter_map(callee)
            .collect();
        let f = SourceFacts {
            function: projection.function.clone(),
            calls: calls(compound),
            guards,
            assigned_members,
            ignored_top_level_calls,
            start,
            end,
        };
        if facts.insert(projection.function.clone(), f).is_some() {
            return Err("duplicate source definition".into());
        }
    }
    let require_calls = |function: &str, required: &[&str]| -> Result<(), String> {
        let f = facts.get(function).ok_or("missing source facts")?;
        if required.iter().any(|x| !f.calls.iter().any(|c| c == x)) {
            return Err(format!("missing actual source call in {function}"));
        }
        Ok(())
    };
    require_calls(
        "rd_kafka_plugin_new",
        &[
            "rd_list_find",
            "rd_dl_open",
            "rd_dl_sym",
            "rd_dl_close",
            "rd_list_add",
        ],
    )?;
    require_calls(
        "flb_plugin_load",
        &[
            "get_handle",
            "path_to_plugin_name",
            "load_symbol",
            "memcpy",
            "mk_list_add",
        ],
    )?;
    require_calls("proxy_go_output_register", &["flb_plugin_proxy_symbol"])?;
    if !facts["flb_plugin_proxy_create"]
        .ignored_top_level_calls
        .iter()
        .any(|x| x == "flb_plugin_proxy_register")
    {
        return Err("actual ignored-register-return source fact absent".into());
    }
    if !facts["rd_kafka_plugin_new"].guards.iter().any(|g| {
        g.condition_calls.iter().any(|c| c == "rd_list_find") && g.returns_in_guarded_subtree > 0
    }) {
        return Err("duplicate early-return source guard missing".into());
    }
    if !facts["proxy_go_output_register"].guards.iter().any(|g| {
        g.condition_members.iter().any(|m| m == "cb_init") && g.returns_in_guarded_subtree > 0
    }) {
        return Err("required init-symbol return guard absent".into());
    }
    if !facts["flb_plugin_proxy_register"].guards.iter().any(|g| {
        g.condition_references.iter().any(|r| r == "cb_register")
            && g.returns_in_guarded_subtree > 0
    }) {
        return Err("required registration-symbol return guard absent".into());
    }
    for (function, required) in [
        (
            "rd_kafka_plugin_new",
            vec!["rkplug_path", "rkplug_handle", "rkplug_opaque"],
        ),
        (
            "proxy_go_output_register",
            vec![
                "cb_init",
                "cb_flush",
                "cb_flush_ctx",
                "cb_exit",
                "cb_exit_ctx",
                "name",
                "data",
            ],
        ),
    ] {
        if required
            .iter()
            .any(|member| !facts[function].assigned_members.iter().any(|m| m == member))
        {
            return Err(format!("actual typed field assignment absent: {function}"));
        }
    }
    Ok(facts)
}
fn model() -> Result<BTreeMap<String, Vec<String>>, String> {
    serde_json::from_str(MODEL).map_err(|e| e.to_string())
}
fn exact_layout_domain(
    layouts: &BTreeMap<String, Vec<String>>,
    traces: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    if layouts.keys().ne(traces.keys()) {
        return Err("native ABI-layout domain differs from complete selected trace domain".into());
    }
    Ok(())
}
fn validate(observation: &Observation) -> Result<BTreeMap<String, SourceFacts>, String> {
    if observation.version != 1
        || observation.inventory_sha256 != sha256_hex(INVENTORY)
        || observation.target.is_empty()
        || !observation.compiler.contains("clang version")
    {
        return Err("source/build inventory mismatch".into());
    }
    if observation.traces != model()? {
        return Err("incomplete, truncated, reordered or changed selected trace".into());
    }
    if observation.artifacts.is_empty() || observation.symbols.len() != 13 {
        return Err("native artifact inventory incomplete".into());
    }
    for (name, required) in [
        ("rk_ok.so", "conf_init"),
        ("go_ok.so", "FLBPluginInit"),
        ("flb-in_probe.so", "in_probe_plugin"),
    ] {
        if !observation.symbols.get(name).is_some_and(|s| {
            s.lines().any(|line| {
                line.split_whitespace().last() == Some(required) && !line.contains(" UND ")
            })
        }) {
            return Err("actual ELF symbol missing".into());
        }
    }
    exact_layout_domain(&observation.layouts, &observation.traces)?;
    if observation.layouts.len() != 20
        || observation.layouts.iter().any(|(case, layout)| {
            if case.ends_with("open_fail") {
                !layout.is_empty()
            } else {
                layout.len() != 1 || !layout[0].starts_with("LAYOUT:")
            }
        })
    {
        return Err("native cross-artifact layout check incomplete".into());
    }
    if observation.header_inputs.is_empty() {
        return Err("missing real dependency bytes".into());
    }
    for (path, hash) in &observation.header_inputs {
        if sha256_hex(&fs::read(path).map_err(|e| e.to_string())?) != *hash {
            return Err("actual dependency bytes changed".into());
        }
    }
    source_facts(observation)
}
/// Exercise refusal against an actual observation without manufacturing a
/// checked receipt. Every mutation must fail before launching another producer.
pub fn refusal_controls(
    observation: &Observation,
    directory: &Path,
) -> Result<Vec<String>, String> {
    validate(observation)?;
    let held = context(observation)?;
    let mut passed = vec![];
    let mut refuse = |name: String, changed: Observation, ctx: Context| -> Result<(), String> {
        let scratch = directory.join(&name);
        if replay(&changed, &ctx, &scratch).is_ok() || scratch.exists() {
            return Err(format!(
                "refusal control unexpectedly admitted/launched: {name}"
            ));
        }
        passed.push(name);
        Ok(())
    };
    for case in observation.traces.keys() {
        let mut missing = observation.clone();
        missing.traces.remove(case);
        refuse(format!("missing-case-{case}"), missing, held.clone())?;
        let mut truncated = observation.clone();
        truncated.traces.get_mut(case).unwrap().pop();
        refuse(format!("truncated-{case}"), truncated, held.clone())?;
        let mut order = observation.clone();
        order.traces.get_mut(case).unwrap().swap(0, 1);
        refuse(format!("misordered-{case}"), order, held.clone())?;
    }
    let mut changed = observation.clone();
    changed.projections[0].body.push(' ');
    refuse("changed-source-projection".into(), changed, held.clone())?;
    let mut changed = observation.clone();
    changed
        .asts
        .insert("rd_kafka_plugin_new".into(), "{}".into());
    refuse("forged-original-ast".into(), changed, held.clone())?;
    for (name, edge, value) in [
        (
            "out-of-range-ast",
            "begin",
            serde_json::json!(usize::MAX - 1),
        ),
        ("overflow-ast-end", "end", serde_json::json!(u64::MAX)),
    ] {
        let mut changed = observation.clone();
        let mut values = serde_json::Deserializer::from_str(&changed.asts["rd_kafka_plugin_new"])
            .into_iter::<Value>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let def = values
            .iter_mut()
            .find(|v| {
                v["name"] == "rd_kafka_plugin_new" && children(v).any(|c| kind(c) == "CompoundStmt")
            })
            .ok_or("control original AST missing")?;
        def["range"][edge]["offset"] = value;
        changed.asts.insert(
            "rd_kafka_plugin_new".into(),
            values
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        );
        refuse(name.into(), changed, held.clone())?;
    }
    let mut unicode = observation.clone();
    let selected = unicode.projections[0].function.clone();
    unicode.projections[0].body.insert(0, 'é');
    unicode.projections[0].end += 2;
    unicode.projections[0].sha256 = sha256_hex(unicode.projections[0].body.as_bytes());
    let mut values = serde_json::Deserializer::from_str(&unicode.asts[&selected])
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let def = values
        .iter_mut()
        .find(|v| v["name"] == selected && children(v).any(|c| kind(c) == "CompoundStmt"))
        .ok_or("control Unicode AST missing")?;
    def["range"]["begin"]["offset"] = serde_json::json!(unicode.projections[0].start + 1);
    unicode.asts.insert(
        selected,
        values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    refuse(
        "forged-unicode-body-nonboundary-ast".into(),
        unicode,
        held.clone(),
    )?;
    let mut changed = observation.clone();
    changed.artifacts.clear();
    refuse("missing-native-artifacts".into(), changed, held.clone())?;
    let mut wrong_domain = observation.clone();
    let held_layout = wrong_domain
        .layouts
        .remove("rk_duplicate")
        .ok_or("actual rk_duplicate layout absent")?;
    wrong_domain
        .layouts
        .insert("forged_unknown_case".into(), held_layout);
    refuse(
        "replaced-native-layout-case".into(),
        wrong_domain,
        held.clone(),
    )?;
    let mut changed = observation.clone();
    changed.layouts.get_mut("rk_duplicate").unwrap()[0].push('0');
    refuse("forged-native-layout".into(), changed, held.clone())?;
    let mut changed = observation.clone();
    *changed.header_inputs.values_mut().next().unwrap() = "0".repeat(64);
    refuse("changed-header-bytes".into(), changed, held.clone())?;
    let mut changed = observation.clone();
    changed.symbols.remove("rk_ok.so");
    refuse("missing-artifact-symbols".into(), changed, held.clone())?;
    for axis in 0..9 {
        let mut changed = held.clone();
        let value = match axis {
            0 => &mut changed.artifact,
            1 => &mut changed.property,
            2 => &mut changed.semantic_profile,
            3 => &mut changed.scope,
            4 => &mut changed.bounds,
            5 => &mut changed.assumptions,
            6 => &mut changed.source_identity,
            7 => &mut changed.build_identity,
            _ => &mut changed.evidence_version,
        };
        *value = "old-universal-plugin-or-deployed-null-context".into();
        refuse(
            format!("foreign-context-axis-{axis}"),
            observation.clone(),
            changed,
        )?;
    }
    Ok(passed)
}
/// Produces an UNTRUSTED observation using fixed reviewed code and pinned input
/// inventory. The directory must not exist; no arbitrary executable is accepted.
pub fn observe(directory: &Path) -> Result<Observation, String> {
    fs::create_dir(directory)
        .map_err(|e| format!("fresh private output directory required: {e}"))?;
    let inventory = directory.join("inventory.json");
    fs::write(&inventory, INVENTORY).map_err(|e| e.to_string())?;
    let output = Command::new("python3")
        .args(["-c", PRODUCER])
        .arg(&inventory)
        .arg(directory)
        .env("SOURCE_DATE_EPOCH", "1761004800")
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(directory.join("producer.stderr"), &output.stderr).map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "native source observation failed {:?}; see producer.stderr",
            output.status.code()
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}
/// Required exact context derived from actual independently replayed identities.
/// Its public construction grants nothing; `replay` checks every field anew.
pub fn context(observation: &Observation) -> Result<Context, String> {
    Ok(Context {
        artifact: digest(&(
            &observation.artifacts,
            &observation.layouts,
            &observation.symbols,
        ))?,
        property: "plugin_fixture_selected_domain_v1".into(),
        semantic_profile: PROFILE.into(),
        scope: "complete-selected-20-case-domain-v1".into(),
        bounds: BOUNDS.into(),
        assumptions: ASSUMPTIONS.into(),
        source_identity: sha256_hex(INVENTORY),
        build_identity: digest(&(
            observation.compiler.clone(),
            observation.compiler_sha256.clone(),
            observation.target.clone(),
            sha256_hex(PRODUCER.as_bytes()),
            sha256_hex(CHECKER),
            sha256_hex(MODEL.as_bytes()),
            sha256_hex(FORMAL_MODEL),
            sha256_hex(FORMAL_AUDIT),
            digest(&observation.header_inputs)?,
        ))?,
        evidence_version: format!("plugin-source-witness-v1/{}", sha256_hex(CHECKER)),
    })
}
/// Every admission independently repeats the native source observation. No raw
/// JSON/status/receipt can bypass that replay. IDs and AST allocator addresses
/// are not propositions: compare derived typed facts, exact source projections,
/// compiler, target, actual artifacts/symbols and the entire selected trace.
pub fn replay(
    presented: &Observation,
    expected_context: &Context,
    fresh_directory: &Path,
) -> Result<CheckedWitness, String> {
    let presented_facts = validate(presented)?;
    if context(presented)? != *expected_context {
        return Err("foreign declared context".into());
    }
    let actual = observe(fresh_directory)?;
    let source_facts = validate(&actual)?;
    if presented_facts != source_facts
        || presented.projections != actual.projections
        || presented.artifacts != actual.artifacts
        || presented.symbols != actual.symbols
        || presented.layouts != actual.layouts
        || presented.header_inputs != actual.header_inputs
        || presented.compiler_sha256 != actual.compiler_sha256
        || presented.compiler != actual.compiler
        || presented.target != actual.target
        || presented.traces != actual.traces
        || context(&actual)? != *expected_context
        || context(presented)? != *expected_context
    {
        return Err("independent source/native/context replay mismatch".into());
    }
    Ok(CheckedWitness {
        context: expected_context.clone(),
        observation_sha256: digest(&(
            actual.inventory_sha256,
            &actual.artifacts,
            &actual.layouts,
            &actual.header_inputs,
            &actual.traces,
            &source_facts,
        ))?,
        model_sha256: sha256_hex(MODEL.as_bytes()),
        producer_sha256: sha256_hex(PRODUCER.as_bytes()),
        source_facts,
        cases: actual.traces,
        remaining_obligations: vec![
            "deployed-artifact-inventory-and-loader-traces".into(),
            "foreign-callback-frame-custody-and-whole-world-closure".into(),
            "transitive-lifetime-and-unload-order".into(),
            "faithful-clang-extraction-and-maintained-rust-lean-correspondence".into(),
        ],
    })
}
impl CheckedWitness {
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn cases(&self) -> &BTreeMap<String, Vec<String>> {
        &self.cases
    }
}

/// This record is produced only from a freshly replayed checked witness. It is
/// an observed finite predicate and proposed logic result, not a safety verdict.
#[derive(Debug, Serialize)]
pub struct BoundedFlowResult {
    checker: String,
    bounded_atom: String,
    source_observation_sha256: String,
    result: qcore_schemas::premise_flow::ProposedEvaluation,
    evaluations: BTreeMap<String, qcore_schemas::premise_flow::ProposedEvaluation>,
    document_sha256: String,
    graph_sha256: String,
    flow_model_sha256: String,
    source_contract_sha256: String,
    formal_bindings: BTreeMap<String, qcore_schemas::hypothesis::FormalBinding>,
    explicit_artifacts: BTreeMap<String, String>,
}
/// Build separately held flow inputs for the new bounded predicate. None of the
/// old seven universal plugin propositions occurs in this standalone graph.
struct FlowInputs {
    sources: BTreeMap<String, String>,
    graphs: BTreeMap<String, qcore_schemas::hypothesis::Graph>,
    models: BTreeMap<String, qcore_schemas::premise_flow::Model>,
    artifacts: BTreeMap<String, Vec<u8>>,
}
fn flow_inputs() -> Result<FlowInputs, String> {
    use qcore_schemas::hypothesis::{ArtifactReference, EnvironmentDefinition, FormalBinding};
    let source = format!(
        r#"module contract;
extern fn plugin_fixture_duplicate() -> unit
contract {{
 requires resource_contract_v1;
 requires resource_hypothesis({BOUNDED_ATOM}, "The selected rk_duplicate fixture invocation under plugin-loader-fixture-v1 has exactly begin duplicate_test return:0 end; no loader operation occurs inside this invocation");
 requires resource_hypothesis({COMPLETION_ATOM}, "The selected rk_duplicate fixture invocation under plugin-loader-fixture-v1 returns zero and reaches its matching end marker");
 requires resource_hypothesis({GO_COMPLETE_ATOM}, "The selected go_create_ignores_register_failure fixture invocation has a nonnull creator return and matching end marker");
 requires resource_hypothesis({GO_IMPLIES_ATOM}, "Within this exact selected Go creator fixture invocation, a nonnull creator return implies that the required registration callback succeeded");
 requires resource_premise_all(fixture_invocation_observations, {GO_COMPLETE_ATOM}, {GO_IMPLIES_ATOM}, {COMPLETION_ATOM}, {BOUNDED_ATOM});
 requires resource_premise_flow_v1(native_duplicate, {BOUNDED_ATOM});
 requires resource_premise_flow_v1(observed_counterexample, {GO_IMPLIES_ATOM});
 requires resource_premise_flow_v1(known_error, ({GO_COMPLETE_ATOM} !-> {GO_IMPLIES_ATOM}));
 requires resource_premise_flow_v1(alternative, ({BOUNDED_ATOM} ||-> flow(known_error)));
 requires resource_premise_flow_v1(combined, ({BOUNDED_ATOM} |-> flow(known_error)));
 requires resource_premise_flow_v1(finished, (flow(combined) ->| {COMPLETION_ATOM}));
 ensures normal resource_return_none();
}};
"#
    );
    let sources = BTreeMap::from([("bounded-plugin-fixture.qv".into(), source)]);
    let graphs = qcore_contract::hypothesis::replay(&sources)?;
    let graph = graphs
        .get("plugin_fixture_duplicate")
        .ok_or("standalone fixture graph absent")?;
    let model_path = "proof/QCore/PluginSourceWitnesses.lean";
    let audit_path = "proof/AuditPluginSourceWitnesses.lean";
    let model = FORMAL_MODEL.to_vec();
    let audit = FORMAL_AUDIT.to_vec();
    let environment = EnvironmentDefinition {
        id: PROFILE.into(),
        variables: BTreeMap::from([
            ("source_inventory_sha256".into(), sha256_hex(INVENTORY)),
            (
                "native_producer_sha256".into(),
                sha256_hex(PRODUCER.as_bytes()),
            ),
            ("checker_sha256".into(), sha256_hex(CHECKER)),
            ("formal_model_sha256".into(), sha256_hex(FORMAL_MODEL)),
            ("formal_audit_sha256".into(), sha256_hex(FORMAL_AUDIT)),
            ("trace_model_sha256".into(), sha256_hex(MODEL.as_bytes())),
            ("scope".into(), BOUNDS.into()),
            ("assumptions".into(), ASSUMPTIONS.into()),
        ]),
    };
    let binding=FormalBinding{atom_sha256:digest(&graph.atoms[BOUNDED_ATOM])?,proposition:"QCore.PluginSourceWitnesses.DuplicateNoLoaderCalls".into(),model:ArtifactReference{path:model_path.into(),sha256:sha256_hex(&model)},audit:ArtifactReference{path:audit_path.into(),sha256:sha256_hex(&audit)},environment:environment.id.clone(),quantifiers:vec!["trace : List String; selected = rk_duplicate; exact held Context and explicit stub environment".into()],prerequisites:vec![environment.id.clone()],conclusion:"DuplicateNoLoaderCalls trace for this selected invocation only; old plugin atoms/envelopes remain independent".into()};
    let mut completion = binding.clone();
    completion.atom_sha256 = digest(&graph.atoms[COMPLETION_ATOM])?;
    completion.proposition = "QCore.PluginSourceWitnesses.DuplicateCompleted".into();
    completion.conclusion =
        "DuplicateCompleted trace: zero return and matching end of the same selected invocation"
            .into();
    let mut go_complete = binding.clone();
    go_complete.atom_sha256 = digest(&graph.atoms[GO_COMPLETE_ATOM])?;
    go_complete.proposition = "QCore.PluginSourceWitnesses.GoCreatorCompleted".into();
    go_complete.quantifiers=vec!["trace : List String; selected = go_create_ignores_register_failure; exact held Context and explicit stub environment".into()];
    go_complete.conclusion = "GoCreatorCompleted trace for this exact selected invocation".into();
    let mut go_implies = go_complete.clone();
    go_implies.atom_sha256 = digest(&graph.atoms[GO_IMPLIES_ATOM])?;
    go_implies.proposition = "QCore.PluginSourceWitnesses.GoCreatorImpliesRegistration".into();
    go_implies.conclusion="GoCreatorImpliesRegistration trace for this exact selected invocation; a finite counterexample refutes only this implication".into();
    let models = BTreeMap::from([(
        "plugin_fixture_duplicate".into(),
        qcore_schemas::premise_flow::Model {
            environment,
            atoms: BTreeMap::from([
                (BOUNDED_ATOM.into(), binding),
                (COMPLETION_ATOM.into(), completion),
                (GO_COMPLETE_ATOM.into(), go_complete),
                (GO_IMPLIES_ATOM.into(), go_implies),
            ]),
        },
    )]);
    let artifacts = BTreeMap::from([(model_path.into(), model), (audit_path.into(), audit)]);
    Ok(FlowInputs {
        sources,
        graphs,
        models,
        artifacts,
    })
}
fn assignments(
    cases: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<&'static str, qcore_schemas::hypothesis::Disposition>, String> {
    use qcore_schemas::hypothesis::Disposition;
    let duplicate = cases
        .get("rk_duplicate")
        .ok_or("missing actual duplicate invocation")?;
    let go = cases
        .get("go_create_ignores_register_failure")
        .ok_or("missing actual Go invocation")?;
    let exact_duplicate = duplicate
        == &vec![
            "begin".to_owned(),
            "duplicate_test".into(),
            "return:0".into(),
            "end".into(),
        ];
    let duplicate_complete =
        duplicate.iter().any(|v| v == "return:0") && duplicate.last().is_some_and(|v| v == "end");
    let returned = go.iter().any(|v| v == "return:nonnull");
    let go_complete = returned && go.last().is_some_and(|v| v == "end");
    let registered = go.iter().any(|v| v == "register:success");
    if !exact_duplicate || !duplicate_complete || !go_complete || registered {
        return Err("bounded predicates lack complete native assignment".into());
    }
    Ok(BTreeMap::from([
        (BOUNDED_ATOM, Disposition::Supported),
        (COMPLETION_ATOM, Disposition::Supported),
        (GO_COMPLETE_ATOM, Disposition::Supported),
        (GO_IMPLIES_ATOM, Disposition::Refuted),
    ]))
}
impl CheckedWitness {
    /// Consume an admitted witness through the source-replayed operator API.
    /// The distinct completion leaf and no-loader leaf come from the same
    /// complete trace; no caller-provided Supported is accepted.
    pub fn evaluate_duplicate_flow(&self) -> Result<BoundedFlowResult, String> {
        use qcore_contract::premise_flow::{self, FlowInstance};
        use qcore_schemas::hypothesis::{Disposition, Evidence, Instance, Warrant};
        if self.cases != model()? {
            return Err("sealed finite domain changed".into());
        }
        let FlowInputs {
            sources,
            graphs,
            models,
            artifacts,
        } = flow_inputs()?;
        let docs = premise_flow::compile(&sources, &graphs, &models, &artifacts)?;
        let doc = docs
            .get("plugin_fixture_duplicate")
            .ok_or("fixture flow missing")?;
        let graph = graphs["plugin_fixture_duplicate"].clone();
        let packet = FlowInstance {
            document: doc.clone(),
            document_sha256: premise_flow::document_digest(doc)?,
            hypotheses: Instance {
                graph_sha256: qcore_contract::hypothesis::digest(&graph)?,
                context: self.context.clone(),
                evidence: [
                    BOUNDED_ATOM,
                    COMPLETION_ATOM,
                    GO_COMPLETE_ATOM,
                    GO_IMPLIES_ATOM,
                ]
                .into_iter()
                .map(|id| {
                    Ok(Evidence {
                        atom: id.into(),
                        atom_sha256: digest(&graph.atoms[id])?,
                        context: self.context.clone(),
                        disposition: assignments(&self.cases)?[id],
                        warrant: Warrant::Observed,
                        reference: self.observation_sha256.clone(),
                    })
                })
                .collect::<Result<_, String>>()?,
                graph,
            },
        };
        let result = premise_flow::evaluate_proposed(
            &packet,
            premise_flow::Expected {
                sources: &sources,
                graphs: &graphs,
                models: &models,
                artifacts: &artifacts,
                document: doc,
                context: &self.context,
            },
            "finished",
        )?;
        if result.disposition != Disposition::Supported
            || !result.terminated
            || result.atom_dependencies
                != vec![
                    GO_COMPLETE_ATOM.to_owned(),
                    GO_IMPLIES_ATOM.to_owned(),
                    COMPLETION_ATOM.to_owned(),
                    BOUNDED_ATOM.to_owned(),
                ]
        {
            return Err("unexpected bounded flow result".into());
        }
        let mut evaluations = BTreeMap::new();
        for entry in [
            "native_duplicate",
            "observed_counterexample",
            "known_error",
            "alternative",
            "combined",
            "finished",
        ] {
            let evaluated = premise_flow::evaluate_proposed(
                &packet,
                premise_flow::Expected {
                    sources: &sources,
                    graphs: &graphs,
                    models: &models,
                    artifacts: &artifacts,
                    document: doc,
                    context: &self.context,
                },
                entry,
            )?;
            if (entry == "observed_counterexample" && evaluated.disposition != Disposition::Refuted)
                || (entry != "observed_counterexample"
                    && evaluated.disposition != Disposition::Supported)
            {
                return Err("finite observed-error propagation mismatch".into());
            }
            evaluations.insert(entry.into(), evaluated);
        }
        Ok(BoundedFlowResult {
            checker: "quirin-source-analysis/plugin-source-witness-v1".into(),
            bounded_atom: BOUNDED_ATOM.into(),
            source_observation_sha256: self.observation_sha256.clone(),
            result,
            evaluations,
            document_sha256: packet.document_sha256.clone(),
            graph_sha256: packet.hypotheses.graph_sha256.clone(),
            flow_model_sha256: doc.model_sha256.clone(),
            source_contract_sha256: doc.source_sha256.clone(),
            formal_bindings: models["plugin_fixture_duplicate"].atoms.clone(),
            explicit_artifacts: artifacts
                .iter()
                .map(|(path, bytes)| (path.clone(), sha256_hex(bytes)))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaced_native_layout_case_refuses_exact_domain() {
        let traces = model().unwrap();
        let mut layouts = traces
            .keys()
            .map(|k| (k.clone(), vec!["LAYOUT:1:1".into()]))
            .collect::<BTreeMap<_, _>>();
        assert!(exact_layout_domain(&layouts, &traces).is_ok());
        let held = layouts.remove("rk_duplicate").unwrap();
        layouts.insert("forged_unknown_case".into(), held);
        assert_eq!(layouts.len(), 20);
        assert!(exact_layout_domain(&layouts, &traces).is_err());
    }
    #[test]
    fn malformed_offsets_and_utf8_prefix_refuse_without_panics() {
        let projection = Projection {
            module: "synthetic boundary only".into(),
            function: "boundary".into(),
            start: 10,
            end: 14,
            body: "é{}".into(),
            sha256: String::new(),
        };
        assert!(checked_projection_prefix(&projection, 11, 14).is_err());
        assert!(checked_projection_prefix(&projection, usize::MAX, 14).is_err());
        assert!(checked_projection_prefix(&projection, 9, 14).is_err());
        assert!(
            offset(
                &serde_json::json!({"range":{"end":{"offset":u64::MAX}}}),
                "end"
            )
            .unwrap()
            .checked_add(1)
            .is_none()
        );
    }
    #[test]
    fn finite_domain_and_exact_trace_refusals() {
        let traces = model().unwrap();
        assert_eq!(traces.len(), 20);
        assert_eq!(
            traces["rk_duplicate"],
            vec!["begin", "duplicate_test", "return:0", "end"]
        );
        assert!(!traces["go_create_ignores_register_failure"].contains(&"register".into()));
        assert!(traces["flb_c_context_alloc_fail"].contains(&"link".into()));
        for key in traces.keys() {
            let mut changed = traces.clone();
            changed.remove(key);
            assert_ne!(changed, model().unwrap());
        }
    }
    #[test]
    fn bounded_flow_source_replay_and_tampered_model_refuse() {
        let FlowInputs {
            sources,
            graphs,
            models,
            artifacts,
        } = flow_inputs().unwrap();
        let docs =
            qcore_contract::premise_flow::compile(&sources, &graphs, &models, &artifacts).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(graphs["plugin_fixture_duplicate"].atoms.len(), 4);
        assert_eq!(graphs["plugin_fixture_duplicate"].premises.len(), 1);
        let mut wrong = artifacts.clone();
        wrong.values_mut().next().unwrap().push(b' ');
        assert!(qcore_contract::premise_flow::compile(&sources, &graphs, &models, &wrong).is_err());
        let mut missing = models.clone();
        missing.values_mut().next().unwrap().atoms.clear();
        assert!(
            qcore_contract::premise_flow::compile(&sources, &graphs, &missing, &artifacts).is_err()
        );
    }
}
