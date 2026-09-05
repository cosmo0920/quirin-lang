use super::*;

fn contracts() -> ResourceContracts {
    ResourceContracts::new([
        ("q_alloc", ResourceApiKind::Acquire),
        ("q_free", ResourceApiKind::Release),
        ("q_transfer", ResourceApiKind::Escape),
    ])
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
    let mut contracts = contracts();
    contracts.insert_contract(
        "q_observe",
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None),
    );
    contracts.insert_contract(
        "q_mutate",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Owned { nullable: false }],
            ResourceReturnMode::None,
        ),
    );
    contracts.insert_contract(
        "q_consume",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Consumed { nullable: false }],
            ResourceReturnMode::None,
        ),
    );
    contracts.insert_contract(
        "q_make",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: false }],
            ResourceReturnMode::None,
        ),
    );

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
    let mut contracts = contracts();
    contracts.insert_contract(
        "q_maybe_alloc",
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: true }),
    );
    contracts.insert_contract(
        "q_nullable_free",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: true }],
            ResourceReturnMode::None,
        ),
    );

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
fn realloc_preserves_old_custody_on_failure_and_replaces_it_on_success() {
    let mut contracts = contracts();
    contracts.insert_contract(
        "q_realloc",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::Reallocated {
                parameter: 0,
                nullable: true,
            },
        ),
    );

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
    let mut contracts = contracts();
    contracts.insert_contract(
        "q_store",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Stored {
                    owner_parameter: 0,
                    nullable: false,
                },
            ],
            ResourceReturnMode::None,
        ),
    );

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
    let mut contracts = contracts();
    contracts.insert_contract(
        "q_store",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Stored {
                    owner_parameter: 0,
                    nullable: false,
                },
            ],
            ResourceReturnMode::None,
        ),
    );
    contracts.insert_contract(
        "q_remove",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: false,
                },
            ],
            ResourceReturnMode::None,
        ),
    );

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
        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p->field"
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
    let source = "void bad(void) { struct Local { int value; }; }\nvoid subject(void) { return; }";
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
fn imports_struct_fields_and_constant_array_elements() {
    let summary = analyze(
        "struct Pair { void *left; void *right[2]; };\nvoid subject(void) {\n struct Pair p;\n p.left = q_alloc();\n p.right[0] = q_alloc();\n q_free(p.right[0]);\n q_free(p.left);\n}",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn translation_unit_summary_propagates_callee_defects() {
    let source = "void helper(void) { void *p = q_alloc(); return; }\nvoid subject(void) { helper(); return; }";
    let summary = analyze_legacy_c_translation_unit(source, "subject", &contracts()).unwrap();
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
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

#[test]
fn main_source_scope_excludes_macro_expanded_header_bodies() {
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
            assert!(!partial.functions.contains_key("header_helper"));
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
    match analyze_unrestricted_c11(&source_only, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.checked_functions, vec!["subject"]);
            check_legacy_safety_certificate(&certificate).unwrap();
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("source-only proof was blocked by expanded helper: {other:?}"),
    }
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
