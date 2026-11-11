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

//! Card REASON-CODES: the closed L21.1 reason vocabulary as a schema surface.
//!
//! Three properties are pinned here, all of them fail-closed directions:
//!
//! 1. **Environment / source separation.** No environment reason, and no
//!    environment *category*, is readable as a statement about the analysed
//!    source, and refusal is never resolved by defaulting.
//! 2. **Mapping totality over the real corpus.** Every free-text reason this
//!    workspace emits classifies, and to the documented category. The corpus is
//!    enumerated from the code (see `reason_corpus`), not sampled.
//! 3. **Both-direction digest refusal.** A `DerivationIncomplete` recorded
//!    before the category field is refused now, and one recorded now is refused
//!    by the previous shape — the compatibility event `qcore-schemas` 6.0.0
//!    documents.

use std::collections::BTreeMap;

use qcore_schemas::obligation::{
    DerivationIncomplete, TaskBounds, VERIFICATION_TASK_VERSION, VerificationTask,
};
use qcore_schemas::plugin::{TargetProfile, UnitIdentity, VerifierCapability};
use qcore_schemas::reason::{ReasonCategory, ReasonSubject, UnclassifiedReason};

/// Every free-text reason family this workspace emits, with the category the
/// documented mapping must assign. Enumerated from the emitting code, not
/// sampled:
///
/// * `crates/qcore-verification/src/obligation_derivation.rs` — the seven
///   `DerivationIncomplete` sites and the nine `LifecycleScan` gap sites,
///   including all eight `unresolved` details.
/// * `crates/qcore-c-gateway/src/legacy_c.rs` — the projection summary and all
///   five `LegacyCError` Display forms.
/// * `crates/qcore-c-gateway/src/boolean_source.rs` — the three typed-import
///   refusals.
/// * `crates/qcore-audit/src/{compile_db,current,lib,source_audit}.rs` — every
///   reason the audit crate mints itself.
/// * `crates/qcore-audit/src/current_route.rs` — the two overrides.
fn reason_corpus() -> Vec<(&'static str, ReasonCategory)> {
    use ReasonCategory::{
        AwaitingHumanReview as Review, CompositionConflict as Conflict,
        ContractConflict as Bundles, ContractPinMismatch as Pin, EnvironmentUnavailable as Env,
        IncompleteObligationCoverage as Coverage, MissingPremise as Premise,
        OuterContract as Outer, PluginClaimUncertified as Plugin, ProviderFailure as Provider,
        ResidualCustody as Custody, StaleEvidence as Stale, UnsupportedSyntax as Syntax,
    };
    vec![
        // -- qcore-verification: obligation derivation ------------------------
        (
            "no core derivation for this capability in evidence v2.0 \
             (event/concurrency model extraction is W5)",
            Coverage,
        ),
        (
            "caller refuses the observations_trusted assumption required for \
             recorded observation replay",
            Premise,
        ),
        (
            "contract module `contract/qcore/x.qcore` does not parse",
            Premise,
        ),
        ("ambiguous observation contract `hook`", Premise),
        (
            "required observation hook `qcore_observe_close` has no `extern fn` \
             contract in the unit's trusted contract sources",
            Premise,
        ),
        (
            "per-allocation-site custody derivation runs in the unrestricted-C \
             gateway; the QCore verify-scope path derives call-effect and \
             parameter-mode obligations only (gateway summaries are not imported here)",
            Coverage,
        ),
        (
            "lifecycle call sites appear in 2 functions; the evidence-v2.0 order \
             bound covers a single analyzed function",
            Coverage,
        ),
        (
            "source lifecycle effect/ABI import is unavailable; model-call spelling \
             does not establish runtime outcomes or provider-chosen parameters",
            Coverage,
        ),
        // -- qcore-verification: LifecycleScan gaps ---------------------------
        (
            "`main` contains structured control flow; the evidence-v2.0 schedule \
             bound is a single path per function",
            Syntax,
        ),
        ("`main` has operations after a return", Syntax),
        (
            "`main` has a statement whose call structure could not be recovered",
            Syntax,
        ),
        ("`main` has unknown call effect `qcore_frobnicate`", Syntax),
        (
            "`main` has unsupported model-call arity for `qcore_chunk_open`",
            Syntax,
        ),
        ("`main` shadows a lifecycle identity", Syntax),
        // the eight `unresolved` details
        (
            "`main` calls `qcore_ctx_create`: a created context is not bound to a name",
            Syntax,
        ),
        (
            "`main` calls `qcore_stream_create`: a created stream is not bound to a name",
            Syntax,
        ),
        (
            "`main` calls `qcore_chunk_open`: an opened chunk is not bound to a name",
            Syntax,
        ),
        (
            "`main` calls `qcore_stream_create`: its context argument does not name \
             a created context",
            Syntax,
        ),
        (
            "`main` calls `qcore_chunk_open`: its stream argument does not name a \
             created stream",
            Syntax,
        ),
        (
            "`main` calls `qcore_ctx_destroy`: its argument does not name a created context",
            Syntax,
        ),
        (
            "`main` calls `qcore_stream_destroy`: its argument does not name a created stream",
            Syntax,
        ),
        (
            "`main` calls `qcore_chunk_close`: its argument does not name an opened chunk",
            Syntax,
        ),
        // -- qcore-c-gateway: projection summary and LegacyCError -------------
        (
            "resource projection unresolved: 3 unsupported operation(s), \
             0 unresolved alias(es), 0 custody summary conflict(s)",
            Syntax,
        ),
        (
            "resource projection unresolved: 0 unsupported operation(s), \
             2 unresolved alias(es), 0 custody summary conflict(s)",
            Custody,
        ),
        (
            "resource projection unresolved: 0 unsupported operation(s), \
             0 unresolved alias(es), 1 custody summary conflict(s)",
            Custody,
        ),
        // Card W45-CONTRACT-DEFERRED-REVIEW: the held-call component is named
        // only when present, and a held call is named before every source
        // category, whatever else the unit leaves open.
        (
            "resource projection unresolved: 0 unsupported operation(s), \
             0 unresolved alias(es), 0 custody summary conflict(s), \
             1 call(s) held awaiting human review",
            Review,
        ),
        (
            "resource projection unresolved: 4 unsupported operation(s), \
             2 unresolved alias(es), 0 custody summary conflict(s), \
             3 call(s) held awaiting human review",
            Review,
        ),
        (
            "call to `k_close` whose declared contract awaits human review \
             (awaiting-human-review)",
            Review,
        ),
        // Card W45-CONTRACT-REVISION-MODE: the pin component follows the
        // review component, is named only when present, and a pin-held call
        // is named before every source category.
        (
            "resource projection unresolved: 0 unsupported operation(s), \
             0 unresolved alias(es), 0 custody summary conflict(s), \
             2 call(s) held for a contract pin mismatch",
            Pin,
        ),
        (
            "resource projection unresolved: 5 unsupported operation(s), \
             1 unresolved alias(es), 0 custody summary conflict(s), \
             1 call(s) held for a contract pin mismatch",
            Pin,
        ),
        (
            "resource projection unresolved: 0 unsupported operation(s), \
             0 unresolved alias(es), 0 custody summary conflict(s), \
             1 call(s) held awaiting human review, 1 call(s) held for a contract pin mismatch",
            Review,
        ),
        (
            "call to `k_close` whose pinned contract is not in force: the audited source \
             does not match its lock (contract-pin-mismatch)",
            Pin,
        ),
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: a call held because its
        // callee's own analysed body refutes the declaration.
        (
            "call to `k_close` whose declared contract its own analysed body refutes \
             (contract-refuted-by-body)",
            Stale,
        ),
        // Card W48-RK-LOG-CALLER-FRAME (rule 32a): a call held inside its own
        // caller frame (`qcore_c_gateway::legacy_c::caller_frame_hold`).
        (
            "call to `rd_kafka_log_buf` inside its own caller frame: a caller-frame declaration \
             serves only callers outside the frame, whose own bodies decide their rows \
             (caller-frame-self-reliance)",
            Coverage,
        ),
        // Card W45-USER-CONTRACT-PLUGINS: the plugin firewall's two
        // uncertified-claim spellings (`qcore_verifier_host::plugin_firewall`).
        (
            "plugin claim uncertified: `acme.pool_check` proposed Safe without a certificate \
             the audit checks independently (QP016: no core witness checker for capability \
             `resource_contracts` at evidence version 2)",
            Plugin,
        ),
        (
            "plugin claim uncertified: `acme.pool_check` proposed Unsafe without a witness \
             the audit validates (QP010: unverified provider allegation: evidence v2 has no \
             counterexample checker)",
            Plugin,
        ),
        // Card OUTER-CONTRACT (rule 31): the runtime projection's reason
        // (`qcore_audit::outer_selection`).
        (
            "rests on outer contract `flb_external_plugin@0123abcd` through \
             stable:flb_output_instance.p (runtime projection, rule 31)",
            Outer,
        ),
        // Card AUDIT-COMPOSITION: the conflict leaf
        // (`qcore_schemas::composition`).
        (
            "composition-conflict: components of row `unit#0` under one provenance \
             (analyzer `qcore-audit@0.1.0:unrecorded-build`) differ; all are kept and none \
             is chosen",
            Conflict,
        ),
        // Card CONTRACT-SUPERSESSION: the held call of a function whose
        // bundles' pin stamps do not decide (`qcore_c_gateway::legacy_c`).
        (
            "call to `k_close` whose contract bundles conflict: their pin stamps do not decide \
             which declaration is in force (contract-conflict)",
            Bundles,
        ),
        (
            "`k_close`'s declared contract in k.qcore awaits human review \
             (awaiting-human-review), so it admits no call: the size is unverified",
            Review,
        ),
        ("C function `_cnd_timedwait_win32` was not found", Coverage),
        (
            "could not project body for C function `cmt_counter_create` at input line 91",
            Syntax,
        ),
        (
            "production C parser is unavailable: QCORE_CLANG is empty",
            Env,
        ),
        (
            "production C parser rejected the translation unit: expected ';'",
            Syntax,
        ),
        (
            "translation unit input identity cannot answer a filename-sensitive \
             construct: __FILE__",
            Coverage,
        ),
        // -- qcore-c-gateway: typed_c_ast ------------------------------------
        ("Clang AST unavailable: no such file", Env),
        ("Clang stdin unavailable", Env),
        ("Clang input writer panicked", Env),
        ("Clang input failed: broken pipe", Env),
        ("Clang AST failed: signal 9", Env),
        ("unsupported Clang AST JSON: trailing characters", Env),
        ("Clang AST rejected source: expected ';'", Syntax),
        (
            "expected one selected definition after preprocessing, found 0",
            Coverage,
        ),
        // -- qcore-c-gateway: boolean_source ---------------------------------
        (
            "outside single borrowed chunk / int-or-void-result source profile",
            Syntax,
        ),
        ("helper signature or parameter binding mismatch", Syntax),
        ("unmodeled statement DeclStmt", Syntax),
        ("unmodeled expression CallExpr", Syntax),
        ("unmodeled function parameters or attributes", Syntax),
        ("unmodeled function definition", Syntax),
        ("unmodeled helper parameters or attributes", Syntax),
        ("unmodeled helper definition", Syntax),
        ("unmodeled helper callee", Syntax),
        ("expected one ordinary declared record", Syntax),
        ("expected one complete chunk record definition", Syntax),
        ("expected one direct lifecycle field", Syntax),
        (
            "chunk dependency layout has unmodeled attributes or fields",
            Syntax,
        ),
        (
            "lifecycle field must be an ordinary non-volatile int, not a bitfield",
            Syntax,
        ),
        (
            "field assignment leaves the Boolean lifecycle domain",
            Syntax,
        ),
        ("ambiguous Boolean field models for one record ABI", Syntax),
        ("unsupported Boolean field operation mapping", Syntax),
        ("unsupported Boolean field result ABI", Syntax),
        ("function has no modeled chunk field access", Syntax),
        (
            "source path falls through without an explicit return",
            Syntax,
        ),
        (
            "helper record has no selected declared Boolean model",
            Syntax,
        ),
        ("helper record is not a selected Boolean model", Syntax),
        ("helper record differs from caller model record", Syntax),
        (
            "helper requires one declared record pointer and void result",
            Syntax,
        ),
        ("helper requires direct constant field assignments", Syntax),
        ("helper requires a direct assignment", Syntax),
        ("helper requires direct borrowed parameter", Syntax),
        (
            "helper field is not bound to its borrowed parameter",
            Syntax,
        ),
        ("helper assignment is not a constant", Syntax),
        ("helper has no checked field writes", Syntax),
        ("helper writes a declared Boolean model field", Syntax),
        (
            "helper declaration, structural layout, or uniqueness mismatch",
            Syntax,
        ),
        // Identity, revision and analyser-bound refusals from the same importer:
        // these must NOT reach a source-subject rule.
        ("Clang identity changed during Boolean source import", Env),
        (
            "ChunkIO source revision does not match the pinned revision",
            Stale,
        ),
        ("chunk helper source revision mismatch", Stale),
        ("helper source/build/profile selection mismatch", Stale),
        ("source expression exceeds the import depth limit", Coverage),
        ("source statement exceeds the import depth limit", Coverage),
        ("duplicate helper identity", Coverage),
        // -- qcore-audit: compile_db -----------------------------------------
        ("empty compiler executable", Env),
        ("unsupported compilation dialect `c++17`", Env),
        ("unterminated quoting in compile command", Env),
        ("shell expansion in compile command is unsupported", Env),
        ("shell expression in compile command is unsupported", Env),
        ("compiler executable `cc` cannot be resolved", Env),
        (
            "frontend build flags require the Clang C driver, not a C++ or CL driver",
            Env,
        ),
        (
            "audit requires the configured Clang executable (/usr/bin/clang); \
             compilation database selects /usr/bin/cc. Prepare an explicit Clang \
             build with quirin-audit prepare and run quirin-audit current over it \
             (prepare --run chains both); GCC compiler equivalence is unproven",
            Env,
        ),
        ("compile directory and source must be absolute", Env),
        ("empty or unexpanded compiler option is unsupported", Env),
        ("sysroot-relative include paths are unsupported", Env),
        ("invalid -Wundef-prefix argument", Env),
        ("missing -arch argument", Env),
        ("invalid -arch argument", Env),
        // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y).
        (
            "unsupported compilation target `x86_64`: a target is two to four plain components",
            Env,
        ),
        (
            "unsupported compilation target: data model ILP32 is not the readers' LP64 premise; \
             integer widths per target are card CG20's (rule 11q)",
            Env,
        ),
        (
            "unsupported compilation ABI `lp128`: no recorded data-model consequence",
            Env,
        ),
        ("missing -target argument", Env),
        ("missing compile output argument", Env),
        ("invalid compile output argument", Env),
        (
            "compiler option `-fplugin=x` cannot be preserved by the bounded gateway",
            Env,
        ),
        (
            "compile command requires one -c, one source input and at most one output",
            Env,
        ),
        ("compile command has no recorded source input", Env),
        (
            "frontend build flags require an explicit supported -std= dialect",
            Env,
        ),
        ("compilation database contains no command for src/x.c", Env),
        (
            "compilation database contains 4 distinct configurations for src/x.c; \
             the audit will not choose one",
            Env,
        ),
        // -- qcore-audit: lib / current --------------------------------------
        ("source outside selected checkout", Env),
        ("source outside current checkout", Env),
        (
            "selected source `src/x.c` does not match its pinned bytes",
            Stale,
        ),
        ("source is not UTF-8", Syntax),
        // -- qcore-audit: source_audit ---------------------------------------
        (
            "raw source definition absent from the selected typed import; \
             possibly build-excluded",
            Coverage,
        ),
        ("source-contract configuration: unknown policy", Provider),
        // -- qcore-c-gateway: legacy_c::enumeration (unit E01-F1) -------------
        (
            "enumerated definition uses an identifier-list (K&R) parameter \
             list, which the resource analyzer's body finder does not read",
            Syntax,
        ),
        (
            "enumerated definition `knr` is not analysed: enumerated \
             definition uses an identifier-list (K&R) parameter list, which \
             the resource analyzer's body finder does not read",
            Syntax,
        ),
        (
            "enumeration completeness refused: macro expansion for this unit \
             is unavailable, so a definition produced only by preprocessing \
             cannot be enumerated",
            Env,
        ),
        (
            "enumeration completeness refused: the line-marked preprocessing \
             does not name its primary input, so main-source attribution is \
             unavailable",
            Env,
        ),
        (
            "enumeration completeness refused: a file-scope delimiter has no \
             match, so definition boundaries cannot be established",
            Syntax,
        ),
        (
            "enumeration completeness refused: a file-scope construct is \
             neither a definition nor a declaration this scanner reads, so \
             definition boundaries cannot be established",
            Syntax,
        ),
        // -- crates/qcore-audit/src/current_route.rs ---------------
        (
            "selected library requires evidence unavailable in current-revision \
             resource projection",
            Stale,
        ),
        (
            "historical source/native/provider evidence is not admitted by the \
             current-revision resource audit",
            Stale,
        ),
    ]
}

