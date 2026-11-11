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

//! W3 derivation determinism and identity (design §5.1).

use super::*;
use qcore_frontend::frontend::parse_source_program;

pub(crate) const LIFECYCLE_UNIT: &str = "\
fn run() -> unit {
    let ctx = cio_ctx_create();
    let st = cio_stream_create(ctx);
    let ch = cio_chunk_open(ctx, st);
    cio_chunk_close(ch);
    cio_stream_destroy(st);
    cio_ctx_destroy(ctx);
    return;
}
";

pub(crate) fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: "source".into(),
        profile: "default".into(),
        dialect: "qcore".into(),
        platform: "linux".into(),
        backend: "native".into(),
    }
}

pub(crate) fn inputs<'a>(program: &'a SourceProgram) -> DerivationInputs<'a> {
    DerivationInputs {
        translation_unit: "unit.qc",
        program,
        target: target(),
        contract_sources: BTreeMap::from([("unit.qc".to_owned(), program.source().to_owned())]),
        required_observation_hooks: BTreeSet::new(),
        assumptions: BTreeMap::new(),
        verifier: None,
    }
}

#[test]
fn lifecycle_derivation_is_deterministic_but_source_import_is_incomplete() {
    let program = parse_source_program(LIFECYCLE_UNIT).expect("fixture parses");
    let first = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    let second = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert_eq!(first, second, "derivation must be deterministic");
    assert_eq!(first.task.task_digest(), second.task.task_digest());
    assert!(
        first
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );
    assert_eq!(first.task.validate(), Ok(()));
    let inventory = first
        .replay
        .lifecycle
        .as_ref()
        .expect("lifecycle inventory");
    assert_eq!(inventory.sites.len(), 6);
    assert_eq!(inventory.contexts.len(), 1);
    assert_eq!(inventory.streams.len(), 1);
    assert_eq!(inventory.chunks.len(), 1);
    // one context + one stream + one chunk + the program-complete obligation
    assert_eq!(first.task.obligations.len(), 4);
    let ids = first
        .task
        .coverage_for(VerifierCapability::ChunkLifecycle)
        .len();
    assert_eq!(ids, 4, "obligation identifiers are unique");
}

#[test]
fn any_identity_change_changes_the_task_digest() {
    let program = parse_source_program(LIFECYCLE_UNIT).expect("fixture parses");
    let base = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle)
        .task
        .task_digest();

    let mut changed = inputs(&program);
    changed.translation_unit = "other.qc";
    assert_ne!(
        base,
        derive_task(&changed, VerifierCapability::ChunkLifecycle)
            .task
            .task_digest()
    );

    let mut changed = inputs(&program);
    changed.target.platform = "windows".into();
    assert_ne!(
        base,
        derive_task(&changed, VerifierCapability::ChunkLifecycle)
            .task
            .task_digest()
    );

    let mut changed = inputs(&program);
    changed.assumptions.insert("callbacks_bounded".into(), true);
    assert_ne!(
        base,
        derive_task(&changed, VerifierCapability::ChunkLifecycle)
            .task
            .task_digest()
    );

    // A different derivation (semantic profile) is a different task.
    assert_ne!(
        base,
        derive_task(&inputs(&program), VerifierCapability::ResourceContracts)
            .task
            .task_digest()
    );

    // A source edit is a different unit.
    let edited = parse_source_program(&LIFECYCLE_UNIT.replace("cio_chunk_close(ch);", ""))
        .expect("edited fixture parses");
    assert_ne!(
        base,
        derive_task(&inputs(&edited), VerifierCapability::ChunkLifecycle)
            .task
            .task_digest()
    );
}

