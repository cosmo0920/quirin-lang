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

//! O04-B08 — aggregate stores and the aggregate branch condition
//! (`docs/reports/remaining-o04-b08/README.md`).
//!
//! Card O04-ARMS settled four of the six exits of byte-verbatim
//! `flb_input_chunk_create` and recorded the routes-mask arm C5's residual as
//! exactly two open families: **aggregate stores** (O00 blocker B08, including
//! `ic->chunk = chunk`) and the **aggregate branch condition**
//! `if (ic->routes_mask == NULL)`. Both were destination-side and condition-side
//! refusals that fire *before* any contract is consulted.
//!
//! This card supplies the missing typed premises for both, in
//! `crate::aggregate_store`, and measures what C5 does with them.
//!
//! Two legs, both through the public gateway entry points only.
//!
//! **Held-out generic family.** A `k_*` API deliberately unrelated to ChunkIO,
//! to Fluent Bit, and to the `g_*` (O04) and `h_*` (O04-ARMS) families: an
//! admitted store whose every exit settles, the adversarial matrix the card
//! names — a store through a released aggregate, a double store, an aliased
//! destination, a borrowed-in aggregate that escapes, a branch on an unrelated
//! field — and declaration substitutions the independent certificate recheck
//! refuses.
//!
//! **Real source.** The same byte-verbatim upstream regions O04 and O04-ARMS
//! pinned, under their byte-unchanged premise sets plus one declared callee.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for
//! a synthetic declared custody projection under premises this card names; both
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
/// The single declared callee premise this card adds; see the fixture header.
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_b08_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";
/// The existing rejection an unmodeled branch condition keeps.
const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn path_contracts() -> String {
    format!(
        "{}\n{ARMS_CALLEES}\n{B08_CALLEES}",
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
            eprintln!("skipping Clang-dependent O04-B08 test: {reason}");
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

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

/// Deliberately unrelated to every library in this repository and to the `g_*`
/// and `h_*` families of cards O04 and O04-ARMS. Every rule under test is
/// stated over declared C types and contract modes, so no name here can matter.
///
/// * `k_box` is the admitted aggregate: a scalar member, two `struct k_item *`
///   members, one arithmetic-pointee member, and — deliberately — one `void *`,
///   one double-star and one array member the reader must refuse.
/// * `k_odd` carries a member whose declared type the reader will not resolve.
/// * `union k_alt` is the union the base premise must refuse.
const DECLARATIONS: &str = "\
typedef unsigned long k_word; \
struct k_ctx; \
struct k_item { int weight; }; \
struct k_box { int flag; struct k_item *item; struct k_item *spare; \
               k_word *raw; void *any; struct k_item **deep; \
               struct k_item *fixed[2]; volatile struct k_item *hot; }; \
struct k_odd { struct k_ctx *ctx; void (*hook)(int); }; \
union k_alt { struct k_item *item; int tag; }; \
struct k_outer { struct k_box *box; }; \
struct k_box *k_make(int n); \
struct k_item *k_take(int n); \
k_word *k_words(int n); \
void k_drop(struct k_box *b); \
void k_release(struct k_item *i); \
void k_borrow(struct k_item *i); \
void k_adopt(struct k_item *i); \
void k_untracked(struct k_box *b); \
struct k_box *k_pick(struct k_outer *o);";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_b08_generic.qcore", GENERIC_CONTRACTS),
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

/// The shape the card exists for, in the generic family: a resource is stored
/// into a member of a live aggregate this body acquired, the routes-mask-style
/// branch tests another member, and the failure arm releases both.
const K_CREATE: &str = "\
struct k_box *k_create(int n) \
{ \
    struct k_item *it; \
    struct k_box *b; \
    it = k_take(n); \
    if (!it) { return 0; } \
    b = k_make(n); \
    if (!b) { k_release(it); return 0; } \
    b->flag = 0; \
    b->item = it; \
    b->spare = 0; \
    b->raw = k_words(n); \
    if (b->raw == 0) { k_release(it); k_drop(b); return 0; } \
    return b; \
}";

/// Non-vacuity, positive direction: the store, the acquisition into a member
/// and the branch over a member are all modeled, so every exit settles.
#[test]
fn an_aggregate_store_and_its_branch_condition_discharge_every_exit() {
    if !clang_available() {
        return;
    }
    let certificate = certified(K_CREATE, "k_create");
    let measured = certificate
        .summary
        .functions
        .get("k_create")
        .expect("entry present");
    assert_eq!(measured.status, AnalysisStatus::Safe);
    assert!(
        unsupported(measured).is_empty(),
        "no refusal remains: {:?}",
        measured.diagnostics
    );
    assert!(
        measured
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "every exit settles: {:?}",
        exit_shape(measured)
    );
}

/// P-B08-SHALLOW's *reporting* half: publishing the aggregate publishes what
/// its admitted members link, so the stored resource is not a leak at the
/// success exit — and the failure arm that releases both is still Safe.
#[test]
fn publishing_the_aggregate_carries_its_linked_child() {
    if !clang_available() {
        return;
    }
    let measured = summary_of(K_CREATE, "k_create");
    assert!(
        leaks(&measured).is_empty(),
        "the escaping aggregate carries its member: {:?}",
        measured.diagnostics
    );
}

/// The adversarial row the card names: a second store into the same member
/// must not discharge the first resource. It stays live and is reported.
#[test]
fn a_double_store_reports_the_first_resource_as_a_leak() {
    if !clang_available() {
        return;
    }
    let body = "\
struct k_box *k_twice(int n) \
{ \
    struct k_item *first; \
    struct k_item *second; \
    struct k_box *b; \
    first = k_take(n); \
    if (!first) { return 0; } \
    second = k_take(n); \
    if (!second) { k_release(first); return 0; } \
    b = k_make(n); \
    if (!b) { k_release(first); k_release(second); return 0; } \
    b->item = first; \
    b->item = second; \
    return b; \
}";
    let measured = summary_of(body, "k_twice");
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "the overwritten resource is reported: {:?}",
        measured.diagnostics
    );
    assert_eq!(
        leaks(&measured).len(),
        1,
        "exactly the first resource leaks: {:?}",
        measured.diagnostics
    );
}

/// The C00 premise in its store direction, checked flow-sensitively: a store
/// through an aggregate this body already released is never admitted, and can
/// never reach `Safe`.
#[test]
fn a_store_through_a_released_aggregate_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
int k_after_free(int n) \
{ \
    struct k_item *it; \
    struct k_box *b; \
    b = k_make(n); \
    if (!b) { return 0; } \
    it = k_take(n); \
    if (!it) { k_drop(b); return 0; } \
    k_drop(b); \
    b->item = it; \
    return 0; \
}";
    let measured = summary_of(body, "k_after_free");
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "a store through a released aggregate must never be admitted: {:?}",
        measured.diagnostics
    );
    assert!(
        measured.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "b"
        )),
        "the aggregate is reported unresolved: {:?}",
        measured.diagnostics
    );
}

