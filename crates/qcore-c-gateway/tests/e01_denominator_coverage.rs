//! A whole-unit `Safe` requires every selected definition to have been read
//! (unit **E01-TAIL**, residual **E01-F1-DENOM**,
//! `docs/reports/remaining-e01-tail/README.md`).
//!
//! E01-F1 recorded the residual: a `Safe` unit could still carry a non-empty
//! `not_projected`. Measured, that happens because
//! `analyze_legacy_c_translation_unit_selected_partial` intersects the selected
//! definition set with the definitions its own body finder reports in the
//! preprocessed text and **silently drops the rest** — no diagnostic, no
//! projection failure, no row. The unit's report then reads "N source
//! definitions, M analysed, Safe" with `N > M` and nothing explaining the gap.
//!
//! Hard invariant 2 decides it: projection coverage is not safety coverage, and
//! a whole-unit `Safe` asserts the property over the declared scope. So the
//! `Safe` arm now refuses when the selected scope was not fully read, naming
//! the count and the first unread definition. `Unsafe` and `Unknown` are
//! untouched: an `Unsafe` unit still names a validated reachable violation in a
//! function that *was* analysed, and `Unknown` is already the fail-closed
//! answer.
//!
//! Two causes reach the gate, and it deliberately refuses both, because they
//! are indistinguishable from there and the report's own
//! `not_projected_meaning` already conflates them:
//!
//! * a definition the enumeration's raw half kept but the preprocessor removed
//!   — a `#if 0` body, which is not in this translation unit at all. Refusing
//!   this is the accepted strictness, recorded as **E01-TAIL-EXCUSE**;
//! * a definition present in the preprocessed text that the body finder does
//!   not read. K&R is the named instance; an *unnamed* one would be an analyzer
//!   blind spot, and this gate is what keeps such a spot from buying a `Safe`.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyCError, LegacyFunctionScope, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, enumerate_source_definitions,
};

const Q_BASE: (&str, &str) = ("q_base.qcore", include_str!("fixtures/contracts/q_base.qv"));

fn clang_available() -> bool {
    match std::env::var("QCORE_CLANG") {
        Ok(path) => !path.trim().is_empty(),
        Err(_) => std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success()),
    }
}

fn contracts() -> ResourceContracts {
    let sources = std::collections::BTreeMap::from([(Q_BASE.0.to_owned(), Q_BASE.1.to_owned())]);
    qcore_contract::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    }
}

// ---------------------------------------------------------------------------
// 1. The cause that exists on the pinned corpus: a compiled-out definition
// ---------------------------------------------------------------------------

/// `hidden` is in the enumerated denominator — the raw half keeps it, so the
/// denominator never shrinks (E01-F1 §1.4) — but the preprocessor removed it,
/// so nothing analysed it. The unit is `Unknown` with the gap named, and the
/// control without it is `Safe`.
///
/// Note what the enumeration itself says here: the claim is *established* and
/// nothing is *unanalyzable*, so E01-F1's own `supports_unit_safe` rule would
/// have admitted this `Safe`. The gap is invisible to that rule by
/// construction, which is why the residual survived it.
#[test]
fn a_compiled_out_definition_withholds_the_unit_safe_claim() {
    if !clang_available() {
        return;
    }
    let source = "#if 0\nvoid hidden(void) { }\n#endif\nvoid subject(void) { return; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["hidden", "subject"]);
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    assert!(
        inventory.supports_unit_safe(),
        "the E01-F1 rule alone admits this unit, which is why the gate is needed"
    );

    match analyze_unrestricted_c11(&unit(source), "subject", &contracts()) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Unknown {
            reason,
            partial: Some(summary),
            ..
        }) => {
            assert_eq!(
                summary.functions.keys().collect::<Vec<_>>(),
                vec!["subject"],
                "`subject` is still analysed and retained"
            );
            assert!(
                reason.contains("1 of 2 selected source definition(s)"),
                "{reason}"
            );
            assert!(reason.contains("`hidden`"), "{reason}");
        }
        other => panic!("Safe published over an unread definition: {other:?}"),
    }

    // The control: the same unit without the compiled-out definition.
    match analyze_unrestricted_c11(
        &unit("void subject(void) { return; }\n"),
        "subject",
        &contracts(),
    ) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.checked_functions, ["subject"]);
        }
        other => panic!("the control unit must still be Safe: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. The cause that matters most: a definition the body finder does not read
// ---------------------------------------------------------------------------

/// A K&R definition *is* in the translation unit; the enumeration names it
/// unanalyzable, so E01-F1's rule already withheld `Safe` at the projection
/// layer. What this pins is that the **gateway** refuses it too, on its own
/// ground and with its own reason — so a caller that does not run the audit's
/// projection layer, and a blind spot the enumeration does *not* recognise, are
/// both covered.
#[test]
fn a_definition_the_body_finder_does_not_read_withholds_safe() {
    if !clang_available() {
        return;
    }
    let source = "int knr(a, b) int a; int b; { return a + b; }\nvoid subject(void) { return; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["knr", "subject"]);
    assert_eq!(inventory.unanalyzable().len(), 1);

    match analyze_unrestricted_c11(&unit(source), "subject", &contracts()) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Unknown {
            reason,
            partial: Some(summary),
            ..
        }) => {
            assert_eq!(
                summary.functions.keys().collect::<Vec<_>>(),
                vec!["subject"]
            );
            assert!(reason.contains("`knr`"), "{reason}");
        }
        other => panic!("Safe published over an unread K&R body: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Precision: only the Safe arm is gated
// ---------------------------------------------------------------------------

/// A unit with both a validated reachable violation *and* an unread definition
/// stays `Unsafe`. An `Unsafe` verdict is a statement about a function that was
/// analysed; the unread definition does not retract it, and downgrading here
/// would discard a real finding without gaining any safety.
#[test]
fn an_unread_definition_does_not_retract_an_unsafe_finding() {
    if !clang_available() {
        return;
    }
    let source = concat!(
        "#if 0\nvoid hidden(void) { }\n#endif\n",
        "void *q_alloc(void);\nvoid q_free(void *p);\n",
        "void subject(void) { void *p = q_alloc(); q_free(p); q_free(p); }\n",
    );
    let inventory = enumerate_source_definitions(&unit(source));
    assert!(
        inventory.definitions().contains(&"hidden".to_owned()),
        "the compiled-out definition must be in the denominator: {:?}",
        inventory.definitions()
    );
    match analyze_unrestricted_c11(&unit(source), "subject", &contracts()) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
            assert!(summary.functions.contains_key("subject"));
        }
        other => panic!("an unread definition must not retract an Unsafe finding: {other:?}"),
    }
}

