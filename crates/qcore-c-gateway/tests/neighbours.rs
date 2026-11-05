//! NEIGHBOURS — production declared contract text for the six uncontracted
//! callees standing between `flb_input_chunk_create` and its production
//! discharge (`docs/reports/remaining-neighbours/README.md`).
//!
//! Card PROD-CONTRACTS landed the logging family, the name accessor and the
//! routes-mask helpers, and measured that C6 — the constructor's success exit —
//! still refuses in production. Its
//! `the_remaining_production_blocker_is_one_uncontracted_neighbour` isolated the
//! cause by withdrawing ONE declaration and reproducing the production refusal
//! exactly, and named six uncontracted neighbours: `generate_chunk_name`,
//! `cio_chunk_is_up`, `cio_chunk_up_force`, `input_chunk_write_header`,
//! `flb_time_now` and `cio_chunk_down`.
//!
//! This card reads each of those six pinned bodies and lands all six as
//! production contract text, in three new modules.
//!
//! **Every declaration is a TRUSTED ASSUMPTION.** Each was read against the
//! pinned upstream body at Fluent Bit
//! `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and is cited by file and line in
//! its module header; nothing in this card analyzes a C body. A function-level
//! `Safe` that rests on one is `Safe` **relative to** it, and is never a proof
//! of the C implementation.
//!
//! Three legs, all through the public gateway entry points only: a held-out
//! generic `n_*` family, the byte-verbatim real source, and adversarial
//! substitutions at every new site class.

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
const HELPERS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_helpers.qv"
);
const TIME: &str =
    include_str!("../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/time.qv");
const RESIDENCY: &str =
    include_str!("../../../providers/qcore-provider-chunkio/contracts/chunkio/chunk_residency.qv");

/// The production modules card PROD-CONTRACTS landed, byte-unchanged.
const LOGGING: &str =
    include_str!("../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv");
const CHUNK_NAME: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_name.qv"
);
const ROUTES_MASK: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/routes_mask.qv"
);
/// Card HASH-CONTRACTS' module, byte-unchanged.
const PUBLICATION: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/hash_table_publication.qv"
);
/// The production constructor/destructor pair, byte-unchanged.
const OPEN_CLOSE: &str = include_str!(
    "../../../providers/qcore-provider-chunkio/contracts/chunkio/resource_open_close.qv"
);

/// The earlier cards' fixture premise sets, all byte-unchanged.
const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
const OTHER_CALLEES: &str = include_str!("fixtures/contracts/hash_contracts_callees.qr");

const GENERIC: &str = include_str!("fixtures/contracts/neighbours_generic.qv");

/// The existing rejection an uncontracted call in a condition keeps.
const CONDITION: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())]))
        .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent NEIGHBOURS test: {reason}");
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
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        other => panic!("expected an analyzable summary: {other:?}"),
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

