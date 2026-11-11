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

use super::*;

// Contract environments are loaded from `.qcore` fixtures through the
// total-or-reject v1 loader (W1 literal-site migration) rather than being
// constructed as Rust literals; the loader's own mapping tests stay in
// `compiler/tests/contract_loader.rs`.
const Q_BASE: (&str, &str) = (
    "q_base.qcore",
    include_str!("../fixtures/contracts/q_base.qv"),
);
const Q_RICH_MODES: (&str, &str) = (
    "q_rich_modes.qcore",
    include_str!("../fixtures/contracts/q_rich_modes.qv"),
);
const Q_NULLABLE: (&str, &str) = (
    "q_nullable.qcore",
    include_str!("../fixtures/contracts/q_nullable.qv"),
);
const Q_REALLOC: (&str, &str) = (
    "q_realloc.qcore",
    include_str!("../fixtures/contracts/q_realloc.qv"),
);
const Q_CONTAINER_STORE: (&str, &str) = (
    "q_container_store.qcore",
    include_str!("../fixtures/contracts/q_container_store.qv"),
);
const Q_CONTAINER_REMOVE: (&str, &str) = (
    "q_container_remove.qcore",
    include_str!("../fixtures/contracts/q_container_remove.qv"),
);
const Q_INTERNAL_MAKE: (&str, &str) = (
    "q_internal_make.qcore",
    include_str!("../fixtures/contracts/q_internal_make.qv"),
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
        // Card AOM-BINDING-POISON re-pinned this assertion **deliberately**,
        // in the accusing direction. Before it, the refusal poisoned every
        // binding, so `p` was unresolvable at the first `q_free` and the
        // double release disappeared into one `UnknownAlias`. None of these
        // four declarations mentions `p`, and `p`'s address is never taken, so
        // none of them can write that slot: `p` now survives the refusal, the
        // first release discharges and the second is reported as what it is.
        // The refusal itself is untouched — the `braced` diagnostic above
        // still stands and the function is still not `Safe` — and the outcome
        // is strictly more informative, never weaker.
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::DoubleRelease { .. }
            )),
            "unsupported effects must never certify subsequent cleanup: {declaration} {:?}",
            summary.diagnostics
        );
        assert_ne!(summary.status, AnalysisStatus::Safe, "{declaration}");
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

/// Card **AOM-BINDING-POISON**: the write reader, pinned shape by shape.
///
/// Everything this reader does not positively recognise as a read must come
/// back as a write, so the list below is read in both directions.
#[test]
fn a_refused_statement_reads_its_write_positions_and_defaults_to_written() {
    let write = |text: &str| {
        let texts = text.split_whitespace().collect::<Vec<_>>();
        may_write_names(&texts).contains("n")
    };
    for written in [
        "n = x ;",
        "n = - 1 ;",
        "n + = 1 ;",
        "n - = 1 ;",
        "n & = m ;",
        "n < < = 1 ;",
        "n > > = 1 ;",
        "n + + ;",
        "+ + n ;",
        "- - n ;",
        "( n ) = 1 ;",
        "( ( n ) ) = 1 ;",
        "n . field = 1 ;",
        "x = f ( & n ) , n = 2 ;",
    ] {
        assert!(write(written), "must be read as a write: {written}");
    }
    for read in [
        "n [ i ] = 1 ;",
        "n -> field = 1 ;",
        "n -> a -> b = 1 ;",
        "x = n ;",
        "f ( n ) ;",
        "if ( n = = 0 ) x = 1 ;",
        "if ( n ! = 0 ) x = 1 ;",
        "if ( n < = 0 ) x = 1 ;",
        "if ( n > = - 1 ) x = 1 ;",
        "x = n < < 2 ;",
        "x = n & m ;",
        "x = a [ n ] ;",
        "x = ( n ) ;",
    ] {
        assert!(!write(read), "must be read as a read: {read}");
    }
}

