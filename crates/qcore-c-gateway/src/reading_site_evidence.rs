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

//! Selected libc checker observations. Wire records confer no source authority.
//!
//! A capture is a successful local admission, possibly inside a function whose
//! complete CFG later refuses. It is not exhaustive site coverage. Independent
//! replay reruns the existing reader; no receipt or reference discharges an atom.
use crate::legacy_c::{CCompilerDialect, LegacyCError, Token};
use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract};
use qcore_schemas::hypothesis::{Context, Graph};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA: &str = "libc-reading-site-evidence.v1";
pub const PREMISE: &str = "nested_call_admission";
pub const BINDINGS: &str =
    include_str!("../../../docs/reports/libc-reading-site-evidence/bindings.json");

/// Detached diagnostics only; the existing Observation/Receipt schemas stay v1.
pub const ENVIRONMENTS_SCHEMA: &str = "libc-reading-site-effective-environments.v1";

/// Exact canonical effective environment, bound to an existing observation hash.
/// Parsing these bytes confers neither source correspondence nor premise truth.
pub fn validate_environment(
    canonical: &str,
    expected_sha256: &str,
) -> Result<ResourceContracts, String> {
    if qcore_schemas::sha256_hex(canonical.as_bytes()) != expected_sha256 {
        return Err("reading-site environment: digest mismatch".into());
    }
    let environment: ResourceContracts =
        serde_json::from_str(canonical).map_err(|e| e.to_string())?;
    if serde_json::to_string(&environment).map_err(|e| e.to_string())? != canonical {
        return Err("reading-site environment: noncanonical or unknown fields".into());
    }
    Ok(environment)
}

/// Require exact bytes and the selected loaded contract before publication.
pub fn validate_observation_environment(
    observation: &Observation,
    captured: &BTreeMap<String, String>,
) -> Result<(), String> {
    let canonical = captured
        .get(&observation.environment_sha256)
        .ok_or("reading-site environment: captured bytes missing")?;
    let environment = validate_environment(canonical, &observation.environment_sha256)?;
    if environment.contract(&observation.declaration) != Some(&observation.contract) {
        return Err("reading-site environment: selected loaded contract differs".into());
    }
    Ok(())
}

/// Select only actual successfully bound receipts. Other captured fragments do
/// not enlarge the published domain, and no verdict reads this returned map.
pub fn publication_environments(
    receipts: &[Receipt],
    captured: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut selected = BTreeMap::new();
    for receipt in receipts {
        validate_observation_environment(&receipt.observation, captured)?;
        let sha = &receipt.observation.environment_sha256;
        selected.insert(sha.clone(), captured[sha].clone());
    }
    Ok(selected)
}

/// Independently held artifact bytes resolve the references. This verifies
/// identity and a named source anchor, not Lean validity or source extraction.
pub fn resolve_bindings(
    manifest: &[u8],
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    if manifest != BINDINGS.as_bytes() {
        return Err("reading-site: unexpected binding manifest".into());
    }
    let value: serde_json::Value = serde_json::from_slice(manifest).map_err(|e| e.to_string())?;
    let references = value["artifacts"]
        .as_object()
        .ok_or("reading-site: missing artifact map")?;
    for reference in references.values() {
        let path = reference["path"]
            .as_str()
            .ok_or("reading-site: artifact path absent")?;
        let bytes = artifacts
            .get(path)
            .ok_or_else(|| format!("reading-site: independently held artifact absent: {path}"))?;
        if reference["sha256"].as_str() != Some(qcore_schemas::sha256_hex(bytes).as_str())
            || !std::str::from_utf8(bytes)
                .map_err(|e| e.to_string())?
                .contains(
                    reference["symbol"]
                        .as_str()
                        .ok_or("reading-site: symbol absent")?,
                )
        {
            return Err(format!("reading-site: artifact reference mismatch: {path}"));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    ScalarCondition,
    NestedArgument,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sibling {
    pub function: String,
    pub argument: usize,
    pub call: Span,
    pub outer_argument: Span,
    pub dereferenced: bool,
}

/// An observation of accepted reader predicates, not a C execution observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub function: String,
    pub declaration: String,
    pub route: Route,
    pub primary_source_sha256: String,
    pub token_stream_sha256: String,
    pub environment_sha256: String,
    pub statement: Span,
    pub call: Span,
    pub outer_argument: Option<Span>,
    pub siblings: Vec<Sibling>,
    pub contract: ResourceFunctionContract,
    pub checked_predicates: BTreeMap<String, bool>,
    pub prerequisite_ids: Vec<String>,
    pub model_ids: Vec<String>,
}

/// Context is supplied by the audit, never inferred from this serialized record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: String,
    pub id: String,
    pub claim: String,
    pub coverage: String,
    pub observation: Observation,
    pub context: Context,
    pub binding_manifest_sha256: String,
    pub graph_sha256: String,
    pub atom_sha256: BTreeMap<String, String>,
    pub proposition_ids: BTreeMap<String, String>,
    pub universal_disposition: String,
}

pub(crate) fn token_digest(tokens: &[Token]) -> String {
    qcore_schemas::sha256_hex(
        &serde_json::to_vec(&tokens.iter().map(|t| (&t.text, t.line)).collect::<Vec<_>>())
            .expect("token pairs serialize"),
    )
}

fn range(full: &[Token], part: &[Token]) -> Option<Span> {
    if part.is_empty() || part.len() > full.len() {
        return None;
    }
    let mut matches = full.windows(part.len()).enumerate().filter(|(_, window)| {
        window
            .iter()
            .zip(part)
            .all(|(a, b)| a.text == b.text && a.line == b.line)
    });
    let (start, _) = matches.next()?;
    if matches.next().is_some() {
        return None; // A line number alone never disambiguates two sites.
    }
    Some(Span {
        start,
        end: start + part.len(),
    })
}

pub(crate) struct Capture<'a> {
    pub function: &'a str,
    pub declaration: &'a str,
    pub route: Route,
    pub full: &'a [Token],
    pub statement: &'a [Token],
    pub call: Span,
    pub outer_argument: Option<Span>,
    pub siblings: Vec<Sibling>,
    pub contract: &'a ResourceFunctionContract,
    pub environment: &'a ResourceContracts,
    pub checked_predicates: BTreeMap<String, bool>,
}

