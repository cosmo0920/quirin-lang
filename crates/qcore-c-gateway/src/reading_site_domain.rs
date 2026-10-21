//! Checked complete *recorded* finite domains. No universal source or C claim.
//! Checked output is sealed: deserialize observations, then replay held inputs.
use crate::{
    contract_dependencies::Recording,
    legacy_c::CCompilerDialect,
    reading_site_evidence::{self as site, Receipt},
};
use qcore_schemas::{
    contract::ResourceContracts,
    hypothesis::{Context, Graph},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Independently selected domain: all successful captures for these owners and
/// declarations in this exact prepared input/environment. Not all relevant C reads.
pub struct HeldDomain<'a> {
    pub source: &'a str,
    pub raw_source: &'a [u8],
    pub raw_sha256: &'a str,
    pub compile_configuration: &'a str,
    pub expected_compile_configuration: &'a str,
    pub entry: &'a str,
    pub contracts: &'a ResourceContracts,
    pub selected: Option<&'a [String]>,
    pub dialect: CCompilerDialect,
    pub graphs: &'a BTreeMap<String, Graph>,
    pub contexts: &'a BTreeMap<String, Context>,
    pub owners: &'a BTreeSet<String>,
    pub declarations: &'a BTreeSet<String>,
    pub expected: &'a [Receipt],
    pub binding_manifest: &'a [u8],
    pub formal_artifacts: &'a BTreeMap<String, Vec<u8>>,
}

/// Source replay and exact finite domain equality, never hypothesis::Evidence.
#[derive(Debug, Serialize)]
pub struct CheckedDomain {
    schema: &'static str,
    claim: &'static str,
    domain_sha256: String,
    receipt_ids: Vec<String>,
    local_predicates: BTreeMap<String, BTreeMap<String, bool>>,
    raw_sha256: String,
    compile_sha256: String,
    universal_disposition: &'static str,
    open_prerequisites: [&'static str; 4],
}
impl CheckedDomain {
    pub fn receipt_ids(&self) -> &[String] {
        &self.receipt_ids
    }
}
fn canonical(receipts: &[Receipt]) -> Result<BTreeSet<String>, String> {
    let set = receipts
        .iter()
        .map(|r| serde_json::to_string(r).map_err(|e| e.to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if set.len() != receipts.len() {
        return Err("local domain: duplicate record".into());
    }
    Ok(set)
}

/// Requires exact caller-held expected domain AND original-reader recollection.
/// A truncated claim cannot define its own denominator. Compilation provenance
/// remains supplied; the production driver must independently prepare raw bytes
/// with the actual held DB command before calling this API.
pub fn check(held: &HeldDomain<'_>, presented: &[Receipt]) -> Result<CheckedDomain, String> {
    if held.expected.is_empty()
        || held.owners.is_empty()
        || held.declarations.is_empty()
        || held.owners.iter().any(|o| !held.contexts.contains_key(o))
        || held.compile_configuration.is_empty()
        || held.compile_configuration != held.expected_compile_configuration
        || qcore_schemas::sha256_hex(held.raw_source) != held.raw_sha256
        || canonical(presented)? != canonical(held.expected)?
    {
        return Err(
            "local domain: empty, truncated, substituted source/configuration or record domain"
                .into(),
        );
    }
    site::resolve_bindings(held.binding_manifest, held.formal_artifacts)?;
    qcore_contract::hypothesis::check_replay(held.contracts.source_assumptions(), held.graphs)?;
    for r in held.expected {
        let o = &r.observation;
        if !held.owners.contains(&o.function) || !held.declarations.contains(&o.declaration) {
            return Err("local domain: unexpected owner/declaration".into());
        }
        let ctx = held
            .contexts
            .get(&o.function)
            .ok_or("local domain: held Context absent")?;
        if ctx.source_identity != qcore_schemas::sha256_hex(held.source.as_bytes())
            || site::bind(
                o,
                ctx,
                held.graphs
                    .get(&o.declaration)
                    .ok_or("local domain: graph absent")?,
            )? != *r
        {
            return Err("local domain: foreign source/context/graph/atom".into());
        }
    }
    let recording = Recording::start();
    crate::legacy_c::replay_reading_observations(
        held.source,
        held.entry,
        held.contracts,
        held.selected,
        held.dialect,
    )
    .map_err(|e| format!("local domain: original reader refused: {e:?}"))?;
    let fresh = recording.finish();
    let mut rebuilt = Vec::new();
    for o in fresh
        .reading_sites
        .iter()
        .filter(|o| held.owners.contains(&o.function) && held.declarations.contains(&o.declaration))
    {
        if !fresh
            .functions
            .get(&o.function)
            .is_some_and(|f| f.found.contains(&o.declaration))
        {
            return Err("local domain: missing actual consultation".into());
        }
        site::validate_observation_environment(o, &fresh.reading_environments)?;
        rebuilt.push(site::bind(
            o,
            &held.contexts[&o.function],
            &held.graphs[&o.declaration],
        )?);
    }
    if canonical(&rebuilt)? != canonical(held.expected)? {
        return Err(format!(
            "local domain: complete fresh recorded domain differs (fresh {}, held {})",
            rebuilt.len(),
            held.expected.len()
        ));
    }
    let mut ids = rebuilt.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    ids.sort();
    Ok(CheckedDomain {
        schema: "libc-local-source-domain.v1",
        claim: "complete-selected-recorded-domain-original-reader-reproduced",
        domain_sha256: qcore_schemas::sha256_hex(
            &serde_json::to_vec(&canonical(&rebuilt)?).map_err(|e| e.to_string())?,
        ),
        receipt_ids: ids,
        local_predicates: rebuilt
            .iter()
            .map(|r| (r.id.clone(), r.observation.checked_predicates.clone()))
            .collect(),
        raw_sha256: held.raw_sha256.into(),
        compile_sha256: qcore_schemas::sha256_hex(held.compile_configuration.as_bytes()),
        universal_disposition: "open",
        open_prerequisites: [
            "all-relevant-C-reading-domain",
            "loaded-contract-mode-truth",
            "boundary28.source-extraction",
            "boundary28.rust-lean-correspondence",
        ],
    })
}
