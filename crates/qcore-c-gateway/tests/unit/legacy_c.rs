use super::*;

// Contract environments are loaded from `.qcore` fixtures through the
// total-or-reject v1 loader (W1 literal-site migration) rather than being
// constructed as Rust literals; the loader's own mapping tests stay in
// `compiler/tests/contract_loader.rs`.
const Q_BASE: (&str, &str) = (
    "q_base.qcore",
    include_str!("../fixtures/contracts/q_base.qcore"),
);
const Q_RICH_MODES: (&str, &str) = (
    "q_rich_modes.qcore",
    include_str!("../fixtures/contracts/q_rich_modes.qcore"),
);
const Q_NULLABLE: (&str, &str) = (
    "q_nullable.qcore",
    include_str!("../fixtures/contracts/q_nullable.qcore"),
);
const Q_REALLOC: (&str, &str) = (
    "q_realloc.qcore",
    include_str!("../fixtures/contracts/q_realloc.qcore"),
);
const Q_CONTAINER_STORE: (&str, &str) = (
    "q_container_store.qcore",
    include_str!("../fixtures/contracts/q_container_store.qcore"),
);
const Q_CONTAINER_REMOVE: (&str, &str) = (
    "q_container_remove.qcore",
    include_str!("../fixtures/contracts/q_container_remove.qcore"),
);
const Q_INTERNAL_MAKE: (&str, &str) = (
    "q_internal_make.qcore",
    include_str!("../fixtures/contracts/q_internal_make.qcore"),
);

fn fixture_contracts(modules: &[(&str, &str)]) -> ResourceContracts {
    let sources = modules
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

fn contracts() -> ResourceContracts {
    fixture_contracts(&[Q_BASE])
}

fn analyze(body: &str) -> LegacyFunctionSummary {
    analyze_legacy_c(body, "subject", &contracts()).unwrap()
}

fn has(summary: &LegacyFunctionSummary, expected: fn(&LegacyDiagnosticKind) -> bool) -> bool {
    summary
        .diagnostics
        .iter()
        .any(|diagnostic| expected(&diagnostic.kind))
}

#[test]
fn safe_annotated_program_reaches_a_safe_fixed_point() {
    let summary = analyze("void subject(void) {\n void *p = q_alloc();\n q_free(p);\n return;\n}");
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert!(summary.fixed_point_steps >= 3);
}

#[test]
fn finds_a_seeded_normal_exit_leak() {
    let summary = analyze("void subject(void) {\n void *p = q_alloc();\n return;\n}");
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn finds_a_seeded_double_free_through_an_alias() {
    let summary = analyze(
        "void subject(void) {\n void *p = q_alloc();\n void *q = p;\n q_free(p);\n q_free(q);\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::DoubleRelease { .. }
    )));
}

#[test]
fn finds_a_seeded_error_path_leak() {
    let summary = analyze(
        "int subject(int fail) {\n void *p = q_alloc();\n if (fail) { return -1; }\n q_free(p);\n return 0;\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn branch_cleanup_on_both_paths_is_safe() {
    let summary = analyze(
        "int subject(int fail) {\n void *p = q_alloc();\n if (fail) { q_free(p); return -1; } else { q_free(p); return 0; }\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn loop_analysis_terminates_at_a_conservative_fixed_point() {
    let summary = analyze(
        "void subject(int again) {\n void *p = q_alloc();\n while (again) { q_free(p); }\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::DoubleRelease { .. }
    )));
}