/// A production module's declarations without its `module contract;` header, so
/// several modules concatenate into one loadable text. The module texts
/// themselves are byte-unchanged; only the concatenation drops the repeated
/// header, and the assertion below fails if a module ever stops carrying one.
fn declarations_of(module: &str) -> String {
    let module = normalize_newlines(module);
    let header = "module contract;\n";
    assert!(
        module.starts_with(header),
        "a production contract module must declare `module contract;`"
    );
    module[header.len()..].to_owned()
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

const DECLARATIONS: &str = "\
void *n_open(int size); \
void n_close(void *handle); \
int n_is_up(void *handle); \
int n_up(void *handle); \
int n_down(void *handle); \
void n_name(void *instance, char *out_buf, int buf_size); \
int n_header(void *handle, int kind, char *tag, int tag_len); \
double n_clock(void);";

fn analyze_with(contracts: &str, body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("neighbours_generic.qcore", contracts),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn summary_with(contracts: &str, body: &str, entry: &str) -> LegacyFunctionSummary {
    function(program(&analyze_with(contracts, body, entry)), entry).clone()
}

fn summary_of(body: &str, entry: &str) -> LegacyFunctionSummary {
    summary_with(&normalize_newlines(GENERIC), body, entry)
}

/// The generic mirror of `flb_input_chunk_create`'s residency shape: acquire,
/// interrogate, transition, write a header, read a clock, transition back, and
/// hand the handle to the caller. Every failure arm releases, and the success
/// arm escapes.
const RESIDENCY_BODY: &str = "\
void *n_create(void *instance, char *tag, int tag_len) { \
    char name[64]; \
    void *handle; \
    int ret; \
    int set_down = 0; \
    double created; \
    n_name(instance, name, 63); \
    handle = n_open(64); \
    if (!handle) { return 0; } \
    ret = n_is_up(handle); \
    if (ret == 0) { \
        ret = n_up(handle); \
        if (ret == -1) { n_close(handle); return 0; } \
        set_down = 1; \
    } \
    ret = n_header(handle, 1, tag, tag_len); \
    if (ret == -1) { n_close(handle); return 0; } \
    created = n_clock(); \
    (void) created; \
    if (set_down == 1) { n_down(handle); } \
    return handle; \
}";

/// **The headline for the generic family.** The whole residency shape is
/// admitted end to end and the caller's own obligation settles on every exit:
/// no refusal, and no leak.
#[test]
fn the_residency_shape_is_admitted_end_to_end() {
    if !clang_available() {
        return;
    }
    let summary = summary_of(RESIDENCY_BODY, "n_create");
    assert!(
        unsupported(&summary).is_empty(),
        "unexpected refusals: {:?}",
        unsupported(&summary)
    );
    assert!(
        leaks(&summary).is_empty(),
        "unexpected leaks: {:?}",
        leaks(&summary)
    );
    let result = analyze_with(&normalize_newlines(GENERIC), RESIDENCY_BODY, "n_create");
    let certificate = match result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected Safe for the admitted residency shape: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate)
        .expect("the certificate rechecks from its own retained sources");
}

/// **Precision in the defect direction too.** The same body with one failure
/// arm's release deleted is `Unsafe` with exactly one leak: admitting the
/// residency calls does not admit the leak they surround.
#[test]
fn an_admitted_residency_shape_does_not_hide_a_leak() {
    if !clang_available() {
        return;
    }
    let leaky = RESIDENCY_BODY.replace(
        "if (ret == -1) { n_close(handle); return 0; } \
    created = n_clock();",
        "if (ret == -1) { return 0; } \
    created = n_clock();",
    );
    assert_ne!(leaky, RESIDENCY_BODY, "the substitution did not apply");
    let summary = summary_of(&leaky, "n_create");
    assert_eq!(
        leaks(&summary).len(),
        1,
        "expected exactly one leak: {:?} / {:?}",
        leaks(&summary),
        unsupported(&summary)
    );
}

/// **Adversarial row 1: the modes are load-bearing in BOTH directions.**
///
/// `n_down` declared `resource_released` — the mode a careless reading of
/// "puts the chunk down" would reach for — discharges the caller's obligation
/// instead, so the arm that drops the handle without closing stops leaking.
/// That is precisely why the residency modes had to be read off the bodies:
/// neither `cio_chunk_down` nor `cio_chunk_up_force` frees the chunk, so
/// `resource_released` would be an unfaithful declaration that ERASES a real
/// obligation.
#[test]
fn the_residency_modes_are_load_bearing_in_both_directions() {
    if !clang_available() {
        return;
    }
    // A body whose only exit drops the handle after a `n_down`.
    let body = "\
void n_probe(void) { \
    void *handle; \
    handle = n_open(64); \
    if (!handle) { return; } \
    n_down(handle); \
}";
    let faithful = summary_of(body, "n_probe");
    assert_eq!(
        leaks(&faithful).len(),
        1,
        "the faithful borrowed declaration keeps the leak visible: {:?}",
        unsupported(&faithful)
    );
    let generic = normalize_newlines(GENERIC);
    let released = generic.replace(
        "extern fn n_down(handle: ptr<mut, u8>) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(handle);",
        "extern fn n_down(handle: ptr<mut, u8>) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(handle, nonnull);",
    );
    assert_ne!(released, generic, "the substitution did not apply");
    let substituted = summary_with(&released, body, "n_probe");
    assert!(
        leaks(&substituted).is_empty(),
        "the unfaithful `resource_released` discharges the obligation: {:?}",
        leaks(&substituted)
    );
}

/// **Adversarial row 2: withdrawing each declaration restores its refusal.**
/// Six substitutions on the same generic body, one per declaration this card
/// lands, each asserted to restore a refusal naming that callee.
#[test]
fn withdrawing_each_generic_declaration_restores_its_refusal() {
    if !clang_available() {
        return;
    }
    let generic = normalize_newlines(GENERIC);
    for (name, declaration) in [
        (
            "n_name",
            "extern fn n_name(instance: ptr<mut, u8>, out_buf: ptr<mut, u8>, buf_size: i32) -> unit",
        ),
        ("n_is_up", "extern fn n_is_up(handle: ptr<mut, u8>) -> i32"),
        ("n_up", "extern fn n_up(handle: ptr<mut, u8>) -> i32"),
        ("n_down", "extern fn n_down(handle: ptr<mut, u8>) -> i32"),
        (
            "n_header",
            "extern fn n_header(handle: ptr<mut, u8>, kind: i32, tag: ptr<mut, u8>, tag_len: i32) -> i32",
        ),
        ("n_clock", "extern fn n_clock() -> f64"),
    ] {
        let start = generic
            .find(declaration)
            .unwrap_or_else(|| panic!("the generic fixture no longer declares {name}"));
        let end = generic[start..]
            .find("};\n")
            .map(|offset| start + offset + 3)
            .expect("a declaration ends with `};`");
        let without = format!("{}{}", &generic[..start], &generic[end..]);
        let summary = summary_with(&without, RESIDENCY_BODY, "n_create");
        let refusals = unsupported(&summary);
        let expected = format!("call to uncontracted function `{name}`");
        assert!(
            refusals.iter().any(|text| *text == expected) || refusals.contains(&CONDITION),
            "withdrawing {name} must restore a refusal: {refusals:?}"
        );
        assert!(
            !refusals.is_empty(),
            "withdrawing {name} left the body fully admitted"
        );
    }
}

/// **Adversarial row 3: a tail actual that may carry a tracked identity is
/// still refused at the new sites.** The residency declarations decide their
/// statements; they do not widen what a logging tail may carry.
#[test]
fn a_tracked_handle_is_never_admitted_where_the_projection_tracks_it() {
    if !clang_available() {
        return;
    }
    // The handle's ADDRESS in a residency argument: an address-of escape is not
    // a borrowed actual under any declaration.
    let body = "\
void n_probe(void) { \
    void *handle; \
    handle = n_open(64); \
    if (!handle) { return; } \
    n_down((void *) &handle); \
    n_close(handle); \
}";
    let summary = summary_of(body, "n_probe");
    assert!(
        !unsupported(&summary).is_empty() || !leaks(&summary).is_empty(),
        "an address-of escape must not be silently admitted"
    );
}

/// **Contract substitution, one per new declaration.** The certificate the
/// admitted body produced is rechecked against a contract table built from
/// SUBSTITUTED text: each of the five pointer-taking declarations in turn has
/// its faithful `resource_borrowed` replaced by `resource_released`, the mode a
/// careless reading would reach for. The independent recheck must refuse every
/// one — the certificate rests on the declared text it retained, not on the
/// shape of the body it described.
#[test]
fn a_certificate_resting_on_the_new_modules_is_rederived_from_its_own_sources() {
    if !clang_available() {
        return;
    }
    let generic = normalize_newlines(GENERIC);
    let result = analyze_with(&generic, RESIDENCY_BODY, "n_create");
    let certificate = match result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected Safe: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate).expect("the unmutated certificate rechecks");
    for (callee, parameter) in [
        ("n_is_up", "handle"),
        ("n_up", "handle"),
        ("n_down", "handle"),
        ("n_header", "handle"),
        ("n_name", "out_buf"),
    ] {
        let start = generic
            .find(&format!("extern fn {callee}("))
            .unwrap_or_else(|| panic!("the fixture no longer declares {callee}"));
        let end = generic[start..]
            .find("};\n")
            .map(|offset| start + offset + 3)
            .expect("a declaration ends with `};`");
        let faithful = format!("ensures normal resource_borrowed({parameter});");
        let unfaithful = format!("ensures normal resource_released({parameter}, nonnull);");
        let declaration = generic[start..end].replace(&faithful, &unfaithful);
        assert_ne!(
            declaration,
            generic[start..end],
            "the {callee} substitution did not apply"
        );
        let substituted = format!("{}{declaration}{}", &generic[..start], &generic[end..]);
        let mut mutated = certificate.clone();
        mutated.contracts = load("neighbours_generic.qcore", &substituted);
        assert!(
            check_legacy_safety_certificate(&mutated).is_err(),
            "a certificate rechecked against a substituted `{callee}` declaration must be refused"
        );
    }
}

