//! C02-P-ALIAS — the bounded may-alias refinement
//! (`docs/reports/remaining-c02-p-alias/README.md`).
//!
//! Card C02-P recorded, and deliberately did not repair, that two *distinct
//! declared names* may denote the same object and that a name-based projection
//! relates them not at all (`docs/reports/remaining-c02-p/README.md` §6). This
//! file is the held-out acceptance family for the repair: a release through one
//! name leaves a modeled read of a same-typed peer **unresolved**, never
//! `Safe`, unless the body discharges the pair.
//!
//! Every body below was measured at the branch base `695f0e1` before the
//! refinement landed; the `BASE:` line of each test says what it answered
//! there, so every assertion is a *measured* movement or a *measured*
//! preservation rather than an assumption.
//!
//! Every fixture uses the generic `b_*` API card C02-P introduced, deliberately
//! unrelated to ChunkIO, Fluent Bit and O04's `g_*` family, so no name in it can
//! matter. `Safe` here is `Safe` for a synthetic declared custody projection
//! over trusted test assumptions; nothing here promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/c02p_generic.qcore");

/// C02-P's declarations plus `struct b_other`, a second incomplete handle
/// record, so that "the same declared record-pointer type" is a real premise
/// with a measured negative rather than a vacuous one.
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

fn contracts() -> ResourceContracts {
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
            eprintln!("skipping Clang-dependent C02-P-ALIAS test: {reason}");
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

/// The `place/peer` composites the may-alias rule reports, in a stable order.
/// Every other diagnostic — including the ordinary `UnknownAlias` a lost W4
/// summary produces once this rule refuses a body — is deliberately excluded,
/// so each assertion names exactly this card's obligations.
fn obligations(summary: &LegacyFunctionSummary) -> Vec<String> {
    let mut names = summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } if variable.contains('/') => {
                Some(variable.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

/// A body the rule refuses: never `Safe`, carrying exactly the expected
/// obligations.
fn refused(label: &str, body: &str, expected: &[&str]) {
    let result = analyze(body);
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a may-aliased read must never certify"
    );
    let w = entry(program(&result));
    assert_eq!(
        w.status,
        AnalysisStatus::Unknown,
        "{label}: {:?}",
        w.diagnostics
    );
    assert_eq!(
        obligations(w),
        expected
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>(),
        "{label}: {:?}",
        w.diagnostics
    );
}

/// A body the rule leaves alone: the `Safe` answer it had before this card,
/// with no obligation outstanding and a certificate that rechecks.
fn admitted(label: &str, body: &str) -> LegacySafetyCertificate {
    match analyze(body) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            let w = entry(&certificate.summary);
            assert_eq!(w.status, AnalysisStatus::Safe, "{label}");
            assert!(
                obligations(w).is_empty(),
                "{label}: no obligation should remain: {:?}",
                w.diagnostics
            );
            check_legacy_safety_certificate(&certificate)
                .unwrap_or_else(|error| panic!("{label}: certificate rechecks: {error}"));
            certificate
        }
        other => panic!("{label}: expected a validated custody projection: {other:?}"),
    }
}

// --------------------------------------------------------------------------
// The measured shape of C02-P §6, in both its forms
// --------------------------------------------------------------------------

/// The shape the card exists for. Two formals of one declared record-pointer
/// class; nothing establishes `a != b`; releasing `a` and then borrowing `b`
/// answered `Safe`.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_measured_plain_name_control_no_longer_certifies() {
    if !clang_available() {
        return;
    }
    refused(
        "the measured plain-name control",
        "void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }",
        &["b/a"],
    );
}

/// The field-read form of the same boundary: the released formal `dup` and the
/// local `storage` share `struct b_storage *`, and `storage` is the base of the
/// two borrowed pointer-field actuals C02-P admits. The admission is withdrawn
/// exactly where the base may be the released object.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_measured_field_read_shape_no_longer_certifies() {
    if !clang_available() {
        return;
    }
    refused(
        "the measured field-read form",
        "struct b_handle *w(struct b_instance *in, struct b_storage *dup, int n) { \
         struct b_storage *storage; struct b_handle *chunk; b_release(dup); \
         storage = in->storage; chunk = b_open(storage->cio, storage->stream, n); \
         if (!chunk) { return 0; } b_close(chunk); return 0; }",
        &["storage/dup"],
    );
}

