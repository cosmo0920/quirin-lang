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

//! O04-B09B10 — member-address actuals and custody-taking bound reads
//! (`docs/reports/remaining-o04-b09b10/README.md`).
//!
//! Card O04-READ-BOUND left the constructor's success exit C6 carrying **nine**
//! refusals: eight `indirect memory access …` refusals and the identity residue
//! on `return ic;`. Seven of the eight hand a callee the **address of a
//! member** (`&ic->mp_pck`, `&ic->_head`, `&in->metrics_chunk_lock`) or read a
//! member of a formal at an **uncontracted** callee (`flb_hash_table_add`).
//! Those are O00's blockers **B09** and **B10**.
//!
//! This card supplies the missing typed premise for the member-address shape,
//! and the missing *frame* premise (F) for the custody-taking direction card
//! O04-READ-BOUND recorded open as `O04-READ-BOUND-CUSTODY`. It also repairs a
//! pre-existing false `Safe` its own subject made reachable: a call nested
//! inside another call's argument was dropped without a diagnostic.
//!
//! Three legs, all through the public gateway entry points only.
//!
//! **Held-out generic family.** An `m_*` API deliberately unrelated to ChunkIO,
//! to Fluent Bit and to the `g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08) and
//! `r_*` (O04-READ-BOUND) families.
//!
//! **Real source.** The same byte-verbatim upstream regions the four earlier
//! cards pinned, under their byte-unchanged premise sets plus five declared
//! callees.
//!
//! **One labelled control.** A single non-verbatim substitution that isolates
//! what the remaining refusal costs.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for a
//! synthetic declared custody projection under premises this card names; both
//! real functions keep function-level status `Unknown`, and no raw unit is
//! promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, SiteDisposition,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qr");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
/// The five declared callee premises this card adds; see the fixture header.
const B09B10_CALLEES: &str = include_str!("fixtures/contracts/o04_b09b10_callees.qr");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_b09b10_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn path_contracts() -> String {
    format!(
        "{}\n{ARMS_CALLEES}\n{B08_CALLEES}\n{B09B10_CALLEES}",
        PATH_CONTRACTS.replace("\r\n", "\n")
    )
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent O04-B09B10 test: {reason}");
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

fn refuses_base(summary: &LegacyFunctionSummary, base: &str) -> bool {
    summary.diagnostics.iter().any(|diagnostic| {
        matches!(&diagnostic.kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == base)
    })
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

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

/// * `m_box` carries the two member shapes the rules separate: `pad`/`count`
///   are declared with **no** `*`, so their addresses are admissible; `item`,
///   `spare`, `any` and `deep` are pointers, so theirs are not. `fixed` is an
///   array and `hot` is qualified, both refused.
const DECLARATIONS: &str = "\
struct m_item { int weight; }; \
struct m_pad { int a; int b; }; \
struct m_box { int flag; struct m_item *item; struct m_item *spare; \
               void *any; struct m_item **deep; struct m_pad pad; int count; \
               struct m_item *fixed[2]; volatile int hot; }; \
struct m_box *m_make(int n); \
struct m_item *m_take(int n); \
void m_drop(struct m_box *b); \
void m_free(struct m_item *i); \
void m_eat(struct m_item *i); \
void m_visit(void *p); \
int m_borrow(struct m_box *b); \
void m_reap(void *p);";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_b09b10_generic.qcore", GENERIC_CONTRACTS),
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

/// The prologue every generic body shares: acquire the aggregate, acquire the
/// child, link the child into a member.
const LINKED: &str = "\
    struct m_box *b; struct m_item *i; \
    b = m_make(1); if (!b) { return; } \
    i = m_take(2); if (!i) { m_drop(b); return; } \
    b->item = i;";

/// The headline of the member-address half: two member addresses at borrowed
/// positions, on a bound aggregate, and every exit settles.
#[test]
fn a_member_address_at_a_borrowed_position_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void m_g1(void) {{ {LINKED} m_visit(&b->pad); m_visit(&b->count); \
         m_free(i); m_drop(b); }}"
    );
    let certificate = certified(&body, "m_g1");
    let summary = certificate.summary.functions.get("m_g1").expect("present");
    assert!(
        summary.diagnostics.is_empty(),
        "no refusal remains: {:?}",
        summary.diagnostics
    );
    assert!(
        summary
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "every exit settles: {:?}",
        exit_shape(summary)
    );
}

