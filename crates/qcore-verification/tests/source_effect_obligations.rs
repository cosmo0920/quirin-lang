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

//! E01: source-derived obligation completeness for the ChunkIO Boolean-field
//! source-contract capability (`qcore.c_boolean_field_contract.v2`).
//!
//! Every test compares the obligation inventory the pipeline actually derives
//! for a fixture against an inventory hand-enumerated in the fixture table.
//! The property under audit is *inventory completeness*, not source safety:
//! an unsupported construct must leave a residual `Unknown` row carrying a
//! reason, never disappear from the derived inventory.
//!
//! Boundaries these tests pin rather than repair are named in
//! `docs/reports/remaining-e01/README.md`.

#[path = "fixtures/boolean_source.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::boolean_source::{
    ChunkFrameSummary, ChunkSourceResult, ChunkSourceSummary, import_chunk_frame_helper,
    import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    enumerate_source_definitions, legacy_c_source_functions,
};
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::chunk_summary::{
    BooleanFieldContractWitness, BooleanFieldExpression, BooleanFieldStatement,
};
use qcore_schemas::obligation::{ObligationProperty, ObligationScope, ObligationSubject};
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::{
    BuiltInProvider, ProviderFailure, VerifierProvider, VerifierRegistry, WitnessChecker,
    compose_verification_with_inventory,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent source-obligation test: {reason}");
            false
        }
    }
}

const RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);
/// Declared ChunkIO Boolean model in `contract/profiles/fluent-bit`: record
/// `cio_chunk`, field `lock`. `error_n` is the ordinary metadata field a frame
/// helper is allowed to write.
const LAYOUT: &str = "struct cio_chunk { int lock; int error_n; };";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn imports_with(
    unit: &C11TranslationUnit<'_>,
    helpers: &[ChunkFrameSummary],
) -> Result<Vec<ChunkSourceResult>, String> {
    let profile = load_profile_directory(&root().join("contract/profiles/fluent-bit")).unwrap();
    import_chunk_source_with_profile(
        unit,
        &profile.pin_for("chunkio").unwrap().git_commit,
        &profile,
        &["chunkio"],
        helpers,
        RULES,
    )
}

/// The one frame helper the supported call grammar admits.
fn frame_helper() -> ChunkFrameSummary {
    let profile = load_profile_directory(&root().join("contract/profiles/fluent-bit")).unwrap();
    let revision = &profile.pin_for("chunkio").unwrap().git_commit;
    import_chunk_frame_helper(
        &C11TranslationUnit {
            source: &format!(
                "{LAYOUT} void cio_chunk_reset(struct cio_chunk *p) {{ p->error_n = 0; }}"
            ),
            ..Default::default()
        },
        revision,
        revision,
        "cio_chunk_reset",
        RULES,
    )
    .unwrap()
}

/// What the pipeline derived for one source function.
#[derive(Debug)]
enum Derived {
    /// Exactly one `BooleanFieldContract` obligation, with its identifier.
    Contract(String),
    /// A residual row: the function stays in the inventory with a reason.
    Unknown(String),
}

/// Hand-enumerated expectation, written independently of the implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    Contract,
    Unknown,
}

// Inventory fixtures name their source function `run`; the production template
// declares `cio_chunk_is_locked`. Instantiate the template's symbol explicitly
// before requesting an obligation. Production derivation must still refuse a
// contract with no declaration for the requested symbol. These tests inspect
// obligation completeness; they do not claim this getter postcondition holds
// for every mutation of the source body.
fn inventory_task(
    summary: &ChunkSourceSummary,
) -> qcore_verification::source_effect::BooleanFieldSourceTask {
    assert_ne!(summary.function(), "cio_chunk_unlock");
    let contract = CONTRACT.replacen(
        "extern fn cio_chunk_is_locked(",
        &format!("extern fn {}(", summary.function()),
        1,
    );
    task(summary, &contract)
}

