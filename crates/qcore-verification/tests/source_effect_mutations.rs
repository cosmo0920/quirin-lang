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

//! E02: the capability-specific mutation matrix for source-bound witness
//! replay and cached summaries, for the ChunkIO Boolean-field source-contract
//! capability (`qcore.c_boolean_field_contract.v2`).
//!
//! Each test is one or more rows of `docs/reports/remaining-e02/matrix.json`.
//! Rows already pinned by E01 (`source_effect_obligations.rs`), G01
//! (`compile_task_identity.rs`) or the pre-existing replay tests
//! (`source_effect.rs`) are cited in that matrix instead of duplicated here.
//!
//! The property under audit is *replay invalidation*: a semantically changed
//! input must retire the task and refuse every witness filed under the old one,
//! and no proposal may be admitted except by the core independently recomputing
//! the Boolean-field paths of its own sealed import.

#[path = "fixtures/boolean_source.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::boolean_source::{
    ChunkSourceResult, ChunkSourceSummary, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, LegacyFunctionScope};
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::chunk_summary::{BooleanFieldContractWitness, ChunkSourceSummaryRecord};
use qcore_schemas::obligation::{Obligation, VERIFICATION_TASK_VERSION, VerificationTask};
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verification::source_effect::BooleanFieldSourceTask;
use qcore_verifier_host::{
    BuiltInProvider, ProviderFailure, VerificationOutcome, VerifierProvider, VerifierRegistry,
    WitnessChecker, compose_verification_with_inventory,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

const RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);

/// Two functions of the declared ChunkIO Boolean model in one translation unit.
/// `read_lock` returns the field; `peek_lock` normalizes it through a branch.
/// Both satisfy `result == old_lock` under `lock_is_boolean`, so neither path
/// set is intrinsically contract-violating — only one of them belongs to each
/// function.
const PAIR_SOURCE: &str = "struct cio_chunk { int lock; };\n\
     int read_lock(struct cio_chunk *ch) { return ch->lock; }\n\
     int peek_lock(struct cio_chunk *ch) { if (ch->lock) { return 1; } return 0; }";

const PAIR_CONTRACT: &str = "module contract;\n\
     extern fn read_lock(chunk: borrow<'call, const, Chunk> use one) -> i32 contract {\n\
       requires chunk_is_live; requires exclusive_chunk_access;\n\
       requires busy_is_boolean; requires lock_is_boolean;\n\
       ensures normal result == old_lock;\n\
       ensures normal lock == old_lock;\n\
       ensures normal busy == old_busy;\n\
     };\n\
     extern fn peek_lock(chunk: borrow<'call, const, Chunk> use one) -> i32 contract {\n\
       requires chunk_is_live; requires exclusive_chunk_access;\n\
       requires busy_is_boolean; requires lock_is_boolean;\n\
       ensures normal result == old_lock;\n\
       ensures normal lock == old_lock;\n\
       ensures normal busy == old_busy;\n\
     };";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Import one translation unit exactly the way the audit does, with an explicit
/// analyzed function scope.
fn import_scoped(source: &str, scope: LegacyFunctionScope) -> Vec<ChunkSourceResult> {
    let profile = load_profile_directory(&root().join("contract/profiles/fluent-bit")).unwrap();
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source,
            function_scope: scope,
            ..Default::default()
        },
        &profile.pin_for("chunkio").unwrap().git_commit,
        &profile,
        &["chunkio"],
        &[],
        RULES,
    )
    .unwrap()
}

fn summaries(source: &str) -> BTreeMap<String, ChunkSourceSummary> {
    import_scoped(source, LegacyFunctionScope::default())
        .into_iter()
        .filter_map(|result| match result {
            ChunkSourceResult::Summary(summary) => Some((summary.function().to_owned(), *summary)),
            ChunkSourceResult::Unknown { .. } => None,
        })
        .collect()
}

fn witness_of(digest: &str, record: &ChunkSourceSummaryRecord) -> SemanticWitness {
    SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
        task_digest: digest.to_owned(),
        summary: record.clone(),
    }))
}

fn honest(replay: &BooleanFieldSourceTask, summary: &ChunkSourceSummary) -> SemanticWitness {
    witness_of(&replay.task().task_digest(), summary)
}

/// What a scripted provider answers for one request.
#[derive(Clone)]
enum Answer {
    /// `Safe` with a caller-chosen coverage set, witness and evidence version.
    Safe {
        coverage: BTreeSet<String>,
        witness: SemanticWitness,
        evidence_version: Option<u16>,
    },
    /// `Safe` with a fully caller-controlled evidence envelope.
    SafeEnvelope(Box<ProposedEvidenceV2>),
    /// `Safe` with the evidence field omitted entirely.
    SafeWithoutEvidence,
    Unknown(Vec<PluginDiagnostic>),
    Unsafe(Vec<PluginDiagnostic>),
    /// A v1 `Safe` with a fully well-formed v1 evidence envelope.
    LegacySafe,
}

/// Untrusted in-process provider. It never sees the core's replay inputs and
/// answers purely from a script keyed by the request's task digest (v2) or
/// request id (v1).
struct ScriptedProvider {
    builtin: BuiltInProvider,
    answers: BTreeMap<String, Answer>,
    fallback: Answer,
}

impl ScriptedProvider {
    fn new(manifest: PluginManifest, fallback: Answer) -> Self {
        Self {
            builtin: BuiltInProvider::new(manifest).unwrap(),
            answers: BTreeMap::new(),
            fallback,
        }
    }

    fn with(mut self, key: &str, answer: Answer) -> Self {
        self.answers.insert(key.to_owned(), answer);
        self
    }

    fn answer(&self, key: &str) -> Answer {
        self.answers
            .get(key)
            .cloned()
            .unwrap_or(self.fallback.clone())
    }
}