#[test]
fn ownership_transfer_discharges_the_local_obligation() {
    let summary =
        analyze("void subject(void) {\n void *p = q_alloc();\n q_transfer(p);\n return;\n}");
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn rich_contracts_distinguish_borrowed_owned_consumed_and_out_parameters() {
    let contracts = fixture_contracts(&[Q_BASE, Q_RICH_MODES]);

    let observed = analyze_legacy_c(
        "void subject(void) { void *p = q_alloc(); q_observe(p); q_mutate(p); q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(observed.status, AnalysisStatus::Safe);

    let consumed = analyze_legacy_c(
        "void subject(void) { void *p = q_alloc(); q_consume(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(consumed.status, AnalysisStatus::Safe);

    let out = analyze_legacy_c(
        "void subject(void) { void *p; q_make(&p); q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(out.status, AnalysisStatus::Safe);

    let missing_owned =
        analyze_legacy_c("void subject(void) { q_mutate(p); }", "subject", &contracts).unwrap();
    assert_eq!(missing_owned.status, AnalysisStatus::Unknown);
}

#[test]
fn nullable_owned_results_require_nonnull_path_refinement() {
    let contracts = fixture_contracts(&[Q_BASE, Q_NULLABLE]);

    let checked = analyze_legacy_c(
        "void subject(void) { void *p = q_maybe_alloc(); if (!p) return; q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(checked.status, AnalysisStatus::Safe);

    let nullable_release = analyze_legacy_c(
        "void subject(void) { void *p = q_maybe_alloc(); q_nullable_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(nullable_release.status, AnalysisStatus::Safe);

    let unchecked = analyze_legacy_c(
        "void subject(void) { void *p = q_maybe_alloc(); q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(unchecked.status, AnalysisStatus::Unknown);
    assert!(has(&unchecked, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("nullable resource")
    )));

    let reconverged = analyze_legacy_c(
        "void subject(void) { void *p = q_maybe_alloc(); if (!p) { } q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(reconverged.status, AnalysisStatus::Unknown);
}

#[test]
fn null_checks_accept_parentheses_and_preprocessed_void_pointer_zero() {
    let contracts = fixture_contracts(&[Q_BASE, Q_NULLABLE]);
    for condition in [
        "((p)) == ((void *)0)",
        "((void *) (0)) == (p)",
        "!((p) != ((void *)0))",
        "!((p))",
        "((p) == (0))",
    ] {
        let source = format!(
            "void subject(void) {{ void *p = q_maybe_alloc(); if ({condition}) return; q_free((p)); }}"
        );
        let summary = analyze_legacy_c(&source, "subject", &contracts).unwrap();
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{condition}: {:?}",
            summary.diagnostics
        );
    }
    // The same predicate must refine the other edge, not discharge both.
    let summary = analyze_legacy_c(
        "void subject(void) { void *p = q_maybe_alloc(); if ((p) != ((void *)0)) q_free(p); }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(summary.status, AnalysisStatus::Safe);
    for condition in [
        "(p) == (void *)1",
        "(p) + 1 == 0",
        "p == 0 || other()",
        "p == 0 && other()",
    ] {
        let source = format!(
            "void subject(void) {{ void *p = q_maybe_alloc(); if ({condition}) return; q_free(p); }}"
        );
        let summary = analyze_legacy_c(&source, "subject", &contracts).unwrap();
        assert_ne!(summary.status, AnalysisStatus::Safe, "{condition}");
    }
}

#[test]
fn braced_declarations_retain_following_cleanup_without_certifying_their_effects() {
    for declaration in [
        "int values[2][2] = {{1, 2}, {3, 4}};",
        "struct local { int value; } item = {.value = 1};",
        "struct pair pair = (struct pair){.value = opaque()};",
        "void *other = ({ void *nested = q_alloc(); nested; });",
    ] {
        let source = format!(
            "void subject(void) {{ void *p = q_alloc(); {declaration} q_free(p); q_free(p); }}"
        );
        let summary = analyze(&source);
        assert!(has(
            &summary,
            |kind| matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.contains("braced"))
        ));
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::UnknownAlias { .. }
            )),
            "unsupported effects must keep subsequent cleanup unresolved: {declaration}"
        );
    }
    let returned = analyze("struct pair subject(void) { return (struct pair){.value = 0}; }");
    assert_eq!(returned.status, AnalysisStatus::Unknown);
    for source in [
        "void subject(void) { int x[] = {1, 2; }",
        "void subject(void) { int x[2) = {1, 2}; }",
        "void subject(void) { int x = 1 } void next(void) { ; }",
    ] {
        assert!(
            analyze_legacy_c(source, "subject", &contracts()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn c_type_operators_remain_unknown_without_being_reported_as_missing_contracts() {
    for operator in ["sizeof", "_Alignof", "typeof", "__typeof__"] {
        let source = format!("void subject(void) {{ int x = {operator}(int); }}");
        let summary = analyze(&source);
        assert_eq!(summary.status, AnalysisStatus::Unknown);
        assert!(has(
            &summary,
            |kind| matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.contains("requires type/operand analysis"))
        ));
    }
    // sizeof may evaluate a VLA bound; treating every sizeof as neutral is unsound.
    let summary = analyze("void subject(void) { int x = sizeof(int[opaque()]); }");
    assert_eq!(summary.status, AnalysisStatus::Unknown);
}

#[test]
fn parenthesized_null_assignment_does_not_erase_an_owned_obligation() {
    let null_only = analyze("void subject(void) { void *p = ((void *)0); if ((p)) q_free(p); }");
    assert_eq!(null_only.status, AnalysisStatus::Safe);
    let lost = analyze("void subject(void) { void *p = q_alloc(); p = ((void *)0); }");
    assert_eq!(lost.status, AnalysisStatus::Unsafe);
    assert!(has(&lost, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
    let invalid = analyze("void subject(void) { void *p = ((void *)1); q_free(p); }");
    assert_ne!(invalid.status, AnalysisStatus::Safe);
}

#[test]
fn braced_control_expressions_do_not_erase_nested_control_flow() {
    for source in [
        "void subject(void) { if (({ goto done; 1; })) {} done: return; }",
        "void subject(void) { while (({ goto done; 1; })) {} done: return; }",
        "void subject(void) { switch (({ goto done; 1; })) { default: break; } done: return; }",
    ] {
        assert_eq!(analyze(source).status, AnalysisStatus::Unknown, "{source}");
    }
}

#[test]
fn realloc_preserves_old_custody_on_failure_and_replaces_it_on_success() {
    let contracts = fixture_contracts(&[Q_BASE, Q_REALLOC]);

    let checked = analyze_legacy_c(
        "void subject(void) {\n\
             void *p = q_alloc();\n\
             void *next = q_realloc(p, 8);\n\
             if (!next) { q_free(p); return; }\n\
             p = next; q_free(p);\n\
             }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(checked.status, AnalysisStatus::Safe);

    let loses_old = analyze_legacy_c(
        "void subject(void) {\n\
             void *p = q_alloc();\n\
             p = q_realloc(p, 8);\n\
             if (!p) return;\n\
             q_free(p);\n\
             }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(loses_old.status, AnalysisStatus::Unsafe);
    assert!(has(&loses_old, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn container_contract_cascades_child_custody_on_release_or_transfer() {
    let contracts = fixture_contracts(&[Q_BASE, Q_CONTAINER_STORE]);

    let released = analyze_legacy_c(
        "void subject(void) {\n\
             void *container = q_alloc(); void *item = q_alloc();\n\
             q_store(container, item); q_free(container);\n\
             }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(released.status, AnalysisStatus::Safe);

    let transferred = analyze_legacy_c(
        "void *subject(void) {\n\
             void *container = q_alloc(); void *item = q_alloc();\n\
             q_store(container, item); return container;\n\
             }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(transferred.status, AnalysisStatus::Safe);

    let conflicting = analyze_legacy_c(
        "void subject(void) {\n\
             void *container = q_alloc(); void *item = q_alloc();\n\
             q_store(container, item); q_free(item); q_free(container);\n\
             }",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(conflicting.status, AnalysisStatus::Unsafe);
    assert!(has(&conflicting, |kind| matches!(
        kind,
        LegacyDiagnosticKind::CustodyConflict { .. }
    )));
}

#[test]
fn container_removal_restores_direct_child_custody() {
    let contracts = fixture_contracts(&[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]);

    let removed = analyze_legacy_c(
        "void subject(void) {\n\
         void *container = q_alloc(); void *item = q_alloc();\n\
         q_store(container, item); q_remove(container, item);\n\
         q_free(container); q_free(item);\n}",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(removed.status, AnalysisStatus::Safe);

    let wrong_owner = analyze_legacy_c(
        "void subject(void) {\n\
         void *left = q_alloc(); void *right = q_alloc(); void *item = q_alloc();\n\
         q_store(left, item); q_remove(right, item);\n\
         q_free(left); q_free(right);\n}",
        "subject",
        &contracts,
    )
    .unwrap();
    assert_eq!(wrong_owner.status, AnalysisStatus::Unsafe);
    assert!(has(&wrong_owner, |kind| matches!(
        kind,
        LegacyDiagnosticKind::CustodyConflict { .. }
    )));
}

#[test]
fn returned_ownership_discharges_the_local_obligation() {
    let summary = analyze("void *subject(void) {\n void *p = q_alloc();\n return p;\n}");
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn uncontracted_calls_report_unknown_instead_of_safe() {
    let summary = analyze("void subject(void) {\n void *p = mystery();\n return;\n}");
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { .. }
    )));
}

#[test]
fn unresolved_release_alias_reports_unknown() {
    let summary = analyze("void subject(void) {\n q_free(p);\n}");
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { .. }
    )));
}

#[test]
fn calls_in_conditions_and_returns_cannot_be_silently_accepted() {
    let condition = analyze("int subject(void) { if (mystery()) { return 1; } return 0; }");
    assert_eq!(condition.status, AnalysisStatus::Unknown);

    let returned = analyze("void *subject(void) { return mystery(); }");
    assert_eq!(returned.status, AnalysisStatus::Unknown);
}

#[test]
fn indirect_pointer_expressions_cannot_be_silently_accepted() {
    let arithmetic =
        analyze("void subject(void) { void *p = q_alloc(); p = p + 1; q_free(p); return; }");
    assert_eq!(arithmetic.status, AnalysisStatus::Unknown);

    let field = analyze("void subject(void) { void *p = q_alloc(); q_free(p->field); return; }");
    assert_eq!(field.status, AnalysisStatus::Unsafe);
    assert!(has(&field, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description } if description.contains("indirect memory access")
    )));
}

#[test]
fn standalone_compound_blocks_are_imported() {
    let summary = analyze("void subject(void) { { void *p = q_alloc(); q_free(p); } return; }");
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn for_do_while_break_and_continue_are_imported_as_cfg_edges() {
    let for_loop = analyze(
        "void subject(int again) {\n\
         for (; again; ) { void *p = q_alloc(); q_free(p); continue; }\n\
         return;\n}",
    );
    assert_eq!(for_loop.status, AnalysisStatus::Safe, "{for_loop:#?}");

    let do_while = analyze(
        "void subject(int again) {\n\
         do { void *p = q_alloc(); q_free(p); if (!again) break; } while (again);\n\
         return;\n}",
    );
    assert_eq!(do_while.status, AnalysisStatus::Safe, "{do_while:#?}");
}

#[test]
fn cleanup_goto_and_labels_preserve_resource_paths() {
    let cleaned = analyze(
        "void subject(int fail) {\n\
         void *p = q_alloc(); if (fail) goto cleanup;\n\
         cleanup: q_free(p); return;\n}",
    );
    assert_eq!(cleaned.status, AnalysisStatus::Safe);

    let skipped = analyze(
        "void subject(void) {\n\
         void *p = q_alloc(); goto done; q_free(p); done: return;\n}",
    );
    assert_eq!(skipped.status, AnalysisStatus::Unsafe);
    assert!(has(&skipped, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn switch_cases_fallthrough_and_break_are_explicit() {
    let complete = analyze(
        "void subject(int choice) {\n\
         void *p = q_alloc();\n\
         switch (choice) {\n\
           case 0: q_free(p); break;\n\
           case 1: q_free(p); break;\n\
           default: q_free(p); break;\n\
         }\n\
         return;\n}",
    );
    assert_eq!(complete.status, AnalysisStatus::Safe);

    let unmatched = analyze(
        "void subject(int choice) {\n\
         void *p = q_alloc();\n\
         switch (choice) { case 0: q_free(p); break; }\n\
         return;\n}",
    );
    assert_eq!(unmatched.status, AnalysisStatus::Unsafe);
}

#[test]
fn malformed_and_missing_functions_are_parse_errors() {
    assert!(matches!(
        analyze_legacy_c("void other(void) {}", "subject", &contracts()),
        Err(LegacyCError::FunctionNotFound(_))
    ));
    assert!(matches!(
        analyze_legacy_c("void subject(void) { q_free(p);", "subject", &contracts()),
        Err(LegacyCError::MalformedFunction { .. })
    ));
}

#[test]
fn unrestricted_projection_retains_successful_siblings() {
    let source = "void bad(void *target) { goto *target; }\nvoid subject(void) { return; }";
    let analysis =
        analyze_legacy_c_translation_unit_selected_partial(source, "subject", &contracts(), None)
            .unwrap();
    assert_eq!(analysis.summary.status, AnalysisStatus::Unknown);
    assert!(analysis.summary.functions.contains_key("subject"));
    assert!(!analysis.summary.functions.contains_key("bad"));
    assert_eq!(analysis.projection_failures.len(), 1);
}

#[test]
fn source_scope_uses_a_definition_that_survives_preprocessing() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "#if 0\nvoid hidden(void) {}\n#endif\nvoid subject(void) { return; }";
    let selected = legacy_c_source_functions(source);
    assert_eq!(selected, vec!["hidden", "subject"]);
    let unit = C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&unit, "hidden", &contracts()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.entry, "subject");
            assert_eq!(certificate.checked_functions, vec!["subject"]);
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("compiled-out source entry blocked the unit: {other:?}"),
    }

    assert!(matches!(
        analyze_unrestricted_c11(&unit, "typo", &contracts()),
        Ok(UnrestrictedLegacyResult::Unknown { partial: None, .. })
            | Err(LegacyCError::ProductionParserUnavailable(_))
    ));
}

#[test]
fn unrestricted_gateway_accepts_data_only_translation_units() {
    if !crate::test_support::clang_available() {
        return;
    }
    let unit = C11TranslationUnit {
        source: "const int generated_table[] = { 1, 2, 3 };",
        ..C11TranslationUnit::default()
    };
    match validate_unrestricted_c11(&unit) {
        Ok(preprocessed) => assert!(preprocessed.contains("generated_table")),
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        Err(error) => panic!("data-only unit failed the C gateway: {error}"),
    }
}

#[test]
fn aggregate_resource_places_require_an_access_importer() {
    let summary = analyze(
        "struct Pair { void *left; void *right[2]; };\nvoid subject(void) {\n struct Pair p;\n p.left = q_alloc();\n p.right[0] = q_alloc();\n q_free(p.right[0]);\n q_free(p.left);\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
}

#[test]
fn translation_unit_summary_propagates_callee_defects() {
    let source = "void helper(void) { void *p = q_alloc(); return; }\nvoid subject(void) { helper(); return; }";
    let summary = analyze_legacy_c_translation_unit(source, "subject", &contracts()).unwrap();
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unknown);
    assert_eq!(summary.functions["helper"].status, AnalysisStatus::Unsafe);
}

#[test]
fn documented_alias_widening_can_only_lose_a_safe_result() {
    let mut source = String::from("void subject(int c) {\n");
    for index in 0..=8 {
        source.push_str(&format!(" void *p{index} = q_alloc();\n"));
    }
    source.push_str(" void *q = p0;\n");
    for index in 1..=8 {
        source.push_str(&format!(" if (c) q = p{index};\n"));
    }
    for index in 0..=8 {
        source.push_str(&format!(" q_free(p{index});\n"));
    }
    source.push_str("}\n");
    let summary = analyze(&source);
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("widened")
    )));
}

#[test]
fn clang_gateway_rejects_invalid_c_before_projection() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "void *q_alloc(void); void q_free(void *); void subject(void) { void *p = q_alloc(); q_free(p); }";
    match analyze_legacy_c11(source, "subject", &contracts()) {
        Ok(summary) => assert_eq!(summary.status, AnalysisStatus::Safe),
        Err(LegacyCError::ProductionParserUnavailable(_)) => return,
        Err(error) => panic!("unexpected production parser result: {error}"),
    }
    assert!(matches!(
        analyze_legacy_c11("void subject( {", "subject", &contracts()),
        Err(LegacyCError::ProductionParserRejected(_))
            | Err(LegacyCError::ProductionParserUnavailable(_))
    ));
}

#[test]
fn unrestricted_gateway_certifies_only_fully_checked_safe_summaries() {
    if !crate::test_support::clang_available() {
        return;
    }
    let unit = C11TranslationUnit {
        source: "#define RELEASE(x) q_free(x)\nvoid *q_alloc(void); void q_free(void *); void subject(void) { void *p = q_alloc(); RELEASE(p); }",
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&unit, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            check_legacy_safety_certificate(&certificate).unwrap();
            assert_eq!(certificate.checked_functions, vec!["subject"]);
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("unexpected unrestricted-C result: {other:?}"),
    }
}

#[test]
fn unrestricted_projection_gaps_remain_unknown() {
    if !crate::test_support::clang_available() {
        return;
    }
    let unit = C11TranslationUnit {
        source: "int mystery(void); int subject(void) { return mystery(); }",
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&unit, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Unknown { .. })
        | Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("projection gap must remain Unknown: {other:?}"),
    }
}

#[test]
fn unrestricted_gateway_requires_an_explicit_gnu_dialect() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "void subject(void) { typeof(1) value = 1; }";
    let strict = C11TranslationUnit {
        source,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&strict, "subject", &contracts()) {
        Err(LegacyCError::ProductionParserRejected(_)) => {}
        Err(LegacyCError::ProductionParserUnavailable(_)) => return,
        other => panic!("strict C11 unexpectedly admitted `typeof`: {other:?}"),
    }

    let gnu = C11TranslationUnit {
        source,
        dialect: CCompilerDialect::Gnu17,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&gnu, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Unknown { partial, .. }) => {
            assert!(partial.is_some());
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("GNU17 gateway did not reach conservative projection: {other:?}"),
    }
}

/// Deliberate pin update, unit **E01-F1**
/// (`docs/reports/remaining-e01-f1/README.md`).
///
/// This test used to assert that a definition produced by expanding a macro
/// *in the main source* was outside `MainSourceDefinitions` scope, and that
/// `subject` was therefore `Safe`. That was boundary **E01-B1** enshrined as a
/// feature: the expansion happens at a main-source line, so the definition is a
/// main-source definition, and excluding it bought a `Safe` verdict for the
/// rest of the unit. It is now enumerated, analysed by the unmodified analyzer,
/// and — being unmodeled — leaves the unit `Unknown`.
///
/// The property the test was really protecting is boundary **E01-B2**, header
/// scope, and that is kept: the third leg moves the same helper into a real
/// header, where the exclusion is genuine and `subject` is `Safe` again.
#[test]
fn main_source_scope_excludes_header_bodies_but_not_main_source_expansions() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "\
#define HEADER_BODY static void header_helper(void) { struct Local { int value; }; }\n\
HEADER_BODY\n\
void *q_alloc(void);\n\
void q_free(void *);\n\
void subject(void) { void *p = q_alloc(); q_free(p); }\n";
    let whole_unit = C11TranslationUnit {
        source,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&whole_unit, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Unknown { partial, .. }) => {
            let partial = partial.expect("whole-unit projection should be available");
            assert_eq!(
                partial.functions["header_helper"].status,
                AnalysisStatus::Unknown
            );
            assert!(partial.functions.contains_key("subject"));
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => return,
        other => panic!("expanded helper must prevent a whole-unit proof: {other:?}"),
    }

    let source_only = C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    // The raw token scan still misses it — that scan is the enumeration's
    // monotonicity base, not the audited denominator.
    assert_eq!(legacy_c_source_functions(source), vec!["subject"]);
    let inventory = enumerate_source_definitions(&source_only);
    assert_eq!(inventory.definitions(), ["header_helper", "subject"]);
    assert_eq!(inventory.absent_from_raw_scan(), ["header_helper"]);
    match analyze_unrestricted_c11(&source_only, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Unknown { partial, .. }) => {
            let partial = partial.expect("projection should be available");
            assert_eq!(
                partial.functions["header_helper"].status,
                AnalysisStatus::Unknown
            );
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("a main-source expansion must not be excluded: {other:?}"),
    }

    // Boundary E01-B2, unchanged: a definition spelled in a real header is out
    // of the audited scope, and the main source still proves.
    let directory = std::env::temp_dir().join(format!(
        "qcore-e01f1-scope-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("e01f1_scope.h"),
        "static void header_helper(void) { struct Local { int value; }; }\n",
    )
    .unwrap();
    let header_source = "\
#include \"e01f1_scope.h\"\n\
void *q_alloc(void);\n\
void q_free(void *);\n\
void subject(void) { void *p = q_alloc(); q_free(p); }\n";
    let with_header = C11TranslationUnit {
        source: header_source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&with_header, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.checked_functions, vec!["subject"]);
            check_legacy_safety_certificate(&certificate).unwrap();
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("source-only proof was blocked by a header body: {other:?}"),
    }
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn source_function_discovery_ignores_block_macros() {
    let source = "\
void subject(void) {\n\
    cfl_list_foreach(item, list) { use(item); }\n\
}\n";
    assert_eq!(legacy_c_source_functions(source), vec!["subject"]);
}

#[test]
fn source_function_discovery_enters_cxx_linkage_blocks() {
    let source = "extern \"C\" {\nvoid first(void) {}\nint second(void) { return 0; }\n}";
    assert_eq!(legacy_c_source_functions(source), vec!["first", "second"]);
}

#[test]
fn internal_definitions_never_receive_an_automatic_neutral_contract() {
    // LC01's rule survives W4: being defined in the translation unit buys a
    // function nothing. A call is modeled only through a declared contract or a
    // custody summary that validated, and each body below defeats validation
    // for a different reason — an unmodeled statement, an unresolved alias, a
    // path-dependent parameter disposition, and recursion carrying an effect.
    // Positive movement from validated summaries is pinned in
    // `compiler/tests/unit/legacy_summary.rs`.
    //
    // The last case was narrowed by card C03-NFR: bare `subject() ` recursion
    // performs no custody event, so the neutral-frame route now validates and
    // publishes the neutral contract for it. Recursion that carries an effect
    // is still refused, which is what this list is about.
    for source in [
        "void helper(void *p) { int a[2] = { 1, 2 }; q_free(p); } void subject(void) { void *p = q_alloc(); helper(p); q_free(p); }",
        "void helper(void) { q_free(missing); } void subject(void) { void *p = q_alloc(); helper(); q_free(p); }",
        "void helper(void *p, int c) { if (c) { q_free(p); } } void subject(void) { void *p = q_alloc(); helper(p, 1); }",
        "void subject(void) { void *p = q_alloc(); subject(); q_free(p); q_free(p); }",
    ] {
        let summary = analyze_legacy_c_translation_unit(source, "subject", &contracts()).unwrap();
        assert_ne!(summary.status, AnalysisStatus::Safe, "{source}");
        assert!(
            summary.functions["subject"]
                .cfg
                .nodes
                .iter()
                .any(|node| matches!(node.instruction, LegacyInstruction::Unknown { .. })),
            "{source}"
        );
    }
}

#[test]
fn lc01_explicit_internal_return_contract_is_preserved() {
    let contracts = fixture_contracts(&[Q_BASE, Q_INTERNAL_MAKE]);
    for (cleanup, expected) in [
        ("", AnalysisStatus::Unsafe),
        ("q_free(p);", AnalysisStatus::Safe),
    ] {
        let source = format!(
            "void *make(void) {{ void *p = q_alloc(); return p; }} void subject(void) {{ void *p = make(); {cleanup} return; }}"
        );
        let summary = analyze_legacy_c_translation_unit(&source, "subject", &contracts).unwrap();
        assert_eq!(summary.status, expected);
        assert!(summary.functions["subject"].cfg.nodes.iter().any(|node| matches!(
            &node.instruction, LegacyInstruction::ContractCall { function, contract, .. }
            if function == "make" && contract.result == ResourceReturnMode::Owned { nullable: false }
        )));
    }
}

#[test]
fn unmodeled_memory_reads_do_not_become_neutral_effects() {
    let mut accepted = Vec::new();
    for (label, setup, tail) in [
        (
            "return dereference",
            "int *p = q_alloc(); q_free(p);",
            "return *p;",
        ),
        (
            "discarded dereference",
            "int *p = q_alloc(); q_free(p);",
            "*p; return 0;",
        ),
        (
            "alias dereference",
            "int *p = q_alloc(); int *alias = p; q_free(p);",
            "return *alias;",
        ),
        (
            "return subscript",
            "int *p = q_alloc(); q_free(p);",
            "return p[0];",
        ),
        (
            "discarded subscript",
            "int *p = q_alloc(); q_free(p);",
            "p[0]; return 0;",
        ),
        (
            "return member",
            "struct Cell *p = q_alloc(); q_free(p);",
            "return p->field;",
        ),
        (
            "discarded member",
            "struct Cell *p = q_alloc(); q_free(p);",
            "p->field; return 0;",
        ),
        (
            "condition dereference",
            "int *p = q_alloc(); q_free(p);",
            "if (*p) { return 1; } return 0;",
        ),
        (
            "condition subscript",
            "int *p = q_alloc(); q_free(p);",
            "if (p[0] == 0) { return 1; } return 0;",
        ),
        (
            "loop member",
            "struct Cell *p = q_alloc(); q_free(p);",
            "while (p->field) { break; } return 0;",
        ),
        (
            "borrowed argument",
            "int *p = q_alloc(); q_free(p);",
            "q_observe(*p); return 0;",
        ),
        (
            "cast dereference",
            "int *p = q_alloc(); q_free(p);",
            "return (int)*p;",
        ),
    ] {
        let source =
            format!("struct Cell {{ int field; }}; int subject(void) {{ {setup} {tail} }}");
        let result = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_RICH_MODES]),
        )
        .unwrap();
        if result.status == AnalysisStatus::Safe {
            accepted.push(label);
        }
    }
    assert!(
        accepted.is_empty(),
        "unmodeled reads were Safe: {accepted:?}"
    );
}

#[test]
fn memory_access_boundary_preserves_identifiable_scalar_expressions() {
    for body in [
        "int subject(int n) { return n * 2 + 1; }",
        "int subject(int n) { n++; if (n > 0) { return n; } return 0; }",
        "int subject(void) { int n = 0; n; return 3 * (4 + 5); }",
    ] {
        assert_eq!(analyze(body).status, AnalysisStatus::Safe, "{body}");
    }
    // A multiplication followed by dereference cannot be confused with a
    // typedef pointer declarator. No symbol-type evidence is available here.
    assert_ne!(
        analyze("int subject(int n) { int *p = q_alloc(); q_free(p); n **p; return 0; }").status,
        AnalysisStatus::Safe
    );
}

#[test]
fn c11_memory_read_projection_and_certificate_recheck_fail_closed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let prefix = "void *q_alloc(void); void q_free(void *);";
    for tail in [
        "return *p;",
        "p[0]; return 0;",
        "if (*p) { return 1; } return 0;",
    ] {
        let source =
            format!("{prefix} int subject(void) {{ int *p = q_alloc(); q_free(p); {tail} }}");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "subject",
            &contracts(),
        )
        .unwrap();
        assert!(
            matches!(result, UnrestrictedLegacyResult::Unknown { .. }),
            "{tail}: {result:?}"
        );
    }
    let source =
        format!("{prefix} int subject(void) {{ int *p = q_alloc(); q_free(p); return 0; }}");
    let UnrestrictedLegacyResult::Safe(certificate) = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap() else {
        panic!("scalar control must certify")
    };
    for replacement in ["return *p;", "*p; return 0;", "p[0]; return 0;"] {
        let mut forged = certificate.clone();
        forged.preprocessed_source = forged.preprocessed_source.replace("return 0;", replacement);
        assert!(check_legacy_safety_certificate(&forged).is_err());
    }
}

