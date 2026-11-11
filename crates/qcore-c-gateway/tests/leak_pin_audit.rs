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

//! Card W44-LEAK-PIN-AUDIT — the two class (b) leak mechanisms the audit of
//! the 271 baseline leak leaves repaired, and the one it deliberately left.
//!
//! * `RETURN-CAST-PLACE-NOT-ESCAPED`: `return (T *) p;` is decided exactly as
//!   `return p;` is; a cast moving a possibly tracked value between an integer
//!   and a pointer is refused, never escaped; `(int) fd` keeps a descriptor as
//!   custody.
//! * The refused hand-on (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`): a tracked
//!   identity the classifier sees named in a refused statement (store value,
//!   return, call argument, operand, `&x`) gets an unresolved effect there, so
//!   no leak is claimed through it. The function stays `Unknown`, never
//!   `Safe`; nothing is discharged.
//! * Deliberately left: a member value read (`p->len`) hands on the member,
//!   not the identity, and a modelled condition beside a refused statement is
//!   not read for it.
//!
//! Each body asserts the **complete** diagnostic vector and the status, so a
//! rule that silently dropped an unrelated accusation would fail. The fixtures
//! use the generic `k_*` surface of `fixtures/contracts/descriptor_generic.qcore`;
//! `Safe` here is `Safe` over synthetic trusted test assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/descriptor_generic.qv");

const DECLARATIONS: &str = "\
int k_open(const char *path); \
int k_close(int fd); \
void *k_fopen(const char *path); \
int k_fclose(void *stream); \
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
void k_keep(void *p); \
struct k_box { void *item; long n; }; \
struct k_list { struct k_box **items; long n; }; \
typedef struct k_opaque *k_handle;";

const CAST_REFUSAL: &str =
    "unsupported:cast of a possibly tracked value to another carrier in return expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the descriptor fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent leak-pin test: {reason}");
            false
        }
    }
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

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}")
            .split([' ', '{'])
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result).functions.get("w").expect("w analysed");
    if status == "Safe" {
        assert_eq!(function.status, AnalysisStatus::Safe);
    }
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    let (measured, keys) = outcome(body);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

// -- RETURN-CAST-PLACE-NOT-ESCAPED -------------------------------------------

/// The corpus shape (`mk_string_copy_substr`, `rd_dl_open0`): a pointer cast of
/// the returned place escapes it, for the heap and for a stream alike, exactly
/// as the uncast return does.
#[test]
fn a_pointer_cast_return_escapes_its_place() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *w(void) { void *p = k_alloc(1); if (!p) return 0; return (char *)p; }",
        "void *w(void) { void *p = k_fopen(\"p\"); if (!p) return 0; return (char *)p; }",
        "void *w(void) { char *p = k_alloc(1); if (!p) return 0; return (void *)p; }",
        "void *w(void) { void *p = k_alloc(1); if (!p) return 0; return (const char **)p; }",
        "struct k_box *w(void) { struct k_box *p = k_alloc(1); if (!p) return 0; \
         return (struct k_box *)(void *)p; }",
        "struct k_box *w(void) { void *p = k_alloc(1); if (!p) return 0; return (struct k_box *)p; }",
    ] {
        expect("cast return", body, "Safe", &[]);
    }
    for body in [
        "void *w(void) { void *p = k_alloc(1); if (!p) return 0; return p; }",
        "void *w(void) { void *p = k_fopen(\"p\"); if (!p) return 0; return p; }",
    ] {
        expect("plain return", body, "Safe", &[]);
    }
}

/// A cast return of an identity already released is still a return of it:
/// the cast neither hides nor invents anything the plain return reports.
#[test]
fn a_cast_return_is_decided_as_the_plain_return_is() {
    if !clang_available() {
        return;
    }
    let plain = outcome("void *w(void) { void *p = k_alloc(1); k_free(p); return p; }");
    let cast = outcome("void *w(void) { void *p = k_alloc(1); k_free(p); return (char *)p; }");
    assert_eq!(plain, cast);
}

/// The cast adds no admission of its own: a pointer cast of a member chain is
/// decided exactly as the uncast return of that chain, by the same rules.
#[test]
fn a_cast_of_a_member_chain_is_decided_as_the_uncast_return() {
    if !clang_available() {
        return;
    }
    for (plain, cast) in [
        (
            "void *w(struct k_box *b) { return b->item; }",
            "void *w(struct k_box *b) { return (char *)b->item; }",
        ),
        (
            "void *w(struct k_box *b) { b = k_alloc(1); if (!b) return 0; return b->item; }",
            "void *w(struct k_box *b) { b = k_alloc(1); if (!b) return 0; return (const char **)b->item; }",
        ),
        // The corpus's `rd_kafka_TopicPartitionInfo_isr` shape: the uncast
        // return of a never-bound chain is admitted and escapes nothing, so
        // the cast one is too.
        (
            "struct k_box **w(struct k_list *l, long *n) { *n = l->n; return l->items; }",
            "struct k_box **w(struct k_list *l, long *n) { *n = l->n; return (struct k_box **)l->items; }",
        ),
    ] {
        assert_eq!(outcome(plain), outcome(cast), "{cast}");
    }
    expect(
        "an admitted chain under a cast",
        "struct k_box **w(struct k_list *l, long *n) { *n = l->n; return (struct k_box **)l->items; }",
        "Safe",
        &[],
    );
}