#[test]
fn unclassifiable_constructs_make_the_derivation_incomplete() {
    // Structured control flow exceeds the evidence-v2.0 single-path bound.
    let branching = "\
fn run() -> unit {
    let ctx = cio_ctx_create();
    while ready {
        cio_ctx_destroy(ctx);
    }
    return;
}
";
    let program = parse_source_program(branching).expect("fixture parses");
    let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert!(
        derived
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );

    // An unbound constructor result cannot be numbered by the core.
    let unbound = "\
fn run() -> unit {
    cio_ctx_create();
    return;
}
";
    let program = parse_source_program(unbound).expect("fixture parses");
    let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert!(
        derived
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );

    // A teardown naming something the core never created.
    let dangling = "\
fn run() -> unit {
    cio_ctx_destroy(ctx);
    return;
}
";
    let program = parse_source_program(dangling).expect("fixture parses");
    let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert!(
        derived
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );

    // Lifecycle sites split across functions exceed the order bound.
    let split = "\
fn open() -> unit {
    let ctx = cio_ctx_create();
    return;
}
fn close() -> unit {
    let other = cio_ctx_create();
    cio_ctx_destroy(other);
    return;
}
";
    let program = parse_source_program(split).expect("fixture parses");
    let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert!(
        derived
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );
}

#[test]
fn reserved_capabilities_have_no_derivation() {
    let program = parse_source_program(LIFECYCLE_UNIT).expect("fixture parses");
    for capability in [
        VerifierCapability::EventTraces,
        VerifierCapability::Concurrency,
    ] {
        let derived = derive_task(&inputs(&program), capability);
        assert!(derived.task.is_incomplete_for(capability));
        assert!(derived.task.coverage_for(capability).is_empty());
    }
}

#[test]
fn observation_derivation_requires_a_declared_contract() {
    let source =
        "extern fn resize() -> unit contract { requires size > 0; ensures normal ok == 1; };";
    let program = parse_source_program(source).expect("fixture parses");
    let mut with_hook = inputs(&program);
    with_hook.required_observation_hooks = BTreeSet::from(["resize".to_owned()]);
    let derived = derive_task(&with_hook, VerifierCapability::ObservationHooks);
    assert!(
        !derived
            .task
            .is_incomplete_for(VerifierCapability::ObservationHooks)
    );
    // one ObservationChecked + one ObservationCoverageComplete
    assert_eq!(derived.task.obligations.len(), 2);
    assert_eq!(
        derived.task.assumptions.get("observations_trusted"),
        Some(&true),
        "recorded observations are a declared trusted assumption"
    );
    assert!(derived.task.bounds.observed_runs_only);
    assert_eq!(derived.task.contract_sources.len(), 1);

    let mut missing = inputs(&program);
    missing.required_observation_hooks = BTreeSet::from(["absent".to_owned()]);
    let derived = derive_task(&missing, VerifierCapability::ObservationHooks);
    assert!(
        derived
            .task
            .is_incomplete_for(VerifierCapability::ObservationHooks)
    );
}

#[test]
fn source_import_never_hides_unknown_effects_or_control_flow() {
    for statement in [
        "wrapper(cio_ctx_destroy(ctx));",
        "unknown(ctx);",
        "cio_ctx_destroy(ctx + ctx);",
        "cio_ctx_destroy(ctx, ctx);",
        "return; cio_ctx_destroy(ctx);",
        "let ctx = cio_ctx_create();",
        "ctx = other;",
        "return cio_ctx_destroy(ctx);",
    ] {
        let source =
            format!("fn run() -> unit {{ let ctx = cio_ctx_create(); {statement} return; }}");
        let program = parse_source_program(&source).unwrap();
        let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
        assert!(
            derived.task.derivation_incomplete.len() >= 2,
            "must diagnose specific gap as well as importer boundary: {statement}"
        );
        assert_eq!(
            derived.replay.lifecycle.unwrap().sites.len(),
            1,
            "unsupported expression cannot contribute a cleanup site: {statement}"
        );
    }
}

#[test]
fn duplicate_contract_names_are_not_resolved_by_last_write_wins() {
    let program =
        parse_source_program("extern fn hook() -> unit contract { ensures normal ok; };").unwrap();
    let mut input = inputs(&program);
    input
        .contract_sources
        .insert("other.qcore".into(), program.source().into());
    input.required_observation_hooks.insert("hook".into());
    assert!(
        derive_task(&input, VerifierCapability::ObservationHooks)
            .task
            .is_incomplete_for(VerifierCapability::ObservationHooks)
    );
}