// --------------------------------------------------------------------------
// Real source
// --------------------------------------------------------------------------

/// The upstream logging macros, reproduced verbatim from card PROD-CONTRACTS'
/// own test so this card's real-source leg preprocesses to exactly the
/// production shape.
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

fn expanded_prelude() -> String {
    let original = "/* Deliberately uncontracted: the logging and errno expansions. */\n\
void flb_error(const char *format, ...);\n\
void flb_trace(const char *format, ...);\n\
void flb_errno(void);\n";
    let prelude = normalize_newlines(PRELUDE);
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

/// One fixture declaration, cut out by name so the PRODUCTION module supplies
/// it instead. The loader rejects a duplicate function contract, so this is how
/// a fixture premise is replaced by production text rather than shadowed by it.
fn without_declaration(text: &str, signature: &str) -> String {
    let start = text
        .find(signature)
        .unwrap_or_else(|| panic!("the fixture no longer declares `{signature}`"));
    let end = text[start..]
        .find("};\n")
        .map(|offset| start + offset + 3)
        .expect("a declaration ends with `};`");
    format!("{}{}", &text[..start], &text[end..])
}

/// **The six fixture premises this card replaces with production text.** Each
/// name is cut from the fixture that carried it, and supplied instead by one of
/// the three modules under `the canonical contract corpus`. The fixture FILES stay
/// byte-unchanged on disk, so cards O04, O04-ARMS, O04-B08, O04-READ-BOUND,
/// O04-B09B10, O04-TRACE, HASH-CONTRACTS and PROD-CONTRACTS keep their exact
/// premise sets and their exact measurements.
const REPLACED: [&str; 6] = [
    "extern fn cio_chunk_is_up(",
    "extern fn cio_chunk_up_force(",
    "extern fn cio_chunk_down(",
    "extern fn generate_chunk_name(",
    "extern fn input_chunk_write_header(",
    "extern fn flb_time_now(",
];

/// The real caller's premise set with all six neighbours coming from the three
/// PRODUCTION modules, plus card PROD-CONTRACTS' four and the fixture premises
/// for everything else.
fn real_contracts() -> String {
    let mut path = normalize_newlines(PATH_CONTRACTS);
    // PROD-CONTRACTS already takes these two from production modules.
    for signature in [
        "extern fn flb_routes_mask_set_by_tag(",
        "extern fn cio_chunk_open(",
        "extern fn cio_chunk_close(",
    ] {
        path = without_declaration(&path, signature);
    }
    for signature in &REPLACED[..5] {
        path = without_declaration(&path, signature);
    }
    let b08 = without_declaration(&normalize_newlines(B08_CALLEES), REPLACED[5]);
    normalize_newlines(
        &(format!(
        "{path}\n{b08}\n{OTHER_CALLEES}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        declarations_of(LOGGING),
        declarations_of(CHUNK_NAME),
        declarations_of(ROUTES_MASK),
        declarations_of(PUBLICATION),
        declarations_of(HELPERS),
        declarations_of(TIME),
        declarations_of(RESIDENCY),
    )
    // The ChunkIO constructor/destructor pair also comes from production.
    + "\n"
        + &declarations_of(OPEN_CLOSE)),
    )
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
        &load("neighbours_real.qcore", contracts),
    )
    .unwrap_or_else(|error| panic!("real-source fixture rejected by Clang: {error:?}"))
}