impl VerifierProvider for ScriptedProvider {
    fn manifest(&self) -> &PluginManifest {
        self.builtin.manifest()
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        let envelope = |verdict, diagnostics, evidence| VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict,
            diagnostics,
            evidence,
        };
        Ok(match self.answer(&request.request_id) {
            Answer::Unknown(diagnostics) => envelope(ProposedVerdict::Unknown, diagnostics, None),
            Answer::Unsafe(diagnostics) => envelope(ProposedVerdict::Unsafe, diagnostics, None),
            Answer::LegacySafe => envelope(
                ProposedVerdict::Safe,
                Vec::new(),
                Some(ProposedEvidence {
                    evidence_version: request.evidence_version,
                    unit: request.unit.clone(),
                    capability: request.capability,
                    claims: request
                        .facts
                        .iter()
                        .map(|fact| EvidenceClaim {
                            fact_id: fact.fact_id.clone(),
                            subject: fact.subject.clone(),
                            predicate: fact.predicate,
                            holds: fact.holds,
                        })
                        .collect(),
                    coverage: request
                        .facts
                        .iter()
                        .map(|fact| fact.fact_id.clone())
                        .collect(),
                    assumptions_used: request.assumptions.clone(),
                    reproducibility: Reproducibility {
                        provider_version: self.manifest().package_version.clone(),
                        package_digest: self.manifest().package_digest.clone(),
                        deterministic: true,
                    },
                }),
            ),
            // Everything else is not expressible in a v1 frame; fall back to
            // the built-in provider's unchanged v1 behavior.
            _ => self.builtin.verify(request)?,
        })
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        let envelope = |verdict, diagnostics, evidence| VerificationResponseV2 {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict,
            diagnostics,
            evidence,
        };
        Ok(match self.answer(&request.task_digest) {
            Answer::Safe {
                coverage,
                witness,
                evidence_version,
            } => envelope(
                ProposedVerdict::Safe,
                Vec::new(),
                Some(ProposedEvidenceV2 {
                    evidence_version: evidence_version.unwrap_or(request.evidence_version),
                    unit: request.unit.clone(),
                    capability: request.capability,
                    coverage,
                    assumptions_used: request.assumptions.clone(),
                    reproducibility: Reproducibility {
                        provider_version: self.manifest().package_version.clone(),
                        package_digest: self.manifest().package_digest.clone(),
                        deterministic: true,
                    },
                    witness,
                }),
            ),
            Answer::SafeEnvelope(evidence) => {
                envelope(ProposedVerdict::Safe, Vec::new(), Some(*evidence))
            }
            Answer::SafeWithoutEvidence => envelope(ProposedVerdict::Safe, Vec::new(), None),
            Answer::Unsafe(diagnostics) => envelope(ProposedVerdict::Unsafe, diagnostics, None),
            Answer::Unknown(diagnostics) => envelope(ProposedVerdict::Unknown, diagnostics, None),
            Answer::LegacySafe => envelope(ProposedVerdict::Unknown, Vec::new(), None),
        })
    }
}

fn manifest_for(plugin_id: &str, target: &TargetProfile, evidence: &[u16]) -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(plugin_id).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: evidence.iter().copied().collect(),
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
            // This fixture declares no additional limit kinds, which is what it
            // meant before `PluginLimits::additional` existed.
            additional: BTreeMap::new(),
        },
        trust_class: PluginTrustClass::ThirdParty,
        certificate_checker: "qcore.boolean_field_contract.v1".into(),
        lean_theorem_family: "QCore.BooleanFieldContract".into(),
    }
}

const PLUGIN: &str = "qcore.e02_mutation";

fn registry_of(provider: ScriptedProvider) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    registry.register(Arc::new(provider)).unwrap();
    registry
}

/// Dispatch one replay through the real registry with a scripted answer.
fn dispatch(replay: &BooleanFieldSourceTask, answer: Answer) -> VerificationOutcome {
    let manifest = manifest_for(PLUGIN, &replay.task().target, &[1, 2]);
    let provider = ScriptedProvider::new(manifest, Answer::Unknown(Vec::new()))
        .with(&replay.task().task_digest(), answer);
    let registry = registry_of(provider);
    registry.verify_v2(
        &replay.request(PluginId::new(PLUGIN).unwrap()),
        replay.task(),
        Some(replay),
    )
}