/// The frame premise, pinned from the other side: a name a `&` reaches, a
/// `static` local and a file-scope object are never spared, whatever the
/// refused statement says.
#[test]
fn only_automatic_locals_whose_address_is_never_taken_are_spared() {
    let unit =
        "int global ; void subject ( void ) { int a ; int b ; static int c ; int d ; f ( & d ) ; }"
            .split_whitespace()
            .collect::<Vec<_>>();
    let body = &unit[unit.iter().position(|text| *text == "{").unwrap() + 1..unit.len() - 1];
    let formals = BTreeSet::from(["formal".to_owned()]);
    let protected = frame_protected_locals(&unit, body, &formals);
    assert!(protected.contains("a"), "{protected:?}");
    assert!(protected.contains("b"), "{protected:?}");
    assert!(protected.contains("formal"), "{protected:?}");
    assert!(!protected.contains("c"), "a static local outlives the call");
    assert!(!protected.contains("d"), "its address is taken");
    assert!(!protected.contains("global"), "a file-scope object");
}

/// The repair spares a binding an unrelated refusal cannot have written, and
/// keeps the full poison where any of the three premises fails.
#[test]
fn an_unrelated_refusal_no_longer_poisons_a_binding_it_cannot_write() {
    // `p` is automatic, never address-taken, and the refused store writes
    // through `q` rather than to `p`, so the release still discharges.
    let spared = analyze(
        "void subject(void) { void *p = q_alloc(); struct pair *q = opaque_pair(); q->value = 1; q_free(p); }",
    );
    assert!(
        !has(&spared, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
        )) && !has(&spared, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )),
        "{:?}",
        spared.diagnostics
    );
    // Still `Unknown`: the refusal itself is untouched.
    assert_eq!(spared.status, AnalysisStatus::Unknown);

    // Address taken anywhere in the body withdraws the sparing for that name,
    // so the same shape keeps the pre-repair unresolved release.
    let addressed = analyze(
        "void subject(void) { void *p = q_alloc(); void **slot = &p; struct pair *q = opaque_pair(); q->value = 1; q_free(p); consume(slot); }",
    );
    assert!(
        has(&addressed, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
        )),
        "{:?}",
        addressed.diagnostics
    );

    // A refused statement that *writes* the name poisons it exactly as before.
    let written =
        analyze("void subject(void) { void *p = q_alloc(); p = opaque_pair()->value; q_free(p); }");
    assert!(
        has(&written, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
        )),
        "{:?}",
        written.diagnostics
    );
}

