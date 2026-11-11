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

//! Card W47-ACCESS-PATH-POSITIONS (gateway rule 10k) — the Clang-typed
//! access-path layer in condition operands, `return` expressions and call
//! actuals, each a fallback after the position's existing routes.
//!
//! Every test asserts the complete diagnostic vector of the entry with its
//! status. The fixtures use a generic `c_*` surface: `Safe` here is `Safe`
//! over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn c_make(n: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn c_drop(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(b, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_use(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_look(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_peek(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(l);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_forget(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(l, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef enum { C_OFF, C_ON } c_mode; \
typedef union c_u { int x; float f; } c_u; \
struct c_leaf { int weight; char *name; c_mode mode; }; \
struct c_item { int count; struct c_leaf *leaf; struct c_leaf cells[4]; }; \
struct c_box { int flag; struct c_item *item; struct c_item *items; c_u *hidden; c_mode mode; }; \
struct c_box *c_make(int n); \
void c_drop(void *b); \
void c_use(int v); \
void c_look(struct c_item *p); \
void c_peek(struct c_leaf *l); \
void c_forget(struct c_leaf *l); \
int c_count(struct c_box *b); \
int c_other(int v);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "access_path_positions.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent access-path-positions test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
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

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn safe() -> (&'static str, Vec<String>) {
    ("Safe", Vec::new())
}

fn unknown(keys: &[&str]) -> (&'static str, Vec<String>) {
    (
        "Unknown",
        keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>(),
    )
}

const BRANCH: &str = "unsupported:unmodeled call, aggregate or indirect access in branch condition";
const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";
const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

/// Each controlling position reads a typed operand the lexical routes
/// refuse (an indexed embedded record through an untracked formal).
#[test]
fn a_typed_operand_certifies_in_every_controlling_position() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b, int i) { if (b->items[i].count > 1) return 1; return 0; }",
        "int w(struct c_box *b, int i) { while (b->items[i].count > 1) i++; return i; }",
        "int w(struct c_box *b, int i) { do { i++; } while (b->items[i].count > 1); return i; }",
        "int w(struct c_box *b, int i) { for (i = 0; i < b->items[i].count; i++) {} return i; }",
        "int w(struct c_box *b, int i) { switch (b->items[i].count) { case 1: return 1; } return 0; }",
        "int w(struct c_box *b, int i) { if (b->items[i].leaf == 0 || !b->items[i].leaf->weight) return 1; return 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// `p && p->x`: the right operand is evaluated only on the edge where the
/// existing null branch refined `p` non-null, so its obligation discharges.
/// `p || p->x` evaluates it only where `p` is null, so it refuses. The typed
/// operand on an untracked formal makes the lexical whole-condition routes
/// refuse, so the desugared plan decides.
#[test]
fn a_short_circuit_scopes_each_operand_obligation() {
    if !clang_available() {
        return;
    }
    let guarded = "int w(int n, struct c_box *x, int i) { struct c_box *b = c_make(n); int r = 0; \
        if (b && b->flag && x->items[i].count) r = 1; if (b) c_drop(b); return r; }";
    assert_eq!(outcome(guarded), safe());
    let unguarded = "int w(int n, struct c_box *x, int i) { struct c_box *b = c_make(n); int r = 0; \
        if (b || b->flag || x->items[i].count) r = 1; if (b) c_drop(b); return r; }";
    assert_eq!(outcome(unguarded), unknown(&["unknown_alias:b"]));
}

/// `c ? p->x : q->x`: each arm's obligation applies only on its own edge,
/// so a guarded `p` discharges and an unguarded `q` alone refuses.
#[test]
fn a_conditional_operator_scopes_each_arm() {
    if !clang_available() {
        return;
    }
    let one = "int w(int n, int c, struct c_box *x, int i) { \
        struct c_box *p = c_make(n); struct c_box *q = c_make(n); int r = 0; \
        if (!p) { if (q) c_drop(q); return 0; } \
        if ((c ? p->flag : q->flag) && x->items[i].count) r = 1; \
        c_drop(p); if (q) c_drop(q); return r; }";
    assert_eq!(outcome(one), unknown(&["unknown_alias:q"]));
    let both = "int w(int n, int c, struct c_box *x, int i) { \
        struct c_box *p = c_make(n); struct c_box *q = c_make(n); int r = 0; \
        if (!p) { if (q) c_drop(q); return 0; } if (!q) { c_drop(p); return 0; } \
        if ((c ? p->flag : q->flag) && x->items[i].count) r = 1; \
        c_drop(p); c_drop(q); return r; }";
    assert_eq!(outcome(both), safe());
}

