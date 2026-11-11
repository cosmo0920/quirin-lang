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

//! O04-READ-BOUND — member reads through a base this body bound
//! (`docs/reports/remaining-o04-read-bound/README.md`).
//!
//! Card C02-P admits a member *read* only through a base the projection **never**
//! binds (its premise 4); card O04-B08 admits a *store* into a member of a base
//! this body acquired, and recorded the complementary read as its last named
//! blocker, `O04-B08-READ-BOUND-BASE` — the one refusal standing between the
//! constructor's routes-mask arm and its success exit's other blockers.
//!
//! This card supplies the missing typed premise, in
//! `crate::aggregate_store::bound_read_actuals` plus a flow-sensitive
//! revalidation in `legacy_c`'s transfer function, and measures what the real
//! path does with it.
//!
//! Two legs, both through the public gateway entry points only.
//!
//! **Held-out generic family.** An `r_*` API deliberately unrelated to ChunkIO,
//! to Fluent Bit, and to the `g_*` (O04), `h_*` (O04-ARMS) and `k_*` (O04-B08)
//! families: the admitted read, the four adversarial rows the card names — read
//! after overwrite, read after release of the linked child, read through an
//! aliased base, read of a different member — every refused position and member
//! shape, and declaration substitutions the independent certificate recheck
//! refuses.
//!
//! **Real source.** The same byte-verbatim upstream regions O04, O04-ARMS and
//! O04-B08 pinned, under their byte-unchanged premise sets.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for a
//! synthetic declared custody projection under premises this card names; both
//! real functions keep function-level status `Unknown`, the success exit C6 is
//! **not** discharged, and no raw unit is promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyInstruction, LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts,
    SiteDisposition, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qr");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_read_bound_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";
/// The exact refusal this card removes from the real path.
const READ_BOUND_SITE: &str = "has_routes = flb_routes_mask_set_by_tag(ic->routes_mask,";

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
            eprintln!("skipping Clang-dependent O04-READ-BOUND test: {reason}");
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

fn reports_unresolved(summary: &LegacyFunctionSummary, name: &str) -> bool {
    summary.diagnostics.iter().any(|diagnostic| {
        matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == name
        )
    })
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

/// Deliberately unrelated to every library in this repository and to the `g_*`,
/// `h_*` and `k_*` families of cards O04, O04-ARMS and O04-B08.
///
/// * `r_box` is the admitted aggregate: a scalar member, two `struct r_item *`
///   members, one arithmetic-pointee member (the routes-mask shape), and —
///   deliberately — one `void *`, one double-star, one array and one `volatile`
///   member the reader must refuse.
/// * `r_outer` supplies an aggregate read out of another object's memory.
/// * `r_probe` accepts every pointer shape, including qualifiers, so Clang
///   accepts the adversarial calls and the gateway decides their admission.
const DECLARATIONS: &str = "\
typedef unsigned long r_word; \
struct r_item { int weight; struct r_item *next; }; \
struct r_box { int flag; struct r_item *item; struct r_item *spare; \
               r_word *raw; void *any; struct r_item **deep; \
               struct r_item *fixed[2]; volatile struct r_item *hot; }; \
struct r_outer { struct r_box *box; }; \
struct r_box *r_make(int n); \
struct r_item *r_take(int n); \
r_word *r_words(int n); \
void r_drop(struct r_box *b); \
void r_release(struct r_item *i); \
int r_probe(const volatile void *i); \
int r_scan(r_word *w); \
int r_count(int n); \
void r_reap(struct r_item *i); \
void r_adopt(struct r_item *i); \
void r_untracked(struct r_box *b);";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_read_bound_generic.qcore", GENERIC_CONTRACTS),
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
/// into a member of a live aggregate this body acquired, an arithmetic-pointee
/// member is acquired directly into the aggregate, and **both** are later read
/// back at borrowed positions — the `flb_routes_mask_set_by_tag(ic->routes_mask,
/// …)` shape.
const R_CREATE: &str = "\
struct r_box *r_create(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) { return 0; } \
    b = r_make(n); \
    if (!b) { r_release(it); return 0; } \
    b->item = it; \
    b->raw = r_words(n); \
    if (b->raw == 0) { r_release(it); r_drop(b); return 0; } \
    r_probe(b->item); \
    r_scan(b->raw); \
    return b; \
}";

