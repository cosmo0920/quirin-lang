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
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
/// The two additional declared callee premises this card names. Concatenated
/// onto O04's, which stays byte-unchanged.
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qr");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_arms_generic.qv");

const CIO_STREAM: &str = include_str!("fixtures/o04_arms/cio_stream.h.txt");
const FLB_CONFIG: &str = include_str!("fixtures/o04_arms/flb_config.h.txt");
const ARMS_PROVENANCE: &str = include_str!("fixtures/o04_arms/provenance.json");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn arms_contracts() -> String {
    format!("{PATH_CONTRACTS}\n{ARMS_CALLEES}").replace("\r\n", "\n")
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
struct h_hooked { void (*hook[1])(int); struct h_ctx *ctx; }; \
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
            "an intermediate link through a union",
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
        // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins three rows
        // deliberately. This chain rule still refuses them; the Clang-typed
        // call-actual fallback asked after it reads each pointer actual (a
        // record Clang completes, a chain past this rule's bound, a subscripted
        // link) at `h_open`'s borrowed position over the untracked formal. The
        // gate refusal gives way to the fixture's own obligation: `h` is used
        // by `h_close` without a null test. Still Unknown, never Safe.
        if TYPED.contains(&label) {
            assert!(
                !unsupported(w).contains(&INDIRECT)
                    && unsupported(w)
                        .contains(&"nullable resource `h` used by a non-null contract"),
                "{label}: the typed actual exposes the unguarded result, got {:?}",
                w.diagnostics
            );
            continue;
        }
        // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d) re-pins this row
        // deliberately: the pointer actual on `link` is left to the flow,
        // which refuses it because `h_adopt(link)` refused `link` earlier on
        // the path (the per-path premise ends there). BEFORE: the gate's
        // refusal. Still Unknown, never Safe.
        if label == "a chain whose root the projection may bind is never admitted" {
            assert!(
                w.diagnostics.iter().any(|d| matches!(
                    &d.kind,
                    LegacyDiagnosticKind::UnknownAlias { variable } if variable == "link"
                )),
                "{label}: refused at the flow, got {:?}",
                w.diagnostics
            );
            continue;
        }
        assert!(
            unsupported(w).contains(&INDIRECT),
            "{label}: the existing diagnostic must be retained, got {:?}",
            w.diagnostics
        );
    }
}

// Card CG13 (rule 10w) adds the union link: `view` is a union step, and the
// pointer `ctx` read through it (unpunned) is a borrow at `h_open`'s borrowed
// position. BEFORE: the gate's refusal.
const TYPED: [&str; 4] = [
    "an intermediate link through a union",
    "an intermediate link whose record body the reader refuses",
    "a chain longer than the bound is refused, not truncated",
    "a subscripted link is not a member selection",
];

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
            "struct h_link { void (*hook[1])(int); int kind;",
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
    // Git may check out the reduced prelude with CRLF on Windows. Keep the
    // assembled test input consistent with the LF mutation anchors below.
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n").replace("\r\n", "\n")
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
        let source = source.replace("\r\n", "\n");
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

/// The probe both chain tests read: the FULL upstream declarations of every
/// link, and one chained actual at each of the two positions that matter.
///
/// Uses the pure-token entry point, so the 306-line `struct flb_config` and the
/// 239-member `struct flb_input_instance` need none of the types they
/// reference to be defined.
///
/// **Card LOADER-ABI changed one statement of this probe, deliberately.** It
/// used to pass `storage->stream->name` as the second actual of
/// `flb_error(0, …, 0)`. Under this fixture's *old* three-parameter `flb_error`
/// declaration that was a declared `Borrowed` position, so the chain admission
/// decided it — but C declares `void flb_error(const char *format, ...)`, so
/// the declaration named modes for two parameters C never typed, and the
/// faithful declaration puts that actual in the variadic tail, where a chained
/// read is refused (`NEIGHBOURS-CHAINED-TAIL`). The chain is therefore read at
/// `cio_log_error`'s declared `Borrowed` second parameter instead, which is
/// what the test was always about: the *chain* admission, not the tail.
fn chain_probe() -> String {
    format!(
        "{INSTANCE}\n{STORAGE_INPUT}\n{CIO_STREAM}\n{FLB_CONFIG}\n\
         struct cio_chunk *probe(struct flb_input_instance *in, int flags)\n\
         {{\n\
         \x20   struct flb_storage_input *storage;\n\
         \x20   struct cio_chunk *chunk;\n\
         \x20   storage = in->storage;\n\
         \x20   chunk = cio_chunk_open(storage->cio, storage->stream, 0, flags, 0, 0);\n\
         \x20   if (!chunk) {{\n\
         \x20       cio_log_error(storage->cio, storage->stream->name);\n\
         \x20       return 0;\n\
         \x20   }}\n\
         \x20   flb_routes_mask_get_size(in->config->router);\n\
         \x20   cio_chunk_close(chunk, 1);\n\
         \x20   return 0;\n\
         }}\n"
    )
}

