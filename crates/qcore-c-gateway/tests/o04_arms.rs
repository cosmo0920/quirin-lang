//! O04-ARMS — per-arm discharge on the *remaining* early-return arms of
//! `flb_input_chunk_create` (`docs/reports/remaining-o04-arms/README.md`).
//!
//! Card O04 discharged the caller's three modeled cleanup arms (C2/C3/C4) and
//! left three exits `Unknown`: the logging arm C1, the routes-mask arm C5 and
//! the success exit C6. C6 and the unenumerated exit stay out of scope. This
//! card takes C1 and C5, and it either discharges an arm or records its named
//! residual — never both, and never a claim in the defect direction.
//!
//! The one analyzer capability it adds is the **bounded access chain** of
//! `crate::borrowed_actual`: `base -> f1 -> f2` where every intermediate link
//! is declared exactly `struct POINTEE *` over a complete, unambiguous,
//! non-union record. That is what C02-P refused by construction and what O04
//! §7 recorded as the reason C1 stays `Unknown`.
//!
//! Two legs, both through the public gateway entry points only.
//!
//! **Held-out generic family.** An `h_*` API unrelated to ChunkIO, to Fluent
//! Bit and to O04's own `g_*` family: an admitted chain that discharges every
//! exit, a refusal matrix in which every link the reader did not fully type
//! keeps its existing `Unknown` and its existing diagnostic, a leak the chain
//! must not hide, the containment of the released-intermediate hazard, and
//! declaration substitutions the independent certificate recheck refuses.
//!
//! **Real source.** The same byte-verbatim upstream regions O04 pinned, plus
//! the two intermediate-link record declarations the chains cross, pinned here
//! by their own `provenance.json`.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for
//! a synthetic declared custody projection; both real functions keep
//! function-level status `Unknown`, and no raw unit is promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, SiteDisposition,
    UnrestrictedLegacyResult, analyze_legacy_c_translation_unit, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/digest.rs"]
mod digest_support;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const INSTANCE: &str = include_str!("fixtures/o04/flb_input_instance.h.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qcore");
/// The two additional declared callee premises this card names. Concatenated
/// onto O04's, which stays byte-unchanged.
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qcore");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_arms_generic.qcore");

const CIO_STREAM: &str = include_str!("fixtures/o04_arms/cio_stream.h.txt");
const FLB_CONFIG: &str = include_str!("fixtures/o04_arms/flb_config.h.txt");
const ARMS_PROVENANCE: &str = include_str!("fixtures/o04_arms/provenance.json");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn arms_contracts() -> String {
    format!("{}\n{ARMS_CALLEES}", PATH_CONTRACTS.replace("\r\n", "\n"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent O04-ARMS test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("expected an analyzable partial summary: {other:?}"),
    }
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn exit_shape(summary: &LegacyFunctionSummary) -> Vec<(ExitCustody, Vec<SiteDisposition>)> {
    summary
        .exits
        .iter()
        .map(|exit| {
            (
                exit.custody,
                exit.sites.iter().map(|site| site.disposition).collect(),
            )
        })
        .collect()
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<&str> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.as_str()),
            _ => None,
        })
        .collect()
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

/// Deliberately unrelated to every library in this repository and to O04's own
/// `g_*` family. Every rule under test is stated over declared C types and
/// contract modes, so no name here can matter.
///
/// * `h_env -> h_link -> h_conf` is the fully typed chain.
/// * `h_shady -> h_view` crosses a `union`.
/// * `h_holder -> h_hooked` crosses a record whose body the reader refuses.
/// * `h_deep` is self-referential, for the depth bound.
/// * `h_boxed` holds an array of links, for the subscripted shape.
const DECLARATIONS: &str = "\
struct h_ctx; struct h_pool; struct h_handle; \
struct h_conf { int mode; struct h_ctx *ctx; struct h_pool *pool; }; \
struct h_link { int kind; struct h_conf *conf; }; \
struct h_env { int flags; struct h_link *link; }; \
union h_view { struct h_ctx *ctx; int tag; }; \
struct h_shady { int kind; union h_view *view; }; \
struct h_hooked { void (*hook)(int); struct h_ctx *ctx; }; \
struct h_holder { int kind; struct h_hooked *hooked; }; \
struct h_deep { struct h_deep *next; struct h_ctx *ctx; }; \
struct h_boxed { struct h_link *links[2]; }; \
struct h_handle *h_open(struct h_ctx *ctx, struct h_pool *pool, int flags); \
void h_close(struct h_handle *h); \
int h_probe(struct h_handle *h); \
void h_adopt(void *p); \
void h_note(int first, ...);";