/// Non-vacuity, positive direction. Both bound reads are modeled — one of a
/// record-pointer member, one of the arithmetic-pointee member that has the real
/// routes-mask shape — so every exit settles and no refusal remains.
#[test]
fn a_bound_member_read_at_a_borrowed_position_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let certificate = certified(R_CREATE, "r_create");
    let measured = certificate
        .summary
        .functions
        .get("r_create")
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
    assert!(
        leaks(measured).is_empty(),
        "the escaping aggregate carries both members: {:?}",
        measured.diagnostics
    );
}

/// The admission is a precision gain in the **defect** direction too: the same
/// body with the failure arm's release removed reports the leak rather than
/// disappearing into a refusal.
#[test]
fn an_admitted_read_does_not_hide_a_leak() {
    if !clang_available() {
        return;
    }
    let body = R_CREATE.replace("{ r_release(it); r_drop(b); return 0; }", "{ return 0; }");
    assert_ne!(body, R_CREATE, "the substitution applied");
    let measured = summary_of(&body, "r_create");
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "the unreleased arm is reported: {:?}",
        measured.diagnostics
    );
    assert!(
        !leaks(&measured).is_empty(),
        "a leak is reported: {:?}",
        measured.diagnostics
    );
}

/// Adversarial row 1a — **read after overwrite with a live replacement.** The
/// read denotes the *current* link, and the overwritten resource stays live and
/// is reported. A read never discharges anything.
#[test]
fn a_read_after_an_overwrite_denotes_the_current_link_and_reports_the_first() {
    if !clang_available() {
        return;
    }
    let body = "\
struct r_box *r_twice(int n) \
{ \
    struct r_item *first; \
    struct r_item *second; \
    struct r_box *b; \
    first = r_take(n); \
    if (!first) { return 0; } \
    second = r_take(n); \
    if (!second) { r_release(first); return 0; } \
    b = r_make(n); \
    if (!b) { r_release(first); r_release(second); return 0; } \
    b->item = first; \
    b->item = second; \
    r_probe(b->item); \
    return b; \
}";
    let measured = summary_of(body, "r_twice");
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "the overwritten resource is reported: {:?}",
        measured.diagnostics
    );
    assert_eq!(
        leaks(&measured).len(),
        1,
        "exactly the first resource leaks; the read discharged nothing: {:?}",
        measured.diagnostics
    );
    assert!(
        unsupported(&measured).is_empty(),
        "the read itself is admitted: {:?}",
        measured.diagnostics
    );
}

/// Adversarial row 1b — **read after the link is cleared.** Storing a null
/// pointer unlinks the member, so a later read denotes nothing this body
/// tracks: it is refused rather than resolved to the stale identity.
#[test]
fn a_read_after_the_member_is_cleared_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
struct r_box *r_cleared(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) { return 0; } \
    b = r_make(n); \
    if (!b) { r_release(it); return 0; } \
    b->item = it; \
    b->item = 0; \
    r_probe(b->item); \
    return b; \
}";
    let measured = summary_of(body, "r_cleared");
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "an unlinked member carries no identity to read: {:?}",
        measured.diagnostics
    );
    assert!(
        reports_unresolved(&measured, "b"),
        "the aggregate is reported unresolved: {:?}",
        measured.diagnostics
    );
}

/// Adversarial row 2 — **read after release of the linked child.** This is card
/// C00's lesson in the read direction: the member is left pointing at an
/// identity this body already released, so the read is refused and can never
/// reach `Safe`.
#[test]
fn a_read_of_a_released_linked_child_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
struct r_box *r_dangling(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) { return 0; } \
    b = r_make(n); \
    if (!b) { r_release(it); return 0; } \
    b->item = it; \
    r_release(it); \
    r_probe(b->item); \
    return b; \
}";
    let measured = summary_of(body, "r_dangling");
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "a read through a released member must never be admitted: {:?}",
        measured.diagnostics
    );
    assert!(
        reports_unresolved(&measured, "b"),
        "the aggregate is reported unresolved: {:?}",
        measured.diagnostics
    );
}

/// The same lesson one level up — **read through a released aggregate.**
#[test]
fn a_read_through_a_released_aggregate_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
int r_after_free(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) { return 0; } \
    b = r_make(n); \
    if (!b) { r_release(it); return 0; } \
    b->item = it; \
    r_drop(b); \
    r_probe(b->item); \
    return 0; \
}";
    let measured = summary_of(body, "r_after_free");
    assert_ne!(measured.status, AnalysisStatus::Safe);
    assert!(
        reports_unresolved(&measured, "b"),
        "the released aggregate is reported unresolved: {:?}",
        measured.diagnostics
    );
}