/// A caller-owned aggregate is refused: a transfer into it is invisible at the
/// call site, and this card models publication, not interface change.
#[test]
fn a_store_into_a_borrowed_in_aggregate_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
void k_fill(struct k_box *b, int n) \
{ \
    struct k_item *it; \
    it = k_take(n); \
    if (!it) { return; } \
    b->item = it; \
}";
    let measured = summary_of(body, "k_fill");
    assert_ne!(measured.status, AnalysisStatus::Safe);
    assert!(
        unsupported(&measured).contains(&INDIRECT),
        "the base is not one this body acquired, so the store keeps its \
         existing refusal: {:?}",
        measured.diagnostics
    );
}

/// Every base premise, one row per violated premise, each keeping the existing
/// `Unknown` **and** the existing diagnostic. Nothing here is a new code.
#[test]
fn every_refused_base_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 6] = [
        (
            "the aggregate is read out of another object rather than acquired",
            "void k_w(struct k_outer *o, int n) { struct k_item *it; struct k_box *b; \
             b = o->box; it = k_take(n); if (!it) { return; } b->item = it; }",
        ),
        (
            "the aggregate is copied to a second declared name",
            "void k_w(int n) { struct k_item *it; struct k_box *b; struct k_box *c; \
             b = k_make(n); if (!b) { return; } c = b; it = k_take(n); \
             if (!it) { k_drop(b); return; } b->item = it; k_untracked(c); }",
        ),
        (
            "the aggregate is address-taken whole",
            "void k_w(int n) { struct k_item *it; struct k_box *b; struct k_box **pp; \
             b = k_make(n); if (!b) { return; } pp = &b; it = k_take(n); \
             if (!it) { k_drop(b); return; } b->item = it; k_untracked(*pp); }",
        ),
        (
            "the aggregate is bound twice",
            "void k_w(int n) { struct k_item *it; struct k_box *b; \
             b = k_make(n); b = k_make(n); if (!b) { return; } it = k_take(n); \
             if (!it) { k_drop(b); return; } b->item = it; }",
        ),
        (
            "the aggregate is dereferenced whole",
            "void k_w(int n) { struct k_item *it; struct k_box *b; struct k_box copy; \
             b = k_make(n); if (!b) { return; } copy = *b; it = k_take(n); \
             if (!it) { k_drop(b); return; } b->item = it; }",
        ),
        (
            "the aggregate is subscripted",
            "void k_w(int n) { struct k_item *it; struct k_box *b; \
             b = k_make(n); if (!b) { return; } it = k_take(n); \
             if (!it) { k_drop(b); return; } b[0].flag = 0; b->item = it; }",
        ),
    ];
    for (reason, body) in cases {
        let measured = summary_of(body, "k_w");
        assert_ne!(
            measured.status,
            AnalysisStatus::Safe,
            "{reason}: must stay refused, diagnostics {:?}",
            measured.diagnostics
        );
        assert!(
            unsupported(&measured).contains(&INDIRECT),
            "{reason}: the existing diagnostic must be retained, got {:?}",
            measured.diagnostics
        );
    }
}

