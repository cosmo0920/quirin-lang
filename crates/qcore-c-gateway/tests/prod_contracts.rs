//! PROD-CONTRACTS — production declared contract text for the logging family,
//! the input-chunk name accessor and the routes-mask helpers
//! (`docs/reports/remaining-prod-contracts/README.md`).
//!
//! Card O04-TRACE built the variadic-data-tail vocabulary and the one-level
//! nested-call admission, and deliberately kept its two callee declarations in
//! the gateway fixtures, recording the production reach of both as ZERO
//! (`O04-TRACE-REACH`). This card lands production contract text.
//!
//! **The measured correction it starts from.** Card O04-TRACE's fixture prelude
//! declares `void flb_trace(const char *format, ...)` as a FUNCTION. Upstream it
//! is a function-like MACRO
//! (`include/fluent-bit/flb_log.h:293-296` at Fluent Bit
//! `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`), and under the pinned build's
//! `-DFLB_HAVE_TRACE` it expands to
//!
//! ```text
//! if (flb_log_check(FLB_LOG_TRACE))
//!     flb_log_print(FLB_LOG_TRACE, __FILE__, __LINE__, fmt, ...)
//! ```
//!
//! so no analyzed statement in the pinned corpus ever names `flb_trace`, and a
//! production declaration of that name would be inert. The production modules
//! this card lands therefore declare `flb_log_check` and `flb_log_print` — what
//! the call sites reach — and the real-source leg below drives the byte-verbatim
//! upstream caller through the *upstream macro* so the preprocessor produces
//! exactly the production shape.
//!
//! Three legs, all through the public gateway entry points only.
//!
//! **Held-out generic family.** A `w_*` API unrelated to Fluent Bit, to ChunkIO
//! and to every earlier card's family.
//!
//! **Real source.** The byte-verbatim upstream `flb_input_chunk_create` region
//! the six earlier cards pinned, expanded through the upstream logging macro,
//! under the *production* contract modules.
//!
//! **Adversarial.** A tail actual that may carry a tracked identity is refused
//! at every new site class; withdrawing each declaration restores the exact
//! refusal it removed; the routes-mask mode is exercised in both directions; and
//! the certificate recheck rests on the declared text, not on the shape.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for a
//! synthetic declared custody projection under premises this card names, and
//! `flb_input_chunk_create` keeps function-level status `Unknown` on the real
//! source. Every declaration is a TRUSTED ASSUMPTION about a body read by hand
//! and cited by line; none is a proof of the C implementation.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CSourceInput, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

/// **The three production contract modules this card lands.**
const LOGGING: &str =
    include_str!("../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv");
const CHUNK_NAME: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_name.qv"
);
const ROUTES_MASK: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/routes_mask.qv"
);
/// The production hash publication, needed by the real caller's four
/// publications; card HASH-CONTRACTS' module, byte-unchanged.
const PUBLICATION: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/hash_table_publication.qv"
);

/// The earlier cards' fixture premise sets, all byte-unchanged.
const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
/// O04-B09B10's four non-hash premises; the hash declaration comes from the
/// production module instead, and the loader rejects a duplicate.
const OTHER_CALLEES: &str = include_str!("fixtures/contracts/hash_contracts_callees.qr");

const GENERIC: &str = include_str!("fixtures/contracts/prod_contracts_generic.qv");

/// The existing rejection an uncontracted call in a condition keeps.
const CONDITION: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn generic_contracts() -> String {
    normalize_newlines(GENERIC)
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent PROD-CONTRACTS test: {reason}");
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

fn leaks(summary: &LegacyFunctionSummary) -> Vec<usize> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic.kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => Some(acquisition_line),
            _ => None,
        })
        .collect()
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

const DECLARATIONS: &str = "\
struct w_env { int level; }; \
void *w_open(int size); \
void w_close(void *handle); \
int w_gate(int level); \
void w_print(int level, const char *file, int line, const char *format, ...); \
char *w_name(void *handle); \
int w_mask(void *mask, const char *tag, int tag_len, void *instance);";

fn analyze_with(contracts: &str, body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("prod_contracts_generic.qcore", contracts),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    analyze_with(&generic_contracts(), body, entry)
}

fn summary_of(body: &str, entry: &str) -> LegacyFunctionSummary {
    let result = analyze_generic(body, entry);
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate
            .summary
            .functions
            .get(entry)
            .expect("entry present")
            .clone(),
        _ => function(program(&result), entry).clone(),
    }
}