/// Adversarial row 3 — **read through an aliased base.** A second declared name
/// for the same aggregate refuses the base outright (store premise 5), so both
/// the store and the read keep their existing `Unknown` and diagnostic.
#[test]
fn a_read_through_an_aliased_base_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
void r_aliased(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    struct r_box *c; \
    b = r_make(n); \
    if (!b) { return; } \
    c = b; \
    it = r_take(n); \
    if (!it) { r_drop(b); return; } \
    b->item = it; \
    r_probe(b->item); \
    r_untracked(c); \
}";
    let measured = summary_of(body, "r_aliased");
    assert_ne!(measured.status, AnalysisStatus::Safe);
    assert!(
        unsupported(&measured).contains(&INDIRECT),
        "the aliased base keeps the existing diagnostic: {:?}",
        measured.diagnostics
    );
}

/// Adversarial row 4 — **read of a different member.** A member this body never
/// stored into carries no field link, so reading it is refused even though the
/// aggregate and a sibling member are both perfectly resolved.
#[test]
fn a_read_of_an_unlinked_member_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
struct r_box *r_sibling(int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) { return 0; } \
    b = r_make(n); \
    if (!b) { r_release(it); return 0; } \
    b->item = it; \
    r_probe(b->spare); \
    return b; \
}";
    let measured = summary_of(body, "r_sibling");
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "an unlinked sibling member carries no identity: {:?}",
        measured.diagnostics
    );
    assert!(
        reports_unresolved(&measured, "b"),
        "the aggregate is reported unresolved: {:?}",
        measured.diagnostics
    );
}

/// `O04-READ-BOUND-CUSTODY`, **deliberately superseded** by card O04-B09B10 and
/// re-pinned at its new boundary.
///
/// This card refused every custody-bearing position outright, because a
/// contracted callee that merely *borrows* the aggregate could rewrite the
/// member invisibly and this projection maintained no frame condition for that.
/// Card O04-B09B10 supplies that frame condition as premise **F** — no
/// contracted call of the body may leave the aggregate both live and
/// rewritable — and admits `Released` and `Consumed` positions for bases it
/// insulates. `Owned`, `Stored`, `Removed` and `Out` still refuse, and so does
/// any base premise F does not insulate.
///
/// Both halves are asserted here on this card's own `r_*` family. The whole
/// mode vocabulary and the adversarial matrix are asserted in
/// `crates/qcore-c-gateway/tests/o04_b09b10.rs`.
#[test]
fn a_custody_bearing_position_is_admitted_only_for_an_insulated_base() {
    if !clang_available() {
        return;
    }
    // A borrowing sink over the *aggregate*, declared locally so that this
    // card's fixture contract file stays byte-unchanged. It is the shape
    // premise F refuses: a callee holding the box could rewrite `b->item`.
    let declarations = format!("{DECLARATIONS} int r_hold(struct r_box *b);");
    let contracts = format!(
        "{GENERIC_CONTRACTS}\n\
extern fn r_hold(b: ptr<mut, u8>) -> i32\n\
contract {{\n\
    requires resource_contract_v1;\n\
    ensures normal resource_borrowed(b);\n\
    ensures normal resource_return_none();\n\
}};\n"
    );
    let cases: [(&str, &str); 2] = [
        ("a releasing position", "r_reap(b->item);"),
        ("a consuming position", "r_adopt(b->item);"),
    ];
    for (reason, call) in cases {
        for (exposure, insulated) in [("", true), ("r_hold(b);", false)] {
            let body = format!(
                "void r_w(int n) \
{{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) {{ return; }} \
    b = r_make(n); \
    if (!b) {{ r_release(it); return; }} \
    b->item = it; \
    {exposure} \
    {call} \
    r_drop(b); \
}}"
            );
            let source = format!("{declarations} {body}");
            let result = analyze_unrestricted_c11(
                &C11TranslationUnit {
                    source: &source,
                    ..Default::default()
                },
                "r_w",
                &load("o04_read_bound_generic.qcore", &contracts),
            )
            .expect("generic fixture accepted by Clang");
            let measured = match &result {
                UnrestrictedLegacyResult::Safe(certificate) => certificate
                    .summary
                    .functions
                    .get("r_w")
                    .expect("entry present")
                    .clone(),
                _ => function(program(&result), "r_w").clone(),
            };
            if insulated {
                assert_eq!(
                    measured.status,
                    AnalysisStatus::Safe,
                    "{reason}: an insulated base transfers the linked child, diagnostics {:?}",
                    measured.diagnostics
                );
            } else {
                assert_ne!(
                    measured.status,
                    AnalysisStatus::Safe,
                    "{reason}: an exposed base must stay refused, diagnostics {:?}",
                    measured.diagnostics
                );
                assert!(
                    unsupported(&measured).contains(&INDIRECT),
                    "{reason}: the existing diagnostic must be retained, got {:?}",
                    measured.diagnostics
                );
            }
        }
    }
}

