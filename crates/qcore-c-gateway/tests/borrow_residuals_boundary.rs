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

//! `BORROW-RESIDUALS` — the four residuals card BORROW-LIVENESS left open
//! (`docs/reports/remaining-borrow-residuals/README.md`).
//!
//! Card BORROW-LIVENESS gave a `Borrowed` position a real liveness
//! precondition and recorded four bounded residuals: `-PRESTATE` (the
//! precondition is read in the call's pre-state, so a call that releases one
//! argument and borrows the same name at another position is not refused),
//! `-PLACES` (plain names only), `-OUT` (`Out` positions untouched) and
//! `-ASSUMPTION` (the source-effect route's same-named premise).
//!
//! This file is the held-out acceptance family for that quartet. Every body was
//! measured on the card's base `19fbaa5` with the extension absent, and each
//! test's `BASE:` line records what it answered there, so every assertion is a
//! *measured* movement or a *measured* preservation.
//!
//! The fixtures use a `y_*` API deliberately unrelated to ChunkIO, Fluent Bit
//! and to the `b_*` (C02-P), `g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08) and
//! `r_*` (O04-READ-BOUND) families, so no name in them can matter. `Safe` here
//! is `Safe` for a synthetic declared custody projection over trusted test
//! assumptions; nothing here promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/borrow_residuals.qv");

const DECLARATIONS: &str = "\
typedef unsigned long y_word; \
struct y_item { int weight; struct y_item *next; }; \
struct y_box { int flag; struct y_item *item; y_word *raw; void *any; }; \
struct y_box *y_make(int n); \
struct y_item *y_take(int n); \
void y_drop(struct y_box *b); \
void y_release(struct y_item *i); \
int y_probe(const volatile void *i); \
int y_count(int n); \
void y_retire(struct y_item *dead, const volatile void *seen); \
void y_retire_late(const volatile void *seen, struct y_item *dead); \
void y_close(struct y_item *dead, int delete); \
void y_hand_over(struct y_item *gone, const volatile void *seen); \
void y_fill(struct y_item **destination); \
void y_fill_from(struct y_item **destination, const volatile void *seen);";

fn contracts() -> qcore_c_gateway::legacy_c::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "borrow_residuals.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the residual contract fixture loads")
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
/// Deliberately complete rather than filtered, exactly as in
/// `borrow_liveness_boundary.rs`: a rule that silently swallowed a pre-existing
/// leak or double release while adding its own refusal would pass a subset
/// assertion.
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

/// A body the extension refuses: never `Safe`, `Unknown`, carrying exactly the
/// expected complete diagnostic vector.
fn refused(label: &str, body: &str, expected: &[&str]) {
    let result = analyze(body);
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a call that destroys what it reads must never certify"
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

/// A body the extension leaves alone: the `Safe` answer it had before, with no
/// diagnostic at all and a certificate that rechecks independently.
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
// `BORROW-LIVENESS-PRESTATE` — the residual, and its measured movements
// --------------------------------------------------------------------------

/// **The headline of this card.** One call, one name, declared
/// `resource_released(dead)` at one position and `resource_borrowed(seen)` at
/// another. Card BORROW-LIVENESS's path rule reads the call's pre-state, where
/// the identity is still live, so nothing asked a question.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_call_that_releases_what_it_borrows_no_longer_certifies() {
    refused(
        "release at position 0, borrow at position 1",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_retire(one, one); }",
        &["unknown_alias:one"],
    );
}

/// The mirror order. If the answer moved only in one direction it would be an
/// artefact of the parameter loop, not a decision about the call.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn the_mirror_parameter_order_answers_the_same_way() {
    refused(
        "borrow at position 0, release at position 1",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_retire_late(one, one); }",
        &["unknown_alias:one"],
    );
}

