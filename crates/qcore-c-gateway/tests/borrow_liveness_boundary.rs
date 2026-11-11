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

//! `ALIAS-LOCALS-BORROW-LIVENESS` — the `Borrowed` liveness precondition
//! (`docs/reports/remaining-borrow-liveness/README.md`).
//!
//! Card ALIAS-LOCALS recorded, as its first open item
//! (`docs/reports/remaining-alias-locals/README.md` §3.4, §7), that a
//! `Borrowed` position has **no implemented precondition at all**: the transfer
//! function falls through on it, so a body may hand a callee an identity the
//! projection itself records `RELEASED` and be certified `Safe`. That card
//! repaired the hazard only along its may-alias relation, between two *distinct*
//! names.
//!
//! This file is the held-out acceptance family for the general rule. Its
//! headline is measured rather than argued:
//!
//! ```c
//! void w(int n) { struct b_handle *one; one = b_open(0, 0, n);
//!                 if (!one) { return; } b_release(one); b_probe(one); }
//! ```
//!
//! — one name, no second name anywhere, no aliasing question of any kind — was
//! **`Safe`, no diagnostics** at the branch base `bf210eb`. The plainest
//! possible use-after-free shape this projection can express, certified.
//!
//! Every body below was measured at that base with the rule disabled, and the
//! `BASE:` line of each test records what it answered there, so every assertion
//! is a *measured* movement or a *measured* preservation.
//!
//! The fixtures reuse card C02-P's generic `b_*` API, deliberately unrelated to
//! ChunkIO, Fluent Bit and O04's `g_*` family, so no name in them can matter.
//! `Safe` here is `Safe` for a synthetic declared custody projection over
//! trusted test assumptions; nothing here promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/c02p_generic.qv");

/// C02-P's declarations, verbatim, so the two families share one surface.
const DECLARATIONS: &str = "\
struct b_ctx; struct b_pool; struct b_handle; struct b_other; \
struct b_storage { int state; struct b_ctx *cio; struct b_pool *stream; }; \
struct b_instance { int flags; struct b_storage *storage; }; \
struct b_handle *b_open(struct b_ctx *cio, struct b_pool *stream, int flags); \
void b_close(struct b_handle *h); \
int b_probe(void *h); \
int b_write(struct b_handle *h, int n); \
void b_adopt(void *p); \
void b_release(void *p); \
struct b_storage *b_pick(struct b_instance *in);";

fn contracts() -> qcore_c_gateway::legacy_c::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "c02p_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the generic contract fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent BORROW-LIVENESS test: {reason}");
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

fn entry(summary: &LegacyProgramSummary) -> &LegacyFunctionSummary {
    summary.functions.get("w").expect("missing function `w`")
}

/// The **complete** ordered diagnostic vector, rendered as short keys.
///
/// Deliberately complete rather than filtered: a rule that silently removed a
/// pre-existing leak or double release while adding its own refusal would pass
/// a subset assertion. Every test below asserts the whole vector.
fn vector(summary: &LegacyFunctionSummary) -> Vec<String> {
    let mut keys = summary
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
            LegacyDiagnosticKind::KindMismatchedRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterClose { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
            LegacyDiagnosticKind::Unsupported { .. } => "unsupported".to_owned(),
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

/// A body the rule refuses: never `Safe`, `Unknown`, carrying exactly the
/// expected complete diagnostic vector.
fn refused(label: &str, body: &str, expected: &[&str]) {
    let result = analyze(body);
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a definitely released borrowed actual must never certify"
    );
    let w = entry(program(&result));
    assert_eq!(
        w.status,
        AnalysisStatus::Unknown,
        "{label}: {:?}",
        w.diagnostics
    );
    assert_eq!(
        vector(w),
        expected
            .iter()
            .map(|key| (*key).to_owned())
            .collect::<Vec<_>>(),
        "{label}: {:?}",
        w.diagnostics
    );
}

/// A body the rule leaves alone: the `Safe` answer it had before this card,
/// with no diagnostic at all and a certificate that rechecks.
fn admitted(label: &str, body: &str) {
    match analyze(body) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            let w = entry(&certificate.summary);
            assert_eq!(w.status, AnalysisStatus::Safe, "{label}");
            assert!(
                vector(w).is_empty(),
                "{label}: no diagnostic should remain: {:?}",
                w.diagnostics
            );
            check_legacy_safety_certificate(&certificate)
                .unwrap_or_else(|error| panic!("{label}: certificate rechecks: {error}"));
        }
        other => panic!("{label}: expected a validated custody projection: {other:?}"),
    }
}