#[test]
fn the_documented_mapping_is_total_over_every_reason_this_workspace_emits() {
    for (reason, expected) in reason_corpus() {
        match ReasonCategory::classify(reason) {
            Ok(actual) => assert_eq!(
                actual, expected,
                "`{reason}` classified as {actual}, expected {expected}"
            ),
            Err(_) => panic!("`{reason}` is emitted in production but classifies to nothing"),
        }
    }
}

#[test]
fn the_corpus_exercises_every_category_so_the_mapping_is_not_vacuous() {
    let used: std::collections::BTreeSet<_> = reason_corpus()
        .into_iter()
        .map(|(_, category)| category)
        .collect();
    for category in ReasonCategory::ALL {
        assert!(
            used.contains(&category),
            "no real reason maps to {category}; the mapping would be vacuous there"
        );
    }
}

#[test]
fn no_environment_reason_is_readable_as_a_statement_about_the_analysed_source() {
    for (reason, expected) in reason_corpus() {
        if expected != ReasonCategory::EnvironmentUnavailable {
            continue;
        }
        let actual = ReasonCategory::classify(reason).expect("an environment reason classifies");
        assert_eq!(actual, ReasonCategory::EnvironmentUnavailable);
        assert!(
            !actual.describes_analysed_source(),
            "`{reason}` would be readable as a source defect"
        );
        assert!(
            !actual.may_support_unsafe(),
            "`{reason}` could contribute to a provisional Unsafe"
        );
        assert_eq!(actual.subject(), ReasonSubject::Environment);
    }
}

