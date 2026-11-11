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

//! NEIGHBOURS-CHAINED-TAIL — a bounded typed access chain as a **variadic data
//! tail** actual (`docs/reports/remaining-neighbours-chained-tail/README.md`).
//!
//! Card NEIGHBOURS measured one surviving refusal on the byte-verbatim upstream
//! `flb_input_chunk_create`: an `indirect memory access` on the `flb_error` arm
//! of the `!chunk` exit, whose whole cause is the tail actual
//! `storage->stream->name`. The same refusal is the corpus's `flb_input_chunk.c`
//! L19581.
//!
//! **A correction this card measured, stated first.** NEIGHBOURS §3.4 attributed
//! the refusal to the chain rule not reaching a *pointer-typed final field*.
//! That is not the mechanism: `TypedAccessModel::borrowable_field` has admitted a
//! pointer member since card C02-P, and the depth-one **scalar** access
//! `storage->flag` refused in a tail position identically (measured, and pinned
//! below as `a_depth_one_scalar_field_was_refused_in_a_tail_too`). The blocker
//! was premise **T2** of `variadic_nested::is_data_only_actual`, which refused
//! `&`, `*`, `[`, `]`, `.` and `->` outright in a tail actual — and, because a
//! tail actual's span was never claimed by any whitelist, the lexical
//! indirect-access gate fired ahead of the tail rule and reported the access.
//!
//! What this card admits is clause **T5**: a tail actual, at a position the
//! callee's declaration covers with `resource_variadic_data_tail()`, that is
//! **exactly** one admitted bounded typed access chain — the same predicate,
//! over the same declaration facts, that already decides a declared `Borrowed`
//! actual (cards C02-P and O04-ARMS).
//!
//! Everything here runs through the public gateway entry points over a held-out
//! generic `y_*` family. Nothing in this file is a production declaration.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const GENERIC: &str = include_str!("fixtures/contracts/chained_tail_generic.qv");

/// The held-out C declarations. Every record shape the premises separate is
/// present: a fully typed two-link chain, a four-link chain, a record-valued
/// base, a `union` link, a link into a record this unit never defines, a double
/// pointer member, an array member and a nested aggregate member.
const DECLARATIONS: &str = "\
struct y_leaf { char *label; int n; }; \
struct y_mid { int flag; struct y_leaf *leaf; struct y_mid *self; }; \
struct y_deep { struct y_mid *mid; }; \
struct y_deeper { struct y_deep *deep; }; \
struct y_top { struct y_deeper *deeper; }; \
union y_view { struct y_leaf *leaf; int tag; }; \
struct y_uholder { union y_view *view; }; \
struct y_bits { unsigned a : 3; char *label; }; \
struct y_bhold { struct y_bits *bits; }; \
typedef struct y_leaf *y_leafp; \
struct y_tdef { y_leafp leaf; }; \
struct y_val { int k; }; \
struct y_hold { struct y_val v; }; \
void *y_open(int n); \
void y_close(void *h); \
void y_log(const char *fmt, ...); \
void y_plain(const char *fmt, ...); \
void y_eat(void *h); \
void *y_id(void *h); \
void y_borrow(void *h);";

fn load(source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "chained_tail_generic.qcore".to_owned(),
        source.to_owned(),
    )]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent NEIGHBOURS-CHAINED-TAIL test: {reason}");
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
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        other => panic!("expected an analyzable summary: {other:?}"),
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