// --------------------------------------------------------------------------
// C02-P-ALIAS: the may-alias pre-pass
// --------------------------------------------------------------------------

/// The pre-pass is deliberately separate from the custody work list. These
/// tests drive it directly over hand-built CFGs, so they measure the dataflow
/// and not the analyzer around it.
fn alias_cfg(nodes: Vec<LegacyCfgNode>) -> LegacyCfg {
    LegacyCfg {
        function: "w".to_owned(),
        entry: nodes.len() - 1,
        nodes,
    }
}

fn alias_exit() -> LegacyCfgNode {
    LegacyCfgNode {
        instruction: LegacyInstruction::NoOp,
        successors: Vec::new(),
        line: 0,
        is_exit: true,
    }
}

fn alias_node(instruction: LegacyInstruction, successors: Vec<LegacyNodeId>) -> LegacyCfgNode {
    LegacyCfgNode {
        instruction,
        successors,
        line: 1,
        is_exit: false,
    }
}

fn alias_release(source: &str) -> LegacyInstruction {
    LegacyInstruction::Release {
        source: source.to_owned(),
    }
}

#[test]
fn the_alias_pre_pass_carries_a_release_forward() {
    // node 2 (entry): release a -> node 1: no-op -> node 0: exit
    let cfg = alias_cfg(vec![
        alias_exit(),
        alias_node(LegacyInstruction::NoOp, vec![0]),
        alias_node(alias_release("a"), vec![1]),
    ]);
    let facts = alias_facts(&cfg);
    assert!(facts[2].released.is_empty(), "entry facts precede the node");
    assert!(facts[1].released.contains("a"));
    assert!(facts[0].released.contains("a"));
}