#[test]
fn observation_trust_preserves_explicit_caller_refusal() {
    let program =
        parse_source_program("extern fn resize() -> unit contract { ensures normal ok == 1; };")
            .unwrap();
    for trust in [None, Some(false), Some(true)] {
        let mut input = inputs(&program);
        input.required_observation_hooks.insert("resize".into());
        input.assumptions.insert("other_condition".into(), false);
        if let Some(trust) = trust {
            input
                .assumptions
                .insert("observations_trusted".into(), trust);
        }
        let derived = derive_task(&input, VerifierCapability::ObservationHooks);
        let expected = trust.unwrap_or(true);
        assert_eq!(
            derived.task.assumptions.get("observations_trusted"),
            Some(&expected)
        );
        assert_eq!(
            derived.task.assumptions.get("other_condition"),
            Some(&false)
        );
        assert!(
            derived
                .task
                .obligations
                .iter()
                .all(|o| o.assumptions == derived.task.assumptions)
        );
        assert_eq!(
            derived
                .task
                .is_incomplete_for(VerifierCapability::ObservationHooks),
            !expected
        );
    }
}

// ---------------------------------------------------------------------------
// Card REASON-CODES: the closed L21.1 category carried on every marker.
// ---------------------------------------------------------------------------

/// Every marker this deriver can emit, produced by a real derivation run
/// rather than by quoting the source. Each entry names the site it exercises.
fn emitted_markers() -> Vec<(
    &'static str,
    qcore_schemas::obligation::DerivationIncomplete,
)> {
    let mut rows = Vec::new();
    let mut push = |label: &'static str, derived: DerivedTask| {
        for marker in derived.task.derivation_incomplete {
            rows.push((label, marker));
        }
    };

    // Reserved capabilities: no derivation at all.
    let program = parse_source_program(LIFECYCLE_UNIT).expect("fixture parses");
    push(
        "no-core-derivation",
        derive_task(&inputs(&program), VerifierCapability::EventTraces),
    );
    // Resource contracts: the gateway-owned custody derivation.
    push(
        "gateway-custody",
        derive_task(&inputs(&program), VerifierCapability::ResourceContracts),
    );
    // Chunk lifecycle over a clean unit: the unconditional ABI-import gap.
    push(
        "lifecycle-abi-import",
        derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle),
    );

    // LifecycleScan gaps, one crafted unit per site.
    for (label, source) in [
        (
            "structured-control-flow",
            "fn run() -> unit { if 1 { let c = cio_ctx_create(); } return; }",
        ),
        (
            "operations-after-return",
            "fn run() -> unit { return; let c = cio_ctx_create(); }",
        ),
        (
            "unknown-call-effect",
            "fn run() -> unit { frobnicate(); return; }",
        ),
        (
            "model-call-arity",
            "fn run() -> unit { let c = cio_ctx_create(1); return; }",
        ),
        (
            "unbound-created-identity",
            "fn run() -> unit { cio_ctx_create(); return; }",
        ),
        (
            "unresolved-argument",
            "fn run() -> unit { cio_ctx_destroy(nope); return; }",
        ),
        (
            "cross-function-order-bound",
            "fn a() -> unit { let c = cio_ctx_create(); return; }\nfn b() -> unit { let d = cio_ctx_create(); return; }",
        ),
        (
            "shadowed-identity",
            "fn run() -> unit { let c = cio_ctx_create(); let c = cio_ctx_create(); return; }",
        ),
    ] {
        if let Ok(program) = parse_source_program(source) {
            push(
                label,
                derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle),
            );
        }
    }

    // Observation hooks: the premise sites.
    let program =
        parse_source_program("extern fn resize() -> unit contract { ensures normal ok == 1; };")
            .expect("fixture parses");
    let mut refused = inputs(&program);
    refused.required_observation_hooks = BTreeSet::from(["resize".to_owned()]);
    refused
        .assumptions
        .insert("observations_trusted".into(), false);
    push(
        "refused-observations-trusted",
        derive_task(&refused, VerifierCapability::ObservationHooks),
    );

    let mut absent = inputs(&program);
    absent.required_observation_hooks = BTreeSet::from(["absent".to_owned()]);
    push(
        "missing-extern-hook",
        derive_task(&absent, VerifierCapability::ObservationHooks),
    );

    let mut ambiguous = inputs(&program);
    ambiguous
        .contract_sources
        .insert("other.qcore".into(), program.source().into());
    ambiguous.required_observation_hooks = BTreeSet::from(["resize".to_owned()]);
    push(
        "ambiguous-contract",
        derive_task(&ambiguous, VerifierCapability::ObservationHooks),
    );

    let mut unparseable = inputs(&program);
    unparseable
        .contract_sources
        .insert("broken.qcore".into(), "fn (".into());
    unparseable.required_observation_hooks = BTreeSet::from(["resize".to_owned()]);
    push(
        "unparseable-contract",
        derive_task(&unparseable, VerifierCapability::ObservationHooks),
    );

    rows
}