/// Every member premise, same discipline. A member the reader will not fully
/// type keeps the destination side of the lexical gate exactly as it was.
#[test]
fn every_refused_member_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let prologue = "struct k_item *it; struct k_box *b; b = k_make(n); if (!b) { return; } \
                    it = k_take(n); if (!it) { k_drop(b); return; }";
    // **Deliberate pin update, card ALIAS-AGGREGATE.** The row
    // `("a `void *` member has no resolved pointee", "b->any = it;")` moved out
    // of this table: a `void *` destination is now the admitted third member
    // class, measured on the production corpus where
    // `struct flb_input_chunk` declares `void *chunk`. It is asserted admitted
    // — and asserted still unreadable — in
    // `crates/qcore-c-gateway/tests/alias_aggregate.rs`. Every other row of
    // this table is unchanged and is the load-bearing half.
    // **Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS.** The row
    // `("a chained destination is not one selection", "b->item->weight = 1;")`
    // moved to `a_chained_member_of_an_acquired_base_refuses_on_its_link`. It
    // still refuses, but now by the chain route's own obligation: `b` is one
    // live identity, and the link `b->item` has no admitted store in that
    // body.
    let cases: [(&str, &str); 4] = [
        (
            "a double-star member is not a single pointer",
            "b->deep = 0;",
        ),
        ("an array member is never admitted", "b->fixed[0] = it;"),
        ("a `volatile` member is never admitted", "b->hot = it;"),
        // `b->flag = it;` — a pointer into a scalar member — is not
        // expressible: Clang rejects the fixture before the analyzer sees it,
        // so that direction is covered by the module's own unit tests
        // (`a_pointer_field_never_takes_a_scalar_expression` and its mirror)
        // rather than here. Recorded, not skipped.
        (
            "a member read is not a single declared identifier",
            "b->spare = b->item;",
        ),
    ];
    for (reason, store) in cases {
        let body = format!("void k_w(int n) {{ {prologue} {store} k_release(it); k_drop(b); }}");
        let measured = summary_of(&body, "k_w");
        assert_ne!(
            measured.status,
            AnalysisStatus::Safe,
            "{reason}: must stay refused, diagnostics {:?}",
            measured.diagnostics
        );
        assert!(
            unsupported(&measured).contains(&INDIRECT),
            "{reason}: the existing diagnostic must be retained, got {:?}",
            measured.diagnostics
        );
    }
}