#[test]
fn the_alias_pre_pass_unions_releases_at_a_join() {
    // 4: branch -> {3: release a, 2: release b} -> 1 -> 0
    let cfg = alias_cfg(vec![
        alias_exit(),
        alias_node(LegacyInstruction::NoOp, vec![0]),
        alias_node(alias_release("b"), vec![1]),
        alias_node(alias_release("a"), vec![1]),
        alias_node(LegacyInstruction::NoOp, vec![3, 2]),
    ]);
    let facts = alias_facts(&cfg);
    assert!(facts[1].released.contains("a") && facts[1].released.contains("b"));
}

#[test]
fn the_alias_pre_pass_intersects_guards_at_a_join() {
    // 3: branch on a != b -> {2: no-op, 1: no-op} -> 0.
    // Only the true edge proves distinctness, so the join drops it.
    let cfg = alias_cfg(vec![
        alias_exit(),
        alias_node(LegacyInstruction::NoOp, vec![0]),
        alias_node(LegacyInstruction::NoOp, vec![0]),
        alias_node(
            LegacyInstruction::BranchOnDistinct {
                left: "a".to_owned(),
                right: "b".to_owned(),
                true_is_distinct: true,
            },
            vec![2, 1],
        ),
    ]);
    let facts = alias_facts(&cfg);
    assert!(
        facts[2].proved_distinct("a", "b"),
        "the distinct edge carries it"
    );
    assert!(
        !facts[1].proved_distinct("a", "b"),
        "the other edge does not"
    );
    assert!(!facts[0].proved_distinct("a", "b"), "the join intersects");
}

