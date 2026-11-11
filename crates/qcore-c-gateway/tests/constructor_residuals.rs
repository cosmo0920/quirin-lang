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

//! CONSTRUCTOR-RESIDUALS — the dereference of a neutral call's result
//! (`docs/reports/remaining-constructor-residuals/README.md`).
//!
//! Card ALIAS-AGGREGATE measured the four roots of production
//! `flb_input_chunk_create`'s residuals and left two as reasoned refusals. This
//! file is the held-out evidence for the first of them,
//! `ALIAS-AGGREGATE-ERRNO`: the statement
//!
//! ```c
//! flb_errno_print((*__errno_location ()), "<stdin>", 2556);   /* flb_errno() */
//! ```
//!
//! whose first argument is a **dereference of a call result**. Card
//! W4-EXPR-POS's neutral-expression families are proved under the sentence *an
//! admitted expression contains no pointer-valued operand*, and
//! `__errno_location ()` is pointer-valued by construction, so that sentence
//! cannot be stretched to cover this. The refined sentence is stated over what
//! the projection tracks rather than over C types:
//!
//! > An admitted dereference-of-a-call-result **binds, releases, escapes and
//! > addresses nothing this projection tracks.**
//!
//! Its four conjuncts are discharged by premises N6, N7, N9 and the new N10
//! respectively; see `crate::variadic_nested::dereference_wrapper` for the
//! argument. This file measures both directions of every one of them, through
//! the public gateway entry points only.
//!
//! Nothing here promotes a verdict. `Safe` in this family is `Safe` for a
//! synthetic declared custody projection under premises this card names.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/constructor_residuals_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

/// Deliberately unrelated to every library in this repository and to the `g_*`,
/// `h_*`, `k_*`, `r_*`, `m_*` and `v_*` families of the earlier cards.
///
/// `d_slot` is the admitted shape — a zero-argument function returning a
/// pointer to a **scalar**, exactly glibc's `int *__errno_location (void)`.
/// `d_handle` and `d_record` are its two refused neighbours: a dereference of
/// either yields a pointer, which is what premise N10 exists to refuse.
const DECLARATIONS: &str = "\
struct d_rec { int weight; }; \
typedef int d_int; \
struct d_rec *d_open(int n); \
void d_close(struct d_rec *h); \
void d_report(int code, const char *label); \
void d_consume(struct d_rec *h); \
int *d_slot(void); \
struct d_rec **d_handle(void); \
struct d_rec *d_record(void); \
int *d_owned_slot(void); \
int *d_slot_of(struct d_rec *h); \
int *d_slot_taking(struct d_rec *h); \
const char *d_label_of(struct d_rec *h); \
void d_note(int code, const char *label, ...); \
void d_note_tail(int code, const char *label, ...);";

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
            eprintln!("skipping Clang-dependent CONSTRUCTOR-RESIDUALS test: {reason}");
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

fn analyze_with(declarations: &str, body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{declarations} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("constructor_residuals_generic.qcore", GENERIC_CONTRACTS),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn summary_with(declarations: &str, body: &str, entry: &str) -> LegacyFunctionSummary {
    let result = analyze_with(declarations, body, entry);
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate).expect("certificate rechecks");
            certificate
                .summary
                .functions
                .get(entry)
                .expect("entry present")
                .clone()
        }
        _ => program(&result)
            .functions
            .get(entry)
            .unwrap_or_else(|| panic!("missing function `{entry}`"))
            .clone(),
    }
}