/// Every member shape the reader refuses keeps its existing `Unknown` **and**
/// its existing diagnostic, at a borrowed position. Nothing here is a new code.
#[test]
fn every_refused_member_shape_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 7] = [
        ("a void pointer member", "r_probe(b->any);"),
        ("a double pointer member", "r_probe(b->deep);"),
        ("an array member", "r_probe(b->fixed);"),
        ("a qualified member", "r_probe(b->hot);"),
        ("a chained access", "r_probe(b->item->next);"),
        // `O04-READ-BOUND-SCALAR`: a scalar member can never hold an identity,
        // and card C02's scalar route refuses a base this body binds, so a
        // scalar member read through a bound base keeps its refusal in both.
        ("a scalar member", "r_count(b->flag);"),
        // Not a whole argument: the lexical gate governs the rest unchanged.
        ("a partial argument", "r_count(b->item->weight + n);"),
    ];
    for (reason, call) in cases {
        let body = format!(
            "void r_w(int n) \
{{ \
    struct r_item *it; \
    struct r_box *b; \
    it = r_take(n); \
    if (!it) {{ return; }} \
    b = r_make(n); \
    if (!b) {{ r_release(it); return; }} \
    b->item = it; \
    {call} \
    r_release(it); \
    r_drop(b); \
}}"
        );
        let measured = summary_of(&body, "r_w");
        // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS moves these rows
        // deliberately. Its call argument rule reads a pointer member at a
        // `Borrowed` position as a typed pointer actual, and its second
        // obligation refuses it because `b` owns the field link `b->item`
        // (`UnknownAlias` of `b` in place of the gate's diagnostic). A scalar
        // member of the live `b` is data, never an identity, and is admitted
        // under `b`'s dereference obligation.
        // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d) adds "a chained access"
        // deliberately: the pointer read `b->item->next` on a root the
        // closure binds is left to the flow, which reads a deferred actual's
        // root only as untracked, so the tracked `b` refuses there
        // (`UnknownAlias` of `b` in place of the gate's diagnostic).
        const OBLIGATED: &[&str] = &[
            "a void pointer member",
            "a double pointer member",
            "a chained access",
        ];
        // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins "a partial
        // argument" deliberately: the Clang-typed call-actual fallback reads
        // the scalar value `b->item->weight + n` whole, with the chain's
        // per-link obligation, which the live child `it` stored at `b->item`
        // discharges. The lexical gate and this rule still refuse it.
        const ADMITTED: &[&str] = &["a scalar member", "a partial argument"];
        if ADMITTED.contains(&reason) {
            assert!(
                measured.diagnostics.is_empty(),
                "{reason}: admitted under the base's obligation, got {:?}",
                measured.diagnostics
            );
            continue;
        }
        assert_ne!(
            measured.status,
            AnalysisStatus::Safe,
            "{reason}: must stay refused, diagnostics {:?}",
            measured.diagnostics
        );
        if OBLIGATED.contains(&reason) {
            assert!(
                measured.diagnostics.iter().any(|diagnostic| matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::UnknownAlias { variable } if variable == "b"
                )),
                "{reason}: the second obligation must refuse `b`, got {:?}",
                measured.diagnostics
            );
            continue;
        }
        assert!(
            unsupported(&measured).contains(&INDIRECT),
            "{reason}: the existing diagnostic must be retained, got {:?}",
            measured.diagnostics
        );
    }
}

