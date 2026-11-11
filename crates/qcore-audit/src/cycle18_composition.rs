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

//! Configured composition replays the original evidence through sealed inputs.
use crate::cycle18_inputs::{
    Failures, ObservedWait, Payload, PreparedAudit, SourceIdentity, Timeout,
};
use qcore_c_gateway::typed_source::SourceSummary;
use qcore_schemas::sha256_hex;
use qcore_verification::cycle18_callback_obligations::{
    CallbackObligationSources, check_cycle18_callback_obligations,
};
use qcore_verification::cycle18_composition::{
    CYCLE18_COMPOSITION_POLICY, Cycle18CompositionInput, Cycle18PreparedSourceIdentity,
    check_cycle18_composition,
};
use qcore_verification::event_payload::{EventPayloadEvidence, declared_event_payload_callback};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    observed_wait: PathBuf,
    wait_failures: PathBuf,
    event_payload: PathBuf,
    timeout_teardown: PathBuf,
    policy: PathBuf,
    report: PathBuf,
}

fn identity(source: &SourceIdentity) -> Cycle18PreparedSourceIdentity {
    Cycle18PreparedSourceIdentity {
        source_revision: source.revision.clone(),
        source_compile_task_sha256: source.compile_task.clone(),
        source_sha256: source.source.clone(),
        profile_sha256: source.profile.clone(),
        build_sha256: source.build.clone(),
    }
}

fn selected<'a>(sources: &'a [SourceSummary], name: &str) -> Result<&'a SourceSummary, String> {
    let matches = sources
        .iter()
        .filter(|source| source.function() == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [source] => Ok(source),
        _ => Err(format!(
            "expected exactly one independently imported {name}"
        )),
    }
}

fn require_publishable_aggregate(
    joined: &BTreeSet<String>,
    refused: &BTreeMap<String, String>,
) -> Result<(), String> {
    let expected = BTreeSet::from([
        "cb_event".to_owned(),
        "cb_timeout".to_owned(),
        "cb_wait_2_timeout".to_owned(),
    ]);
    if joined != &expected || !refused.is_empty() {
        return Err(format!(
            "aggregate publication requires exactly {expected:?} with no refused evidence; joined={joined:?}, refused={refused:?}"
        ));
    }
    Ok(())
}

/// The four freshly prepared leaf audits of one composition configuration.
struct PreparedComposition {
    bytes: Vec<u8>,
    policy_path: PathBuf,
    policy: Vec<u8>,
    report: PathBuf,
    wait: PreparedAudit<ObservedWait>,
    failures: PreparedAudit<Failures>,
    payload: PreparedAudit<Payload>,
    timeout: PreparedAudit<Timeout>,
}

