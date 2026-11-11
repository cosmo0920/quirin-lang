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

//! O04 — one validated constructor-to-caller custody path
//! (`docs/reports/remaining-o04/README.md`).
//!
//! Two legs, both through the public gateway entry points only.
//!
//! **Real source.** The constructor `cio_chunk_open` and the caller
//! `flb_input_chunk_create` that card O00 selected
//! (`docs/reports/remaining-o00/README.md` §1), as byte-verbatim upstream
//! regions whose digests are pinned by `provenance.json` and asserted here.
//! The measurement is per-exit: which arms discharge the constructed chunk
//! identity, and which stay `Unknown` because they cross an aggregate this
//! projection cannot follow.
//!
//! **Held-out generic family.** A deliberately unrelated `g_*` API exercising
//! the same capability end to end — owned success, a real leak, a real invalid
//! transfer, a success-looking integer that must NOT be read as a discharge,
//! ambiguous shapes that keep their `Unknown`, and declaration substitutions
//! that the independent certificate recheck refuses.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for
//! a synthetic declared custody projection; the real leg's function-level
//! status stays `Unknown` in both functions, and no raw unit is promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, SiteDisposition,
    UnrestrictedLegacyResult, analyze_legacy_c_translation_unit, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const INSTANCE: &str = include_str!("fixtures/o04/flb_input_instance.h.txt");
const PROVENANCE: &str = include_str!("fixtures/o04/provenance.json");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_generic.qv");

/// The existing rejection every ambiguous shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent O04 test: {reason}");
            false
        }
    }
}

// --------------------------------------------------------------------------
// Real source: the path O00 selected
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
        &load("o04_path.qcore", PATH_CONTRACTS),
    )
    .unwrap_or_else(|error| panic!("O04 fixture rejected by Clang: {error:?}"))
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

/// The `(custody, dispositions)` shape of every exit, in CFG order.
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

#[path = "support/digest.rs"]
mod digest_support;

/// Hex SHA-256 of a fixture's bytes, so the pin needs no extra dependency.
fn digest(bytes: &str) -> String {
    digest_support::sha256(bytes.replace("\r\n", "\n").as_bytes())
}

/// The excerpts really are the upstream bytes. Needs no Clang.
#[test]
fn the_extracted_regions_are_the_upstream_bytes() {
    for (name, source) in [
        ("cio_chunk_open", CONSTRUCTOR),
        ("flb_input_chunk_create", CALLER),
        ("flb_input_instance", INSTANCE),
        ("flb_storage_input", STORAGE_INPUT),
    ] {
        let needle = format!("\"sha256\": \"{}\"", digest(source));
        assert!(
            PROVENANCE.contains(&needle),
            "region `{name}` no longer matches the digest recorded in provenance.json; \
             it is {}",
            digest(source)
        );
    }
    // The pinned upstream revision the digests were taken at.
    assert!(PROVENANCE.contains("90a010910e5b3328f496cc20fadc937547133439"));
}

/// The premise the reduced prelude rests on, checked against the FULL upstream
/// record declarations rather than the reduced ones: both records are readable
/// by the same declaration reader, and both members the caller reads are
/// admitted. Uses the pure-token entry point, so the 239-member upstream
/// declaration needs none of the ~50 types it references to be defined.
#[test]
fn the_upstream_record_declarations_are_readable_and_declare_the_read_members() {
    let probe = format!(
        "{INSTANCE}\n{STORAGE_INPUT}\n\
         struct cio_chunk *probe(struct flb_input_instance *in, int flags)\n\
         {{\n\
         \x20   struct flb_storage_input *storage;\n\
         \x20   struct cio_chunk *chunk;\n\
         \x20   storage = in->storage;\n\
         \x20   chunk = cio_chunk_open(storage->cio, storage->stream, 0, flags, 0, 0);\n\
         \x20   if (!chunk) {{ return 0; }}\n\
         \x20   cio_chunk_close(chunk, 1);\n\
         \x20   return 0;\n\
         }}\n"
    );
    let summary =
        analyze_legacy_c_translation_unit(&probe, "probe", &load("o04_path.qcore", PATH_CONTRACTS))
            .expect("the upstream declarations tokenize");
    let probe = function(&summary, "probe");
    let unsupported = probe
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !unsupported.contains(&INDIRECT),
        "the upstream declarations must admit `in->storage`, `storage->cio` and \
         `storage->stream`; diagnostics were {unsupported:?}"
    );
    assert_eq!(
        probe.status,
        AnalysisStatus::Safe,
        "diagnostics: {:?}",
        probe.diagnostics
    );
}