/// The two read routes are **disjoint**, measured rather than argued.
///
/// A caller-owned aggregate carries no identity this body binds, so its member
/// read belongs to card C02-P and is admitted there — unchanged, and it stays
/// `Safe` exactly as it did before this card. This card's route records nothing
/// for it, so no bound-read revalidation applies. The `R_CREATE` body is the
/// mirror image: its base *is* bound, C02-P refuses it, and this card's route
/// records the place.
#[test]
fn the_two_read_routes_are_disjoint() {
    if !clang_available() {
        return;
    }
    let caller_owned = summary_of("void r_use(struct r_box *b) { r_probe(b->item); }", "r_use");
    assert_eq!(
        caller_owned.status,
        AnalysisStatus::Safe,
        "card C02-P's admission is unchanged: {:?}",
        caller_owned.diagnostics
    );
    assert!(
        bound_reads(&caller_owned).is_empty(),
        "an untracked base is not this card's route: {:?}",
        bound_reads(&caller_owned)
    );
    let bound = summary_of(R_CREATE, "r_create");
    assert_eq!(
        bound_reads(&bound),
        vec!["b->item".to_owned(), "b->raw".to_owned()],
        "a base this body bound is exactly this card's route"
    );
}

/// The recorded places every admitted bound read of one function names.
fn bound_reads(summary: &LegacyFunctionSummary) -> Vec<String> {
    let mut places = summary
        .cfg
        .nodes
        .iter()
        .filter_map(|node| match &node.instruction {
            LegacyInstruction::ContractCall {
                aggregate_reads, ..
            } => Some(aggregate_reads.clone()),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    places.sort();
    places
}

/// An aggregate read out of another object's memory is refused (store premise
/// 3), so nothing is ever admitted through it — read or store.
#[test]
fn a_read_through_an_aggregate_read_out_of_memory_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
void r_w(struct r_outer *o, int n) \
{ \
    struct r_item *it; \
    struct r_box *b; \
    b = o->box; \
    it = r_take(n); \
    if (!it) { return; } \
    b->item = it; \
    r_probe(b->item); \
}";
    let measured = summary_of(body, "r_w");
    assert_ne!(measured.status, AnalysisStatus::Safe);
    assert!(
        unsupported(&measured).contains(&INDIRECT),
        "the base is not one this body acquired: {:?}",
        measured.diagnostics
    );
}

/// W4 derivation, W4 validation and the independent certificate recheck all
/// repeat this classification from the certificate's own content: substituting a
/// member or base declaration in the certificate's source withdraws the
/// projection.
#[test]
fn a_certificate_resting_on_a_bound_read_is_rederived_from_source() {
    if !clang_available() {
        return;
    }
    let certificate = certified(R_CREATE, "r_create");
    let substitutions: [(&str, &str, &str); 5] = [
        (
            "the read member becomes a double pointer",
            "struct r_item *item;",
            "struct r_item **item;",
        ),
        (
            "the read member becomes an array",
            "struct r_item *item;",
            "struct r_item *item[2];",
        ),
        (
            "the read member becomes volatile",
            "struct r_item *item;",
            "volatile struct r_item *item;",
        ),
        (
            "the aggregate record becomes a union",
            "struct r_box { int flag;",
            "union r_box { int flag;",
        ),
        (
            "the acquired member's pointee stops resolving",
            "r_word *raw;",
            "void *raw;",
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

/// The admission rests on the declared **contract mode** carried in the
/// certificate, not only on the declarations: substituting the borrowing sink's
/// mode withdraws the projection.
///
/// Card O04-B09B10 re-pinned which substitutions do that. Under this card every
/// custody-bearing mode withdrew it; that card admits `Released` and `Consumed`
/// for a base premise **F** insulates, and `b` here is one — its only whole
/// occurrence in a call argument is `r_drop(b)`, a terminal position. The other
/// custody-bearing modes still withdraw the projection, and `Owned` is what
/// this test now pins at the source level; the whole mode vocabulary is pinned
/// without Clang by `aggregate_store`'s
/// `only_released_and_consumed_positions_take_custody_of_a_bound_read`.
#[test]
fn a_certificate_resting_on_a_bound_read_rests_on_the_declared_mode() {
    if !clang_available() {
        return;
    }
    let borrowed = "extern fn r_probe(i: ptr<const, u8>) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(i);";
    {
        let mode = "resource_owned(i, nonnull)";
        let contracts = GENERIC_CONTRACTS.replace("\r\n", "\n");
        let weakened = contracts.replace(
            borrowed,
            &format!(
                "extern fn r_probe(i: ptr<const, u8>) -> i32\ncontract {{\n    requires resource_contract_v1;\n    ensures normal {mode};"
            ),
        );
        assert_ne!(weakened, contracts, "the substitution applied");
        let source = format!("{DECLARATIONS} {R_CREATE}");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "r_create",
            &load("o04_read_bound_generic.qcore", &weakened),
        )
        .expect("fixture accepted by Clang");
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "`{mode}` withdraws the admission: {result:?}"
        );
    }
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
        &load("o04_read_bound_path.qcore", &path_contracts()),
    )
    .unwrap_or_else(|error| panic!("O04-READ-BOUND fixture rejected by Clang: {error:?}"))
}

/// The card's headline on real source, stated honestly in **both** directions:
/// the named blocker `O04-B08-READ-BOUND-BASE` is gone from
/// `flb_routes_mask_set_by_tag(ic->routes_mask, …)`, and the success exit C6 is
/// **still `Unknown`** on its other blockers. The five exits O04-B08 settled
/// stay settled and nothing new is discharged.
#[test]
fn the_named_read_blocker_is_gone_and_the_success_exit_is_still_unknown() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let shape = exit_shape(caller);

    // The refusal this card removes is gone, and it is gone from that exact
    // statement: no `Unsupported` diagnostic remains on the routes-mask read's
    // line.
    let read_line = preprocessed(&result)
        .lines()
        .position(|line| line.trim_start().starts_with(READ_BOUND_SITE))
        .map(|index| index + 1)
        .expect("the routes-mask read is present in the preprocessed source");
    assert!(
        !caller.diagnostics.iter().any(|diagnostic| {
            diagnostic.line == read_line
                && matches!(diagnostic.kind, LegacyDiagnosticKind::Unsupported { .. })
        }),
        "the bound read is admitted: {:?}",
        caller.diagnostics
    );

    // And nothing else moved: exactly the exits O04-B08 settled.
    //
    // Composed-tree reconciliation (LOADER-ABI × NEIGHBOURS-CHAINED-TAIL): back
    // to the five O04-B08 settled. Card LOADER-ABI's measurement alone dropped
    // the logging arm C1 from this set when the fixture's `flb_error` was
    // corrected against C's `(const char *, ...)`; clause T5 admits the chained
    // tail actual at the declared data tail that correction creates, so C1
    // settles again. See
    // `o04_arms.rs::the_declaration_decides_the_same_chain_in_a_variadic_tail`
    // for the composed grounds and both cards' retained negative halves.
    assert_eq!(
        caller
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged)
            .count(),
        5,
        "the settled arms are unchanged; exits were {shape:?}"
    );
    assert_eq!(caller.status, AnalysisStatus::Unknown, "no promotion");
    let unresolved = caller
        .exits
        .iter()
        .filter(|exit| exit.custody == ExitCustody::Unknown)
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved.len(),
        1,
        "the success exit C6 is still unresolved: {shape:?}"
    );
    // Kept from LOADER-ABI's reconciliation of this pin: the unresolved exit is
    // told apart by what it *carries*, not by its position, so a site-free
    // unresolved exit could not rejoin this set unnoticed.
    let carrying = unresolved
        .iter()
        .filter(|exit| !exit.sites.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(carrying.len(), 1, "only C6 carries identities: {shape:?}");
    assert!(
        carrying[0]
            .sites
            .iter()
            .all(|site| site.disposition == SiteDisposition::Mixed),
        "C6 still carries every identity unresolved: {shape:?}"
    );
    assert!(
        caller
            .exits
            .iter()
            .all(|exit| exit.custody != ExitCustody::Residual),
        "no residual is alleged on this path: {shape:?}"
    );
    assert!(
        leaks(caller).is_empty(),
        "no leak is alleged: {:?}",
        caller.diagnostics
    );
}