/// The prologue every generic body shares.
const OPENED: &str = "void *h; h = w_open(1); if (!h) { return; }";

/// The macro-expanded logging shape, verbatim in structure with what
/// `flb_trace("... '%s' ... '%s'", flb_input_chunk_get_name(ic), tag)` becomes:
/// a gate call in the condition, and a printer call whose declared vector is
/// followed by a nested borrowed-handback accessor and one data actual.
const EXPANDED: &str =
    "if (w_gate(5)) w_print(5, \"file.c\", 42, \"chunk '%s' tag '%s'\", w_name(h), tag);";

/// **The headline for the generic family.** The whole expanded shape is
/// admitted, and the caller's own obligation still settles on every exit.
#[test]
fn the_expanded_logging_shape_is_admitted_end_to_end() {
    if !clang_available() {
        return;
    }
    let body = format!("void w_g1(const char *tag) {{ {OPENED} {EXPANDED} w_close(h); }}");
    let result = analyze_generic(&body, "w_g1");
    let certificate = match result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected a validated custody projection: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
}

/// **Precision in the defect direction too.** The same body without the release
/// is `Unsafe` with exactly one leak: the admission decides the statement, it
/// does not erase the accounting around it.
#[test]
fn an_admitted_logging_statement_does_not_hide_a_leak() {
    if !clang_available() {
        return;
    }
    let body = format!("void w_g2(const char *tag) {{ {OPENED} {EXPANDED} }}");
    let summary = summary_of(&body, "w_g2");
    assert_eq!(leaks(&summary).len(), 1, "{:?}", summary.diagnostics);
}

/// **Adversarial row 1 — the tail must be data, at every new site class.**
/// The tracked handle itself, its address, and a member read through it are each
/// refused in a tail position, in the gate's argument and in the nested call's
/// argument, UNDER the declarations.
#[test]
fn a_tail_actual_that_may_carry_an_identity_is_never_admitted() {
    if !clang_available() {
        return;
    }
    for (label, statement) in [
        (
            "the handle itself in a tail position",
            "w_print(5, \"f\", 1, \"%p\", h);",
        ),
        (
            "its address in a tail position",
            "w_print(5, \"f\", 1, \"%p\", &h);",
        ),
        (
            "a dereference in a tail position",
            "w_print(5, \"f\", 1, \"%p\", *(char **) h);",
        ),
        (
            "the handle inside the nested call's arguments",
            "w_print(5, \"f\", 1, \"%s\", w_name(&h));",
        ),
        (
            "the handle in the gate's own argument",
            "if (w_gate(*(int *) h)) w_print(5, \"f\", 1, \"x\");",
        ),
    ] {
        let body = format!("void w_g3(void) {{ {OPENED} {statement} w_close(h); }}");
        let summary = summary_of(&body, "w_g3");
        assert!(
            !summary.diagnostics.is_empty(),
            "{label} was admitted with no diagnostic: {body}"
        );
    }
}

