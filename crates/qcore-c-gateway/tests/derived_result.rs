//! Card W43-LIBC-RESULT-ALIAS — the derived-result mode.
//!
//! `resource_return_alias(p, n)` (`memcpy` returns `dst`) and
//! `resource_return_interior(p, n)` (`strchr` returns a pointer into `s` or
//! NULL) state that a call's result is derived from a parameter and owns
//! nothing. Every body below exercises one of the rules contract spec §C04.3
//! states for that mode, and asserts the **complete** diagnostic vector of the
//! entry function together with its status, so a rule that silently dropped a
//! pre-existing leak or double release while adding its own answer would fail.
//!
//! The fixtures use a generic `d_*` surface (`fixtures/contracts/
//! derived_result_generic.qcore`), deliberately unrelated to libc: `Safe` here
//! is `Safe` over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::AbiOutcome;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, DerivedExtent, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, ResourceReturnMode, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate, declared_abi_census,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/derived_result_generic.qcore");

/// The C declarations the fixture contracts describe. `d_copy` spells
/// `restrict` after each `*`, as glibc's `memcpy` does, so every body below
/// also exercises the pointer-qualifier reader end to end: without it the
/// declaration would be withdrawn at every call site.
const DECLARATIONS: &str = "\
char *d_alloc(unsigned long n); \
void d_free(void *p); \
void d_consume(void *p); \
unsigned long d_len(const char *s); \
char *d_chr(const char *s, int c); \
void *d_copy(void *restrict dst, const void *restrict src, unsigned long n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "derived_result_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the derived-result fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent derived-result test: {reason}");
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

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`");
    let mut keys = function
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
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    let (measured, keys) = outcome(body);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

// -- The loader ---------------------------------------------------------------

#[test]
fn the_two_predicates_load_to_the_derived_mode_and_record_the_dropped_const() {
    let contracts = contracts();
    assert_eq!(
        contracts.contract("d_chr").map(|contract| contract.result),
        Some(ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Interior,
            nullable: true,
            const_dropped: true,
        })
    );
    assert_eq!(
        contracts.contract("d_copy").map(|contract| contract.result),
        Some(ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Exact,
            nullable: false,
            const_dropped: false,
        })
    );
}

/// Every placement outside a borrowed depth-one pointer base under a depth-one
/// pointer result rejects the whole module; nothing is approximated.
#[test]
fn every_other_placement_is_rejected() {
    let module = |signature: &str, clauses: &str| {
        format!(
            "module contract;\nextern fn k{signature}\ncontract {{\n    \
             requires resource_contract_v1;\n{clauses}}};\n"
        )
    };
    let rejected = [
        (
            "a base that is not borrowed",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_released(p, nullable);\n    \
                 ensures normal resource_return_interior(p, nullable);\n",
            ),
            "a derived result requires a borrowed base parameter",
        ),
        (
            "a scalar base",
            module(
                "(n: usize) -> ptr<mut, u8>",
                "    ensures normal resource_borrowed(n);\n    \
                 ensures normal resource_return_alias(n, nonnull);\n",
            ),
            "a derived result requires a depth-one pointer base and result",
        ),
        (
            "a pointer-to-pointer base",
            module(
                "(p: ptr<mut, ptr<mut, u8>>) -> ptr<mut, u8>",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_interior(p, nullable);\n",
            ),
            "a derived result requires a depth-one pointer base and result",
        ),
        (
            "a pointer-to-pointer result",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, ptr<mut, u8>>",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_alias(p, nonnull);\n",
            ),
            "a derived result requires a depth-one pointer base and result",
        ),
        (
            "a scalar result",
            module(
                "(p: ptr<mut, u8>) -> usize",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_alias(p, nonnull);\n",
            ),
            "owned result requires a pointer return type",
        ),
        (
            "an unknown base name",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_alias(q, nonnull);\n",
            ),
            "unknown parameter q",
        ),
        (
            "a second result mode",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_alias(p, nonnull);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "duplicate result mode",
        ),
        (
            "a malformed nullability",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_borrowed(p);\n    \
                 ensures normal resource_return_interior(p, maybe);\n",
            ),
            "unsupported nullability maybe",
        ),
    ];
    for (label, source, reason) in rejected {
        let error =
            load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), source)]))
                .expect_err(label);
        assert!(error.contains(reason), "{label}: {error}");
    }
}

/// A `const` base under a `const` result drops nothing, and a mutable base
/// under a `const` result adds constness rather than dropping it.
#[test]
fn const_dropped_is_exactly_const_in_and_mutable_out() {
    for (base, result, dropped) in [
        ("ptr<const, u8>", "ptr<mut, u8>", true),
        ("ptr<const, u8>", "ptr<const, u8>", false),
        ("ptr<mut, u8>", "ptr<const, u8>", false),
        ("ptr<mut, u8>", "ptr<mut, u8>", false),
    ] {
        let source = format!(
            "module contract;\nextern fn k(p: {base}) -> {result}\ncontract {{\n    \
             requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    \
             ensures normal resource_return_interior(p, nullable);\n}};\n"
        );
        let contracts =
            load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), source)]))
                .expect("a derived declaration loads");
        assert_eq!(
            contracts.contract("k").map(|contract| contract.result),
            Some(ResourceReturnMode::Derived {
                parameter: 0,
                extent: DerivedExtent::Interior,
                nullable: true,
                const_dropped: dropped,
            }),
            "{base} -> {result}"
        );
    }
}

/// The fixture's `restrict`-qualified prototype agrees with its declaration,
/// so the end-to-end tests below are not measuring a withdrawn contract.
#[test]
fn the_restrict_prototype_agrees_with_its_declaration() {
    let rows = declared_abi_census(&format!("{DECLARATIONS} void w(void) {{ }}"), &contracts());
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row.outcome, AbiOutcome::Agreed, "{}", row.function);
    }
}

// -- A discarded derived result is a discarded `None` -------------------------

#[test]
fn a_discarded_exact_alias_is_admitted_and_the_body_certifies() {
    if !clang_available() {
        return;
    }
    expect(
        "memcpy as a statement",
        "void w(void) { char *b; b = d_alloc(8); d_copy(b, \"x\", 1); d_free(b); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_derived_result_tested_in_a_condition_is_admitted() {
    if !clang_available() {
        return;
    }
    expect(
        "if (strchr(s, c))",
        "void w(const char *s) { if (d_chr(s, 58)) { return; } }",
        "Safe",
        &[],
    );
}

// -- Scanning an untracked string ----------------------------------------------

#[test]
fn scanning_a_borrowed_string_certifies() {
    if !clang_available() {
        return;
    }
    expect(
        "strchr over a caller's string, null-tested, then read",
        "void w(const char *s) { char *p; p = d_chr(s, 58); if (p) { d_len(p); } }",
        "Safe",
        &[],
    );
}

// -- Releasing the base invalidates every derived pointer ---------------------

#[test]
fn an_interior_pointer_used_after_its_base_is_released_is_a_violation_candidate() {
    if !clang_available() {
        return;
    }
    expect(
        "strchr, free(base), strlen(p)",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         d_free(b); d_len(p); }",
        "Unsafe",
        // The borrowed-liveness path rule does not decide a possibly-null
        // actual; the derived rule decides the resolved one.
        &["use_after_release"],
    );
}

#[test]
fn an_exact_alias_used_after_its_base_is_released_is_a_violation_candidate() {
    if !clang_available() {
        return;
    }
    expect(
        "memcpy's result, free(base), strlen(q)",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = d_copy(b, \"x\", 1); \
         d_free(b); d_len(q); }",
        "Unsafe",
        // A nonnull exact alias is settled on the released identity, so the
        // borrowed-liveness path rule refuses the same read as well.
        &["unknown_alias:q", "use_after_release"],
    );
}

#[test]
fn an_offset_base_is_the_same_object() {
    if !clang_available() {
        return;
    }
    expect(
        "strchr(p + 1, c) after free(base)",
        "void w(void) { char *b; char *p; char *q; b = d_alloc(8); p = d_chr(b, 58); \
         if (!p) { d_free(b); return; } q = d_chr(p + 1, 59); d_free(b); d_len(q); }",
        "Unsafe",
        &["use_after_release"],
    );
}

// -- Releasing a derived pointer is not releasing the base --------------------

#[test]
fn releasing_an_interior_pointer_is_a_violation_candidate_and_releases_nothing() {
    if !clang_available() {
        return;
    }
    // The base is still released exactly once afterwards: no double release,
    // because `d_free(p)` released nothing.
    expect(
        "free(strchr(...)) then free(base)",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         d_free(p); d_free(b); }",
        "Unsafe",
        &["invalid_release"],
    );
}

#[test]
fn releasing_an_interior_pointer_after_its_base_is_a_use_after_release() {
    if !clang_available() {
        return;
    }
    expect(
        "free(base) then free(strchr(...))",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         d_free(b); d_free(p); }",
        "Unsafe",
        &["use_after_release"],
    );
}

#[test]
fn releasing_an_exact_alias_is_refused_rather_than_credited() {
    if !clang_available() {
        return;
    }
    // `free(memcpy(b, ...))` frees `b` in C. The projection neither credits it
    // as the base's release nor accuses it: it refuses, and the base carries an
    // unresolved effect so no leak is claimed either.
    expect(
        "free(memcpy(b, ...))",
        "void w(void) { char *b; char *q; b = d_alloc(8); q = d_copy(b, \"x\", 1); \
         d_free(q); }",
        "Unknown",
        &["unknown_alias:q"],
    );
}

#[test]
fn consuming_or_returning_a_derived_pointer_is_refused() {
    if !clang_available() {
        return;
    }
    expect(
        "a derived pointer handed to a consuming callee",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         if (p) { d_consume(p); } d_free(b); }",
        "Unknown",
        &["unknown_alias:p"],
    );
    // Returning it would otherwise escape the base and certify a body that
    // hands its caller a pointer it cannot release.
    expect(
        "return strchr(b, c) through a name",
        "char *w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); return p; }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

// -- A derived null test narrows only the derived pointer ---------------------

#[test]
fn a_null_derived_pointer_does_not_make_its_base_disappear() {
    if !clang_available() {
        return;
    }
    // Were the null test read as a test of the base, the null branch would
    // drop the base's obligation and this leak would vanish.
    expect(
        "if (!strchr(b, c)) return; — leaks b on that path",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         if (!p) { return; } d_free(b); }",
        "Unsafe",
        &["leak"],
    );
    expect(
        "the same body releasing b on both paths",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         if (!p) { d_free(b); return; } d_free(b); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_binding_derived_on_only_some_paths_is_refused() {
    if !clang_available() {
        return;
    }
    expect(
        "p is b on one path and strchr(b, c) on the other",
        "void w(int n) { char *b; char *p; b = d_alloc(8); if (n) { p = d_chr(b, 58); } \
         else { p = b; } d_free(p); }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

// -- The base must be something the projection can look up -------------------

#[test]
fn a_bound_result_over_an_unmodeled_base_expression_is_refused() {
    if !clang_available() {
        return;
    }
    // A base mentioning a name a custody call already protects is refused by
    // the pre-existing borrowed-expression gate; this is the case that gate
    // does not reach, a borrowed formal inside a two-offset expression.
    expect(
        "a two-offset base over a borrowed formal",
        "void w(const char *s, int n) { char *q; q = d_chr(s + n + 1, 59); \
         if (q) { d_len(q); } }",
        "Unknown",
        &["unsupported:derived result of `d_chr` from an unmodeled base expression"],
    );
    // The same call with its result discarded binds nothing and is admitted.
    expect(
        "the same base, result discarded in a condition",
        "void w(const char *s, int n) { if (d_chr(s + n + 1, 59)) { return; } }",
        "Safe",
        &[],
    );
}