fn analyze(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load(&normalize_newlines(GENERIC)),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn summary(body: &str, entry: &str) -> LegacyFunctionSummary {
    function_of(&analyze(body, entry), entry)
}

fn function_of(result: &UnrestrictedLegacyResult, entry: &str) -> LegacyFunctionSummary {
    program(result)
        .functions
        .get(entry)
        .unwrap_or_else(|| panic!("missing function `{entry}`"))
        .clone()
}

/// The subject shape, generically: acquire, and on the failure arm log a tail
/// actual read out of an aggregate this body never binds. `ACTUAL` is
/// substituted per case.
fn probe_body(actual: &str) -> String {
    format!(
        "void *y_probe(struct y_mid *storage, struct y_top *top, \
             struct y_uholder *uh, struct y_bhold *bh, struct y_tdef *td, \
             struct y_hold *hold) {{ \
             struct y_mid value; \
             void *handle; \
             handle = y_open(8); \
             if (!handle) {{ y_log(\"failed %s\", {actual}); return 0; }} \
             return handle; \
         }}"
    )
}

fn probe(actual: &str) -> LegacyFunctionSummary {
    summary(&probe_body(actual), "y_probe")
}

// --------------------------------------------------------------------------
// The admission
// --------------------------------------------------------------------------

/// **THE HEADLINE.** The subject shape — a two-link chain whose final member is
/// pointer-typed, at a declared variadic data tail — is admitted, and the
/// function carries no refusal and no leak.
#[test]
fn a_two_link_pointer_tail_chain_is_admitted() {
    if !clang_available() {
        return;
    }
    let summary = probe("storage->leaf->label");
    assert_eq!(
        unsupported(&summary),
        Vec::<&str>::new(),
        "the subject shape must be admitted"
    );
    assert!(
        summary.diagnostics.is_empty(),
        "no diagnostic remains: {:?}",
        summary.diagnostics
    );
}

/// **The correction, pinned.** A depth-one *scalar* field — squarely inside card
/// C02's original whitelist, and nothing to do with a pointer-typed final field
/// — was refused in a tail position by exactly the same gate. Both directions
/// are asserted here: it is admitted now, and the module doc records that it was
/// not before. This is why NEIGHBOURS' §3.4 attribution is corrected in the
/// report rather than repeated.
#[test]
fn a_depth_one_scalar_field_was_refused_in_a_tail_too() {
    if !clang_available() {
        return;
    }
    for actual in ["storage->flag", "storage->leaf", "storage->leaf->n"] {
        let summary = probe(actual);
        assert!(
            summary.diagnostics.is_empty(),
            "`{actual}` must be admitted: {:?}",
            summary.diagnostics
        );
    }
}

/// A record-valued base with a single `.` selection, and two admitted chains in
/// the same argument list.
#[test]
fn a_record_valued_base_and_two_chains_are_admitted() {
    if !clang_available() {
        return;
    }
    assert!(probe("value.flag").diagnostics.is_empty());
    let summary = summary(
        &probe_body("storage->leaf->label").replace(
            "y_log(\"failed %s\", storage->leaf->label)",
            "y_log(\"failed %s %d\", storage->leaf->label, storage->flag)",
        ),
        "y_probe",
    );
    assert!(
        summary.diagnostics.is_empty(),
        "two admitted chains in one tail: {:?}",
        summary.diagnostics
    );
}

/// The depth bound is O04-ARMS' own `ACCESS_CHAIN_LIMIT`, unchanged: four
/// selections are admitted and five are refused, never truncated.
#[test]
fn the_depth_bound_is_inherited_unchanged() {
    if !clang_available() {
        return;
    }
    assert!(
        probe("top->deeper->deep->mid->leaf").diagnostics.is_empty(),
        "four selections are at the bound and must be admitted"
    );
    // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins this deliberately:
    // this rule's bound is unchanged, and the fifth selection is now read by
    // the typed call-actual fallback asked after it, whole and never
    // truncated, with a per-hop obligation on every link.
    assert!(
        probe("top->deeper->deep->mid->leaf->label")
            .diagnostics
            .is_empty(),
        "five selections are read by the typed fallback, not truncated"
    );
}

/// A `Safe` result resting on this admission is independently rederived from the
/// certificate's own content: the classification is repeated from the same
/// declaration facts, so W4 derivation, W4 validation and certificate rechecking
/// all reach it.
#[test]
fn a_certificate_resting_on_the_admission_is_rederived() {
    if !clang_available() {
        return;
    }
    let source = format!("{DECLARATIONS} {}", probe_body("storage->leaf->label"));
    let contracts = load(&normalize_newlines(GENERIC));
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "y_probe",
        &contracts,
    )
    .expect("clang");
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("the admitted probe must reach Safe: {result:?}");
    };
    check_legacy_safety_certificate(&certificate)
        .expect("the certificate must be independently rederived");
}