/// Precision in the defect direction too: admitting the member addresses does
/// not hide the missing release.
#[test]
fn an_admitted_member_address_does_not_hide_a_leak() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void m_g2(void) {{ {LINKED} m_visit(&b->pad); m_visit(&b->count); m_drop(b); }}");
    let summary = summary_of(&body, "m_g2");
    assert_eq!(
        leaks(&summary).len(),
        1,
        "the unreleased child is reported: {:?}",
        summary.diagnostics
    );
}

/// A **pointer** member's address is a handle with which the callee could
/// overwrite the very link this projection maintains. It keeps its refusal, and
/// so does every other member shape the reader will not type.
#[test]
fn a_pointer_or_untyped_member_address_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    for member in ["item", "spare", "any", "deep", "fixed", "hot"] {
        let body =
            format!("void m_g3(void) {{ {LINKED} m_visit(&b->{member}); m_free(i); m_drop(b); }}");
        let summary = summary_of(&body, "m_g3");
        assert!(
            unsupported(&summary).contains(&INDIRECT),
            "`&b->{member}` must keep its refusal: {:?}",
            summary.diagnostics
        );
    }
}

/// Card C00's lesson for the member-address direction: the address of a member
/// of a **released** aggregate is never admitted.
#[test]
fn a_member_address_through_a_released_aggregate_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = format!("void m_g4(void) {{ {LINKED} m_free(i); m_drop(b); m_visit(&b->pad); }}");
    let summary = summary_of(&body, "m_g4");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        refuses_base(&summary, "b"),
        "the released aggregate is reported: {:?}",
        summary.diagnostics
    );
}

/// A second declared name for the same aggregate refuses the base outright, so
/// the member address keeps the existing lexical refusal.
#[test]
fn a_member_address_through_an_aliased_base_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void m_g5(void) {{ struct m_box *c; {LINKED} c = b; m_borrow(c); \
         m_visit(&b->pad); m_free(i); m_drop(b); }}"
    );
    let summary = summary_of(&body, "m_g5");
    assert!(
        unsupported(&summary).contains(&INDIRECT),
        "an aliased base keeps its refusal: {:?}",
        summary.diagnostics
    );
}

/// The C02-P half: a member address on a base this body can never bind to a
/// tracked identity needs no liveness check, because there is no identity.
#[test]
fn a_member_address_on_an_untracked_base_is_admitted() {
    if !clang_available() {
        return;
    }
    let body = "void m_g6(struct m_box *p) { m_visit(&p->pad); m_visit(&p->count); }";
    let certificate = certified(body, "m_g6");
    let summary = certificate.summary.functions.get("m_g6").expect("present");
    assert!(
        summary.diagnostics.is_empty(),
        "no refusal remains: {:?}",
        summary.diagnostics
    );
}

/// A member address may occupy only a `Borrowed` position. Releasing a
/// sub-object of a live aggregate is not a release this projection could
/// account for.
#[test]
fn a_custody_bearing_position_never_takes_a_member_address() {
    if !clang_available() {
        return;
    }
    let body = format!("void m_g7(void) {{ {LINKED} m_reap(&b->pad); m_free(i); m_drop(b); }}");
    let summary = summary_of(&body, "m_g7");
    assert!(
        unsupported(&summary).contains(&INDIRECT),
        "a releasing position keeps its refusal: {:?}",
        summary.diagnostics
    );
}

// --- the custody-taking read direction --------------------------------------

