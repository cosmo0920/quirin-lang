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

//! P08 — the *package* half of the bounded five-provider composition audit.
//!
//! All five shipped packages are built once and registered in one registry.
//! Every implemented evidence-v2 capability route is then driven through that
//! registry against every one of the five providers, and the certificates the
//! routes actually mint are handed to the real composition relation.
//!
//! What this file establishes is deliberately narrower than "the five-provider
//! composition works":
//!
//! * exactly four capability routes are implemented — `qcore.chunkio` Boolean
//!   field, `qcore.chunkio` unsigned counter, `qcore.cmetrics` Boolean field,
//!   `qcore.libevent` reactor callback — and each is answered by exactly one
//!   package. Every off-diagonal cell of the 4 x 5 matrix is `QP005`, so no
//!   package can silently substitute for another;
//! * the CTraces and librdkafka slots are **registered and retained**, not
//!   dropped: they answer protocol v1 like the other three, and every
//!   implemented v2 route asked of them is refused with no certificate. No
//!   evidence-v2 task can be derived *for* them at all, which is the core-side
//!   half of P06's and P07's censuses;
//! * multi-provider composition over the real packages is **unreachable in
//!   both directions**. Across capabilities it is blocked by E03-B2
//!   (capability-scoped unit identity); across providers it is blocked by the
//!   core's model-library binding, which ties a Boolean-field task's target
//!   library to the library its source was imported under. Every cross-route
//!   pair therefore refuses with `QP015`, and `QP012`/`QP013` are not reachable
//!   through the shipped packages at all;
//! * registering all five changes no route's verdict, certificate or coverage
//!   denominator against a registry holding only that route's implementer.
//!
//! Nothing is promoted. No raw Cycle 19 unit changes class and no denominator
//! moves; see `docs/reports/remaining-p08/README.md`.
#![cfg(unix)]

#[allow(dead_code)]
#[path = "../../../crates/qcore-verification/tests/fixtures/boolean_source.rs"]
mod boolean;
#[allow(dead_code)]
#[path = "../../../crates/qcore-verification/tests/fixtures/counter_source.rs"]
mod counter;
#[allow(dead_code)]
#[path = "../../../crates/qcore-verification/tests/fixtures/reactor_program.rs"]
mod reactor;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use qcore_c_gateway::boolean_source::{ChunkSourceResult, import_chunk_source_with_profile};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::plugin::{
    NormalizedFact, PluginManifest, TargetProfile, VerificationRequest, VerifierCapability,
};
use qcore_schemas::{
    FactPredicate, PluginId, ProposedVerdict, SemanticWitness, UnitIdentity, VerificationRequestV2,
    VerificationTask,
};
use qcore_verification::reactor_source::{
    REACTOR_SEMANTIC_PROFILE, ReactorCallbackTask, ReactorProgramIdentity, import_reactor_program,
};
use qcore_verification::source_effect::{BooleanFieldSourceTask, semantic_profile_for};
use qcore_verifier_host::{
    CoreCertificate, ProcessOutcome, ProcessProvider, ProviderFailure, UnverifiedProvider,
    VerificationOutcome, VerificationResponse, VerificationResponseV2, VerifierProvider,
    VerifierRegistry, WitnessChecker, compose_certificates, compose_verification,
    compose_verification_with_inventory,
};

/// A deliberately unverified package, wrapped so that it can be registered.
///
/// [`UnverifiedProvider`] is not a [`VerifierProvider`] (card `P01-F3`), so a
/// package nobody verified cannot reach a registry by accident. The cases below
/// need one anyway: they register manifests that are *deliberately* mutated — a
/// protocol version the process does not speak, an unreachable one-millisecond
/// timeout — to drive the registry's own `QP004`/`QP006` gates, and such a
/// manifest cannot bind to the shipped executable's embedded template by
/// construction. Writing this wrapper is the explicit act the type now demands;
/// nothing in production writes one.
struct DeliberatelyUnverified(UnverifiedProvider);

impl VerifierProvider for DeliberatelyUnverified {
    fn manifest(&self) -> &PluginManifest {
        self.0.manifest()
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        self.0.verify(request)
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        self.0.verify_v2(request)
    }
}

