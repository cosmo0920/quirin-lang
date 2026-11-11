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

//! User-defined contracts and checker plugins (card
//! **W45-USER-CONTRACT-PLUGINS**; contract specification §C11.1, language
//! specification §L21.2).
//!
//! A user keeps contract crates outside this repository, laid out like the
//! `quirin-provider-*` crates, and lists them in a load-path manifest that
//! extends a shipped one (`qcore_contract::load_path_manifest`). Their modules
//! are selected, loaded, checked and pinned exactly like shipped modules. This
//! module adds what is particular to them:
//!
//! * **Collisions.** A selected user module that declares a function a
//!   shipped crate also declares is refused, unless the user manifest carries
//!   an explicit override naming the function, the user crate, the shipped
//!   crate and a reason; every applied override is recorded under the
//!   selection's `user_load_path` ([`check_overrides`]). The declared names are
//!   read lexically (`extern fn NAME`), which can only over-report a
//!   collision, never miss one.
//! * **Checker plugins.** A checker bound to a user module runs once per run,
//!   out of process, as a verified package through the verifier host's
//!   registry, and its answer is classified by the plugin firewall
//!   (`qcore_verifier_host::plugin_firewall`) into a four-value verdict
//!   ([`run`]). The request is evidence v2 over a core-derived task: one
//!   `parameter_mode_respected` obligation per bound function, bound to the
//!   module's text digest.
//! * **Chains.** A function's published status is the fold of its own
//!   analysis and the verdicts of the checkers attached to every contract it
//!   applies — a callee chain ([`fold_unit`]). A unit's status is joined with
//!   its functions' folded statuses. Without a checker binding nothing here
//!   runs, and every report is byte-identical to one written before the card.
//!
//! Which contracts a function applies is read from its control-flow graph: a
//! contracted call is lowered to `LegacyInstruction::ContractCall` (with its
//! admitted nested calls), and a call the analysis refused applies no contract
//! and already leaves the function `Unknown`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_c_gateway::legacy_c::{
    LegacyCError, LegacyFunctionSummary, LegacyInstruction, UnrestrictedLegacyResult,
};
use qcore_contract::load_path_manifest::{CheckerBinding, LoadPathExtension, LoadPathManifest};
use qcore_schemas::sha256_hex;
use qcore_schemas::verdict::{ReasonedVerdict, Verdict};
use qcore_verifier_host::{
    CounterexampleChecker, Obligation, ObligationProperty, ObligationScope, ObligationSubject,
    PLUGIN_EVIDENCE_VERSION_V2, PLUGIN_PROTOCOL_VERSION, PluginClaim, ProcessProvider,
    TargetProfile, TaskBounds, UnitIdentity, VERIFICATION_TASK_VERSION, VerificationRequestV2,
    VerificationTask, VerifierCapability, VerifierProvider, VerifierRegistry, WitnessChecker,
};
use serde_json::{Value, json};

use crate::providers::SelectedModule;

/// The semantic profile of a user-contract check task.
pub const SEMANTIC_PROFILE: &str = "quirin.user-contract-check.v1";

/// What the `plugin_checks` sidecar section means, carried in it.
pub const MEANING: &str = "checker plugins attached to user contracts (card W45-USER-CONTRACT-PLUGINS, contract spec C11.1): each binding runs once, out of process, as a verified package, on an evidence-v2 task the core derives (one parameter_mode_respected obligation per bound function, bound to the module's text digest); a plugin's Safe counts only with a certificate the registry issued from a trusted-core witness replay, and without one it is Pending (plugin-claim-uncertified); its Unsafe counts, provisionally, only with a witness a trusted-core counterexample checker validated, and without one it is Pending; a crash, timeout, quota breach, protocol error, mismatch or refuted evidence is Unknown; every function that applies a bound contract publishes the join of its own status and the binding's verdict (Safe < Pending < Unknown < Unsafe), and every unit the join of its functions'";

/// The declared function names of a contract module, read lexically: every
/// `extern fn NAME` outside a comment. Over-reports rather than misses.
#[must_use]
pub fn declared_functions(text: &str) -> BTreeSet<String> {
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after.find('\n').map_or("", |end| &after[end..]);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |end| &after[end + 2..]);
            stripped.push(' ');
        } else {
            let character = rest.chars().next().expect("non-empty");
            stripped.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    let words: Vec<&str> = stripped
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect();
    words
        .windows(3)
        .filter(|window| window[0] == "extern" && window[1] == "fn")
        .map(|window| window[2].to_owned())
        .collect()
}