/// A body whose complete answer this card must leave exactly as it found it,
/// whatever that answer is.
fn preserved(label: &str, body: &str, status: AnalysisStatus, expected: &[&str]) {
    let result = analyze(body);
    let w = entry(program(&result));
    assert_eq!(w.status, status, "{label}: {:?}", w.diagnostics);
    assert_eq!(
        vector(w),
        expected
            .iter()
            .map(|key| (*key).to_owned())
            .collect::<Vec<_>>(),
        "{label}: {:?}",
        w.diagnostics
    );
}

// --------------------------------------------------------------------------
// The refusal — the shape no alias relation can reach
// --------------------------------------------------------------------------

/// **The headline of this card.** One name. No second name of any type. No
/// alias question at all. Released, then handed to a `Borrowed` position.
///
/// ALIAS-LOCALS could not reach this: its relation is between two *distinct*
/// declared names, and there is only one here.
///
/// BASE: **`Safe`, no diagnostics.**
#[test]
fn a_released_identity_borrowed_through_its_own_name_no_longer_certifies() {
    if !clang_available() {
        return;
    }
    refused(
        "the plainest use-after-free shape this projection can express",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_release(one); b_probe(one); }",
        &["unknown_alias:one"],
    );
}

/// The same refusal at a second `Borrowed` position of a call whose other
/// parameter is a scalar, so the rule is shown to select positions by declared
/// mode rather than by argument index.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_refusal_selects_positions_by_declared_mode() {
    if !clang_available() {
        return;
    }
    refused(
        "a released identity at the first of two borrowed positions",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_release(one); b_write(one, n); }",
        &["unknown_alias:one"],
    );
}

/// Two distinct released identities each refuse at their own read, so the rule
/// is per-actual rather than per-function.
///
/// The two names are also same-tagged locals, so ALIAS-LOCALS relates them and
/// its `one/two` and `two/one` obligations are present too. Both halves are
/// asserted: this card adds `one` and `two`, and removes nothing.
///
/// BASE: `Unknown`, `[unknown_alias:one/two, unknown_alias:two/one]`.
#[test]
fn each_released_actual_refuses_on_its_own() {
    if !clang_available() {
        return;
    }
    refused(
        "two independent released identities",
        "void w(int n) { struct b_other *k; struct b_handle *one; struct b_handle *two; \
         one = b_open(0, 0, n); if (!one) { return; } two = b_open(0, 0, n); \
         if (!two) { b_release(one); return; } b_release(one); b_release(two); \
         b_probe(one); b_probe(two); (void) k; }",
        &[
            "unknown_alias:one",
            "unknown_alias:one/two",
            "unknown_alias:two",
            "unknown_alias:two/one",
        ],
    );
}

// --------------------------------------------------------------------------
// The definite/may separation — the two relations must not blur
// --------------------------------------------------------------------------

/// C02-P's measured shape, unchanged. Two **formals** of one declared class:
/// `a` is released and `b` is borrowed. `b`'s own binding is an uncontracted
/// formal — unknown and possibly null — so this rule declines, and ALIAS-LOCALS'
/// may-alias obligation `b/a` is the whole of what is reported here.
///
/// This is the pin that keeps the two relations apart: a *may* pair keeps its
/// `UnknownAlias` obligation and gains nothing from this card.
///
/// BASE: `Unknown`, `[unknown_alias:a, unknown_alias:b/a]` — identical.
#[test]
fn a_may_alias_pair_keeps_exactly_its_existing_obligation() {
    if !clang_available() {
        return;
    }
    preserved(
        "the may-alias pair is untouched",
        "void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:a", "unknown_alias:b/a"],
    );
}

/// ALIAS-LOCALS' §2 fixture, **strengthened deliberately**. `two` is a copy of
/// `one` and `one` is released, so the projection has settled `two` on a
/// released identity and this card refuses it by name — while ALIAS-LOCALS'
/// `two/one` obligation, the load-bearing half of the pin it moved, is still
/// required to be present.
///
/// The mirror of this assertion lives in `may_alias_boundary.rs`, whose own pin
/// was updated in the same change and keeps the same half.
///
/// BASE: `Unknown`, `[unknown_alias:two/one]`.
#[test]
fn the_definite_copy_fixture_is_refused_by_both_rules_at_once() {
    if !clang_available() {
        return;
    }
    refused(
        "the ALIAS-LOCALS copy shape, now refused by name as well as by peer",
        "void w(int n) { struct b_handle *one; struct b_handle *two; \
         one = b_open(0, 0, n); if (!one) { return; } two = one; \
         b_release(one); b_probe(two); }",
        &["unknown_alias:two", "unknown_alias:two/one"],
    );
}