/// Builds the wrapper over a mutated manifest, keeping the mutation visible at
/// the call site.
fn deliberately_unverified(
    manifest: PluginManifest,
    package_root: PathBuf,
) -> DeliberatelyUnverified {
    DeliberatelyUnverified(
        UnverifiedProvider::new(manifest, package_root).expect("the manifest is well formed"),
    )
}

/// The five packages `providers/package.py` builds.
const FIVE: [&str; 5] = [
    "qcore.chunkio",
    "qcore.cmetrics",
    "qcore.ctraces",
    "qcore.librdkafka",
    "qcore.libevent",
];

fn id(name: &str) -> PluginId {
    PluginId::new(name).expect("valid plugin id")
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_packages(tag: &str) -> PathBuf {
    let providers = repository_root().join("providers");
    let output = std::env::temp_dir().join(format!("qcore-p08-{tag}-{}", std::process::id()));
    let built = Command::new("python3")
        .arg(providers.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(providers.join("target/conformance-artifacts"))
        .output()
        .expect("package.py runs");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    output
}

fn manifest_of(output: &Path, plugin: &str) -> PluginManifest {
    let bytes = std::fs::read(output.join(plugin).join("verifier-plugin.json"))
        .expect("installed manifest is readable");
    serde_json::from_slice(&bytes).expect("installed manifest parses")
}

fn registry_of(output: &Path, plugins: &[&str]) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    for plugin in plugins {
        registry
            .register(Arc::new(
                ProcessProvider::from_package(output.join(plugin)).expect("package loads"),
            ))
            .expect("package registers");
    }
    registry
}

/// One implemented capability route: the core-derived task, the request that
/// carries it, and the trusted checker that alone can turn a provider proposal
/// into a certificate.
struct Route {
    label: &'static str,
    /// The single package whose declared targets admit this route.
    implementer: &'static str,
    request: VerificationRequestV2,
    task: VerificationTask,
    checker: Box<dyn WitnessChecker>,
}

impl Route {
    fn run(&self, registry: &VerifierRegistry, plugin: &str) -> VerificationOutcome {
        let mut request = self.request.clone();
        request.plugin_id = id(plugin);
        registry.verify_v2(&request, &self.task, Some(self.checker.as_ref()))
    }

    fn required(&self) -> BTreeSet<String> {
        self.task.coverage_for(self.checker.capability())
    }
}

fn boolean_target(summary: &qcore_c_gateway::boolean_source::ChunkSourceSummary) -> TargetProfile {
    TargetProfile {
        library: summary
            .input_identity()
            .model_library()
            .expect("an imported summary names its model library")
            .into(),
        revision: summary.source_revision().into(),
        profile: semantic_profile_for(summary).into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

fn boolean_route(
    label: &'static str,
    implementer: &'static str,
    summary: &qcore_c_gateway::boolean_source::ChunkSourceSummary,
    contract: &str,
) -> Route {
    let task =
        BooleanFieldSourceTask::derive(summary, "fixture.c", contract, boolean_target(summary))
            .expect("the Boolean route derives");
    Route {
        label,
        implementer,
        request: task.request(id(implementer)),
        task: task.task().clone(),
        checker: Box::new(task),
    }
}

/// The real CMetrics unit-result source, imported under the separately
/// configured `cmetrics-counter` profile — the only profile that declares a
/// Boolean field model for that library.
const CMETRICS_SOURCE: &str = "struct cmt_counter {int allow_reset; int untouched;}; void cmt_counter_allow_reset(struct cmt_counter *counter) { if (counter->allow_reset) {return;} counter->allow_reset=1; }";

fn cmetrics_summary() -> qcore_c_gateway::boolean_source::ChunkSourceSummary {
    let root = repository_root();
    let profile = load_profile_directory(&root.join("contract/profiles/cmetrics-counter"))
        .expect("the CMetrics analysis profile loads");
    let mut imports = import_chunk_source_with_profile(
        &C11TranslationUnit {
            source: CMETRICS_SOURCE,
            ..Default::default()
        },
        &profile.pin_for("cmetrics").expect("pinned").git_commit,
        &profile,
        &["cmetrics"],
        &[],
        include_str!(
            "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
        ),
    )
    .expect("the CMetrics unit source imports");
    assert_eq!(imports.len(), 1);
    let ChunkSourceResult::Summary(summary) = imports.remove(0) else {
        panic!("missing unit import")
    };
    *summary
}

fn reactor_route() -> Route {
    let program = reactor::one_shot(1, reactor::key(1), 9);
    let import = import_reactor_program(
        &program,
        &[reactor::TestProfile(true)],
        ReactorProgramIdentity {
            library: "libevent".into(),
            source_revision: "pinned-source".into(),
            build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
            profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
        },
    )
    .expect("the admitted family imports");
    let task = ReactorCallbackTask::derive(
        import,
        "reactor.c",
        TargetProfile {
            library: "libevent".into(),
            revision: "pinned-source".into(),
            profile: REACTOR_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
    )
    .expect("the reactor route derives");
    Route {
        label: "libevent.reactor_callback",
        implementer: "qcore.libevent",
        request: task.request(id("qcore.libevent")),
        task: task.task().clone(),
        checker: Box::new(task),
    }
}

fn counter_route() -> Route {
    let import = counter::imported(counter::SOURCE, vec![]);
    let task = counter::task(&import, counter::CONTRACT);
    Route {
        label: "chunkio.counter",
        implementer: "qcore.chunkio",
        request: task.request(id("qcore.chunkio")),
        task: task.task().clone(),
        checker: Box::new(task),
    }
}

fn certificate(outcome: &VerificationOutcome) -> &CoreCertificate {
    outcome
        .certificate
        .as_ref()
        .unwrap_or_else(|| panic!("expected a certificate: {:?}", outcome.diagnostics))
}

fn has(outcome: &VerificationOutcome, code: &str) -> bool {
    outcome
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn the_five_package_registry_composes_only_what_its_packages_implement() {
    if !counter::clang_available() {
        return;
    }
    let output = build_packages("matrix");
    let registry = registry_of(&output, &FIVE);

    // ---------------------------------------------------------------- routes
    let chunkio_summary = boolean::checked(boolean::SOURCE, vec![]);
    let cmetrics_source = cmetrics_summary();
    let cmetrics_contract = include_str!(
        "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/counter_reset.qv"
    )
    .to_owned();
    let routes = vec![
        boolean_route(
            "chunkio.boolean_field",
            "qcore.chunkio",
            &chunkio_summary,
            boolean::CONTRACT,
        ),
        boolean_route(
            "cmetrics.boolean_field",
            "qcore.cmetrics",
            &cmetrics_source,
            &cmetrics_contract,
        ),
        counter_route(),
        reactor_route(),
    ];

    // ------------------------------------------------- the 4 x 5 answer matrix
    let mut safe_cells = 0;
    for route in &routes {
        for plugin in FIVE {
            let outcome = route.run(&registry, plugin);
            if plugin == route.implementer {
                safe_cells += 1;
                assert_eq!(
                    outcome.verdict,
                    ProposedVerdict::Safe,
                    "{}/{plugin}: {:?}",
                    route.label,
                    outcome.diagnostics
                );
                let certificate = certificate(&outcome);
                assert_eq!(certificate.plugin_id().as_str(), plugin);
                assert_eq!(certificate.task(), Some(&route.task));
                assert_eq!(*certificate.coverage(), route.required());
                // No composed route mints an observation witness, so this
                // composition carries no observation identity at all.
                assert_ne!(
                    certificate.witness().map(SemanticWitness::kind),
                    Some("contract_observation"),
                    "{}",
                    route.label
                );
            } else {
                // Disjoint declared targets: no package can answer another's
                // route, so an absent provider is never silently covered.
                assert_eq!(
                    outcome.verdict,
                    ProposedVerdict::Unknown,
                    "{}/{plugin}",
                    route.label
                );
                assert!(outcome.certificate.is_none(), "{}/{plugin}", route.label);
                assert!(
                    has(&outcome, "QP005"),
                    "{}/{plugin}: {:?}",
                    route.label,
                    outcome.diagnostics
                );
            }
        }
    }
    assert_eq!(
        safe_cells, 4,
        "exactly four of the twenty cells are implemented capabilities"
    );

    // ---------------- the design-only slots are registered, retained, and v1
    // CTraces and librdkafka are in the registry and are really consulted over
    // protocol v1, where they answer Unknown with no evidence — the same answer
    // the other three give on v1. That is what "explicitly retained" means
    // here: they are present and answer, they are not skipped and they cannot
    // stand in for an implementing provider.
    for plugin in FIVE {
        let manifest = manifest_of(&output, plugin);
        let v1 = VerificationRequest {
            protocol_version: 1,
            evidence_version: 1,
            request_id: "p08".into(),
            plugin_id: manifest.plugin_id.clone(),
            capability: VerifierCapability::ResourceContracts,
            target: TargetProfile {
                library: manifest.targets[0].library.clone(),
                revision: "pinned".into(),
                profile: "default".into(),
                dialect: "c11".into(),
                platform: "linux".into(),
                backend: "native".into(),
            },
            unit: UnitIdentity {
                translation_unit: "fixture.c".into(),
                source_digest: "a".repeat(64),
                build_digest: "b".repeat(64),
            },
            facts: vec![NormalizedFact {
                fact_id: "claimed".into(),
                subject: "function".into(),
                predicate: FactPredicate::ResourceBalanced,
                holds: true,
            }],
            assumptions: BTreeMap::new(),
        };
        let outcome = registry.verify(&v1);
        assert_eq!(
            outcome.verdict,
            ProposedVerdict::Unknown,
            "{plugin} must stay v1-noncertifying"
        );
        assert!(outcome.certificate.is_none(), "{plugin}");

        // The same four failure modes, on the only route the two design-only
        // slots have. Each is a distinct fail-closed code; none is a downgrade
        // into some other provider's answer.
        let others = FIVE
            .iter()
            .copied()
            .filter(|other| *other != plugin)
            .collect::<Vec<_>>();
        let absent = registry_of(&output, &others).verify(&v1);
        assert_eq!(absent.verdict, ProposedVerdict::Unknown, "{plugin}");
        assert!(has(&absent, "QP002"), "{plugin}: {:?}", absent.diagnostics);

        let mut off = registry_of(&output, &FIVE);
        off.set_enabled(&id(plugin), false).expect("disables");
        let disabled = off.verify(&v1);
        assert!(
            has(&disabled, "QP003"),
            "{plugin}: {:?}",
            disabled.diagnostics
        );

        let mut older = manifest_of(&output, plugin);
        older.protocol_versions = BTreeSet::from([2]);
        let mut mismatched = VerifierRegistry::default();
        mismatched
            .register(Arc::new(deliberately_unverified(
                older,
                output.join(plugin),
            )))
            .expect("registers");
        let refused = mismatched.verify(&v1);
        assert!(
            has(&refused, "QP004"),
            "{plugin}: {:?}",
            refused.diagnostics
        );

        let mut impatient_manifest = manifest_of(&output, plugin);
        impatient_manifest.limits.timeout_ms = 1;
        let provider = Arc::new(deliberately_unverified(
            impatient_manifest,
            output.join(plugin),
        ));
        let mut impatient = VerifierRegistry::default();
        impatient.register(provider.clone()).expect("registers");
        let timed_out = impatient.verify(&v1);
        assert!(
            has(&timed_out, "QP006"),
            "{plugin}: {:?}",
            timed_out.diagnostics
        );
        assert_eq!(
            provider.0.last_execution().expect("recorded").outcome,
            ProcessOutcome::WallClockTimeout,
            "{plugin}"
        );
    }

    // ------------------------------------------ packaging changes no verdict
    // Each route answers identically through a registry holding only its
    // implementer and through the five-provider registry: same verdict, same
    // certificate, same coverage denominator.
    for route in &routes {
        let alone = registry_of(&output, &[route.implementer]);
        assert_eq!(
            route.run(&alone, route.implementer),
            route.run(&registry, route.implementer),
            "{}: registering the other four changed the answer",
            route.label
        );
    }

    // ------------------------------------------------ source identity across routes
    let answers = routes
        .iter()
        .map(|route| (route.label, route.run(&registry, route.implementer)))
        .collect::<Vec<_>>();
    let units = answers
        .iter()
        .map(|(label, outcome)| (*label, certificate(outcome).unit().clone()))
        .collect::<Vec<_>>();
    for (index, (label, unit)) in units.iter().enumerate() {
        for (other_label, other) in units.iter().skip(index + 1) {
            assert_ne!(
                unit, other,
                "{label} and {other_label} certify the same unit identity"
            );
        }
    }
    // Every route's coverage is exactly the core's own derived denominator, and
    // no two routes share an obligation identifier.
    let mut all = BTreeSet::new();
    for (route, (label, outcome)) in routes.iter().zip(&answers) {
        let coverage = certificate(outcome).coverage().clone();
        assert_eq!(coverage, route.required(), "{label}");
        assert!(!coverage.is_empty(), "{label}");
        for obligation in &coverage {
            assert!(
                all.insert(obligation.clone()),
                "{label} shares {obligation}"
            );
        }
    }

    // ------------------------------------- cross-route composition always refuses
    // Neither direction of multi-provider composition is reachable through the
    // shipped packages: capability-scoped unit identity (E03-B2) separates the
    // two ChunkIO routes, and the core's model-library binding separates the
    // providers. `QP012` and `QP013` are unreachable here for the same reason.
    for (index, (label, outcome)) in answers.iter().enumerate() {
        for (other_label, other) in answers.iter().skip(index + 1) {
            let composed = compose_verification(vec![outcome.clone(), other.clone()]);
            assert_eq!(
                composed.verdict,
                ProposedVerdict::Unknown,
                "{label} + {other_label}"
            );
            assert!(
                has(&composed, "QP015"),
                "{label} + {other_label}: {:?}",
                composed.diagnostics
            );
        }
    }

    // ------------------------------------------- single-route composition is exact
    // The non-vacuity control: composition does accept the certificate a real
    // package minted, against the core's own required set, and a duplicate
    // neither adds nor destroys coverage.
    let (_, chunkio_boolean) = &answers[0];
    let required = routes[0].required();
    assert_eq!(
        compose_verification_with_inventory(vec![chunkio_boolean.clone()], &required).verdict,
        ProposedVerdict::Safe
    );
    let duplicated = vec![chunkio_boolean.clone(), chunkio_boolean.clone()];
    assert_eq!(
        compose_verification_with_inventory(duplicated.clone(), &required).verdict,
        ProposedVerdict::Safe,
        "duplication neither adds nor destroys exact coverage"
    );
    let mut widened = required.clone();
    widened.insert("resource_contracts:never-derived".into());
    assert_eq!(
        compose_verification_with_inventory(duplicated.clone(), &widened).verdict,
        ProposedVerdict::Unknown,
        "two copies of one certificate cannot discharge what it does not cover"
    );
    assert_eq!(
        compose_verification_with_inventory(duplicated, &BTreeSet::new()).verdict,
        ProposedVerdict::Unknown,
        "an empty inventory is not a vacuous success"
    );
    let composite = compose_certificates(std::slice::from_ref(chunkio_boolean)).expect("composes");
    assert_eq!(composite.certificates().len(), 1);

    // --------------------------------------------------- absence, per provider
    for route in &routes {
        let others = FIVE
            .iter()
            .copied()
            .filter(|plugin| *plugin != route.implementer)
            .collect::<Vec<_>>();
        let without = registry_of(&output, &others);
        let absent = route.run(&without, route.implementer);
        assert_eq!(absent.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&absent, "QP002"),
            "{}: {:?}",
            route.label,
            absent.diagnostics
        );
        assert!(absent.certificate.is_none(), "{}", route.label);
        // A missing provider blocks the composite; the four still present
        // never cover for it.
        let blocked = compose_verification(vec![chunkio_boolean.clone(), absent]);
        assert_eq!(blocked.verdict, ProposedVerdict::Unknown);
        assert!(has(&blocked, "QP011"), "{:?}", blocked.diagnostics);
    }

    // --------------------------------------------------- disabled, per provider
    for route in &routes {
        let mut disabled = registry_of(&output, &FIVE);
        disabled
            .set_enabled(&id(route.implementer), false)
            .expect("disables");
        let outcome = route.run(&disabled, route.implementer);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&outcome, "QP003"),
            "{}: {:?}",
            route.label,
            outcome.diagnostics
        );
        assert!(outcome.certificate.is_none(), "{}", route.label);
    }

    // -------------------------------------------- version mismatch, per provider
    // (a) an installed manifest admitting evidence v1 only refuses the v2
    //     request at the manifest gate, before the process is spawned. This is
    //     the shape the *profile-declared* built-in registry is permanently in;
    //     see `compiler/tests/composed_five_providers.rs`.
    for route in &routes {
        let mut manifest = manifest_of(&output, route.implementer);
        manifest.evidence_versions = BTreeSet::from([1]);
        let mut older = VerifierRegistry::default();
        older
            .register(Arc::new(deliberately_unverified(
                manifest,
                output.join(route.implementer),
            )))
            .expect("registers");
        let outcome = route.run(&older, route.implementer);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&outcome, "QP004"),
            "{}: {:?}",
            route.label,
            outcome.diagnostics
        );
        assert!(outcome.certificate.is_none(), "{}", route.label);
    }
    // (b) a transported projection at an unsupported input schema version never
    //     reaches a provider: the request does not validate.
    for (route, key) in [
        (&routes[0], "source_effect"),
        (&routes[1], "source_effect"),
        (&routes[2], "counter_effect"),
        (&routes[3], "reactor_program"),
    ] {
        let mut request = route.request.clone();
        match key {
            "source_effect" => {
                request
                    .source_effect
                    .as_mut()
                    .expect("projection")
                    .schema_version = 0;
            }
            "counter_effect" => {
                request
                    .counter_effect
                    .as_mut()
                    .expect("projection")
                    .schema_version = 0;
            }
            _ => {
                request
                    .reactor_program
                    .as_mut()
                    .expect("projection")
                    .schema_version = 0;
            }
        }
        assert!(request.validate().is_err(), "{}", route.label);
        let outcome = registry.verify_v2(&request, &route.task, Some(route.checker.as_ref()));
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&outcome, "QP001"),
            "{}: {:?}",
            route.label,
            outcome.diagnostics
        );
    }
    // (c) a declared semantic-profile version the package does not serve is
    //     answered Unknown by the process itself, with no certificate. The
    //     ChunkIO package declares v2, v3 and the counter profile on its
    //     `fluent_bit` target; the wildcard `chunkio` target lets the request
    //     through to a process that then declines it.
    let mut mismatched = boolean_target(&chunkio_summary);
    mismatched.profile = "qcore.c_boolean_field_contract.v3".into();
    let mut request = routes[0].request.clone();
    request.target = mismatched.clone();
    let mut retargeted = routes[0].task.clone();
    retargeted.target = mismatched;
    request.task_digest = retargeted.task_digest();
    let outcome = registry.verify_v2(&request, &retargeted, Some(routes[0].checker.as_ref()));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("qcore.chunkio.")),
        "the process itself declined the profile version: {:?}",
        outcome.diagnostics
    );

    // ---------------------------------------------------- timeout, per provider
    for route in &routes {
        let mut manifest = manifest_of(&output, route.implementer);
        manifest.limits.timeout_ms = 1;
        let provider = Arc::new(deliberately_unverified(
            manifest,
            output.join(route.implementer),
        ));
        let mut impatient = VerifierRegistry::default();
        impatient.register(provider.clone()).expect("registers");
        let outcome = route.run(&impatient, route.implementer);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&outcome, "QP006"),
            "{}: {:?}",
            route.label,
            outcome.diagnostics
        );
        assert!(outcome.certificate.is_none(), "{}", route.label);
        assert_eq!(
            provider.0.last_execution().expect("recorded").outcome,
            ProcessOutcome::WallClockTimeout,
            "{}",
            route.label
        );
        // A timed-out component blocks the composite; it is never skipped.
        let composed = compose_verification(vec![chunkio_boolean.clone(), outcome]);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert!(has(&composed, "QP011"), "{:?}", composed.diagnostics);
    }

    // ----------------------------------------- conflicting evidence, per provider
    // (a) a well-formed projection the core never imported is refused by the
    //     host's digest binding; rebinding it to a recomputed task digest is
    //     then refused by the core's own comparison, with no certificate.
    for (route, program) in [
        (
            &routes[0],
            qcore_schemas::chunk_summary::BooleanFieldStatement::Return(
                qcore_schemas::chunk_summary::BooleanFieldExpression::Integer(7),
            ),
        ),
        (
            &routes[1],
            qcore_schemas::chunk_summary::BooleanFieldStatement::ReturnUnit,
        ),
    ] {
        let mut forged = route.request.clone();
        forged.source_effect.as_mut().expect("projection").program = program;
        let refused = registry.verify_v2(&forged, &route.task, Some(route.checker.as_ref()));
        assert_eq!(refused.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(has(&refused, "QP001"), "{}", route.label);
        let mut rebound = route.task.clone();
        let projection = forged.source_effect.as_ref().expect("projection");
        rebound
            .contract_sources
            .insert(projection.digest_key().into(), projection.digest());
        forged.task_digest = rebound.task_digest();
        let refused = registry.verify_v2(&forged, &rebound, Some(route.checker.as_ref()));
        assert_eq!(refused.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&refused, "QP009"),
            "{}: {:?}",
            route.label,
            refused.diagnostics
        );
        assert!(refused.certificate.is_none(), "{}", route.label);
        let blocked = compose_verification(vec![chunkio_boolean.clone(), refused]);
        assert!(has(&blocked, "QP011"), "{:?}", blocked.diagnostics);
    }
    // (b) a witness that reads its own source correctly still cannot buy a
    //     false contract from either Boolean package.
    for (route, summary, contract, mutation) in [
        (
            &routes[0],
            &chunkio_summary,
            boolean::CONTRACT.to_owned(),
            // Card LANDING-REPAIR: the quant migration typed this
            // postcondition as `#exit_1 ... = (#result == #old_lock)`; the
            // legacy spelling no longer occurs, so the rewrite was a no-op.
            ("(#result == #old_lock)", "(#result == 7)"),
        ),
        (
            &routes[1],
            &cmetrics_source,
            cmetrics_contract.clone(),
            // Card LANDING-REPAIR: only the normal exit condition; the
            // entry domain `(#old_field == 0) || (#old_field == 1)` stays.
            ("(#field == 1)", "(#field == 0)"),
        ),
    ] {
        // Card LANDING-REPAIR: a rewrite that matches nothing would test the
        // true contract; each must falsify exactly one normal condition.
        assert_eq!(contract.matches(mutation.0).count(), 1, "{}", route.label);
        let false_contract = BooleanFieldSourceTask::derive(
            summary,
            "fixture.c",
            &contract.replace(mutation.0, mutation.1),
            boolean_target(summary),
        )
        .expect("derives");
        let outcome = registry.verify_v2(
            &false_contract.request(id(route.implementer)),
            false_contract.task(),
            Some(&false_contract),
        );
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{}", route.label);
        assert!(
            has(&outcome, "QP009"),
            "{}: {:?}",
            route.label,
            outcome.diagnostics
        );
        assert!(outcome.certificate.is_none(), "{}", route.label);
    }
    // (c) the same conflict on the counter and reactor routes: a proposal the
    //     process reads faithfully cannot bypass the core's own evaluation.
    let false_counter = counter::task(
        &counter::imported(counter::SOURCE, vec![]),
        &counter::CONTRACT.replace("ensures normal delta == -1;", "ensures normal delta == 1;"),
    );
    let outcome = registry.verify_v2(
        &false_counter.request(id("qcore.chunkio")),
        false_counter.task(),
        Some(&false_counter),
    );
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(has(&outcome, "QP009"), "{:?}", outcome.diagnostics);
    let mut unauthorized = reactor::one_shot(1, reactor::key(1), 9);
    if let qcore_core::event_reactor::EventOperation::Assign { summary, .. } =
        &mut unauthorized.operations[1]
    {
        summary.payload_authority.clear();
    }
    let refused_program = ReactorCallbackTask::derive(
        import_reactor_program(
            &unauthorized,
            &[reactor::TestProfile(true)],
            ReactorProgramIdentity {
                library: "libevent".into(),
                source_revision: "pinned-source".into(),
                build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
                profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
            },
        )
        .expect("imports"),
        "reactor.c",
        routes[3].task.target.clone(),
    )
    .expect("derives");
    assert!(
        refused_program.evaluate().is_err(),
        "the model must refuse it"
    );
    let outcome = registry.verify_v2(
        &refused_program.request(id("qcore.libevent")),
        refused_program.task(),
        Some(&refused_program),
    );
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(has(&outcome, "QP009"), "{:?}", outcome.diagnostics);

    std::fs::remove_dir_all(output).expect("temporary packages are removed");
}

