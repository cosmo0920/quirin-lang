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

//! E03-B-O3 — the task-bounds intersection, measured against real derivations.
//!
//! E03-B recorded that the intersection was "argued and unit-tested, not
//! measured against a real claim", because no derivation was believed to emit
//! any of the three count kinds it takes a minimum of. A census of every
//! production `TaskBounds` construction corrects half of that:
//!
//! | kind | production derivations that state it |
//! |---|---|
//! | `PathBounds::BoundedPaths` | three: `source_effect.rs` (2 for a unit result, 4 otherwise), `call_use_source.rs`, `teardown_source.rs` |
//! | `schedules_explored` | none |
//! | `max_operations` | none |
//!
//! So the `BoundedPaths` branch is exercised here against two *real* derived
//! claims over one imported translation unit, and the other two kinds keep a
//! tripwire that fails the day a derivation first states one.

#[path = "fixtures/boolean_source.rs"]
// The shared fixture also serves test files that use its other helpers.
#[allow(dead_code)]
mod fixture;
use fixture::{CONTRACT as SCALAR_CONTRACT, imports, task};

use qcore_c_gateway::boolean_source::{ChunkSourceResult, ChunkSourceSummary};
use qcore_schemas::chunk_summary::{BooleanFieldContractWitness, BooleanFieldResult};
use qcore_schemas::obligation::PathBounds;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, TargetProfile};
use qcore_verification::source_effect::BooleanFieldSourceTask;
use qcore_verifier_host::{
    BuiltInProvider, ProviderFailure, VerificationOutcome, VerifierProvider, VerifierRegistry,
    compose_verification, compose_verification_with_inventory,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

const UNIT_CONTRACT: &str = "module contract; extern fn reset(ch: borrow<'call, mut, cio_chunk> use one)->unit contract { requires object_is_live; requires exclusive_object_access; requires field_is_boolean; ensures normal field == 1; };";

/// One translation unit with two admitted bodies: a scalar-result reader and a
/// unit-result setter. Both are imported from the same source, so they carry
/// the same `UnitIdentity` and can legitimately be composed.
const SOURCE: &str = "struct cio_chunk { int lock; }; \
int cio_chunk_is_locked(struct cio_chunk *ch) { return ch->lock; } \
void reset(struct cio_chunk *ch) { ch->lock = 1; }";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent composition-bounds test: {reason}");
            false
        }
    }
}

/// A provider that returns the honest witness for the task it is asked about.
/// The core replays it; nothing here decides anything.
struct Echo {
    builtin: BuiltInProvider,
    coverage: BTreeSet<String>,
    witness: SemanticWitness,
}

impl VerifierProvider for Echo {
    fn manifest(&self) -> &PluginManifest {
        self.builtin.manifest()
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        self.builtin.verify(request)
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        Ok(VerificationResponseV2 {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Safe,
            diagnostics: Vec::new(),
            evidence: Some(ProposedEvidenceV2 {
                evidence_version: request.evidence_version,
                unit: request.unit.clone(),
                capability: request.capability,
                coverage: self.coverage.clone(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: Reproducibility {
                    provider_version: self.manifest().package_version.clone(),
                    package_digest: self.manifest().package_digest.clone(),
                    deterministic: true,
                },
                witness: self.witness.clone(),
            }),
        })
    }
}

fn manifest(plugin_id: &str, target: &TargetProfile) -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(plugin_id).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1, 2]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{plugin_id}"),
        targets: vec![SupportedTarget {
            library: target.library.clone(),
            revisions: BTreeSet::from([target.revision.clone()]),
            profiles: BTreeSet::from([target.profile.clone()]),
            dialects: BTreeSet::from([target.dialect.clone()]),
            platforms: BTreeSet::from([target.platform.clone()]),
            backends: BTreeSet::from([target.backend.clone()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits {
            timeout_ms: 1_000,
            memory_bytes: 1 << 20,
            output_bytes: 1 << 16,
            additional: BTreeMap::new(),
        },
        trust_class: PluginTrustClass::ThirdParty,
        certificate_checker: "qcore.boolean_field_contract.v1".into(),
        lean_theorem_family: "QCore.BooleanFieldContract".into(),
    }
}

/// Mints one real certificate through the real firewall: the provider echoes
/// the summary, and `BooleanFieldSourceTask` replays it core-side.
fn mint(
    plugin_id: &str,
    replay: &BooleanFieldSourceTask,
    summary: &ChunkSourceSummary,
) -> VerificationOutcome {
    let witness = SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
        task_digest: replay.task().task_digest(),
        summary: (**summary).clone(),
    }));
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(Echo {
            builtin: BuiltInProvider::new(manifest(plugin_id, &replay.task().target)).unwrap(),
            coverage: replay
                .task()
                .coverage_for(VerifierCapability::ResourceContracts),
            witness,
        }))
        .unwrap();
    let request = replay.request(PluginId::new(plugin_id).unwrap());
    registry.verify_v2(&request, replay.task(), Some(replay))
}