/// **Adversarial row 2 — the tail declaration covers only the tail.** With the
/// declaration in force, an excess actual that is data is admitted and an excess
/// actual that may carry an identity is not; the two differ only in the actual.
#[test]
fn the_declaration_covers_only_what_it_covers() {
    if !clang_available() {
        return;
    }
    let data = format!(
        "void w_g4(const char *tag) {{ {OPENED} w_print(5, \"f\", 1, \"%s\", tag); w_close(h); }}"
    );
    assert!(
        matches!(
            analyze_generic(&data, "w_g4"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "a data tail actual is admitted"
    );
    let identity = format!(
        "void w_g4(const char *tag) {{ {OPENED} w_print(5, \"f\", 1, \"%p\", h); w_close(h); }}"
    );
    assert!(
        !matches!(
            analyze_generic(&identity, "w_g4"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "a tail actual naming the tracked handle is refused under the same declaration"
    );
}

/// **Adversarial row 3 — withdrawing the tail restores the refusal, in both of
/// the two shapes the tail carries.** A nested accessor beyond the declared
/// vector needs the tail (card O04-TRACE's premise N8), and a plain data actual
/// beyond it needs the tail too (the arity gate). The two refusals are distinct
/// and both are asserted, so a later change that keeps one and loses the other
/// fails here.
#[test]
fn withdrawing_the_tail_restores_the_refusal() {
    if !clang_available() {
        return;
    }
    let generic = generic_contracts();
    let without = generic.replace("    ensures normal resource_variadic_data_tail();\n", "");
    assert_ne!(without, generic, "the substitution must actually happen");

    let nested = format!("void w_g5(const char *tag) {{ {OPENED} {EXPANDED} w_close(h); }}");
    let result = analyze_with(&without, &nested, "w_g5");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "without the declared tail the expanded statement is refused"
    );
    let summary = function(program(&result), "w_g5").clone();
    assert!(
        unsupported(&summary)
            .iter()
            .any(|text| text.contains("nested call `w_name` inside a modeled call")),
        "{:?}",
        summary.diagnostics
    );

    let plain = format!(
        "void w_g5b(const char *tag) {{ {OPENED} w_print(5, \"f\", 1, \"%s\", tag); w_close(h); }}"
    );
    assert!(
        matches!(
            analyze_generic(&plain, "w_g5b"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the same call is admitted with the declared tail"
    );
    let result = analyze_with(&without, &plain, "w_g5b");
    let summary = function(program(&result), "w_g5b").clone();
    assert!(
        unsupported(&summary)
            .iter()
            .any(|text| text.contains("beyond the declared parameters of `w_print`")),
        "{:?}",
        summary.diagnostics
    );
}

/// **Adversarial row 4 — the gate declaration is what admits the condition.**
/// Withdrawing `w_gate` restores exactly the branch-condition refusal, which is
/// the production `flb_log_check` situation stated as a test.
#[test]
fn withdrawing_the_gate_restores_the_branch_condition_refusal() {
    if !clang_available() {
        return;
    }
    let generic = generic_contracts();
    let without = generic.replace(
        "extern fn w_gate(level: i32) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(level);\n    ensures normal resource_return_none();\n};\n",
        "",
    );
    assert_ne!(without, generic, "the substitution must actually happen");
    let body = format!("void w_g6(const char *tag) {{ {OPENED} {EXPANDED} w_close(h); }}");
    let result = analyze_with(&without, &body, "w_g6");
    let summary = function(program(&result), "w_g6").clone();
    assert!(
        unsupported(&summary).contains(&CONDITION),
        "{:?}",
        summary.diagnostics
    );
}

/// **Adversarial row 5 — the accessor's result mode, both directions.** With
/// `resource_return_none()` the nested accessor is admitted; substituting an
/// owning result makes the same statement refuse, because a nested call that
/// produces custody may not occupy a position that needs no identity.
#[test]
fn the_accessor_result_mode_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let generic = generic_contracts();
    let owning = generic.replace(
        "extern fn w_name(handle: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(handle);\n    ensures normal resource_return_none();\n};",
        "extern fn w_name(handle: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(handle);\n    ensures normal resource_return_owned(nullable);\n};",
    );
    assert_ne!(owning, generic, "the substitution must actually happen");
    let body = format!("void w_g7(const char *tag) {{ {OPENED} {EXPANDED} w_close(h); }}");
    assert!(
        matches!(
            analyze_generic(&body, "w_g7"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the borrowed-handback reading is admitted"
    );
    assert!(
        !matches!(
            analyze_with(&owning, &body, "w_g7"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "an owning accessor result is refused in the same position"
    );
}

/// **Adversarial row 6 — the routes-mask mode, both directions.** Declared
/// `resource_borrowed`, the call moves nothing: a caller that fails to release
/// afterwards still leaks. Substituting `resource_released` on the same
/// parameter makes the projection discharge the obligation instead — which is
/// exactly why the declared mode had to be read off the body, and why the
/// faithful one is `borrowed`.
///
/// **Pin updated deliberately, card BORROW-RESIDUALS.** This body passes `h`
/// twice, so the *substituted* declaration additionally describes a call that
/// releases and borrows one identity, with nothing ordering the two — the shape
/// `BORROW-LIVENESS-PRESTATE` names. The mutated run is therefore now `Unknown`
/// rather than `Safe`. The load-bearing halves are unchanged and both are still
/// asserted: the faithful mode leaks and the substituted one **discharges** the
/// obligation. What moved is the published class of the deliberately unfaithful
/// declaration, and it moved in the refusing direction. The faithful
/// declaration has no released position at all, so no rule of that card can
/// reach it — asserted below.
#[test]
fn the_routes_mask_mode_is_load_bearing_in_both_directions() {
    if !clang_available() {
        return;
    }
    let body = "void w_g8(const char *tag) { void *h; h = w_open(1); if (!h) { return; } \
                w_mask(h, tag, 3, h); }";
    let borrowed = summary_of(body, "w_g8");
    assert_eq!(
        leaks(&borrowed).len(),
        1,
        "borrowed moves nothing, so the caller still leaks: {:?}",
        borrowed.diagnostics
    );
    assert!(
        !borrowed
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::UnknownAlias { .. })),
        "the faithful declaration releases nothing, so BORROW-RESIDUALS cannot reach it: {:?}",
        borrowed.diagnostics
    );

    let generic = generic_contracts();
    let releasing = generic.replace(
        "    ensures normal resource_borrowed(mask);",
        "    ensures normal resource_released(mask, nonnull);",
    );
    assert_ne!(releasing, generic, "the substitution must actually happen");
    let result = analyze_with(&releasing, body, "w_g8");
    let mutated = match &result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary
            .functions
            .get("w_g8")
            .expect("missing `w_g8`")
            .clone(),
        other => panic!("expected a projection for the substituted declaration: {other:?}"),
    };
    assert!(
        leaks(&mutated).is_empty(),
        "a releasing mask parameter still discharges the obligation: {:?}",
        mutated.diagnostics
    );
    assert!(
        mutated.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "h"
        )),
        "…and the substituted declaration now releases and borrows one identity: {:?}",
        mutated.diagnostics
    );
}

/// **Adversarial row 7 — the certificate rests on the declared text.**
/// Withdrawing the tail clause from a certificate's own retained sources leaves
/// the loaded contract table byte-identical, and the independent recheck
/// refuses.
#[test]
fn a_certificate_resting_on_the_tail_is_rederived_from_its_own_sources() {
    if !clang_available() {
        return;
    }
    let body = format!("void w_g9(const char *tag) {{ {OPENED} {EXPANDED} w_close(h); }}");
    let mut certificate = match analyze_generic(&body, "w_g9") {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected a validated custody projection: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate).expect("the unmodified certificate rechecks");

    let without =
        generic_contracts().replace("    ensures normal resource_variadic_data_tail();\n", "");
    let withdrawn = load("prod_contracts_generic.qcore", &without);
    // The tail is not carried on a contract, so the loaded table is unchanged.
    for name in ["w_open", "w_close", "w_gate", "w_print", "w_name", "w_mask"] {
        assert_eq!(
            certificate.contracts.contract(name),
            withdrawn.contract(name),
            "{name}: the loaded contract table must be byte-identical"
        );
    }
    certificate.contracts = withdrawn;
    assert!(
        check_legacy_safety_certificate(&certificate).is_err(),
        "the recheck must refuse a certificate whose retained sources no longer declare the tail"
    );
}

// --------------------------------------------------------------------------
// Real source
// --------------------------------------------------------------------------

/// The upstream logging macros, reproduced from
/// `include/fluent-bit/flb_log.h` at the pinned revision (`:250-252` for
/// `flb_error` and `:293-296` for `flb_trace`, under `-DFLB_HAVE_TRACE`), so the
/// preprocessor expands the byte-verbatim caller's `flb_trace(...)` exactly as
/// the production build does.
///
/// The one deviation from upstream, recorded rather than hidden: `flb_log_check`
/// is upstream a `static inline` WITH a body in the header, and here it is a
/// prototype. The excerpt cannot carry its body (it reads a thread-local worker
/// through several types this reduced prelude does not define), and the
/// declared contract wins over an inferred summary either way.
const MACRO_PRELUDE: &str = "\
/* Card PROD-CONTRACTS: what the call sites actually reach. */\n\
int flb_log_check(int level);\n\
void flb_log_print(int level, const char *file, int line, const char *format, ...);\n\
#define flb_error(fmt, ...) \\\n\
    if (flb_log_check(1)) \\\n\
        flb_log_print(1, ((void *) 0), 0, fmt, ##__VA_ARGS__)\n\
#define flb_trace(fmt, ...) \\\n\
    if (flb_log_check(5)) \\\n\
        flb_log_print(5, __FILE__, __LINE__, fmt, ##__VA_ARGS__)\n\
void flb_errno(void);\n";

/// The o04 prelude with its two logging PROTOTYPES replaced by the upstream
/// MACROS. `fixtures/o04/prelude.c` itself stays byte-unchanged, so cards O04,
/// O04-ARMS, O04-B08, O04-READ-BOUND, O04-B09B10, O04-TRACE and HASH-CONTRACTS
/// keep their exact premise sets and measurements.
fn expanded_prelude() -> String {
    let original = "/* Deliberately uncontracted: the logging and errno expansions. */\n\
void flb_error(const char *format, ...);\n\
void flb_trace(const char *format, ...);\n\
void flb_errno(void);\n";
    let prelude = PRELUDE.replace("\r\n", "\n");
    assert!(
        prelude.contains(original),
        "the o04 prelude no longer carries the logging prototypes this card replaces"
    );
    prelude.replace(original, MACRO_PRELUDE)
}

fn real_source() -> String {
    format!(
        "{}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n",
        expanded_prelude()
    )
}

/// The production modules plus the fixture premises the caller's other callees
/// need. `o04_path.qcore` is used with its own `flb_routes_mask_set_by_tag` and
/// `flb_input_chunk_get_name` declarations REMOVED, because those two now come
/// from the production modules and the loader rejects a duplicate.
fn real_contracts() -> String {
    let path = PATH_CONTRACTS.replace("\r\n", "\n");
    let routes = "extern fn flb_routes_mask_set_by_tag(routes_mask: ptr<mut, u8>, tag: ptr<const, u8>, tag_len: i32, in: ptr<mut, u8>) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(routes_mask);\n    ensures normal resource_borrowed(tag);\n    ensures normal resource_borrowed(tag_len);\n    ensures normal resource_borrowed(in);\n    ensures normal resource_return_none();\n};\n";
    assert!(
        path.contains(routes),
        "the o04 path module no longer carries the routes-mask declaration"
    );
    let path = path.replace(routes, "");
    format!(
        "{path}\n{B08_CALLEES}\n{OTHER_CALLEES}\n{}\n{}\n{}\n{}",
        declarations_of(LOGGING),
        declarations_of(CHUNK_NAME),
        declarations_of(ROUTES_MASK),
        declarations_of(PUBLICATION),
    )
}

/// A production module's declarations without its `module contract;` header, so
/// several modules concatenate into one loadable text. The module texts
/// themselves are byte-unchanged; only the concatenation drops the repeated
/// header, and the assertion below fails if a module ever stops carrying one.
fn declarations_of(module: &str) -> String {
    let module = module.replace("\r\n", "\n");
    let header = "module contract;\n";
    assert!(
        module.starts_with(header),
        "a production contract module must declare `module contract;`"
    );
    module[header.len()..].to_owned()
}

fn analyze_real(source: &str, contracts: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            // The pinned 185-unit compatibility route's own declared input
            // identity (card G02). The expanded `flb_trace` carries `__FILE__`,
            // which an anonymous text must refuse; this is the same declared
            // assumption the production route makes, not a bypass of it.
            input: CSourceInput::HistoricalStdin,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("prod_contracts_real.qcore", contracts),
    )
    .unwrap_or_else(|error| panic!("real-source fixture rejected by Clang: {error:?}"))
}

/// **The headline for the real-source leg.** With the logging macro expanded as
/// the production build expands it, the production declarations decide the
/// statement that card O04-TRACE's fixture reached through a non-existent
/// `flb_trace` function: neither the gate's branch condition nor the printer's
/// call is refused any more.
#[test]
fn the_expanded_logging_statement_is_decided_on_the_real_caller() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let result = analyze_real(&source, &real_contracts());
    let summary = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate
            .summary
            .functions
            .get("flb_input_chunk_create")
            .expect("entry present")
            .clone(),
        _ => function(program(&result), "flb_input_chunk_create").clone(),
    };
    let refusals = unsupported(&summary);
    assert!(
        !refusals
            .iter()
            .any(|text| text.contains("flb_log_print") || text.contains("flb_log_check")),
        "no logging callee is refused as uncontracted: {refusals:?}"
    );
    assert!(
        !refusals.contains(&CONDITION),
        "the gate's branch condition is decided: {refusals:?}"
    );
}