/// C02-P pinned that the two forms answer *identically*, so that a later
/// increment relating the two names would move both rows together. It does:
/// the pin flips from `Safe == Safe` to `Unknown == Unknown` by its own design,
/// and the agreement itself is the load-bearing half this card keeps.
#[test]
fn both_forms_of_the_boundary_still_answer_identically() {
    if !clang_available() {
        return;
    }
    let control =
        analyze("void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }");
    let with_field_read = analyze(
        "struct b_handle *w(struct b_instance *in, struct b_storage *dup, int n) { \
         struct b_storage *storage; struct b_handle *chunk; b_release(dup); \
         storage = in->storage; chunk = b_open(storage->cio, storage->stream, n); \
         if (!chunk) { return 0; } b_close(chunk); return 0; }",
    );
    let control = entry(program(&control)).status;
    let field_read = entry(program(&with_field_read)).status;
    assert_eq!(control, field_read);
    assert_eq!(control, AnalysisStatus::Unknown);
}

// --------------------------------------------------------------------------
// The reach of the relation
// --------------------------------------------------------------------------

/// Three formals of one class: one release compromises both survivors, and
/// each obligation names its own peer.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_three_formal_class_refuses_every_survivor() {
    if !clang_available() {
        return;
    }
    refused(
        "three formals of one class",
        "void w(struct b_handle *a, struct b_handle *b, struct b_handle *c) \
         { b_release(a); b_probe(b); b_probe(c); }",
        &["b/a", "c/a"],
    );
}

/// A local copy of a formal is a peer of it, and the relation is symmetric:
/// releasing either name compromises a read of the other.
///
/// BASE: both `Safe`, no diagnostics.
#[test]
fn a_same_type_local_copy_is_refused_in_both_directions() {
    if !clang_available() {
        return;
    }
    refused(
        "release the formal, read the local copy",
        "void w(struct b_handle *a) { struct b_handle *held; held = a; \
         b_release(a); b_probe(held); }",
        &["held/a"],
    );
    refused(
        "release the local copy, read the formal",
        "void w(struct b_handle *a) { struct b_handle *held; held = a; \
         b_release(held); b_probe(a); }",
        &["a/held"],
    );
}

/// A release recorded on one arm reaches the join, because `released_names` is
/// joined by union. `one` is a body-made acquisition and `b` a formal, so the
/// pair is in scope in v1.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_release_on_one_arm_reaches_the_join() {
    if !clang_available() {
        return;
    }
    refused(
        "released on one arm only",
        "void w(struct b_handle *b, int n) { struct b_handle *one; one = 0; \
         if (n) { one = b_open(0, 0, n); if (!one) { return; } b_release(one); } \
         b_probe(b); }",
        &["b/one"],
    );
}

/// Rebinding the *releasing* name does not resurrect the object it released,
/// so the peer stays refused. This is why `released_names` is monotone and is
/// deliberately not cleared by `AbstractState::rebind`.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn rebinding_the_releasing_name_does_not_discharge_its_peers() {
    if !clang_available() {
        return;
    }
    refused(
        "the releasing name is reassigned afterwards",
        "void w(struct b_handle *b, int n) { struct b_handle *one; \
         one = b_open(0, 0, n); if (!one) { return; } b_release(one); \
         one = b_open(0, 0, n); if (!one) { return; } b_probe(b); b_close(one); }",
        &["b/one"],
    );
}

// --------------------------------------------------------------------------
// The bounds of the relation — each a *retained* pre-existing answer
// --------------------------------------------------------------------------

/// The relation is per declared record tag. A different record is a different
/// class, and the pre-existing answer stands.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_different_declared_record_is_never_a_peer() {
    if !clang_available() {
        return;
    }
    admitted(
        "distinct record tags",
        "void w(struct b_handle *a, struct b_other *b) { b_release(a); b_probe(b); }",
    );
}

/// A body with no release at all is untouched, even when its names share a
/// class: the rule is triggered by a release, not by a declaration.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_class_with_no_release_is_untouched() {
    if !clang_available() {
        return;
    }
    admitted(
        "same class, nothing released",
        "void w(struct b_handle *a, struct b_handle *b) { b_probe(a); b_probe(b); }",
    );
}

/// The v1 blast-radius bound `C02-P-ALIAS-LOCALS`: two *locals* are related by
/// nothing here. This body's answer is **byte-for-byte the one it had before
/// the card** — `Unknown` with the single ordinary `UnknownAlias` its
/// unresolvable release already produced, and no obligation of this rule. It is
/// a recorded boundary, not an endorsement: `one` and `two` are read from the
/// same field place and may well denote the same object.
///
/// BASE: `Unknown`, `[unknown_alias one]`.
#[test]
fn two_locals_keep_the_pre_existing_answer_in_v1() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void w(struct b_instance *in) { struct b_storage *one; struct b_storage *two; \
         one = in->storage; two = in->storage; b_release(one); b_probe(two); }",
    );
    let w = entry(program(&result));
    assert_eq!(w.status, AnalysisStatus::Unknown);
    assert!(obligations(w).is_empty(), "{:?}", w.diagnostics);
    assert_eq!(
        w.diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect::<Vec<_>>(),
        vec![LegacyDiagnosticKind::UnknownAlias {
            variable: "one".to_owned()
        }],
        "the pre-existing answer must be preserved exactly"
    );
}