fn summary_of(results: &[ChunkSourceResult], function: &str) -> ChunkSourceSummary {
    results
        .iter()
        .find_map(|result| match result {
            ChunkSourceResult::Summary(summary) if summary.function() == function => {
                Some((**summary).clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{function}` must import as a summary"))
}

#[test]
fn the_bounds_envelope_of_two_real_derivations_is_the_tighter_path_count() {
    if !clang_available() {
        return;
    }
    let results = imports(SOURCE, vec![]);
    let scalar = summary_of(&results, "cio_chunk_is_locked");
    let unit = summary_of(&results, "reset");
    assert_eq!(unit.result_kind, BooleanFieldResult::Unit);
    assert_ne!(scalar.result_kind, BooleanFieldResult::Unit);

    let scalar_task = task(&scalar, SCALAR_CONTRACT);
    let unit_task = task(&unit, UNIT_CONTRACT);
    // Measured, not chosen: these are the bounds the two production
    // derivations state for these two real bodies.
    assert_eq!(
        scalar_task.task().bounds.path_exploration,
        PathBounds::BoundedPaths { paths: 4 }
    );
    assert_eq!(
        unit_task.task().bounds.path_exploration,
        PathBounds::BoundedPaths { paths: 2 }
    );
    // One imported translation unit is one artifact, so the two claims are
    // composable at all.
    assert_eq!(scalar_task.task().unit, unit_task.task().unit);
    assert_ne!(
        scalar_task.task().semantic_profile,
        unit_task.task().semantic_profile
    );

    let outcomes = vec![
        mint("qcore.e03bo3_scalar", &scalar_task, &scalar),
        mint("qcore.e03bo3_unit", &unit_task, &unit),
    ];
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.verdict == ProposedVerdict::Safe),
        "{outcomes:?}"
    );
    let required = scalar_task
        .task()
        .coverage_for(VerifierCapability::ResourceContracts)
        .into_iter()
        .chain(
            unit_task
                .task()
                .coverage_for(VerifierCapability::ResourceContracts),
        )
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 2);

    let composed = compose_verification_with_inventory(outcomes, &required);
    assert_eq!(
        composed.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        composed.diagnostics
    );
    let composite = composed.composite.as_ref().expect("composite published");
    // The envelope is the *weakest* component's path space, so the composite
    // never claims the wider exploration of its stronger component.
    assert_eq!(
        composite.bounds().unwrap().path_exploration,
        PathBounds::BoundedPaths { paths: 2 }
    );
    assert!(!composite.bounds().unwrap().observed_runs_only);
    // Per-obligation bounds keep what each obligation was actually checked
    // under: the intersection is the envelope, never a rewrite of the parts.
    assert_eq!(composite.obligation_bounds().len(), 2);
    let stated = composite
        .obligation_bounds()
        .values()
        .map(|bounds| bounds.path_exploration)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        stated,
        BTreeSet::from([
            PathBounds::BoundedPaths { paths: 2 },
            PathBounds::BoundedPaths { paths: 4 }
        ])
    );
    // And the order of the components does not change the envelope.
    let reversed = compose_verification(vec![
        mint("qcore.e03bo3_unit", &unit_task, &unit),
        mint("qcore.e03bo3_scalar", &scalar_task, &scalar),
    ]);
    assert_eq!(
        reversed.composite.as_ref().unwrap().bounds(),
        composite.bounds()
    );
}

// ---------------------------------------------------------------------------
// The vacuity tripwire for the two kinds no derivation states.
// ---------------------------------------------------------------------------

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every tracked `.rs` file under a `src/` directory of `crates/*` or of
/// `compiler/`. Card W45-TEST-HYGIENE: an untracked or ignored file there is
/// not a production source.
fn production_sources() -> Vec<PathBuf> {
    let root = root();
    tracked_files::tracked_files(&root.join("crates"))
        .into_iter()
        .filter(|path| {
            let relative = path.strip_prefix(root.join("crates")).unwrap();
            relative
                .components()
                .nth(1)
                .is_some_and(|c| c.as_os_str() == "src")
        })
        .chain(tracked_files::tracked_files(&root.join("compiler/src")))
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect()
}

/// E03-B-O3's recorded vacuity, pinned rather than argued.
///
/// `tighter_count` takes the minimum of `schedules_explored` and of
/// `max_operations`, with `None` meaning "not applicable" and never
/// "unbounded". No production derivation states either kind, so those two
/// minima are exercised only by unit tests. **If this test fails, a derivation
/// has started stating one: re-examine the intersection argument in
/// `docs/reports/remaining-e03-b/README.md` §1 against what that derivation
/// means by the count, and record the outcome under E03-B-O3.**
#[test]
fn no_production_derivation_states_a_schedule_or_operation_count() {
    let sources = production_sources();
    assert!(sources.len() > 50, "the census must actually read sources");

    let mut counts = Vec::new();
    let mut bounded_paths = BTreeSet::new();
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("readable source");
        let relative = path
            .strip_prefix(root())
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            if line.contains("schedules_explored: Some") || line.contains("max_operations: Some") {
                counts.push(format!("{relative}: {line}"));
            }
            if line.contains("PathBounds::BoundedPaths") {
                bounded_paths.insert(relative.clone());
            }
        }
    }
    assert!(
        counts.is_empty(),
        "a production site now states a schedule or operation count; E03-B-O3's \
         vacuity argument for that kind must be re-examined: {counts:?}"
    );
    // The census behind this file's header table, pinned so it stays true:
    // five derivations state `BoundedPaths`, and the sixth site is the
    // intersection itself.
    //
    // Deliberate pin updates (cards `P04-B2`, then `P04-COMPOSE`): the symbolic
    // cleanup-replay derivation and then the composition derivation join the
    // list. Both are the same kind of site as the other three — a
    // function-scoped source task whose exact complete-path count is bound into
    // the task; the composition binds its *wrapper's* path count, which is the
    // scope its single obligation is about — so E03-B-O3's vacuity argument for
    // the *count* kinds is unaffected: the load-bearing negative half above,
    // that no production site states `schedules_explored: Some` or
    // `max_operations: Some`, is unchanged and still asserted.
    assert_eq!(
        bounded_paths,
        BTreeSet::from([
            "crates/qcore-verification/src/call_use_source.rs".to_owned(),
            "crates/qcore-verification/src/cleanup_replay_source.rs".to_owned(),
            "crates/qcore-verification/src/composition_source.rs".to_owned(),
            "crates/qcore-verification/src/source_effect.rs".to_owned(),
            "crates/qcore-verification/src/teardown_source.rs".to_owned(),
            "crates/qcore-verifier-host/src/lib.rs".to_owned(),
        ]),
        "the set of production sites naming BoundedPaths moved; re-measure the \
         E03-B-O3 census"
    );
}