/// **THE HEADLINE — DELIBERATELY RE-PINNED by card NEIGHBOURS-CHAINED-TAIL.**
///
/// When this test was written it pinned **exactly one** surviving refusal on the
/// byte-verbatim upstream `flb_input_chunk_create`: an `indirect memory access`
/// on the `flb_error` arm of the `!chunk` exit, whose whole cause was the tail
/// actual `storage->stream->name`. Card NEIGHBOURS-CHAINED-TAIL admits that
/// shape — a tail actual at a **declared variadic data tail** position that is
/// exactly one admitted bounded typed access chain — so the fixture leg now
/// reaches **ZERO** diagnostics. The pin moves in the admitting direction and is
/// therefore stated loudly, with all three of its load-bearing negative halves
/// kept and one added:
///
///   * **31 diagnostics without the six declarations, 0 with them.** Six of the
///     thirty-one name a callee and one is the C6 tail refusal; the other
///     twenty-four are the aliasing and indirect-access cascade those seven
///     leave behind. All three of those facts are still asserted below,
///     unchanged.
///   * **The C6 statement itself is decided**, as before.
///   * **The admission is what decides the residual, and it is narrow.** The
///     added half re-measures the same statement with the same actual made
///     non-conforming — `storage->stream->name + 0`, pointer arithmetic the
///     chain rule refuses — and requires the *old* measurement back, verbatim:
///     exactly one `indirect memory access`. So the zero is the admission's,
///     not a widened gate's. (Card W47-ACCESS-PATH-POSITIONS re-pinned the
///     substitution: the former `storage->stream->name[0]` is a scalar byte the
///     W47 typed call-actual fallback now reads in the declared tail, while
///     pointer arithmetic stays refused by every route.)
///   * **No leak, in every state.**
///
/// Nothing here is promoted, and the reading of a zero has not changed. It is a
/// FUNCTION-LEVEL projection resting on twelve named declared contracts, each a
/// trusted assumption about a body read by hand, and never a proof of the C;
/// `the_real_program_is_not_promoted_by_this_card` keeps the program `Unknown`.
///
/// **And this fixture UNDERSTATES production, measured.** The caller excerpt is
/// byte-verbatim but its prelude is reduced, so the real headers' richer types
/// are absent here. On the pinned 185-unit corpus the same function stays
/// `Unknown` with twenty residual leaves this reduced prelude does not
/// reproduce; the card reports' §4 carries that measurement and this test
/// carries the fixture's.
#[test]
fn the_constructor_keeps_exactly_one_refusal_under_production_declarations() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let result = analyze_real(&source, &real_contracts());
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    assert_eq!(
        unsupported(&summary),
        Vec::<&str>::new(),
        "the constructor's refusal vector moved"
    );
    assert!(
        leaks(&summary).is_empty(),
        "the constructor must not leak: {:?}",
        leaks(&summary)
    );
    assert_eq!(
        summary.diagnostics.len(),
        0,
        "no diagnostic remains: {:?}",
        summary.diagnostics
    );

    // **The added load-bearing half.** The same actual, made non-conforming by
    // a construct every route refuses, restores the exact measurement this test
    // used to pin. The admission is narrow and it is what decides the residual.
    // **Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round
    // 2** (rule 14d): the non-conforming spelling was pointer arithmetic,
    // `storage->stream->name + 0`, which rule 14d now reads at the tail (an
    // interior offset of a path whose roots are never bound), so it decides
    // nothing here any more (asserted below); the control is now `*&` over the
    // same path, which no reader reads.
    let offset = source.replace(
        "storage->stream->name, name);",
        "storage->stream->name + 0, name);",
    );
    assert_ne!(offset, source, "the offset substitution did not apply");
    let read = function(
        program(&analyze_real(&offset, &real_contracts())),
        "flb_input_chunk_create",
    )
    .clone();
    assert_eq!(
        unsupported(&read),
        Vec::<&str>::new(),
        "rule 14d reads an interior offset at the tail"
    );
    assert!(leaks(&read).is_empty());
    let arithmetic = source.replace(
        "storage->stream->name, name);",
        "*&storage->stream->name, name);",
    );
    assert_ne!(
        arithmetic, source,
        "the indirection substitution did not apply"
    );
    let refused = function(
        program(&analyze_real(&arithmetic, &real_contracts())),
        "flb_input_chunk_create",
    )
    .clone();
    assert_eq!(
        unsupported(&refused),
        vec!["indirect memory access is outside the resource projection"],
        "an unread tail actual must keep the original refusal"
    );
    assert!(leaks(&refused).is_empty());

    // And the before-state, measured in the same run so the movement is not
    // asserted from memory.
    let mut without = real_contracts();
    for signature in REPLACED {
        without = without_declaration(&without, signature);
    }
    let before = function(
        program(&analyze_real(&source, &without)),
        "flb_input_chunk_create",
    )
    .clone();
    // **Deliberate pin update, card W44-INDIRECT-CALL-ARGUMENT-ACCESS.** This
    // count was 31. In the before-state (the replaced declarations withdrawn)
    // the call argument rule admits typed reads at `Borrowed` and tail
    // positions of the calls that remain contracted, so 11 fewer diagnostics
    // remain; the rule was measured off and on in one tree, and off it
    // reproduces 31. The load-bearing half, below, is unchanged: the
    // before-state still names every uncontracted callee.
    // **Deliberate pin update, card W45-CALLEE-SUMMARY-CASCADE** (the
    // `AGGREGATE-NULL-TEST-AFTER-RELEASE` repair): 20 → 21. The aggregate null
    // test `if (ic->routes_mask == NULL)` now carries the typed read of `ic`,
    // which does not resolve in this before-state (its constructor's
    // declaration is withdrawn), so one `UnknownAlias ic` is added. Measured
    // with the repair off in the same tree: 20.
    // W46 typed scalar statements additionally retain the unresolved `chunk`
    // base at line 248 when the constructor declarations are withdrawn.
    // **Deliberate pin update, card CG10-BOUND-UNTRACKED-ROOTS** (rule 11d):
    // 22 → 21. The read through `in` at line 204 precedes every refusal on
    // its path (the first uncontracted call is at line 205), so the per-path
    // premise reads `in` as an untracked borrowed object there and its
    // `UnknownAlias in` is gone; `in` keeps its obligation at 237 and 242,
    // after the refused calls. Measured with the premise off in the same
    // tree: 22.
    // **Deliberate pin update, card CONTRACT-NORETURN-AND-BORROWED-TAILS**:
    // 21 → 20 under the production text, which now declares
    // `flb_log_print`'s tail `resource_variadic_borrowed_tail()`. In this
    // before-state the C6 statement's `tag` is risky (the withdrawn
    // `input_chunk_write_header` seeds it), so the data tail refused it; the
    // borrowed tail reads it as a borrowed place (rule 11w, T8) and the tail
    // refusal is gone, while every withdrawn callee keeps its own refusal. The
    // 21 is still measured, in the same run, with the data-tail text.
    // **Deliberate pin update, card NULL-SAFETY-PROPERTY** (rule 14n, USER
    // DECISION 2026-10-10, decision 1): 20 -> 19 and data tail 21 -> 20. In
    // this before-state `storage` (`storage = in->storage`, line 204) is a
    // target-free, possibly-null borrowed alias; its dereference at line 206
    // is read non-null under the named premise
    // `null-deref:flb_input_chunk_create:206:storage` the summary lists, so
    // its `UnknownAlias storage` is gone. Every withdrawn callee keeps its own
    // refusal.
    assert!(
        before
            .null_premises
            .iter()
            .any(|premise| premise.line == 206 && premise.variable == "storage"),
        "the premise on `storage` is listed: {:?}",
        before.null_premises
    );
    assert_eq!(
        before.diagnostics.len(),
        19,
        "the before-state diagnostic count moved: {:?}",
        before.diagnostics
    );
    let before_data_tail = function(
        program(&analyze_real(&source, &with_data_tail(&without))),
        "flb_input_chunk_create",
    )
    .clone();
    assert_eq!(
        before_data_tail.diagnostics.len(),
        20,
        "the data-tail before-state diagnostic count moved: {:?}",
        before_data_tail.diagnostics
    );
    for callee in [
        "generate_chunk_name",
        "cio_chunk_is_up",
        "cio_chunk_up_force",
        "input_chunk_write_header",
        "flb_time_now",
        "cio_chunk_down",
    ] {
        let expected = format!("call to uncontracted function `{callee}`");
        assert!(
            unsupported(&before).iter().any(|text| *text == expected),
            "the before-state must name {callee}"
        );
    }
    // The C6 tail refusal card PROD-CONTRACTS isolated is present before and
    // absent after under the data tail; under the production borrowed tail it
    // is absent in both (CONTRACT-NORETURN-AND-BORROWED-TAILS).
    let tail = "arguments beyond the declared parameters of `flb_log_print` are outside the resource projection";
    assert!(
        unsupported(&before_data_tail).contains(&tail),
        "{:?}",
        unsupported(&before_data_tail)
    );
    assert!(
        !unsupported(&before).contains(&tail),
        "{:?}",
        unsupported(&before)
    );
    assert!(!unsupported(&summary).contains(&tail));
}