/// The branch-condition half is bounded to admitted **pointer** members of
/// admitted bases. Everything else keeps the existing branch refusal, which is
/// a different diagnostic from the store one and stays that way.
#[test]
fn a_branch_on_an_unrelated_field_keeps_its_existing_refusal() {
    if !clang_available() {
        return;
    }
    // Deliberate pin update, card W43-INDEX-TRAVERSAL: `b->flag == 0` over a
    // live, null-tested identity is now the typed condition read, whose
    // dereference obligation `b` meets, so the first row this test carried
    // certifies. It is still not a null test: nothing about `b` is refined.
    let measured = summary_of(
        "void k_w(int n) { struct k_box *b; b = k_make(n); if (!b) { return; } \
         if (b->flag == 0) { k_drop(b); return; } k_drop(b); }",
        "k_w",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "{:?}",
        measured.diagnostics
    );
    // Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS: the row
    // `b->item->weight == 0` moved to
    // `a_chained_member_of_an_acquired_base_refuses_on_its_link`, which
    // refuses it by the chain route's own obligation.
    let cases: [(&str, &str); 1] = [(
        // Card REACH-GATE's F4 admits a member read on a base this body
        // never binds, so the base here is bound — by `k_drop(o)` — which
        // is the load-bearing half of this row: an aggregate branch on a
        // base whose custody this body may change keeps its refusal.
        "a member of a base this rule never admitted, on a bound base",
        "void k_w(struct k_outer *o, int n) { struct k_box *b; struct k_box *p; \
         b = k_make(n); if (!b) { return; } p = k_pick(o); \
         if (o->box == 0) { k_drop(b); return; } k_drop(b); }",
    )];
    // Deliberate pin update, card W45-CALLEE-SUMMARY-CASCADE (gateway
    // constraint 5j): `o->box == 0` is a null test of one pointer member, now
    // lowered to the typed read of `o` rather than refused. `o` is an unbound
    // formal (`k_pick` is uncontracted and binds nothing), so the read's
    // obligation is met and nothing is refined; the function stays Unknown on
    // the uncontracted call alone. This aggregate rule still admits nothing
    // here.
    for (reason, body) in cases {
        let measured = summary_of(body, "k_w");
        assert!(
            !unsupported(&measured).contains(&BRANCH),
            "{reason}: the member null test is the typed read, got {:?}",
            measured.diagnostics
        );
        assert_eq!(
            unsupported(&measured),
            vec!["call to uncontracted function `k_pick`"],
            "{reason}"
        );
        assert_ne!(measured.status, AnalysisStatus::Safe, "{reason}");
    }
}

/// Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS. Two rows moved here
/// from the tables above: a chained store and a chained branch through an
/// aggregate this body acquired. The chain route now reads both. `b` is one
/// live identity (tracked), so its link `b->item` is a member place this body
/// could bind. No admitted store bound it, so there is no evidence, and each
/// refuses with `UnknownAlias` on the link. Neither reaches `Safe`.
#[test]
fn a_chained_member_of_an_acquired_base_refuses_on_its_link() {
    if !clang_available() {
        return;
    }
    let prologue = "struct k_item *it; struct k_box *b; b = k_make(n); if (!b) { return; } \
                    it = k_take(n); if (!it) { k_drop(b); return; }";
    for body in [
        format!("void k_w(int n) {{ {prologue} b->item->weight = 1; k_release(it); k_drop(b); }}"),
        "void k_w(int n) { struct k_box *b; b = k_make(n); if (!b) { return; } \
         if (b->item->weight == 0) { k_drop(b); return; } k_drop(b); }"
            .to_owned(),
    ] {
        let measured = summary_of(&body, "k_w");
        assert_ne!(measured.status, AnalysisStatus::Safe, "{body}");
        assert!(
            measured.diagnostics.iter().any(|diagnostic| matches!(
                &diagnostic.kind,
                LegacyDiagnosticKind::UnknownAlias { variable } if variable == "b->item"
            )),
            "{body}: {:?}",
            measured.diagnostics
        );
    }
}