/// Pointer to integer: an operand the reader commits to as a pointer, cast to
/// an integer type, is refused — never escaped, and never accused either.
#[test]
fn a_pointer_cast_to_an_integer_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "long w(void) { char *p = k_alloc(1); if (!p) return 0; return (long)p; }",
        "int w(void) { char *p = k_alloc(1); if (!p) return 0; return (int)p; }",
        "unsigned long w(void) { struct k_box *p = k_alloc(1); if (!p) return 0; return (unsigned long)p; }",
    ] {
        expect("pointer to integer", body, "Unknown", &[CAST_REFUSAL]);
    }
}

/// A tracked descriptor through `(int)` stays custody: it escapes exactly as
/// the uncast return does. Any other cast of it is refused.
#[test]
fn a_descriptor_through_int_stays_custody_and_other_casts_refuse() {
    if !clang_available() {
        return;
    }
    let plain =
        outcome("int w(void) { int fd = k_open(\"p\"); if (fd < 0) return -1; return fd; }");
    let cast =
        outcome("int w(void) { int fd = k_open(\"p\"); if (fd < 0) return -1; return (int)fd; }");
    assert_eq!(plain, cast);
    assert!(!cast.1.iter().any(|key| key == "leak"), "{cast:?}");
    for body in [
        "long w(void) { int fd = k_open(\"p\"); if (fd < 0) return -1; return (long)fd; }",
        "void *w(void) { int fd = k_open(\"p\"); if (fd < 0) return 0; return (void *)fd; }",
    ] {
        expect("descriptor crossing", body, "Unknown", &[CAST_REFUSAL]);
    }
}

/// Untracked scalar data keeps its pre-card answer.
#[test]
fn untracked_scalar_casts_are_unchanged() {
    if !clang_available() {
        return;
    }
    expect("scalar", "long w(int n) { return (long)n; }", "Safe", &[]);
    expect(
        "scalar to pointer",
        "void *w(long n) { return (void *)n; }",
        "Safe",
        &[],
    );
}

// -- The refused escape ------------------------------------------------------

/// A refused store of the identity withdraws the leak; the refusal keeps the
/// function `Unknown`.
#[test]
fn a_refused_store_of_the_identity_withdraws_the_leak() {
    if !clang_available() {
        return;
    }
    let (status, keys) =
        outcome("void w(void **out) { void *p = k_alloc(1); if (!p) return; *out = p; }");
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(!keys.iter().any(|key| key == "leak"), "{keys:?}");
    let (status, keys) = outcome(
        "void w(struct k_box *b) { void *p = k_alloc(1); if (!p) return; b->item = (void *)p; }",
    );
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(!keys.iter().any(|key| key == "leak"), "{keys:?}");
}

/// Only the identity the refused statement's value side hands on is
/// withdrawn: an unrelated live acquisition is still accused.
#[test]
fn an_unrelated_identity_keeps_its_accusation() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome(
        "void w(void **out) { void *p = k_alloc(1); void *q = k_alloc(1); \
         if (!p || !q) return; *out = p; }",
    );
    assert_eq!(status, "Unsafe", "{keys:?}");
    assert!(keys.iter().any(|key| key == "leak"), "{keys:?}");
}

/// An identity handed to an uncontracted callee is withdrawn too: the refused
/// call may have released or retained it (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`).
#[test]
fn a_call_argument_hand_on_withdraws_the_accusation() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome(
        "void k_touch(void *p); void w(void) { void *p = k_alloc(1); if (!p) return; k_touch(p); }",
    );
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(!keys.iter().any(|key| key == "leak"), "{keys:?}");
    // An interior address handed on counts as the identity.
    let (status, keys) = outcome(
        "void k_link(void *p); void w(void) { struct k_box *b = k_alloc(1); if (!b) return; \
         k_link(&b->item); }",
    );
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(!keys.iter().any(|key| key == "leak"), "{keys:?}");
}

/// The macro shape `RD_UT_ASSERT(cond(p), …)`: a refused statement on the line
/// of a **modelled** condition that names the identity does not withdraw it;
/// the refused statement reads only its line's statement table. (Hand-ons are
/// read per source line, so the fixture spans lines.)
#[test]
fn a_refused_statement_beside_a_condition_keeps_the_accusation() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome(
        "void k_log(const char *m);\n\
         int w(int n) { char *p = k_alloc(4); if (!p) return 0;\n\
         if (p != 0 && n > 3) { k_log(\"empty\"); return 1; }\n\
         k_free(p); return 0; }",
    );
    assert_eq!(status, "Unsafe", "{keys:?}");
    assert!(keys.iter().any(|key| key == "leak"), "{keys:?}");
}

/// A member value read through the identity hands on the member, not the
/// identity.
#[test]
fn a_member_value_store_keeps_the_accusation() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome(
        "long g; void w(struct k_box *b) { struct k_box *p = k_alloc(1); if (!p) return; \
         b->n = p->n; }",
    );
    assert_eq!(status, "Unsafe", "{keys:?}");
    assert!(keys.iter().any(|key| key == "leak"), "{keys:?}");
}

/// A refused return of the identity withdraws it too (the
/// `RETURN-REFUSAL-LEAK-REPORT-PREMISE` shape, for a returned identity).
#[test]
fn a_refused_return_of_the_identity_withdraws_it() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome(
        "struct k_box *w(void) { struct k_box *p = k_alloc(1); if (!p) return 0; \
         return p->n > 0 ? p : 0; }",
    );
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // return is no longer refused. It is lowered by branching on `p->n > 0`:
    // one edge returns `p`, the other returns `0` while `p` is still owned,
    // the fixture's own leak, now reported (BEFORE: the refused return
    // withdrew it). A still-refused return keeps withdrawing it:
    // `a_member_value_store_keeps_the_accusation`'s neighbours pin that.
    assert_eq!(status, "Unsafe", "{keys:?}");
    assert!(keys.iter().any(|key| key == "leak"), "{keys:?}");
}