/// **Withdrawing each production declaration restores the exact refusal it
/// removed.** Three substitutions, each on the same real source.
#[test]
fn withdrawing_each_production_declaration_restores_its_refusal() {
    if !clang_available() {
        return;
    }
    let source = real_source();

    // (1) no `flb_log_check`: the branch condition refuses again.
    let without_gate = real_contracts().replace(
        "extern fn flb_log_check(level: i32) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(level);\n    ensures normal resource_return_none();\n};\n",
        "",
    );
    assert_ne!(
        without_gate,
        real_contracts(),
        "the substitution must actually happen"
    );
    let result = analyze_real(&source, &without_gate);
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    assert!(
        unsupported(&summary).contains(&CONDITION),
        "{:?}",
        summary.diagnostics
    );

    // (2) no `flb_log_print`: the print statement refuses as uncontracted.
    let without_print = real_contracts().replace(
        "extern fn flb_log_print(level: i32, file: ptr<const, u8>, line: i32, format: ptr<const, u8>) -> unit",
        "extern fn flb_log_print_unused(level: i32, file: ptr<const, u8>, line: i32, format: ptr<const, u8>) -> unit",
    );
    assert_ne!(
        without_print,
        real_contracts(),
        "the substitution must actually happen"
    );
    let result = analyze_real(&source, &without_print);
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    assert!(
        unsupported(&summary)
            .iter()
            .any(|text| text.contains("call to uncontracted function `flb_log_print`")),
        "{:?}",
        summary.diagnostics
    );

    // (3) no `flb_input_chunk_get_name`: the nested call refuses again.
    let without_name = real_contracts().replace(
        "extern fn flb_input_chunk_get_name(ic: ptr<mut, u8>) -> ptr<mut, u8>",
        "extern fn flb_input_chunk_get_name_unused(ic: ptr<mut, u8>) -> ptr<mut, u8>",
    );
    assert_ne!(
        without_name,
        real_contracts(),
        "the substitution must actually happen"
    );
    let result = analyze_real(&source, &without_name);
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    assert!(
        unsupported(&summary)
            .iter()
            .any(|text| text.contains("nested call")),
        "{:?}",
        summary.diagnostics
    );
}