/// The contract text with every borrowed tail replaced by the data tail it
/// strengthens: `flb_log_print`'s declaration before card
/// CONTRACT-NORETURN-AND-BORROWED-TAILS, so the earlier cards' measurements
/// stay reproducible.
fn with_data_tail(text: &str) -> String {
    let old = text.replace(
        "ensures normal resource_variadic_borrowed_tail();",
        "ensures normal resource_variadic_data_tail();",
    );
    assert_ne!(old, text, "no borrowed tail to replace");
    old
}

/// **The residual isolated by substitution.** Replacing the one chained
/// pointer-field tail actual with the plain local array the same statement
/// already passes leaves the constructor with ZERO diagnostics.
///
/// This is a PROBE on a derived source, and its substitution is asserted to
/// have applied. It is evidence about where the remaining refusal comes from —
/// one actual, in one argument list, on one failure arm — and it is not a claim
/// about the upstream code, which is byte-unchanged in every other test here.
/// Extending the chained-access admission to a pointer-typed final field is a
/// checker question with a real soundness obligation (the read yields a pointer
/// the projection does not track), and this contract-text card deliberately
/// does not take it.
///
/// **SUPERSEDED, and retained as a control.** Card NEIGHBOURS-CHAINED-TAIL took
/// that checker question. The un-probed source now reaches zero too — see
/// `the_constructor_keeps_exactly_one_refusal_under_production_declarations`,
/// which was re-pinned in the admitting direction and carries the narrowness
/// half. The assertion below is byte-unchanged: the probe still applies and
/// still decides. It measured a real isolation and it still holds; what it no
/// longer isolates is a *difference*.
///
/// One correction the implementation measured, recorded rather than smoothed
/// over: the cause was NOT that the final field is pointer-typed.
/// `borrowable_field` has always admitted a pointer member, and the depth-one
/// SCALAR access `storage->flag` refused in a tail position identically. The
/// cause was premise **T2** of the variadic-tail walker, which refused every
/// member access outright, ahead of the chain rule ever being consulted.
#[test]
fn withdrawing_the_chained_tail_actual_decides_the_constructor() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let probed = source.replace("storage->stream->name, name);", "name, name);");
    assert_ne!(probed, source, "the probe substitution did not apply");
    let summary = function(
        program(&analyze_real(&probed, &real_contracts())),
        "flb_input_chunk_create",
    )
    .clone();
    assert!(
        summary.diagnostics.is_empty(),
        "the chained tail actual is the whole residual: {:?}",
        summary.diagnostics
    );
}

