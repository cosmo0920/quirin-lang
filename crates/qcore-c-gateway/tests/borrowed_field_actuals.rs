//! C02-P — borrowed pointer-field reads as contracted-call actuals
//! (`docs/reports/remaining-c02-p/README.md`).
//!
//! Card C02 admitted one conjunctive whitelist of direct *scalar* field
//! accesses as custody-neutral no-ops and left every pointer field refused
//! (`docs/reports/remaining-c02/README.md` §7). This file is the held-out
//! acceptance family for the pointer-field extension: a read of a pointer
//! member is admitted **only** in the actual-argument position of a call whose
//! contract in force declares that position `Borrowed`, and in the right-hand
//! side of an assignment to a plain local, which binds an opaque value.
//!
//! Every fixture is a `b_*` API deliberately unrelated to ChunkIO, to Fluent
//! Bit and to O04's `g_*` family, so no name in it can matter. `Safe` here is
//! `Safe` for a synthetic declared custody projection over trusted test
//! assumptions; nothing in this file promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, SiteDisposition,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/c02p_generic.qcore");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

/// Declarations shared by every fixture. `b_untracked` is declared in C and
/// deliberately absent from the contract file: a callee without a contract in
/// force never admits an actual.
/// `b_probe` accepts any object pointer so the address-of-member refusal is
/// valid C and reaches QCore's projection without an incompatible-pointer error
/// or an explicit cast that would test a different refused shape.
const DECLARATIONS: &str = "\
struct b_ctx; struct b_pool; struct b_handle; \
struct b_storage { int state; struct b_ctx *cio; struct b_pool *stream; }; \
struct b_instance { int flags; struct b_storage *storage; }; \
union b_view { struct b_ctx *cio; int tag; }; \
struct b_holder { int kind; union b_view *view; }; \
struct b_handle *b_open(struct b_ctx *cio, struct b_pool *stream, int flags); \
void b_close(struct b_handle *h); \
int b_probe(void *h); \
int b_write(struct b_handle *h, int n); \
void b_adopt(void *p); \
void b_release(void *p); \
void b_untracked(struct b_ctx *cio); \
void b_note(int first, ...); \
struct b_storage *b_pick(struct b_instance *in);";

/// The admitted shape, mirroring the real `flb_input_chunk_create` call of
/// `cio_chunk_open`: one opaque field read binds a local aggregate pointer,
/// and two of that aggregate's pointer members occupy declared borrowed
/// positions of the constructor.
const CREATE: &str = "\
struct b_handle *w_create(struct b_instance *in, int n) \
{ \
    int ret; \
    struct b_storage *storage; \
    struct b_handle *chunk; \
    storage = in->storage; \
    chunk = b_open(storage->cio, storage->stream, n); \
    if (!chunk) { return 0; } \
    ret = b_probe(chunk); \
    if (ret == -1) { b_close(chunk); return 0; } \
    ret = b_write(chunk, n); \
    if (ret == -1) { b_close(chunk); return 0; } \
    return chunk; \
}";

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
            eprintln!("skipping Clang-dependent C02-P test: {reason}");
            false
        }
    }
}

fn analyze(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
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

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn certified(body: &str, entry: &str) -> LegacySafetyCertificate {
    match analyze(body, entry) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
            certificate
        }
        other => panic!("expected a validated custody projection: {other:?}"),
    }
}

/// Assert that a body keeps the pre-C02-P answer: `Unknown`, carrying the
/// existing indirect-access diagnostic, and never `Safe`.
fn refused(label: &str, body: &str) {
    let result = analyze(body, "w");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a refused shape must never certify"
    );
    let w = function(program(&result), "w");
    assert_eq!(w.status, AnalysisStatus::Unknown, "{label}");
    assert!(
        w.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description } if description == INDIRECT
        )),
        "{label}: the existing diagnostic must be retained, got {:?}",
        w.diagnostics
    );
}

// --------------------------------------------------------------------------
// Positive
// --------------------------------------------------------------------------

