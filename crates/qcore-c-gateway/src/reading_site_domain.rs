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
    targets: BTreeMap<String, (Context, String)>,
    local_predicates: BTreeMap<String, BTreeMap<String, bool>>,
    raw_sha256: String,
    compile_sha256: String,
    finite_property: &'static str,
    model_sha256: String,
    checker_sha256: String,
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
        targets: rebuilt
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    (r.context.clone(), r.observation.declaration.clone()),
                )
            })
            .collect(),
        local_predicates: rebuilt
            .iter()
            .map(|r| (r.id.clone(), r.observation.checked_predicates.clone()))
            .collect(),
        raw_sha256: held.raw_sha256.into(),
        compile_sha256: qcore_schemas::sha256_hex(held.compile_configuration.as_bytes()),
        finite_property: "libc.recorded-domain.local-accepted-predicates.v1",
        model_sha256: qcore_schemas::sha256_hex(include_bytes!(
            "../../../proof/QCore/LocalSourceBindings.lean"
        )),
        checker_sha256: qcore_schemas::sha256_hex(include_bytes!("reading_site_domain.rs")),
        universal_disposition: "open",
        open_prerequisites: [
            "all-relevant-C-reading-domain",
            "loaded-contract-mode-truth",
            "boundary28.source-extraction",
            "boundary28.rust-lean-correspondence",
        ],
    })
}

/// Explicit records from actual direct/summary consultation closure. These
/// contain no mutable verdict. The caller retains status/property payloads.
pub struct DependentRecord {
    pub id: String,
    pub context: Context,
    pub declarations: BTreeSet<String>,
}
#[derive(Debug, Serialize)]
pub struct ScopedImpact {
    receipt_id: String,
    failure_kind: &'static str,
    affected_records: Vec<String>,
    dependent_other_contexts_unchanged: Vec<String>,
    unrelated_records: Vec<String>,
    verdict_mutations: usize,
    universal_atoms: &'static str,
}
/// Distinguish a source mismatch from a mathematical/guard counterexample.
/// A report of either kind supplies no reachable C violation or Unsafe.
pub enum FailureKind {
    SourceMismatch,
    RefutedPrerequisite,
    FinitePredicateCounterexample,
}
impl CheckedDomain {
    pub fn impact(
        &self,
        receipt_id: &str,
        independently_held_context: &Context,
        kind: FailureKind,
        records: &[DependentRecord],
    ) -> Result<ScopedImpact, String> {
        let (context, declaration) = self
            .targets
            .get(receipt_id)
            .ok_or("local impact: receipt outside checked finite domain")?;
        if context != independently_held_context {
            return Err("local impact: foreign Context".into());
        }
        if records.iter().map(|r| &r.id).collect::<BTreeSet<_>>().len() != records.len() {
            return Err("local impact: duplicate record identities".into());
        }
        let mut affected = Vec::new();
        let mut other = Vec::new();
        let mut unrelated = Vec::new();
        for row in records {
            if row.declarations.contains(declaration) {
                if row.context == *context {
                    affected.push(row.id.clone())
                } else {
                    other.push(row.id.clone())
                }
            } else {
                unrelated.push(row.id.clone())
            }
        }
        Ok(ScopedImpact {
            receipt_id: receipt_id.into(),
            failure_kind: match kind {
                FailureKind::SourceMismatch => "source-mismatch-evidence-refused",
                FailureKind::RefutedPrerequisite => {
                    "hypothetical-refuted-prerequisite-no-C-violation"
                }
                FailureKind::FinitePredicateCounterexample => {
                    "hypothetical-finite-predicate-counterexample-no-C-violation"
                }
            },
            affected_records: affected,
            dependent_other_contexts_unchanged: other,
            unrelated_records: unrelated,
            verdict_mutations: 0,
            universal_atoms: "open",
        })
    }
}