/// The positive case: one three-link chain at each of two declared borrowed
/// positions, one failure arm that cleans up, and a success exit that hands
/// the identity back to the caller.
const H_ACQUIRE: &str = "\
struct h_handle *h_acquire(struct h_env *env, int n) \
{ \
    int ready; \
    struct h_handle *h; \
    h = h_open(env->link->conf->ctx, env->link->conf->pool, n); \
    if (!h) { return 0; } \
    ready = h_probe(h); \
    if (ready == -1) { h_close(h); return 0; } \
    return h; \
}";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_arms_generic.qcore", GENERIC_CONTRACTS),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn certified(body: &str, entry: &str) -> LegacySafetyCertificate {
    match analyze_generic(body, entry) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
            certificate
        }
        other => panic!("expected a validated custody projection: {other:?}"),
    }
}

/// Non-vacuity, positive direction: a fully typed chain is modeled, so every
/// exit of the generic path settles.
#[test]
fn a_fully_typed_chain_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let certificate = certified(H_ACQUIRE, "h_acquire");
    let acquire = function(&certificate.summary, "h_acquire");
    let shape = exit_shape(acquire);
    assert!(
        shape
            .iter()
            .all(|(custody, _)| *custody == ExitCustody::Discharged),
        "every exit settles: {shape:?}"
    );
    assert_eq!(acquire.status, AnalysisStatus::Safe);
    assert!(
        unsupported(acquire).is_empty(),
        "no refusal remains: {:?}",
        unsupported(acquire)
    );
}

/// Every link the reader did not fully type keeps its existing `Unknown` *and*
/// its existing diagnostic. The chain rule removes only the spans it justified.
#[test]
fn every_untyped_link_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 7] = [
        (
            "an intermediate link through a union is never admitted",
            "struct h_handle *w(struct h_shady *shady, int n) { struct h_handle *h; \
             h = h_open(shady->view->ctx, 0, n); h_close(h); return 0; }",
        ),
        (
            "an intermediate link whose record body the reader refuses",
            "struct h_handle *w(struct h_holder *hold, int n) { struct h_handle *h; \
             h = h_open(hold->hooked->ctx, 0, n); h_close(h); return 0; }",
        ),
        (
            "a chain longer than the bound is refused, not truncated",
            "struct h_handle *w(struct h_deep *d, int n) { struct h_handle *h; \
             h = h_open(d->next->next->next->next->ctx, 0, n); h_close(h); return 0; }",
        ),
        (
            "a subscripted link is not a member selection",
            "struct h_handle *w(struct h_boxed *b, int n) { struct h_handle *h; \
             h = h_open(b->links[0]->conf->ctx, 0, n); h_close(h); return 0; }",
        ),
        (
            "a chain at a consuming position is never admitted",
            "void w(struct h_env *env) { h_adopt(env->link->conf); }",
        ),
        (
            "a chain beyond the declared parameter vector is never admitted",
            "void w(struct h_env *env) { h_note(0, env->link->conf); }",
        ),
        (
            "a chain whose root the projection may bind is never admitted",
            "struct h_handle *w(struct h_env *env, int n) { struct h_handle *h; \
             struct h_link *link; link = env->link; h_adopt(link); \
             h = h_open(link->conf->ctx, link->conf->pool, n); h_close(h); return 0; }",
        ),
    ];
    for (label, body) in cases {
        let result = analyze_generic(body, "w");
        let w = function(program(&result), "w");
        assert_eq!(w.status, AnalysisStatus::Unknown, "{label}");
        assert!(
            unsupported(w).contains(&INDIRECT),
            "{label}: the existing diagnostic must be retained, got {:?}",
            w.diagnostics
        );
    }
}