/// The headline of the custody half: a member read at a **releasing** or
/// **consuming** position transfers the linked child out of the aggregate, and
/// every exit settles.
#[test]
fn a_custody_taking_bound_read_transfers_the_linked_child() {
    if !clang_available() {
        return;
    }
    for sink in ["m_free", "m_eat"] {
        let body = format!("void m_c1(void) {{ {LINKED} {sink}(b->item); m_drop(b); }}");
        let certificate = certified(&body, "m_c1");
        let summary = certificate.summary.functions.get("m_c1").expect("present");
        assert!(
            summary.diagnostics.is_empty(),
            "`{sink}` leaves no refusal: {:?}",
            summary.diagnostics
        );
        assert!(
            summary
                .exits
                .iter()
                .all(|exit| exit.custody == ExitCustody::Discharged),
            "every exit settles: {:?}",
            exit_shape(summary)
        );
    }
}

/// Adversarial row 1 — **double free through the link**. The second read finds
/// no binding for the member and refuses; the identity is never released twice.
#[test]
fn a_double_release_through_the_same_link_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void m_c2(void) {{ {LINKED} m_free(b->item); m_free(b->item); m_drop(b); }}");
    let summary = summary_of(&body, "m_c2");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        refuses_base(&summary, "b"),
        "the second read is refused: {:?}",
        summary.diagnostics
    );
}

/// Adversarial row 2 — **free then read**. A borrowed read of the same member
/// after the transfer is refused too, by the same conjunct.
#[test]
fn a_borrowed_read_after_the_transfer_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void m_c3(void) {{ {LINKED} m_free(b->item); m_visit(b->item); m_drop(b); }}");
    let summary = summary_of(&body, "m_c3");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        refuses_base(&summary, "b"),
        "the stale read is refused: {:?}",
        summary.diagnostics
    );
}

/// Adversarial row 3 — **free through an exposed base**. Premise F: a callee
/// that receives the aggregate whole at a position leaving it live could have
/// rewritten the member, so the custody-taking read keeps card
/// O04-READ-BOUND's refusal. The *borrowed* read of the same member through the
/// same base stays admitted, because borrowing discharges nothing.
#[test]
fn a_custody_taking_read_through_an_exposed_base_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let exposed =
        format!("void m_c4(void) {{ {LINKED} m_borrow(b); m_free(b->item); m_drop(b); }}");
    let summary = summary_of(&exposed, "m_c4");
    assert!(
        unsupported(&summary).contains(&INDIRECT),
        "an exposed base keeps its refusal: {:?}",
        summary.diagnostics
    );
    let borrowed = format!(
        "void m_c5(void) {{ {LINKED} m_borrow(b); m_visit(b->item); m_free(i); m_drop(b); }}"
    );
    let control = summary_of(&borrowed, "m_c5");
    assert_eq!(
        control.status,
        AnalysisStatus::Safe,
        "the borrowed read through the same base stays admitted: {:?}",
        control.diagnostics
    );
}

/// Adversarial row 4 — **links surviving partial teardown**. Releasing the
/// aggregate does not cascade (premise P-B08-SHALLOW), so a child still linked
/// when its aggregate goes is reported as the residual it is, not discharged.
#[test]
fn a_link_surviving_the_aggregate_is_reported_not_discharged() {
    if !clang_available() {
        return;
    }
    let body = format!("void m_c6(void) {{ {LINKED} m_drop(b); }}");
    let summary = summary_of(&body, "m_c6");
    assert_eq!(
        leaks(&summary).len(),
        1,
        "the still-linked child is reported: {:?}",
        summary.diagnostics
    );
}

/// The transfer is not a licence to keep using the local the store came from:
/// releasing through the link and then through the original name is a double
/// release, and it is reported.
#[test]
fn releasing_through_the_link_and_the_original_name_is_reported() {
    if !clang_available() {
        return;
    }
    let body = format!("void m_c7(void) {{ {LINKED} m_free(b->item); m_free(i); m_drop(b); }}");
    let summary = summary_of(&body, "m_c7");
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "a second release of the same identity is never admitted: {:?}",
        summary.diagnostics
    );
}

/// A member this body never stored into carries no identity to transfer.
#[test]
fn a_custody_taking_read_of_an_unlinked_member_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = format!("void m_c8(void) {{ {LINKED} m_free(b->spare); m_free(i); m_drop(b); }}");
    let summary = summary_of(&body, "m_c8");
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(
        refuses_base(&summary, "b"),
        "the unlinked member is refused: {:?}",
        summary.diagnostics
    );
}