/// Invoked only after the existing local checker succeeded. Foreign/rewrite
/// streams and ambiguous spans are omitted, never relabelled primary evidence.
pub(crate) fn capture(input: Capture<'_>) {
    use crate::contract_dependencies as recording;
    if !recording::collecting() || input.function.is_empty() {
        return;
    }
    let Some((primary_source_sha256, primary_tokens)) = recording::reading_primary() else {
        return;
    };
    let token_stream_sha256 = token_digest(input.full);
    if token_stream_sha256 != primary_tokens {
        recording::reading_omitted("foreign-or-rewritten-stream");
        return;
    }
    let Some(statement) = range(input.full, input.statement) else {
        recording::reading_omitted("ambiguous-or-unlocated-token-span");
        return;
    };
    let shift = |s: Span| Span {
        start: statement.start + s.start,
        end: statement.start + s.end,
    };
    if input.call.start >= input.call.end
        || input.call.end > input.statement.len()
        || input.checked_predicates.values().any(|x| !x)
    {
        recording::reading_omitted("incomplete-local-check");
        return;
    }
    let call = shift(input.call);
    let outer_argument = input.outer_argument.map(shift);
    let siblings = input
        .siblings
        .into_iter()
        .map(|mut s| {
            s.call = shift(s.call);
            s.outer_argument = shift(s.outer_argument);
            s
        })
        .collect();
    let model = match input.route {
        Route::ScalarCondition => "QCore.ScalarAccessorCondition.admission_needs_every_fact",
        Route::NestedArgument => "QCore.VariadicNested.an_admitted_nested_slot_is_data",
    };
    let canonical_environment =
        serde_json::to_string(input.environment).expect("environment serializes");
    recording::reading_observed(
        Observation {
            schema: SCHEMA.into(),
            function: input.function.into(),
            declaration: input.declaration.into(),
            route: input.route,
            primary_source_sha256,
            token_stream_sha256,
            environment_sha256: qcore_schemas::sha256_hex(canonical_environment.as_bytes()),
            statement,
            call,
            outer_argument,
            siblings,
            contract: input.contract.clone(),
            checked_predicates: input.checked_predicates,
            prerequisite_ids: vec![
                "C02.declaration-completeness".into(),
                "C02-P.alias-and-actual-closure".into(),
                "boundary28.source-extraction".into(),
                "boundary28.rust-lean-correspondence".into(),
                "loaded-contract-mode-truth".into(),
            ],
            model_ids: vec![model.into(), "QCore.LibcReadingPremises".into()],
        },
        canonical_environment,
    );
}

fn atoms(declaration: &str) -> Option<&'static [&'static str]> {
    match declaration {
        "__errno_location" => Some(&[
            "checked_reading_route",
            "scalar_declaration",
            "untracked_effects",
        ]),
        "strerror" => Some(&[
            "checked_nested_site",
            "contract_in_force",
            "no_custody_result",
            "parameters_borrowed",
            "remaining_site_checks",
        ]),
        _ => None,
    }
}