/// The card's property, on real source: on each of the caller's three modeled
/// early-return arms the identity produced by the constructor call is released
/// exactly once, and nothing else is left live.
#[test]
fn the_caller_cleanup_arms_discharge_the_constructed_chunk() {
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
        3,
        "expected the three modeled cleanup arms; exits were {:?}",
        exit_shape(caller)
    );
    let acquisitions = discharged
        .iter()
        .map(|exit| {
            assert_eq!(
                exit.sites.len(),
                1,
                "a cleanup arm accounts for exactly the constructed chunk"
            );
            assert_eq!(exit.sites[0].disposition, SiteDisposition::Released);
            exit.sites[0].acquisition_line
        })
        .collect::<Vec<_>>();
    assert_eq!(
        acquisitions[0], acquisitions[1],
        "all three arms release the same acquisition site"
    );
    assert_eq!(acquisitions[1], acquisitions[2]);
    // No promotion: the function itself stays Unknown.
    assert_eq!(caller.status, AnalysisStatus::Unknown);
}

/// Acceptance item 3: the exits that cross an aggregate stay `Unknown`, in
/// both directions — no discharge claimed, and no leak alleged either.
#[test]
fn the_aggregate_and_logging_arms_stay_unknown() {
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
        3,
        "expected the logging arm, the routes-mask arm and the success exit; \
         exits were {:?}",
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
    // Two of them — the success exit and the routes-mask arm — still carry
    // both identities, each unresolved rather than alleged either way.
    assert_eq!(
        unknown
            .iter()
            .filter(|exit| exit.sites.len() == 2
                && exit
                    .sites
                    .iter()
                    .all(|site| site.disposition == SiteDisposition::Mixed))
            .count(),
        2,
        "exits were {:?}",
        exit_shape(caller)
    );
    // The logging arm crosses no acquisition at all.
    assert_eq!(
        unknown.iter().filter(|exit| exit.sites.is_empty()).count(),
        1
    );
}