/// **The isolation experiment, re-run against the new state.** Card
/// PROD-CONTRACTS withdrew `input_chunk_write_header` alone and reproduced the
/// production refusal. With the six neighbours now production text, the same
/// one-withdrawal experiment still reproduces the same refusal — the
/// declaration is load-bearing, and it is load-bearing from `the canonical contract corpus`
/// rather than from a fixture.
#[test]
fn withdrawing_one_neighbour_still_reproduces_the_refusal() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let contracts = real_contracts();
    let without = without_declaration(&contracts, "extern fn input_chunk_write_header(");
    assert_ne!(without, contracts, "the withdrawal did not apply");
    let summary = function(
        program(&analyze_real(&source, &without)),
        "flb_input_chunk_create",
    )
    .clone();
    // **Deliberate pin update, card CONTRACT-NORETURN-AND-BORROWED-TAILS.**
    // Under the production borrowed tail the withdrawn neighbour no longer
    // makes the C6 statement's tail refuse (T8 reads `tag` as a borrowed
    // place); the constructor still refuses, at the neighbour itself. Under
    // the data tail the isolated refusal is reproduced exactly as before.
    let refusals = unsupported(&summary);
    let tail = "arguments beyond the declared parameters of `flb_log_print` are outside the resource projection";
    assert!(!refusals.contains(&tail), "{refusals:?}");
    assert!(
        refusals.contains(&"call to uncontracted function `input_chunk_write_header`"),
        "{refusals:?}"
    );
    let data_tail = function(
        program(&analyze_real(&source, &with_data_tail(&without))),
        "flb_input_chunk_create",
    )
    .clone();
    let refusals = unsupported(&data_tail);
    assert!(
        refusals.contains(&tail),
        "the isolated refusal is reproduced: {refusals:?}"
    );
    // The gate stays decided: the neighbour affects the tail actual, not the
    // branch condition.
    assert!(!refusals.contains(&CONDITION), "{refusals:?}");
}