fn prepare_composition(path: &Path) -> Result<PreparedComposition, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let config: Config = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 {
        return Err("unsupported Cycle 18 composition configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |path: &Path| crate::resolve(base, path);
    let policy_path = resolve(&config.policy);
    let policy = fs::read(&policy_path).map_err(|error| error.to_string())?;
    if policy != CYCLE18_COMPOSITION_POLICY.as_bytes() {
        return Err("configured aggregate policy differs from executable bundled policy".into());
    }
    Ok(PreparedComposition {
        wait: crate::observed_wait::prepare_observed_wait(&resolve(&config.observed_wait))?,
        failures: crate::wait_failures::prepare_wait_failures(&resolve(&config.wait_failures))?,
        payload: crate::event_payload::prepare_event_payload(&resolve(&config.event_payload))?,
        timeout: crate::timeout_teardown::prepare_timeout_teardown(&resolve(
            &config.timeout_teardown,
        ))?,
        report: resolve(&config.report),
        bytes,
        policy_path,
        policy,
    })
}

impl PreparedComposition {
    /// Build the aggregate input from the prepared leaves and hand it to `check`.
    fn with_input<R>(
        &self,
        check: impl FnOnce(Cycle18CompositionInput<'_>) -> Result<R, String>,
    ) -> Result<R, String> {
        let failure_identity = identity(&self.failures.source_identity);
        let timeout_identity = identity(&self.timeout.source_identity);
        let callback = declared_event_payload_callback(&self.payload.evidence.authority);
        check(Cycle18CompositionInput {
            callback_summaries: [
                selected(&self.payload.evidence.summaries, "cb_event")?,
                selected(&self.timeout.evidence.summaries, "cb_timeout")?,
                selected(&self.wait.evidence.summaries, "cb_wait_2_timeout")?,
            ],
            wait_source: &self.wait.evidence.projection,
            wait_context: &self.wait.evidence.context,
            wait_observations: &self.wait.evidence.observations,
            wait_bindings: &self.wait.evidence.bindings,
            wait_failure_observations: &self.failures.evidence.observations,
            wait_failure_source: &self.failures.evidence.projection,
            payload_source: &self.payload.evidence.projection,
            payload_context: &self.payload.evidence.context,
            payload_evidence: EventPayloadEvidence {
                observations: &self.payload.evidence.observations,
                bindings: &self.payload.evidence.bindings,
                allocation: &self.payload.evidence.allocation,
            },
            payload_authority: &self.payload.evidence.authority,
            payload_callback: &callback,
            timeout_source: &self.timeout.evidence.projection,
            timeout_evidence: &self.timeout.evidence.evidence,
            wait_failure_identity: &failure_identity,
            timeout_identity: &timeout_identity,
            // No configured production caller inventory has been established.
            caller_summaries: &[],
        })
    }

    fn protected(&self, config: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut protected = vec![config.to_path_buf(), self.policy_path.clone()];
        let mut directories = Vec::new();
        for (files, dirs) in [
            (&self.wait.protected_files, &self.wait.protected_directories),
            (
                &self.failures.protected_files,
                &self.failures.protected_directories,
            ),
            (
                &self.payload.protected_files,
                &self.payload.protected_directories,
            ),
            (
                &self.timeout.protected_files,
                &self.timeout.protected_directories,
            ),
        ] {
            protected.extend(files.iter().cloned());
            directories.extend(dirs.iter().cloned());
        }
        (protected, directories)
    }

    /// Refuse a destination that aliases any leaf or aggregate report or input.
    fn reject_destination(
        &self,
        config: &Path,
        destination: &Path,
        extra_reports: &[&Path],
    ) -> Result<(), String> {
        for leaf_report in [
            self.wait.report(),
            self.failures.report(),
            self.payload.report(),
            self.timeout.report(),
        ]
        .into_iter()
        .chain(extra_reports.iter().copied())
        {
            crate::observed_wait::reject_report_destination_alias(destination, leaf_report)?;
        }
        let (protected, directories) = self.protected(config);
        crate::observed_wait::reject_report_aliases(destination, &protected, &directories)
    }

    fn recheck(&self, config: &Path) -> Result<(), String> {
        self.wait.recheck()?;
        self.failures.recheck()?;
        self.payload.recheck()?;
        self.timeout.recheck()?;
        if fs::read(config).map_err(|error| error.to_string())? != self.bytes
            || fs::read(&self.policy_path).map_err(|error| error.to_string())? != self.policy
        {
            return Err("aggregate configuration or policy changed during checking".into());
        }
        Ok(())
    }
}

pub fn run_cycle18_composition_config(path: &Path) -> Result<(), String> {
    let prepared = prepare_composition(path)?;
    let aggregate = prepared
        .with_input(|input| check_cycle18_composition(input).map_err(|error| error.to_string()))?;
    require_publishable_aggregate(
        &aggregate.joined_evidence_families,
        &aggregate.refused_evidence_families,
    )?;
    let destination = prepared.report.clone();
    prepared.reject_destination(path, &destination, &[])?;
    prepared.recheck(path)?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "cycle18_bounded_composition_checked",
        "aggregate_config_sha256": sha256_hex(&prepared.bytes),
        "aggregate": aggregate,
        "limitations": [
            "Aggregate remains Unknown; bounded evidence is not full callback safety",
            "No complete production caller inventory or universal null/size bound is claimed",
            "Each configured leaf audit was rerun; serialized leaf reports were not consumed",
            "Runtime and linkage evidence remains producer-supplied, not attestation"
        ]
    });
    crate::wait_failures::write_atomic(
        &destination,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObligationsConfig {
    schema_version: u32,
    /// A `cycle18-composition` configuration; its leaves are rerun here.
    composition: PathBuf,
    monkey_profile: PathBuf,
    fluent_checkout: PathBuf,
    monkey_inputs: PathBuf,
    source_compile_database: PathBuf,
    report: PathBuf,
}

const OBLIGATION_SOURCES: [&str; 9] = [
    "cb_event",
    "cb_timeout",
    "cb_wait_2_timeout",
    "_mk_event_init",
    "_mk_event_loop_create",
    "_mk_event_add",
    "_mk_event_update",
    "_mk_event_timeout_create",
    "_mk_event_wait_2",
];

fn same_identity(left: &SourceIdentity, right: &SourceIdentity) -> bool {
    left.revision == right.revision
        && left.compile_task == right.compile_task
        && left.source == right.source
        && left.profile == right.profile
        && left.build == right.build
}

/// Recheck the bounded aggregate and publish the per-callback, per-path
/// obligation table with its source-bound discharges. Every additional
/// summary comes from the same pinned compile task as the aggregate's
/// wait-failure and timeout evidence.
pub fn run_cycle18_obligations_config(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let config: ObligationsConfig =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 {
        return Err("unsupported Cycle 18 obligations configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |path: &Path| crate::resolve(base, path);
    let composition_path = resolve(&config.composition);
    let prepared = prepare_composition(&composition_path)?;
    let fluent_checkout = resolve(&config.fluent_checkout);
    let source = crate::observed_wait_inputs::load_monkey_source(
        &resolve(&config.monkey_profile),
        &fluent_checkout,
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
        &OBLIGATION_SOURCES,
    )?;
    let source_identity = SourceIdentity::from_source(&source)?;
    if !same_identity(&source_identity, &prepared.failures.source_identity)
        || !same_identity(&source_identity, &prepared.timeout.source_identity)
    {
        return Err(
            "obligation sources were not imported from the aggregate's pinned compile task".into(),
        );
    }
    let revision = source.revision.clone();
    let summaries = source.summaries;
    let extra = PreparedAudit::new(
        source_identity,
        (),
        resolve(&config.report),
        Vec::new(),
        source.protected_files,
        source.protected_directories,
    )?
    .with_checkout_revision(fluent_checkout, revision);
    let report = prepared.with_input(|input| {
        check_cycle18_callback_obligations(
            input,
            &CallbackObligationSources {
                callbacks: [
                    selected(&summaries, "cb_event")?,
                    selected(&summaries, "cb_timeout")?,
                    selected(&summaries, "cb_wait_2_timeout")?,
                ],
                event_init: selected(&summaries, "_mk_event_init")?,
                loop_create: selected(&summaries, "_mk_event_loop_create")?,
                event_add: selected(&summaries, "_mk_event_add")?,
                event_update: selected(&summaries, "_mk_event_update")?,
                timeout_create: selected(&summaries, "_mk_event_timeout_create")?,
                wait_2: selected(&summaries, "_mk_event_wait_2")?,
                timeout_projection: &prepared.timeout.evidence.projection,
            },
        )
    })?;
    let destination = resolve(&config.report);
    prepared.reject_destination(&composition_path, &destination, &[&prepared.report])?;
    let (mut protected, mut directories) = prepared.protected(&composition_path);
    protected.push(path.to_path_buf());
    protected.extend(extra.protected_files.iter().cloned());
    directories.extend(extra.protected_directories.iter().cloned());
    crate::observed_wait::reject_report_aliases(&destination, &protected, &directories)?;
    prepared.recheck(&composition_path)?;
    extra.recheck()?;
    if fs::read(path).map_err(|error| error.to_string())? != bytes {
        return Err("obligations configuration changed during checking".into());
    }
    let document = serde_json::json!({
        "schema_version": 1,
        "status": "cycle18_callback_obligations_checked",
        "obligations_config_sha256": sha256_hex(&bytes),
        "aggregate_config_sha256": sha256_hex(&prepared.bytes),
        "obligations": report,
        "limitations": [
            "A callback is composed only when every relevant obligation of every path is discharged; none is",
            "Source facts rest on faithful Clang import of the pinned translation unit under the selected profile",
            "Model proofs check source-derived Cycle 18 event programs; they are not native observations",
            "Joined native families identify presented bytes; they do not attest execution",
            "No complete production caller inventory or universal null/size bound is claimed"
        ]
    });
    crate::wait_failures::write_atomic(
        &destination,
        &serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_evidence_join_refuses_aggregate_publication() {
        let partial = BTreeSet::from(["cb_event".into(), "cb_timeout".into()]);
        assert!(require_publishable_aggregate(&partial, &BTreeMap::new()).is_err());

        let exact = BTreeSet::from([
            "cb_event".into(),
            "cb_timeout".into(),
            "cb_wait_2_timeout".into(),
        ]);
        assert!(require_publishable_aggregate(&exact, &BTreeMap::new()).is_ok());
        assert!(
            require_publishable_aggregate(
                &exact,
                &BTreeMap::from([("timeout".into(), "refused".into())]),
            )
            .is_err()
        );
    }

    #[test]
    fn obligations_configuration_refuses_unknown_fields_and_versions() {
        let valid = serde_json::json!({
            "schema_version": 1,
            "composition": "c.json",
            "monkey_profile": "p",
            "fluent_checkout": "f",
            "monkey_inputs": "m.json",
            "source_compile_database": "d.json",
            "report": "r.json"
        });
        assert!(serde_json::from_value::<ObligationsConfig>(valid.clone()).is_ok());
        let mut extra = valid;
        extra["discharged"] = serde_json::json!(["registration_liveness"]);
        assert!(serde_json::from_value::<ObligationsConfig>(extra).is_err());
    }

    #[test]
    fn identity_join_requires_every_component() {
        let base = SourceIdentity {
            revision: "r".into(),
            compile_task: "c".into(),
            source: "s".into(),
            profile: "p".into(),
            build: "b".into(),
        };
        assert!(same_identity(&base, &base));
        for field in 0..5 {
            let mut changed = SourceIdentity {
                revision: base.revision.clone(),
                compile_task: base.compile_task.clone(),
                source: base.source.clone(),
                profile: base.profile.clone(),
                build: base.build.clone(),
            };
            match field {
                0 => changed.revision.push('x'),
                1 => changed.compile_task.push('x'),
                2 => changed.source.push('x'),
                3 => changed.profile.push('x'),
                _ => changed.build.push('x'),
            }
            assert!(!same_identity(&base, &changed));
        }
    }
}