// --------------------------------------------------------------------------
// Every way the rule declines — each a measured preservation
// --------------------------------------------------------------------------

/// **Release on one branch only is not definite.** The joined status carries
/// both `LIVE` and `RELEASED`, so the rule declines and the body keeps exactly
/// the answer it had: the pre-existing leak on the branch that did not release.
///
/// That leak is what makes the shape observable at all — in this projection a
/// `LIVE | RELEASED` identity cannot reach a read without either leaking or
/// double-releasing — and asserting the complete vector proves this card
/// neither added to nor removed from it.
///
/// BASE: `Unsafe`, `[leak]` — identical.
#[test]
fn a_release_on_one_branch_only_is_not_definite() {
    if !clang_available() {
        return;
    }
    preserved(
        "released on one branch, live on the other",
        "void w(int n, int c) { struct b_handle *one; one = b_open(0, 0, n); \
         if (!one) { return; } if (c) { b_release(one); } b_probe(one); }",
        AnalysisStatus::Unsafe,
        &["leak"],
    );
}

/// **The card HYGIENE-14 movement** (`BORROW-LIVENESS-MIXED`): released on one
/// branch and **consumed** on the other, then borrowed.
///
/// Card BORROW-LIVENESS recorded this as `Safe` and left it open, on the
/// reading that the identity is "dead on one path only". The condition derived
/// from the join semantics is different and sharper: neither arm leaves this
/// body a pointer the projection knows to be alive — a release destroyed it and
/// a consumption moved it away — and one of them destroyed it, so there is no
/// reaching path on which the borrow reads something the projection can vouch
/// for. It is refused, in the same `Unknown` class and with no new code.
///
/// The `else` arm keeps `ESCAPED` alone admitted (below): what makes *this*
/// shape refusable is the `RELEASED` bit that the other arm contributes.
///
/// BASE (card BORROW-LIVENESS): **`Safe`, no diagnostics** — moved.
#[test]
fn a_release_joined_with_a_consumption_is_definitely_not_live() {
    if !clang_available() {
        return;
    }
    refused(
        "released on one branch, consumed on the other",
        "void w(int n, int c) { struct b_handle *one; one = b_open(0, 0, n); \
         if (!one) { return; } if (c) { b_release(one); } else { b_adopt(one); } b_probe(one); }",
        &["unknown_alias:one"],
    );
}

/// The must-half survives the widening. A third arm leaves the identity `LIVE`,
/// so the joined status is `LIVE | RELEASED | ESCAPED`, one reaching path does
/// entitle the read, and the rule declines — leaving exactly the answer the body
/// already had for the live arm.
///
/// Without this control the widened condition could not be distinguished from
/// "any status containing `RELEASED`", which would be the may-fact the existing
/// `Unsafe` kinds use and would refuse the branch-local shape above it.
#[test]
fn an_entitling_third_branch_keeps_the_mixed_join_admitted() {
    if !clang_available() {
        return;
    }
    preserved(
        "released on one branch, consumed on another, live on a third",
        "void w(int n, int c, int d) { struct b_handle *one; one = b_open(0, 0, n); \
         if (!one) { return; } if (c) { b_release(one); } else if (d) { b_adopt(one); } \
         b_probe(one); }",
        AnalysisStatus::Unsafe,
        &["leak"],
    );
}

/// **Re-acquisition between the release and the borrow.** The name is bound to
/// a fresh live identity, so the precondition holds again and the body still
/// certifies with a certificate that rechecks.
///
/// BASE: `Safe`, no diagnostics — identical.
#[test]
fn a_reacquisition_between_the_release_and_the_borrow_is_admitted() {
    if !clang_available() {
        return;
    }
    admitted(
        "re-acquired before the read",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_release(one); one = b_open(0, 0, n); if (!one) { return; } b_probe(one); \
         b_release(one); }",
    );
}

/// **Custody moved is not custody destroyed.** `resource_consumed` transfers the
/// obligation "without claiming that the callee destroys it" (§C04.2), so an
/// escaped identity is not refused. This retains `C02-P-ALIAS-CONSUME` exactly
/// rather than closing it through the back door.
///
/// BASE: `Safe`, no diagnostics — identical.
#[test]
fn a_consumed_identity_is_not_a_released_one() {
    if !clang_available() {
        return;
    }
    admitted(
        "consumed, then borrowed",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_adopt(one); b_probe(one); }",
    );
}