/// The other half: C6's residual is exactly the blockers this card does **not**
/// touch, enumerated so a silent change in either direction fails.
#[test]
fn the_success_exit_keeps_its_named_residual() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let caller = function(program(&result), "flb_input_chunk_create");
    let remaining = unsupported(caller);
    assert!(
        !remaining.is_empty(),
        "the untouched blockers are still reported"
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
    // The address-of-a-member, list, hash-table and mutex statements are all
    // still refused: eight `INDIRECT` refusals remain on C6's path, one fewer
    // than before this card (the routes-mask read).
    //
    // Composed-tree reconciliation: back to eight. The ninth card LOADER-ABI
    // recorded here was never on C6's path and was never this card's — it was
    // the `flb_error` chained tail actual on the `!chunk` arm, refused while
    // that corrected declaration had no rule to admit it. Clause T5 of card
    // NEIGHBOURS-CHAINED-TAIL admits it at the declared data tail, so this
    // count is again exactly B09/B10's territory.
    //
    // Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule
    // 12k): each of the eight is a gate refusal whose callee this fixture
    // leaves uncontracted (`msgpack_packer_init`, `mk_list_add`, four
    // `flb_hash_table_add`, `pthread_mutex_lock`/`_unlock`), with actuals W44
    // reads at a `Borrowed` position, so each now names its callee. The same
    // eight statements stay refused; none reads the gate's text any more.
    assert_eq!(
        remaining
            .iter()
            .filter(|description| **description == INDIRECT)
            .count(),
        0,
        "B09/B10 territory is untouched: {:?}",
        caller.diagnostics
    );
    assert_eq!(
        remaining
            .iter()
            .filter(
                |description| description.starts_with("call to uncontracted function `")
                    && **description != "call to uncontracted function `flb_trace`"
            )
            .count(),
        8,
        "B09/B10 territory is untouched: {:?}",
        caller.diagnostics
    );
}

