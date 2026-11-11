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

//! Card **W43-RETURN-EXPRESSION**: a whole-call `return` whose call the
//! statement classifier refuses is lowered to that refusal, end to end through
//! the real gateway.
//!
//! The card's census measured the refused return expressions of the pinned
//! corpus in the analyzer's own token view. The dominant family is one call
//! spanning the whole expression whose synthetic assignment `T = f(args);` the
//! ordinary classifier refuses — overwhelmingly an uncontracted callee. The
//! return-position refusal named none of those causes. The rule keeps the
//! classifier's refusal verbatim and a `NoOp` exit, so it **re-attributes and
//! never admits**. Every row below is evidence about one of three things:
//!
//! * the **re-attribution** — the return-position wording is replaced by the
//!   classifier's own, and the lowered body agrees with the hand-written
//!   two-statement form on status, certification and every diagnostic;
//! * the **fail-closed** half — nothing lowered becomes `Safe`, is certified,
//!   escapes or discharges an obligation, or suppresses a leak on another path;
//! * the **boundary** — every other return shape keeps the return-position
//!   refusal verbatim (negative controls), each paired with a statement-position
//!   spelling of the same C so the refusal is shown to be about the return
//!   shape rather than the fixture.
//!
//! `Unknown` here is `Unknown` for a synthetic declared custody projection
//! under the premises this file names; nothing here promotes a corpus verdict.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The return-position refusal this card replaces for exactly one family.
const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";

/// The reserved synthetic destination the desugaring binds.
const RESERVED: &str = "__qcore_returned_value";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
void *q_realloc(void *p, __SIZE_TYPE__ size); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p); \
void *r_wrap(void *p); \
int z_unknown(int n); void *z_unknown_ptr(void); int z_take(void *p); \
struct w_pair { int a; int b; };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "q_realloc.qcore".to_owned(),
                include_str!("fixtures/contracts/q_realloc.qv").to_owned(),
            ),
            (
                "r_return_call.qcore".to_owned(),
                include_str!("fixtures/contracts/r_return_call.qv").to_owned(),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    certified: bool,
}

impl Run {
    fn unsupported(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    fn refuses_return(&self) -> bool {
        self.unsupported().iter().any(|text| text == RETURN)
    }

    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn names_reserved_place(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|kind| format!("{kind:?}").contains(RESERVED))
    }

    /// Status, certification and the sorted diagnostic kinds: what a consumer
    /// reads, independent of order.
    fn shape(&self) -> (AnalysisStatus, bool, Vec<String>) {
        let mut rendered = self
            .diagnostics
            .iter()
            .map(|kind| format!("{kind:?}"))
            .collect::<Vec<_>>();
        rendered.sort();
        (self.status, self.certified, rendered)
    }
}

fn run(body: &str) -> Run {
    let source = format!("{DECLARATIONS}\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let (summary, certified) = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe certificate must recheck independently");
            (certificate.summary.clone(), true)
        }
        UnrestrictedLegacyResult::Unsafe(summary) => (summary, false),
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            partial.expect("these fixtures always project partially"),
            false,
        ),
    };
    let function = summary
        .functions
        .get("subject")
        .expect("the entry is projected");
    Run {
        status: function.status,
        diagnostics: function
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
        certified,
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent refused-call lowering test: {reason}");
            false
        }
    }
}

/// The card's central property as a harness: the lowered `return f(args);`
/// agrees with the hand-written `T = f(args); return T;` on status,
/// certification and every diagnostic, and it no longer carries the
/// return-position wording. The harness refuses to pass vacuously: the
/// lowered body must name `expected` among its refusals and must be neither
/// `Safe` nor certified.
fn lowering_agrees(returned: &str, desugared: &str, expected: &str) -> Run {
    let lowered = run(returned);
    assert!(
        !lowered.refuses_return(),
        "the lowering did not fire, so this row measures nothing: {lowered:?}"
    );
    assert!(
        lowered.unsupported().iter().any(|text| text == expected),
        "the classifier's own refusal `{expected}` must be published: {lowered:?}"
    );
    assert_ne!(lowered.status, AnalysisStatus::Safe, "{lowered:?}");
    assert!(!lowered.certified, "a lowered refusal is never certified");
    assert!(
        !lowered.names_reserved_place(),
        "no published reason may name the synthetic place: {lowered:?}"
    );
    let reference = run(desugared);
    assert_eq!(
        lowered.shape(),
        reference.shape(),
        "the lowering must be the desugaring"
    );
    lowered
}