/// Deliberate pin update, card REACH-GATE.
///
/// O04-B08 pinned "a member of a base this rule never admitted" as refused,
/// meaning refused *by O04-B08's aggregate null-condition rule*. REACH-GATE's
/// F4 admits that same read by a different, typed route — the base is a formal
/// this body never binds, so no place under it is one the projection binds, and
/// the value the read produces is discarded into the branch decision. The row
/// above keeps the negative half on a bound base; this records the positive
/// half explicitly rather than leaving it to a silently weakened assertion.
#[test]
fn a_branch_on_a_never_bound_formals_member_is_admitted_by_reach_gate() {
    if !clang_available() {
        return;
    }
    let measured = summary_of(
        "void k_w(struct k_outer *o, int n) { struct k_box *b; b = k_make(n); \
         if (!b) { return; } if (o->box == 0) { k_drop(b); return; } k_drop(b); }",
        "k_w",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert!(
        unsupported(&measured).is_empty(),
        "no refusal must remain, got {:?}",
        measured.diagnostics
    );
}

/// The admission must never manufacture a discharge: an arm that stores and
/// then returns without releasing is reported, with a residual naming the
/// acquisition.
#[test]
fn a_store_never_manufactures_a_discharge() {
    if !clang_available() {
        return;
    }
    let body = "\
void k_w(int n) \
{ \
    struct k_item *it; \
    struct k_box *b; \
    b = k_make(n); \
    if (!b) { return; } \
    it = k_take(n); \
    if (!it) { k_drop(b); return; } \
    b->item = it; \
    k_drop(b); \
}";
    let measured = summary_of(body, "k_w");
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "releasing the aggregate does not discharge its member \
         (premise P-B08-SHALLOW): {:?}",
        measured.diagnostics
    );
    assert_eq!(leaks(&measured).len(), 1, "{:?}", measured.diagnostics);
    assert!(
        measured
            .exits
            .iter()
            .any(|exit| exit.custody == ExitCustody::Residual),
        "the residual is recorded: {:?}",
        exit_shape(&measured)
    );
}