fn codes(outcome: &VerificationOutcome) -> Vec<String> {
    outcome
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

fn assert_refused(label: &str, outcome: &VerificationOutcome, code: &str) {
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{label}");
    assert!(outcome.certificate.is_none(), "{label}");
    assert!(
        codes(outcome).iter().any(|actual| actual == code),
        "{label}: expected {code}, got {:?}",
        codes(outcome)
    );
}

// ---------------------------------------------------------------------------
// Row 1e: import coverage (analyzed function scope, imported function set)
// ---------------------------------------------------------------------------

/// The analyzed function scope decides *which* definitions the import covers.
/// Two runs that agree on every checked path must still be two tasks, and a
/// witness filed under one must not be admitted against the other. Adding an
/// unrelated definition to the same translation unit changes the covered set
/// and must retire the surviving function's task as well.
#[test]
fn analyzed_scope_and_imported_function_set_retire_the_task_and_its_witness() {
    let whole_unit = import_scoped(SOURCE, LegacyFunctionScope::PreprocessedTranslationUnit);
    let main_only = import_scoped(SOURCE, LegacyFunctionScope::MainSourceDefinitions);
    let unwrap = |mut results: Vec<ChunkSourceResult>| {
        assert_eq!(results.len(), 1);
        let ChunkSourceResult::Summary(summary) = results.remove(0) else {
            panic!("expected a checked import");
        };
        *summary
    };
    let whole_unit = unwrap(whole_unit);
    let main_only = unwrap(main_only);
    assert_eq!(
        *whole_unit, *main_only,
        "the schema record is identical; only the import coverage differs"
    );
    assert_ne!(
        whole_unit.input_identity().configuration_digest(),
        main_only.input_identity().configuration_digest(),
        "the analyzed function scope is part of the configuration identity"
    );

    let a = task(&whole_unit, CONTRACT);
    let b = task(&main_only, CONTRACT);
    assert_ne!(a.task().task_digest(), b.task().task_digest());
    assert!(
        a.check(a.task(), &honest(&b, &main_only)).is_err(),
        "a witness for the other analyzed scope was admitted"
    );
    assert!(b.check(b.task(), &honest(&a, &whole_unit)).is_err());
    // Separation, not blanket refusal: each task still discharges its own.
    for (replay, summary) in [(&a, &whole_unit), (&b, &main_only)] {
        assert_eq!(
            replay
                .check(replay.task(), &honest(replay, summary))
                .unwrap(),
            replay.task().coverage_for(replay.capability())
        );
    }

    // Widening the covered set with a definition the grammar refuses leaves the
    // checked function's paths untouched but must still retire its task: the
    // whole translation unit is the import's identity.
    let widened =
        format!("{SOURCE}\nint extra(struct cio_chunk *ch) {{ while (ch->lock) {{}} return 0; }}");
    let results = import_scoped(&widened, LegacyFunctionScope::default());
    assert_eq!(
        results.len(),
        2,
        "the refused definition stays in the inventory"
    );
    let Some(ChunkSourceResult::Summary(widened_summary)) = results
        .into_iter()
        .find(|result| matches!(result, ChunkSourceResult::Summary(_)))
    else {
        panic!("expected the checked function to survive");
    };
    let widened_summary = *widened_summary;
    assert_eq!(widened_summary.paths, whole_unit.paths);
    let c = task(&widened_summary, CONTRACT);
    assert_ne!(a.task().task_digest(), c.task().task_digest());
    assert!(a.check(a.task(), &honest(&c, &widened_summary)).is_err());
    assert!(c.check(c.task(), &honest(&a, &whole_unit)).is_err());
}

// ---------------------------------------------------------------------------
// Row 1f / 2f: checker and schema version fields; unsupported evidence versions
// ---------------------------------------------------------------------------

/// Every version field that identifies the checker or the schema is part of
/// the admitted shape. None of them may be substituted, and an evidence
/// version the request did not negotiate is refused rather than coerced.
#[test]
fn checker_and_schema_version_fields_are_bound_and_unsupported_versions_reject() {
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let plugin = PluginId::new(PLUGIN).unwrap();
    let manifest = manifest_for(PLUGIN, &replay.task().target, &[1, 2]);
    let good = Answer::Safe {
        coverage: replay
            .task()
            .coverage_for(VerifierCapability::ResourceContracts),
        witness: honest(&replay, &summary),
        evidence_version: None,
    };
    let registry = registry_of(
        ScriptedProvider::new(manifest.clone(), Answer::Unknown(Vec::new()))
            .with(&replay.task().task_digest(), good.clone()),
    );

    // Control: the unmutated shape certifies.
    let control = registry.verify_v2(
        &replay.request(plugin.clone()),
        replay.task(),
        Some(&replay),
    );
    assert_eq!(control.verdict, ProposedVerdict::Safe);

    // The task's own version fields.
    for (label, mutate) in [
        (
            "task_version",
            (|task: &mut VerificationTask| {
                task.task_version = VERIFICATION_TASK_VERSION + 1;
            }) as fn(&mut VerificationTask),
        ),
        ("task evidence_version", |task: &mut VerificationTask| {
            task.evidence_version = 1;
        }),
        (
            "obligation evidence_version",
            |task: &mut VerificationTask| {
                task.obligations[0].evidence_version = 1;
            },
        ),
    ] {
        let mut mutated = replay.task().clone();
        mutate(&mut mutated);
        let mut request = replay.request(plugin.clone());
        request.task_digest = mutated.task_digest();
        request.obligations = mutated.obligations.clone();
        let outcome = registry.verify_v2(&request, &mutated, Some(&replay));
        assert_refused(label, &outcome, "QP001");
    }

    // The request's negotiated evidence version and the source projection's
    // schema version.
    let mut request = replay.request(plugin.clone());
    request.evidence_version = 3;
    assert_refused(
        "request evidence_version 3",
        &registry.verify_v2(&request, replay.task(), Some(&replay)),
        "QP001",
    );
    let mut request = replay.request(plugin.clone());
    request.source_effect.as_mut().unwrap().schema_version = 3;
    assert_refused(
        "source projection schema_version",
        &registry.verify_v2(&request, replay.task(), Some(&replay)),
        "QP001",
    );

    // The proposal's own evidence version, in both directions.
    for version in [1u16, 3] {
        let answer = Answer::Safe {
            coverage: replay
                .task()
                .coverage_for(VerifierCapability::ResourceContracts),
            witness: honest(&replay, &summary),
            evidence_version: Some(version),
        };
        assert_refused(
            &format!("evidence_version {version}"),
            &dispatch(&replay, answer),
            "QP009",
        );
    }

    // A provider that does not declare evidence v2 never reaches the firewall.
    let v1_only = registry_of(
        ScriptedProvider::new(
            manifest_for(PLUGIN, &replay.task().target, &[1]),
            Answer::Unknown(Vec::new()),
        )
        .with(&replay.task().task_digest(), good),
    );
    assert_refused(
        "manifest without evidence v2",
        &v1_only.verify_v2(&replay.request(plugin), replay.task(), Some(&replay)),
        "QP004",
    );
}

// ---------------------------------------------------------------------------
// Row 1a/1d: substituted source projections and declared assumptions
// ---------------------------------------------------------------------------

/// The request's source projection and the proposal's declared assumptions are
/// both bound to the core task. Substituting the raw program of another import
/// of the same function, dropping the projection, or reporting a different
/// assumption set is refused.
#[test]
fn substituted_source_projections_and_declared_assumptions_reject() {
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let plugin = PluginId::new(PLUGIN).unwrap();
    let required = replay
        .task()
        .coverage_for(VerifierCapability::ResourceContracts);
    let manifest = manifest_for(PLUGIN, &replay.task().target, &[1, 2]);
    let registry = registry_of(
        ScriptedProvider::new(manifest.clone(), Answer::Unknown(Vec::new())).with(
            &replay.task().task_digest(),
            Answer::Safe {
                coverage: required.clone(),
                witness: honest(&replay, &summary),
                evidence_version: None,
            },
        ),
    );

    // The same function, imported from a different program. Its projection
    // carries the same symbol, so `validate` admits the shape and the core task
    // binding is what refuses it.
    let other_program = SOURCE.replace("return ch->lock;", "if (ch->lock) { return 1; } return 0;");
    let other = checked(&other_program, vec![]);
    assert_eq!(other.function(), summary.function());
    assert_ne!(
        other.source_input().digest(),
        summary.source_input().digest()
    );
    let mut request = replay.request(plugin.clone());
    request.source_effect = Some(other.source_input());
    assert_refused(
        "substituted source projection",
        &registry.verify_v2(&request, replay.task(), Some(&replay)),
        "QP001",
    );

    // No projection at all under a Boolean-field obligation.
    let mut request = replay.request(plugin.clone());
    request.source_effect = None;
    assert_refused(
        "omitted source projection",
        &registry.verify_v2(&request, replay.task(), Some(&replay)),
        "QP001",
    );

    // Declared assumptions: added, removed and flipped.
    let base = |assumptions: BTreeMap<String, bool>| {
        Answer::SafeEnvelope(Box::new(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: replay.task().unit.clone(),
            capability: VerifierCapability::ResourceContracts,
            coverage: required.clone(),
            assumptions_used: assumptions,
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: honest(&replay, &summary),
        }))
    };
    // Control: the honest assumption set certifies through the same envelope.
    assert_eq!(
        dispatch(&replay, base(replay.task().assumptions.clone())).verdict,
        ProposedVerdict::Safe
    );
    for (label, mutate) in [
        (
            "added assumption",
            (|map: &mut BTreeMap<String, bool>| {
                map.insert("exclusive_chunk_access".into(), true);
            }) as fn(&mut BTreeMap<String, bool>),
        ),
        ("removed assumption", |map: &mut BTreeMap<String, bool>| {
            map.remove("lock_is_boolean");
        }),
        ("flipped assumption", |map: &mut BTreeMap<String, bool>| {
            map.insert("borrowed_parameter_is_live".into(), false);
        }),
    ] {
        let mut assumptions = replay.task().assumptions.clone();
        mutate(&mut assumptions);
        assert_refused(label, &dispatch(&replay, base(assumptions)), "QP009");
    }

    // A nondeterministic proposal is refused whatever it carries.
    let Answer::SafeEnvelope(mut evidence) = base(replay.task().assumptions.clone()) else {
        unreachable!()
    };
    evidence.reproducibility.deterministic = false;
    assert_refused(
        "nondeterministic reproducibility",
        &dispatch(&replay, Answer::SafeEnvelope(evidence)),
        "QP009",
    );
}