/// Derive the obligation inventory the way the audit does, and check the
/// per-function obligation shape while collecting it.
fn inventory(
    unit: &C11TranslationUnit<'_>,
    helpers: &[ChunkFrameSummary],
) -> BTreeMap<String, Derived> {
    let mut derived = BTreeMap::new();
    for result in imports_with(unit, helpers).expect("import must not fail for these fixtures") {
        match result {
            ChunkSourceResult::Summary(summary) => {
                let replay = inventory_task(&summary);
                let obligations = &replay.task().obligations;
                assert_eq!(
                    obligations.len(),
                    1,
                    "the capability derives exactly one function obligation"
                );
                let obligation = &obligations[0];
                assert_eq!(
                    obligation.property,
                    ObligationProperty::BooleanFieldContract
                );
                assert_eq!(obligation.capability, VerifierCapability::ResourceContracts);
                assert_eq!(
                    obligation.subject,
                    ObligationSubject::Function {
                        symbol: summary.function().into(),
                        translation_unit: "fixture.c".into(),
                    }
                );
                assert_eq!(
                    obligation.scope,
                    ObligationScope::Functions {
                        symbols: BTreeSet::from([summary.function().to_owned()]),
                    }
                );
                derived.insert(
                    summary.function().to_owned(),
                    Derived::Contract(obligation.obligation_id.clone()),
                );
            }
            ChunkSourceResult::Unknown {
                function, reason, ..
            } => {
                assert!(!reason.is_empty(), "`{function}` has no residual reason");
                derived.insert(function, Derived::Unknown(reason));
            }
        }
    }
    derived
}

