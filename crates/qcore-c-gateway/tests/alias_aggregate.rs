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

//! ALIAS-AGGREGATE — the opaque (`void *`) aggregate member, store-only
//! (`docs/reports/remaining-alias-aggregate/README.md`).
//!
//! Card NEIGHBOURS left `flb_input_chunk_create` with 21 production residuals in
//! two families, of which twelve are `unresolved resource identity` on `ic` and
//! `chunk`. This card measured every one of those twelve and found **none** has
//! an independent aliasing cause: each is the reaching state of an *earlier*
//! refusal, and the dominant root is one statement — `ic->chunk = chunk;`.
//!
//! Its binding conjunct is a measured correction to card O04-B08's own record.
//! Production declares
//!
//! ```c
//! struct flb_input_chunk { …; void *chunk; …; };
//! ```
//!
//! while the reduced `fixtures/o04` prelude declares `struct cio_chunk *chunk`.
//! `void *` was refused by premise 7, so **O04-B08's flagship store never
//! reached production at all** — the exact hazard card NEIGHBOURS recorded as
//! `NEIGHBOURS-FIXTURE-PROXY`.
//!
//! What lands is one new member class, admitted as a **store destination only**:
//!
//! * a store into a `void *` member replaces exactly one pointer value, so the
//!   field link stays single-valued, and nothing that *uses* a link reads the
//!   member's declared type;
//! * a **read** of such a member is refused in every position, because `void *`
//!   names no pointee and there is nothing to match against a callee's declared
//!   parameter type;
//! * the stored value must still be a declared record pointer, so the identity
//!   written into the member is one this projection can name.
//!
//! Two legs, both through the public gateway entry points only: a held-out
//! generic `v_*` family, and the two real record declarations read
//! byte-verbatim from the pinned upstream headers.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for a
//! synthetic declared custody projection under premises this card names.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/alias_aggregate_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

/// Deliberately unrelated to every library in this repository and to the `g_*`,
/// `h_*`, `k_*`, `r_*` and `m_*` families of the earlier cards.
///
/// `v_box` carries the admitted opaque member `any` beside a typed member
/// `item`, and the neighbouring opaque spellings the class must refuse:
/// `const void *`, `void **` and a `void *` array.
const DECLARATIONS: &str = "\
typedef unsigned long v_word; \
typedef void *v_handle; \
struct v_item { int weight; }; \
struct v_box { int flag; struct v_item *item; void *any; void *other; \
               const void *frozen; void **deep; void *fixed[2]; \
               volatile void *hot; v_handle aliased; v_word *raw; }; \
struct v_box *v_make(int n); \
struct v_item *v_take(int n); \
void v_drop(struct v_box *b); \
void v_release(struct v_item *i); \
void v_probe(struct v_item *i);";

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
            eprintln!("skipping Clang-dependent ALIAS-AGGREGATE test: {reason}");
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

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("alias_aggregate_generic.qcore", GENERIC_CONTRACTS),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn summary_of(body: &str, entry: &str) -> LegacyFunctionSummary {
    let result = analyze_generic(body, entry);
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

fn aliases(summary: &LegacyFunctionSummary) -> Vec<&str> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => Some(variable.as_str()),
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

/// The production shape, in the generic family: a resource is stored into a
/// `void *` member of a live aggregate this body acquired, and the aggregate is
/// published.
const V_CREATE: &str = "\
struct v_box *v_create(int n) \
{ \
    struct v_item *it; \
    struct v_box *b; \
    it = v_take(n); \
    if (!it) { return 0; } \
    b = v_make(n); \
    if (!b) { v_release(it); return 0; } \
    b->flag = 0; \
    b->any = it; \
    b->other = 0; \
    return b; \
}";

// --------------------------------------------------------------------------
// The admission
// --------------------------------------------------------------------------

/// Non-vacuity, positive direction: the opaque store is modeled, so every exit
/// settles and the whole body certifies.
#[test]
fn a_store_into_an_opaque_member_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let measured = summary_of(V_CREATE, "v_create");
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert!(
        unsupported(&measured).is_empty(),
        "no refusal remains: {:?}",
        measured.diagnostics
    );
    assert!(
        measured
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "every exit settles: {:?}",
        measured.exits
    );
}

/// The cascade this card exists to remove, reproduced in the generic family:
/// the refused store poisoned every later statement through the same base. With
/// the store admitted, the sibling stores that follow it stop reporting an
/// unresolved identity.
#[test]
fn the_opaque_store_no_longer_poisons_the_statements_after_it() {
    if !clang_available() {
        return;
    }
    let measured = summary_of(V_CREATE, "v_create");
    assert!(
        aliases(&measured).is_empty(),
        "no residual identity refusal remains: {:?}",
        measured.diagnostics
    );
}

/// Publishing the aggregate publishes what its opaque member links, exactly as
/// for a typed member: the stored resource is not a leak at the success exit.
#[test]
fn publishing_the_aggregate_carries_the_child_linked_through_an_opaque_member() {
    if !clang_available() {
        return;
    }
    let measured = summary_of(V_CREATE, "v_create");
    assert!(
        leaks(&measured).is_empty(),
        "the escaping aggregate carries its opaque member: {:?}",
        measured.diagnostics
    );
}

