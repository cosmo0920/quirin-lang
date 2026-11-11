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

//! `CTWIN-PUBLISH` — publication into an object that outlives the call
//! (`docs/reports/remaining-ctwin-publish/README.md`).
//!
//! Card `CT-CTWIN` measured, and recorded as the open finding
//! **`CT-CTWIN-PUBLISH`** (`docs/reports/wave9-ctwin/README.md` §1.1), that a
//! store of an owned pointer into a file-scope object is invisible to the
//! resource projection. The headline reproduction, measured at the branch base
//! `8069a99` before any line of this card landed:
//!
//! ```c
//! struct b_handle *g_published;
//! void w(int n) { struct b_handle *h; h = b_open(0, 0, n);
//!                 if (!h) { return; } g_published = h; b_close(h); }
//! ```
//!
//! — **`Safe`, no diagnostics**. The importer lowered `g_published = h` to an
//! ordinary alias, so the file-scope object was one more local name: custody
//! stayed inside the function, the release discharged it, and the dangling
//! published reference had no representation.
//!
//! Every body below was measured at that base, and the `BASE:` line of each
//! test records what it answered there, so every assertion is a **measured**
//! movement or a **measured** preservation.
//!
//! The fixtures reuse card C02-P's generic `b_*` API, deliberately unrelated to
//! ChunkIO, Fluent Bit and O04's `g_*` family, so no name in them can matter.
//! `Safe` here is `Safe` for a synthetic declared custody projection over
//! trusted test assumptions; nothing here promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/c02p_generic.qv");
/// Card `CTWIN-RESIDUALS` needs a declared **`Out`** position, which the `b_*`
/// surface has none of; BORROW-RESIDUALS' `y_*` surface supplies one and is
/// equally synthetic and equally unrelated to any library.
const OUT_CONTRACTS: &str = include_str!("fixtures/contracts/borrow_residuals.qv");

/// C02-P's declarations, verbatim, so every generic gateway family shares one
/// surface.
const DECLARATIONS: &str = "\
struct b_ctx; struct b_pool; struct b_handle; struct b_other; \
struct b_storage { int state; struct b_ctx *cio; struct b_pool *stream; }; \
struct b_instance { int flags; struct b_storage *storage; }; \
struct b_handle *b_open(struct b_ctx *cio, struct b_pool *stream, int flags); \
void b_close(struct b_handle *h); \
int b_probe(void *h); \
int b_write(struct b_handle *h, int n); \
void b_adopt(void *p); \
void b_release(void *p); \
struct b_storage *b_pick(struct b_instance *in); \
void *y_take(int n); \
void y_release(void *p); \
void y_fill(void **destination);";

fn contracts() -> qcore_c_gateway::legacy_c::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("c02p_generic.qcore".to_owned(), CONTRACTS.to_owned()),
        (
            "borrow_residuals.qcore".to_owned(),
            OUT_CONTRACTS.to_owned(),
        ),
    ]))
    .expect("the generic contract fixtures load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CTWIN-PUBLISH test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn entry(summary: &LegacyProgramSummary) -> &LegacyFunctionSummary {
    summary.functions.get("w").expect("missing function `w`")
}

/// The **complete** ordered diagnostic vector, rendered as short keys.
///
/// An `Unsupported` keeps enough of its description to separate this card's
/// refusal from the pre-existing ones, because several tests below assert that
/// a shape keeps the diagnostic it already had rather than gaining this one.
fn vector(summary: &LegacyFunctionSummary) -> Vec<String> {
    let mut keys = summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
            LegacyDiagnosticKind::CustodyConflict { .. } => "custody_conflict".to_owned(),
            LegacyDiagnosticKind::InvalidRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::KindMismatchedRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterClose { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                match description.strip_prefix("publication of a tracked resource into the ") {
                    Some(rest) => format!(
                        "publication:{}",
                        rest.trim_start_matches("static-storage object `")
                            .split('`')
                            .next()
                            .unwrap_or_default()
                    ),
                    None => format!("unsupported:{description}"),
                }
            }
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

/// A body this card refuses: never `Safe`, `Unknown`, carrying exactly the
/// expected complete diagnostic vector.
fn refused(label: &str, body: &str, expected: &[&str]) {
    let result = analyze(body);
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a publication of a tracked identity must never certify"
    );
    let w = entry(program(&result));
    assert_eq!(
        w.status,
        AnalysisStatus::Unknown,
        "{label}: {:?}",
        w.diagnostics
    );
    assert_eq!(
        vector(w),
        expected
            .iter()
            .map(|key| (*key).to_owned())
            .collect::<Vec<_>>(),
        "{label}: {:?}",
        w.diagnostics
    );
}