/// P2's refusals hold in a condition: a typedef-hidden union, a call inside
/// the path, a union behind a cast, and a mixed path under a tracked root.
#[test]
fn a_condition_keeps_every_path_refusal() {
    if !clang_available() {
        return;
    }
    // Card CG13 flip (rule 10w): a scalar read through the typedef-hidden
    // union, bare or behind a scalar cast, is a custody-neutral condition
    // operand. BEFORE: both refused (BRANCH).
    for body in [
        "int w(struct c_box *b) { if (b->hidden->x) return 1; return 0; }",
        "int w(struct c_box *b, int i) { if (b->items[i].count > (int)b->hidden->f) return 1; return 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    assert_eq!(
        outcome(
            "int w(struct c_box *b, int i) { if (b->items[c_count(b)].count) return 1; return 0; }"
        ),
        unknown(&[BRANCH])
    );
    // Card CG10 (rule 10t): the tracked mixed path is now read, and its
    // `b->items` link, which no admitted store bound, refuses at the flow.
    let tracked = "int w(int n) { struct c_box *b = c_make(n); int r = 0; \
        if (b && b->items[0].count) r = 1; if (b) c_drop(b); return r; }";
    let (status, keys) = outcome(tracked);
    assert_eq!(status, "Unknown");
    assert_eq!(keys, ["unknown_alias:b->items"], "{keys:?}");
}

/// One `NAME = PATH` operand is P2's borrowed read, then the existing null
/// branch on `NAME`, which narrows only that alias and keeps its identity
/// refusal (constraint 10b).
#[test]
fn an_assignment_operand_is_a_borrowed_read_then_the_null_branch() {
    if !clang_available() {
        return;
    }
    let body =
        "int w(struct c_box *b) { struct c_item *it; if (!(it = b->item)) return 0; return 1; }";
    assert_eq!(outcome(body), unknown(&["unknown_alias:it"]));
}