// --------------------------------------------------------------------------
// The refusal inventory
// --------------------------------------------------------------------------

/// Every non-conforming shape keeps its existing `Unknown` and its existing
/// diagnostic. Each row is a separate substitution into the same statement, so
/// nothing but the actual differs between the admitted case and the refused one.
#[test]
fn every_non_conforming_tail_shape_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    const INDIRECT: &str = "indirect memory access is outside the resource projection";
    // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS moved one row out of this table:
    // `storage->flag + 1` is a scalar value over one typed access, which its
    // call argument rule admits in a declared tail under the base's
    // dereference obligation (pinned in
    // `a_scalar_value_in_a_tail_is_admitted_by_the_call_argument_rule`).
    // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins this table
    // deliberately. The chained-tail rule is unchanged and still refuses every
    // row; the Clang-typed call-actual fallback asked after it now reads five
    // of them in the declared tail under its per-hop obligations over the
    // untracked formals (an indexed byte, a sibling of a bitfield, a
    // typedef-spelled link, an embedded record, a chain past this rule's
    // bound). They move to `TYPED` below; the rest keep their refusal.
    let rows: [(&str, &str); 4] = [
        // The chain is not the whole actual.
        ("&storage->leaf->label", INDIRECT),
        ("storage->flag = 1", INDIRECT),
        // A chain whose base this body may bind: the base is laundered by a
        // whole-object address-of elsewhere in the same body. Since W47 the
        // typed actual reads the chain and the flow obligation refuses the
        // laundered base (`UnknownAlias` of `storage`), in place of the gate.
        ("storage->self->flag", "UnknownAlias:storage"),
        // A nested call whose INNER actual is a chain: premise N9 is
        // deliberately NOT widened, and neither is the lexical gate's view of a
        // nested call's arguments. Measured: the gate reports the access first,
        // so the recorded diagnostic is the indirect-access one rather than
        // O04-B09B10's nested-call wording. Both are the same `Unknown`; the
        // load-bearing fact is that the shape is refused, unchanged.
        ("y_id(storage->leaf)", INDIRECT),
    ];
    // Card CG13 (rule 10w) moves `uh->view->leaf` here deliberately: the
    // union link is a union step, and the pointer read through it (unpunned)
    // is a target-free borrow in the declared tail. BEFORE: refused.
    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d) moves `*storage->leaf->label`
    // here: rule 10t reads the place and the flow now reads its root, which
    // the AST closure binds (a variadic-tail actual) but no path reaching the
    // read bound or handed to a custody position, as an untracked borrowed
    // object. BEFORE: `UnknownAlias:storage` at the flow.
    const TYPED: [&str; 7] = [
        "*storage->leaf->label",
        "uh->view->leaf",
        "storage->leaf->label[0]",
        "bh->bits->label",
        "td->leaf->label",
        "hold->v.k",
        "top->deeper->deep->mid->leaf->label",
    ];
    for (actual, expected) in rows {
        let body = if actual == "storage->self->flag" {
            // The launder: `&storage` seeds the base risky, so the very chain
            // that is admitted everywhere else in this table is refused here.
            probe_body(actual).replace(
                "void *handle;",
                "void *handle; struct y_mid **alias = &storage;",
            )
        } else {
            probe_body(actual)
        };
        let summary = summary(&body, "y_probe");
        if let Some(variable) = expected.strip_prefix("UnknownAlias:") {
            assert!(
                summary.diagnostics.iter().any(|diagnostic| matches!(
                    &diagnostic.kind,
                    LegacyDiagnosticKind::UnknownAlias { variable: found } if found == variable
                )),
                "`{actual}` must keep an obligation on `{variable}`, got {:?}",
                summary.diagnostics
            );
            continue;
        }
        assert!(
            unsupported(&summary).contains(&expected),
            "`{actual}` must keep `{expected}`, got {:?}",
            unsupported(&summary)
        );
    }
    for actual in TYPED {
        assert!(
            probe(actual).diagnostics.is_empty(),
            "`{actual}` is read by the W47 typed call-actual fallback: {:?}",
            probe(actual).diagnostics
        );
    }
    // The launder is load-bearing: the SAME chain, in the SAME statement, with
    // the address-of removed, is admitted. So `storage->self->flag`'s refusal is
    // premise (4) firing and not an unrelated blocker.
    assert!(
        probe("storage->self->flag").diagnostics.is_empty(),
        "the laundered row must be admitted once the address-of is removed"
    );
}