#[test]
fn only_fixed_scalar_c_queries_leave_the_type_operand_refusal() {
    // Wave 23 deliberately replaces the sizeof/_Alignof scalar pins. GNU
    // type operators and the evaluated VLA case keep their original refusal.
    for operator in ["sizeof", "_Alignof"] {
        let summary = analyze(&format!(
            "void subject(void) {{ int x = {operator}(int); }}"
        ));
        assert_eq!(summary.status, AnalysisStatus::Safe);
    }
    for operator in ["typeof", "__typeof__"] {
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
    // A `do` without its `while` fails the statement parse. (Before card
    // CG21-CALL-EXITS-AND-BOUNDARIES this test used a computed `goto *`,
    // which rule 11o now refuses by name inside its own function.)
    let source = "void bad(void) { do { } }\nvoid subject(void) { return; }";
    let analysis = analyze_legacy_c_translation_unit_selected_partial(
        source,
        "subject",
        &contracts(),
        None,
        (CCompilerDialect::C11, None),
    )
    .unwrap();
    assert_eq!(analysis.summary.status, AnalysisStatus::Unknown);
    assert!(analysis.summary.functions.contains_key("subject"));
    assert!(!analysis.summary.functions.contains_key("bad"));
    assert_eq!(analysis.projection_failures.len(), 1);
}

/// A compiled-out source entry does not block the unit from being analysed:
/// the effective entry falls back to a definition that survives preprocessing,
/// and that definition is analysed and clean.
///
/// **Deliberate pin update, unit E01-TAIL, residual E01-F1-DENOM.** This test
/// previously also asserted the *unit* was `Safe`. It is now `Unknown`: the
/// selected scope carries `hidden`, which the preprocessor removed, so nothing
/// analysed it, and hard invariant 2 forbids publishing a whole-unit `Safe`
/// over a definition the analysis never read. The property this test was really
/// protecting — that a compiled-out *entry selection* does not blind the unit,
/// and that `subject` is still reached and checked — is kept below, now read
/// out of the retained partial summary; and the control immediately after
/// shows the same source without the compiled-out definition is still `Safe`,
/// so the refusal is attributable to the unprojected definition alone.
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
        Ok(UnrestrictedLegacyResult::Unknown {
            reason,
            partial: Some(summary),
            ..
        }) => {
            // The entry fallback still happened, and `subject` was analysed.
            assert_eq!(summary.entry, "subject");
            assert_eq!(summary.status, AnalysisStatus::Safe);
            assert_eq!(
                summary.functions.keys().collect::<Vec<_>>(),
                vec!["subject"]
            );
            assert!(
                reason.contains("carry no analysed row") && reason.contains("`hidden`"),
                "the withheld Safe must name its ground and the definition: {reason}"
            );
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("compiled-out source entry blocked the unit: {other:?}"),
    }

    // Control: the same source without the compiled-out definition. Every
    // selected definition now carries a row, so the unit is `Safe` — the
    // refusal above is the unprojected definition and nothing else.
    let clean = "void subject(void) { return; }";
    let clean_unit = C11TranslationUnit {
        source: clean,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&clean_unit, "subject", &contracts()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.checked_functions, vec!["subject"]);
        }
        Err(LegacyCError::ProductionParserUnavailable(_)) => {}
        other => panic!("the control unit must still be Safe: {other:?}"),
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
///
/// Deliberate pin update, card **SCALAR-STATEMENT-FORMS** (rule 41): the
/// helper's body was `struct Local { int value; };`, a declaration of no
/// object, which rule 41 now lowers to nothing. The helper only has to stay
/// unmodeled, so its body is now an object of an inline union, which every
/// route still refuses (rule 41's `inline-record-object`).
#[test]
fn main_source_scope_excludes_header_bodies_but_not_main_source_expansions() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "\
#define HEADER_BODY static void header_helper(void) { union { int value; } local; local.value = 1; }\n\
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
        "static void header_helper(void) { union { int value; } local; local.value = 1; }\n",
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
        // Deliberate pin update, card W43-INDEX-TRAVERSAL: the condition
        // `*p` is now a typed read whose base was released on the reaching
        // path, so it is a `use_after_release` candidate (provisional
        // `Unsafe`) rather than a bare refusal. Never `Safe` either way.
        // Deliberate pin update, card W45-RETURN-EXPRESSION-FAMILIES: `return
        // *p;` in a function returning `int` is that card's scalar-result read,
        // whose one dereference of the bound `p` keeps the same obligation.
        // W46 typed statement paths also resolve the indexed read.
        if tail.starts_with("if") || tail == "return *p;" || tail == "p[0]; return 0;" {
            let UnrestrictedLegacyResult::Unsafe(summary) = &result else {
                panic!("{tail}: {result:?}");
            };
            assert!(
                summary.functions["subject"]
                    .diagnostics
                    .iter()
                    .any(|diagnostic| matches!(
                        diagnostic.kind,
                        LegacyDiagnosticKind::UseAfterRelease { .. }
                    )),
                "{tail}: {result:?}"
            );
            continue;
        }
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
        // These hand-built CFGs name no static-storage destination, so every
        // CTWIN-PUBLISH / CTWIN-RESIDUALS fact is empty and the may-alias
        // pre-pass measures exactly what it measured before those cards.
        publications: PublicationFacts::default(),
        conditional_names: BTreeSet::new(),
        tie_names: BTreeSet::new(),
        traversal: Default::default(),
        cell_tests: Default::default(),
        cell_frames: Default::default(),
    }
}

fn alias_exit() -> LegacyCfgNode {
    LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction: LegacyInstruction::NoOp,
        successors: Vec::new(),
        line: 0,
        is_exit: true,
        loop_exit_edge: None,
    }
}