/// **Every one of the six is load-bearing, measured one withdrawal at a time.**
/// Withdrawing any single production declaration leaves the constructor with at
/// least one refusal, so none of the six is decoration.
#[test]
fn withdrawing_any_single_neighbour_leaves_the_constructor_refusing() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let contracts = real_contracts();
    for signature in REPLACED {
        let without = without_declaration(&contracts, signature);
        let summary = function(
            program(&analyze_real(&source, &without)),
            "flb_input_chunk_create",
        )
        .clone();
        assert!(
            !unsupported(&summary).is_empty(),
            "withdrawing `{signature}` left the constructor fully admitted"
        );
    }
}

/// **A `Safe` function is not a `Safe` unit, and this card promotes nothing.**
/// The fixture program is not `Safe`: `cio_chunk_open`, in the same translation
/// unit, keeps its own refusals, exactly as card O04 recorded.
#[test]
fn the_real_program_is_not_promoted_by_this_card() {
    if !clang_available() {
        return;
    }
    let source = real_source();
    let result = analyze_real(&source, &real_contracts());
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "the fixture program must not be reported Safe"
    );
    let summary = function(program(&result), "cio_chunk_open").clone();
    assert!(
        !unsupported(&summary).is_empty() || !leaks(&summary).is_empty(),
        "the constructor's own callee keeps its refusals"
    );
}