/// A discharge follows the *declared* contract, not the statement's shape:
/// weakening the destructor to a borrow turns the arm into a reported residual.
#[test]
fn a_borrowed_destructor_turns_the_store_path_arm_into_a_residual() {
    if !clang_available() {
        return;
    }
    let weakened = GENERIC_CONTRACTS.replace(
        "ensures normal resource_released(i, nonnull);",
        "ensures normal resource_borrowed(i);",
    );
    assert_ne!(weakened, GENERIC_CONTRACTS, "the substitution applied");
    let source = format!("{DECLARATIONS} {K_CREATE}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "k_create",
        &load("o04_b08_generic.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    let measured = function(program(&result), "k_create");
    assert_eq!(measured.status, AnalysisStatus::Unsafe);
    assert!(
        !leaks(measured).is_empty(),
        "the weakened premise is reported, never silently discharged: {:?}",
        measured.diagnostics
    );
}

/// W4 derivation, W4 validation and the independent certificate recheck all
/// repeat this classification from the certificate's own content: substituting
/// a member or base declaration in the certificate's source withdraws the
/// projection.
#[test]
fn a_certificate_resting_on_an_aggregate_store_is_rederived_from_source() {
    if !clang_available() {
        return;
    }
    let certificate = certified(K_CREATE, "k_create");
    let substitutions: [(&str, &str, &str); 5] = [
        (
            "the member becomes a double pointer",
            "struct k_item *item;",
            "struct k_item **item;",
        ),
        (
            "the member becomes an array",
            "struct k_item *item;",
            "struct k_item *item[2];",
        ),
        (
            "the member becomes volatile",
            "struct k_item *item;",
            "volatile struct k_item *item;",
        ),
        (
            "the aggregate record becomes a union",
            "struct k_box { int flag;",
            "union k_box { int flag;",
        ),
        // Substituting one *resolved* pointee for another is deliberately
        // NOT a refusal: the premise is that the member holds exactly one
        // pointer identity, which `struct k_ctx *` satisfies just as
        // `k_word *` does. The premise-violating substitution is a pointee the
        // reader will not resolve at all.
        //
        // **Deliberate pin update, card ALIAS-AGGREGATE.** This row used to
        // substitute `void *raw;`, which is now the admitted third member
        // class, so it would no longer withdraw anything. The substitution is
        // an undeclared typedef instead — still exactly "a pointee the reader
        // will not resolve at all", and still refused. The row keeps its
        // load-bearing direction; only the spelling that violates the premise
        // moved.
        (
            "the acquired member's pointee stops resolving",
            "k_word *raw;",
            "k_hidden *raw;",
        ),
    ];
    for (reason, from, to) in substitutions {
        let mut substituted = certificate.clone();
        assert!(
            substituted.preprocessed_source.contains(from),
            "{reason}: `{from}` is present in the certificate source"
        );
        substituted.preprocessed_source = substituted.preprocessed_source.replace(from, to);
        assert!(
            check_legacy_safety_certificate(&substituted).is_err(),
            "{reason}: the recheck must refuse the substituted declaration"
        );
    }
    check_legacy_safety_certificate(&certificate)
        .expect("the unmodified certificate still rechecks");
}

// --------------------------------------------------------------------------
// Real source
// --------------------------------------------------------------------------

fn real_unit() -> String {
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n")
}

fn analyze_real(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_b08_path.qcore", &path_contracts()),
    )
    .unwrap_or_else(|error| panic!("O04-B08 fixture rejected by Clang: {error:?}"))
}

/// The card's headline, on real source: the routes-mask arm C5 settles. The
/// three cleanup arms and C5 all release the same constructor acquisition site,
/// and C5 additionally releases the `ic` allocation it made.
///
/// **Composed-tree reconciliation (LOADER-ABI × NEIGHBOURS-CHAINED-TAIL).** The
/// count is O04-B08's original five. Card LOADER-ABI corrected this fixture's
/// `flb_error` from three named parameters to the faithful
/// `(const char *, ...)` reading, which moved the chained tail actual
/// `storage->stream->name` out of a declared `Borrowed` position; measured
/// alone that dropped the *logging* arm C1 from the settled set. Clause **T5**
/// of card NEIGHBOURS-CHAINED-TAIL admits that actual at the declared data tail
/// the corrected declaration gives it, so C1 settles again — on a premise C's
/// own `...` supports. **C5, this card's subject, was never involved**, and is
/// still asserted below by shape rather than by position: it is the exit
/// carrying **two** released sites.
#[test]
fn the_routes_mask_arm_discharges_under_the_declared_premises() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let shape = exit_shape(caller);
    let discharged = caller
        .exits
        .iter()
        .filter(|exit| exit.custody == ExitCustody::Discharged)
        .collect::<Vec<_>>();
    assert_eq!(
        discharged.len(),
        5,
        "C1 through C5 all settle; exits were {shape:?}"
    );
    // C1 crosses no acquisition; C2/C3/C4 release the constructor's chunk;
    // C5 releases that chunk *and* the aggregate it allocated.
    let mut released = discharged
        .iter()
        .map(|exit| exit.sites.len())
        .collect::<Vec<_>>();
    released.sort_unstable();
    assert_eq!(released, vec![0, 1, 1, 1, 2], "exits were {shape:?}");
    for exit in &discharged {
        assert!(
            exit.sites
                .iter()
                .all(|site| site.disposition == SiteDisposition::Released),
            "every site a settled exit carries is released: {shape:?}"
        );
    }
    let cleanup = discharged
        .iter()
        .filter(|exit| !exit.sites.is_empty())
        .collect::<Vec<_>>();
    let constructor_site = cleanup[0].sites[0].acquisition_line;
    for exit in &cleanup {
        assert_eq!(
            exit.sites[0].acquisition_line, constructor_site,
            "all four cleanup arms release the same acquisition site: {shape:?}"
        );
    }
    // No promotion: the function itself stays Unknown on the success exit.
    assert_eq!(caller.status, AnalysisStatus::Unknown);
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Unknown && !exit.sites.is_empty())
            .count(),
        1,
        "only the out-of-scope success exit stays unresolved carrying sites: {shape:?}"
    );
    // Composed-tree reconciliation: no exit is left unresolved while crossing
    // nothing. The site-free `Unknown` this assertion used to count was the
    // logging arm under card LOADER-ABI's measurement alone; clause T5 settles
    // it, and the direction is kept here so a site-free unresolved exit cannot
    // reappear unnoticed.
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Unknown && exit.sites.is_empty())
            .count(),
        0,
        "no exit is unresolved while crossing nothing: {shape:?}"
    );
    assert!(
        caller
            .exits
            .iter()
            .all(|exit| exit.custody != ExitCustody::Residual),
        "no residual is alleged on this path: {shape:?}"
    );
}