/// A chain must not manufacture a discharge. An arm that returns without
/// releasing is still a reported residual, and the function is still `Unsafe`.
#[test]
fn a_chain_never_manufactures_a_discharge() {
    if !clang_available() {
        return;
    }
    let leaky = H_ACQUIRE.replacen(
        "if (ready == -1) { h_close(h); return 0; }",
        "if (ready == -1) { return 0; }",
        1,
    );
    assert_ne!(leaky, H_ACQUIRE);
    let result = analyze_generic(&leaky, "h_acquire");
    let acquire = function(program(&result), "h_acquire");
    assert_eq!(acquire.status, AnalysisStatus::Unsafe);
    assert!(
        acquire
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "the retained identity is reported: {:?}",
        acquire.diagnostics
    );
    assert_eq!(
        exit_shape(acquire)
            .iter()
            .filter(|(custody, sites)| *custody == ExitCustody::Residual
                && sites == &vec![SiteDisposition::Live])
            .count(),
        1,
        "exactly the leaking arm is a residual: {:?}",
        exit_shape(acquire)
    );
}

/// The containment the chain rule's premise (3') names. A chain reads through
/// intermediate *places* this function never binds, so it cannot check whether
/// one of them was released through some other local. It does not have to: an
/// intermediate can only be released in this function through a name bound by
/// an *opaque* field read, and releasing an opaque-bound name reports
/// `UnknownAlias`, so no such body reaches `Safe`.
#[test]
fn releasing_an_opaque_bound_name_is_never_silently_admitted() {
    if !clang_available() {
        return;
    }
    let body = "struct h_handle *w(struct h_env *env, int n) { struct h_handle *h; \
                struct h_conf *taken; taken = env->link->conf; h_adopt(taken); \
                h = h_open(env->link->conf->ctx, env->link->conf->pool, n); \
                h_close(h); return 0; }";
    let result = analyze_generic(body, "w");
    let w = function(program(&result), "w");
    assert_ne!(
        w.status,
        AnalysisStatus::Safe,
        "releasing an opaque-bound intermediate must never reach Safe: {:?}",
        w.diagnostics
    );
    assert!(
        w.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "taken"
        )),
        "the opaque-bound name is reported: {:?}",
        w.diagnostics
    );
}