/// Acceptance item 1, on real source: the constructor's declared contract is an
/// assumption this fixture NAMES, not a fact its body discharges. Its own
/// rollback and success exits stay `Unknown` because the chunk is published
/// into `st->chunks` (O00 blockers B02/B03).
#[test]
fn the_constructor_body_does_not_discharge_its_own_declared_contract() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let constructor = function(program(&result), "cio_chunk_open");
    assert_eq!(constructor.status, AnalysisStatus::Unknown);
    let shape = exit_shape(constructor);
    let discharged = shape
        .iter()
        .filter(|(custody, sites)| *custody == ExitCustody::Discharged && sites.is_empty())
        .count();
    assert_eq!(
        discharged, 4,
        "the four guard/allocation-failure exits acquire nothing: {shape:?}"
    );
    // DELIBERATE PIN UPDATE, card O04-B08: the per-exit *site count* on the two
    // unresolved exits moved from one to two. `ch->name = strdup(name);` is an
    // admitted aggregate store under typed premises
    // (`crate::aggregate_store`), so the `strdup` allocation this projection
    // previously erased is now a tracked identity of this body. The
    // load-bearing half is unchanged and still asserted: the constructor keeps
    // status `Unknown`, its four guard exits acquire nothing, and **every**
    // site on both unresolved exits stays `Mixed` — the projection got finer,
    // not more permissive. The exact arity is pinned separately below so a
    // silent change in either direction still fails.
    let unresolved = shape
        .iter()
        .filter(|(custody, sites)| {
            *custody == ExitCustody::Unknown
                && !sites.is_empty()
                && sites
                    .iter()
                    .all(|disposition| *disposition == SiteDisposition::Mixed)
        })
        .count();
    assert_eq!(
        unresolved, 2,
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

/// Acceptance item 3: the discharge depends on the declared modes and on the
/// record declarations, and the analysis re-derives both. Each substitution is
/// applied to the same fixture and must remove the discharged arms.
#[test]
fn substituting_a_dependency_identity_withdraws_the_discharged_arms() {
    if !clang_available() {
        return;
    }
    let baseline = real_unit();
    let cases: [(&str, &[(&str, &str)]); 4] = [
        (
            "the read record gains a member the reader cannot resolve",
            &[(
                "struct flb_storage_input {",
                "struct flb_storage_input {\n    void (*hook[1])(int);",
            )],
        ),
        (
            "the read member becomes volatile",
            &[(
                "    struct cio_ctx *cio;",
                "    struct cio_ctx *volatile cio;",
            )],
        ),
        (
            "the read member becomes _Atomic",
            &[(
                "    struct cio_stream *stream;",
                "    struct cio_stream *_Atomic stream;",
            )],
        ),
        (
            "the base is passed at a consuming position",
            &[(
                "    generate_chunk_name(in, name, sizeof(name) - 1);",
                "    flb_free(storage);\n    generate_chunk_name(in, name, sizeof(name) - 1);",
            )],
        ),
    ];
    for (label, edits) in cases {
        let mut mutated = baseline.clone();
        for (from, to) in edits {
            assert!(mutated.contains(from), "{label}: anchor `{from}` not found");
            mutated = mutated.replacen(from, to, 1);
        }
        let result = analyze_real(&mutated);
        let caller = function(program(&result), "flb_input_chunk_create");
        let discharged = caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged)
            .count();
        assert_eq!(caller.status, AnalysisStatus::Unknown, "{label}");
        // **Deliberate move, card NULL-SAFETY-PROPERTY** (rule 14n, USER
        // DECISION 2026-10-10, decision 1): with the unresolvable member the
        // read `storage = in->storage` binds a target-free, possibly-null
        // borrowed alias, whose dereference at `storage->cio` was the only
        // refusal on the three cleanup arms. It is now read non-null under
        // the named premise on `storage` the summary lists, so those arms
        // discharge resting on it; the function stays `Unknown` (the rollback
        // and success exits are unresolved). The other substitutions still
        // withdraw every discharge.
        if label == "the read record gains a member the reader cannot resolve" {
            assert_eq!(
                discharged,
                3,
                "{label}: the cleanup arms discharge under the premise, exits were {:?}",
                exit_shape(caller)
            );
            assert!(
                caller
                    .null_premises
                    .iter()
                    .any(|premise| premise.variable == "storage"),
                "{label}: the premise on `storage` is listed: {:?}",
                caller.null_premises
            );
            continue;
        }
        assert_eq!(
            discharged,
            0,
            "{label}: the discharge must be withdrawn, exits were {:?}",
            exit_shape(caller)
        );
    }
}