/// **The released base.** A base handed to a declared `Released` position is
/// seeded risky by C02-P's own closure, so reading through it in a tail is
/// refused — the use-after-free shape the brief names, refused fail-closed
/// rather than admitted as data.
#[test]
fn a_released_base_is_never_read_through_in_a_tail() {
    if !clang_available() {
        return;
    }
    let body = "void y_probe(struct y_mid *storage) { \
         y_close(storage); \
         y_log(\"gone %s\", storage->leaf->label); \
     }";
    let summary = summary(body, "y_probe");
    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the pointer read is now
    // left to the flow, which refuses it after the release on the path
    // (`unknown_alias storage`). BEFORE: the gate's refusal (`unsupported`).
    assert!(
        summary.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "storage"
        )),
        "a released base must refuse: {:?}",
        summary.diagnostics
    );
    assert!(
        !matches!(analyze(body, "y_probe"), UnrestrictedLegacyResult::Safe(_)),
        "a released base must never reach Safe"
    );
}

/// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS**: a scalar value over one typed
/// access in a declared tail is admitted, with the base's dereference
/// obligation, where this card's chain predicate refused it as "not the whole
/// actual".
#[test]
fn a_scalar_value_in_a_tail_is_admitted_by_the_call_argument_rule() {
    if !clang_available() {
        return;
    }
    let result = analyze(&probe_body("storage->flag + 1"), "y_probe");
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("the admitted tail read must reach Safe: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("the certificate rechecks");
}

/// **A body-acquired aggregate base is refused in a tail by this card, and
/// deliberately so.**
/// Card O04-READ-BOUND admits a member read through a base *this body bound*, at
/// a declared position, with the linked children's liveness required
/// flow-sensitively. That route reads `contract.parameters.get(position)` and so
/// covers **declared** positions only: a tail index yields `None` and is skipped.
/// This card did not extend it, so the shape where a field could hold a tracked
/// identity this body stored there simply does not arise on the tail route — it
/// is refused by premise (4), because a call-bound base is risky.
///
/// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS** moves this pin deliberately: its
/// call argument rule admits the *scalar* member read `m->flag` in the tail
/// because `m` resolves here to one live, non-null identity this body acquired
/// (the dereference obligation replaces premise (4) for that rule). A pointer
/// member that could hold a linked identity still refuses there
/// (`call_argument_access.rs`, `a_pointer_read_through_a_linked_owner_refuses`),
/// and the same read after the release is refused and accused below.
#[test]
fn a_body_acquired_aggregate_base_is_refused_in_a_tail() {
    if !clang_available() {
        return;
    }
    let body = "void y_probe(void) { \
         struct y_mid *m; \
         m = y_open(8); \
         if (!m) { return; } \
         y_log(\"held %d\", m->flag); \
         y_close(m); \
     }";
    let UnrestrictedLegacyResult::Safe(certificate) = analyze(body, "y_probe") else {
        panic!("a scalar read through a live acquired base is admitted by W44");
    };
    check_legacy_safety_certificate(&certificate).expect("the certificate rechecks");
    let released = "void y_probe(void) { \
         struct y_mid *m; \
         m = y_open(8); \
         if (!m) { return; } \
         y_close(m); \
         y_log(\"held %d\", m->flag); \
     }";
    let result = analyze(released, "y_probe");
    assert!(
        matches!(result, UnrestrictedLegacyResult::Unsafe(_)),
        "a read through the released base is refused and accused: {result:?}"
    );
    let summary = summary(released, "y_probe");
    assert!(
        summary.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::UseAfterRelease { .. }
        )),
        "the settled state accuses the read: {:?}",
        summary.diagnostics
    );
}