/// Two **distinct** declared names the projection settled to one identity. The
/// decision is over identities, so the definite-alias shape answers the same
/// way — and note this is not the may-alias relation: nothing here reads a
/// declared record tag or a peer.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn two_names_settled_to_one_identity_are_refused() {
    refused(
        "definite alias across the two positions",
        "void w(int n) { struct y_item *one; struct y_item *two; one = y_take(n); \
         if (!one) { return; } two = one; y_retire(one, two); }",
        &["unknown_alias:two"],
    );
}

/// The residual's stated bound survives its own repair: the *path* rule still
/// reads the pre-state, so the release side keeps its own diagnostics and the
/// identity is refused at every **later** borrow exactly as before.
///
/// BASE: `Unknown`, `[unknown_alias:one]` — from the later `y_probe` alone.
#[test]
fn a_later_borrow_of_the_retired_identity_is_still_refused() {
    refused(
        "retire, then borrow again",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_retire(one, one); y_probe(one); }",
        &["unknown_alias:one"],
    );
}

// --------------------------------------------------------------------------
// `BORROW-LIVENESS-PLACES` — the composite half the same-call rule reaches
// --------------------------------------------------------------------------

/// A whole `BASE->FIELD` bound read at both positions of one call. O04-READ-BOUND
/// requires the owner and the field link to be `LIVE` **in the pre-state**, and
/// they are, so its channel is silent; no other channel asks this call's own
/// effects about its own reads.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_bound_read_a_call_releases_and_borrows_is_refused() {
    refused(
        "bound read at both positions",
        "void w(int n) { struct y_box *b; struct y_item *c; b = y_make(n); if (!b) { return; } \
         c = y_take(n); if (!c) { y_drop(b); return; } b->item = c; y_retire(b->item, b->item); \
         y_drop(b); }",
        &["unknown_alias:b->item"],
    );
}

/// The same in the mirror order.
///
/// BASE: `Safe`, no diagnostics.
#[test]
fn a_bound_read_in_the_mirror_order_is_refused() {
    refused(
        "bound read, mirror order",
        "void w(int n) { struct y_box *b; struct y_item *c; b = y_make(n); if (!b) { return; } \
         c = y_take(n); if (!c) { y_drop(b); return; } b->item = c; \
         y_retire_late(b->item, b->item); y_drop(b); }",
        &["unknown_alias:b->item"],
    );
}

/// The path rule keeps its plain-name bound, and the measurement shows why it
/// costs nothing: a bound read whose child this body already released is
/// refused by O04-READ-BOUND's own `LIVE` requirement, naming the owner.
///
/// BASE and now: `Unknown`, `[unknown_alias:b]`.
#[test]
fn a_bound_read_of_a_released_child_keeps_o04_s_refusal() {
    preserved(
        "O04-READ-BOUND's channel, unchanged",
        "void w(int n) { struct y_box *b; struct y_item *c; b = y_make(n); if (!b) { return; } \
         c = y_take(n); if (!c) { y_drop(b); return; } b->item = c; y_release(b->item); \
         y_probe(b->item); y_drop(b); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:b"],
    );
}

/// A member read through an aggregate this body already released is refused on
/// the owner, and the pre-existing leak of the child is still reported. The
/// complete vector is asserted, so nothing was swallowed.
///
/// BASE and now: `Unsafe`, `[leak, unknown_alias:b]`.
#[test]
fn a_bound_read_through_a_released_owner_keeps_its_whole_answer() {
    preserved(
        "released owner",
        "void w(int n) { struct y_box *b; struct y_item *c; b = y_make(n); if (!b) { return; } \
         c = y_take(n); if (!c) { y_drop(b); return; } b->item = c; y_drop(b); \
         y_probe(b->item); }",
        AnalysisStatus::Unsafe,
        &["leak", "unknown_alias:b"],
    );
}

/// The linked-aggregate prelude the member-address route needs: an admitted
/// store is what makes `b` a base O04-B09B10 will consider at all.
const LINKED: &str = "\
    struct y_box *b; struct y_item *c; \
    b = y_make(1); if (!b) { return; } \
    c = y_take(2); if (!c) { y_drop(b); return; } \
    b->item = c;";