/// The declared release mode is what discharges the arms. Weakening it to
/// `Borrowed` must turn each arm into a reported residual, never a silent
/// discharge — the direct form of "do not infer liveness from the arm's shape".
#[test]
fn a_borrowed_destructor_turns_every_cleanup_arm_into_a_residual() {
    if !clang_available() {
        return;
    }
    let baseline = PATH_CONTRACTS.replace("\r\n", "\n");
    let weakened = baseline.replace(
        "    ensures normal resource_released(ch, nonnull);\n    ensures normal resource_borrowed(delete);",
        "    ensures normal resource_borrowed(ch);\n    ensures normal resource_borrowed(delete);",
    );
    assert_ne!(weakened, baseline, "the destructor clause moved");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &real_unit(),
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_path.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    let caller = function(program(&result), "flb_input_chunk_create");
    let residual = caller
        .exits
        .iter()
        .filter(|exit| exit.custody == ExitCustody::Residual)
        .count();
    assert_eq!(
        residual,
        3,
        "the three arms now retain the chunk: {:?}",
        exit_shape(caller)
    );
    assert!(
        caller
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "and the retained identity is reported"
    );
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

// The consuming sink accepts each object-pointer type used by the fixtures,
// like free. Its custody contract still consumes its argument; the rejection
// cases must reach the resource projection with well-typed C.
const GENERIC_DECLARATIONS: &str = "\
struct g_ctx; struct g_pool; struct g_handle; \
struct g_conf { int mode; struct g_ctx *ctx; struct g_pool *pool; }; \
struct g_env { int flags; struct g_conf *conf; }; \
union g_view { struct g_ctx *ctx; int tag; }; \
struct g_holder { int kind; union g_view *view; }; \
struct g_handle *g_open(struct g_ctx *ctx, struct g_pool *pool, int flags); \
void g_close(struct g_handle *h); \
int g_probe(struct g_handle *h); \
int g_prepare(struct g_handle *h, int n); \
void g_adopt(void *h); \
void g_note(int first, ...);";

/// The positive case: acquisition, two failure arms that clean up, and a
/// success exit that hands the identity back to the caller.
const GENERIC_ACQUIRE: &str = "\
struct g_handle *w_acquire(struct g_env *env, int n) \
{ \
    int ready; \
    struct g_conf *conf; \
    struct g_handle *h; \
    conf = env->conf; \
    h = g_open(conf->ctx, conf->pool, n); \
    if (!h) { return 0; } \
    ready = g_probe(h); \
    if (ready == -1) { g_close(h); return 0; } \
    ready = g_prepare(h, n); \
    if (ready == -1) { g_close(h); return 0; } \
    return h; \
}";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{GENERIC_DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_generic.qcore", GENERIC_CONTRACTS),
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

#[test]
fn a_generic_constructor_caller_path_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let certificate = certified(GENERIC_ACQUIRE, "w_acquire");
    let acquire = function(&certificate.summary, "w_acquire");
    let shape = exit_shape(acquire);
    assert!(
        shape
            .iter()
            .all(|(custody, _)| *custody == ExitCustody::Discharged),
        "every exit of a fully modeled path is settled: {shape:?}"
    );
    // The two cleanup arms release; the success exit hands the identity back.
    assert_eq!(
        shape
            .iter()
            .filter(|(_, sites)| sites == &vec![SiteDisposition::Released])
            .count(),
        2
    );
    assert_eq!(
        shape
            .iter()
            .filter(|(_, sites)| sites == &vec![SiteDisposition::Escaped])
            .count(),
        1
    );
}

/// Acceptance item 3's first half: the caller's own callee summary is inferred,
/// validated by re-analysis, and independently re-derived by the recheck.
#[test]
fn the_callee_summary_is_validated_and_independently_rederived() {
    if !clang_available() {
        return;
    }
    let user = "void w_use(struct g_env *env) { struct g_handle *h; h = w_acquire(env, 1); \
                if (!h) { return; } g_close(h); }";
    let certificate = certified(&format!("{GENERIC_ACQUIRE} {user}"), "w_use");
    let summary = certificate
        .summaries
        .iter()
        .find(|record| record.function == "w_acquire")
        .expect("a published summary for the wrapper");
    assert_eq!(
        summary.trust,
        qcore_c_gateway::legacy_c::ContractTrust::InferredValidated
    );
    // The recheck already ran inside `certified`; repeat it explicitly so the
    // rederivation is the assertion rather than a side effect.
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn a_leak_on_one_arm_is_reported_as_a_residual() {
    if !clang_available() {
        return;
    }
    let leaky = GENERIC_ACQUIRE.replacen(
        "if (ready == -1) { g_close(h); return 0; } \\",
        "if (ready == -1) { return 0; } \\",
        1,
    );
    let leaky = if leaky == GENERIC_ACQUIRE {
        GENERIC_ACQUIRE.replacen(
            "if (ready == -1) { g_close(h); return 0; }",
            "if (ready == -1) { return 0; }",
            1,
        )
    } else {
        leaky
    };
    assert_ne!(leaky, GENERIC_ACQUIRE, "the first cleanup arm was removed");
    let result = analyze_generic(&leaky, "w_acquire");
    let acquire = function(program(&result), "w_acquire");
    assert_eq!(acquire.status, AnalysisStatus::Unsafe);
    assert!(
        acquire
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. }))
    );
    assert_eq!(
        exit_shape(acquire)
            .iter()
            .filter(|(custody, sites)| *custody == ExitCustody::Residual
                && sites == &vec![SiteDisposition::Live])
            .count(),
        1,
        "the arm that stopped releasing is the residual: {:?}",
        exit_shape(acquire)
    );
}