// ---------------------------------------------------------------------------
// Row 1g: mismatched, permuted and foreign obligation identifiers
// ---------------------------------------------------------------------------

/// Right count, wrong identifiers; a permuted vector; and identifiers belonging
/// to another function of the same translation unit. None of them may be
/// admitted, and the checker itself refuses a task it did not derive.
#[test]
fn mismatched_permuted_and_foreign_obligation_identifiers_reject() {
    let pair = summaries(PAIR_SOURCE);
    let (read, peek) = (&pair["read_lock"], &pair["peek_lock"]);
    let a = task(read, PAIR_CONTRACT);
    let b = task(peek, PAIR_CONTRACT);
    let id_a = a.task().coverage_for(VerifierCapability::ResourceContracts);
    let id_b = b.task().coverage_for(VerifierCapability::ResourceContracts);
    assert_eq!(id_a.len(), 1);
    assert_eq!(id_b.len(), 1);
    assert_ne!(id_a, id_b, "distinct functions have distinct obligations");

    let plugin = PluginId::new(PLUGIN).unwrap();
    let manifest = manifest_for(PLUGIN, &a.task().target, &[1, 2]);
    let registry = registry_of(
        ScriptedProvider::new(manifest, Answer::Unknown(Vec::new())).with(
            &a.task().task_digest(),
            Answer::Safe {
                coverage: id_a.clone(),
                witness: honest(&a, read),
                evidence_version: None,
            },
        ),
    );

    // Right count, foreign identifier: the other function's obligation.
    let mut request = a.request(plugin.clone());
    request.obligations = b.task().obligations.clone();
    assert_eq!(request.obligations.len(), 1, "the count is right");
    assert_refused(
        "foreign obligation vector",
        &registry.verify_v2(&request, a.task(), Some(&a)),
        "QP001",
    );

    // Right count, invented identifier on an otherwise honest obligation.
    let mut request = a.request(plugin.clone());
    request.obligations[0].obligation_id =
        format!("resource_contracts:{}", &sha256_hex(b"forged")[..24]);
    assert_refused(
        "substituted obligation identifier",
        &registry.verify_v2(&request, a.task(), Some(&a)),
        "QP001",
    );

    // A two-obligation vector, and its permutation. Neither shape is derivable
    // for this capability, and neither may be admitted.
    let mut widened = a.task().clone();
    widened.obligations.push(b.task().obligations[0].clone());
    assert!(
        widened.validate().is_ok(),
        "the widened task is well formed"
    );
    let permuted = {
        let mut permuted = widened.clone();
        permuted.obligations.swap(0, 1);
        permuted
    };
    for (label, forged) in [("widened", &widened), ("permuted", &permuted)] {
        let mut request = a.request(plugin.clone());
        request.task_digest = forged.task_digest();
        request.obligations = forged.obligations.clone();
        assert_refused(
            &format!("{label} obligation vector"),
            &registry.verify_v2(&request, forged, Some(&a)),
            "QP001",
        );
        // Independently of the request frame, the core checker refuses to
        // replay against a task it did not derive.
        assert!(
            a.check(forged, &honest(&a, read)).is_err(),
            "{label}: the checker replayed a task it did not derive"
        );
        assert!(
            !forged
                .coverage_for(VerifierCapability::ResourceContracts)
                .is_empty()
        );
    }

    // The other function's coverage set with this function's honest witness.
    assert_refused(
        "foreign coverage identifier",
        &dispatch(
            &a,
            Answer::Safe {
                coverage: id_b.clone(),
                witness: honest(&a, read),
                evidence_version: None,
            },
        ),
        "QP009",
    );
    // The obligation was never removed from the core's inventory.
    assert_eq!(
        a.task().coverage_for(VerifierCapability::ResourceContracts),
        id_a
    );
}