/// The five packages' declared scopes, read from the built manifests. This is
/// the package half of the version table in `docs/reports/remaining-p08`; the
/// profile half is pinned by `compiler/tests/composed_five_providers.rs`.
#[test]
fn the_shipped_packages_declare_disjoint_libraries_and_no_observation_hook() {
    let output = build_packages("manifests");
    let mut libraries = BTreeMap::new();
    let mut versions = BTreeMap::new();
    for plugin in FIVE {
        let manifest = manifest_of(&output, plugin);
        assert_eq!(manifest.plugin_id.as_str(), plugin);
        assert_eq!(
            manifest.evidence_versions,
            BTreeSet::from([1, 2]),
            "{plugin} package admits evidence v1 and v2"
        );
        assert!(
            manifest.required_observation_hooks.is_empty()
                && manifest.filesystem_inputs.is_empty()
                && !manifest.network_access,
            "{plugin}"
        );
        versions.insert(plugin.to_owned(), manifest.package_version.clone());
        libraries.insert(
            plugin.to_owned(),
            manifest
                .targets
                .iter()
                .map(|target| target.library.clone())
                .collect::<BTreeSet<_>>(),
        );
    }
    assert_eq!(
        versions,
        BTreeMap::from([
            // Deliberate pin update: 0.6.0 adds the two teardown-contract
            // capability routes to the same `chunkio` library target, and
            // 0.7.0 adds the call-result def/use route to it, 0.8.0 the
            // symbolic cleanup-replay route, 0.9.0 the unmap-wrapper
            // composition route and 0.10.0 the two `cio_file_close`
            // composition routes.
            ("qcore.chunkio".to_owned(), "0.13.0".to_owned()),
            ("qcore.cmetrics".to_owned(), "0.2.0".to_owned()),
            ("qcore.ctraces".to_owned(), "0.1.0".to_owned()),
            ("qcore.librdkafka".to_owned(), "0.1.0".to_owned()),
            ("qcore.libevent".to_owned(), "0.2.0".to_owned()),
        ]),
        "the shipped package versions this audit measured"
    );
    // Every declared library belongs to exactly one package: no route can be
    // answered by a substitute, so absence is always visible as QP005/QP002.
    let mut seen = BTreeMap::<String, String>::new();
    for (plugin, declared) in &libraries {
        for library in declared {
            assert!(
                seen.insert(library.clone(), plugin.clone()).is_none(),
                "library `{library}` is declared by more than one package"
            );
        }
    }
    assert_eq!(
        seen.len(),
        6,
        "five packages declare six libraries (chunkio also declares fluent_bit)"
    );
    // The two design-only slots declare `resource_contracts` and nothing else,
    // and both still name a certificate checker this repository does not
    // implement (recorded as P06-I4, not repaired here).
    for plugin in ["qcore.ctraces", "qcore.librdkafka"] {
        let manifest = manifest_of(&output, plugin);
        assert!(
            manifest
                .capabilities
                .contains(&VerifierCapability::ResourceContracts)
        );
        assert_eq!(
            manifest.certificate_checker, "qcore.unsupported_assertions.v1",
            "{plugin} names an unimplemented certificate checker (P06-I4)"
        );
    }
    std::fs::remove_dir_all(output).expect("temporary packages are removed");
}