fn alias_node(instruction: LegacyInstruction, successors: Vec<LegacyNodeId>) -> LegacyCfgNode {
    LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction,
        successors,
        line: 1,
        is_exit: false,
        loop_exit_edge: None,
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
            nested: Vec::new(),
            container: None,
            retention: None,
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

#[test]
fn scalar_increments_never_leave_a_stale_zero_branch_fact() {
    for mutation in [
        "n++",
        "++n",
        "n--",
        "--n",
        "++(n)",
        "((n))--",
        "n = !0",
        "n += 1",
        "n = 1 + 2",
    ] {
        let summary = analyze(&format!(
            "void subject(void) {{ int n = 0; {mutation}; if (n) {{ void *p = q_alloc(); }} }}"
        ));
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            "{mutation}: {:?}",
            summary.diagnostics
        );
        let clean = analyze(&format!(
            "void subject(void) {{ int n = 0; {mutation}; void *p = q_alloc(); q_free(p); }}"
        ));
        assert_eq!(
            clean.status,
            AnalysisStatus::Safe,
            "{mutation}: {:?}",
            clean.diagnostics
        );
    }
}

#[test]
fn scalar_assignment_effects_survive_unrelated_condition_frames() {
    for mutation in ["n = !0", "n = ON", "n = s->value", "n += s->value"] {
        let source = format!(
            "enum flag {{ ON = 1 }}; struct scalar {{ int value; }}; void subject(struct scalar *s) {{ int n = 0; {mutation}; if (external()) {{ }} if (n) {{ void *p = q_alloc(); }} }}"
        );
        let summary = analyze(&source);
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            "{mutation}: {:?}",
            summary.diagnostics
        );
    }
    for expression in ["n = 1", "n += 1", "n >>= 1", "n = (m = 1)"] {
        let summary = analyze(&format!(
            "void subject(void) {{ int n = 0; int m = 0; if ({expression}) {{ }} if (n) {{ void *p = q_alloc(); }} }}"
        ));
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Unsupported { .. }
            )),
            "{expression}"
        );
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            "{expression}: {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn increment_side_effects_outside_the_unary_scalar_statement_remain_refused() {
    // Deliberate pin update, card CG02-ELABORATION-CORE (rule 10s): the
    // elaborator lowers these increments to a `ForgetScalar` of `n` sequenced
    // before the value's consumer, so no stale branch fact survives
    // (AOM-POISON-NOOP-INCREMENT's concern) and each is admitted.
    for body in [
        "int n = 0; int m = 0; m = n++;",
        "int n = 0; if (n++) {}",
        "int n = 0; while (n++) {}",
        "int n = 0; for (; n++; ) {}",
        "int n = 0; do {} while (n++);",
        "int n = 0; switch(n++) { default: break; }",
        "int n = 0; return n++;",
    ] {
        let summary = analyze(&format!("int subject(void) {{ {body} }}"));
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{body}: {:?}",
            summary.diagnostics
        );
    }
    // A call whose actual carries the increment keeps its refusal (CG03).
    let summary = analyze("int subject(void) { int n = 0; consume(n++); }");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { .. }
        )),
        "{:?}",
        summary.diagnostics
    );
    // Deliberate pin update, card W43-INDEX-TRAVERSAL. `p++` on a declared
    // `char *` is now a modeled pointer step. It refuses because it steps the
    // allocation's only name (`unknown_alias`), and releasing the stepped
    // pointer is an `invalid_release` candidate: `p` no longer holds the value
    // the allocator returned. Stronger than the refusal it replaces, and still
    // never `Safe`.
    let body = "char *p = q_alloc(); p++; q_free(p);";
    let summary = analyze(&format!("int subject(void) {{ {body} }}"));
    assert_eq!(summary.status, AnalysisStatus::Unsafe, "{body}");
    assert!(
        has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p"
        )) && has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::InvalidRelease { .. }
        )),
        "{body}: {:?}",
        summary.diagnostics
    );
}