/// The shape the card exists for: the constructor call is no longer rejected
/// before its declared contract is consulted, so the returned identity binds
/// and every arm of the caller settles.
#[test]
fn a_constructor_call_reading_two_borrowed_pointer_fields_is_admitted() {
    if !clang_available() {
        return;
    }
    let certificate = certified(CREATE, "w_create");
    let create = function(&certificate.summary, "w_create");
    assert_eq!(create.status, AnalysisStatus::Safe);
    assert!(
        create.diagnostics.is_empty(),
        "the admitted statement leaves no diagnostic: {:?}",
        create.diagnostics
    );
    let shape = create
        .exits
        .iter()
        .map(|exit| {
            (
                exit.custody,
                exit.sites
                    .iter()
                    .map(|site| site.disposition)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        shape
            .iter()
            .all(|(custody, _)| *custody == ExitCustody::Discharged),
        "every exit of a fully modeled path settles: {shape:?}"
    );
    assert_eq!(
        shape
            .iter()
            .filter(|(_, sites)| sites == &vec![SiteDisposition::Released])
            .count(),
        2,
        "two cleanup arms release: {shape:?}"
    );
}

/// The admission is a precision gain in the defect direction too: with the
/// constructor call admitted, a missing release on one arm is now *reported*
/// instead of being hidden behind the refused statement.
#[test]
fn a_leak_behind_the_admitted_constructor_call_is_now_reported() {
    if !clang_available() {
        return;
    }
    let leaky = CREATE.replacen(
        "if (ret == -1) { b_close(chunk); return 0; }",
        "if (ret == -1) { return 0; }",
        1,
    );
    assert_ne!(leaky, CREATE, "the first cleanup arm was removed");
    let result = analyze(&leaky, "w_create");
    let create = function(program(&result), "w_create");
    assert_eq!(create.status, AnalysisStatus::Unsafe);
    assert!(
        create
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "diagnostics: {:?}",
        create.diagnostics
    );
}

/// The `.` form of the same rule: a record-valued local's pointer members
/// occupy borrowed positions.
#[test]
fn a_record_valued_base_is_admitted_through_the_dot_form() {
    if !clang_available() {
        return;
    }
    let certificate = certified(
        "struct b_handle *w_local(int n) { struct b_storage local; struct b_handle *chunk; \
         chunk = b_open(local.cio, local.stream, n); if (!chunk) { return 0; } \
         b_close(chunk); return 0; }",
        "w_local",
    );
    let local = function(&certificate.summary, "w_local");
    assert_eq!(local.status, AnalysisStatus::Safe);
    assert!(local.diagnostics.is_empty(), "{:?}", local.diagnostics);
}

// --------------------------------------------------------------------------
// Adversarial — every one of these must keep the pre-C02-P answer
// --------------------------------------------------------------------------

/// A base the projection released is never an admitted base. This is the
/// premise C02 §2 had to add after an implementation of C00's recommendation
/// turned a read through a released pointer into `Safe`; the borrowed-actual
/// rule inherits it, and a released base is stronger evidence than a merely
/// bound one.
#[test]
fn a_released_base_is_never_admitted() {
    if !clang_available() {
        return;
    }
    refused(
        "the base was released before the read",
        "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
         struct b_handle *chunk; storage = in->storage; b_release(storage); \
         chunk = b_open(storage->cio, storage->stream, n); if (!chunk) { return 0; } \
         b_close(chunk); return 0; }",
    );
}

/// A base consumed through a direct lexical alias is still a bound base.
#[test]
fn an_alias_of_a_consumed_base_is_never_admitted() {
    if !clang_available() {
        return;
    }
    refused(
        "the base is consumed through an alias",
        "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
         struct b_storage *alias; struct b_handle *chunk; storage = in->storage; \
         alias = storage; b_adopt(alias); \
         chunk = b_open(storage->cio, storage->stream, n); if (!chunk) { return 0; } \
         b_close(chunk); return 0; }",
    );
}

/// Two locals read from the *same* field place denote the same object even
/// though neither is a lexical alias of the other. Releasing one must refuse
/// the other as a base.
#[test]
fn a_second_read_of_a_released_field_place_is_never_admitted() {
    if !clang_available() {
        return;
    }
    refused(
        "the same field place was read into two locals and one was released",
        "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
         struct b_storage *other; struct b_handle *chunk; storage = in->storage; \
         other = in->storage; b_release(other); \
         chunk = b_open(storage->cio, storage->stream, n); if (!chunk) { return 0; } \
         b_close(chunk); return 0; }",
    );
}

/// A store through the field between the read and the call is a destination
/// the lexical gate still refuses, so the whole function keeps its `Unknown`.
#[test]
fn a_store_through_the_field_between_the_read_and_the_call_is_never_admitted() {
    if !clang_available() {
        return;
    }
    refused(
        "the field is mutated between the read and the call",
        "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
         struct b_handle *chunk; storage = in->storage; storage->cio = 0; \
         chunk = b_open(storage->cio, storage->stream, n); if (!chunk) { return 0; } \
         b_close(chunk); return 0; }",
    );
}