/// **The chain is admitted by its POSITION, and the position comes from the
/// callee's declaration.** The identical actual at a `Consumed` position, and at
/// a tail of a callee with no declared tail, both keep their refusals: no actual
/// can buy itself a data tail.
#[test]
fn the_position_is_decided_by_the_declaration_not_the_actual() {
    if !clang_available() {
        return;
    }
    let consumed = "void y_probe(struct y_mid *storage) { y_eat(storage->leaf); }";
    assert!(
        !unsupported(&summary(consumed, "y_probe")).is_empty(),
        "a custody-taking position must keep its refusal"
    );
    let untailed = "void y_probe(struct y_mid *storage) { \
         y_plain(\"x\", storage->leaf->label); \
     }";
    assert!(
        !unsupported(&summary(untailed, "y_probe")).is_empty(),
        "a callee with no declared tail must keep its refusal"
    );
}

/// **Exempting the span is not admitting the call.** The chain's span leaves the
/// lexical gate, but the tail rule still requires EVERY tail actual to be data,
/// so a sibling actual naming a tracked identity refuses the whole statement —
/// and the leak the refusal leaves behind is still visible.
#[test]
fn a_sibling_tail_actual_carrying_an_identity_still_refuses() {
    if !clang_available() {
        return;
    }
    let body = "void y_probe(struct y_mid *storage) { \
         void *handle; \
         handle = y_open(8); \
         if (!handle) { return; } \
         y_log(\"both %s %p\", storage->leaf->label, handle); \
         y_close(handle); \
     }";
    let summary = summary(body, "y_probe");
    assert!(
        !unsupported(&summary).is_empty(),
        "a tracked sibling must refuse the whole statement: {:?}",
        summary.diagnostics
    );
    assert!(
        !matches!(analyze(body, "y_probe"), UnrestrictedLegacyResult::Safe(_)),
        "a tracked sibling must never reach Safe"
    );
}

/// **The value binds no name, and the complementary shape is unchanged.** A tail
/// actual is an argument, never a destination, so nothing of this function
/// derives from the read. The shape that *does* bind — `x = base->f->g;` — keeps
/// its own existing `OpaqueValue` route, and releasing an opaque-bound name is
/// still never silently admitted. That containment is what makes premise (3)'s
/// intermediate-place exposure argument hold, and this card does not touch it.
#[test]
fn an_opaque_bound_name_is_still_never_released_silently() {
    if !clang_available() {
        return;
    }
    let body = "void y_probe(struct y_mid *storage) { \
         char *held; \
         held = storage->leaf->label; \
         y_close(held); \
     }";
    let summary = summary(body, "y_probe");
    assert!(
        !summary.diagnostics.is_empty(),
        "releasing an opaque-bound name must not be silently admitted"
    );
    assert!(
        !matches!(analyze(body, "y_probe"), UnrestrictedLegacyResult::Safe(_)),
        "releasing an opaque-bound name must never reach Safe"
    );
}

/// **The admission is confined to the tail.** A declared `Borrowed` position of
/// the same callee already admitted the same chain before this card (card C02-P,
/// widened by O04-ARMS), and still does — this card widened nothing there, and
/// nothing about the `Borrowed` route moves.
#[test]
fn the_borrowed_route_is_unchanged() {
    if !clang_available() {
        return;
    }
    let body = "void y_probe(struct y_mid *storage) { y_borrow(storage->leaf); }";
    assert!(
        summary(body, "y_probe").diagnostics.is_empty(),
        "the declared Borrowed route must stay admitted"
    );
}