#[test]
fn complete_refused_control_expressions_preserve_unwritten_automatic_bindings() {
    for expression in [
        "if (opaque()) {}",
        "while (opaque()) { break; }",
        "for (; opaque(); ) { break; }",
        "do {} while (opaque());",
        "switch (opaque()) { default: break; }",
        "switch (0) { case sizeof(struct Pair): break; }",
    ] {
        let summary = analyze(&format!(
            "struct Pair {{ int x; }}; void subject(void) {{ void *p = q_alloc(); {expression} q_free(p); q_free(p); }}"
        ));
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Unsupported { .. }
            )),
            "{expression}"
        );
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::DoubleRelease { .. }
            )),
            "{expression}: {:?}",
            summary.diagnostics
        );
        assert!(
            !has(
                &summary,
                |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p")
            ),
            "{expression}"
        );
    }
    let returned = analyze("int subject(void) { void *p = q_alloc(); return opaque(); }");
    assert!(has(&returned, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { .. }
    )));
    assert!(has(&returned, |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn refused_control_expressions_still_poison_written_or_reachable_slots() {
    for (setup, expression) in [
        ("", "if ((p = opaque())) {}"),
        ("", "if (++(p)) {}"),
        ("", "if (--((p))) {}"),
        ("", "if (((p))++) {}"),
        ("void **slot = &p;", "if (opaque(slot)) {}"),
    ] {
        let summary = analyze(&format!(
            "void subject(void) {{ char *p = q_alloc(); {setup} {expression} q_free(p); }}"
        ));
        assert!(
            has(
                &summary,
                |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p")
            ),
            "{expression}: {:?}",
            summary.diagnostics
        );
        assert_ne!(summary.status, AnalysisStatus::Safe);
    }
    for declaration in ["void *p;", "static void *p;"] {
        let source = if declaration.starts_with("static") {
            format!(
                "void subject(void) {{ {declaration} p = q_alloc(); if (opaque()) {{}} q_free(p); }}"
            )
        } else {
            format!(
                "{declaration} void subject(void) {{ p = q_alloc(); if (opaque()) {{}} q_free(p); }}"
            )
        };
        assert_ne!(analyze(&source).status, AnalysisStatus::Safe);
    }
    let malformed = analyze("void subject(void) { void *p = q_alloc(); goto missing; q_free(p); }");
    assert!(has(
        &malformed,
        |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p")
    ));
}

#[test]
fn joined_lookup_and_acquisition_release_only_the_conditional_known_site() {
    for release in ["q_free(p);", "void *r = p; q_free(r);"] {
        let source = format!(
            "extern void *external; void subject(void) {{ void *p = external; if (!p) {{ p = q_maybe_alloc(); if (!p) return; }} {release} }}"
        );
        let summary = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_NULLABLE]),
        )
        .unwrap();
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{:?}",
            summary.diagnostics
        );
        assert!(has(&summary, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
        )));
        assert!(
            summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Released),
            "{release}: {:?}",
            summary.exits
        );
        assert!(
            !summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Mixed),
            "{release}: {:?}",
            summary.exits
        );
    }
}

#[test]
fn an_overwritten_live_acquisition_is_not_a_conditional_lookup_acquisition() {
    for body in [
        "void *p = q_maybe_alloc(); if (!p) return; if (flag) p = external; if (!p) return; q_free(p);",
        "void *p = external; if (flag) { p = q_maybe_alloc(); if (!p) return; } q_free(p);",
        "void *p = external; if (!p) { p = q_maybe_alloc(); if (!p) return; } p = external; if (!p) return; q_free(p);",
        // Card JOINED-PROVENANCE-CYCLES moved the fourth body of this table —
        // the same shape with a trailing `while` that rebinds nothing — into
        // `a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release`.
        // It was pinned here by the wave-31 card that introduced the acyclic
        // pre-pass, to record that a cyclic CFG derived no facts at all; that
        // restriction is what this card lifts, so the pin is updated
        // deliberately rather than weakened. An overwrite still refuses, which
        // is what this test is named for and what the three bodies above check.
    ] {
        let source = format!("extern void *external; void subject(int flag) {{ {body} }}");
        let summary = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_NULLABLE]),
        )
        .unwrap();
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
        assert!(
            summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Mixed),
            "{body}: {:?}",
            summary.exits
        );
    }
}