fn summary_of(body: &str, entry: &str) -> LegacyFunctionSummary {
    summary_with(DECLARATIONS, body, entry)
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

/// The production shape, in the generic family. A resource is acquired,
/// reported on with a dereferenced neutral accessor in the argument, and
/// released on every path.
const D_ROUND_TRIP: &str = "\
void d_run(int n) \
{ \
    struct d_rec *h; \
    h = d_open(n); \
    if (!h) { d_report((*d_slot ()), \"open\"); return; } \
    d_report((*d_slot ()), \"ok\"); \
    d_close(h); \
}";

#[test]
fn the_production_dereference_shape_is_admitted_and_accounts_for_its_resource() {
    if !clang_available() {
        return;
    }
    let summary = summary_of(D_ROUND_TRIP, "d_run");
    assert_eq!(unsupported(&summary), Vec::<&str>::new());
    assert_eq!(leaks(&summary), Vec::<usize>::new());
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn a_write_through_a_dereferenced_call_result_never_discharges_a_resource() {
    if !clang_available() {
        return;
    }
    // Not this card's rule, but reachable BECAUSE of this card's declaration,
    // so it is measured rather than assumed. `(*f ()) = X;` — the shape
    // `errno = EINVAL;` preprocesses to — is an assignment whose destination is
    // not a plain name, and the lexical indirect-access gate inspects only the
    // tokens right of `=`. So the statement was already reaching the call rule
    // before this card; contracting `__errno_location` is what lets it be
    // modeled instead of refused, and the question is which direction that
    // moves.
    //
    // MEASURED: it moves in the **fail-closed** direction. The store is
    // dropped, not applied: the destination is not a place the projection
    // names, so nothing is bound to it and — the load-bearing half — the stored
    // identity is NOT discharged. `h` stayed live and leaked at the exit.
    //
    // Card W44-LEAK-PIN-AUDIT: the store is refused and its value side is
    // `h`, so the refused-escape rule withdraws the accusation (`h` carries an
    // unresolved effect). It is still never a discharge: the function stays
    // non-`Safe`, now `Unknown` with the refusal as its reason.
    let published = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         (*d_handle ()) = h; }",
        "d_run",
    );
    assert_ne!(published.status, AnalysisStatus::Safe);
    assert_eq!(published.status, AnalysisStatus::Unknown);
    assert_eq!(leaks(&published).len(), 0);

    // The control: a scalar write through the same shape is admitted, and the
    // resource beside it is still accounted for on every path.
    let scalar = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         (*d_slot ()) = 5; d_close(h); }",
        "d_run",
    );
    assert_eq!(unsupported(&scalar), Vec::<&str>::new());
    assert_eq!(leaks(&scalar), Vec::<usize>::new());
    assert_eq!(scalar.status, AnalysisStatus::Safe);
}

#[test]
fn both_spellings_of_the_wrapper_answer_alike() {
    if !clang_available() {
        return;
    }
    // `errno` is spelled with the enclosing parentheses; a hand-written
    // dereference usually is not. Refusing either would make the rule depend on
    // parenthesisation rather than on the expression.
    for body in [
        "void d_run(int n) { d_report((*d_slot ()), \"a\"); (void) n; }",
        "void d_run(int n) { d_report(*d_slot (), \"a\"); (void) n; }",
    ] {
        let summary = summary_of(body, "d_run");
        assert_eq!(unsupported(&summary), Vec::<&str>::new(), "{body}");
        assert_eq!(summary.status, AnalysisStatus::Safe, "{body}");
    }
}

#[test]
fn the_admission_does_not_hide_the_resource_it_stands_beside() {
    if !clang_available() {
        return;
    }
    // The whole point of the exemption is that it exempts a *span*, not a
    // statement. A body that leaks still leaks, and the leak is reported at the
    // acquisition, with the dereference admitted around it.
    let summary = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); \
         if (!h) { return; } d_report((*d_slot ()), \"leak\"); }",
        "d_run",
    );
    assert_eq!(unsupported(&summary), Vec::<&str>::new());
    assert_eq!(leaks(&summary).len(), 1);
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
}

#[test]
fn a_dereferenced_pointer_result_is_refused_by_premise_n10() {
    if !clang_available() {
        return;
    }
    // `*d_handle ()` is a `struct d_rec *` and `*d_record ()` a `struct d_rec`:
    // neither is a scalar, so neither may reach a position of this projection
    // as a value. Both keep the existing lexical refusal.
    for (body, callee) in [
        (
            "void d_run(int n) { d_consume((*d_handle ())); (void) n; }",
            "d_handle",
        ),
        (
            "void d_run(int n) { d_report((int) (*d_record ()).weight, \"x\"); (void) n; }",
            "d_record",
        ),
    ] {
        let summary = summary_of(body, "d_run");
        assert!(
            unsupported(&summary).contains(&INDIRECT)
                || unsupported(&summary)
                    .iter()
                    .any(|text| text.contains(callee)),
            "{body}: {:?}",
            unsupported(&summary)
        );
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
    }
}