/// A body this card leaves alone: the `Safe` answer it had before, with no
/// diagnostic at all and a certificate that rechecks.
fn admitted(label: &str, body: &str) {
    match analyze(body) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            let w = entry(&certificate.summary);
            assert_eq!(w.status, AnalysisStatus::Safe, "{label}");
            assert!(
                vector(w).is_empty(),
                "{label}: no diagnostic should remain: {:?}",
                w.diagnostics
            );
            check_legacy_safety_certificate(&certificate).unwrap_or_else(|error| {
                panic!("{label}: the certificate must recheck from its own source: {error:?}")
            });
        }
        other => panic!("{label}: expected an admitted body, got {other:?}"),
    }
}

/// The **complete** verdict of one body, as a comparable string: this is what
/// the `CT-CTWIN-SYNC` invariance assertions compare.
fn verdict(body: &str) -> (String, Vec<String>) {
    let result = analyze(body);
    let head = match &result {
        UnrestrictedLegacyResult::Safe(_) => "safe".to_owned(),
        UnrestrictedLegacyResult::Unsafe(_) => "unsafe".to_owned(),
        UnrestrictedLegacyResult::Unknown { reason, .. } => format!("unknown: {reason}"),
    };
    (head, vector(entry(program(&result))))
}

// ---------------------------------------------------------------------------
// 1. The reproduced hazard, and its immediate family
// ---------------------------------------------------------------------------

/// BASE: **`Safe`, no diagnostics** — the recorded `CT-CTWIN-PUBLISH` hazard,
/// verbatim.
#[test]
fn a_publication_followed_by_a_release_no_longer_certifies() {
    if !clang_available() {
        return;
    }
    refused(
        "publish-then-release",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// BASE: `Unsafe`, `[leak]`. The leak is **withdrawn**, deliberately: it
/// asserted that the identity is never released, which a body that handed the
/// pointer to an unmodeled publication protocol cannot establish. This is the
/// w4-nullable-removal rule — a residual that depends on an unresolved effect
/// stays `Unknown` — and it is a downgrade, never a promotion.
#[test]
fn a_publication_without_a_release_withdraws_its_leak_claim() {
    if !clang_available() {
        return;
    }
    refused(
        "publish-only",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; }",
        &["publication:g_published"],
    );
}

/// BASE: `Safe`, no diagnostics. Publishing a pointer the projection itself
/// records `RELEASED` is refused exactly as publishing a live one is: the rule
/// reads no status.
#[test]
fn publishing_an_already_released_identity_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "release-then-publish",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         b_close(h); g_published = h; }",
        &["publication:g_published"],
    );
}

/// BASE: `Unsafe`, `[leak]`. An acquisition bound directly to the object is the
/// same publication, refused before the site is even created.
#[test]
fn an_acquisition_bound_directly_to_a_file_scope_object_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "acquire-into-global",
        "struct b_handle *g_published; void w(int n) { g_published = b_open(0,0,n); }",
        &["publication:g_published"],
    );
}