#[test]
fn the_alias_pre_pass_drops_a_guard_when_either_name_is_rebound() {
    for rebound in ["a", "b"] {
        let cfg = alias_cfg(vec![
            alias_exit(),
            alias_node(
                LegacyInstruction::Acquire {
                    destination: rebound.to_owned(),
                    site: 0,
                },
                vec![0],
            ),
            alias_node(LegacyInstruction::NoOp, vec![0]),
            alias_node(
                LegacyInstruction::BranchOnDistinct {
                    left: "a".to_owned(),
                    right: "b".to_owned(),
                    true_is_distinct: true,
                },
                vec![1, 2],
            ),
        ]);
        let facts = alias_facts(&cfg);
        assert!(facts[1].proved_distinct("a", "b"), "before the rebinding");
        assert!(
            !facts[0].proved_distinct("a", "b"),
            "rebinding `{rebound}` drops the guard"
        );
    }
}

#[test]
fn the_alias_pre_pass_keeps_a_release_across_a_rebinding_of_that_name() {
    let cfg = alias_cfg(vec![
        alias_exit(),
        alias_node(
            LegacyInstruction::Acquire {
                destination: "a".to_owned(),
                site: 0,
            },
            vec![0],
        ),
        alias_node(alias_release("a"), vec![1]),
    ]);
    let facts = alias_facts(&cfg);
    assert!(
        facts[0].released.contains("a"),
        "rebinding a released name does not resurrect the object"
    );
}