#[test]
fn a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release() {
    // Card JOINED-PROVENANCE-CYCLES. The acquisition runs at most once — its
    // node lies on no cycle — and nothing on the back edge rebinds `p`, so `p`
    // denotes the site on every path on which the site exists. Before this card
    // the function declined for carrying a back edge anywhere at all.
    for body in [
        "void *p = external; if (!p) { p = q_maybe_alloc(); if (!p) return; } while (flag) { flag = 0; } q_free(p);",
        "while (flag) { flag = 0; } void *p = external; if (!p) { p = q_maybe_alloc(); if (!p) return; } q_free(p);",
    ] {
        let source = format!("extern void *external; void subject(int flag) {{ {body} }}");
        let summary = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_NULLABLE]),
        )
        .unwrap();
        // The lookup's identity refusal is retained: this settles a
        // disposition, it does not resolve an identity (constraint 10b).
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{body}: {:?}",
            summary.diagnostics
        );
        assert!(
            has(&summary, |kind| matches!(
                kind,
                LegacyDiagnosticKind::UnknownAlias { .. }
            )),
            "{body}"
        );
        assert!(
            summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Released),
            "{body}: {:?}",
            summary.exits
        );
        assert!(
            !summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Mixed),
            "{body}: {:?}",
            summary.exits
        );
    }
}

#[test]
fn an_acquisition_inside_a_cycle_derives_no_conditional_fact() {
    // The eligibility rule, and the arm that keeps the cyclic extension
    // fail-closed: a site created inside a loop stands for one abstract
    // resource per *site* and not per dynamic object, so a per-site identity
    // fact would not be a statement about one of them. Such a site declines.
    for body in [
        "void *p = external; while (flag) { if (!p) { p = q_maybe_alloc(); if (!p) return; } flag = 0; } q_free(p);",
        "void *p = external; while (flag) { p = q_maybe_alloc(); if (!p) return; flag = 0; } q_free(p);",
    ] {
        let source = format!("extern void *external; void subject(int flag) {{ {body} }}");
        let summary = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_NULLABLE]),
        )
        .unwrap();
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
        assert!(
            !summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Released),
            "{body}: {:?}",
            summary.exits
        );
    }
}

#[test]
fn unknown_identity_null_refinement_never_erases_a_resource() {
    for targets in [BTreeSet::new(), BTreeSet::from([1]), BTreeSet::from([1, 2])] {
        let mut state = AbstractState::default();
        state.resources.insert(1, LIVE);
        state.resources.insert(2, RELEASED);
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets,
                unknown: true,
                may_be_null: true,
                derivation: Derivation::Identity,
            },
        );
        let before = state.clone();
        assert!(matches!(
            refine_pointer_null(&mut state, "p", false),
            PointerRefinement::Unknown
        ));
        assert_eq!(state.resources, before.resources);
        assert!(state.pointers["p"].unknown);
        assert!(!state.pointers["p"].may_be_null);
        assert!(matches!(
            refine_pointer_null(&mut state, "p", true),
            PointerRefinement::Unknown
        ));
        assert_eq!(state.resources, before.resources);
        assert!(state.pointers["p"].targets.is_empty());
    }
}

