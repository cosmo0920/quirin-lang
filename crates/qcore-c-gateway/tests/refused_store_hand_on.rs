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

//! Card CURRENT-CONTEXT-NEW-TRIAGE (gateway rule 5l (6)): a B08 field store
//! refused on its owner hands on its value.
//!
//! `OWNER->F = V` with `OWNER` a local record pointer bound once by a call
//! passes B08's lexical base premises, so the importer lowers it as a typed
//! field store; the flow-sensitive owner check then refuses it when `OWNER`
//! names no live identity this body acquired (a borrowed result, a
//! `pthread_getspecific` value, a released or nullable object) and reports
//! `UnknownAlias OWNER`. Before this card that refusal returned without
//! touching `V`'s site, so `V` was accused of a leak at every later exit
//! (Monkey's `context_new`, `lib/monkey/tls/tls_openssl.c`). The store is
//! refused before its effect, so `V` now carries an unresolved effect: the
//! accusation is withdrawn, never discharged, and the refusal keeps the
//! function out of `Safe`.
//!
//! The fixture contracts are the CG00 `q_*` assumptions plus
//! `bound_untracked.qv` (`bu_get`: a borrowed, custody-free result) and
//! `r_return_call.qv` (`r_wrap`: borrows its argument, returns a fresh
//! owned object). Trusted test assumptions, not library facts. The Lean model
//! is `QCore.RefusedStoreHandOn`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_consume(void *p);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
void *bu_get(void *p); void *r_wrap(void *p);
void *ssl_new(void); void ssl_set(void *s, int fd);
struct N { void *ssl; int fd; int want_event; struct N *_next; };
struct T { struct N *contexts; int x; };
";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_nullable.qcore",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "bound_untracked.qcore",
            include_str!("fixtures/contracts/bound_untracked.qv"),
        ),
        file(
            "r_return_call.qcore",
            include_str!("fixtures/contracts/r_return_call.qv"),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<(usize, LegacyDiagnosticKind)>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|(_, kind)| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn refused_owner(&self, owner: &str) -> bool {
        self.diagnostics.iter().any(|(_, kind)| {
            matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == owner)
        })
    }
}

fn run_function(source: &str, function: &str) -> Run {
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
    let result = summary
        .and_then(|summary| summary.functions.get(function))
        .unwrap_or_else(|| panic!("`{function}` projected: {outcome:?}"));
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.kind.clone()))
            .collect(),
    }
}

fn run(source: &str) -> Run {
    run_function(source, "subject")
}

/// BASE: `Unsafe`, one `Leak` beside `UnknownAlias t`. The store into a
/// borrowed result's member is refused on its owner; the node it stored is
/// withdrawn, not discharged: `Unknown`, never `Safe`.
#[test]
fn a_store_into_a_borrowed_roots_member_withdraws_the_stored_value() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { struct T *t; struct N *n; t = bu_get(0); \
         if (t == 0) return; n = q_maybe_alloc(); if (n == 0) return; \
         t->contexts = n; }",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    assert!(run.refused_owner("t"), "{run:?}");
}

/// The head insert of Monkey's per-thread context list, both returns of
/// `context_new` kept: every path from the allocation to a return passes the
/// refused link store. BASE: `Unsafe`, two `Leak`s (the `ssl_new` failure
/// return and the success return), as in the evidence run.
#[test]
fn context_new_s_head_insert_is_withdrawn_at_both_returns() {
    if !clang_available() {
        return;
    }
    let source = "void *context_new(int fd)\n\
        {\n\
            void *ssl;\n\
            struct N *ctx_head;\n\
            struct T *thctx;\n\
            thctx = bu_get(0);\n\
            if (thctx == 0) {\n\
                return 0;\n\
            }\n\
            ctx_head = bu_get(thctx);\n\
            if (ctx_head == 0) {\n\
                ctx_head = q_maybe_alloc();\n\
                if (ctx_head == 0) {\n\
                    return 0;\n\
                }\n\
                ctx_head->fd = -1;\n\
                ctx_head->want_event = 1;\n\
                ctx_head->_next = thctx->contexts;\n\
                thctx->contexts = ctx_head;\n\
            }\n\
            ssl = ssl_new();\n\
            if (ssl == 0) {\n\
                return 0;\n\
            }\n\
            ssl_set(ssl, fd);\n\
            ctx_head->ssl = ssl;\n\
            ctx_head->fd = fd;\n\
            return ssl;\n\
        }";
    let run = run_function(source, "context_new");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    assert!(run.refused_owner("thctx"), "{run:?}");
}

/// Precision: only the stored value is handed on. A second allocation the
/// refused store never names keeps its leak, so the function stays `Unsafe`.
#[test]
fn an_object_the_refused_store_does_not_name_still_leaks() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { struct T *t; struct N *n; void *other; \
         t = bu_get(0); if (t == 0) return; other = q_alloc(); \
         n = q_maybe_alloc(); if (n == 0) { q_free(other); return; } \
         t->contexts = n; }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1, "{run:?}");
    assert!(run.refused_owner("t"), "{run:?}");
}

/// An admitted store is untouched: into a live owner this body acquired the
/// store records a field link (B08), and releasing the owner does not
/// release the child (premise P-B08-SHALLOW), so the child still leaks.
#[test]
fn a_store_into_a_released_body_owner_keeps_the_childs_leak() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { struct T *o; struct N *n; o = q_alloc(); \
         n = q_alloc(); o->contexts = n; q_free(o); }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1, "{run:?}");
    assert!(!run.refused_owner("o"), "{run:?}");
}

/// A store refused on an owner this body already released hands its value on
/// too (the refusal is uniform, as rule 5l (5)'s is): `Unknown`, not a leak
/// claim, and the refusal stays.
#[test]
fn a_store_through_a_released_owner_is_refused_and_withdraws() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { struct T *o; struct N *n; o = q_alloc(); \
         n = q_alloc(); q_free(o); o->contexts = n; }",
    );
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    assert!(run.refused_owner("o"), "{run:?}");
}

/// A contracted call whose result the refused store would take is refused
/// before its effects: its fresh result is never created, and an actual at a
/// custody position is handed on (rule 5l (5)'s list). `r_wrap` borrows its
/// argument, so the argument is not handed on and keeps its leak.
#[test]
fn a_call_result_stored_into_a_refused_member_hands_on_only_custody_actuals() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { struct T *t; void *p; t = bu_get(0); \
         if (t == 0) return; p = q_alloc(); t->contexts = r_wrap(p); }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refused_owner("t"), "{run:?}");
    assert_eq!(
        run.leaks(),
        1,
        "the borrowed actual keeps its leak: {run:?}"
    );
}

/// Nothing becomes `Safe`: a store into a borrowed root's member followed by
/// an explicit release of the stored value is refused, not certified.
#[test]
fn a_refused_store_never_certifies() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { struct T *t; struct N *n; t = bu_get(0); \
         if (t == 0) return; n = q_maybe_alloc(); if (n == 0) return; \
         t->contexts = n; q_nullable_free(n); }",
    );
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refused_owner("t"), "{run:?}");
}