fn source_of(extra: &str, body: &str) -> String {
    format!("{LAYOUT} {extra} int run(struct cio_chunk *p) {{ {body} }}")
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

/// Assert the derived inventory equals the hand-enumerated one, and that every
/// raw source definition the gateway enumerated is accounted for.
fn assert_inventory(
    label: &str,
    source: &str,
    helpers: &[ChunkFrameSummary],
    expected: &[(&str, Expect)],
) -> BTreeMap<String, Derived> {
    let derived = inventory(&unit(source), helpers);
    let hand: BTreeMap<&str, Expect> = expected.iter().copied().collect();
    let observed: BTreeMap<&str, Expect> = derived
        .iter()
        .map(|(name, value)| {
            (
                name.as_str(),
                match value {
                    Derived::Contract(_) => Expect::Contract,
                    Derived::Unknown(_) => Expect::Unknown,
                },
            )
        })
        .collect();
    assert_eq!(observed, hand, "{label}: derived inventory");
    // Unit E01-F1: the denominator this compares against is the *enumerated*
    // definition set, not the raw token scan. The enumeration is a superset of
    // that scan, so this assertion is strictly tighter than the one it replaces.
    let enumerated: BTreeSet<String> = enumerate_source_definitions(&unit(source))
        .definitions()
        .iter()
        .cloned()
        .collect();
    let accounted: BTreeSet<String> = derived.keys().cloned().collect();
    assert_eq!(
        accounted, enumerated,
        "{label}: every enumerated definition must stay in the inventory"
    );
    derived
}

// ---------------------------------------------------------------------------
// Acceptance 1: nested calls in every expression/statement position
// ---------------------------------------------------------------------------

#[test]
fn nested_calls_in_returns_arguments_conditions_and_initializers_stay_unknown() {
    if !clang_available() {
        return;
    }
    let helper = frame_helper();
    let helpers = std::slice::from_ref(&helper);

    // Positive control: the same shape with the call in the one supported
    // position (a whole statement calling a source-validated frame helper).
    let control = source_of(
        "void cio_chunk_reset(struct cio_chunk *);",
        "if (!p->lock) { return -1; } cio_chunk_reset(p); p->lock = 0; return 0;",
    );
    assert_inventory("control", &control, helpers, &[("run", Expect::Contract)]);

    // Every other position: hand-enumerated as a residual Unknown row.
    for (label, extra, body, needs_helper) in [
        (
            "return position",
            "int probe(struct cio_chunk *);",
            "if (!p->lock) { return -1; } return probe(p);",
            false,
        ),
        (
            "argument position",
            "void cio_chunk_reset(struct cio_chunk *); struct cio_chunk *pick(struct cio_chunk *);",
            "cio_chunk_reset(pick(p)); p->lock = 0; return 0;",
            true,
        ),
        (
            "condition position",
            "int probe(struct cio_chunk *);",
            "if (probe(p)) { p->lock = 0; } return p->lock;",
            false,
        ),
        (
            "initializer position",
            "int probe(struct cio_chunk *);",
            "int v = probe(p); p->lock = 0; return v;",
            false,
        ),
        (
            "binary operand position",
            "int probe(struct cio_chunk *);",
            "return p->lock == probe(p);",
            false,
        ),
        (
            "assignment right-hand side",
            "int probe(struct cio_chunk *);",
            "p->lock = probe(p); return 0;",
            false,
        ),
        (
            "discarded call statement",
            "int probe(struct cio_chunk *);",
            "probe(p); p->lock = 0; return 0;",
            false,
        ),
        (
            "nested call inside a supported unary operator",
            "int probe(struct cio_chunk *);",
            "return !probe(p);",
            false,
        ),
    ] {
        let source = source_of(extra, body);
        let derived = assert_inventory(
            label,
            &source,
            if needs_helper { helpers } else { &[] },
            &[("run", Expect::Unknown)],
        );
        let Derived::Unknown(reason) = &derived["run"] else {
            unreachable!()
        };
        assert!(
            reason.contains("CallExpr") || reason.contains("helper") || reason.contains("DeclStmt"),
            "{label}: unexpected residual reason `{reason}`"
        );
    }

    // The initializer position is unsupported with or without a nested call:
    // the rejection is the enclosing declaration statement, so a nested call
    // there is refused by the enclosing form rather than by call analysis.
    let no_call = source_of("", "int v = 1; p->lock = 0; return v;");
    assert_inventory(
        "initializer control without a call",
        &no_call,
        &[],
        &[("run", Expect::Unknown)],
    );

    // The frame-helper importer visits its own statements with the same
    // fail-closed grammar, so an unchecked effect cannot enter the caller's
    // inventory through an admitted helper.
    let profile = load_profile_directory(&root().join("contract/profiles/fluent-bit")).unwrap();
    let revision = &profile.pin_for("chunkio").unwrap().git_commit;
    for body in [
        "p->error_n = probe(p);",
        "probe(p); p->error_n = 0;",
        "int v = 1; p->error_n = v;",
    ] {
        let source = format!(
            "{LAYOUT} int probe(struct cio_chunk *); void cio_chunk_reset(struct cio_chunk *p) {{ {body} }}"
        );
        assert!(
            import_chunk_frame_helper(&unit(&source), revision, revision, "cio_chunk_reset", RULES)
                .is_err(),
            "helper body `{body}` must not import"
        );
    }
}

// ---------------------------------------------------------------------------
// Acceptance 2: indirect calls, unreachable code, scope exclusions
// ---------------------------------------------------------------------------

#[test]
fn indirect_calls_and_callback_fields_stay_unknown() {
    if !clang_available() {
        return;
    }
    // Call through a file-scope function pointer: the callee is an
    // `LValueToRValue` load, not a `FunctionToPointerDecay` of a `FunctionDecl`.
    let indirect = source_of(
        "static void (*hook)(struct cio_chunk *);",
        "hook(p); p->lock = 0; return 0;",
    );
    assert_inventory(
        "file-scope function pointer",
        &indirect,
        &[],
        &[("run", Expect::Unknown)],
    );

    // Call through a record callback field: the callee is a `MemberExpr` and
    // carries no `referencedDecl` name at all.
    let callback = "struct cio_chunk { int lock; int error_n; void (*cb)(struct cio_chunk *); }; \
                    int run(struct cio_chunk *p) { p->cb(p); p->lock = 0; return 0; }";
    assert_inventory(
        "record callback field",
        callback,
        &[],
        &[("run", Expect::Unknown)],
    );
}

#[test]
fn unreachable_statements_are_imported_and_unmodeled_dead_code_fails_closed() {
    if !clang_available() {
        return;
    }
    // A statement after `return` is imported into the checked program — it is
    // visited, not erased — while neither Boolean prestate executes it.
    let source = source_of("", "return p->lock; p->lock = 0;");
    let derived = assert_inventory(
        "dead modeled write",
        &source,
        &[],
        &[("run", Expect::Contract)],
    );
    assert!(matches!(derived["run"], Derived::Contract(_)));
    let mut results = imports_with(&unit(&source), &[]).unwrap();
    let ChunkSourceResult::Summary(summary) = results.remove(0) else {
        panic!("expected a summary")
    };
    assert_eq!(
        summary.source_input().program,
        BooleanFieldStatement::Block(vec![
            BooleanFieldStatement::Return(BooleanFieldExpression::Field),
            BooleanFieldStatement::Assign(BooleanFieldExpression::Integer(0)),
        ]),
        "the unreachable assignment stays in the imported program"
    );
    for path in summary.paths() {
        assert!(
            path.writes.is_empty(),
            "no prestate executes the dead write"
        );
    }

    // Unreachable code the grammar does not model still refuses the function.
    let unmodeled = source_of(
        "int probe(struct cio_chunk *);",
        "return p->lock; probe(p);",
    );
    assert_inventory(
        "dead unmodeled call",
        &unmodeled,
        &[],
        &[("run", Expect::Unknown)],
    );
}

#[test]
fn scope_exclusions_leave_definitions_outside_the_derived_inventory() {
    if !clang_available() {
        return;
    }
    // (a) Header definitions are outside the inventory by construction: the
    // enumeration reads the main source text only.
    let directory = std::env::temp_dir().join(format!("qcore-e01-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let header = directory.join("e01_header.h");
    std::fs::write(
        &header,
        format!("{LAYOUT}\nstatic int header_run(struct cio_chunk *p) {{ return p->lock; }}\n"),
    )
    .unwrap();
    let source = "#include \"e01_header.h\"\nint run(struct cio_chunk *p) { return p->lock; }\n";
    let with_header = C11TranslationUnit {
        source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        ..Default::default()
    };
    let derived = inventory(&with_header, &[]);
    assert_eq!(
        derived.keys().cloned().collect::<Vec<_>>(),
        ["run"],
        "header definitions are outside the audited main-source scope"
    );
    std::fs::remove_dir_all(&directory).unwrap();

    // (b) Boundary E01-B1, *repaired* by unit E01-F1
    // (`docs/reports/remaining-e01-f1/README.md`). This block previously pinned
    // the gap: a definition that exists only after macro expansion, and a K&R
    // definition, were absent from the inventory entirely — no obligation and
    // no residual Unknown row. Both are now counted.
    //
    // The pin is deliberately updated in place rather than deleted, and its
    // load-bearing half is kept: `legacy_c_source_functions` is *still* the raw
    // token scan and still misses both shapes. That function is the
    // enumeration's monotonicity base, not the audited denominator; the
    // denominator is `enumerate_source_definitions`, and that is what the
    // assertions moved to.
    let macro_defined = format!(
        "{LAYOUT}\n#define DEFN(n) int n(struct cio_chunk *p) {{ return p->lock; }}\nDEFN(run)\n"
    );
    assert!(legacy_c_source_functions(&macro_defined).is_empty());
    assert_eq!(
        enumerate_source_definitions(&unit(&macro_defined)).definitions(),
        ["run"]
    );
    // A macro-expanded definition is an ordinary definition once expanded, so
    // the *unmodified* importer reaches it and it carries a real row.
    assert_eq!(
        inventory(&unit(&macro_defined), &[])
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        ["run"],
        "a macro-expanded definition must carry a row, not vanish"
    );

    let knr = format!("{LAYOUT} int run(p) struct cio_chunk *p; {{ return p->lock; }}");
    assert!(legacy_c_source_functions(&knr).is_empty());
    let knr_inventory = enumerate_source_definitions(&unit(&knr));
    assert_eq!(knr_inventory.definitions(), ["run"]);
    // Counted, and counted as unread: this unit completes the denominator, it
    // does not teach any analysis to read a K&R body.
    assert_eq!(knr_inventory.unanalyzable().len(), 1);
    assert_eq!(knr_inventory.unanalyzable()[0].function(), "run");
    assert!(!knr_inventory.supports_unit_safe());
    // It carries a row. Which row is capability-specific: `unanalyzable` names
    // the shapes the *resource* analyzer's body finder cannot read, and this
    // Boolean importer reads Clang's typed AST instead, so it may well derive a
    // contract here. What E01-B1 forbade — no row at all — is what is repaired.
    let knr_row = inventory(&unit(&knr), &[]);
    assert_eq!(knr_row.keys().cloned().collect::<Vec<_>>(), ["run"]);

    // The caller direction is unchanged, and is now demonstrated on a
    // definition that is *in* the denominator rather than absent from it: the
    // enumeration selects the resource route's analyzed set, and a callee whose
    // own effects are unresolved leaves its caller `Unknown` either way.
    let program = "void sink(int *);\n\
                   #define DEFN(n) void n(void) { int *p = 0; sink(p); }\n\
                   DEFN(hidden)\n\
                   int main(void) { hidden(); return 0; }\n";
    let selected = C11TranslationUnit {
        source: program,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    };
    assert_eq!(legacy_c_source_functions(program), ["main"]);
    assert_eq!(
        enumerate_source_definitions(&selected).definitions(),
        ["hidden", "main"],
        "the previously excluded definition is now in the denominator"
    );
    let result = analyze_unrestricted_c11(&selected, "main", &Default::default()).unwrap();
    assert!(
        matches!(result, UnrestrictedLegacyResult::Unknown { .. }),
        "a call into a definition with unresolved effects must stay Unknown"
    );
}

// ---------------------------------------------------------------------------
// Acceptance 2 (second half): a provider cannot remove a requested obligation
// ---------------------------------------------------------------------------

/// Untrusted test provider: proposes `Safe` while reporting a caller-chosen
/// coverage set. It never sees the core's replay inputs.
struct CoverageProvider {
    builtin: BuiltInProvider,
    coverage: BTreeSet<String>,
    witness: SemanticWitness,
}

impl VerifierProvider for CoverageProvider {
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

fn test_manifest(plugin_id: &str, target: &TargetProfile) -> PluginManifest {
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
            // This fixture declares no additional limit kinds, which is what it
            // meant before `PluginLimits::additional` existed. Added here only
            // to repair the E01/P02 merge: E01 introduced this initializer and
            // P02 added the field, and neither side rebuilt against the other.
            additional: BTreeMap::new(),
        },
        trust_class: PluginTrustClass::ThirdParty,
        certificate_checker: "qcore.boolean_field_contract.v1".into(),
        lean_theorem_family: "QCore.BooleanFieldContract".into(),
    }
}

#[test]
fn a_provider_cannot_remove_or_substitute_a_requested_obligation() {
    if !clang_available() {
        return;
    }
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let required = replay
        .task()
        .coverage_for(VerifierCapability::ResourceContracts);
    assert_eq!(required.len(), 1, "one function obligation is requested");
    let honest = SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
        task_digest: replay.task().task_digest(),
        summary: (*summary).clone(),
    }));

    let plugin_id = "qcore.e01_coverage";
    // Every proposal below carries the honest witness; only the reported
    // coverage set changes, so the rejection is attributable to coverage alone.
    for (label, coverage) in [
        ("omitted obligation", BTreeSet::new()),
        (
            "substituted obligation",
            BTreeSet::from([format!(
                "resource_contracts:{}",
                &sha256_hex(b"forged")[..24]
            )]),
        ),
        ("added obligation", {
            let mut extra = required.clone();
            extra.insert("resource_contracts:invented".into());
            extra
        }),
    ] {
        let mut registry = VerifierRegistry::default();
        registry
            .register(Arc::new(CoverageProvider {
                builtin: BuiltInProvider::new(test_manifest(plugin_id, &replay.task().target))
                    .unwrap(),
                coverage,
                witness: honest.clone(),
            }))
            .unwrap();
        let request = replay.request(PluginId::new(plugin_id).unwrap());
        let outcome = registry.verify_v2(&request, replay.task(), Some(&replay));
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{label}");
        assert!(outcome.certificate.is_none(), "{label}");
        assert!(
            outcome.diagnostics.iter().any(|d| d.code == "QP009"),
            "{label}: {:?}",
            outcome.diagnostics
        );
        // The obligation is still required; it was never dropped from the set.
        assert_eq!(
            replay
                .task()
                .coverage_for(VerifierCapability::ResourceContracts),
            required,
            "{label}"
        );
        // Composition over the core inventory keeps the scope unverified.
        let composed = compose_verification_with_inventory(vec![outcome], &required);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown, "{label}");
    }

    // An honest coverage set with the correct witness is the only admitted
    // shape, and the core recomputes the contract itself before accepting it.
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(CoverageProvider {
            builtin: BuiltInProvider::new(test_manifest(plugin_id, &replay.task().target)).unwrap(),
            coverage: required.clone(),
            witness: honest,
        }))
        .unwrap();
    let outcome = registry.verify_v2(
        &replay.request(PluginId::new(plugin_id).unwrap()),
        replay.task(),
        Some(&replay),
    );
    assert_eq!(outcome.verdict, ProposedVerdict::Safe);
    assert_eq!(
        compose_verification_with_inventory(vec![outcome], &required).verdict,
        ProposedVerdict::Safe
    );

    // A provider that simply answers `Unknown` leaves the obligation open too.
    let mut plain = VerifierRegistry::default();
    plain
        .register(Arc::new(
            BuiltInProvider::new(test_manifest("qcore.e01_silent", &replay.task().target)).unwrap(),
        ))
        .unwrap();
    let outcome = plain.verify_v2(
        &replay.request(PluginId::new("qcore.e01_silent").unwrap()),
        replay.task(),
        Some(&replay),
    );
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    let composed = compose_verification_with_inventory(vec![outcome], &required);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
}