/// Card VA-LOOP-AND-POINTER-ARG (rule 12c): the `va_arg` pointer binding is
/// the Lean `vaArgBinding := ⟨[], true, true⟩` of
/// `proof/QCore/VariadicConditions.lean` (no target, unknown, derived): never
/// accused at a release, and a null test narrows the binding alone.
#[test]
fn valoop_the_va_arg_pointer_binding_is_the_lean_vector() {
    let lean = include_str!("../../../../proof/QCore/VariadicConditions.lean");
    assert!(lean.contains("def vaArgBinding : Binding := ⟨[], true, true⟩"));
    let binding = derive_pointer(None, DerivedExtent::Exact, true, false);
    assert!(binding.targets.is_empty() && binding.unknown);
    assert!(matches!(binding.derivation, Derivation::Derived { .. }));
    for mode in [
        ResourceParameterMode::Released { nullable: true },
        ResourceParameterMode::Released { nullable: false },
    ] {
        assert!(!derived_release_is_accusable(mode, &binding));
    }
    let mut state = AbstractState::default();
    state.resources.insert(1, LIVE);
    state.pointers.insert("p".into(), binding);
    let before = state.resources.clone();
    assert!(matches!(
        refine_pointer_null(&mut state, "p", false),
        PointerRefinement::Refined
    ));
    assert!(matches!(
        refine_pointer_null(&mut state, "p", true),
        PointerRefinement::Refined
    ));
    assert_eq!(state.resources, before);
    assert!(state.pointers["p"].targets.is_empty());
}