/// Refuse a selected user module that declares a shipped function without an
/// explicit override, and record the overrides applied (the selection's
/// `user_load_path` record).
///
/// # Errors
///
/// A collision without an override.
pub fn check_overrides(
    manifest: &LoadPathManifest,
    extension: &LoadPathExtension,
    modules: &[SelectedModule],
) -> Result<Value, String> {
    let mut shipped = BTreeMap::<String, Vec<(String, String)>>::new();
    for name in &extension.shipped_crates {
        let Some(krate) = manifest.load_path.crate_named(name) else {
            continue;
        };
        for (module, text) in &krate.modules {
            for function in declared_functions(text) {
                shipped
                    .entry(function)
                    .or_default()
                    .push((name.clone(), module.clone()));
            }
        }
    }
    let mut applied = Vec::new();
    let mut used = BTreeSet::new();
    for module in modules
        .iter()
        .filter(|module| extension.user_crates.contains(&module.crate_name))
    {
        let text = &manifest
            .load_path
            .crate_named(&module.crate_name)
            .ok_or_else(|| format!("user crate `{}` is not on the load path", module.crate_name))?
            .modules[&module.module];
        for function in declared_functions(text) {
            let Some(owners) = shipped.get(&function) else {
                continue;
            };
            for (shipped_crate, shipped_module) in owners {
                let Some((index, record)) =
                    extension.overrides.iter().enumerate().find(|(_, record)| {
                        record.function == function
                            && record.user_crate == module.crate_name
                            && &record.shipped_crate == shipped_crate
                    })
                else {
                    return Err(format!(
                        "user contract `{function}` in `{}:{}` collides with the shipped contract \
                         in `{shipped_crate}:{shipped_module}`: a user contract never replaces a \
                         shipped one silently; rename it, or add an explicit override \
                         {{\"function\": \"{function}\", \"crate\": \"{}\", \"shipped_crate\": \
                         \"{shipped_crate}\", \"reason\": \"…\"}} to the user load path",
                        module.crate_name, module.module, module.crate_name
                    ));
                };
                used.insert(index);
                applied.push(json!({
                    "function": function,
                    "crate": record.user_crate,
                    "module": module.module,
                    "shipped_crate": shipped_crate,
                    "shipped_module": shipped_module,
                    "reason": record.reason,
                }));
            }
        }
    }
    let unused = extension
        .overrides
        .iter()
        .enumerate()
        .filter(|(index, _)| !used.contains(index))
        .map(|(_, record)| {
            json!({
                "function": record.function, "crate": record.user_crate,
                "shipped_crate": record.shipped_crate, "reason": record.reason,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "extends": extension.base,
        "extends_sha256": extension.base_sha256,
        "user_crates": extension.user_crates,
        "overrides_applied": applied,
        "overrides_unused": unused,
        "checkers": extension.checkers.iter().map(|binding| json!({
            "crate": binding.user_crate, "module": binding.module,
            "functions": binding.functions, "package": binding.package_spelling,
        })).collect::<Vec<_>>(),
        "rule": "a selected user module that declares (extern fn NAME, read lexically) a function a shipped crate declares is refused unless an override names the function, the user crate, the shipped crate and a reason; each applied override is listed here",
    }))
}

/// The trusted-core checkers a run may certify with. The command line has
/// none, so every plugin `Safe` and `Unsafe` it sees is `Pending`; a library
/// caller supplies the ones it trusts.
#[derive(Default, Clone, Copy)]
pub struct PluginCheckers<'a> {
    pub witness: Option<&'a dyn WitnessChecker>,
    pub counterexample: Option<&'a dyn CounterexampleChecker>,
}

/// One checker's verdict on one bound function's contract.
#[derive(Clone, Debug)]
struct Link {
    plugin: String,
    user_crate: String,
    module: String,
    verdict: ReasonedVerdict,
}

/// Every bound function's links, keyed by the function a contract declares.
#[derive(Clone, Debug, Default)]
pub struct PluginLinks {
    links: BTreeMap<String, Vec<Link>>,
}

