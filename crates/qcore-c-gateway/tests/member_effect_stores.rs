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

//! Card MEMBER-EFFECT-STORES, body half (gateway rule 38k): the hand-off
//! store into a member of the caller's object, end to end through the
//! custody gateway (route G).
//!
//! Every `Safe` here is `Safe` over the synthetic `q_*` fixture contracts and
//! the stated premise `mes_caller_slot`, which the row lists; every `Safe`
//! certificate is rechecked.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyFunctionSummary, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_c_gateway::member_handoff;

const S: &str = "struct I { char *buf; long len; };\n\
struct S { char *buf; struct I in; void *any; struct S *next; long n; };\n\
void use(char *b); void q_take(struct S *s);\n";

/// The row of `function` and whether its unit was decided `Safe`.
fn row(source: &str, function: &str) -> LegacyFunctionSummary {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        function,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    summary
        .and_then(|summary| summary.functions.get(function))
        .cloned()
        .unwrap_or_else(|| panic!("no row for `{function}`: {outcome:?}"))
}

fn carries_premise(summary: &LegacyFunctionSummary) -> bool {
    member_handoff::premise_carried(summary)
}

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// Admitted: each real sub-shape
// ---------------------------------------------------------------------------

#[test]
fn a_hand_off_into_a_formals_member_is_admitted_under_the_premise() {
    if !clang_available() {
        return;
    }
    for body in [
        // `rd_buf_init`, `rd_kafka_assignment_init`: an owned call result.
        "void subject(struct S *p) { p->buf = q_alloc(); }",
        // A nullable owned result: a failed acquisition stores NULL.
        "void subject(struct S *p) { p->buf = q_maybe_alloc(); }",
        // `mk_vhost_set_property`: an embedded member path.
        "void subject(struct S *p) { p->in.buf = q_alloc(); p->in.len = 1; }",
        // `rd_kafka_assignment_init`: sibling places, each handed off once.
        "void subject(struct S *p) { p->buf = q_alloc(); p->in.buf = q_alloc(); }",
        // `rd_kafka_snappy_init_env`, `otel_span_set_events`: a null test of
        // the stored value after the store.
        "int subject(struct S *p) { p->buf = q_maybe_alloc(); if (!p->buf) return -1; return 0; }",
        "int subject(struct S *p) { p->buf = q_maybe_alloc(); if (p->n > 0 && p->buf == 0) return -1; return 0; }",
        // A local acquisition, tested, then handed off.
        "int subject(struct S *p) { char *l = q_maybe_alloc(); if (l == 0) return -1; p->buf = l; return 0; }",
        // `create_counter_instance`: a cast pointer-to-pointer of a local the
        // body acquired and initialised.
        "int subject(struct S *p) { struct S *l; if (0 == p) return -1; l = q_maybe_alloc(); \
         if (0 == l) return -2; l->next = 0; p->any = (void *) l; return 0; }",
    ] {
        let summary = row(&format!("{S}{body}"), "subject");
        assert_eq!(summary.status, AnalysisStatus::Safe, "{body}: {summary:?}");
        assert!(carries_premise(&summary), "{body}: no premise");
    }
}

// ---------------------------------------------------------------------------
// Refused: adversarial near-misses
// ---------------------------------------------------------------------------

#[test]
fn a_near_miss_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        // The local used, released or stored again after the store.
        "void subject(struct S *p) { char *l = q_alloc(); p->buf = l; use(l); }",
        "void subject(struct S *p) { char *l = q_alloc(); p->buf = l; q_free(l); }",
        "void subject(struct S *p) { char *l = q_alloc(); p->buf = l; p->any = l; }",
        // The local copied before the store, then released through the copy.
        "void subject(struct S *p) { char *l = q_alloc(); char *m = l; p->buf = l; q_free(m); }",
        // A borrowed value: example C, not a hand-off.
        "void subject(struct S *p) { char *l = p->in.buf; p->buf = l; }",
        // Realloc self-assignment: a nullable reallocation loses the old block.
        "void subject(struct S *p, unsigned long n) { p->buf = q_realloc(p->buf, n); }",
        // A rebound formal.
        "void subject(struct S *p, struct S *r) { p = r; p->buf = q_alloc(); }",
        // A store through a local alias of the formal (`unpack_opts_unit`).
        "void subject(struct S *p) { struct S *m = p; m->buf = q_alloc(); }",
        // A store in a loop.
        "void subject(struct S *p, int n) { while (n-- > 0) p->buf = q_alloc(); }",
        // A pointer hop below the formal (`rd_kafkap_Fetch_reply_tags_set_topic`).
        "void subject(struct S *p) { p->next->buf = q_alloc(); }",
        // The place used after the store (SCRAM, `mk_buffer_cat`): a failed
        // acquisition stores NULL, then the place is dereferenced.
        "void subject(struct S *p) { p->buf = q_maybe_alloc(); p->buf[0] = 'x'; }",
        "void subject(struct S *p) { p->buf = q_alloc(); use(p->buf); }",
        // The same through the local.
        "void subject(struct S *p) { char *l = q_maybe_alloc(); p->buf = l; l[0] = 'x'; }",
        // A second store into the place: the first hand-off lost.
        "void subject(struct S *p) { p->buf = q_alloc(); p->buf = q_alloc(); }",
        // The formal handed on whole after the store.
        "void subject(struct S *p) { p->buf = q_alloc(); q_take(p); }",
        // A release after the store, through another pointer.
        "void subject(struct S *p, struct S *r) { p->buf = q_alloc(); q_free(r->buf); }",
        // A write through another object after the store (it may alias).
        "void subject(struct S *p, struct S *r) { p->buf = q_alloc(); r->buf = 0; }",
        // A value read from memory or an integer cast.
        "void subject(struct S *p, struct S *r) { p->buf = r->buf; }",
        "void subject(struct S *p) { char *l = q_alloc(); p->n = (long) l; }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