// --------------------------------------------------------------------------
// Discharge 1 — a pointer-inequality guard
// --------------------------------------------------------------------------

/// The release and the read are both on the edge where `a != b` holds, so the
/// pair is proved distinct and the body keeps its `Safe` answer. The `else`
/// arm releases too, so the enclosing wrapper still carries a validated
/// `Released` custody summary and the comparison is against a real `Safe`.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_pointer_inequality_guard_discharges_the_pair() {
    if !clang_available() {
        return;
    }
    admitted(
        "guarded by a != b",
        "void w(struct b_handle *a, struct b_handle *b) \
         { if (a != b) { b_release(a); b_probe(b); } else { b_release(a); } }",
    );
}

/// Its measured control: the identical body with the guard replaced by an
/// unrelated condition is refused. The discharge is the guard, not the shape.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_same_body_without_the_guard_is_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "the same shape guarded by an unrelated condition",
        "void w(struct b_handle *a, struct b_handle *b, int n) \
         { if (n) { b_release(a); b_probe(b); } else { b_release(a); } }",
        &["b/a"],
    );
}

/// The `a == b` spelling, with the work in the `else` arm. Only the edge on
/// which the two names are distinct carries the fact.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_equality_spelling_of_the_guard_discharges_its_else_arm() {
    if !clang_available() {
        return;
    }
    admitted(
        "guarded by a == b, work in the else arm",
        "void w(struct b_handle *a, struct b_handle *b) \
         { if (a == b) { b_release(a); } else { b_release(a); b_probe(b); } }",
    );
}

/// And the fail-closed direction of the same guard: the arm on which the two
/// names may be **equal** discharges nothing at all.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_arm_where_the_names_may_be_equal_is_still_refused() {
    if !clang_available() {
        return;
    }
    refused(
        "work on the may-be-equal arm",
        "void w(struct b_handle *a, struct b_handle *b) \
         { if (a != b) { b_release(a); } else { b_release(a); b_probe(b); } }",
        &["b/a"],
    );
}

/// A guard proved before either name is reassigned stops discharging: the
/// inequality was about the old value.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_guard_does_not_survive_a_reassignment_of_either_name() {
    if !clang_available() {
        return;
    }
    refused(
        "the guarded name is reassigned inside the guard",
        "void w(struct b_handle *a, struct b_handle *b, int n) \
         { if (a != b) { b_release(a); a = b_open(0, 0, n); \
         if (!a) { b_probe(b); return; } b_probe(b); b_close(a); } \
         else { b_release(a); } }",
        &["b/a"],
    );
}

// --------------------------------------------------------------------------
// Discharge 2 — provenance
// --------------------------------------------------------------------------

/// `fresh` is bound to exactly one live identity this body acquired, so no
/// release of a caller-held peer can reach it.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_body_made_acquisition_discharges_its_own_reads() {
    if !clang_available() {
        return;
    }
    admitted(
        "the read place was acquired here",
        "void w(struct b_handle *a, int n) { struct b_handle *fresh; b_release(a); \
         fresh = b_open(0, 0, n); if (!fresh) { return; } b_probe(fresh); b_close(fresh); }",
    );
}

/// The provenance discharge is about *this body's* acquisitions. A formal
/// carrying declared entry custody is a caller-held identity, and two such
/// formals are exactly the pair this card exists for, so entry custody must
/// never discharge. Both names here carry entry custody and the read is still
/// refused.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn caller_held_entry_custody_never_discharges() {
    if !clang_available() {
        return;
    }
    refused(
        "both names carry caller-held custody",
        "void w(struct b_handle *a, struct b_handle *b) \
         { b_release(a); b_write(b, 1); b_close(b); }",
        &["b/a"],
    );
}

/// Provenance also requires *liveness*. A place this body acquired and then
/// released is not discharged — and this is the C00 lesson: the pre-refinement
/// answer for this body was `Safe`, because a `Borrowed` position of a released
/// identity was a no-op.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_released_body_acquisition_no_longer_discharges() {
    if !clang_available() {
        return;
    }
    refused(
        "the acquired place was released before the read",
        "void w(struct b_handle *a, int n) { struct b_handle *fresh; b_release(a); \
         fresh = b_open(0, 0, n); if (!fresh) { return; } b_close(fresh); b_probe(fresh); }",
        &["fresh/a"],
    );
}