#[test]
fn the_word_unsupported_in_a_compile_command_never_becomes_a_source_statement() {
    // The hazard this ordering exists for: several environment reasons contain
    // "unsupported", and a naive marker table would classify them as a
    // construct in the analysed source.
    for reason in [
        "unsupported compilation dialect `c++17`",
        "empty or unexpanded compiler option is unsupported",
        "sysroot-relative include paths are unsupported",
        "shell expansion in compile command is unsupported",
    ] {
        let category = ReasonCategory::classify(reason).unwrap();
        assert_eq!(category, ReasonCategory::EnvironmentUnavailable, "{reason}");
        assert!(!category.describes_analysed_source(), "{reason}");
    }
}

#[test]
fn an_unrecognised_reason_is_refused_and_never_defaulted() {
    for reason in [
        "",
        "something nobody in this workspace writes",
        "No such file or directory (os error 2)",
        "resource projection unresolved: many unsupported operation(s)",
        // Card W45-CONTRACT-DEFERRED-REVIEW: a malformed or zero held count is
        // refused rather than read as either category.
        "resource projection unresolved: 0 unsupported operation(s), \
         0 unresolved alias(es), 0 custody summary conflict(s), \
         many call(s) held awaiting human review",
        "resource projection unresolved: 0 unsupported operation(s), \
         0 unresolved alias(es), 0 custody summary conflict(s), \
         0 call(s) held awaiting human review",
        // Card W45-CONTRACT-REVISION-MODE: likewise for the pin component,
        // and the components in the wrong order.
        "resource projection unresolved: 0 unsupported operation(s), \
         0 unresolved alias(es), 0 custody summary conflict(s), \
         0 call(s) held for a contract pin mismatch",
        "resource projection unresolved: 0 unsupported operation(s), \
         0 unresolved alias(es), 0 custody summary conflict(s), \
         1 call(s) held for a contract pin mismatch, 1 call(s) held awaiting human review",
    ] {
        assert_eq!(
            ReasonCategory::classify(reason),
            Err(UnclassifiedReason),
            "`{reason}` must be refused, not defaulted"
        );
    }
}