// ------------------------------------------------------------------------
// The re-attribution
// ------------------------------------------------------------------------

#[test]
fn an_uncontracted_whole_call_reports_the_call_not_the_return() {
    if !clang_available() {
        return;
    }
    lowering_agrees(
        "int subject(int n) { return z_unknown(n); }",
        "int subject(int n) { int t; t = z_unknown(n); return t; }",
        "call to uncontracted function `z_unknown`",
    );
}

#[test]
fn a_pointer_valued_uncontracted_call_escapes_nothing_and_stays_unknown() {
    if !clang_available() {
        return;
    }
    let lowered = lowering_agrees(
        "void *subject(void) { return z_unknown_ptr(); }",
        "void *subject(void) { void *t; t = z_unknown_ptr(); return t; }",
        "call to uncontracted function `z_unknown_ptr`",
    );
    assert_eq!(lowered.leaks(), 0);
}

#[test]
fn a_contracted_call_refused_for_its_argument_reports_the_argument() {
    if !clang_available() {
        return;
    }
    // `r_borrow_code` is contracted; the classifier refuses the call because
    // its argument is itself an unmodeled nested call. The published reason is
    // the classifier's, whatever its exact wording, never the return's.
    let lowered = run("int subject(void) { return r_borrow_code(z_unknown_ptr()); }");
    assert!(!lowered.refuses_return(), "{lowered:?}");
    assert_eq!(lowered.status, AnalysisStatus::Unknown);
    assert!(!lowered.certified);
    let reference =
        run("int subject(void) { int t; t = r_borrow_code(z_unknown_ptr()); return t; }");
    assert_eq!(lowered.shape(), reference.shape());
    assert!(
        !lowered.unsupported().is_empty(),
        "the argument refusal must be published: {lowered:?}"
    );
}

// ------------------------------------------------------------------------
// Fail-closed: nothing escapes, nothing is discharged, nothing is hidden
// ------------------------------------------------------------------------

/// The return-position refusal this card leaves in place for a call inside
/// arithmetic, measured on the same body: what it reports is the old answer
/// the lowering must match on everything except the refusal's wording.
///
/// Deliberately re-pinned by card W45-RETURN-EXPRESSION-FAMILIES: the
/// reference spelling was a cast of the call, which that card now lowers as
/// this card lowers the uncast call; `CALL + 0` keeps the refusal.
fn old_answer_leaks(body_with_arithmetic: &str) -> usize {
    let old = run(body_with_arithmetic);
    assert!(old.refuses_return(), "{old:?}");
    old.leaks()
}

#[test]
fn an_owned_resource_handed_to_an_uncontracted_call_is_not_escaped() {
    if !clang_available() {
        return;
    }
    // The discharging direction invariant 1 forbids: were the lowering to
    // escape `p` through the call's result, the live obligation would vanish
    // and the body could certify. It is decided exactly as the return
    // refusal decides it (the `+ 0` spelling keeps that refusal), and the
    // function stays non-`Safe` with the call's refusal.
    let lowered = lowering_agrees(
        "int subject(void) { void *p = q_alloc(); return z_take(p); }",
        "int subject(void) { void *p = q_alloc(); int t; t = z_take(p); return t; }",
        "call to uncontracted function `z_take`",
    );
    assert_eq!(
        lowered.leaks(),
        old_answer_leaks("int subject(void) { void *p = q_alloc(); return z_take(p) + 0; }")
    );
    // It was accused (1 leak) until card W44-LEAK-PIN-AUDIT: the refused call
    // hands `p` to `z_take`, so the refused hand-on rule withdraws the
    // accusation on both spellings. The obligation is still never discharged:
    // the body does not certify.
    assert_eq!(lowered.leaks(), 0, "{lowered:?}");
    assert!(!lowered.certified, "{lowered:?}");
}

#[test]
fn a_live_obligation_is_accused_exactly_as_the_return_refusal_accused_it() {
    if !clang_available() {
        return;
    }
    // `report_leaks` runs at every exit whatever the path carries, and a
    // refusal never touches resource states. The old exit-node refusal and the
    // lowered `Unknown` evaluation node followed by a `NoOp` exit therefore
    // leave the same state at the exit: neither hides nor manufactures a leak.
    let lowered = lowering_agrees(
        "int subject(int n) { void *p = q_alloc(); return z_unknown(n); }",
        "int subject(int n) { void *p = q_alloc(); int t; t = z_unknown(n); return t; }",
        "call to uncontracted function `z_unknown`",
    );
    assert_eq!(
        lowered.leaks(),
        old_answer_leaks("int subject(int n) { void *p = q_alloc(); return z_unknown(n) + 0; }")
    );
}

