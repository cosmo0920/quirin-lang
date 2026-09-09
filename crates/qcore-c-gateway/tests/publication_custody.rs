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

const CONTRACTS: &str = include_str!("fixtures/contracts/c02p_generic.qcore");

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
struct b_storage *b_pick(struct b_instance *in);";

fn contracts() -> qcore_c_gateway::legacy_c::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "c02p_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the generic contract fixture loads")
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

/// BASE: `Safe`. Publishing a **borrowed** pointer — one whose identity the
/// caller holds — is a lifetime question this projection never claimed to
/// decide. It stays admitted; the boundary is recorded as
/// `CTWIN-PUBLISH-BORROWED`.
#[test]
fn publishing_a_borrowed_pointer_is_not_newly_refused() {
    if !clang_available() {
        return;
    }
    admitted(
        "publish-borrowed",
        "struct b_handle *g_published; \
         void w(struct b_handle *p, int n) { g_published = p; }",
    );
}

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
    let expected = &[
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
        expected,
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
        assert_eq!(
            verdict(plain),
            verdict(qualified),
            "{label}: a `volatile` qualifier must move no resource-projection verdict, \
             because this projection derives no synchronization fact for it to change"
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