#[test]
fn an_invalid_transfer_is_reported_and_never_discharges() {
    if !clang_available() {
        return;
    }
    let invalid = GENERIC_ACQUIRE.replacen(
        "ready = g_prepare(h, n);",
        "g_adopt(h); ready = g_prepare(h, n);",
        1,
    );
    let result = analyze_generic(&invalid, "w_acquire");
    let acquire = function(program(&result), "w_acquire");
    assert_eq!(acquire.status, AnalysisStatus::Unsafe);
    assert!(
        acquire.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::ReleaseAfterEscape { .. }
        )),
        "diagnostics: {:?}",
        acquire.diagnostics
    );
    // The arms *before* the transfer are unaffected; every exit the transfer
    // reaches becomes unresolved, and none of them claims a discharge.
    let shape = exit_shape(acquire);
    assert_eq!(
        shape
            .iter()
            .filter(|(custody, _)| *custody == ExitCustody::Discharged)
            .count(),
        2,
        "only the acquisition-failure arm and the arm before the transfer: {shape:?}"
    );
    assert_eq!(
        shape
            .iter()
            .filter(|(custody, _)| *custody == ExitCustody::Unknown)
            .count(),
        2,
        "the transfer leaves the arms it reaches unresolved: {shape:?}"
    );
}

/// Acceptance item 1: a success-looking integer is not a liveness fact. The
/// callee's `0` return says nothing about custody, so an arm that returns on it
/// without releasing is a reported residual.
#[test]
fn a_success_looking_integer_does_not_discharge_the_identity() {
    if !clang_available() {
        return;
    }
    let optimistic = GENERIC_ACQUIRE.replacen(
        "ready = g_prepare(h, n); if (ready == -1) { g_close(h); return 0; }",
        "ready = g_prepare(h, n); if (ready == 0) { return 0; }",
        1,
    );
    assert_ne!(optimistic, GENERIC_ACQUIRE);
    let result = analyze_generic(&optimistic, "w_acquire");
    let acquire = function(program(&result), "w_acquire");
    assert_eq!(acquire.status, AnalysisStatus::Unsafe);
    assert_eq!(
        exit_shape(acquire)
            .iter()
            .filter(|(custody, sites)| *custody == ExitCustody::Residual
                && sites == &vec![SiteDisposition::Live])
            .count(),
        1,
        "the `ready == 0` arm keeps the identity: {:?}",
        exit_shape(acquire)
    );
}