#[test]
fn premise_n8_decides_the_position_in_both_directions() {
    if !clang_available() {
        return;
    }
    // MEASURED BOUND, recorded rather than faked. A dereferenced actual is a
    // *scalar* by premise N10, and every declared custody mode requires a
    // pointer parameter (the loader refuses otherwise), so "a dereference at a
    // `Released` position" is not a well-typed C program and cannot be tested
    // here. The position premise is still live, and the position it decides
    // that a scalar can occupy is the one beyond the declared vector: with a
    // declared data tail it is admitted, without one it is refused. Both
    // reporters are declared at the same arity and differ only in the tail
    // clause, so nothing but N8 separates them.
    let admitted = summary_of(
        "void d_run(int n) { d_note_tail(1, \"x\", (*d_slot ())); (void) n; }",
        "d_run",
    );
    assert_eq!(unsupported(&admitted), Vec::<&str>::new());
    assert_eq!(admitted.status, AnalysisStatus::Safe);

    let refused = summary_of(
        "void d_run(int n) { d_note(1, \"x\", (*d_slot ())); (void) n; }",
        "d_run",
    );
    assert_ne!(refused.status, AnalysisStatus::Safe);
    assert!(!unsupported(&refused).is_empty());
}

#[test]
fn a_dereferenced_result_carrying_custody_is_refused_by_premise_n6() {
    if !clang_available() {
        return;
    }
    // `d_owned_slot` hands back an owned resource. Dereferencing it does not
    // make the discarded acquisition disappear: the nested call is not admitted
    // at all, so card O04-B09B10's refusal stands.
    let summary = summary_of(
        "void d_run(int n) { d_report((*d_owned_slot ()), \"x\"); (void) n; }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(!unsupported(&summary).is_empty());
}

#[test]
fn a_dereferenced_result_of_a_releasing_accessor_is_refused_by_premise_n7() {
    if !clang_available() {
        return;
    }
    // `d_slot_taking` releases its argument, so its declared effects are not
    // the identity on the abstract state and the composition the wrapper needs
    // is not well defined. The release must not vanish into the exemption.
    let summary = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_report((*d_slot_taking (h)), \"x\"); d_close(h); (void) n; }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(!unsupported(&summary).is_empty());
}

#[test]
fn an_inner_actual_the_projection_cannot_denote_is_refused_by_premise_n9() {
    if !clang_available() {
        return;
    }
    // The wrapper does not weaken N9: `&h` is an address `canonical_place`
    // refuses to denote, and the exemption would otherwise put it where nobody
    // models it.
    let summary = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_report((*d_slot_of ((struct d_rec *) &h)), \"x\"); d_close(h); (void) n; }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn a_borrowed_inner_actual_composes_and_stays_accounted() {
    if !clang_available() {
        return;
    }
    // The complementary direction of the previous test: an inner actual that
    // *is* a place, at a declared `Borrowed` position, composes — and the
    // resource it names is still released on every path.
    let summary = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_report((*d_slot_of (h)), \"x\"); d_close(h); }",
        "d_run",
    );
    assert_eq!(unsupported(&summary), Vec::<&str>::new());
    assert_eq!(leaks(&summary), Vec::<usize>::new());
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn sibling_neutral_nested_calls_compose_and_recheck() {
    if !clang_available() {
        return;
    }
    // B04: two sibling arguments may be evaluated in either order. Both inner
    // contracts are identity effects, so either order reaches the same state;
    // the outer call is applied only after both argument evaluations complete.
    let result = analyze_with(
        DECLARATIONS,
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_report((*d_slot()), d_label_of(h)); d_close(h); }",
        "d_run",
    );
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected sibling neutral calls to be Safe: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("sibling admission rechecks");
}

#[test]
fn every_sibling_keeps_its_borrow_liveness_obligation() {
    if !clang_available() {
        return;
    }
    // The second nested call is the load-bearing one. Dropping it from the
    // vector conversion would make this function erroneously Safe.
    let summary = summary_with(
        DECLARATIONS,
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_close(h); d_report((*d_slot()), d_label_of(h)); }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(summary.diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "h"
    )));
}

#[test]
fn every_sibling_keeps_its_may_alias_obligation() {
    if !clang_available() {
        return;
    }
    // `a` and `b` have the same declared record-pointer type. Releasing `a`
    // makes the second nested call's read of `b` uncertain unless a guard has
    // established that they differ. The first sibling must not hide that read.
    let source = "void d_run(struct d_rec *a, struct d_rec *b) { \
        d_close(a); d_report((*d_slot()), d_label_of(b)); }";
    let summary = summary_with(DECLARATIONS, source, "d_run");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        summary.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
        LegacyDiagnosticKind::UnknownAlias { variable } if variable.contains('b')
        )),
        "{:?}",
        summary.diagnostics
    );
}