/// The independent certificate recheck repeats the whole chain classification
/// from the certificate's own content. Each substitution touches only a
/// *declaration* an intermediate or final link rests on; the call statement is
/// left byte-identical, and each must be refused.
#[test]
fn a_certificate_resting_on_a_chain_is_rederived_from_source() {
    if !clang_available() {
        return;
    }
    let certificate = certified(H_ACQUIRE, "h_acquire");
    let substitutions: [(&str, &str, &str); 6] = [
        (
            "the intermediate link's record becomes a union",
            "struct h_link { int kind;",
            "union h_link { int kind;",
        ),
        (
            "the intermediate link's record gains an unreadable member",
            "struct h_link { int kind;",
            "struct h_link { void (*hook)(int); int kind;",
        ),
        (
            "the intermediate member becomes an array",
            "struct h_link { int kind; struct h_conf *conf; }",
            "struct h_link { int kind; struct h_conf *conf[2]; }",
        ),
        (
            "the intermediate member gains a second star",
            "struct h_link { int kind; struct h_conf *conf; }",
            "struct h_link { int kind; struct h_conf **conf; }",
        ),
        (
            "the intermediate tag is defined twice",
            "struct h_link { int kind; struct h_conf *conf; };",
            "struct h_link { int kind; struct h_conf *conf; }; struct h_link { int kind; };",
        ),
        (
            "the final member becomes volatile",
            "struct h_ctx *ctx; struct h_pool *pool; }",
            "struct h_ctx *volatile ctx; struct h_pool *pool; }",
        ),
    ];
    for (label, from, to) in substitutions {
        let mut mutated = certificate.clone();
        assert!(
            mutated.preprocessed_source.contains(from),
            "{label}: anchor `{from}` not found"
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
    // The unmodified certificate still rechecks, so the six refusals above
    // were caused by the substitutions and not by an unrelated failure.
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}

/// Weakening the declared release mode must turn the cleanup arm into a
/// reported residual, never a silent discharge — the chain admission changes
/// what is *read*, never what discharges.
#[test]
fn a_borrowed_destructor_turns_the_chain_path_arm_into_a_residual() {
    if !clang_available() {
        return;
    }
    let weakened = GENERIC_CONTRACTS.replace("\r\n", "\n").replace(
        "    ensures normal resource_released(h, nonnull);\n    ensures normal resource_return_none();\n};\n\n// probe",
        "    ensures normal resource_borrowed(h);\n    ensures normal resource_return_none();\n};\n\n// probe",
    );
    assert_ne!(
        weakened,
        GENERIC_CONTRACTS.replace("\r\n", "\n"),
        "the destructor clause moved"
    );
    let source = format!("{DECLARATIONS} {H_ACQUIRE}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "h_acquire",
        &load("o04_arms_generic.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    let acquire = function(program(&result), "h_acquire");
    assert_eq!(acquire.status, AnalysisStatus::Unsafe);
    assert!(
        exit_shape(acquire)
            .iter()
            .any(|(custody, _)| *custody == ExitCustody::Residual),
        "the arm retains the identity: {:?}",
        exit_shape(acquire)
    );
}

// --------------------------------------------------------------------------
// Real source: the remaining early-return arms
// --------------------------------------------------------------------------

fn real_unit() -> String {
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n")
}

fn analyze_real(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_arms_path.qcore", &arms_contracts()),
    )
    .unwrap_or_else(|error| panic!("O04-ARMS fixture rejected by Clang: {error:?}"))
}

/// The two intermediate-link declarations really are the upstream bytes, at the
/// same revision O04 pinned. Needs no Clang.
#[test]
fn the_intermediate_link_declarations_are_the_upstream_bytes() {
    for (name, source) in [("cio_stream", CIO_STREAM), ("flb_config", FLB_CONFIG)] {
        let needle = format!(
            "\"sha256\": \"{}\"",
            digest_support::sha256(source.as_bytes())
        );
        assert!(
            ARMS_PROVENANCE.contains(&needle),
            "region `{name}` no longer matches the digest recorded in provenance.json; it is {}",
            digest_support::sha256(source.as_bytes())
        );
    }
    assert!(ARMS_PROVENANCE.contains("90a010910e5b3328f496cc20fadc937547133439"));
}

/// The premise the reduced prelude rests on for the *chains*, checked against
/// the FULL upstream declarations of every link rather than the reduced ones.
/// Uses the pure-token entry point, so the 306-line `struct flb_config` and the
/// 239-member `struct flb_input_instance` need none of the types they
/// reference to be defined.
#[test]
fn the_upstream_link_declarations_admit_both_chains() {
    let probe = format!(
        "{INSTANCE}\n{STORAGE_INPUT}\n{CIO_STREAM}\n{FLB_CONFIG}\n\
         struct cio_chunk *probe(struct flb_input_instance *in, int flags)\n\
         {{\n\
         \x20   struct flb_storage_input *storage;\n\
         \x20   struct cio_chunk *chunk;\n\
         \x20   storage = in->storage;\n\
         \x20   chunk = cio_chunk_open(storage->cio, storage->stream, 0, flags, 0, 0);\n\
         \x20   if (!chunk) {{\n\
         \x20       flb_error(0, storage->stream->name, 0);\n\
         \x20       return 0;\n\
         \x20   }}\n\
         \x20   flb_routes_mask_get_size(in->config->router);\n\
         \x20   cio_chunk_close(chunk, 1);\n\
         \x20   return 0;\n\
         }}\n"
    );
    let summary = analyze_legacy_c_translation_unit(
        &probe,
        "probe",
        &load("o04_arms_path.qcore", &arms_contracts()),
    )
    .expect("the upstream declarations tokenize");
    let probe = function(&summary, "probe");
    assert!(
        !unsupported(probe).contains(&INDIRECT),
        "the upstream declarations must admit `storage->stream->name` and \
         `in->config->router`; diagnostics were {:?}",
        unsupported(probe)
    );
    assert_eq!(
        probe.status,
        AnalysisStatus::Safe,
        "diagnostics: {:?}",
        probe.diagnostics
    );
}

/// The card's headline, on real source: the logging arm C1 now settles. Four of
/// the caller's six exits are `Discharged`, the three cleanup arms still
/// release the same acquisition site exactly once, and the logging arm crosses
/// no acquisition at all.
#[test]
fn the_logging_arm_discharges_under_the_declared_callee_premises() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let discharged = caller
        .exits
        .iter()
        .filter(|exit| exit.custody == ExitCustody::Discharged)
        .collect::<Vec<_>>();
    assert_eq!(
        discharged.len(),
        4,
        "expected the three cleanup arms plus the logging arm; exits were {:?}",
        exit_shape(caller)
    );
    let cleanup = discharged
        .iter()
        .filter(|exit| !exit.sites.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 3, "exits were {:?}", exit_shape(caller));
    for exit in &cleanup {
        assert_eq!(exit.sites.len(), 1);
        assert_eq!(exit.sites[0].disposition, SiteDisposition::Released);
        assert_eq!(
            exit.sites[0].acquisition_line, cleanup[0].sites[0].acquisition_line,
            "all cleanup arms release the same acquisition site"
        );
    }
    assert_eq!(
        discharged
            .iter()
            .filter(|exit| exit.sites.is_empty())
            .count(),
        1,
        "the logging arm crosses no acquisition: {:?}",
        exit_shape(caller)
    );
    // No promotion: the function itself stays Unknown.
    assert_eq!(caller.status, AnalysisStatus::Unknown);
}

/// The other half of the card's obligation: the arm that is *not* discharged
/// records its named residual, and records nothing in the defect direction.
/// C5 and the out-of-scope success exit C6 both stay `Unknown` carrying both
/// identities unresolved; no exit on this path is a `Residual`.
#[test]
fn the_routes_mask_arm_records_its_named_residual() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let unknown = caller
        .exits
        .iter()
        .filter(|exit| exit.custody == ExitCustody::Unknown)
        .collect::<Vec<_>>();
    assert_eq!(
        unknown.len(),
        2,
        "expected the routes-mask arm and the success exit; exits were {:?}",
        exit_shape(caller)
    );
    assert!(
        unknown.iter().all(|exit| exit.sites.len() == 2
            && exit
                .sites
                .iter()
                .all(|site| site.disposition == SiteDisposition::Mixed)),
        "both carry both identities, each unresolved: {:?}",
        exit_shape(caller)
    );
    assert!(
        caller
            .exits
            .iter()
            .all(|exit| exit.custody != ExitCustody::Residual),
        "no residual is alleged on this path: {:?}",
        exit_shape(caller)
    );
    // The named blockers, and only those. Every refusal left in the caller is
    // an aggregate *store* (O00 blocker B08, open), a branch condition over
    // one (out of scope for C02 §7 and for this card), the deliberately
    // uncontracted success-path logging call, or the consequence of `ic`
    // staying an unresolved alias because of the stores. Reading is modeled;
    // writing is not.
    let remaining = unsupported(caller);
    assert!(
        !remaining.is_empty(),
        "the named blockers are still reported"
    );
    for description in &remaining {
        assert!(
            *description == INDIRECT
                || *description
                    == "unmodeled call, aggregate or indirect access in branch condition"
                || *description == "call to uncontracted function `flb_trace`"
                || *description == "nullable resource `ic` used by a non-null contract",
            "unexpected remaining refusal `{description}`"
        );
    }
    // Not one of them is a chained *read*: the card's admission removed exactly
    // that family, and nothing else.
    assert!(
        !caller
            .exits
            .iter()
            .any(|exit| exit.custody == ExitCustody::Discharged && exit.sites.len() == 2),
        "no exit claims both identities discharged: {:?}",
        exit_shape(caller)
    );
}

/// The logging arm's discharge rests on the chain, and the chain rests on
/// declarations the analysis re-derives. Substituting any link withdraws it.
#[test]
fn substituting_a_chain_link_withdraws_the_logging_arm() {
    if !clang_available() {
        return;
    }
    let baseline = real_unit();
    // Each substitution must leave a translation unit Clang still accepts, so
    // that the withdrawal is caused by the declaration and not by a parse
    // failure. Retagging `struct cio_stream` as a `union` is therefore not
    // expressible here — five other declarations in the same unit name the
    // tag — and stays covered by the unit tests of `crate::borrowed_actual`
    // and by the generic certificate substitutions above.
    let cases: [(&str, &str, &str); 4] = [
        (
            "the intermediate link's record gains an unreadable member",
            "struct cio_stream {",
            "struct cio_stream {\n    void (*hook)(int);",
        ),
        (
            "the intermediate member becomes _Atomic",
            "    struct cio_stream *stream;",
            "    struct cio_stream *_Atomic stream;",
        ),
        (
            "the chain's final member becomes volatile",
            "struct cio_stream {\n    int type;\n    char *name;",
            "struct cio_stream {\n    int type;\n    volatile char *name;",
        ),
        (
            "the chain's final member becomes an array",
            "struct cio_stream {\n    int type;\n    char *name;",
            "struct cio_stream {\n    int type;\n    char *name[2];",
        ),
    ];
    for (label, from, to) in cases {
        assert!(
            baseline.contains(from),
            "{label}: anchor `{from}` not found"
        );
        let mutated = baseline.replacen(from, to, 1);
        let result = analyze_real(&mutated);
        let caller = function(program(&result), "flb_input_chunk_create");
        let discharged = caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged && exit.sites.is_empty())
            .count();
        assert_eq!(
            discharged,
            0,
            "{label}: the logging arm's discharge must be withdrawn, exits were {:?}",
            exit_shape(caller)
        );
        assert_eq!(caller.status, AnalysisStatus::Unknown, "{label}");
    }
}