/// Bind an actual captured observation to independently held graph/context.
/// This returns records only, never hypothesis::Evidence or a verdict.
pub fn bind(
    observation: &Observation,
    context: &Context,
    graph: &Graph,
) -> Result<Receipt, String> {
    if context.source_identity != observation.primary_source_sha256
        || !context
            .artifact
            .ends_with(&format!("#{}", observation.function))
        || context.property != "resource-custody.v1"
        || observation.schema != SCHEMA
        || observation.checked_predicates.is_empty()
        || observation.checked_predicates.values().any(|x| !x)
        || [
            &context.artifact,
            &context.semantic_profile,
            &context.scope,
            &context.bounds,
            &context.assumptions,
            &context.build_identity,
            &context.evidence_version,
        ]
        .iter()
        .any(|x| x.is_empty())
    {
        return Err("reading-site: incomplete or incompatible binding".into());
    }
    let names = atoms(&observation.declaration).ok_or("reading-site: unselected declaration")?;
    let premise = graph
        .premises
        .get(PREMISE)
        .ok_or("reading-site: missing existing premise")?;
    if premise.children != names.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>() {
        return Err("reading-site: existing premise differs from selected atom map".into());
    }
    let mut atom_sha256 = BTreeMap::new();
    let mut proposition_ids = BTreeMap::new();
    for &name in names {
        let atom = graph.atoms.get(name).ok_or("reading-site: missing atom")?;
        atom_sha256.insert(
            name.into(),
            qcore_schemas::sha256_hex(&serde_json::to_vec(atom).map_err(|e| e.to_string())?),
        );
        proposition_ids.insert(
            name.into(),
            format!("libc.{}.{}.{}.v1", observation.declaration, PREMISE, name),
        );
    }
    let graph_sha256 = qcore_contract::hypothesis::digest(graph)?;
    let id = qcore_schemas::sha256_hex(
        &serde_json::to_vec(&(observation, context, &graph_sha256, &atom_sha256))
            .map_err(|e| e.to_string())?,
    );
    Ok(Receipt {
        schema: SCHEMA.into(),
        id,
        claim: "accepted-local-checker-fragment".into(),
        coverage: "selected-observations-only".into(),
        observation: observation.clone(),
        context: context.clone(),
        binding_manifest_sha256: qcore_schemas::sha256_hex(BINDINGS.as_bytes()),
        graph_sha256,
        atom_sha256,
        proposition_ids,
        universal_disposition: "open".into(),
    })
}