/// The refusal of a base is flow-insensitive: a later opaque reassignment
/// refuses the name at every earlier read too.
#[test]
fn a_later_opaque_reassignment_of_the_base_refuses_every_earlier_read() {
    if !clang_available() {
        return;
    }
    refused(
        "the base is reassigned from an uncontracted call later in the body",
        "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
         struct b_handle *chunk; storage = in->storage; \
         chunk = b_open(storage->cio, storage->stream, n); storage = b_pick(in); \
         if (!chunk) { return 0; } b_close(chunk); return 0; }",
    );
}

/// The rest of the whitelist, one premise violated per row.
#[test]
fn every_other_refused_shape_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 10] = [
        (
            "a partial argument is not a whole member read",
            "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
             struct b_handle *chunk; storage = in->storage; \
             chunk = b_open(storage->cio, storage->stream, storage->state + n); \
             if (!chunk) { return 0; } b_close(chunk); return 0; }",
        ),
        (
            "a parenthesized actual is not a bare member read",
            "struct b_handle *w(struct b_instance *in, int n) { struct b_storage *storage; \
             struct b_handle *chunk; storage = in->storage; \
             chunk = b_open((storage->cio), storage->stream, n); \
             if (!chunk) { return 0; } b_close(chunk); return 0; }",
        ),
        (
            "a callee with no contract in force never admits an actual",
            "void w(struct b_instance *in) { struct b_storage *storage; storage = in->storage; \
             b_untracked(storage->cio); }",
        ),
        (
            "a consuming position is not a borrowed position",
            "void w(struct b_instance *in) { struct b_storage *storage; storage = in->storage; \
             b_adopt(storage->cio); }",
        ),
        (
            "a releasing position is not a borrowed position",
            "void w(struct b_instance *in) { struct b_storage *storage; storage = in->storage; \
             b_release(storage->cio); }",
        ),
        (
            "an actual beyond the declared parameter vector is never admitted",
            "void w(struct b_instance *in) { struct b_storage *storage; storage = in->storage; \
             b_note(0, storage->cio); }",
        ),
        // Deliberately widened by card O04-ARMS. C02-P refused every chain by
        // construction; the bounded chain rule now admits one whose every
        // intermediate link is declared exactly `struct POINTEE *` over a
        // complete, unambiguous, non-union record, so `in->storage->cio` is
        // modeled (pinned positively in `o04_arms.rs`). The load-bearing half
        // is kept here on a chain the reader did *not* fully type: `view` is a
        // `union` pointer, so the walk refuses at its intermediate link.
        (
            "a chain through a link the reader did not type is not a direct member read",
            "struct b_handle *w(struct b_holder *hold, int n) { struct b_handle *chunk; \
             chunk = b_open(hold->view->cio, 0, n); b_close(chunk); return 0; }",
        ),
        (
            "a union base is never admitted",
            "struct b_handle *w(union b_view *v, int n) { struct b_handle *chunk; \
             chunk = b_open(v->cio, 0, n); b_close(chunk); return 0; }",
        ),
        (
            "an address-of escape of the member is never admitted",
            "void w(struct b_instance *in) { struct b_storage *storage; storage = in->storage; \
             b_probe(&storage->cio); }",
        ),
        (
            "a cast operand is not a bare member read",
            "struct b_handle *w(struct b_instance *in, int n) { struct b_handle *chunk; \
             struct b_storage *storage; storage = in->storage; \
             chunk = b_open((struct b_ctx *) storage->cio, storage->stream, n); \
             b_close(chunk); return 0; }",
        ),
    ];
    for (label, body) in cases {
        refused(label, body);
    }
}