/// Everything outside the whitelist keeps its existing `Unknown` and its
/// existing diagnostic.
#[test]
fn ambiguous_actual_shapes_keep_their_unknown() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 5] = [
        // O04-ARMS deliberately widened this row. C02-P refused every chain by
        // construction; the bounded chain rule now admits one whose every
        // intermediate link is declared exactly `struct POINTEE *` over a
        // complete non-union record, so `env->conf->ctx` is modeled (pinned
        // positively by `o04_arms.rs`). The load-bearing half is kept here on a
        // chain the reader did *not* fully type: `view` is a `union` pointer,
        // so the walk refuses at its intermediate link and the statement keeps
        // both its `Unknown` and its existing diagnostic.
        (
            "a chain through a link the reader did not type is not a direct member read",
            "struct g_handle *w(struct g_holder *hold, int n) { struct g_handle *h; \
             h = g_open(hold->view->ctx, 0, n); g_close(h); return 0; }",
        ),
        (
            "a union base is never admitted",
            "struct g_handle *w(union g_view *v, int n) { struct g_handle *h; \
             h = g_open(v->ctx, 0, n); g_close(h); return 0; }",
        ),
        (
            "a consuming position is not a borrowed position",
            "void w(struct g_conf *conf) { g_adopt(conf->ctx); }",
        ),
        (
            "an actual beyond the declared vector is never admitted",
            "void w(struct g_conf *conf) { g_note(0, conf->ctx); }",
        ),
        (
            "a base the projection may bind is never admitted",
            "struct g_handle *w(struct g_env *env, int n) { struct g_handle *h; \
             struct g_conf *conf; conf = env->conf; g_adopt(conf); \
             h = g_open(conf->ctx, conf->pool, n); g_close(h); return 0; }",
        ),
    ];
    for (label, body) in cases {
        let result = analyze_generic(body, "w");
        let w = function(program(&result), "w");
        assert_eq!(w.status, AnalysisStatus::Unknown, "{label}");
        // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS moves this row deliberately:
        // its call argument rule reads `conf->ctx` and `conf->pool` at
        // `Borrowed` positions under `conf`'s dereference obligation, which
        // refuses the consumed base (`UnknownAlias` of `conf`) in place of the
        // gate's diagnostic. The body stays `Unknown`.
        // Card CG13 (rule 10w) moves the union-link and union-base rows
        // deliberately (the base row keeps its label as written): `view`
        // is a union step and `ctx` read through it a borrow at `g_open`'s
        // borrowed position, so the gate refusal gives way to the fixture's
        // own obligation (`h` used by `g_close` without a null test). Still
        // Unknown, never Safe.
        if matches!(
            label,
            "a chain through a link the reader did not type is not a direct member read"
                | "a union base is never admitted"
        ) {
            assert!(
                w.diagnostics.iter().all(|diagnostic| !matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::Unsupported { description } if description == INDIRECT
                )) && w.diagnostics.iter().any(|diagnostic| matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::Unsupported { description }
                        if description == "nullable resource `h` used by a non-null contract"
                )),
                "{label}: the typed actual exposes the unguarded result, got {:?}",
                w.diagnostics
            );
            continue;
        }
        if label == "a base the projection may bind is never admitted" {
            assert!(
                w.diagnostics.iter().any(|diagnostic| matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::UnknownAlias { variable } if variable == "conf"
                )),
                "{label}: the base's obligation must refuse, got {:?}",
                w.diagnostics
            );
            continue;
        }
        assert!(
            w.diagnostics.iter().any(|diagnostic| matches!(
                &diagnostic.kind,
                LegacyDiagnosticKind::Unsupported { description } if description == INDIRECT
            )),
            "{label}: existing diagnostic must be retained, got {:?}",
            w.diagnostics
        );
    }
}

/// Acceptance item 3's second half: a certificate resting on the borrowed
/// pointer-field admission is refused when only the *declaration* it rests on
/// is substituted, with the statement left byte-identical.
#[test]
fn a_certificate_resting_on_the_admission_is_rederived_from_source() {
    if !clang_available() {
        return;
    }
    let certificate = certified(GENERIC_ACQUIRE, "w_acquire");
    let substitutions: [(&str, &str, &str); 4] = [
        (
            "the record becomes a union",
            "struct g_conf { int mode;",
            "union g_conf { int mode;",
        ),
        (
            "the read member becomes volatile",
            "struct g_ctx *ctx; struct g_pool *pool; }",
            "struct g_ctx *volatile ctx; struct g_pool *pool; }",
        ),
        (
            "the read member becomes an array",
            "struct g_ctx *ctx; struct g_pool *pool; }",
            "struct g_ctx *ctx[2]; struct g_pool *pool; }",
        ),
        (
            "the record body gains an unreadable member",
            "struct g_conf { int mode;",
            "struct g_conf { void (*hook[1])(int); int mode;",
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
    // The unmodified certificate still rechecks, so the four refusals above
    // were caused by the substitutions and not by an unrelated failure.
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}

/// Not an assertion: prints the measured per-exit ledger of the real path so
/// the archived report can be regenerated. Run with
/// `cargo test -p qcore-c-gateway --test o04_constructor_caller -- --ignored
///  --nocapture print_the_measured_exit_ledger`.
#[test]
#[ignore = "reporting aid, not an assertion"]
fn print_the_measured_exit_ledger() {
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
        for exit in &measured.exits {
            println!(
                "  {:<11?} {:<48} sites={:?}",
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