/// Weakening the declared logging premise withdraws the arm too: the discharge
/// is a consequence of a *declared* contract, not of the statement's shape.
#[test]
fn weakening_the_logging_premise_withdraws_the_arm() {
    if !clang_available() {
        return;
    }
    let weakened = arms_contracts().replace(
        "    ensures normal resource_borrowed(detail);",
        "    ensures normal resource_consumed(detail, nonnull);",
    );
    assert_ne!(weakened, arms_contracts(), "the logging clause moved");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &real_unit(),
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_arms_path.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    let caller = function(program(&result), "flb_input_chunk_create");
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged && exit.sites.is_empty())
            .count(),
        0,
        "the logging arm must lose its discharge: {:?}",
        exit_shape(caller)
    );
}

/// The constructor is untouched by this card: its four guard exits stay
/// discharged and its rollback and success exits stay unresolved, because its
/// blockers are aggregate *stores* and uncontracted backend constructors.
#[test]
fn the_constructor_body_still_does_not_discharge_its_own_contract() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let constructor = function(program(&result), "cio_chunk_open");
    assert_eq!(constructor.status, AnalysisStatus::Unknown);
    let shape = exit_shape(constructor);
    assert_eq!(
        shape
            .iter()
            .filter(|(custody, sites)| *custody == ExitCustody::Discharged && sites.is_empty())
            .count(),
        4,
        "the four guard/allocation-failure exits acquire nothing: {shape:?}"
    );
    assert_eq!(
        shape
            .iter()
            .filter(|(custody, sites)| {
                *custody == ExitCustody::Unknown && sites == &vec![SiteDisposition::Mixed]
            })
            .count(),
        2,
        "the rollback and success exits stay unresolved: {shape:?}"
    );
}