/// The modules under test are the files under `the canonical contract corpus`, all six
/// declarations are in force through the real production manifest, and the
/// clock declaration carries its own C result type rather than an integer
/// projection of it.
#[test]
fn the_modules_under_test_are_the_production_ones() {
    assert!(HELPERS.contains("extern fn generate_chunk_name("));
    assert!(HELPERS.contains("extern fn input_chunk_write_header("));
    assert!(RESIDENCY.contains("extern fn cio_chunk_is_up("));
    assert!(RESIDENCY.contains("extern fn cio_chunk_up_force("));
    assert!(RESIDENCY.contains("extern fn cio_chunk_down("));
    // The measured faithfulness point this card rests on: `flb_time_now`
    // returns `double`, so it is declared `-> f64`. Card O04-B08's gateway
    // FIXTURE projected the same function onto `-> u64`; that is acceptable as
    // a fixture premise, where its header says so, and it is not what
    // production text may say.
    assert!(
        TIME.contains("extern fn flb_time_now() -> f64"),
        "the clock declaration must carry its own C result type"
    );
    for integer in [
        "extern fn flb_time_now() -> u64",
        "extern fn flb_time_now() -> i64",
    ] {
        assert!(
            !TIME.contains(integer),
            "an integer projection of a `double` result is an unfaithful declaration"
        );
    }
    // The three modules load together and with the rest of the manifest.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../cycle19-resource-contracts.json");
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_manifest(&manifest).unwrap();
    for name in [
        "generate_chunk_name",
        "input_chunk_write_header",
        "flb_time_now",
        "cio_chunk_is_up",
        "cio_chunk_up_force",
        "cio_chunk_down",
    ] {
        assert!(
            contracts.contract(name).is_some(),
            "{name} is not in force in the production manifest"
        );
    }
    // 9 -> 12 manifest-selected modules, deliberately; 12 -> 14 by card
    // CONSTRUCTOR-RESIDUALS' deliberate addition of `libc/errno.qv` and
    // `monkey/intrusive_list.qv`; 14 -> 16 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's deliberate addition of
    // `libc/pthread_mutex.qv`; 15 -> 16 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER's deliberate addition of
    // `msgpack/packer.qv`, which that card measured at a payoff the earlier
    // one measured at zero; 16 -> 17 by card
    // ADDRESS-OF-MEMBER-DESTROY-UNDECLARED's deliberate addition of
    // `fluent_bit/input_chunk_destroy.qv`, a third declaration of a
    // function this very unit defines, under the same
    // declaration-beats-summary arrangement this card established. This card's
    // six declarations are asserted above by name, so the count moving does
    // not weaken what it pins. 17 -> 18 by card AOM-DESTROY-SIBLINGS, which
    // adds `fluent_bit/input_chunk_destroy_direct_routes.qv`, a fourth
    // declaration of a function this very unit defines. 18 -> 19 by card
    // W29-CORPUS-CONTRACTS, which adds `fluent_bit/input_chunk_sizes.qv` —
    // a fifth and sixth declaration of functions this very unit defines,
    // under the same declaration-beats-summary arrangement. 19 -> 21 by card
    // W31-CORPUS-CONTRACTS-2, which adds `chunkio/chunk_sizes.qv` and
    // `fluent_bit/input_chunk_total_size.qv` — the latter a seventh
    // declaration of a function this very unit defines, on the same
    // arrangement.

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
    // 68 -> 69 by card LIBRDKAFKA-HEADER-INLINES: `librdkafka/header_inlines.qv`.
    assert_eq!(contracts.source_assumptions().len(), 69);
}