#[test]
fn a_typed_return_certifies_and_hands_nothing_back() {
    if !clang_available() {
        return;
    }
    for body in [
        "struct c_item *w(struct c_box *b) { return b->item; }",
        "char *w(struct c_box *b) { return b->item->leaf->name; }",
        "int w(struct c_box *b, int i) { return b->items[i].count; }",
        "char *w(struct c_item *it) { return it->cells[0].name; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// A released root is never read through: the scalar chain refuses, and a
/// pointer read from a root the body binds refuses outright.
#[test]
fn a_return_through_a_released_or_tracked_root_is_not_safe() {
    if !clang_available() {
        return;
    }
    let scalar = "int w(int n) { struct c_box *b = c_make(n); if (!b) return 0; c_drop(b); \
        return b->item->count; }";
    let (status, keys) = outcome(scalar);
    assert_ne!(status, "Safe");
    assert!(keys.contains(&"unknown_alias:b".to_owned()), "{keys:?}");
    for body in [
        "struct c_item *w(int n) { struct c_box *b = c_make(n); if (!b) return 0; c_drop(b); return b->item; }",
        "struct c_item *w(int n) { struct c_box *b = c_make(n); if (!b) return 0; return b->item; }",
    ] {
        // The second body also leaks `b` (a provisional `Unsafe` from the
        // leak check, not from this route): either way never `Safe`, and the
        // return keeps its refusal.
        let (status, keys) = outcome(body);
        assert_ne!(status, "Safe", "{body}");
        assert!(keys.contains(&RETURN.to_owned()), "{body}: {keys:?}");
    }
}

/// An array member returned by decay is an interior address (rule 5j): a
/// derived result this route never answers.
#[test]
fn an_interior_address_return_refuses() {
    if !clang_available() {
        return;
    }
    let body = "struct c_leaf *w(struct c_item *it) { return it->cells; }";
    assert_eq!(outcome(body).0, "Unknown");
}

#[test]
fn a_typed_actual_at_a_borrowed_position_certifies() {
    if !clang_available() {
        return;
    }
    for body in [
        "void w(struct c_box *b, int i) { c_use(b->items[i].count); }",
        "void w(struct c_box *b, int i) { c_use(b->items[i].leaf->weight + 1); }",
        "void w(struct c_box *b, int i) { c_peek(b->items[i].leaf); }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// A custody-taking position keeps the refusal, and so does an uncontracted
/// callee and a pointer read from a root the body binds.
#[test]
fn a_typed_actual_at_a_custody_position_or_uncontracted_callee_refuses() {
    if !clang_available() {
        return;
    }
    for body in [
        "void w(struct c_box *b, int i) { c_forget(b->items[i].leaf); }",
        "void w(struct c_box *b) { c_drop(b->item->leaf); }",
    ] {
        assert_eq!(outcome(body), unknown(&[GATE]), "{body}");
    }
    let uncontracted =
        "int w(struct c_box *b, int i) { int v; v = c_other(b->items[i].count); return v; }";
    assert_eq!(outcome(uncontracted).0, "Unknown");
    let tracked = "void w(int n) { struct c_box *b = c_make(n); if (!b) return; \
        c_peek(b->items[0].leaf); c_drop(b); }";
    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the pointer read on a
    // root the closure binds is now left to the flow, which reads it only
    // as untracked; a tracked root keeps the refusal there (rule 10k(3)'s
    // pointer-actual premise is not relaxed). BEFORE: the gate's refusal.
    let (status, keys) = outcome(tracked);
    assert_eq!(status, "Unknown");
    assert!(keys.contains(&"unknown_alias:b".to_owned()), "{keys:?}");
}

/// A typedef redeclared as C11 permits (librdkafka declares `rd_kafka_Node_t`
/// in `rdkafka.h` and again with its body in `rdkafka_aux.h`): with one
/// declaration an older lexical route admits the getter; with two it falls
/// to the typed reader. Through the `const` pointee `node->host` is a
/// `char *const` value, which the typed reader refused until
/// W47-TYPED-CONST-POINTER-VALUE (rule 10m) erased that top-level `const`
/// (W47-TYPED-ROOT-DECLARATION-READER had measured the declaration premise
/// granted; see `typed_root_declarations.rs`'s
/// `the_redeclared_typedef_control_certifies_with_its_const_pointee`). Renamed
/// from `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal`, whose
/// cause rule 10m removed; the pointer-return refusals it guarded are pinned
/// by `a_return_through_a_released_or_tracked_root_is_not_safe` and by
/// `typed_const_pointer_value.rs`.
#[test]
fn a_redeclared_typedef_formal_reads_its_const_pointer_return() {
    if !clang_available() {
        return;
    }
    let once = "typedef struct g_s g_t; struct g_s { char *host; }; \
        const char *w(const g_t *node) { return node->host; }";
    assert_eq!(outcome(once), safe());
    let twice = "typedef struct g_s g_t; typedef struct g_s { char *host; } g_t; \
        const char *w(const g_t *node) { return node->host; }";
    assert_eq!(outcome(twice), safe());
}

/// Step B, the owner-aware enum arm: an anonymous enum typedef member is a
/// scalar in a condition, a store and a short-circuit operand under a tracked
/// guard; a typedef-hidden union beside it keeps its refusal.
#[test]
fn an_enumeration_member_is_a_scalar_on_positive_evidence() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b) { if (b->mode == C_ON) return 1; return 0; }",
        "void w(struct c_box *b) { b->mode = C_OFF; b->item->leaf->mode = C_ON; }",
        "int w(struct c_box *b) { switch (b->mode) { case C_ON: return 1; default: return 0; } }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    let guarded = "int w(int n) { struct c_box *b = c_make(n); int r = 0; \
        if (b && b->mode == C_ON) r = 1; if (b) c_drop(b); return r; }";
    assert_eq!(outcome(guarded), safe());
    let unguarded = "int w(int n) { struct c_box *b = c_make(n); int r = 0; \
        if (b || b->mode == C_ON) r = 1; if (b) c_drop(b); return r; }";
    assert_eq!(outcome(unguarded), unknown(&["unknown_alias:b"]));
    // Card CG13 flip (rule 10w): the typedef-hidden union beside it is read
    // too. BEFORE: refused (BRANCH).
    assert_eq!(
        outcome(
            "int w(struct c_box *b) { if (b->mode == C_ON && b->hidden->x) return 1; return 0; }"
        ),
        safe()
    );
}