/// A unit that is already `Unknown` for its own reasons keeps that reason: the
/// gate is reached only from the `Safe` arm, so it cannot overwrite an
/// analysis-derived explanation with a coverage one.
#[test]
fn an_unknown_unit_keeps_its_own_reason() {
    if !clang_available() {
        return;
    }
    let source = concat!(
        "#if 0\nvoid hidden(void) { }\n#endif\n",
        "void opaque(int v);\n",
        "void subject(void) { opaque(1); }\n",
    );
    match analyze_unrestricted_c11(&unit(source), "subject", &contracts()) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Unknown { reason, .. }) => {
            assert!(
                !reason.contains("carry no analysed row"),
                "an analysis-derived Unknown must keep its own reason: {reason}"
            );
        }
        other => panic!("expected an analysis-derived Unknown: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Card CURRENT-RUN-DEFECTS: a unit whose every definition is compiled out
// ---------------------------------------------------------------------------

/// The `tinycthread.c` shape: every raw-source definition sits in a group the
/// unit's own configuration does not select (`#if !WITH_C11THREADS` with the
/// generated `config.h` defining it 1). The expansion attributes no definition
/// to the main source, the completeness claim stands, and the raw scan keeps
/// every name in the denominator (E01-F1 monotonicity). The inventory names
/// that cause, classified as incomplete obligation coverage, which the audit
/// publishes instead of the first raw name "not found"; it never makes the
/// unit data-only or `Safe` (Cycle 19, E01-TAIL-EXCUSE).
#[test]
fn a_unit_whose_every_definition_is_compiled_out_names_that_cause() {
    if !clang_available() {
        return;
    }
    let source = concat!(
        "#define WITH_C11THREADS 1\n",
        "#if !WITH_C11THREADS\n",
        "#if defined(_TTHREAD_WIN32_)\n",
        "int _cnd_timedwait_win32(int *c, int t) { return t; }\n",
        "#endif\n",
        "int mtx_lock(int *m) { return *m; }\n",
        "int thrd_yield(void) { return 0; }\n",
        "#endif\n",
        "static const int unused = 0;\n",
    );
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(
        inventory.definitions(),
        ["_cnd_timedwait_win32", "mtx_lock", "thrd_yield"],
        "every compiled-out definition stays in the denominator"
    );
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    assert!(inventory.build_excluded_only());
    let reason = inventory.build_excluded_reason().expect("a stated cause");
    assert!(reason.contains("3 raw-source definition(s)"), "{reason}");
    assert!(reason.contains("first `_cnd_timedwait_win32`"), "{reason}");
    assert!(reason.contains("build-excluded"), "{reason}");
    assert_eq!(
        qcore_schemas::reason::ReasonCategory::classify(&reason).unwrap(),
        qcore_schemas::reason::ReasonCategory::IncompleteObligationCoverage,
        "the stated cause keeps the coverage category the old reason had"
    );
    // The analysis itself is unchanged: its entry is not found, `Unknown`.
    match analyze_unrestricted_c11(&unit(source), "_cnd_timedwait_win32", &contracts()) {
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Ok(UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        }) => {
            assert_eq!(reason, "C function `_cnd_timedwait_win32` was not found");
            assert!(partial.is_none());
        }
        other => panic!("a compiled-out entry is an Unknown unit: {other:?}"),
    }
}

/// Precision: one live definition beside compiled-out ones, a unit with no
/// definition at all, and a unit whose live definition exists only after
/// expansion are not build-excluded-only.
#[test]
fn a_unit_with_a_live_definition_is_not_build_excluded_only() {
    if !clang_available() {
        return;
    }
    for source in [
        "#if 0\nvoid hidden(void) { }\n#endif\nvoid subject(void) { return; }\n",
        "static const int data_only = 1;\n",
        "#if 0\nvoid hidden(void) { }\n#endif\n#define DEFN(n) int n(void) { return 0; }\nDEFN(live)\n",
    ] {
        let inventory = enumerate_source_definitions(&unit(source));
        assert!(
            inventory.is_complete(),
            "{source}: {:?}",
            inventory.refusals()
        );
        assert!(!inventory.build_excluded_only(), "{source}");
        assert_eq!(inventory.build_excluded_reason(), None, "{source}");
    }
}