/// **The remaining production blocker, isolated and named.** In the pinned
/// 185-unit corpus this card's declarations do NOT decide the `flb_trace`
/// statement: it moves from two refusals (uncontracted callee, refused branch
/// condition) to one — `arguments beyond the declared parameters of
/// \`flb_log_print\` are outside the resource projection`, the refusal a
/// declared tail issues when this call site's tail actuals are not data. The
/// cause is a *neighbouring* uncontracted callee, not this card's text: `tag` is
/// also an argument of `input_chunk_write_header`, which production leaves
/// undeclared, so C02-P's `seed_call_arguments` seeds it risky and premise T4
/// fails on it.
///
/// This test measures exactly that on the fixture, by withdrawing that one
/// neighbouring declaration and nothing else: the same statement that the test
/// above shows decided becomes the same refusal the production corpus reports.
#[test]
fn the_remaining_production_blocker_is_one_uncontracted_neighbour() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let header = "extern fn input_chunk_write_header(chunk: ptr<mut, u8>, event_type: i32, tag: ptr<mut, u8>, tag_len: i32) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(chunk);\n    ensures normal resource_borrowed(event_type);\n    ensures normal resource_borrowed(tag);\n    ensures normal resource_borrowed(tag_len);\n    ensures normal resource_return_none();\n};\n";
    let contracts = real_contracts();
    assert!(
        contracts.contains(header),
        "the o04 path module no longer carries the header-writer declaration"
    );
    let without = contracts.replace(header, "");
    // **Deliberate pin update, card CONTRACT-NORETURN-AND-BORROWED-TAILS.**
    // Production now declares `flb_log_print`'s tail
    // `resource_variadic_borrowed_tail()`, which reads the risky `tag` as a
    // borrowed place (rule 11w, T8): with the neighbour withdrawn the
    // constructor refuses at the neighbour itself and no longer at the tail.
    // The measurement this test pins is taken with the data-tail text the
    // declaration had when it was written.
    let borrowed = function(
        program(&analyze_real(&source, &without)),
        "flb_input_chunk_create",
    )
    .clone();
    let borrowed_refusals = unsupported(&borrowed);
    assert!(
        !borrowed_refusals.contains(
            &"arguments beyond the declared parameters of `flb_log_print` are outside the resource projection"
        ),
        "{borrowed_refusals:?}"
    );
    assert!(
        borrowed_refusals.contains(&"call to uncontracted function `input_chunk_write_header`"),
        "{borrowed_refusals:?}"
    );
    let without = without.replace(
        "ensures normal resource_variadic_borrowed_tail();",
        "ensures normal resource_variadic_data_tail();",
    );
    let result = analyze_real(&source, &without);
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    let refusals = unsupported(&summary);
    // Lint-only rewrite (`clippy::manual_contains`, failing `-D warnings` at
    // base main `19fbaa5`; cache-masked in earlier gates). Cards NEIGHBOURS
    // and BORROW-RESIDUALS made the identical fix independently; the asserted
    // text is byte-identical and the assertion is unchanged.
    assert!(
        refusals.contains(
            &"arguments beyond the declared parameters of `flb_log_print` are outside the resource projection"
        ),
        "the production refusal is reproduced by withdrawing one neighbour: {refusals:?}"
    );
    // The gate is still decided — the neighbour affects the tail actual, not
    // the branch condition — which is why the corpus loses 26 condition
    // refusals and keeps one tail refusal at this statement.
    assert!(!refusals.contains(&CONDITION), "{refusals:?}");
}