#[test]
fn one_effectful_or_uncontracted_sibling_refuses_the_whole_outer_call() {
    if !clang_available() {
        return;
    }
    for (declarations, body) in [
        (
            DECLARATIONS.to_owned(),
            "void d_run(int n) { d_report((*d_slot()), (const char *)d_owned_slot()); (void) n; }",
        ),
        (
            DECLARATIONS.replace(
                "int *d_slot(void);",
                "int *d_slot(void); const char *d_unknown(void);",
            ),
            "void d_run(int n) { d_report((*d_slot()), d_unknown()); (void) n; }",
        ),
    ] {
        let summary = summary_with(&declarations, body, "d_run");
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
        assert!(!unsupported(&summary).is_empty(), "{body}");
    }
}

#[test]
fn a_released_identity_at_a_nested_borrowed_actual_is_refused() {
    if !clang_available() {
        return;
    }
    // A **pre-existing** fail-open card O04-TRACE left and this card repaired,
    // measured at the base tree before any rule of this card existed:
    //
    // ```c
    // d_close(h); d_label_of(h);              /* base: Unknown, UnknownAlias */
    // d_close(h); d_report(0, d_label_of(h)); /* base: Safe, no diagnostic   */
    // ```
    //
    // Card BORROW-LIVENESS's precondition read only the enclosing call's
    // actuals, so the same released identity at the same declared `Borrowed`
    // mode answered differently purely because it was nested. Both directions
    // are pinned here, with and without this card's dereference wrapper.
    for body in [
        // the control the base tree already refused
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_close(h); d_label_of(h); }",
        // the bare nested call: the repaired hole
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_close(h); d_report(0, d_label_of (h)); }",
        // the same, under this card's dereference wrapper
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_close(h); d_report((*d_slot_of (h)), \"x\"); }",
    ] {
        let summary = summary_with(DECLARATIONS, body, "d_run");
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
        assert!(
            summary.diagnostics.iter().any(|diagnostic| matches!(
                &diagnostic.kind,
                LegacyDiagnosticKind::UnknownAlias { variable } if variable == "h"
            )),
            "{body}: {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn every_other_wrapper_shape_keeps_the_existing_refusal() {
    if !clang_available() {
        return;
    }
    // The wrapper is exactly `* CALL` and `( * CALL )`. Two stars, an address,
    // a continuation, an arithmetic combination, a subscript and a cast are all
    // refused, and each keeps a refusal it already had.
    for body in [
        "void d_run(int n) { int **p; p = &n ? (int **) 0 : (int **) 0; \
         d_report(**(int **) d_slot (), \"x\"); (void) p; }",
        "void d_run(int n) { d_report(*d_slot () + 1, \"x\"); (void) n; }",
        "void d_run(int n) { d_report(((*d_slot ())), \"x\"); (void) n; }",
        "void d_run(int n) { d_report((int) (long) &*d_slot (), \"x\"); (void) n; }",
        "void d_run(int n) { d_report(d_slot ()[0], \"x\"); (void) n; }",
        "void d_run(int n) { d_report((int) *(int *) d_slot (), \"x\"); (void) n; }",
    ] {
        let summary = summary_of(body, "d_run");
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
        assert!(!unsupported(&summary).is_empty(), "{body}");
    }
}

#[test]
fn an_uncontracted_accessor_keeps_the_nested_call_refusal() {
    if !clang_available() {
        return;
    }
    // Premise N3 is unchanged: the wrapper admits no callee the environment
    // does not declare.
    let summary = summary_with(
        &DECLARATIONS.replace(
            "int *d_slot(void);",
            "int *d_slot(void); int *d_other(void);",
        ),
        "void d_run(int n) { d_report((*d_other ()), \"x\"); (void) n; }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(!unsupported(&summary).is_empty());
}

#[test]
fn a_declaration_substitution_withdraws_the_admission() {
    if !clang_available() {
        return;
    }
    // Premise N10 is read from **this unit's** declarations, so substituting
    // them moves the answer. Every substitution below is refused, and the
    // unsubstituted control is the `Safe` above.
    // The body writes the two casts that keep every substituted spelling
    // well-typed C, so the substitution is the ONLY thing that moves. The
    // unsubstituted control is asserted last, `Safe`, through the same body.
    const BODY: &str = "void d_run(int n) { d_note_tail(1, \"x\", (*d_slot ())); (void) n; }";
    for substitute in [
        // no longer a pointer to a scalar
        ("int *d_slot(void);", "int **d_slot(void);"),
        ("int *d_slot(void);", "volatile int *d_slot(void);"),
        ("int *d_slot(void);", "_Atomic int *d_slot(void);"),
        // an unreadable declarator: no fact is recorded, so the admission is
        // refused rather than assumed
        ("int *d_slot(void);", "int *(*d_slot)(void);"),
        // an array pointee: the reader commits to no scalar class for it
        ("int *d_slot(void);", "int (*d_slot(void))[4];"),
    ] {
        let declarations = DECLARATIONS.replace(substitute.0, substitute.1);
        assert_ne!(declarations, DECLARATIONS, "substitution {substitute:?}");
        let summary = summary_with(&declarations, BODY, "d_run");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "substitution {substitute:?} was admitted"
        );
        assert!(
            !unsupported(&summary).is_empty(),
            "substitution {substitute:?} lost its diagnostic"
        );
    }
    let control = summary_with(DECLARATIONS, BODY, "d_run");
    assert_eq!(unsupported(&control), Vec::<&str>::new());
    assert_eq!(control.status, AnalysisStatus::Safe);
}

#[test]
fn a_typedef_pointee_resolves_and_an_unresolved_one_refuses() {
    if !clang_available() {
        return;
    }
    // The pointee classification goes through the same typedef resolution every
    // other typed premise uses, in both directions.
    let resolved = DECLARATIONS.replace("int *d_slot(void);", "d_int *d_slot(void);");
    assert_ne!(resolved, DECLARATIONS);
    let summary = summary_with(
        &resolved,
        "void d_run(int n) { d_report((*d_slot ()), \"x\"); (void) n; }",
        "d_run",
    );
    assert_eq!(unsupported(&summary), Vec::<&str>::new());
    assert_eq!(summary.status, AnalysisStatus::Safe);

    // The other direction: a typedef whose own declarator this reader will not
    // read (a function-pointer typedef) leaves the pointee unresolved, so the
    // admission is refused rather than assumed.
    let unresolved = DECLARATIONS.replace(
        "typedef int d_int;",
        "typedef int (*d_fn)(void); typedef int d_int;",
    );
    let unresolved = unresolved.replace("int *d_slot(void);", "d_fn *d_slot(void);");
    let summary = summary_with(
        &unresolved,
        "void d_run(int n) { d_note_tail(1, \"x\", (*d_slot ())); (void) n; }",
        "d_run",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn a_call_that_is_not_a_declaration_does_not_retire_the_fact() {
    if !clang_available() {
        return;
    }
    // This card's reader has a precision guard — a declaration begins a
    // statement — so `return * d_slot ( ) ;` and `x = a * d_slot ( ) ;` do not
    // retire `d_slot`'s scalar-pointer fact. That is pinned directly over token
    // vectors in `typed_field_access`'s own unit tests, because end to end a
    // *different* reader answers first.
    //
    // MEASURED, and recorded rather than repaired
    // (`CONSTRUCTOR-RESIDUALS-ABI-EXPRESSION`): card LOADER-ABI's independent
    // ABI reader has no such guard, so in a unit that also contains
    // `return * d_slot ( ) ;` it reads a second, differing "declaration" of
    // `d_slot`, composes it as ambiguous, and **withdraws** the contract. The
    // admission then refuses at premise N3 — the fail-closed direction — and
    // the statement keeps its existing `indirect memory access`. Repairing that
    // reader would un-withdraw contracts corpus-wide and is out of this card's
    // scope; it is asserted here so it cannot change unobserved.
    let refused = summary_of(
        "int d_peek(void) { return *d_slot (); } \
         void d_run(int n) { d_report((*d_slot ()), \"x\"); (void) n; }",
        "d_run",
    );
    assert_eq!(unsupported(&refused), vec![INDIRECT]);
    assert_ne!(refused.status, AnalysisStatus::Safe);

    // The control: the same dereference of the same accessor in the same unit,
    // with the expression written so that neither reader can mistake it for a
    // declaration, is admitted.
    let admitted = summary_of(
        "int d_peek(void) { int *q; q = d_slot (); return *q; } \
         void d_run(int n) { d_report((*d_slot ()), \"x\"); (void) n; }",
        "d_run",
    );
    assert_eq!(unsupported(&admitted), Vec::<&str>::new());
    assert_eq!(admitted.status, AnalysisStatus::Safe);
}

#[test]
fn the_admission_is_rederived_from_the_certificate_alone() {
    if !clang_available() {
        return;
    }
    // Every premise is read from the certificate's own retained source, so the
    // recheck repeats the classification rather than trusting the derivation.
    // The `Safe` above already rechecks through `summary_of`; this asserts the
    // certificate exists and names the same function.
    match analyze_with(DECLARATIONS, D_ROUND_TRIP, "d_run") {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
            assert!(certificate.summary.functions.contains_key("d_run"));
        }
        other => panic!("expected Safe: {other:?}"),
    }
}

#[test]
fn the_bare_nested_call_family_is_unchanged() {
    if !clang_available() {
        return;
    }
    // Card O04-TRACE's own admission still answers as it did: a bare nested
    // call needs no wrapper and gains no exemption, and an unadmitted one keeps
    // its exact existing wording.
    let summary = summary_with(
        &DECLARATIONS.replace(
            "void d_report(int code, const char *label);",
            "void d_report(int code, const char *label); int d_len(void);",
        ),
        "void d_run(int n) { d_report(d_slot_of ((struct d_rec *) 0) ? 1 : 0, \"x\"); (void) n; }",
        "d_run",
    );
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the call
    // is the deciding operand of the actual's `?:`, decided by the `if`
    // routes before `d_report`, which C sequences after it; each edge's
    // `d_report(1, "x")` / `d_report(0, "x")` is the classifier's. BEFORE:
    // the nested-call family's refusal.
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
    // And the refusal wording card O04-B09B10 fixed is still reachable, at a
    // position premise N8 refuses.
    let summary = summary_of(
        "void d_run(int n) { struct d_rec *h; h = d_open(n); if (!h) { return; } \
         d_consume(d_slot () ? h : h); d_close(h); (void) n; }",
        "d_run",
    );
    // FLIPPED by CG06 (rule 10z): `d_consume(d_slot() ? h : h)` consumes `h`
    // on both edges, so the fixture's own `d_close(h)` is now reached as the
    // double release it is (BEFORE: premise N8's refusal hid it).
    assert_eq!(
        summary.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        summary.diagnostics
    );
    assert!(
        summary
            .diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        "{:?}",
        summary.diagnostics
    );
}

#[test]
fn scalar_accessor_conditions_keep_resources_and_replay() {
    if !clang_available() {
        return;
    }
    for condition in ["(*d_slot()) == 11", "*d_slot() != 11"] {
        let body = format!(
            "void d_run(int n) {{ struct d_rec *h = d_open(n); if (!h) return; if ({condition}) {{ d_report(1, \"x\"); }} d_close(h); }}"
        );
        let summary = summary_of(&body, "d_run");
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{condition}: {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn scalar_accessor_condition_boundaries_refuse_without_erasing_obligations() {
    if !clang_available() {
        return;
    }
    for (declarations, condition) in [
        (DECLARATIONS.to_owned(), "(*d_owned_slot()) == 11"),
        (
            DECLARATIONS.to_owned(),
            "(*d_slot_of((struct d_rec*)0)) == 11",
        ),
        (DECLARATIONS.to_owned(), "(*d_slot()) == n"),
        (DECLARATIONS.to_owned(), "(*d_slot()) += 11"),
        (DECLARATIONS.to_owned(), "(*d_slot()) == 11u"),
        (
            DECLARATIONS.replace("int *d_slot(void);", "int **d_slot(void);"),
            "(**d_slot()) == 11",
        ),
        (
            DECLARATIONS.replace("int *d_slot(void);", "volatile int *d_slot(void);"),
            "(*d_slot()) == 11",
        ),
        (
            DECLARATIONS.replace(
                "int *d_slot(void);",
                "int *d_slot(void) { static int n; return &n; }",
            ),
            "(*d_slot()) == 11",
        ),
    ] {
        let body = format!("void d_run(int n) {{ if ({condition}) {{ d_report(1, \"x\"); }} }}");
        let summary = summary_with(&declarations, &body, "d_run");
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{declarations}: {condition}: {:?}",
            summary.diagnostics
        );
    }
    for body in [
        "void d_run(int n) { struct d_rec *h = d_open(n); if (!h) return; if ((*d_slot()) == 11) {} }",
        "void d_run(int n) { struct d_rec *h = d_open(n); if (!h) return; d_close(h); if ((*d_slot()) == 11) { d_close(h); } }",
    ] {
        let row = summary_of(body, "d_run");
        assert_eq!(row.status, AnalysisStatus::Unsafe, "{row:?}");
        assert!(!row.diagnostics.is_empty());
    }
    let missing = load("without-slot.qv", &GENERIC_CONTRACTS.replace(
        "extern fn d_slot() -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_return_none();\n};", ""));
    let source = format!("{DECLARATIONS} void d_run(void) {{ if ((*d_slot()) == 11) {{}} }}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "d_run",
        &missing,
    )
    .unwrap();
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{result:?}"
    );
}

#[test]
fn scalar_accessor_condition_edges_and_stale_source_are_checked() {
    if !clang_available() {
        return;
    }
    for body in [
        "void d_run(int n) { if (n && (*d_slot()) == 11) {} }",
        "void d_run(int n) { if (n || (*d_slot()) != 11) {} }",
        "void d_run(int n) { while ((*d_slot()) == 11) { if (n) break; continue; } }",
        "void d_run(int n) { do { if (n) break; } while ((*d_slot()) != 11); }",
        "void d_run(int n) { for (; (*d_slot()) == 11;) { if (n) break; } }",
        "void d_run(int n) { switch ((*d_slot()) == 11) { case 1: break; default: break; } }",
    ] {
        let row = summary_of(body, "d_run");
        assert_eq!(row.status, AnalysisStatus::Safe, "{body}: {row:?}");
        let nodes = &row.cfg.nodes;
        assert_eq!(nodes.iter().filter(|node| matches!(&node.instruction, qcore_c_gateway::legacy_c::LegacyInstruction::ContractCall { function, .. } if function == "d_slot")).count(), 1, "{body}");
    }
    let result = analyze_with(
        DECLARATIONS,
        "void d_run(void) { if ((*d_slot()) == 11) {} }",
        "d_run",
    );
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("{result:?}")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let mut changed = certificate.clone();
    changed.preprocessed_source = changed
        .preprocessed_source
        .replace("int *d_slot(void)", "volatile int *d_slot(void)");
    assert_ne!(changed.preprocessed_source, certificate.preprocessed_source);
    assert!(check_legacy_safety_certificate(&changed).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn retained_msg_write_source_derives_and_replays() {
    if !clang_available() {
        return;
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../cycle19-resource-contracts.json");
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_manifest(&manifest).unwrap();
    let source = include_str!("fixtures/scalar-accessor-condition/msg_write.h");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            function_scope: qcore_c_gateway::legacy_c::LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        },
        "msg_write",
        &contracts,
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(
        certificate.summary.functions["msg_write"].status,
        AnalysisStatus::Safe
    );
    check_legacy_safety_certificate(&certificate).unwrap();
    println!(
        "retained production msg_write: {:?}",
        certificate.summary.functions["msg_write"].status
    );
}