fn task(markers: Vec<DerivationIncomplete>) -> VerificationTask {
    VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "fixture.qc".into(),
        },
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "audit".into(),
            profile: "default".into(),
            dialect: "qcore".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        semantic_profile: "qcore.chunk_lifecycle.v1".into(),
        contract_sources: BTreeMap::new(),
        bounds: TaskBounds::single_path(),
        assumptions: BTreeMap::new(),
        evidence_version: 2,
        obligations: Vec::new(),
        derivation_incomplete: markers,
    }
}

fn marker(category: ReasonCategory) -> DerivationIncomplete {
    DerivationIncomplete {
        capability: VerifierCapability::EventTraces,
        category,
        reason: "no derivation in this increment".into(),
    }
}

#[test]
fn an_empty_marker_list_keeps_its_pre_6_0_0_bytes_and_digest() {
    // Measured, not assumed: the element shape is invisible when the list is
    // empty, so every archived artifact whose task carries no marker keeps its
    // exact task digest across this compatibility event.
    let serialized = serde_json::to_string(&task(Vec::new())).unwrap();
    assert!(
        serialized.contains(r#""derivation_incomplete":[]"#),
        "{serialized}"
    );
    assert!(
        !serialized.contains("category"),
        "an empty list must not mention the new field: {serialized}"
    );
}

#[test]
fn a_marker_record_recorded_before_the_category_field_is_refused() {
    // The pre-6.0.0 shape: capability + reason, no category.
    let legacy = serde_json::json!({
        "capability": "event_traces",
        "reason": "no derivation in this increment",
    });
    let error = serde_json::from_value::<DerivationIncomplete>(legacy)
        .expect_err("a record without a category must be refused, not defaulted");
    assert!(
        error.to_string().contains("category"),
        "refusal must name the missing field: {error}"
    );
}

#[test]
fn a_marker_record_written_now_is_refused_by_the_previous_shape() {
    // The mirror direction, modeled with the exact `deny_unknown_fields`
    // discipline the 5.0.0 record used.
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct PreviousShape {
        capability: VerifierCapability,
        reason: String,
    }
    let current = serde_json::to_value(marker(ReasonCategory::IncompleteObligationCoverage))
        .expect("markers serialize");
    let error = serde_json::from_value::<PreviousShape>(current)
        .expect_err("a 5.0.0 consumer must refuse the new field");
    assert!(
        error.to_string().contains("category"),
        "refusal must name the unknown field: {error}"
    );
}

#[test]
fn the_category_is_bound_into_the_task_digest_in_both_directions() {
    let coverage = task(vec![marker(ReasonCategory::IncompleteObligationCoverage)]);
    let environment = task(vec![marker(ReasonCategory::EnvironmentUnavailable)]);
    assert_ne!(
        coverage.task_digest(),
        environment.task_digest(),
        "relabeling a marker must retire the task, so a witness recorded against \
         one cannot be replayed against the other"
    );
    // Deterministic, and unaffected by anything but the recorded fields.
    assert_eq!(coverage.task_digest(), {
        let repeat = task(vec![marker(ReasonCategory::IncompleteObligationCoverage)]);
        repeat.task_digest()
    });
    assert_eq!(coverage.task_digest().len(), 64);
}

#[test]
fn the_category_refines_the_row_and_replaces_nothing() {
    // §L21.1 rule 2: adding the category must not disturb any other field of
    // the marker, the task, or the obligation inventory.
    let with_marker = task(vec![marker(ReasonCategory::MissingPremise)]);
    let row = &with_marker.derivation_incomplete[0];
    assert_eq!(row.capability, VerifierCapability::EventTraces);
    assert_eq!(row.reason, "no derivation in this increment");
    assert!(with_marker.is_incomplete_for(VerifierCapability::EventTraces));
    assert!(with_marker.obligations.is_empty());
    assert!(with_marker.validate().is_ok());
    assert!(!row.describes_analysed_source());
}