/// The third composite route: a **member address** at a `Borrowed` position
/// (O04-B09B10). `resolved_aggregate_owner` requires the aggregate to be one
/// live body-acquired identity here and now, so an address taken through a
/// released owner is refused on the owner before either of this card's rules
/// runs.
#[test]
fn a_member_address_through_a_released_owner_keeps_o04_s_refusal() {
    let result = analyze(&format!(
        "void w(int n) {{ {LINKED} y_release(b->item); y_drop(b); y_probe(&b->flag); }}"
    ));
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a member address through a released owner must never certify"
    );
    let w = entry(program(&result));
    assert!(
        vector(w).contains(&"unknown_alias:b".to_owned()),
        "O04-B09B10 refuses on the owner: {:?}",
        w.diagnostics
    );
}

/// …and the live control, so the refusal above is not vacuous: the identical
/// member address on a live owner is admitted with no diagnostic at all.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn a_member_address_through_a_live_owner_is_admitted() {
    admitted(
        "live owner control",
        &format!("void w(int n) {{ {LINKED} y_probe(&b->flag); y_release(b->item); y_drop(b); }}"),
    );
}

/// A field read on a base this projection never tracks — C02-P's admission —
/// is untouched. Its base is untracked by construction, so it can carry no
/// identity for either rule to settle.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn an_untracked_base_field_read_is_untouched() {
    admitted(
        "C02-P's admitted borrowed pointer-field actual",
        "void w(struct y_box *b) { y_probe(b->any); }",
    );
}

/// A member access outside every admitted route never reaches a `ContractCall`
/// at all: the lexical access gate refuses the statement first. That is why the
/// path rule's plain-name bound leaves no composite place undecided.
///
/// BASE and now: `Unknown`, `[unknown_alias:one]`.
#[test]
fn an_unadmitted_member_access_is_refused_before_the_call() {
    preserved(
        "member access outside every admitted route",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_release(one); y_probe(one->next); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:one"],
    );
}

// --------------------------------------------------------------------------
// The preserved half — every shape that must keep its existing answer
// --------------------------------------------------------------------------

/// **The production shape.** `cio_chunk_close(chunk, delete)` is the only
/// declaration in the pinned 64-module corpus carrying a released and a
/// borrowed parameter at once, and its borrowed parameter is a scalar flag.
/// §C04.2 admits scalars in exactly that position; a rule that fired here would
/// refuse the shape the corpus actually has.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn a_scalar_at_the_borrowed_position_is_untouched() {
    admitted(
        "the `cio_chunk_close(chunk, delete)` shape",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_close(one, n); }",
    );
}

/// `resource_consumed` "is not a proof of destruction" (§C04.2). A
/// consumed-and-borrowed pair therefore keeps its existing answer, exactly as
/// `ESCAPED` alone does on the path rule — `C02-P-ALIAS-CONSUME` is not closed
/// through this back door either.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn a_consumed_and_borrowed_pair_is_not_a_destruction() {
    admitted(
        "consumed, not released",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_hand_over(one, one); }",
    );
}

/// The ordinary well-formed call: two *different* identities at the two
/// positions. Nothing may move here, or the rule would refuse every call that
/// releases anything at all.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn distinct_identities_at_the_two_positions_are_admitted() {
    admitted(
        "distinct identities",
        "void w(int n) { struct y_item *one; struct y_item *two; one = y_take(n); \
         if (!one) { return; } two = y_take(n); if (!two) { y_release(one); return; } \
         y_retire(one, two); y_release(two); }",
    );
}