/// The other half of the obligation: every refusal that remains on the real
/// path belongs to the success exit C6, which is out of scope, and none of them
/// is an aggregate store or an aggregate branch condition any more.
#[test]
fn every_remaining_refusal_belongs_to_the_out_of_scope_success_exit() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let remaining = unsupported(caller);
    assert!(
        !remaining.is_empty(),
        "the named blockers are still reported"
    );
    // Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule
    // 12k): the gate's refusals of calls whose callees this fixture leaves
    // uncontracted (`msgpack_packer_init(&ic->mp_pck, …)`, `mk_list_add`,
    // `flb_hash_table_add`, `pthread_mutex_lock`/`_unlock` with member
    // actuals) now name those callees.
    let unexpected = remaining
        .iter()
        .filter(|description| {
            !(**description == INDIRECT
                || **description == "call to uncontracted function `flb_trace`"
                || **description == "call to uncontracted function `msgpack_packer_init`"
                || **description == "call to uncontracted function `mk_list_add`"
                || **description == "call to uncontracted function `flb_hash_table_add`"
                || **description == "call to uncontracted function `pthread_mutex_lock`"
                || **description == "call to uncontracted function `pthread_mutex_unlock`"
                || **description == "nullable resource `ic` used by a non-null contract")
        })
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty(),
        "unexpected remaining refusals {unexpected:?}"
    );
    // Nothing is alleged in the defect direction anywhere on this path.
    assert!(
        leaks(caller).is_empty(),
        "no leak is alleged: {:?}",
        caller.diagnostics
    );
}

/// The constructor keeps its own boundary: its four guard exits stay
/// discharged and its rollback and success exits stay unresolved. Its
/// `ch->name = strdup(name);` store is admitted, so the allocation this
/// projection previously erased is now a *second* tracked identity — which
/// makes both unresolved exits carry more, never less.
#[test]
fn the_constructor_body_still_does_not_discharge_its_own_contract() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let constructor = function(program(&result), "cio_chunk_open");
    assert_eq!(constructor.status, AnalysisStatus::Unknown);
    let shape = exit_shape(constructor);
    assert_eq!(
        shape
            .iter()
            .filter(|(custody, sites)| *custody == ExitCustody::Discharged && sites.is_empty())
            .count(),
        4,
        "the four guard/allocation-failure exits acquire nothing: {shape:?}"
    );
    let unresolved = shape
        .iter()
        .filter(|(custody, _)| *custody == ExitCustody::Unknown)
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 2, "{shape:?}");
    for (_, sites) in &unresolved {
        assert_eq!(
            sites.len(),
            2,
            "the chunk and its name allocation: {shape:?}"
        );
        assert!(
            sites
                .iter()
                .all(|disposition| *disposition == SiteDisposition::Mixed),
            "both stay unresolved: {shape:?}"
        );
    }
    assert!(
        leaks(constructor).is_empty(),
        "nothing is alleged in the defect direction: {:?}",
        constructor.diagnostics
    );
}

/// The discharge rests on *declared* member types read from this same token
/// stream: substituting one member declaration in the real fixture withdraws
/// the arm, in each case for its own measured reason. Supporting C declarations
/// and casts are adjusted where a newer Clang would otherwise reject the probe
/// before the analyzer can measure that member premise.
/// One substitution row: (name, original, replacement, supporting C edits).
type MemberSubstitution<'a> = (&'a str, &'a str, &'a str, &'a [(&'a str, &'a str)]);