#[test]
fn only_a_released_position_of_a_contracted_call_records_a_release() {
    let modes = [
        (ResourceParameterMode::Released { nullable: false }, true),
        (ResourceParameterMode::Consumed { nullable: false }, false),
        (ResourceParameterMode::Borrowed, false),
        (ResourceParameterMode::Owned { nullable: false }, false),
    ];
    for (mode, expected) in modes {
        let instruction = LegacyInstruction::ContractCall {
            function: "f".to_owned(),
            destination: None,
            arguments: vec![Some("a".to_owned())],
            contract: ResourceFunctionContract::new([mode], ResourceReturnMode::None),
            produced_sites: Vec::new(),
            aggregate_reads: Vec::new(),
            aggregate_consumed: Vec::new(),
            aggregate_addresses: Vec::new(),
            alias_reads: Vec::new(),
        };
        assert_eq!(
            released_names(&instruction) == vec!["a"],
            expected,
            "{mode:?} must{} record a release",
            if expected { "" } else { " not" }
        );
    }
}

#[test]
fn an_indirect_release_place_records_no_release() {
    let instruction = LegacyInstruction::Release {
        source: "p->field".to_owned(),
    };
    assert!(
        released_names(&instruction).is_empty(),
        "a member place is not a name this rule relates"
    );
}