// ---------------------------------------------------------------------------
// Acceptance 3: deletion mutations
// ---------------------------------------------------------------------------

#[test]
fn deleting_a_lifecycle_relevant_statement_changes_the_derived_obligation_set() {
    if !clang_available() {
        return;
    }
    let helper = frame_helper();
    let helpers = std::slice::from_ref(&helper);
    let control = source_of(
        "void cio_chunk_reset(struct cio_chunk *);",
        "if (!p->lock) { return -1; } cio_chunk_reset(p); p->lock = 0; return 0;",
    );
    let before = assert_inventory(
        "before deletion",
        &control,
        helpers,
        &[("run", Expect::Contract)],
    );
    let Derived::Contract(before_id) = &before["run"] else {
        unreachable!()
    };

    // (1) Delete the frame-helper call. The function is still checked, but the
    // derived program, its dependencies and the obligation identity all change.
    let without_call = control.replace("cio_chunk_reset(p); ", "");
    let after = assert_inventory(
        "helper call deleted",
        &without_call,
        helpers,
        &[("run", Expect::Contract)],
    );
    let Derived::Contract(after_id) = &after["run"] else {
        unreachable!()
    };
    assert_ne!(
        before_id, after_id,
        "the obligation identity must not survive a source deletion"
    );

    let mut original = imports_with(&unit(&control), helpers).unwrap();
    let ChunkSourceResult::Summary(original) = original.remove(0) else {
        panic!("expected a summary")
    };
    let mut mutated = imports_with(&unit(&without_call), helpers).unwrap();
    let ChunkSourceResult::Summary(mutated) = mutated.remove(0) else {
        panic!("expected a summary")
    };
    assert!(!original.dependencies().is_empty());
    assert!(
        mutated.dependencies().is_empty(),
        "the deleted call must leave no dependency behind"
    );
    assert!(
        original.paths()[1].frame_writes != mutated.paths()[1].frame_writes,
        "the deleted call must change the checked source paths"
    );

    // A stale witness/task pair from before the deletion cannot be replayed
    // against the mutated import, and vice versa.
    let original_replay = inventory_task(&original);
    let mutated_replay = inventory_task(&mutated);
    assert_ne!(
        original_replay.task().task_digest(),
        mutated_replay.task().task_digest()
    );
    let stale = SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
        task_digest: original_replay.task().task_digest(),
        summary: (**original).clone(),
    }));
    assert!(
        mutated_replay.check(mutated_replay.task(), &stale).is_err(),
        "a pre-deletion witness must not discharge the post-deletion obligation"
    );

    // (2) Delete every modeled field access. The function leaves the checked
    // set entirely and reappears as a residual Unknown row, never vanishing.
    let without_field = source_of(
        "void cio_chunk_reset(struct cio_chunk *);",
        "cio_chunk_reset(p); return 0;",
    );
    let derived = assert_inventory(
        "modeled field access deleted",
        &without_field,
        helpers,
        &[("run", Expect::Unknown)],
    );
    let Derived::Unknown(reason) = &derived["run"] else {
        unreachable!()
    };
    assert!(reason.contains("no modeled chunk field access"), "{reason}");
}