/// Not an assertion: prints the measured per-exit ledger and the remaining
/// refusals of the real path under this card's premise set, so the archived
/// report can be regenerated. Run with
/// `cargo test -p qcore-c-gateway --test o04_arms -- --ignored --nocapture
///  print_the_measured_arm_ledger`.
#[test]
#[ignore = "reporting aid, not an assertion"]
fn print_the_measured_arm_ledger() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let preprocessed = match &result {
        UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            ..
        } => preprocessed_source.clone(),
        _ => String::new(),
    };
    let lines = preprocessed.split('\n').collect::<Vec<_>>();
    let summary = program(&result);
    for name in ["cio_chunk_open", "flb_input_chunk_create"] {
        let measured = function(summary, name);
        println!("{name}: {:?}", measured.status);
        for diagnostic in &measured.diagnostics {
            println!(
                "  refusal L{:<4} {:<58} | {:?}",
                diagnostic.line,
                lines
                    .get(diagnostic.line.saturating_sub(1))
                    .unwrap_or(&"?")
                    .trim(),
                diagnostic.kind
            );
        }
        for exit in &measured.exits {
            println!(
                "  exit    L{:<4} {:<11?} {:<40} sites={:?}",
                exit.line,
                exit.custody,
                lines
                    .get(exit.line.saturating_sub(1))
                    .unwrap_or(&"?")
                    .trim(),
                exit.sites
                    .iter()
                    .map(|site| (site.acquisition_line, site.disposition))
                    .collect::<Vec<_>>()
            );
        }
    }
}