/// The classification is repeated by W4 derivation, W4 validation and the
/// independent certificate recheck, all from the certificate's own content: a
/// callee that is summarizable **only** because its constructor call was
/// admitted publishes an `InferredValidated` record that the recheck re-derives.
#[test]
fn a_custody_summary_derived_through_an_admitted_call_is_validated_and_rederived() {
    if !clang_available() {
        return;
    }
    let user = "void w_use(struct b_instance *in) { struct b_handle *chunk; \
                chunk = w_create(in, 1); if (!chunk) { return; } b_close(chunk); }";
    let certificate = certified(&format!("{CREATE} {user}"), "w_use");
    let summary = certificate
        .summaries
        .iter()
        .find(|record| record.function == "w_create")
        .expect("a published summary for the admitted wrapper");
    assert_eq!(
        summary.trust,
        qcore_c_gateway::legacy_c::ContractTrust::InferredValidated
    );
    assert_eq!(
        summary.contract.parameters.len(),
        2,
        "the formal arity survives: {summary:?}"
    );
    // The recheck already ran inside `certified`; repeat it explicitly so the
    // independent rederivation is the assertion rather than a side effect.
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

/// A recorded boundary, not a claim. Two *distinct declared names* may denote
/// the same object, and this projection had never related them: releasing one
/// formal did not refuse a borrow of another. The pointer-field form answered
/// exactly as the plain-name control did, so the boundary belonged to the
/// projection's name-based model and was not opened by C02-P's admission. If a
/// later increment relates the two names, both rows move together — that is
/// what this test pins.
///
/// **Card C02-P-ALIAS is that increment, and both rows moved together**, from
/// `Safe == Safe` to `Unknown == Unknown`. The agreement is the load-bearing
/// half and is unchanged; the shared value is asserted explicitly below so this
/// pin cannot go quietly vacuous. The obligations each form now reports are
/// pinned in `tests/may_alias_boundary.rs`.
#[test]
fn the_may_alias_boundary_answers_the_same_with_and_without_a_field_read() {
    if !clang_available() {
        return;
    }
    let control = analyze(
        "void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }",
        "w",
    );
    let with_field_read = analyze(
        "struct b_handle *w(struct b_instance *in, struct b_storage *dup, int n) { \
         struct b_storage *storage; struct b_handle *chunk; b_release(dup); \
         storage = in->storage; chunk = b_open(storage->cio, storage->stream, n); \
         if (!chunk) { return 0; } b_close(chunk); return 0; }",
        "w",
    );
    let control = function(program(&control), "w").status;
    let with_field_read = function(program(&with_field_read), "w").status;
    assert_eq!(
        control, with_field_read,
        "the field-read form must not answer differently from the plain-name control"
    );
    assert_eq!(
        control,
        AnalysisStatus::Unknown,
        "C02-P-ALIAS moved both rows together; a `Safe` here would be a regression"
    );
}

// --------------------------------------------------------------------------
// Independent rederivation
// --------------------------------------------------------------------------

/// The admission rests on the *declared* type of the member and of the base.
/// Substituting only the declaration, leaving every statement byte-identical,
/// must be refused by the independent recheck.
#[test]
fn a_certificate_resting_on_the_admission_is_rederived_from_the_declarations() {
    if !clang_available() {
        return;
    }
    let certificate = certified(CREATE, "w_create");
    let substitutions: [(&str, &str, &str); 4] = [
        (
            "the record becomes a union",
            "struct b_storage { int state;",
            "union b_storage { int state;",
        ),
        (
            "the read member becomes volatile",
            "struct b_ctx *cio; struct b_pool *stream; }",
            "struct b_ctx *volatile cio; struct b_pool *stream; }",
        ),
        (
            "the read member becomes an array",
            "struct b_ctx *cio; struct b_pool *stream; }",
            "struct b_ctx *cio[2]; struct b_pool *stream; }",
        ),
        (
            "the record body gains a member the reader cannot read",
            "struct b_storage { int state;",
            "struct b_storage { void (*hook)(int); int state;",
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

/// And it rests on the *contract mode* of the position the actual occupies.
/// A certificate whose carried contract no longer declares that position
/// borrowed is refused, so the admission cannot survive its own premise.
#[test]
fn a_certificate_resting_on_the_borrowed_position_is_refused_when_the_mode_changes() {
    if !clang_available() {
        return;
    }
    let certificate = certified(CREATE, "w_create");
    let mutated_source = CONTRACTS.replacen(
        "    ensures normal resource_borrowed(cio);",
        "    ensures normal resource_consumed(cio, nonnull);",
        1,
    );
    assert_ne!(mutated_source, CONTRACTS, "the mode anchor was found");
    let mut mutated = certificate.clone();
    mutated.contracts = load_resource_contract_sources(&BTreeMap::from([(
        "c02p_generic.qcore".to_owned(),
        mutated_source,
    )]))
    .expect("the mutated contract fixture loads");
    check_legacy_safety_certificate(&mutated)
        .expect_err("a non-borrowed position must withdraw the admission");
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}