// --------------------------------------------------------------------------
// The rule adds refusals only
// --------------------------------------------------------------------------

/// C02-P's own admitted constructor shape has three distinct record classes and
/// releases nothing that could be a peer, so its `Safe` certificate is
/// untouched.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_c02_p_constructor_shape_is_unaffected() {
    if !clang_available() {
        return;
    }
    let certificate = admitted(
        "the C02-P constructor shape",
        "struct b_handle *w(struct b_instance *in, int n) { int ret; \
         struct b_storage *storage; struct b_handle *chunk; storage = in->storage; \
         chunk = b_open(storage->cio, storage->stream, n); if (!chunk) { return 0; } \
         ret = b_probe(chunk); if (ret == -1) { b_close(chunk); return 0; } \
         ret = b_write(chunk, n); if (ret == -1) { b_close(chunk); return 0; } return chunk; }",
    );
    assert!(entry(&certificate.summary).diagnostics.is_empty());
}

/// The refusal never converts an `Unsafe` into something softer: the leak
/// behind a may-aliased read is still reported, with the obligation beside it.
///
/// BASE: `Unsafe`, `[leak, unknown_alias a]`.
#[test]
fn a_leak_is_still_reported_behind_a_refused_read() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void w(struct b_handle *a, struct b_handle *b, int n) { struct b_handle *chunk; \
         b_release(a); chunk = b_open(0, 0, n); if (!chunk) { return; } b_probe(b); }",
    );
    let w = entry(program(&result));
    assert_eq!(w.status, AnalysisStatus::Unsafe, "{:?}", w.diagnostics);
    assert!(
        w.diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{:?}",
        w.diagnostics
    );
    assert_eq!(obligations(w), vec!["b/a".to_owned()]);
}

// --------------------------------------------------------------------------
// Independent rederivation
// --------------------------------------------------------------------------

/// The refusal rests on the *declared* types of the two names, read from the
/// same preprocessed token stream the certificate carries. Substituting only a
/// declaration, so that a previously unrelated pair becomes one class, must be
/// refused by the independent recheck with every statement byte-identical.
#[test]
fn a_certificate_resting_on_the_distinct_classes_is_rederived_from_the_declarations() {
    if !clang_available() {
        return;
    }
    let certificate = admitted(
        "distinct record tags",
        "void w(struct b_handle *a, struct b_other *b) { b_release(a); b_probe(b); }",
    );
    let substitutions: [(&str, &str, &str); 2] = [
        (
            "the second formal joins the first's class",
            "struct b_other *b)",
            "struct b_handle *b)",
        ),
        (
            "the first formal joins the second's class",
            "void w(struct b_handle *a,",
            "void w(struct b_other *a,",
        ),
    ];
    for (label, from, to) in substitutions {
        let mut mutated = certificate.clone();
        assert!(
            mutated.preprocessed_source.contains(from),
            "{label}: anchor not found"
        );
        mutated.preprocessed_source = mutated.preprocessed_source.replacen(from, to, 1);
        let error = check_legacy_safety_certificate(&mutated)
            .expect_err(&format!("{label}: the substitution must be refused"));
        assert!(
            error.contains("do not match the preprocessed source")
                || error.contains("does not match the preprocessed source")
                || error.contains("is not independently Safe")
                || error.contains("retains an unknown instruction"),
            "{label}: unexpected refusal `{error}`"
        );
    }
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}

/// And a certificate resting on the guard is refused when the guard changes
/// sense, leaving every other byte alone.
#[test]
fn a_certificate_resting_on_the_guard_is_refused_when_the_guard_changes() {
    if !clang_available() {
        return;
    }
    let certificate = admitted(
        "guarded by a != b",
        "void w(struct b_handle *a, struct b_handle *b) \
         { if (a != b) { b_release(a); b_probe(b); } else { b_release(a); } }",
    );
    let mut mutated = certificate.clone();
    assert!(mutated.preprocessed_source.contains("if (a != b)"));
    mutated.preprocessed_source =
        mutated
            .preprocessed_source
            .replacen("if (a != b)", "if (a == b)", 1);
    let error =
        check_legacy_safety_certificate(&mutated).expect_err("reversing the guard must be refused");
    assert!(
        error.contains("do not match the preprocessed source")
            || error.contains("does not match the preprocessed source")
            || error.contains("is not independently Safe")
            || error.contains("retains an unknown instruction"),
        "unexpected refusal `{error}`"
    );
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}