/// The may-alias pair the projection will **not** settle: two uncontracted
/// formals of one declared class. This rule takes no position on it — that is
/// C02-P-ALIAS's channel and its own recorded boundary — and the answer is
/// byte-identical to the base.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn a_may_alias_pair_keeps_exactly_its_existing_answer() {
    admitted(
        "two uncontracted formals, C02-P-ALIAS's channel",
        "void w(struct y_item *a, struct y_item *b) { y_retire(a, b); }",
    );
}

// --------------------------------------------------------------------------
// `BORROW-LIVENESS-OUT` — measured, not extended
// --------------------------------------------------------------------------

/// An `Out` actual is the address of a slot. Writing a fresh identity over a
/// binding this body already released is **re-acquisition**, and refusing it
/// would refuse the idiom card BORROW-LIVENESS already pins as `Safe`.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn an_out_slot_over_a_released_binding_is_re_acquisition() {
    admitted(
        "release, refill, release",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_release(one); y_fill(&one); y_release(one); }",
    );
}

/// …and the fresh identity the slot received is borrowable, which is the whole
/// point: an `Out` position produces custody rather than requiring it.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn the_identity_an_out_slot_produced_is_borrowable() {
    admitted(
        "release, refill, borrow, release",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_release(one); y_fill(&one); y_probe(one); y_release(one); }",
    );
}

/// Overwriting a slot that still held a live identity is a **leak**, and the
/// existing `Unsafe` channel reports it. The `Out` position needs no liveness
/// precondition to make that happen.
///
/// BASE and now: `Unsafe`, `[leak]`.
#[test]
fn an_out_slot_over_a_live_binding_is_the_existing_leak() {
    preserved(
        "live slot overwritten",
        "void w(int n) { struct y_item *one; one = y_take(n); if (!one) { return; } \
         y_fill(&one); y_release(one); }",
        AnalysisStatus::Unsafe,
        &["leak"],
    );
}

/// A member-address `Out` actual — the only composite `Out` shape — never
/// reaches a `ContractCall`: the lexical access gate refuses it first. So the
/// one place where "the slot's address may be dangling" could be asked is
/// already fail-closed.
///
/// BASE and now: `Unknown`, `[unknown_alias:b, unsupported]`.
#[test]
fn a_member_address_out_actual_is_refused_before_the_call() {
    preserved(
        "&b->item at an Out position",
        "void w(int n) { struct y_box *b; b = y_make(n); if (!b) { return; } \
         y_fill(&b->item); y_release(b->item); y_drop(b); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:b", "unsupported"],
    );
}

/// A `Borrowed` position beside an `Out` one still gets the path rule's answer,
/// so the `Out` position's presence neither adds nor removes a refusal.
///
/// BASE and now: `Unknown`, `[unknown_alias:one]`.
#[test]
fn a_dead_borrow_beside_an_out_slot_keeps_its_refusal() {
    preserved(
        "Out beside a definitely dead borrow",
        "void w(int n) { struct y_item *one; struct y_item *two; one = y_take(n); \
         if (!one) { return; } y_release(one); y_fill_from(&two, one); y_release(two); }",
        AnalysisStatus::Unknown,
        &["unknown_alias:one"],
    );
}

/// Forwarding this function's own out formal is a slot the analysis sanctions
/// or refuses on its own terms, with no identity to be live or dead.
///
/// BASE and now: `Safe`, no diagnostics.
#[test]
fn a_forwarded_out_formal_is_untouched() {
    admitted(
        "forwarded out formal",
        "void w(struct y_item **out) { y_fill(out); }",
    );
}

/// The last `Out` actual form: a null slot names no place at all, so the
/// contract's declared effect has nowhere to land and the call refuses. There
/// is no identity here either, which completes the enumeration of the arms an
/// `Out` actual can take.
///
/// BASE and now: `Unknown`, `[unsupported]`.
#[test]
fn a_null_out_actual_is_refused_on_its_own_terms() {
    preserved(
        "null out slot",
        "void w(int n) { y_count(n); y_fill(0); }",
        AnalysisStatus::Unknown,
        &["unsupported"],
    );
}