// --- the nested-call repair -------------------------------------------------

/// The measured false `Safe` this card repairs: `find_call` models the *first*
/// call of a statement, so a call nested inside an argument had its whole
/// custody effect dropped. `m_visit(m_take(1))` discards an owned result that
/// the top-level form `m_take(1);` already refuses.
#[test]
fn a_nested_call_inside_a_modeled_call_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let summary = summary_of("void m_n1(void) { m_visit(m_take(1)); }", "m_n1");
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "the nested acquisition is never dropped: {:?}",
        summary.diagnostics
    );
    // Re-pinned by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): the nested
    // call is now applied inside-out, and the owned result `m_visit` only
    // borrows is reported as a leak instead of refused. Either way the
    // acquisition is never dropped.
    assert!(
        unsupported(&summary).iter().any(
            |description| description.starts_with("nested call `m_take` inside a modeled call")
        ) || summary
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "the nested acquisition is refused or reported: {:?}",
        summary.diagnostics
    );
}

/// The C type operators keep their exemption: they evaluate no operand, and
/// they appear inside arguments the existing measurements already admit.
#[test]
fn a_type_operator_is_not_a_nested_call() {
    if !clang_available() {
        return;
    }
    let body = "void m_n2(void) { struct m_box *b; b = m_make((int) sizeof(struct m_box)); \
                if (!b) { return; } m_drop(b); }";
    let summary = summary_of(body, "m_n2");
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "sizeof is not a nested call: {:?}",
        summary.diagnostics
    );
}

// --- certificate rederivation ----------------------------------------------

/// The independent recheck repeats every premise from the certificate's own
/// content. Substituting the member declaration the admission rests on
/// withdraws the projection; the unmodified certificate still rechecks.
#[test]
fn a_certificate_resting_on_a_member_address_is_rederived_from_source() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void m_g1(void) {{ {LINKED} m_visit(&b->pad); m_visit(&b->count); \
         m_free(i); m_drop(b); }}"
    );
    let certificate = certified(&body, "m_g1");
    check_legacy_safety_certificate(&certificate).expect("the unmodified certificate rechecks");
    for (before, after) in [
        // The admitted member becomes a pointer: its address is a handle.
        ("struct m_pad pad;", "struct m_pad *pad;"),
        // The admitted member becomes qualified.
        ("int count;", "volatile int count;"),
        // The record becomes a union, so every member may overlap.
        ("struct m_box { int flag;", "union m_box { int flag;"),
    ] {
        let mut forged = certificate.clone();
        forged.preprocessed_source = forged.preprocessed_source.replace(before, after);
        assert_ne!(
            forged.preprocessed_source, certificate.preprocessed_source,
            "the substitution `{before}` applied"
        );
        assert!(
            check_legacy_safety_certificate(&forged).is_err(),
            "`{before}` -> `{after}` must be refused"
        );
    }
}

/// The certificate rests on the *declared position*, not the statement's shape:
/// turning the borrowing sink into a releasing one withdraws the projection.
#[test]
fn a_certificate_resting_on_a_member_address_rests_on_the_declared_mode() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void m_g1(void) {{ {LINKED} m_visit(&b->pad); m_visit(&b->count); \
         m_free(i); m_drop(b); }}"
    );
    let contracts = GENERIC_CONTRACTS.replace("\r\n", "\n");
    let weakened = contracts.replace(
        "extern fn m_visit(p: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);",
        "extern fn m_visit(p: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(p, nonnull);",
    );
    assert_ne!(weakened, contracts, "the substitution applied");
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "m_g1",
        &load("o04_b09b10_generic.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a releasing position withdraws the projection: {result:?}"
    );
}

// --------------------------------------------------------------------------
// Real source
// --------------------------------------------------------------------------

fn real_unit() -> String {
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n").replace("\r\n", "\n")
}

fn analyze_real(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_b09b10_path.qcore", &path_contracts()),
    )
    .unwrap_or_else(|error| panic!("O04-B09B10 fixture rejected by Clang: {error:?}"))
}