/// Independently held source, graph and context are required. Hash agreement
/// alone is insufficient: every span is read again and the local capture is
/// recollected by the original analysis, with the caller's dialect/selection.
#[allow(clippy::too_many_arguments)]
pub fn replay(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected: Option<&[String]>,
    dialect: CCompilerDialect,
    expected_graphs: &BTreeMap<String, Graph>,
    expected_context: &Context,
    receipts: &[Receipt],
    binding_manifest: &[u8],
    formal_artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    resolve_bindings(binding_manifest, formal_artifacts)?;
    qcore_contract::hypothesis::check_replay(contracts.source_assumptions(), expected_graphs)?;
    if qcore_schemas::sha256_hex(source.as_bytes()) != expected_context.source_identity {
        return Err("reading-site: independently held source differs".into());
    }
    let tokens = crate::legacy_c::tokenize(source);
    for receipt in receipts {
        let graph = expected_graphs
            .get(&receipt.observation.declaration)
            .ok_or("reading-site: expected graph absent")?;
        if bind(&receipt.observation, expected_context, graph)? != *receipt
            || token_digest(&tokens) != receipt.observation.token_stream_sha256
            || receipt.observation.statement.end > tokens.len()
            || receipt.observation.call.start >= receipt.observation.call.end
            || receipt.observation.call.end > receipt.observation.statement.end
            || receipt.observation.call.start < receipt.observation.statement.start
        {
            return Err("reading-site: stale or inconsistent receipt".into());
        }
    }
    let recording = crate::contract_dependencies::Recording::start();
    crate::legacy_c::replay_reading_observations(source, entry, contracts, selected, dialect)
        .map_err(|e: LegacyCError| format!("reading-site replay: {e:?}"))?;
    let fresh = recording.finish();
    for receipt in receipts {
        if !fresh.reading_sites.contains(&receipt.observation)
            || !fresh
                .functions
                .get(&receipt.observation.function)
                .is_some_and(|consulted| consulted.found.contains(&receipt.observation.declaration))
        {
            return Err(
                "reading-site: observation or actual consultation not independently reproduced"
                    .into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_dependencies as recording;
    const SOURCE: &str = "int *__errno_location(void);\nint f(void) { if (*__errno_location() == 1) return 1; return 0; }\n";
    fn contracts() -> ResourceContracts {
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "errno.qv".into(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/errno.qv").into(),
        )]))
        .unwrap()
    }
    #[test]
    fn transaction_discards_success_on_late_refusal_and_retains_committed_capture() {
        let c = contracts();
        let record = recording::Recording::start();
        {
            let _late_refusal = recording::ReadingTransaction::start();
            crate::legacy_c::replay_reading_observations(
                SOURCE,
                "f",
                &c,
                None,
                CCompilerDialect::C11,
            )
            .unwrap();
        }
        let discarded = record.finish();
        assert!(discarded.reading_sites.is_empty());
        assert!(discarded.reading_environments.is_empty());
        let record = recording::Recording::start();
        {
            let mut accepted = recording::ReadingTransaction::start();
            crate::legacy_c::replay_reading_observations(
                SOURCE,
                "f",
                &c,
                None,
                CCompilerDialect::C11,
            )
            .unwrap();
            accepted.commit();
        }
        let accepted = record.finish();
        assert!(!accepted.reading_sites.is_empty());
        assert!(!accepted.reading_environments.is_empty());
        for observation in &accepted.reading_sites {
            validate_observation_environment(observation, &accepted.reading_environments).unwrap();
        }
    }
    #[test]
    fn hypothetical_and_unrecorded_analysis_cannot_publish_success() {
        let c = contracts();
        let record = recording::Recording::start();
        recording::hypothetical(|| {
            let _ = crate::legacy_c::replay_reading_observations(
                SOURCE,
                "f",
                &c,
                None,
                CCompilerDialect::C11,
            );
        });
        recording::unrecorded(|| {
            let _ = crate::legacy_c::replay_reading_observations(
                SOURCE,
                "f",
                &c,
                None,
                CCompilerDialect::C11,
            );
        });
        let discarded = record.finish();
        assert!(discarded.reading_sites.is_empty());
        assert!(discarded.reading_environments.is_empty());
    }
    #[test]
    fn effective_environment_rollback_keeps_only_prior_observation_payloads() {
        let first = contracts();
        let mut second = first.clone();
        second.insert_contract(
            "unrelated_fixture_only",
            ResourceFunctionContract::new(
                Vec::<qcore_schemas::contract::ResourceParameterMode>::new(),
                qcore_schemas::contract::ResourceReturnMode::None,
            ),
        );
        let baseline_recording = recording::Recording::start();
        crate::legacy_c::replay_reading_observations(
            SOURCE,
            "f",
            &first,
            None,
            CCompilerDialect::C11,
        )
        .unwrap();
        let baseline = baseline_recording.finish();
        let recording = recording::Recording::start();
        crate::legacy_c::replay_reading_observations(
            SOURCE,
            "f",
            &first,
            None,
            CCompilerDialect::C11,
        )
        .unwrap();
        let before = recording::reading_checkpoint().unwrap();
        {
            let _rollback = recording::ReadingTransaction::start();
            crate::legacy_c::replay_reading_observations(
                SOURCE,
                "f",
                &second,
                None,
                CCompilerDialect::C11,
            )
            .unwrap();
        }
        assert_eq!(recording::reading_checkpoint().unwrap(), before);
        let kept = recording.finish();
        assert_eq!(kept.reading_sites, baseline.reading_sites);
        assert_eq!(kept.reading_environments, baseline.reading_environments);
        assert!(kept.reading_sites.iter().all(|observation| {
            kept.reading_environments
                .contains_key(&observation.environment_sha256)
        }));
        for observation in &kept.reading_sites {
            validate_observation_environment(observation, &kept.reading_environments).unwrap();
        }
    }

    #[test]
    fn inconsistent_detached_capture_is_omitted_before_any_receipt() {
        let c = contracts();
        let recording = recording::Recording::start();
        crate::legacy_c::replay_reading_observations(SOURCE, "f", &c, None, CCompilerDialect::C11)
            .unwrap();
        let good = recording.finish();
        let observation = good.reading_sites[0].clone();
        let changed = format!(
            "{} ",
            good.reading_environments[&observation.environment_sha256]
        );
        let recording = recording::Recording::start();
        recording::reading_observed(observation, changed);
        let bad = recording.finish();
        assert!(bad.reading_sites.is_empty());
        assert!(bad.reading_environments.is_empty());
        assert_eq!(
            bad.reading_site_omissions["effective-environment-digest-mismatch"],
            1
        );
    }

    #[test]
    fn same_line_duplicate_token_windows_are_omitted_not_guessed() {
        let tokens = crate::legacy_c::tokenize("f(); f();");
        assert!(range(&tokens, &tokens[..4]).is_none());
    }
}