/// The real read rests on the same declared premises as the generic family:
/// substituting the read member's declaration withdraws the admission and the
/// refusal returns.
#[test]
fn substituting_the_read_member_declaration_restores_the_refusal() {
    if !clang_available() {
        return;
    }
    let substitutions: [(&str, &str, &str); 2] = [
        (
            "the read member becomes a double pointer",
            "flb_route_mask_element *routes_mask;",
            "flb_route_mask_element **routes_mask;",
        ),
        (
            "the read member becomes volatile",
            "flb_route_mask_element *routes_mask;",
            "volatile flb_route_mask_element *routes_mask;",
        ),
    ];
    for (reason, from, to) in substitutions {
        let unit = real_unit();
        assert!(unit.contains(from), "{reason}: `{from}` is present");
        // Preserve valid C for both mutated member types without casting the
        // read itself: the gateway must still inspect the original member.
        let changed = unit.replacen(from, to, 1);
        let allocation = "ic->routes_mask = (flb_route_mask_element *)";
        let signature = "int flb_routes_mask_set_by_tag(flb_route_mask_element *routes_mask,";
        assert!(
            changed.contains(allocation),
            "the allocation anchor is present"
        );
        assert!(changed.contains(signature), "the callee anchor is present");
        let changed = changed
            .replace(allocation, "ic->routes_mask = (void *)")
            .replace(
                signature,
                "int flb_routes_mask_set_by_tag(const volatile void *routes_mask,",
            );
        let result = analyze_real(&changed);
        let caller = function(program(&result), "flb_input_chunk_create");
        // Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
        // (rule 12k): a gate refusal whose uncontracted callee decides now
        // names that callee, so the refusals that return are counted under
        // either text. Both are the gate's refusal of the statement.
        assert!(
            unsupported(caller)
                .iter()
                .filter(|description| **description == INDIRECT
                    || description.starts_with("call to uncontracted function `"))
                .count()
                > 8,
            "{reason}: the refusal must return, diagnostics {:?}",
            caller.diagnostics
        );
    }
}

/// The constructor's own boundary is untouched by this card: it reads
/// `ch->name` only at a *releasing* position, which `O04-READ-BOUND-CUSTODY`
/// deliberately refuses, so its exits are byte-for-byte what O04-B08 measured.
#[test]
fn the_constructor_boundary_is_unchanged() {
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
        assert_eq!(sites.len(), 2, "{shape:?}");
        assert!(
            sites
                .iter()
                .all(|disposition| *disposition == SiteDisposition::Mixed),
            "both stay unresolved: {shape:?}"
        );
    }
    assert!(
        unsupported(constructor).contains(&INDIRECT),
        "`free(ch->name)` keeps its refusal: {:?}",
        constructor.diagnostics
    );
    assert!(
        leaks(constructor).is_empty(),
        "nothing is alleged in the defect direction: {:?}",
        constructor.diagnostics
    );
}

fn preprocessed(result: &UnrestrictedLegacyResult) -> String {
    match result {
        UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            ..
        } => preprocessed_source.clone(),
        other => panic!("expected a partial result carrying its source: {other:?}"),
    }
}