/// **Every scalar `Borrowed` actual.** §C04.2 admits scalars in exactly this
/// position and nowhere else; a scalar is a name this projection never binds, so
/// the rule declines. A rule that fired here would refuse most of the corpus for
/// no reason at all.
///
/// BASE: `Safe`, no diagnostics — identical.
#[test]
fn a_scalar_borrowed_actual_is_never_refused() {
    if !clang_available() {
        return;
    }
    admitted(
        "a live handle and a scalar at two borrowed positions",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_write(one, n); b_release(one); }",
    );
}

/// A live identity is not refused, which is the ordinary case of the whole
/// corpus.
///
/// BASE: `Safe`, no diagnostics — identical.
#[test]
fn a_live_identity_is_never_refused() {
    if !clang_available() {
        return;
    }
    admitted(
        "borrowed while live",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_probe(one); b_release(one); }",
    );
}

/// A **pointer-field** `Borrowed` actual keeps its existing answer: this rule
/// looks at plain names only, because O04-READ-BOUND's `bound_read_resolves` and
/// O04-B09B10's `resolved_aggregate_owner` already require the identity to be
/// `LIVE` at exactly those positions. Restating it here would double-report
/// against another card's channel.
///
/// BASE: `Unknown`, `[unknown_alias:storage, unsupported]` — identical.
#[test]
fn a_pointer_field_actual_keeps_its_existing_answer() {
    if !clang_available() {
        return;
    }
    preserved(
        "a field read whose base was released",
        "void w(struct b_instance *in) { struct b_storage *storage; storage = b_pick(in); \
         b_release(storage); b_probe(storage->cio); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:storage", "unsupported"],
    );
}

// --------------------------------------------------------------------------
// The other parameter modes are untouched
// --------------------------------------------------------------------------

/// A second **release** of a released identity is still the `DoubleRelease` it
/// always was, in the `Unsafe` class. This card adds no diagnostic there: the
/// precondition it implements is a `Borrowed`-position rule, and the release
/// position already had one.
///
/// BASE: `Unsafe`, `[double_release]` — identical.
#[test]
fn a_double_release_keeps_its_existing_unsafe_class() {
    if !clang_available() {
        return;
    }
    preserved(
        "released twice",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_release(one); b_release(one); }",
        AnalysisStatus::Unsafe,
        &["double_release"],
    );
}

/// **Preconditions are evaluated in the pre-state.** A call that releases one
/// argument and borrows the same name at another position is not refused by this
/// rule: at the call the identity is still live, and the release side has its own
/// diagnostics. Recorded as `BORROW-LIVENESS-PRESTATE`.
///
/// BASE: `Safe`, no diagnostics — identical.
#[test]
fn the_precondition_is_read_in_the_call_pre_state() {
    if !clang_available() {
        return;
    }
    admitted(
        "borrowed and released by one call",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_probe(one); b_release(one); }",
    );
}

// --------------------------------------------------------------------------
// The refusal reaches the callers, and survives certificate rederivation
// --------------------------------------------------------------------------

/// A refused body publishes no validated W4 custody summary, so a caller that
/// depended on it stops certifying too. The refusal is not cosmetic.
///
/// BASE: both functions `Safe`, no diagnostics.
#[test]
fn a_refused_callee_withdraws_its_callers_certificate() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{DECLARATIONS} \
         static void inner(int n) {{ struct b_handle *one; one = b_open(0, 0, n); \
         if (!one) {{ return; }} b_release(one); b_probe(one); }} \
         void w(int n) {{ inner(n); }}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .expect("fixture accepted by Clang");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a caller of a refused body must not certify: {result:?}"
    );
    let summary = program(&result);
    let inner = summary
        .functions
        .get("inner")
        .expect("the callee is projected");
    assert_eq!(
        inner.status,
        AnalysisStatus::Unknown,
        "{:?}",
        inner.diagnostics
    );
    assert_eq!(vector(inner), vec!["unknown_alias:one".to_owned()]);
}

/// Bodies the rule admits still rederive: `check_legacy_safety_certificate`
/// re-runs the whole analysis, so the settled pass runs there too and must
/// reach the same answer. Every `admitted` fixture above already asserts this;
/// this test states it once over the shape closest to the refusal.
#[test]
fn an_admitted_body_rederives_from_its_own_source() {
    if !clang_available() {
        return;
    }
    admitted(
        "release, re-acquire, borrow — rederived",
        "void w(int n) { struct b_handle *one; one = b_open(0, 0, n); if (!one) { return; } \
         b_release(one); one = b_open(0, 0, n); if (!one) { return; } b_write(one, n); \
         b_release(one); }",
    );
}