#[test]
fn every_emitted_marker_agrees_with_the_documented_reason_mapping() {
    let rows = emitted_markers();
    assert!(
        !rows.is_empty(),
        "no marker was produced; the test is vacuous"
    );
    for (label, marker) in &rows {
        let classified = qcore_schemas::reason::ReasonCategory::classify(&marker.reason)
            .unwrap_or_else(|_| {
                panic!(
                    "[{label}] emitted reason classifies to nothing: `{}`",
                    marker.reason
                )
            });
        assert_eq!(
            classified, marker.category,
            "[{label}] the typed site and the documented mapping disagree for `{}`",
            marker.reason
        );
    }
}

#[test]
fn derivation_never_labels_a_gap_an_environment_failure() {
    // Derivation runs entirely over already-imported trusted inputs. If a row
    // here ever claimed `environment-unavailable`, the category would be
    // describing something this code cannot observe.
    for (label, marker) in emitted_markers() {
        assert!(
            !marker.category.is_environment(),
            "[{label}] derivation claimed an environment failure: `{}`",
            marker.reason
        );
    }
}

#[test]
fn the_derivation_category_corpus_reaches_both_halves_of_the_partition() {
    use qcore_schemas::reason::ReasonCategory as Category;
    let categories: BTreeSet<Category> = emitted_markers()
        .into_iter()
        .map(|(_, marker)| marker.category)
        .collect();
    assert!(categories.contains(&Category::UnsupportedSyntax));
    assert!(categories.contains(&Category::IncompleteObligationCoverage));
    assert!(categories.contains(&Category::MissingPremise));
    assert_eq!(
        categories.len(),
        3,
        "derivation reached an unexpected category set: {categories:?}"
    );
    assert!(categories.iter().any(|c| c.describes_analysed_source()));
    assert!(categories.iter().any(|c| !c.describes_analysed_source()));
}

#[test]
fn relabeling_a_marker_retires_the_task_in_both_directions() {
    let program = parse_source_program(LIFECYCLE_UNIT).expect("fixture parses");
    let derived = derive_task(&inputs(&program), VerifierCapability::ChunkLifecycle);
    assert!(!derived.task.derivation_incomplete.is_empty());
    let original = derived.task.task_digest();

    let mut relabeled = derived.task.clone();
    relabeled.derivation_incomplete[0].category =
        qcore_schemas::reason::ReasonCategory::EnvironmentUnavailable;
    let moved = relabeled.task_digest();
    assert_ne!(
        original, moved,
        "a witness recorded against the original task must not replay against a \
         task whose marker was relabeled, nor the reverse"
    );
    // The reason text itself is untouched: the category refines, never replaces.
    assert_eq!(
        relabeled.derivation_incomplete[0].reason,
        derived.task.derivation_incomplete[0].reason
    );
}