#[test]
fn substituting_a_member_declaration_withdraws_the_arm() {
    if !clang_available() {
        return;
    }
    let substitutions: [MemberSubstitution; 4] = [
        // `ic->chunk = chunk` becomes invalid C if only the destination gains
        // a pointer level. Use the null-initialized member of the same record
        // so Clang accepts the mutation and the analyzer must refuse it.
        // The stored-resource double-pointer substitution remains covered by
        // `a_certificate_resting_on_an_aggregate_store_is_rederived_from_source`.
        (
            "the null-initialized member becomes a double pointer",
            "struct flb_task *task;",
            "struct flb_task **task;",
            &[],
        ),
        (
            "the stored member becomes volatile",
            "struct cio_chunk *chunk;",
            "volatile struct cio_chunk *chunk;",
            &[],
        ),
        // **Deliberate pin update, card ALIAS-AGGREGATE.** `void *` is now an
        // admitted member class, so the substitution that violates the premise
        // is a member that stops being a *single* pointer at all.
        (
            "the branched member stops being a single pointer",
            "flb_route_mask_element *routes_mask;",
            "flb_route_mask_element **routes_mask;",
            &[
                (
                    "flb_route_mask_element *routes_mask,",
                    "flb_route_mask_element **routes_mask,",
                ),
                ("(flb_route_mask_element *)", "(flb_route_mask_element **)"),
            ],
        ),
        // Retagging or duplicating `struct flb_input_chunk` is not
        // expressible on the real fixture — Clang rejects the unit, exactly as
        // card O04-ARMS §4.2 recorded for `struct cio_stream` — so the
        // ambiguous-record direction is covered by the module's unit tests and
        // by the generic certificate substitutions instead. Recorded, not
        // skipped. This row substitutes an admitted *scalar* member instead.
        // **Deliberate pin update, card ALIAS-AGGREGATE**, for the same
        // reason: a `void *` member is admitted now, so the substitution uses a
        // double-star member, which every class refuses.
        (
            "an admitted scalar member stops being one",
            "size_t stream_off;",
            "size_t **stream_off;",
            &[],
        ),
    ];
    for (reason, from, to, supporting_substitutions) in substitutions {
        let unit = real_unit();
        assert!(unit.contains(from), "{reason}: `{from}` is present");
        let mut substituted = unit.replacen(from, to, 1);
        for (supporting_from, supporting_to) in supporting_substitutions {
            assert!(
                substituted.contains(supporting_from),
                "{reason}: supporting declaration `{supporting_from}` is present"
            );
            substituted = substituted.replacen(supporting_from, supporting_to, 1);
        }
        let result = analyze_real(&substituted);
        let caller = function(program(&result), "flb_input_chunk_create");
        assert_eq!(caller.status, AnalysisStatus::Unknown, "{reason}");
        assert!(
            unsupported(caller).contains(&INDIRECT),
            "{reason}: the analyzer must retain the indirect-access refusal: {:?}",
            caller.diagnostics
        );
        assert!(
            caller
                .exits
                .iter()
                .filter(|exit| exit.custody == ExitCustody::Discharged)
                .count()
                < 5,
            "{reason}: the arm must be withdrawn, exits were {:?}",
            exit_shape(caller)
        );
    }
}

/// The discharge follows the *declared* contract, not the statement's shape:
/// weakening the destructor to a borrow turns the settled arms into reported
/// residuals rather than silent discharges.
#[test]
fn weakening_the_destructor_premise_withdraws_the_arms() {
    if !clang_available() {
        return;
    }
    let weakened = path_contracts().replace(
        "ensures normal resource_released(ch, nonnull);",
        "ensures normal resource_borrowed(ch);",
    );
    assert_ne!(weakened, path_contracts(), "the substitution applied");
    let unit = real_unit();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &unit,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_b08_path.qcore", &weakened),
    )
    .expect("fixture accepted by Clang");
    let caller = function(program(&result), "flb_input_chunk_create");
    assert!(
        !leaks(caller).is_empty(),
        "the weakened premise is reported, never silently discharged: {:?}",
        caller.diagnostics
    );
}

/// Not an assertion: prints the measured per-exit ledger and the remaining
/// refusals of the real path under this card's premise set, so the archived
/// report can be regenerated. Run with
/// `cargo test -p qcore-c-gateway --test o04_b08 -- --ignored --nocapture
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