// ---------------------------------------------------------------------------
// Row 2a/2b/2c: empty, omitted and duplicated evidence
// ---------------------------------------------------------------------------

/// One witness discharges one obligation. Duplicating it — inside a single
/// proposal's coverage set, or by submitting the same certificate twice at
/// composition — never covers a second obligation.
#[test]
fn a_duplicated_witness_never_covers_a_second_obligation() {
    let pair = summaries(PAIR_SOURCE);
    let (read, peek) = (&pair["read_lock"], &pair["peek_lock"]);
    let a = task(read, PAIR_CONTRACT);
    let b = task(peek, PAIR_CONTRACT);
    let id_a = a.task().coverage_for(VerifierCapability::ResourceContracts);
    let id_b = b.task().coverage_for(VerifierCapability::ResourceContracts);
    let required = id_a.union(&id_b).cloned().collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 2);

    let plugin = PluginId::new(PLUGIN).unwrap();
    let manifest = manifest_for(PLUGIN, &a.task().target, &[1, 2]);
    let answer_for = |replay: &BooleanFieldSourceTask, summary: &ChunkSourceSummary| Answer::Safe {
        coverage: replay
            .task()
            .coverage_for(VerifierCapability::ResourceContracts),
        witness: honest(replay, summary),
        evidence_version: None,
    };
    let registry = registry_of(
        ScriptedProvider::new(manifest, Answer::Unknown(Vec::new()))
            .with(&a.task().task_digest(), answer_for(&a, read))
            .with(&b.task().task_digest(), answer_for(&b, peek)),
    );
    let outcome_a = registry.verify_v2(&a.request(plugin.clone()), a.task(), Some(&a));
    let outcome_b = registry.verify_v2(&b.request(plugin.clone()), b.task(), Some(&b));
    assert_eq!(outcome_a.verdict, ProposedVerdict::Safe);
    assert_eq!(outcome_b.verdict, ProposedVerdict::Safe);
    // Control: honest, distinct certificates cover the whole inventory.
    assert_eq!(
        compose_verification_with_inventory(vec![outcome_a.clone(), outcome_b.clone()], &required)
            .verdict,
        ProposedVerdict::Safe
    );

    // Duplication at composition: the same certificate submitted twice covers
    // one obligation, not two.
    let duplicated =
        compose_verification_with_inventory(vec![outcome_a.clone(), outcome_a.clone()], &required);
    assert_eq!(duplicated.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&duplicated), vec!["QP011".to_owned()]);
    assert!(
        duplicated.diagnostics[0]
            .message
            .starts_with("1 core-derived")
    );

    // Omission: one certificate for a two-obligation inventory.
    let omitted = compose_verification_with_inventory(vec![outcome_a.clone()], &required);
    assert_eq!(omitted.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&omitted), vec!["QP011".to_owned()]);

    // Duplication inside one proposal: one witness claiming both identifiers.
    assert_refused(
        "one witness claiming two obligations",
        &dispatch(
            &a,
            Answer::Safe {
                coverage: required.clone(),
                witness: honest(&a, read),
                evidence_version: None,
            },
        ),
        "QP009",
    );

    // Reusing one function's witness for the other function's request.
    let cross = registry_of(
        ScriptedProvider::new(
            manifest_for(PLUGIN, &b.task().target, &[1, 2]),
            Answer::Unknown(Vec::new()),
        )
        .with(
            &b.task().task_digest(),
            Answer::Safe {
                coverage: id_b.clone(),
                witness: honest(&a, read),
                evidence_version: None,
            },
        ),
    );
    let reused = cross.verify_v2(&b.request(plugin), b.task(), Some(&b));
    assert_refused("witness reused across functions", &reused, "QP009");
    assert_eq!(
        compose_verification_with_inventory(vec![outcome_a, reused], &required).verdict,
        ProposedVerdict::Unknown
    );

    // A `Safe` proposal with no evidence at all.
    assert_refused(
        "safe without evidence",
        &dispatch(&a, Answer::SafeWithoutEvidence),
        "QP008",
    );
}

// ---------------------------------------------------------------------------
// Row 2d: forged witness with a recomputed binding
// ---------------------------------------------------------------------------