/// The premise the reduced prelude rests on for the *chains*, checked against
/// the FULL upstream declarations of every link rather than the reduced ones.
#[test]
fn the_upstream_link_declarations_admit_both_chains() {
    let summary = analyze_legacy_c_translation_unit(
        &chain_probe(),
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

/// **The composed statement of cards LOADER-ABI and NEIGHBOURS-CHAINED-TAIL.**
///
/// This is the one statement where the two cards meet, so it carries both of
/// their grounds rather than an average of them.
///
/// LOADER-ABI corrected this fixture's `flb_error` from three named parameters
/// — modes for two parameters C never typed — to the faithful
/// `(const char *, ...)` reading: one declared parameter plus
/// `resource_variadic_data_tail()`. Measured alone, that moved
/// `storage->stream->name` out of a declared `Borrowed` position and into a
/// tail, where nothing then admitted it. CHAINED-TAIL's clause **T5** admits a
/// tail actual that is exactly one admitted bounded typed access chain **at a
/// position the callee's declaration covers with a declared data tail** — and
/// the corrected declaration covers it with precisely that vocabulary.
///
/// Both grounds therefore hold at once, and they agree: T5's premise is
/// satisfied *because* the declaration is now faithful, and LOADER-ABI's
/// premise — that a declaration may only claim what C declares — is what makes
/// the tail a tail. The admission is strictly better founded than the one it
/// replaces: it rests on `resource_variadic_data_tail()`, which C's own
/// `...` supports, instead of on a `resource_borrowed` mode asserted for a
/// parameter that does not exist.
///
/// All three directions are pinned here, so neither card's fail-closed half can
/// be lost to the other's admission:
///
/// 1. under the faithful declaration the chain in the tail is **admitted**;
/// 2. drop the declared tail and the identical actual is **refused** — the
///    position is decided by the declaration, not by the actual;
/// 3. restore the *unfaithful* three-parameter declaration and it is refused
///    again, this time by LOADER-ABI's ABI comparison, with its own key: a
///    declaration C contradicts buys nothing, T5 or no T5.
#[test]
fn the_declaration_decides_the_same_chain_in_a_variadic_tail() {
    // The callee's own C declaration is present, so direction (3) can measure
    // the ABI comparison rather than fall into `LOADER-ABI-UNDECLARED`: a name
    // the unit never declares is not compared, and its contract applies
    // whatever it says.
    let probe = chain_probe()
        .replace(
            "cio_log_error(storage->cio, storage->stream->name);",
            "flb_error(0, storage->stream->name, 0);",
        )
        .replace(
            "struct cio_chunk *probe(struct flb_input_instance *in, int flags)",
            "void flb_error(const char *format, ...);\n\
             struct cio_chunk *probe(struct flb_input_instance *in, int flags)",
        );
    let measure = |contracts: &str| {
        let summary = analyze_legacy_c_translation_unit(
            &probe,
            "probe",
            &load("o04_arms_path.qcore", contracts),
        )
        .expect("the upstream declarations tokenize");
        function(&summary, "probe").clone()
    };

    // (1) The faithful declaration: one parameter plus a declared data tail.
    let admitted = measure(&arms_contracts());
    assert!(
        unsupported(&admitted).is_empty(),
        "T5 admits the chain at a declared tail: {:?}",
        unsupported(&admitted)
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);

    // (2) The same declaration without the tail clause. The actual has not
    // moved; only what the callee declares about its position has.
    let untailed =
        arms_contracts().replace("    ensures normal resource_variadic_data_tail();\n", "");
    assert_ne!(untailed, arms_contracts(), "the tail clause was present");
    let refused = measure(&untailed);
    assert!(
        !unsupported(&refused).is_empty(),
        "without a declared tail the identical actual is refused: {:?}",
        unsupported(&refused)
    );
    assert_eq!(refused.status, AnalysisStatus::Unknown);

    // (3) LOADER-ABI's own half, on this very statement: the unfaithful
    // three-parameter declaration this card corrected is withdrawn against the
    // C declaration, and T5 does not rescue it.
    let unfaithful = arms_contracts().replace(
        "extern fn flb_error(format: ptr<const, u8>) -> unit\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_borrowed(format);\n    \
         ensures normal resource_variadic_data_tail();\n    \
         ensures normal resource_return_none();\n};",
        "extern fn flb_error(format: ptr<const, u8>, detail: ptr<const, u8>, name: ptr<const, u8>) -> unit\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_borrowed(format);\n    \
         ensures normal resource_borrowed(detail);\n    \
         ensures normal resource_borrowed(name);\n    \
         ensures normal resource_return_none();\n};",
    );
    assert_ne!(unfaithful, arms_contracts(), "the substitution applied");
    let withdrawn = measure(&unfaithful);
    assert!(
        unsupported(&withdrawn).iter().any(|description| description
            .contains("whose declared contract was withdrawn against its C declaration")),
        "the unfaithful arity is still withdrawn: {:?}",
        unsupported(&withdrawn)
    );
    assert_eq!(withdrawn.status, AnalysisStatus::Unknown);
}

/// The card's headline, on real source: the logging arm C1 settles. Four of the
/// caller's six exits are `Discharged`, the three cleanup arms still release the
/// same acquisition site exactly once, and the logging arm crosses no
/// acquisition at all.
///
/// **Composed-tree reconciliation (LOADER-ABI × NEIGHBOURS-CHAINED-TAIL).** The
/// number is O04-ARMS' original one; what carries it is not. This fixture's
/// `flb_error` used to be declared with three named parameters — the arity of
/// the call site — beside a C prelude declaring the real
/// `void flb_error(const char *format, ...)`, so the chained tail actual
/// `storage->stream->name` sat at a *declared `Borrowed`* position and the chain
/// admission decided it there. Card LOADER-ABI refused that arity and corrected
/// the declaration to the faithful one-parameter-plus-
/// `resource_variadic_data_tail` form; measured alone, that moved the actual
/// into a tail nothing then admitted, and the arm stopped discharging. Card
/// NEIGHBOURS-CHAINED-TAIL's clause **T5** admits exactly that shape — one
/// bounded typed access chain at a position the callee's declaration covers
/// with a declared data tail — so on the composed tree the arm settles again,
/// now on a premise C's own `...` supports rather than on modes asserted for
/// parameters that do not exist.
///
/// Neither card's fail-closed half is lost, and both are asserted rather than
/// assumed: `the_declaration_decides_the_same_chain_in_a_variadic_tail` pins
/// that dropping the tail clause refuses the identical actual and that the
/// unfaithful three-parameter declaration is still withdrawn; the halves below
/// are O04-ARMS' own.
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
    // It settles rather than merely avoiding an allegation: no exit on this
    // path is left unresolved while crossing nothing.
    assert!(
        caller
            .exits
            .iter()
            .all(|exit| exit.custody != ExitCustody::Unknown || !exit.sites.is_empty()),
        "no exit is unresolved while crossing nothing: {:?}",
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
    // Composed-tree reconciliation: back to two. Card LOADER-ABI's measurement
    // alone put the logging arm here as a third, site-free entry; clause T5 of
    // card NEIGHBOURS-CHAINED-TAIL settles it again at the corrected
    // declaration, so this set is O04-ARMS' original pair. The per-arm shape
    // below is kept rather than reverted to a whole-set assertion, because it
    // is strictly sharper: it would fail if a site-free arm ever rejoined this
    // set unnoticed.
    assert_eq!(
        unknown.len(),
        2,
        "expected the routes-mask arm and the success exit; exits were {:?}",
        exit_shape(caller)
    );
    let carrying = unknown
        .iter()
        .filter(|exit| !exit.sites.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        carrying.len(),
        2,
        "expected the routes-mask arm and the success exit; exits were {:?}",
        exit_shape(caller)
    );
    assert!(
        carrying.iter().all(|exit| exit.sites.len() == 2
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
    // The named blockers, and only those.
    //
    // DELIBERATE PIN UPDATE, card O04-B08: the aggregate-store and
    // aggregate-branch-condition families this row named as open are now
    // admitted under typed premises (`crate::aggregate_store`), so they no
    // longer appear here. The load-bearing half of the row is unchanged and
    // still asserted above: this arm is **not** discharged under *this card's*
    // premise set, both Unknown exits carry both identities unresolved, and no
    // exit on the path is a `Residual`.
    //
    // What replaced them was measured, not assumed. O04-ARMS §1.3 recorded
    // `flb_time_now` as *inert* — `ic->create_time = flb_time_now();` refused
    // as an indirect store before any contract was consulted — and deliberately
    // left it undeclared. With the store admitted, the call is no longer inert
    // and becomes this arm's blocker under O04-ARMS's byte-unchanged premise
    // set. Card O04-B08 declares it in its own fixture and measures the arm
    // discharging there; this row keeps O04-ARMS's premise set exactly, so it
    // records the refusal instead.
    //
    // Everything else is unchanged: `INDIRECT` and the uncontracted
    // success-path logging call remain on the success exit's path, and `ic`
    // still reaches the success exit possibly-null.
    let remaining = unsupported(caller);
    assert!(
        !remaining.is_empty(),
        "the named blockers are still reported"
    );
    let unexpected = remaining
        .iter()
        .filter(|description| {
            !(**description == INDIRECT
                || **description
                    == "unmodeled call, aggregate or indirect access in branch condition"
                || **description == "call to uncontracted function `flb_trace`"
                || **description == "call to uncontracted function `flb_time_now`"
                // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k), a
                // deliberate pin update: the gate's refusals of calls to callees
                // this fixture leaves uncontracted now name them.
                || **description == "call to uncontracted function `msgpack_packer_init`"
                || **description == "call to uncontracted function `mk_list_add`"
                || **description == "call to uncontracted function `flb_hash_table_add`"
                || **description == "call to uncontracted function `pthread_mutex_lock`"
                || **description == "call to uncontracted function `pthread_mutex_unlock`"
                || **description == "nullable resource `ic` used by a non-null contract")
        })
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty(),
        "unexpected remaining refusals {unexpected:?}"
    );
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

/// The chain admission rests on declarations the analysis re-derives:
/// substituting any link withdraws it.
///
/// **DELIBERATE PIN UPDATE, card LOADER-ABI.** This test used to measure the
/// *logging arm's discharge* on the real unit. That discharge is gone — the
/// fixture's `flb_error` declaration was unfaithful to `(const char *, ...)`
/// and the actual now sits in the variadic tail — so measuring it here would
/// have made every substitution below **vacuously** pass against a baseline
/// that already fails. The controls are therefore repointed at the same four
/// link substitutions over [`chain_probe`], where the chain is read at
/// `cio_log_error`'s declared `Borrowed` parameter and the baseline is
/// asserted `Safe` first, so nothing here can pass by vacuity.
#[test]
fn substituting_a_chain_link_withdraws_the_chain_admission() {
    let baseline = chain_probe();
    let contracts = load("o04_arms_path.qcore", &arms_contracts());
    // Non-vacuity: the unmutated probe is admitted, so each substitution below
    // measures a withdrawal rather than an already-refused shape.
    assert_eq!(
        function(
            &analyze_legacy_c_translation_unit(&baseline, "probe", &contracts)
                .expect("the probe tokenizes"),
            "probe"
        )
        .status,
        AnalysisStatus::Safe,
        "the baseline probe must be admitted"
    );
    // The four link substitutions, over the *upstream* declarations this probe
    // includes. Retagging `struct cio_stream` as a `union` is not expressible
    // here — other declarations in the same text name the tag — and stays
    // covered by the unit tests of `crate::borrowed_actual` and by the generic
    // certificate substitutions above.
    let cases: [(&str, &str, &str); 4] = [
        (
            "the intermediate link's record gains an unreadable member",
            "struct cio_stream {",
            "struct cio_stream {\n    void (*hook[1])(int);",
        ),
        (
            "the intermediate member becomes _Atomic",
            "    struct cio_stream *stream;",
            "    struct cio_stream *_Atomic stream;",
        ),
        (
            "the chain's final member becomes volatile",
            "    char *name;",
            "    volatile char *name;",
        ),
        (
            "the chain's final member becomes an array",
            "    char *name;",
            "    char *name[2];",
        ),
    ];
    for (label, from, to) in cases {
        assert!(
            baseline.contains(from),
            "{label}: anchor `{from}` not found"
        );
        let mutated = baseline.replacen(from, to, 1);
        let summary = analyze_legacy_c_translation_unit(&mutated, "probe", &contracts)
            .expect("the mutated probe tokenizes");
        let probe = function(&summary, "probe");
        assert!(
            unsupported(probe).contains(&INDIRECT),
            "{label}: the chain admission must be withdrawn, diagnostics were {:?}",
            unsupported(probe)
        );
        assert_eq!(probe.status, AnalysisStatus::Unknown, "{label}");
    }
}

/// Weakening the declared premise at the chain's own position withdraws the
/// admission: it is a consequence of a *declared* contract, not of the
/// statement's shape.
///
/// **DELIBERATE PIN UPDATE, card LOADER-ABI.** This test used to weaken
/// `resource_borrowed(detail)` — the second of `flb_error`'s three declared
/// parameters — and measure the logging arm losing its discharge. That
/// declaration was unfaithful to C's `void flb_error(const char *format, ...)`
/// and no longer exists, so the anchor is gone and the arm no longer
/// discharges at all. The same claim is measured where it is still live: the
/// chained actual at `cio_log_error`'s declared `Borrowed` `message`
/// parameter, over [`chain_probe`], with the unweakened baseline asserted
/// `Safe` so the withdrawal cannot pass by vacuity.
#[test]
fn weakening_the_premise_at_the_chain_position_withdraws_the_admission() {
    let probe = chain_probe();
    let baseline = arms_contracts();
    let weakened = baseline.replace(
        "extern fn cio_log_error(ctx: ptr<mut, u8>, message: ptr<const, u8>) -> unit\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_borrowed(ctx);\n    \
         ensures normal resource_borrowed(message);",
        "extern fn cio_log_error(ctx: ptr<mut, u8>, message: ptr<const, u8>) -> unit\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_borrowed(ctx);\n    \
         ensures normal resource_consumed(message, nonnull);",
    );
    assert_ne!(weakened, baseline, "the logging clause moved");
    assert_eq!(
        function(
            &analyze_legacy_c_translation_unit(
                &probe,
                "probe",
                &load("o04_arms_path.qcore", &baseline)
            )
            .expect("the probe tokenizes"),
            "probe"
        )
        .status,
        AnalysisStatus::Safe,
        "the unweakened baseline must be admitted"
    );
    let summary =
        analyze_legacy_c_translation_unit(&probe, "probe", &load("o04_arms_path.qcore", &weakened))
            .expect("the probe tokenizes");
    let probe = function(&summary, "probe");
    assert!(
        unsupported(probe).contains(&INDIRECT),
        "the chain admission must be withdrawn: {:?}",
        unsupported(probe)
    );
}

/// The constructor is untouched by this card: its four guard exits stay
/// discharged and its rollback and success exits stay unresolved, because its
/// blockers are uncontracted backend constructors, a `void *` member the
/// declaration reader will not type, and list operations through `&`.
///
/// DELIBERATE PIN UPDATE, card O04-B08: the per-exit *site count* on the two
/// unresolved exits moved from one to two. `ch->name = strdup(name);` is an
/// admitted aggregate store under typed premises, so the `strdup` allocation
/// this projection previously erased is now a tracked identity. Both exits
/// still carry **every** site unresolved and the function still keeps status
/// `Unknown`, which is the load-bearing half; the row now asserts that shape
/// rather than a fixed arity, and pins the site count separately so a silent
/// change in either direction still fails.
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
                *custody == ExitCustody::Unknown
                    && !sites.is_empty()
                    && sites
                        .iter()
                        .all(|disposition| *disposition == SiteDisposition::Mixed)
            })
            .count(),
        2,
        "the rollback and success exits stay unresolved: {shape:?}"
    );
    assert!(
        shape
            .iter()
            .filter(|(custody, _)| *custody == ExitCustody::Unknown)
            .all(|(_, sites)| sites.len() == 2),
        "each unresolved exit carries the chunk and the name allocation: {shape:?}"
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