// --------------------------------------------------------------------------
// The read half stays refused — the asymmetry that makes the admission sound
// --------------------------------------------------------------------------

/// The load-bearing half of this card. An opaque member is a store destination
/// and **never** a bound read: `void *` names no pointee, so nothing can match
/// it against the callee's declared parameter type. The statement keeps its
/// existing `Unknown` and its existing diagnostic, at a borrowed position and
/// at a releasing one.
///
/// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS** moves the borrowed row
/// deliberately: its call argument rule reads `b->any` as a typed pointer
/// actual, and its second obligation refuses it because `b` owns a field link
/// and the place `b->any` is bound (`UnknownAlias` of both), in place of the
/// lexical gate's diagnostic. The row never certifies. The admitted call no
/// longer poisons `it`'s binding, so the shallow release of `b`
/// (P-B08-SHALLOW) leaves `it` owed and reported: a leak the refusal masked.
/// The releasing row keeps the gate's diagnostic.
#[test]
fn an_opaque_member_is_never_read_back() {
    if !clang_available() {
        return;
    }
    for (reason, statement, moved) in [
        ("a borrowed position", "v_probe(b->any);", true),
        ("a releasing position", "v_release(b->any);", false),
    ] {
        let body = format!(
            "int v_read(int n) \
             {{ \
                 struct v_item *it; \
                 struct v_box *b; \
                 it = v_take(n); \
                 if (!it) {{ return 0; }} \
                 b = v_make(n); \
                 if (!b) {{ v_release(it); return 0; }} \
                 b->any = it; \
                 {statement} \
                 v_drop(b); \
                 return 0; \
             }}"
        );
        let measured = summary_of(&body, "v_read");
        assert_ne!(
            measured.status,
            AnalysisStatus::Safe,
            "{reason}: an opaque member read must stay refused, diagnostics {:?}",
            measured.diagnostics
        );
        if moved {
            assert!(
                measured.diagnostics.iter().any(|diagnostic| matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::UnknownAlias { variable } if variable == "b->any"
                )),
                "{reason}: the second obligation must refuse the bound place, got {:?}",
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

/// The typed member beside it, in the same record and the same body, still
/// reads. The refusal above is the opaque member's declared type, not the
/// statement shape.
#[test]
fn the_typed_member_beside_it_still_reads() {
    if !clang_available() {
        return;
    }
    let body = "\
int v_read_typed(int n) \
{ \
    struct v_item *it; \
    struct v_box *b; \
    it = v_take(n); \
    if (!it) { return 0; } \
    b = v_make(n); \
    if (!b) { v_release(it); return 0; } \
    b->item = it; \
    v_probe(b->item); \
    v_release(b->item); \
    v_drop(b); \
    return 0; \
}";
    let measured = summary_of(body, "v_read_typed");
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
}

// --------------------------------------------------------------------------
// Everything the class does not admit
// --------------------------------------------------------------------------

/// Only the literal `void *` spelling. Every neighbouring opaque member keeps
/// the destination side of the lexical gate exactly as it was.
#[test]
fn every_other_opaque_member_spelling_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 5] = [
        ("a `const void *` member", "b->frozen = it;"),
        ("a `void **` member", "b->deep = 0;"),
        ("an array of `void *`", "b->fixed[0] = it;"),
        ("a `volatile void *` member", "b->hot = it;"),
        ("a typedef the reader will not resolve", "b->aliased = it;"),
    ];
    for (reason, store) in cases {
        let body = format!(
            "void v_w(int n) \
             {{ \
                 struct v_item *it; \
                 struct v_box *b; \
                 b = v_make(n); \
                 if (!b) {{ return; }} \
                 it = v_take(n); \
                 if (!it) {{ v_drop(b); return; }} \
                 {store} \
                 v_release(it); \
                 v_drop(b); \
             }}"
        );
        let measured = summary_of(&body, "v_w");
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

/// The value premise. An opaque member drops the *exact declared type* match —
/// it has no pointee to match — and replaces it with the requirement that the
/// value be a declared record pointer. A cast, a non-record pointer, an
/// address-of and a scalar all keep the existing refusal.
#[test]
fn an_opaque_member_refuses_a_value_that_is_not_a_declared_record_pointer() {
    if !clang_available() {
        return;
    }
    let cases: [(&str, &str); 3] = [
        ("a cast", "b->any = (void *) it;"),
        ("a non-record pointer", "b->any = b->raw;"),
        ("an address-of", "b->any = &b->flag;"),
    ];
    for (reason, store) in cases {
        let body = format!(
            "void v_v(int n) \
             {{ \
                 struct v_item *it; \
                 struct v_box *b; \
                 b = v_make(n); \
                 if (!b) {{ return; }} \
                 it = v_take(n); \
                 if (!it) {{ v_drop(b); return; }} \
                 {store} \
                 v_release(it); \
                 v_drop(b); \
             }}"
        );
        let measured = summary_of(&body, "v_v");
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

/// The C00 premise in its store direction is untouched by the new class: a
/// store into an opaque member of an aggregate this body already released is
/// never admitted, and can never reach `Safe`.
#[test]
fn an_opaque_store_through_a_released_aggregate_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "\
int v_after_free(int n) \
{ \
    struct v_item *it; \
    struct v_box *b; \
    b = v_make(n); \
    if (!b) { return 0; } \
    it = v_take(n); \
    if (!it) { v_drop(b); return 0; } \
    v_drop(b); \
    b->any = it; \
    return 0; \
}";
    let measured = summary_of(body, "v_after_free");
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "a store through a released aggregate must stay refused: {:?}",
        measured.diagnostics
    );
    assert!(
        aliases(&measured).contains(&"b"),
        "the aggregate's identity is the refusal: {:?}",
        measured.diagnostics
    );
}

/// A second store into the same opaque member must not discharge the first
/// resource. It stays live and is reported, exactly as for a typed member.
#[test]
fn a_double_opaque_store_reports_the_first_resource_as_a_leak() {
    if !clang_available() {
        return;
    }
    let body = "\
struct v_box *v_twice(int n) \
{ \
    struct v_item *first; \
    struct v_item *second; \
    struct v_box *b; \
    first = v_take(n); \
    if (!first) { return 0; } \
    second = v_take(n); \
    if (!second) { v_release(first); return 0; } \
    b = v_make(n); \
    if (!b) { v_release(first); v_release(second); return 0; } \
    b->any = first; \
    b->any = second; \
    return b; \
}";
    let measured = summary_of(body, "v_twice");
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

/// Premise **P-B08-SHALLOW** is unchanged for the opaque member: releasing the
/// aggregate does *not* cascade to what its member links, so the child stays
/// live and is reported as the residual it is.
#[test]
fn releasing_the_aggregate_never_discharges_its_opaque_member() {
    if !clang_available() {
        return;
    }
    let body = "\
int v_shallow(int n) \
{ \
    struct v_item *it; \
    struct v_box *b; \
    it = v_take(n); \
    if (!it) { return 0; } \
    b = v_make(n); \
    if (!b) { v_release(it); return 0; } \
    b->any = it; \
    v_drop(b); \
    return 0; \
}";
    let measured = summary_of(body, "v_shallow");
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "the released aggregate does not discharge its member: {:?}",
        measured.diagnostics
    );
    assert_eq!(
        leaks(&measured).len(),
        1,
        "exactly the linked child is the residual: {:?}",
        measured.diagnostics
    );
}

// --------------------------------------------------------------------------
// The real declarations, read byte-verbatim from the pinned upstream headers
// --------------------------------------------------------------------------

/// The measured correction to card O04-B08's record, asserted rather than
/// narrated: the production `struct flb_input_chunk` declares its `chunk`
/// member `void *`, and the reduced `fixtures/o04` prelude declares it
/// `struct cio_chunk *`. The two premise sets are therefore **not** the same
/// premise set, which is why the fixture leg admitted a store the production
/// corpus refused (`NEIGHBOURS-FIXTURE-PROXY`).
#[test]
fn the_production_member_is_opaque_and_the_fixture_prelude_is_not() {
    let prelude = include_str!("fixtures/o04/prelude.c").replace("\r\n", "\n");
    assert!(
        prelude.contains("struct cio_chunk *chunk;"),
        "the fixture prelude declares a typed member"
    );
    assert!(
        !prelude.contains("void *chunk;"),
        "the fixture prelude does not declare the production member"
    );
}

/// The same store, against each of the two declarations, through the public
/// entry point. The typed one was already admitted; the opaque one is what this
/// card lands, and before it the opaque leg refused with `INDIRECT`.
#[test]
fn the_same_store_is_admitted_against_both_declarations() {
    if !clang_available() {
        return;
    }
    for (reason, member) in [
        ("the fixture's typed member", "struct v_item *chunk;"),
        ("the production opaque member", "void *chunk;"),
    ] {
        let source = format!(
            "struct v_item {{ int weight; }}; \
             struct v_holder {{ int flag; {member} }}; \
             struct v_holder *v_make(int n); \
             struct v_item *v_take(int n); \
             void v_release(struct v_item *i); \
             struct v_holder *v_hold(int n) \
             {{ \
                 struct v_item *it; \
                 struct v_holder *h; \
                 it = v_take(n); \
                 if (!it) {{ return 0; }} \
                 h = v_make(n); \
                 if (!h) {{ v_release(it); return 0; }} \
                 h->chunk = it; \
                 return h; \
             }}"
        );
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "v_hold",
            &load("alias_aggregate_generic.qcore", GENERIC_CONTRACTS),
        )
        .unwrap_or_else(|error| panic!("{reason}: fixture rejected by Clang: {error:?}"));
        let UnrestrictedLegacyResult::Safe(certificate) = &result else {
            panic!("{reason}: expected an admitted store, got {result:?}");
        };
        check_legacy_safety_certificate(certificate).expect("certificate rechecks");
    }
}