/// The card's headline on real source, in both directions at once: **all eight
/// `INDIRECT` refusals of card O04-READ-BOUND's success-exit residual are
/// gone**, and the success exit C6 is **still `Unknown`**.
///
/// **Composed-tree reconciliation (LOADER-ABI × NEIGHBOURS-CHAINED-TAIL): back
/// to zero.** Card LOADER-ABI's measurement alone left one `INDIRECT` here that
/// was **not** one of this card's eight — the chained tail actual
/// `storage->stream->name` of the `flb_error` arm, exposed when that
/// declaration was corrected from three named parameters to the faithful
/// `(const char *, ...)` reading. Clause **T5** of card
/// NEIGHBOURS-CHAINED-TAIL admits exactly that actual at the declared data tail
/// the corrected declaration gives it, so the count returns to zero and this
/// card's headline reads as it was measured.
///
/// **Retired by name:** the probe LOADER-ABI added here
/// (`storage->stream->name, name);` → `name, name);`, asserting the survivor
/// was that actual and nothing else) is removed. With the count back at zero
/// its baseline and its probe agree, so it could no longer discriminate; it
/// would have passed vacuously. The claim it made is now carried where it is
/// still live, by `o04_arms.rs::the_declaration_decides_the_same_chain_in_a_
/// variadic_tail`.
#[test]
fn the_success_exit_residual_narrows_to_one_statement() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    assert_eq!(
        unsupported(caller)
            .iter()
            .filter(|description| **description == INDIRECT)
            .count(),
        0,
        "no indirect-access refusal remains: {:?}",
        caller.diagnostics
    );
    // Exactly one statement is still refused, and it is the uncontracted
    // variadic logging call whose second argument is a nested call.
    // Exactly two `Unsupported` descriptions remain: that statement, and the
    // nullability complaint on `return ic;` that its poisoning of the path
    // produces. The control below measures that the second is a consequence of
    // the first and not an independent blocker.
    assert_eq!(
        unsupported(caller),
        vec![
            "call to uncontracted function `flb_trace`",
            "nullable resource `ic` used by a non-null contract",
        ],
        "the residual is one statement plus its consequence: {:?}",
        caller.diagnostics
    );
    // C6 stays Unknown, and the other five exits stay settled.
    //
    // Composed-tree reconciliation: back to five, for the reason recorded on
    // this test. The site-free-`Unknown` discrimination card LOADER-ABI added
    // below is kept, now asserting that no such exit exists.
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged)
            .count(),
        5,
        "five exits stay settled: {:?}",
        exit_shape(caller)
    );
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Unknown && exit.sites.is_empty())
            .count(),
        0,
        "no exit is unresolved while crossing nothing: {:?}",
        exit_shape(caller)
    );
    let success = caller
        .exits
        .iter()
        .find(|exit| exit.custody != ExitCustody::Discharged && !exit.sites.is_empty())
        .expect("the success exit");
    assert_eq!(success.custody, ExitCustody::Unknown);
    assert_eq!(success.sites.len(), 3);
    assert!(
        success
            .sites
            .iter()
            .all(|site| site.disposition == SiteDisposition::Mixed),
        "every identity stays Mixed, never a claim: {:?}",
        success.sites
    );
    assert!(
        leaks(caller).is_empty(),
        "no leak is alleged: {:?}",
        caller.diagnostics
    );
    assert_eq!(caller.status, AnalysisStatus::Unknown);
}