/// `ctr_link_set_trace_state`, as the two Fluent Bit trees spell it (the
/// landing measurement's one unpredicted row, 2026-11-25).
///
/// The pinned corpus (`c-gateway-wave-pin`, `ctr_link.c:78-89`) stores the
/// owned copy and then null-tests the place: rule 38k's admitted shape (H1's
/// `#mes_null_test`), `Safe` under `mes_caller_slot`. The `current` tree
/// (`046accf6`, `ctr_link.c:82-101`) first releases what the place holds
/// (`cfl_sds_destroy(link->trace_state)`, an occurrence of the place that is
/// no null test), then stores: `mes_refuse_place_used`, so the row keeps its
/// refusal there.
#[test]
fn the_trace_state_setter_follows_its_tree() {
    if !clang_available() {
        return;
    }
    let records = "typedef char *sds_t;\nstruct L { sds_t trace_state; };\n";
    let corpus = "int subject(struct L *link, char *t) { if (!link || !t) return -1; \
                  link->trace_state = q_maybe_alloc(); if (!link->trace_state) return -1; return 0; }";
    let summary = row(&format!("{records}{corpus}"), "subject");
    assert_eq!(summary.status, AnalysisStatus::Safe, "{summary:?}");
    assert!(carries_premise(&summary));
    let current = "int subject(struct L *link, char *t) { sds_t n; if (!link || !t) return -1; \
                   n = q_maybe_alloc(); if (!n) return -1; \
                   if (link->trace_state != 0) { q_nullable_free(link->trace_state); } \
                   link->trace_state = n; return 0; }";
    let run = g(&format!("{records}{current}"));
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(IND), "{run:?}");
    // The same release-then-replace with the call stored directly.
    let direct = "int subject(struct L *link) { q_nullable_free(link->trace_state); \
                  link->trace_state = q_maybe_alloc(); return 0; }";
    let run = g(&format!("{records}{direct}"));
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn a_caller_of_an_admitted_body_stays_non_safe() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{S}void fill(struct S *p) {{ p->buf = q_alloc(); }}\n\
         void subject(struct S *p) {{ fill(p); }}\n\
         void owner(void) {{ struct S *s = q_alloc(); fill(s); q_free(s); }}"
    );
    let callee = row(&source, "fill");
    assert_eq!(callee.status, AnalysisStatus::Safe, "{callee:?}");
    assert!(carries_premise(&callee));
    for caller in ["subject", "owner"] {
        let summary = row(&source, caller);
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "{caller}: {summary:?}"
        );
    }
}

#[test]
fn a_member_release_beside_a_hand_off_publishes_no_effect() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{S}void fill(struct S *p) {{ q_nullable_free(p->any); p->buf = q_alloc(); }}\n\
         void subject(struct S *p) {{ fill(p); }}"
    );
    let summary = row(&source, "subject");
    assert_ne!(summary.status, AnalysisStatus::Safe, "{summary:?}");
}

#[test]
fn a_verdict_lacking_the_premise_is_refused() {
    if !clang_available() {
        return;
    }
    let mut summary = row(
        &format!("{S}void subject(struct S *p) {{ p->buf = q_alloc(); }}"),
        "subject",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
    // Drop the premise: the row is refused by name.
    summary.rule_obligations.premises.clear();
    member_handoff::enforce_premise(&mut summary);
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(summary.diagnostics.iter().any(|diagnostic| {
        format!("{:?}", diagnostic.kind).contains(member_handoff::Reason::PremiseMissing.formula())
    }));
}

#[test]
fn the_premise_is_a_formula_never_prose() {
    let statement = member_handoff::premise_statement();
    assert_eq!(
        statement,
        "mes_caller_slot: forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }"
    );
    assert_eq!(member_handoff::validated(), Ok(()));
}