impl PluginLinks {
    fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

/// The run's checker results: the links the report folds, and the sidecar
/// section.
pub struct PluginRun {
    pub links: Arc<PluginLinks>,
    pub section: Value,
}

/// The core-derived task of one binding.
#[must_use]
pub fn checker_task(binding: &CheckerBinding) -> VerificationTask {
    let translation_unit = format!("{}:{}", binding.user_crate, binding.module);
    let mut task = VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: UnitIdentity {
            source_digest: binding.module_sha256.clone(),
            build_digest: sha256_hex(
                format!("{}@{}", binding.user_crate, binding.crate_version).as_bytes(),
            ),
            translation_unit: translation_unit.clone(),
        },
        target: TargetProfile {
            library: binding.crate_namespace.clone(),
            revision: binding.crate_version.clone(),
            profile: "user-contract".into(),
            dialect: "qcore-resource-v1".into(),
            platform: "any".into(),
            backend: "quirin-audit".into(),
        },
        semantic_profile: SEMANTIC_PROFILE.into(),
        contract_sources: BTreeMap::from([(
            translation_unit.clone(),
            binding.module_sha256.clone(),
        )]),
        bounds: TaskBounds::single_path(),
        assumptions: BTreeMap::new(),
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        obligations: Vec::new(),
        derivation_incomplete: Vec::new(),
    };
    for symbol in &binding.functions {
        let subject = ObligationSubject::Function {
            symbol: symbol.clone(),
            translation_unit: translation_unit.clone(),
        };
        let obligation_id = task.obligation_id(
            VerifierCapability::ResourceContracts,
            ObligationProperty::ParameterModeRespected,
            &subject,
        );
        task.obligations.push(Obligation {
            obligation_id,
            capability: VerifierCapability::ResourceContracts,
            property: ObligationProperty::ParameterModeRespected,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([symbol.clone()]),
            },
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        });
    }
    task.obligations.sort();
    task
}

/// The evidence-v2 request for `task` to `plugin`.
#[must_use]
pub fn checker_request(
    plugin: &qcore_verifier_host::PluginId,
    task: &VerificationTask,
) -> VerificationRequestV2 {
    let digest = task.task_digest();
    VerificationRequestV2 {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: format!("{plugin}:{}", &digest[..16]),
        plugin_id: plugin.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: task.target.clone(),
        unit: task.unit.clone(),
        facts: Vec::new(),
        assumptions: task.assumptions.clone(),
        task_digest: digest,
        obligations: task.obligations.clone(),
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

fn claim_kind(claim: &PluginClaim) -> &'static str {
    match claim {
        PluginClaim::CertifiedSafe(_) => "certified-safe",
        PluginClaim::ValidatedUnsafe { .. } => "validated-unsafe",
        PluginClaim::UncertifiedSafe { .. } => "uncertified-safe",
        PluginClaim::UnvalidatedUnsafe { .. } => "unvalidated-unsafe",
        PluginClaim::Undecided { .. } => "undecided",
        PluginClaim::Failure { .. } => "failure",
    }
}

/// Run every checker binding of a user load path once and classify each
/// answer. `None` when there is no binding.
#[must_use]
pub fn run(
    extension: Option<&LoadPathExtension>,
    checkers: PluginCheckers<'_>,
) -> Option<PluginRun> {
    let extension = extension.filter(|extension| !extension.checkers.is_empty())?;
    let mut links = PluginLinks::default();
    let mut rows = Vec::new();
    let mut counts = BTreeMap::<&'static str, u64>::new();
    for verdict in Verdict::ALL {
        counts.insert(verdict.as_str(), 0);
    }
    for binding in &extension.checkers {
        let task = checker_task(binding);
        let mut row = json!({
            "crate": binding.user_crate,
            "module": binding.module,
            "module_sha256": binding.module_sha256,
            "functions": binding.functions,
            "package": binding.package_spelling,
            "task_digest": task.task_digest(),
        });
        let (plugin, claim) = match ProcessProvider::from_package(&binding.package) {
            Err(error) => (
                binding.package_spelling.clone(),
                PluginClaim::Failure {
                    code: "package".into(),
                    reason: format!("the checker package does not verify: {error}"),
                },
            ),
            Ok(provider) => {
                let manifest = provider.manifest().clone();
                row["plugin_id"] = json!(manifest.plugin_id);
                row["package_digest"] = json!(manifest.package_digest);
                row["execution_profile"] = json!(provider.execution_profile().to_string());
                row["limits"] = json!({
                    "timeout_ms": manifest.limits.timeout_ms,
                    "memory_bytes": manifest.limits.memory_bytes,
                    "output_bytes": manifest.limits.output_bytes,
                });
                let mut registry = VerifierRegistry::default();
                let claim = match registry.register(Arc::new(provider)) {
                    Ok(()) => registry.verify_plugin_claim(
                        &checker_request(&manifest.plugin_id, &task),
                        &task,
                        checkers.witness,
                        checkers.counterexample,
                    ),
                    Err(error) => PluginClaim::Failure {
                        code: "registry".into(),
                        reason: error.to_string(),
                    },
                };
                (manifest.plugin_id.to_string(), claim)
            }
        };
        let verdict = claim.verdict(&plugin);
        *counts.entry(verdict.verdict().as_str()).or_default() += 1;
        row["claim"] = json!(claim_kind(&claim));
        row["code"] = json!(claim.code());
        row["verdict"] = json!(verdict.verdict());
        row["reasons"] = json!(verdict.reasons());
        rows.push(row);
        for function in &binding.functions {
            links.links.entry(function.clone()).or_default().push(Link {
                plugin: plugin.clone(),
                user_crate: binding.user_crate.clone(),
                module: binding.module.clone(),
                verdict: verdict.clone(),
            });
        }
    }
    Some(PluginRun {
        links: Arc::new(links),
        section: json!({
            "schema_version": 1,
            "meaning": MEANING,
            "bindings": rows,
            "counts": counts,
        }),
    })
}

/// The console tally of a [`PluginRun`]'s section.
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section["counts"][key].as_u64().unwrap_or_default();
    format!(
        "checker plugins: {} binding(s): {} Safe / {} Pending / {} Unknown / {} provisional Unsafe",
        section["bindings"].as_array().map_or(0, Vec::len),
        count("Safe"),
        count("Pending"),
        count("Unknown"),
        count("Unsafe"),
    )
}