/// The two diagnostics on `return ic;` are **consequences** of that one
/// refusal, not independent blockers. Measured by a single, clearly labelled
/// non-verbatim substitution: replacing the refused statement with an already
/// contracted, argument-free call settles every exit, and C6 escapes all three
/// identities — the chunk and the routes mask as live linked children of the
/// escaping aggregate, and `ic` itself as the returned value.
///
/// This control is **not** a claim about the upstream source. On the
/// byte-verbatim bodies the assertion above governs, and C6 is `Unknown`.
#[test]
fn the_logging_control_settles_every_exit() {
    if !clang_available() {
        return;
    }
    let unit = real_unit();
    let trace = "        flb_trace(\"[input chunk] no matching route for input chunk '%s' with tag '%s'\",\n                  flb_input_chunk_get_name(ic), tag);";
    assert!(unit.contains(trace), "the control anchor is present");
    // Composed-tree reconciliation: back to ONE substitution. Card LOADER-ABI
    // added a second — withdrawing the chained tail actual — because under its
    // measurement alone that actual was refused. Clause T5 of card
    // NEIGHBOURS-CHAINED-TAIL admits it at the corrected declaration's data
    // tail, so the second substitution is unnecessary, and keeping it would
    // weaken this control: it would substitute away a statement the analyzer
    // now decides on the byte-verbatim source. The control is again what the
    // card's own doc-comment above claims it is.
    let controlled = unit.replace(trace, "        flb_errno();");
    // The *program* stays `Unknown`, because `cio_chunk_open` in the same unit
    // keeps its own refusals. The claim under test is per function.
    let result = analyze_real(&controlled);
    let summary = function(program(&result), "flb_input_chunk_create").clone();
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert!(
        summary.diagnostics.is_empty(),
        "no refusal remains: {:?}",
        summary.diagnostics
    );
    assert_eq!(summary.exits.len(), 6);
    assert!(
        summary
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "every exit settles: {:?}",
        exit_shape(&summary)
    );
    let success = summary
        .exits
        .iter()
        .max_by_key(|exit| exit.line)
        .expect("present");
    assert_eq!(success.sites.len(), 3);
    assert!(
        success
            .sites
            .iter()
            .all(|site| site.disposition == SiteDisposition::Escaped),
        "the success exit escapes all three identities: {:?}",
        success.sites
    );
}

/// The real subject's own declaration substitutions: the member-address
/// admission rests on the declared member shapes, and changing one restores the
/// refusal.
#[test]
fn substituting_the_addressed_member_declarations_restores_the_refusal() {
    if !clang_available() {
        return;
    }
    for (before, after) in [
        // `&ic->mp_pck` on a pointer member is a handle to a link.
        (
            "struct msgpack_packer mp_pck;",
            "struct msgpack_packer *mp_pck;",
        ),
        // The lock member of the untracked base becomes qualified.
        (
            "    pthread_mutex_t metrics_chunk_lock;",
            "    volatile pthread_mutex_t metrics_chunk_lock;",
        ),
    ] {
        let unit = real_unit();
        assert!(
            unit.contains(before),
            "the substitution anchor `{before}` is present"
        );
        let mut changed = unit.replace(before, after);
        // Keep the mutated fixture valid C on Clang versions that reject
        // incompatible pointer arguments. The borrowed position accepts the
        // address of either member shape; its declared type still determines
        // whether the gateway admits that address.
        let signature = "void msgpack_packer_init(struct msgpack_packer *pk,";
        assert!(changed.contains(signature), "the callee anchor is present");
        changed = changed.replace(signature, "void msgpack_packer_init(void *pk,");
        let result = analyze_real(&changed);
        let caller = function(program(&result), "flb_input_chunk_create");
        assert!(
            unsupported(caller).contains(&INDIRECT),
            "`{before}` -> `{after}` must restore the refusal: {:?}",
            caller.diagnostics
        );
    }
}