/// **A function reaching a decided statement is not a unit reaching `Safe`, and
/// this card promotes nothing.** The real caller's status is pinned here so a
/// silent change in either direction fails.
#[test]
fn the_real_caller_is_not_promoted_by_this_card() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let result = analyze_real(&source, &real_contracts());
    // Whatever the projection concludes, the *program* is not Safe: the
    // constructor in the same fixture keeps its own refusals.
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "the fixture program must not be reported Safe"
    );
    let summary = function(program(&result), "cio_chunk_open").clone();
    assert!(
        !summary.diagnostics.is_empty(),
        "the constructor keeps its own refusals"
    );
    assert!(
        !unsupported(&summary).is_empty() || !leaks(&summary).is_empty(),
        "the constructor keeps its own refusals"
    );
}

/// The production modules are the files under `the canonical contract corpus`, and the earlier
/// cards' fixtures are byte-unchanged where this card depends on them.
#[test]
fn the_modules_under_test_are_the_production_ones() {
    assert!(LOGGING.contains("extern fn flb_log_print("));
    // Card CONTRACT-NORETURN-AND-BORROWED-TAILS replaced `flb_log_print`'s data
    // tail with the borrowed tail it strengthens (a deliberate pin update).
    assert!(LOGGING.contains("ensures normal resource_variadic_borrowed_tail();"));
    assert!(CHUNK_NAME.contains("extern fn flb_input_chunk_get_name("));
    assert!(ROUTES_MASK.contains("extern fn flb_routes_mask_set_by_tag("));
    // The measured correction this card rests on: neither macro name is
    // declared in production, because no analyzed statement can name it.
    for name in ["extern fn flb_trace(", "extern fn flb_error("] {
        assert!(
            !LOGGING.contains(name),
            "a function-like macro must not be declared as a production callee"
        );
    }
    // The three modules load together and with the rest of the manifest.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../cycle19-resource-contracts.json");
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_manifest(&manifest).unwrap();
    for name in [
        "flb_log_check",
        "flb_log_print",
        "flb_input_chunk_get_name",
        "flb_routes_mask_set_by_tag",
        "flb_routes_mask_get_size",
    ] {
        assert!(
            contracts.contract(name).is_some(),
            "{name} is not in force in the production manifest"
        );
    }
    // 9 -> 12 by card NEIGHBOURS' deliberate addition of
    // `fluent_bit/input_chunk_helpers.qv`, `fluent_bit/time.qv` and
    // `chunkio/chunk_residency.qv`. This card's own five declarations, its
    // three modules and every assertion above are unchanged: the pin moves
    // because the manifest grew, not because anything measured here did.
    // 12 -> 14 by card CONSTRUCTOR-RESIDUALS' deliberate addition of
    // `libc/errno.qv` and `monkey/intrusive_list.qv`, for the same
    // reason. That card also lands `flb_errno_print` in THIS module, retiring
    // the "deliberately left undeclared" entry this card recorded for it: the
    // blocker it named — the `flb_errno()` macro's dereference refusing the
    // statement before any contract is consulted — no longer holds.
    // 14 -> 15 by card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's deliberate addition
    // of `libc/pthread_mutex.qv`, again for the same reason: the manifest
    // grew, nothing measured here moved.
    // 15 -> 16 by card ABI-TYPEDEF-ADDRESS-OF-MEMBER's deliberate addition of
    // `msgpack/packer.qv`. That card retires the "measured payoff zero"
    // entry the ABI-TYPEDEF card recorded for `msgpack_packer_init` on exactly
    // this rule's own terms: the blocker it named — the address-of-a-member
    // actual refusing the statement before any contract is consulted — no
    // longer holds.
    // 16 -> 17 by card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED's deliberate
    // addition of `fluent_bit/input_chunk_destroy.qv`, again because the
    // manifest grew and nothing measured here moved. That declaration is this
    // rule's own terms once more: the blocker the previous card named — the
    // release through an uncontracted callee that its newly-bound summary
    // exposed — is what the module removes.
    assert!(LOGGING.contains("extern fn flb_errno_print("));
    // 17 -> 18 by card AOM-DESTROY-SIBLINGS, which adds
    // `fluent_bit/input_chunk_destroy_direct_routes.qv` — the sibling
    // release the previous card deliberately left undeclared, read on its own
    // terms and landed with a `nullable` release. 18 -> 19 by card
    // W29-CORPUS-CONTRACTS, which adds `fluent_bit/input_chunk_sizes.qv` —
    // two read-only size queries declared borrowed/none against their pinned
    // bodies, again because the manifest grew and nothing measured here moved.
    // 19 -> 21 by card W31-CORPUS-CONTRACTS-2, which adds
    // `chunkio/chunk_sizes.qv` and
    // `fluent_bit/input_chunk_total_size.qv` — the two size-sibling
    // read-only queries its predecessor named and declined — on the same
    // grounds.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qv`, `cmetrics/context_lifetime.qv`,
    // `ctraces/context_lifetime.qv`, `librdkafka/cjson.qv`,
    // `librdkafka/kafka_mem.qv` and `monkey/logging.qv`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves: `cfl/time.qv`,
    // `cmetrics/metric_values.qv`, `cmetrics/mpack_utils.qv`,
    // `mpack/writer.qv`, `librdkafka/lz4_stream.qv` and
    // `librdkafka/topic_partition.qv`.
    // 33 -> 44 by card W43-LIBC-CONTRACTS: eleven `libc/` modules owned by
    // `providers/quirin-provider-libc` (55 trusted libc/POSIX declarations,
    // each read from the prototype the pinned units declare).
    // 44 -> 45 by card CONTRACT-INHERIT-ADOPTION: `fluent_bit/resource_string.qv`
    // selected by `crate:module` reference (schema 2); `resource_memory.qcore`
    // moved from a path entry to a reference without changing the count.
    // 45 -> 46 by card W43-LIBC-RESTRICT-PROTOTYPES (`libc/regex.qv`).
    // 46 -> 47 by card W43-DESCRIPTOR-RESOURCE-TRACKING: `libc/dirent.qv`.
    // 47 -> 49 by card W45-FB-RETRY-LIFETIME: `fluent_bit/task_users.qv` and
    // `fluent_bit/output_singleplex.qv`.
    // 49 -> 50 by card W45-FB-BACKLOG-CUSTODY: `fluent_bit/storage_backlog.qv`
    // (twelve checked specifications, contract spec C06.3).
    // Combined W46 cards: 54 selected modules, 219 resource declarations, 118 corpus modules.
    // 54 -> 55 by card W47-LOOP-OWNER-CONTRACTS: `fluent_bit/loop_owner.qv`.
    // 55 -> 56 by card W47-CONDVAR-JOIN-PUBLICATION: `libc/pthread_sync.qv`.
    // 56 -> 57 by card W47-CONDVAR-JOIN-STOP-PATHS: `fluent_bit/lib_engine_thread.qv` (five declarations awaiting review).
    // 57 -> 58 by card W47-LOOP-OWNER-INSTANCE-FLOW: `fluent_bit/input_instance.qv`.
    // 58 -> 59 by card CONTRACT-PIN-TIP-MODE: `monkey/vhost_set.qv`.
    // 59 -> 60 by card MONKEY-VHOST-PROPERTY-CUSTODY: `monkey/string.qv`.
    // 60 -> 61 by card W43-ABI-ENUM-AND-CONST-PP: `librdkafka/client.qv`.
    // 61 -> 63 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION:
    // `monkey/event_loop.qv` and `fluent_bit/input_collector_registration.qv`.
    // 63 -> 64 by card FB-TEARDOWN-DESTROY-DECLARATIONS: `fluent_bit/teardown_destroy.qv`.
    // 65 -> 66 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.
    // 66 -> 68 by card LOG-PRINT-BODY-CONTRACTS: `cmetrics/logging.qv` and `chunkio/logging.qv`.
    assert_eq!(contracts.source_assumptions().len(), 68);
}