/// The contracted callees a function applies: every `ContractCall` of its
/// control-flow graph and every nested call it admits.
#[must_use]
pub fn applied_contracts(function: &LegacyFunctionSummary) -> BTreeSet<String> {
    let mut callees = BTreeSet::new();
    for node in &function.cfg.nodes {
        if let LegacyInstruction::ContractCall {
            function, nested, ..
        } = &node.instruction
        {
            callees.insert(function.clone());
            callees.extend(nested.iter().map(|call| call.function.clone()));
        }
    }
    callees
}

/// Fold the checker links into one just-recorded unit row: each function's
/// status becomes the join of its own and the links of every contract it
/// applies, and the unit's the join of its own and its functions'. A row
/// with no linked callee is left untouched.
pub(crate) fn fold_unit(
    links: &PluginLinks,
    row: &mut Value,
    result: &Result<UnrestrictedLegacyResult, LegacyCError>,
) {
    if links.is_empty() {
        return;
    }
    let summary = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => &certificate.summary,
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => summary,
        Ok(UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }) => summary,
        _ => return,
    };
    let Some(unit_verdict) = row["status"]
        .as_str()
        .and_then(|status| status.parse::<Verdict>().ok())
    else {
        return;
    };
    let mut joined = unit_verdict;
    let mut unit_reasons = Vec::new();
    let mut linked_any = false;
    for function_row in row["functions"].as_array_mut().into_iter().flatten() {
        let Some(name) = function_row["function"].as_str() else {
            continue;
        };
        let Some(function) = summary.functions.get(name) else {
            continue;
        };
        let applied = applied_contracts(function);
        let function_links: Vec<(&String, &Link)> = applied
            .iter()
            .filter_map(|callee| links.links.get(callee).map(|links| (callee, links)))
            .flat_map(|(callee, links)| links.iter().map(move |link| (callee, link)))
            .collect();
        if function_links.is_empty() {
            continue;
        }
        linked_any = true;
        let own = function.status.verdict();
        let folded = function_links.iter().fold(own, |verdict, (_, link)| {
            verdict.join(link.verdict.verdict())
        });
        function_row["status"] = json!(folded.as_str());
        function_row["plugin_links"] = function_links
            .iter()
            .map(|(callee, link)| {
                json!({
                    "callee": callee,
                    "plugin": link.plugin,
                    "crate": link.user_crate,
                    "module": link.module,
                    "verdict": link.verdict.verdict(),
                    "reasons": link.verdict.reasons(),
                })
            })
            .collect::<Vec<_>>()
            .into();
        for (_, link) in &function_links {
            if !link.verdict.verdict().is_safe() {
                unit_reasons.extend(link.verdict.reasons().iter().map(|r| r.detail.clone()));
            }
        }
        joined = joined.join(folded);
    }
    if !linked_any {
        return;
    }
    if joined != unit_verdict {
        row["status"] = json!(joined.as_str());
        if row["reason"].is_null()
            && let Some(reason) = unit_reasons.first()
        {
            row["reason"] = json!(reason);
            row["reason_code"] = crate::report::reason_code(Some(reason));
        }
    }
}