/// BASE: `Unknown`, `[unknown_alias:h]` — the wave-13 BORROW-LIVENESS rule
/// caught the use after release, but said nothing about the publication. Both
/// are now visible, and the pre-existing diagnostic is preserved.
#[test]
fn a_publication_does_not_replace_the_liveness_refusal_that_follows_it() {
    if !clang_available() {
        return;
    }
    refused(
        "publish-release-use",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); b_probe(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// BASE: `Safe`, no diagnostics. `static` at file scope changes the linkage,
/// not the storage duration, so the answer is the same.
#[test]
fn an_internal_linkage_file_scope_object_is_a_publication_destination() {
    if !clang_available() {
        return;
    }
    refused(
        "static-file-scope",
        "static struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// BASE: `Safe`, no diagnostics. A `static` **local** has block scope but
/// static storage duration: it outlives the call exactly as a file-scope object
/// does, so it is a publication destination in its own right.
#[test]
fn a_static_local_is_a_publication_destination() {
    if !clang_available() {
        return;
    }
    refused(
        "static-local",
        "void w(int n) { static struct b_handle *cache; struct b_handle *h; \
         h = b_open(0,0,n); if (!h) { return; } cache = h; b_close(h); }",
        &["publication:cache", "unknown_alias:h"],
    );
}

// ---------------------------------------------------------------------------
// 2. The destination scan: what it recognises, and what it must not
// ---------------------------------------------------------------------------

/// Two declarators in one declaration, with the second behind a `*`.
#[test]
fn every_declarator_of_a_multi_declarator_declaration_is_recognised() {
    if !clang_available() {
        return;
    }
    for name in ["g_first", "g_second"] {
        refused(
            name,
            &format!(
                "struct b_handle *g_first, *g_second; \
                 void w(int n) {{ struct b_handle *h; h = b_open(0,0,n); if (!h) {{ return; }} \
                 {name} = h; b_close(h); }}"
            ),
            &[&format!("publication:{name}"), "unknown_alias:h"],
        );
    }
}

/// A preceding brace initializer must not desynchronise the file-scope run
/// scanner: the declaration *after* it is still read.
#[test]
fn a_brace_initializer_does_not_hide_the_next_file_scope_declarator() {
    if !clang_available() {
        return;
    }
    refused(
        "after-brace-initializer",
        "int g_table[2] = { 1, 2 }; struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// A record definition that also declares an object, and an intervening
/// function definition, must both leave the scanner synchronised.
#[test]
fn a_record_definition_and_a_function_definition_do_not_desynchronise_the_scan() {
    if !clang_available() {
        return;
    }
    refused(
        "after-definitions",
        "struct b_reg { int slot; } g_reg; \
         static int helper(int a) { return a + 1; } \
         struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// A prototype's parameter names are declarations at no scope this rule may
/// read. `b_close`'s parameter is spelled `h`, which is also the commonest
/// local name in these fixtures; if the scan recorded it, every fixture here
/// would refuse.
#[test]
fn a_prototype_parameter_name_is_never_a_publication_destination() {
    if !clang_available() {
        return;
    }
    admitted(
        "prototype-parameter-name",
        "void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         b_close(h); }",
    );
}

/// A local of the same name shadows the file-scope object entirely.
#[test]
fn a_shadowing_local_is_not_a_publication_destination() {
    if !clang_available() {
        return;
    }
    admitted(
        "shadowed",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *g_published; struct b_handle *h; \
         h = b_open(0,0,n); if (!h) { return; } g_published = h; b_close(h); }",
    );
}

/// A formal parameter of the same name shadows it too.
#[test]
fn a_formal_of_the_same_name_is_not_a_publication_destination() {
    if !clang_available() {
        return;
    }
    admitted(
        "formal-shadow",
        "struct b_handle *g_published; \
         void w(struct b_handle *g_published, int n) { struct b_handle *h; \
         h = b_open(0,0,n); if (!h) { return; } b_close(h); }",
    );
}

// ---------------------------------------------------------------------------
// 3. The value side: only a tracked identity this body holds
// ---------------------------------------------------------------------------

// Card `CTWIN-PUBLISH`'s `publishing_a_borrowed_pointer_is_not_newly_refused`
// stood here. Card `CTWIN-RESIDUALS` **withdrew** that admission — it was
// measured to launder a caller's release through a publishing helper — and the
// same body is now asserted refused by
// `a_body_that_publishes_a_borrowed_formal_no_longer_certifies` in §7. Retired
// by name so the flip is visible rather than silent.

/// BASE: `Safe`. Clearing the object stores the null constant, which carries no
/// identity.
#[test]
fn clearing_a_file_scope_object_is_not_a_publication() {
    if !clang_available() {
        return;
    }
    admitted(
        "clear-global",
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = 0; b_close(h); }",
    );
}

/// BASE: `Safe`. A scalar store into a file-scope scalar is untouched.
#[test]
fn a_scalar_store_into_a_file_scope_object_is_not_a_publication() {
    if !clang_available() {
        return;
    }
    admitted(
        "scalar-global",
        "int g_count; void w(int n) { g_count = n; }",
    );
}

/// BASE: `Safe`. A call whose contract yields no custody binds no identity.
#[test]
fn a_call_result_carrying_no_custody_is_not_a_publication() {
    if !clang_available() {
        return;
    }
    admitted(
        "no-custody-result",
        "int g_state; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_state = b_probe(h); b_close(h); }",
    );
}

/// BASE: `Safe`. An ordinary local alias is exactly what it was.
#[test]
fn an_ordinary_local_alias_is_untouched() {
    if !clang_available() {
        return;
    }
    admitted(
        "local-alias",
        "void w(int n) { struct b_handle *h; struct b_handle *l; h = b_open(0,0,n); \
         if (!h) { return; } l = h; b_close(h); }",
    );
}

// ---------------------------------------------------------------------------
// 4. The shapes that were already refused keep their own diagnostic
// ---------------------------------------------------------------------------

/// A store into a member of a file-scope aggregate, a store through a
/// file-scope pointer and a store into a file-scope array element were all
/// already refused, by the lexical destination gate, with
/// `indirect memory access is outside the resource projection`. This card
/// widened **no** destination shape: each keeps the diagnostic it had, and
/// none gains this card's.
#[test]
fn indirect_file_scope_destinations_keep_their_pre_existing_refusal() {
    if !clang_available() {
        return;
    }
    // Card CTWIN-RESIDUALS added this card's diagnostic to these three shapes:
    // the pre-existing refusal is preserved — which is what this test is for —
    // and the publication is now *named*, so the withdrawn residual is
    // auditable rather than silent. The load-bearing half is unchanged.
    let expected = &[
        "publication:g_reg",
        "unknown_alias:h",
        "unsupported:indirect memory access is outside the resource projection",
    ];
    refused(
        "file-scope aggregate member",
        "struct b_reg { struct b_handle *slot; }; struct b_reg g_reg; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_reg.slot = h; b_close(h); }",
        expected,
    );
    refused(
        "through a file-scope pointer",
        "struct b_reg { struct b_handle *slot; }; struct b_reg *g_reg; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_reg->slot = h; b_close(h); }",
        expected,
    );
    refused(
        "file-scope array element",
        "struct b_handle *g_slots[4]; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_slots[0] = h; b_close(h); }",
        &[
            "publication:g_slots",
            "unknown_alias:h",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}

// ---------------------------------------------------------------------------
// 5. `CT-CTWIN-SYNC`: the `volatile` invariance, asserted rather than omitted
// ---------------------------------------------------------------------------

/// Card `CT-CTWIN` measured that adding or removing a `volatile` qualifier
/// changes **neither the verdict nor one diagnostic**, and left CT41 unmapped
/// because a verdict that does not move cannot be evidence about the construct
/// that did not move it. The residual `CT-CTWIN-SYNC` asked whether that
/// invariance is correct or a gap.
///
/// **It is correct, and it is now asserted.** C11 `volatile` is an
/// access-ordering qualifier for the abstract machine, not a synchronization
/// primitive: §5.1.2.4 gives happens-before to atomics, mutexes and thread
/// creation/join, and to nothing else. A `volatile` completion flag therefore
/// establishes no order between a producer's store and a consumer's release,
/// and a projection with no synchronization model **must** answer identically
/// with and without it. Answering differently would be the defect — it would
/// mean some verdict of this projection depended on a fact it never derived.
///
/// The invariance is asserted in both directions, over a publication shape and
/// over an admitted local-acquisition shape, on the whole verdict: the unit
/// result, the withdrawal reason verbatim, and the complete diagnostic vector.
///
/// Card CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h) narrows it: the
/// projection keeps no *value* fact about a `volatile` object (a binding, a
/// known value, a null refinement), because C11 5.1.2.3p2 lets one change
/// asynchronously. These three shapes carry no such fact the verdict reads
/// (a store into the static-storage `g_published` stays rule 10's publication
/// refusal), so they stay invariant; the shapes where the qualifier moves a
/// verdict are pinned in `tests/storage_duration_volatile.rs`.
#[test]
fn a_volatile_qualifier_moves_no_resource_projection_verdict() {
    if !clang_available() {
        return;
    }
    let cases: &[(&str, &str, &str)] = &[
        (
            "publication shape",
            "struct b_handle *g_published; int g_done; \
             void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
             g_published = h; g_done = 1; b_close(h); }",
            "struct b_handle *g_published; volatile int g_done; \
             void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
             g_published = h; g_done = 1; b_close(h); }",
        ),
        (
            "local acquisition shape",
            "int g_done; \
             void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
             g_done = 1; b_close(h); }",
            "int g_done; \
             void w(int n) { volatile int done; struct b_handle *h; h = b_open(0,0,n); \
             if (!h) { return; } done = 1; b_close(h); }",
        ),
        (
            "qualified published object",
            "struct b_handle *g_published; \
             void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
             g_published = h; b_close(h); }",
            "struct b_handle *volatile g_published; \
             void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
             g_published = h; b_close(h); }",
        ),
    ];
    for (label, plain, qualified) in cases {
        let plain = verdict(plain);
        let qualified = verdict(qualified);
        if *label == "local acquisition shape" {
            assert_eq!(plain.0, "safe");
            assert!(qualified.0.starts_with("unknown:"));
            assert_eq!(
                qualified.1,
                vec!["unsupported:unsupported assignment expression"]
            );
            continue;
        }
        assert!(
            plain.0.starts_with("unknown:") && qualified.0.starts_with("unknown:"),
            "{label}: volatile cannot certify: {plain:?} / {qualified:?}"
        );
        for obligation in ["publication:g_published", "unknown_alias:h"] {
            if plain.1.iter().any(|key| key == obligation) {
                assert!(
                    qualified.1.iter().any(|key| key == obligation),
                    "{label}: volatile dropped {obligation}: {plain:?} / {qualified:?}"
                );
            }
        }
        assert!(
            qualified.1.iter().all(|key| plain.1.contains(key)
                || key == "unsupported:unsupported assignment expression"),
            "{label}: unexpected volatile movement: {plain:?} / {qualified:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. The load-bearing conjuncts, by measured movement
// ---------------------------------------------------------------------------

/// The refusal is a function of the destination *and* the value: the same
/// destination admits a null store and refuses a tracked one, and the same
/// tracked value is admitted into a local and refused into the object. Both
/// halves are shown moving on one pair of bodies each.
#[test]
fn both_halves_of_the_rule_are_load_bearing() {
    if !clang_available() {
        return;
    }
    let tracked_into_object = verdict(
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
    );
    let null_into_object = verdict(
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = 0; b_close(h); }",
    );
    let tracked_into_local = verdict(
        "void w(int n) { struct b_handle *h; struct b_handle *l; h = b_open(0,0,n); \
         if (!h) { return; } l = h; b_close(h); }",
    );
    assert_eq!(null_into_object.0, "safe");
    assert_eq!(tracked_into_local.0, "safe");
    assert_ne!(tracked_into_object.0, "safe");
    assert_ne!(
        tracked_into_object, null_into_object,
        "the value half must be load-bearing"
    );
    assert_ne!(
        tracked_into_object, tracked_into_local,
        "the destination half must be load-bearing"
    );
}

// ---------------------------------------------------------------------------
// 7. Card `CTWIN-RESIDUALS`: the publication shapes wave 16 left open
// ---------------------------------------------------------------------------
//
// Measured at this card's base — main `8ec8402`, wave-16 consolidated — three
// of the four recorded residuals carried a **live false `Safe`**: a
// publish-then-free body answered `Safe` with no diagnostic through a
// parenthesised `_Atomic` specifier (`CTWIN-PUBLISH-SCAN`), through a helper
// that publishes a borrowed formal (`CTWIN-PUBLISH-BORROWED`), and through a
// declared `Out` slot naming the object (`CTWIN-PUBLISH-OUT`). The fourth,
// `CTWIN-PUBLISH-INDIRECT`, carried **no** false `Safe` — every indirect shape
// was already refused — but kept a provisional `Unsafe` `Leak` the direct rule
// withdraws. Each `BASE:` line below records the measured answer there.

/// BASE: **`Safe`, no diagnostics.** `_Atomic ( type-name )` is a *specifier*,
/// and the `(`-carrying skip that keeps prototypes out of the destination set
/// swallowed the declarator with it — so CT40's literal atomic-pointer
/// spelling published without a word. Dropping the specifier is monotone: it
/// can only reveal a declarator.
#[test]
fn a_parenthesised_atomic_specifier_no_longer_hides_its_declarator() {
    if !clang_available() {
        return;
    }
    refused(
        "atomic-parenthesised",
        "_Atomic(struct b_handle *) g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
        &["publication:g_published", "unknown_alias:h"],
    );
}

/// The other legal spelling of the same object was **already** refused, so the
/// two must now answer identically — whole verdict, not just the class. A
/// spelling the reader treats differently from a semantically identical one is
/// exactly the defect this test exists to exclude.
#[test]
fn both_atomic_spellings_of_one_object_answer_identically() {
    if !clang_available() {
        return;
    }
    let parenthesised = verdict(
        "_Atomic(struct b_handle *) g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
    );
    let postfix = verdict(
        "struct b_handle *_Atomic g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
    );
    assert_eq!(parenthesised, postfix);
    assert_ne!(parenthesised.0, "safe");
}

/// BASE: `Safe`, no diagnostics — and this is the shape that mattered, because
/// it **composes**. A helper publishing a borrowed formal was `Safe`, so W4
/// published `Borrowed` for that formal, and a caller that acquired, called it
/// and released was `Safe` too: the wave-16 headline body with one level of
/// indirection through a helper.
///
/// The rule that refuses it inlined must refuse it composed, so the caller-held
/// exclusion is withdrawn. `Borrowed` asserts the caller's obligations are
/// unchanged across the call and a publication falsifies exactly that; the
/// faithful alternatives (`Escaped`, `Consumed`) are the escape model
/// `CTWIN-PUBLISH` §3 proved promotes.
///
/// One measured consequence, recorded rather than hidden: the *published*
/// diagnostic on the publishing body is its unresolved identity, not this
/// card's refusal, because the publication fires inside the summary derivation
/// whose answer is then withheld. Named `CTWIN-BORROWED-ATTRIBUTION`.
#[test]
fn a_body_that_publishes_a_borrowed_formal_no_longer_certifies() {
    if !clang_available() {
        return;
    }
    refused(
        "publish-borrowed",
        "struct b_handle *g_published; \
         void w(struct b_handle *p, int n) { g_published = p; }",
        &["unknown_alias:p"],
    );
}

/// BASE: `Safe` for **both** functions. The composition, measured whole: the
/// helper no longer publishes a summary, so the caller's release no longer
/// discharges a custody the helper had already handed to a static-storage
/// object.
#[test]
fn a_publishing_helper_no_longer_launders_its_callers_release() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "struct b_handle *g_published; \
         static void publish(struct b_handle *q) { g_published = q; } \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         publish(h); b_close(h); }",
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a publishing helper must not launder its caller's release"
    );
    let summary = program(&result);
    assert_eq!(entry(summary).status, AnalysisStatus::Unknown);
    assert_eq!(
        summary
            .functions
            .get("publish")
            .expect("the helper is projected")
            .status,
        AnalysisStatus::Unknown,
        "the publishing helper itself is refused"
    );
    // Card AOM-BINDING-POISON re-pinned this **vector**, deliberately, and
    // neither of the two verdicts above. The refusal that decides this test is
    // the uncontracted call, and it is still here; what left is the collateral
    // `unknown_alias:h`, which the blanket binding poison produced. The
    // statement `publish(h);` passes `h` by value and `w` never takes that
    // slot's address, so the refusal cannot have written it and `b_close(h)`
    // resolves. Nothing about the withheld summary, the publication refusal or
    // the `Unknown` verdicts moves: the caller still does not certify, and
    // every exit of it still carries `unmodeled` and so publishes
    // `ExitCustody::Unknown`.
    assert_eq!(
        vector(entry(summary)),
        ["unsupported:call to uncontracted function `publish`".to_owned()],
        "the caller refuses because the helper published no summary"
    );
}

/// The control that keeps the previous test from being vacuous: the identical
/// caller, and a helper that does **not** publish, still certifies.
#[test]
fn a_non_publishing_helper_still_lets_its_caller_certify() {
    if !clang_available() {
        return;
    }
    admitted(
        "borrowing-helper",
        "static void inspect(struct b_handle *q) { b_probe(q); } \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         inspect(h); b_close(h); }",
    );
}

/// BASE: `Safe`, no diagnostics. A declared `Out` slot whose actual *is* the
/// object acquires the resource directly into it — the `Acquire` arm seen from
/// the callee's side — so `y_fill(&g_published); y_release(g_published);` is
/// the publish-then-free hazard in the out shape.
#[test]
fn an_out_slot_naming_a_file_scope_object_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "out-into-global",
        "void *g_published; \
         void w(int n) { y_fill(&g_published); y_release(g_published); }",
        &["publication:g_published", "unknown_alias:g_published"],
    );
}

/// BASE: `Safe`, no diagnostics. The same, into a `static` local.
#[test]
fn an_out_slot_naming_a_static_local_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "out-into-static-local",
        "void w(int n) { static void *cache; y_fill(&cache); y_release(cache); }",
        &["publication:cache", "unknown_alias:cache"],
    );
}