#[test]
fn scalar_mutation_and_conditional_release_are_rederived_through_clang() {
    if !crate::test_support::clang_available() {
        return;
    }
    let safe_source = "void *q_alloc(void); void q_free(void *); void subject(void) { int n = 0; n++; if (n) { void *p = q_alloc(); q_free(p); } }";
    let safe_unit = C11TranslationUnit {
        source: safe_source,
        ..C11TranslationUnit::default()
    };
    let UnrestrictedLegacyResult::Safe(certificate) =
        analyze_unrestricted_c11(&safe_unit, "subject", &contracts()).unwrap()
    else {
        panic!("scalar cleanup must certify")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let leaky_source = safe_source.replace("q_alloc(); q_free(p);", "q_alloc();");
    let leaky_unit = C11TranslationUnit {
        source: &leaky_source,
        ..C11TranslationUnit::default()
    };
    assert!(!matches!(
        analyze_unrestricted_c11(&leaky_unit, "subject", &contracts()).unwrap(),
        UnrestrictedLegacyResult::Safe(_)
    ));
    let source = "extern void *external; void *q_maybe_alloc(void); void q_free(void *); void subject(void) { void *p = external; if (!p) { p = q_maybe_alloc(); if (!p) return; } q_free(p); }";
    let unit = C11TranslationUnit {
        source,
        ..C11TranslationUnit::default()
    };
    let UnrestrictedLegacyResult::Unknown {
        partial: Some(summary),
        ..
    } = analyze_unrestricted_c11(&unit, "subject", &fixture_contracts(&[Q_BASE, Q_NULLABLE]))
        .unwrap()
    else {
        panic!("lookup identity must remain Unknown")
    };
    assert!(
        summary
            .functions
            .get("subject")
            .unwrap()
            .exits
            .iter()
            .flat_map(|exit| &exit.sites)
            .any(|site| site.disposition == SiteDisposition::Released)
    );
}

#[test]
fn conditional_provenance_excludes_shadowed_and_address_taken_names() {
    for body in [
        "void *p = external; if (!p) { void *p = q_maybe_alloc(); if (!p) return; } q_free(p);",
        "void *p = external; void **slot = &p; if (!p) { p = q_maybe_alloc(); if (!p) return; } q_free(p);",
    ] {
        let source = format!("extern void *external; void subject(void) {{ {body} }}");
        let summary = analyze_legacy_c(
            &source,
            "subject",
            &fixture_contracts(&[Q_BASE, Q_NULLABLE]),
        )
        .unwrap();
        assert!(!summary.cfg.conditional_names.contains("p"));
        assert!(
            !summary
                .exits
                .iter()
                .flat_map(|exit| &exit.sites)
                .any(|site| site.disposition == SiteDisposition::Released),
            "{body}: {:?}",
            summary.exits
        );
        assert_ne!(summary.status, AnalysisStatus::Safe);
    }
}

/// Card **C-GATEWAY-RETURN-CALL-W4**: a declined return-call probe leaves the
/// parser's site counter exactly where it found it.
///
/// The admission asks `Parser::classify_simple` to classify a synthetic
/// `PLACE = <expression>` and then *inspects* the answer, so that one existing
/// lookup decides which contract is in force. `classify_simple`'s only mutation
/// of the parser is `next_site`, so a probe whose answer the rule declines
/// restores it — otherwise every `return` this card refuses would silently
/// renumber the sites of the acquisitions that follow it, and site numbering is
/// certificate identity.
///
/// The observable is that the sites a body uses are exactly `0..n`. The probe
/// here is a `Reallocated` result, which this card refuses, placed on an
/// **early** `return` so that an unrestored counter would leave site 0 unused
/// and shift both later acquisitions up. Mutating the restore away turns this
/// assertion red, which is what makes it evidence rather than decoration.
#[test]
fn a_declined_return_call_probe_restores_the_site_counter() {
    let source = "void *q_alloc(void); void q_free(void *); \
                  void *q_realloc(void *p, unsigned long size); \
                  void *subject(void *p, int c) { \
                      if (c) { return q_realloc(p, 8); } \
                      void *a = q_alloc(); q_free(a); return q_alloc(); }";
    let tokens = tokenize(source);
    let cfg = build_function_cfg(&tokens, "subject", &fixture_contracts(&[Q_BASE, Q_REALLOC]))
        .expect("the fixture projects");
    let mut sites = cfg
        .nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    sites.sort_unstable();
    sites.dedup();
    assert!(!sites.is_empty(), "the fixture must allocate sites at all");
    assert_eq!(
        sites,
        (0..sites.len()).collect::<Vec<_>>(),
        "a declined return-call probe must not consume a site: {sites:?}"
    );
}

/// Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: the destination reader names
/// a suffix only when it is the whole left-hand side or follows a declarator
/// prefix; every other left-hand side with a place-spelling suffix names a
/// sub-object and is recognised as one.
#[test]
fn a_destination_is_the_whole_left_hand_side_or_a_declared_object() {
    let named = |source: &str| {
        let texts = source.split_whitespace().collect::<Vec<_>>();
        (
            direct_assignment_destination(&texts),
            names_a_sub_object(&texts),
        )
    };
    for (source, expected) in [
        ("name", "name"),
        ("( name )", "name"),
        ("p - > f", "p->f"),
        ("s . a . b", "s.a.b"),
        ("a [ 0 ]", "a[0]"),
        ("* out", "*out"),
        ("char * name", "name"),
        ("struct x_rec * const r", "r"),
        ("char * a , * name", "name"),
        ("int a [ 3 ]", "a[3]"),
    ] {
        assert_eq!(
            named(source),
            (Some(expected.to_owned()), false),
            "{source}"
        );
    }
    for source in [
        "recs [ i ] . name",
        "( * pp ) - > name",
        "( ( T * ) p ) - > u16",
        "f ( x ) - > name",
        "( & s ) - > str",
        "a - > b [ i ] . c",
        "x_note ( p , i",
    ] {
        assert_eq!(named(source), (None, true), "{source}");
    }
    // Neither named nor a sub-object: no suffix spells a place at all.
    for source in ["a [ i ]", "* p - > f", "* * p"] {
        assert_eq!(named(source), (None, false), "{source}");
    }
}

/// Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: the protected-formal alias
/// closure is a candidate inventory in which a larger set only protects more,
/// so it keeps the pre-card suffix reader. `recs[i].q = p` still relates `q`
/// and `p`, and the formal `q` stays protected once `p` is released.
#[test]
fn the_protected_alias_closure_keeps_the_suffix_reader() {
    let source = "void subject(char *p, char *q, int i) { struct r { char *q; } recs[2]; \
                  recs[i].q = p; q_free(p); }";
    let tokens = tokenize(source);
    let (body_start, _) = find_function_body(&tokens, "subject").expect("the body");
    let parameters = function_parameters(&tokens, "subject").expect("the parameters");
    let protected =
        protected_resource_formals(&tokens, body_start, "subject", &parameters, &contracts());
    assert!(protected.contains("p"), "{protected:?}");
    assert!(protected.contains("q"), "{protected:?}");
}