#[test]
fn a_leak_on_another_path_is_still_reported() {
    if !clang_available() {
        return;
    }
    let lowered = lowering_agrees(
        "int subject(int n) { void *p = q_alloc(); if (n) { q_free(p); return z_unknown(n); } return 0; }",
        "int subject(int n) { void *p = q_alloc(); if (n) { q_free(p); int t; t = z_unknown(n); return t; } return 0; }",
        "call to uncontracted function `z_unknown`",
    );
    assert_eq!(
        lowered.leaks(),
        1,
        "the other path's leak must stand: {lowered:?}"
    );
}

// ------------------------------------------------------------------------
// The boundary: every other shape keeps the return-position refusal
// ------------------------------------------------------------------------

/// A negative control: `returned` keeps the return-position refusal and is
/// neither `Safe` nor certified.
fn stays_refused(returned: &str) -> Run {
    let refused = run(returned);
    assert!(
        refused.refuses_return(),
        "this shape is outside the family and must keep the return refusal: {refused:?}"
    );
    assert_ne!(refused.status, AnalysisStatus::Safe);
    assert!(!refused.certified);
    refused
}

#[test]
fn a_cast_of_an_uncontracted_call_is_lowered_by_w45() {
    if !clang_available() {
        return;
    }
    // Deliberately re-pinned by card W45-RETURN-EXPRESSION-FAMILIES: a cast of
    // a whole call whose classification is a refusal is lowered to that
    // refusal exactly as the uncast call is here — re-attribution, nothing
    // admitted, never `Safe`.
    let cast = run("long subject(int n) { return (long) z_unknown(n); }");
    let uncast = run("long subject(int n) { return z_unknown(n); }");
    assert!(!cast.refuses_return(), "{cast:?}");
    assert_eq!(cast.shape(), uncast.shape());
    assert_ne!(cast.status, AnalysisStatus::Safe);
    assert!(!cast.certified);
}

#[test]
fn a_call_inside_arithmetic_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    stays_refused("int subject(int n) { return z_unknown(n) + 1; }");
}

#[test]
fn a_conditional_over_a_call_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    stays_refused("int subject(int n) { return n ? z_unknown(n) : 0; }");
}

#[test]
fn a_comma_expression_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    stays_refused("int subject(int n) { return (z_unknown(n), 0); }");
}

#[test]
fn a_type_operator_is_not_a_callee() {
    if !clang_available() {
        return;
    }
    // `sizeof ( struct w_pair )` is call-shaped and refused by the fixed
    // scalar type-query rule (the operand is a record). It is not a call, and
    // "call to uncontracted function `sizeof`" would misattribute it.
    //
    // Deliberately re-pinned by card W45-RETURN-EXPRESSION-FAMILIES: in a
    // function returning a scalar, a type operator over a resolved, never
    // variably modified type name is a translation-time constant that carries
    // nothing, so the return is decided. It is still never read as a call.
    let decided = run("unsigned long subject(void) { return sizeof (struct w_pair); }");
    assert!(!decided.refuses_return(), "{decided:?}");
    assert!(
        !decided
            .unsupported()
            .iter()
            .any(|text| text.contains("`sizeof`")),
        "{decided:?}"
    );
    assert_eq!(decided.status, AnalysisStatus::Safe);
    assert!(decided.certified);
}

#[test]
fn a_reallocated_result_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    // The W4 route declines a `Reallocated` result on soundness grounds; the
    // classifier *admits* the call, so this rule, which lowers only a refusal,
    // must not touch it.
    stays_refused("void *subject(void *p) { return q_realloc(p, 8); }");
}

#[test]
fn a_body_spelling_the_reserved_place_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    stays_refused(
        "int subject(int n) { int __qcore_returned_value = n; return z_unknown(__qcore_returned_value); }",
    );
}

#[test]
fn an_admitted_whole_call_is_unchanged() {
    if !clang_available() {
        return;
    }
    // The positive W4 route is asked first and is not re-decided here: a
    // contracted `None`-result call with a live obligation still accuses its
    // leak at a modeled exit.
    let admitted = run("int subject(void) { void *p = q_alloc(); return r_borrow_code(p); }");
    assert!(!admitted.refuses_return(), "{admitted:?}");
    assert!(admitted.unsupported().is_empty(), "{admitted:?}");
    assert_eq!(admitted.leaks(), 1, "{admitted:?}");
}