/// The witness carries no import identity of its own — `BooleanSourceIdentity`
/// is `serde(skip)` — so a forger is free to recompute every field the schema
/// exposes. The outermost binding (the core-computed task digest) plus the
/// core's comparison against its *own* sealed import catch it in both
/// directions.
#[test]
fn a_forged_witness_with_a_recomputed_binding_is_refused_in_both_directions() {
    let pair = summaries(PAIR_SOURCE);
    let (read, peek) = (&pair["read_lock"], &pair["peek_lock"]);
    let a = task(read, PAIR_CONTRACT);
    let b = task(peek, PAIR_CONTRACT);

    // The witness schema exposes only these two fields, so "recomputing inner
    // hashes" can only mean re-pointing the outer digest at a real task.
    let serialized = serde_json::to_value(honest(&a, read)).unwrap();
    let object = serialized.as_object().unwrap();
    assert_eq!(object.len(), 3, "witness_kind, task_digest, summary");
    assert!(!serialized.to_string().contains("configuration_digest"));

    // Splice `peek_lock`'s real, well-formed paths into `read_lock`'s record and
    // re-point the binding at `peek_lock`'s real task.
    let mut spliced = (**read).clone();
    spliced.paths = peek.paths.clone();
    for (label, digest) in [
        ("bound to its own task", a.task().task_digest()),
        ("rebound to the donor task", b.task().task_digest()),
        ("rebound to an invented task", "a".repeat(64)),
    ] {
        let forged = witness_of(&digest, &spliced);
        assert!(
            a.check(a.task(), &forged).is_err(),
            "{label}: admitted against the original task"
        );
        assert!(
            b.check(b.task(), &forged).is_err(),
            "{label}: admitted against the donor task"
        );
        assert_refused(
            label,
            &dispatch(
                &a,
                Answer::Safe {
                    coverage: a.task().coverage_for(VerifierCapability::ResourceContracts),
                    witness: forged,
                    evidence_version: None,
                },
            ),
            "QP009",
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3: semantic witnesses, not echoed labels
// ---------------------------------------------------------------------------

/// A proposal that echoes the contract text, the Lean theorem family and the
/// verdict label but carries no computed Boolean-field paths is refused, and a
/// well-formed path set that belongs to a *different* function is refused even
/// though it satisfies the same contract. Only the core's own recomputation
/// certifies.
#[test]
fn echoed_labels_and_semantically_wrong_paths_leave_the_obligation_unknown() {
    let pair = summaries(PAIR_SOURCE);
    let (read, peek) = (&pair["read_lock"], &pair["peek_lock"]);
    let a = task(read, PAIR_CONTRACT);
    let b = task(peek, PAIR_CONTRACT);
    let required = a.task().coverage_for(VerifierCapability::ResourceContracts);

    // The donor path set is genuinely well formed and genuinely satisfies the
    // same contract: `peek_lock` certifies on its own.
    assert_eq!(b.evaluate().unwrap().checked_states(), 4);
    assert_eq!(
        b.check(b.task(), &honest(&b, peek)).unwrap(),
        b.task().coverage_for(VerifierCapability::ResourceContracts)
    );
    assert_ne!(read.paths, peek.paths, "the two path sets really differ");

    // Semantically wrong, structurally impeccable.
    let mut wrong = (**read).clone();
    wrong.paths = peek.paths.clone();
    let outcome = dispatch(
        &a,
        Answer::Safe {
            coverage: required.clone(),
            witness: witness_of(&a.task().task_digest(), &wrong),
            evidence_version: None,
        },
    );
    assert_refused("well-formed wrong path set", &outcome, "QP009");
    assert_eq!(
        a.task().coverage_for(VerifierCapability::ResourceContracts),
        required,
        "the obligation stays in the core inventory"
    );
    assert_eq!(
        compose_verification_with_inventory(vec![outcome], &required).verdict,
        ProposedVerdict::Unknown
    );

    // Assertion text, theorem names and verdict labels instead of paths. The
    // strings live where a provider can put them — the record's declared
    // assumption map and the response diagnostics — and none of them help.
    let mut echoed = (**read).clone();
    for path in echoed.paths.iter_mut() {
        path.writes.clear();
        path.branches.clear();
        path.frame_writes.clear();
        path.return_value = 0;
        path.final_value = path.initial_value;
    }
    echoed.profile_assumptions.insert(
        "ensures".into(),
        "normal result == old_lock; normal lock == old_lock".into(),
    );
    echoed.profile_assumptions.insert(
        "lean_theorem_family".into(),
        "QCore.BooleanFieldContract".into(),
    );
    echoed
        .profile_assumptions
        .insert("verdict".into(), "Safe".into());
    assert!(
        a.check(a.task(), &witness_of(&a.task().task_digest(), &echoed))
            .is_err()
    );
    let manifest = manifest_for(PLUGIN, &a.task().target, &[1, 2]);
    let mut provider = ScriptedProvider::new(manifest, Answer::Unknown(Vec::new()));
    provider = provider.with(
        &a.task().task_digest(),
        Answer::Safe {
            coverage: required.clone(),
            witness: witness_of(&a.task().task_digest(), &echoed),
            evidence_version: None,
        },
    );
    let registry = registry_of(provider);
    let outcome = registry.verify_v2(
        &a.request(PluginId::new(PLUGIN).unwrap()),
        a.task(),
        Some(&a),
    );
    assert_refused("echoed assertion text", &outcome, "QP009");

    // And the honest witness, with the same core replay, does certify — so the
    // refusals above are attributable to the proposed paths alone.
    assert_eq!(
        dispatch(
            &a,
            Answer::Safe {
                coverage: required.clone(),
                witness: honest(&a, read),
                evidence_version: None,
            },
        )
        .verdict,
        ProposedVerdict::Safe
    );
}

// ---------------------------------------------------------------------------
// Row 2g: legacy v1 evidence stays non-certifying, and stays available
// ---------------------------------------------------------------------------

/// Protocol v1 is preserved, not disabled: an `Unknown` response flows through
/// with the provider's own diagnostics and no injected rejection, an `Unsafe`
/// allegation downgrades (QP010), and a fully well-formed v1 `Safe` envelope is
/// refused at the terminal v1 gate (QP009) rather than promoted or errored.
#[test]
fn legacy_v1_evidence_is_preserved_as_non_certifying() {
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let plugin = PluginId::new(PLUGIN).unwrap();
    let obligation = &replay.task().obligations[0];
    let facts = vec![NormalizedFact {
        fact_id: obligation.obligation_id.clone(),
        subject: obligation.subject.canonical(),
        predicate: FactPredicate::ResourceBalanced,
        holds: true,
    }];
    let request = VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "e02-legacy-v1".into(),
        plugin_id: plugin.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: replay.task().target.clone(),
        unit: replay.task().unit.clone(),
        facts,
        assumptions: replay.task().assumptions.clone(),
    };
    let provider_diagnostic = PluginDiagnostic {
        code: format!("{PLUGIN}.N01"),
        message: "no v1 analysis for this capability".into(),
        fact_id: None,
    };
    let manifest = manifest_for(PLUGIN, &replay.task().target, &[1, 2]);

    // `Unknown` flows verbatim: the provider's diagnostic survives, no QP code
    // is added, and nothing is promoted.
    let registry = registry_of(
        ScriptedProvider::new(manifest.clone(), Answer::Unknown(Vec::new())).with(
            &request.request_id,
            Answer::Unknown(vec![provider_diagnostic.clone()]),
        ),
    );
    let outcome = registry.verify(&request);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics, vec![provider_diagnostic.clone()]);
    assert!(
        !codes(&outcome).iter().any(|code| code.starts_with("QP")),
        "a v1 Unknown was reported as a protocol rejection"
    );

    // A complete, well-formed v1 `Safe` envelope: refused, never certified.
    let registry = registry_of(
        ScriptedProvider::new(manifest.clone(), Answer::Unknown(Vec::new()))
            .with(&request.request_id, Answer::LegacySafe),
    );
    let outcome = registry.verify(&request);
    assert_refused("v1 safe proposal", &outcome, "QP009");
    assert!(
        outcome.diagnostics[0]
            .message
            .contains("protocol v1 lacks core-derived obligations"),
        "{:?}",
        outcome.diagnostics
    );

    // An `Unsafe` allegation downgrades and keeps the provider's diagnostics.
    let registry = registry_of(
        ScriptedProvider::new(manifest, Answer::Unknown(Vec::new())).with(
            &request.request_id,
            Answer::Unsafe(vec![provider_diagnostic.clone()]),
        ),
    );
    let outcome = registry.verify(&request);
    assert_refused("v1 unsafe allegation", &outcome, "QP010");
    assert!(outcome.diagnostics.contains(&provider_diagnostic));

    // v2 on the same task is unaffected by any of the above.
    assert_eq!(
        dispatch(
            &replay,
            Answer::Safe {
                coverage: replay
                    .task()
                    .coverage_for(VerifierCapability::ResourceContracts),
                witness: honest(&replay, &summary),
                evidence_version: None,
            }
        )
        .verdict,
        ProposedVerdict::Safe
    );
}

// ---------------------------------------------------------------------------
// Row 2e: oversized and degenerate evidence
// ---------------------------------------------------------------------------

/// Degenerate and oversized proposals are refused structurally or by
/// comparison. Nothing panics, and nothing is coerced into a shorter or
/// longer shape.
#[test]
fn oversized_and_degenerate_evidence_is_bounded_and_never_panics() {
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let required = replay
        .task()
        .coverage_for(VerifierCapability::ResourceContracts);
    let digest = replay.task().task_digest();

    // The path vector is a fixed pair in the schema: a longer or shorter one is
    // a wire-level rejection, not a truncation.
    let mut wire = serde_json::to_value(honest(&replay, &summary)).unwrap();
    let paths = wire["summary"]["paths"].as_array().unwrap().clone();
    for count in [0usize, 1, 3, 512] {
        let mut oversized = paths.clone();
        oversized.resize(count, paths[0].clone());
        wire["summary"]["paths"] = serde_json::Value::Array(oversized);
        assert!(
            serde_json::from_value::<SemanticWitness>(wire.clone()).is_err(),
            "a {count}-path witness deserialized"
        );
    }
    // An unknown field is refused rather than ignored.
    let mut wire = serde_json::to_value(honest(&replay, &summary)).unwrap();
    wire["summary"]["extra"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<SemanticWitness>(wire).is_err());

    // Oversized in-band vectors: compared, never walked into a panic.
    let mut huge = (*summary).clone();
    huge.paths[0].branches = vec![false; 200_000];
    huge.paths[0].writes = vec![true; 200_000];
    huge.paths[1].frame_writes = (0..50_000).map(|n| (format!("f{n}"), n)).collect();
    assert!(
        replay
            .check(replay.task(), &witness_of(&digest, &huge))
            .is_err()
    );
    assert_refused(
        "oversized path vectors",
        &dispatch(
            &replay,
            Answer::Safe {
                coverage: required.clone(),
                witness: witness_of(&digest, &huge),
                evidence_version: None,
            },
        ),
        "QP009",
    );

    // Zero-length and cleared fields.
    assert!(
        summary.dependencies.is_empty() && !summary.profile_assumptions.is_empty(),
        "fixture shape assumed by the degenerate mutations below"
    );
    for (label, mutate) in [
        (
            "empty task digest",
            (|witness: &mut BooleanFieldContractWitness| {
                witness.task_digest = String::new();
            }) as fn(&mut BooleanFieldContractWitness),
        ),
        (
            "empty function",
            |witness: &mut BooleanFieldContractWitness| {
                witness.summary.function = String::new();
            },
        ),
        (
            "empty revision",
            |witness: &mut BooleanFieldContractWitness| {
                witness.summary.source_revision = String::new();
            },
        ),
        // This fixture calls no frame helper, so its dependency map is already
        // empty; the degenerate mutation is a zero-length entry, not a clear.
        (
            "zero-length dependency entry",
            |witness: &mut BooleanFieldContractWitness| {
                witness
                    .summary
                    .dependencies
                    .insert(String::new(), String::new());
            },
        ),
        (
            "cleared profile assumptions",
            |witness: &mut BooleanFieldContractWitness| {
                witness.summary.profile_assumptions.clear();
            },
        ),
    ] {
        let mut value = BooleanFieldContractWitness {
            task_digest: digest.clone(),
            summary: (*summary).clone(),
        };
        mutate(&mut value);
        let proposal = SemanticWitness::BooleanFieldContract(Box::new(value));
        assert!(
            replay.check(replay.task(), &proposal).is_err(),
            "{label} was admitted"
        );
        assert_refused(
            label,
            &dispatch(
                &replay,
                Answer::Safe {
                    coverage: required.clone(),
                    witness: proposal,
                    evidence_version: None,
                },
            ),
            "QP009",
        );
    }

    // A degenerate coverage set: thousands of invented identifiers around the
    // real one.
    let mut flooded = required.clone();
    for n in 0..5_000u32 {
        flooded.insert(format!(
            "resource_contracts:{}",
            &sha256_hex(&n.to_le_bytes())[..24]
        ));
    }
    assert_refused(
        "flooded coverage set",
        &dispatch(
            &replay,
            Answer::Safe {
                coverage: flooded,
                witness: honest(&replay, &summary),
                evidence_version: None,
            },
        ),
        "QP009",
    );
}

// ---------------------------------------------------------------------------
// Row 4: no summary or evidence cache on this route
// ---------------------------------------------------------------------------

/// There is no result cache anywhere on the ChunkIO Boolean-field route: the
/// importer, the task derivation and the firewall are pure functions of their
/// arguments, re-executed per call. This test pins that property behaviorally,
/// so introducing a digest-keyed cache that ignores a semantic change would
/// fail here. See `docs/reports/remaining-e02/README.md` §4 for the static
/// survey behind it.
#[test]
fn no_summary_or_evidence_cache_survives_a_semantically_changed_input() {
    let changed = format!("/* semantically distinct */{SOURCE}");
    // Interleave the two inputs so a stale entry would surface as an answer
    // that follows the call order rather than the argument.
    let first = checked(SOURCE, vec![]);
    let other = checked(&changed, vec![]);
    let again = checked(SOURCE, vec![]);
    let other_again = checked(&changed, vec![]);
    assert_eq!(first, again, "re-import is deterministic for one input");
    assert_eq!(other, other_again);
    assert_ne!(
        first.input_identity().raw_source_digest(),
        other.input_identity().raw_source_digest()
    );

    // Derivation is keyed by its arguments, not by call order.
    let digests = [&first, &other, &again, &other_again]
        .map(|summary| task(summary, CONTRACT).task().task_digest());
    assert_eq!(digests[0], digests[2]);
    assert_eq!(digests[1], digests[3]);
    assert_ne!(digests[0], digests[1]);

    // A changed contract with an unchanged source is a new task too, and
    // re-deriving the original afterwards returns the original digest.
    let original = task(&first, CONTRACT);
    let altered = task(&first, &format!("// changed\n{CONTRACT}"));
    assert_ne!(original.task().task_digest(), altered.task().task_digest());
    assert_eq!(
        task(&first, CONTRACT).task().task_digest(),
        original.task().task_digest()
    );

    // The firewall keeps no per-digest memory either: one registry admits the
    // honest proposal, then refuses the same witness against the changed task.
    let required = original
        .task()
        .coverage_for(VerifierCapability::ResourceContracts);
    let manifest = manifest_for(PLUGIN, &original.task().target, &[1, 2]);
    let registry = registry_of(
        ScriptedProvider::new(manifest, Answer::Unknown(Vec::new()))
            .with(
                &original.task().task_digest(),
                Answer::Safe {
                    coverage: required.clone(),
                    witness: honest(&original, &first),
                    evidence_version: None,
                },
            )
            .with(
                &altered.task().task_digest(),
                Answer::Safe {
                    coverage: altered
                        .task()
                        .coverage_for(VerifierCapability::ResourceContracts),
                    // A stale witness, filed under the retired task.
                    witness: honest(&original, &first),
                    evidence_version: None,
                },
            ),
    );
    let plugin = PluginId::new(PLUGIN).unwrap();
    assert_eq!(
        registry
            .verify_v2(
                &original.request(plugin.clone()),
                original.task(),
                Some(&original)
            )
            .verdict,
        ProposedVerdict::Safe
    );
    assert_refused(
        "stale witness after a contract change",
        &registry.verify_v2(
            &altered.request(plugin.clone()),
            altered.task(),
            Some(&altered),
        ),
        "QP009",
    );
    // And the original still certifies afterwards: the refusal cached nothing.
    assert_eq!(
        registry
            .verify_v2(&original.request(plugin), original.task(), Some(&original))
            .verdict,
        ProposedVerdict::Safe
    );
}

/// Guard against a future obligation vector that this capability's checker
/// could not attribute: `BooleanFieldSourceTask` returns the *whole* coverage
/// set of the task it validated, which is sound only because its own derivation
/// emits exactly one obligation and it refuses every other task.
#[test]
fn the_capability_derives_exactly_one_obligation_and_refuses_every_other_task() {
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    assert_eq!(replay.task().obligations.len(), 1);
    let mut extended = replay.task().clone();
    let mut extra: Obligation = extended.obligations[0].clone();
    extra.obligation_id = format!("resource_contracts:{}", &sha256_hex(b"second")[..24]);
    extended.obligations.push(extra);
    assert_eq!(
        extended
            .coverage_for(VerifierCapability::ResourceContracts)
            .len(),
        2
    );
    assert!(
        replay.check(&extended, &honest(&replay, &summary)).is_err(),
        "the checker discharged obligations it never derived"
    );
}