/// BASE: `Unsafe`, `[leak]`. Refused *before* the call's effects apply, so no
/// site exists to report — the same withdrawal the direct acquisition arm
/// performs, and a downgrade rather than a promotion.
#[test]
fn an_out_publication_without_a_release_withdraws_its_leak_claim() {
    if !clang_available() {
        return;
    }
    refused(
        "out-into-global-unreleased",
        "void *g_published; void w(int n) { y_fill(&g_published); }",
        &["publication:g_published"],
    );
}

/// The control: an `Out` slot naming an ordinary local is untouched, so the
/// destination half of the out rule is load-bearing too.
#[test]
fn an_out_slot_naming_an_ordinary_local_is_untouched() {
    if !clang_available() {
        return;
    }
    admitted(
        "out-into-local",
        "void w(int n) { void *loc; y_fill(&loc); y_release(loc); }",
    );
}

/// BASE: `Unsafe`, `[leak, indirect memory access …]` for each. The statement
/// keeps the refusal it already had — attribution stays with the gate that read
/// it — and additionally withdraws the residual claim, because the destination
/// is a continued access on an object that outlives the call. Wave 16 asserted
/// these shapes gained **no** diagnostic of its card; that pin is deliberately
/// updated here, with its load-bearing half — the pre-existing refusal survives
/// — kept.
#[test]
fn an_indirect_publication_withdraws_its_leak_claim() {
    if !clang_available() {
        return;
    }
    refused(
        "file-scope aggregate member, unreleased",
        "struct b_reg { struct b_handle *slot; }; struct b_reg g_reg; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_reg.slot = h; }",
        &[
            "publication:g_reg",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
    refused(
        "through a file-scope pointer, unreleased",
        "struct b_reg { struct b_handle *slot; }; struct b_reg *g_reg; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_reg->slot = h; }",
        &[
            "publication:g_reg",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
    refused(
        "file-scope array element, unreleased",
        "struct b_handle *g_slots[4]; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_slots[0] = h; }",
        &[
            "publication:g_slots",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
    refused(
        "static local aggregate member, unreleased",
        "struct b_reg { struct b_handle *slot; }; \
         void w(int n) { static struct b_reg s; struct b_handle *h; h = b_open(0,0,n); \
         if (!h) { return; } s.slot = h; }",
        &[
            "publication:s",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}

/// The control: the identical store into a **local** aggregate publishes
/// nothing. The destination half of the indirect rule is load-bearing. Its
/// pre-existing `Leak` is withdrawn since card W44-LEAK-PIN-AUDIT's refused
/// hand-on rule, because the refused store hands `h` on.
#[test]
fn an_indirect_store_into_a_local_aggregate_publishes_nothing() {
    if !clang_available() {
        return;
    }
    let (head, keys) = verdict(
        "struct b_reg { struct b_handle *slot; }; \
         void w(int n) { struct b_reg local; struct b_handle *h; h = b_open(0,0,n); \
         if (!h) { return; } local.slot = h; }",
    );
    // It kept its `Leak` until card W44-LEAK-PIN-AUDIT: the refused store
    // hands `h` on as its value, so the refused hand-on rule withdraws the
    // accusation (`Unknown`, never `Safe`). The destination half is still
    // load-bearing for the publication rule: nothing is published.
    assert!(head.starts_with("unknown"), "{head}");
    assert!(
        !keys.contains(&"leak".to_owned()),
        "a leak through a refused hand-on is not claimed: {keys:?}"
    );
    assert!(
        !keys.iter().any(|key| key.starts_with("publication:")),
        "no publication may be reported for a local destination: {keys:?}"
    );
}

/// The recorded residual, pinned as measured rather than left unstated: a store
/// through a **local pointer** that was assigned the address of a publication
/// destination is not a recognised publication, because the base this scan
/// reads is the local `p`. Deciding it needs a pointer relation the projection
/// does not have. Named `CTWIN-PUBLISH-INDIRECT-POINTER`. It kept its `Leak`
/// until card W44-LEAK-PIN-AUDIT: the refused store `*p = h` hands `h` on as
/// its value, so the refused-escape rule withdraws the accusation and the
/// function is `Unknown` — still never `Safe`, and still no publication.
#[test]
fn a_store_through_a_local_pointer_to_the_object_keeps_its_pre_existing_answer() {
    if !clang_available() {
        return;
    }
    let (head, keys) = verdict(
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle **p = &g_published; struct b_handle *h; \
         h = b_open(0,0,n); if (!h) { return; } *p = h; }",
    );
    assert!(head.starts_with("unknown"), "{head}");
    assert!(!keys.contains(&"leak".to_owned()), "{keys:?}");
    assert!(
        !keys.iter().any(|key| key.starts_with("publication:")),
        "this scan reads the base `p`, which is not a destination: {keys:?}"
    );
}

/// A compound assignment reads its destination and stores an arithmetic value,
/// never a pointer this projection tracks, so it is not a publication — and the
/// `=` reader must not mistake `==` for one either. Both statements sit in a
/// body the projection already refuses for unrelated, pre-existing reasons
/// (`unrecognized assignment destination`, `unmodeled … branch condition`),
/// which is why the assertion is that **no** publication is reported rather
/// than that the body certifies.
#[test]
fn a_compound_assignment_and_a_comparison_are_not_publications() {
    if !clang_available() {
        return;
    }
    let (_, keys) = verdict(
        "struct b_counters { int hits; }; struct b_counters g_reg; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_reg.hits += n; if (g_reg.hits == n) { b_write(h, n); } b_close(h); }",
    );
    assert!(
        !keys.iter().any(|key| key.starts_with("publication:")),
        "neither `+=` nor `==` is a simple assignment: {keys:?}"
    );
}

/// The rule reads no **provenance**, exactly as it reads no per-site status: a
/// body-acquired identity and a caller-held one are refused alike. Measured on
/// two bodies that differ in nothing else.
#[test]
fn the_rule_reads_no_provenance() {
    if !clang_available() {
        return;
    }
    let body_acquired = verdict(
        "struct b_handle *g_published; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_published = h; b_close(h); }",
    );
    let caller_held = verdict(
        "struct b_handle *g_published; \
         static void publish(struct b_handle *q) { g_published = q; } \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         publish(h); b_close(h); }",
    );
    assert_ne!(body_acquired.0, "safe");
    assert_ne!(caller_held.0, "safe");
}

/// The corpus's own witness, reduced: `librdkafka/crc32c.c`'s `crc32c_init_sw`
/// fills a file-scope table with `crc32c_table[k][n] = crc;`, where every
/// operand is a declared scalar. A first implementation of the indirect record
/// tested only whether the source *name* was present in the binding map — which
/// an ordinary scalar assignment makes true — and the pinned corpus answered
/// with two refusals at exactly those two lines. That is refusal without a
/// property, so the value test was tightened to the direct arm's: a source must
/// denote at least one site and must not be a declared scalar.
///
/// Pinned in both directions here, so the imprecision cannot come back.
#[test]
fn a_scalar_table_fill_is_not_an_indirect_publication() {
    if !clang_available() {
        return;
    }
    let (_, keys) = verdict(
        "int g_table[8][256]; \
         void w(int n) { int k; int crc; crc = n; \
         for (k = 0; k < 8; k = k + 1) { g_table[k][n] = crc; } }",
    );
    assert!(
        !keys.iter().any(|key| key.starts_with("publication:")),
        "every operand is a scalar, so nothing is published: {keys:?}"
    );
}

/// And the control that keeps the tightening from disabling the rule: the same
/// destination shape with a **tracked** source still refuses and still withdraws
/// its residual claim.
#[test]
fn a_tracked_source_into_the_same_shape_still_refuses() {
    if !clang_available() {
        return;
    }
    refused(
        "tracked-into-array",
        "struct b_handle *g_slots[4]; \
         void w(int n) { struct b_handle *h; h = b_open(0,0,n); if (!h) { return; } \
         g_slots[n] = h; }",
        &[
            "publication:g_slots",
            "unsupported:unrecognized assignment destination",
        ],
    );
}