/// The constructor's boundary is unchanged: four settled exits, two `Unknown`
/// ones carrying two `Mixed` sites each, and all seven of its indirect
/// refusals — `free(ch->name)` among them — still in place.
///
/// Premise F is what refuses it, and the reason is now *checked* rather than a
/// blanket position rule: under this fixture's premise set `cio_error_reset(ch)`
/// and `cio_chunk_is_up(ch)` hand the chunk whole to a **contracted** callee at
/// a `Borrowed` position, so this projection can offer no frame condition for
/// `ch->name` between the store and the read. (In the pinned 185-unit corpus
/// neither of those callees is contracted, so F does not refuse `ch` there and
/// the *flow-sensitive* gate refuses the same statement instead — both
/// refusals, recorded in the card report §5.)
///
/// The `mk_list_add(&ch->_head, &st->chunks)` statements also keep their
/// refusal, because `mk_list_add` is uncontracted here and `st` is passed at
/// every position of the uncontracted `cio_file_open`/`cio_memfs_open` calls,
/// so it is not a C02-P untracked base either.
#[test]
fn the_constructor_boundary_is_unchanged() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let constructor = function(program(&result), "cio_chunk_open");
    assert_eq!(
        constructor
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged)
            .count(),
        4,
        "four guard exits stay settled: {:?}",
        exit_shape(constructor)
    );
    for exit in constructor
        .exits
        .iter()
        .filter(|exit| exit.custody != ExitCustody::Discharged)
    {
        assert_eq!(exit.sites.len(), 2, "{:?}", exit_shape(constructor));
        assert!(
            exit.sites
                .iter()
                .all(|site| site.disposition == SiteDisposition::Mixed)
        );
    }
    // **Deliberate pin update, card ALIAS-AGGREGATE.** This count was 7. The
    // one that moved is `ch->backend = ((void *) 0);` at L166: `backend` is
    // declared `void *` in `struct cio_chunk`, which is now the admitted
    // opaque member class, and the null pointer constant is a value that class
    // takes. It links nothing — `StoredValue::Null` binds the place and records
    // no field link — so no identity, exit or verdict moved with it.
    //
    // `ch->backend = backend;` at L182 is **still refused** and is asserted
    // below by name: `backend` is itself declared `void *`, so it is not a
    // declared record pointer and the opaque member's value premise refuses it.
    // That is the load-bearing half of this row.
    //
    // **Deliberate pin update, card W44-INDIRECT-CALL-ARGUMENT-ACCESS.** This
    // count was 6. Three `mk_list_add(&ch->…, &st->…)` statements (L167, L185,
    // L188) hand two sub-object addresses to `Borrowed` positions; the call
    // argument rule admits them under the dereference obligation of `ch` and
    // `st`. At L167 both resolve and the call is modeled; at L185 and L188 the
    // obligation refuses `ch` (`UnknownAlias`), asserted below. The rule was
    // measured off and on in one tree: nothing else in the vector moved.
    //
    // **Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule
    // 12k).** This count was 3. `mk_list_del(&ch->_head)` at L177 hands a
    // sub-object address to a callee this fixture leaves uncontracted; W44
    // reads that actual at a `Borrowed` position, so the callee is what
    // decides and the refusal now names it (asserted below). The statement
    // stays refused; no identity, exit or verdict moved with it.
    assert_eq!(
        unsupported(constructor)
            .iter()
            .filter(|description| **description == INDIRECT)
            .count(),
        2,
        "the constructor keeps two indirect refusals: {:?}",
        constructor.diagnostics
    );
    assert!(
        constructor.diagnostics.iter().any(|diagnostic| {
            diagnostic.line == 177
                && matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
                    if description == "call to uncontracted function `mk_list_del`")
        }),
        "{:?}",
        constructor.diagnostics
    );
    for line in [185, 188] {
        assert!(
            constructor.diagnostics.iter().any(|diagnostic| {
                diagnostic.line == line
                    && matches!(&diagnostic.kind,
                        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "ch")
            }),
            "L{line}: the base's obligation refuses `ch`: {:?}",
            constructor.diagnostics
        );
    }
    assert!(
        constructor.diagnostics.iter().any(|diagnostic| {
            diagnostic.line == 182
                && matches!(&diagnostic.kind,
                    LegacyDiagnosticKind::Unsupported { description } if description == INDIRECT)
        }),
        "`ch->backend = backend;` keeps its refusal: {:?}",
        constructor.diagnostics
    );
    assert!(
        leaks(constructor).is_empty(),
        "no leak is alleged against the constructor: {:?}",
        constructor.diagnostics
    );
}

/// Not an assertion: prints the measured per-exit ledger and the remaining
/// refusals of the real path under this card's premise set, so the archived
/// report can be regenerated. Run with
/// `cargo test -p qcore-c-gateway --test o04_b09b10 -- --ignored --nocapture
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
