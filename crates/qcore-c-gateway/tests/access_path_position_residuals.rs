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

//! Card W47-ACCESS-PATH-POSITION-RESIDUALS (gateway rule 10n) — the P3
//! residual shapes this card admits, each after every older route of its
//! position and rule 10k's typed reader refused, and the negative controls
//! written first for each: an address handed to a custody position or read
//! through a tracked root, an address returned (rule 5j), an operand repeated
//! inside one controlling expression, an anonymous enumeration in a union or
//! behind a pointer, and a connective return whose operand C does not
//! evaluate, which calls, assigns or hands back a pointer.
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
extern fn c_drop(q: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(q, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_lock(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(l);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_seize(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(l, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_use(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_peek(n: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef union c_mtx { int word; long pad; } c_mtx; \
struct c_node { struct c_node *next; int v; }; \
struct c_head { struct c_node *first; struct c_node **last; }; \
struct c_q { c_mtx lock; struct c_head head; int cnt; struct c_node *arr; \
  enum { Q_IDLE, Q_BUSY } state; \
  union { enum { U_A, U_B } k; int n; } u; \
  enum { P_A, P_B } *pstate; \
  struct c_q *peer; }; \
struct c_q *c_make(int n); \
void c_drop(struct c_q *q); \
void c_lock(c_mtx *l); \
void c_seize(c_mtx *l); \
void c_free_lock(c_mtx *l); \
void c_use(int v); \
void c_peek(struct c_node *n); \
int c_count(struct c_q *q);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "access_path_position_residuals.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent position-residuals test: {reason}");
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

// ---------------------------------------------------------------- step 1

/// Negative controls first: an address handed to a custody-taking position,
/// to an uncontracted callee, or read through a root the body binds keeps
/// the refusal; an address is never returned (rule 5j's interior address).
#[test]
fn an_address_keeps_every_custody_and_root_refusal() {
    if !clang_available() {
        return;
    }
    // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the address of a formal's member at a
    // release position names its premise; still `Unknown`.
    assert_eq!(
        outcome("void w(struct c_q *q) { c_seize(&q->lock); }"),
        unknown(&[
            "unsupported:release of `&q->lock`, an address inside the formal `q` \
             (member-address-release-through-formal)"
        ])
    );
    let uncontracted = outcome("void w(struct c_q *q) { c_free_lock(&q->lock); }");
    assert_eq!(uncontracted.0, "Unknown");
    let tracked = "void w(int n) { struct c_q *q = c_make(n); if (!q) return; \
        c_lock(&q->lock); c_drop(q); }";
    let (status, keys) = outcome(tracked);
    assert_eq!(status, "Unknown");
    assert!(keys.contains(&GATE.to_owned()), "{keys:?}");
    assert_eq!(
        outcome("c_mtx *w(struct c_q *q) { return &q->lock; }"),
        unknown(&[RETURN])
    );
    // The callee may write through the address: a pointer member (a place
    // this projection links) and an element are never handed out.
    for body in [
        "void w(struct c_q *q) { c_lock((c_mtx *)&q->peer); }",
        "void w(struct c_q *q, int i) { c_peek(&q->arr[i]); }",
    ] {
        assert_eq!(outcome(body).0, "Unknown", "{body}");
    }
}

/// The soundness interlock (rule 10l's disputed set): a root bound only by a
/// call in an assignment nested in a condition, which the lexical closure
/// does not read, still has every hop of an address checked against the
/// state: after its release the address is never `Safe`.
#[test]
fn an_address_through_a_nested_assignment_root_is_checked_against_the_state() {
    if !clang_available() {
        return;
    }
    let released = "void w(int n) { struct c_q *q; if ((q = c_make(n)) == 0) return; \
        c_drop(q); c_lock(&q->lock); }";
    assert_ne!(outcome(released).0, "Safe");
}

/// `&PATH` handed to a `Borrowed` position: every hop of `PATH` is checked
/// as its read would be, the place itself is not read (a lock of union type
/// is fine), and the callee borrows the interior address.
#[test]
fn an_address_of_a_member_at_a_borrowed_position_certifies() {
    if !clang_available() {
        return;
    }
    for body in [
        "void w(struct c_q *q) { c_lock(&q->lock); }",
        "void w(struct c_q *q) { c_lock(&q->peer->lock); }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// `(&X)->f` is `X.f`, and an address compares as a pointer value.
#[test]
fn an_arrow_through_an_address_and_an_address_comparison_read() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_q *q) { if ((&q->head)->first == 0) return 1; return 0; }",
        "int w(struct c_q *q, struct c_node *c) { if (c < &q->arr[q->cnt]) return 1; return 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    // Card CG13 flip (rule 10w): a union field behind the address is a
    // custody-neutral scalar read. BEFORE: P2's refusal (BRANCH).
    assert_eq!(
        outcome("int w(struct c_q *q) { if ((&q->lock)->word) return 1; return 0; }"),
        safe()
    );
}

// ---------------------------------------------------------------- step 2

/// A condition operand repeated on its line outside the controlling
/// expression (the macro shape `if (x) f(x);`) is matched inside it. Two
/// candidates inside one controlling expression in leaf position keep the
/// refusal.
#[test]
fn an_operand_repeated_on_its_line_is_matched_inside_its_condition() {
    if !clang_available() {
        return;
    }
    let body = "void w(struct c_q *q) { if (q->head.first) c_peek(q->head.first); }";
    assert_eq!(outcome(body), safe());
    // `p->x && p->x->y`: the leaf `p->x` is also the base of the second read,
    // which is not a Boolean leaf position.
    let guard =
        "int w(struct c_q *q) { if (q->head.first && q->head.first->v) return 1; return 0; }";
    assert_eq!(outcome(guard), safe());
    // Two leaves with the same tokens in one controlling expression refuse.
    let twice = "int w(struct c_q *q, int c) { if (c ? q->head.first : q->head.first) return 1; return 0; }";
    assert_eq!(outcome(twice), unknown(&[BRANCH]));
}

// ---------------------------------------------------------------- step 3

/// An inline anonymous enumeration member is a scalar; the same kind of
/// member inside a union, or read through a pointer to an anonymous
/// enumeration, keeps the refusal.
#[test]
fn an_anonymous_enumeration_member_is_a_scalar_only_inline_in_a_struct() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_q *q) { if (q->state == Q_BUSY) return 1; return 0; }",
        "int w(struct c_q *q) { switch (q->state) { case Q_IDLE: return 1; default: return 0; } }",
        "void w(struct c_q *q) { c_use(q->state); }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    assert_eq!(
        outcome("int w(struct c_q *q) { if (q->u.k == U_A) return 1; return 0; }"),
        unknown(&[BRANCH])
    );
    assert_eq!(
        outcome("int w(struct c_q *q) { if (*q->pstate == P_A) return 1; return 0; }"),
        unknown(&[BRANCH])
    );
    let tracked_guard = "int w(int n) { struct c_q *q = c_make(n); int r = 0; \
        if (q && q->state == Q_BUSY) r = 1; if (q) c_drop(q); return r; }";
    assert_eq!(outcome(tracked_guard), safe());
    let tracked_unguarded = "int w(int n) { struct c_q *q = c_make(n); int r = 0; \
        if (q || q->state == Q_BUSY) r = 1; if (q) c_drop(q); return r; }";
    assert_eq!(outcome(tracked_unguarded), unknown(&["unknown_alias:q"]));
}

// ---------------------------------------------------------------- step 4

/// Negative controls first: a connective return that calls, assigns, or
/// hands back a pointer keeps the refusal, and an operand C does not
/// evaluate contributes nothing while one it does evaluate keeps its
/// obligation.
#[test]
fn a_connective_return_keeps_every_refusal_it_does_not_prove() {
    if !clang_available() {
        return;
    }
    let body = "int w(struct c_q *q) { return q->cnt > 0 && c_count(q) > 1; }";
    let (status, keys) = outcome(body);
    assert_eq!(status, "Unknown", "{body}");
    assert!(keys.contains(&RETURN.to_owned()), "{body}: {keys:?}");
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // pointer-valued `?:` return is lowered by branching on `c`, each edge's
    // `return q->head.first;` / `return q->arr;` decided by rule 10k's typed
    // pointer return (never-bound root). BEFORE: the return refusal.
    let (status, keys) =
        outcome("struct c_node *w(struct c_q *q, int c) { return c ? q->head.first : q->arr; }");
    assert_eq!(status, "Safe", "{keys:?}");
    // FLIPPED by CG06 (rule 10z) composed with CG04 (rule 10x): the
    // connective is the deciding operand of the branching, whose `NAME = PATH`
    // leaf the condition desugaring now decides on the edge C evaluates it;
    // each edge returns `1` or `0`. BEFORE: the return refusal.
    let (status, keys) =
        outcome("int w(struct c_q *q, int r) { return q->cnt > 0 && (r = q->head.first->v); }");
    assert_eq!(status, "Safe", "{keys:?}");
    // Evaluation order: `q` released on one path. `q && …` is guarded by C's
    // short circuit only on the null edge, so reading `q->cnt` after the
    // release is never Safe; the unguarded `||` form is never Safe either.
    for body in [
        "int w(int n) { struct c_q *q = c_make(n); if (q) c_drop(q); return q && q->cnt > 0; }",
        "int w(int n) { struct c_q *q = c_make(n); if (q) c_drop(q); return !q || q->cnt > 0; }",
    ] {
        assert_ne!(outcome(body).0, "Safe", "{body}");
    }
}

/// `return A && B;`, `||` and a scalar `?:`: each operand is read only on the
/// edges where C evaluates it, and the one return hands nothing back.
#[test]
fn a_scalar_connective_return_is_its_c_evaluation() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_q *q) { return q->head.first && q->head.first->v > 0; }",
        "int w(struct c_q *q) { return !q->head.first || q->head.first->v > 0; }",
        "int w(struct c_q *q, int c) { return c ? q->arr[0].v : q->cnt; }",
        "int w(struct c_q *q) { return q->state == Q_BUSY || q->cnt == 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    // A tracked guard: `q && q->cnt` reads `q->cnt` only on the non-null
    // edge, where the null branch refined `q`; the `||` form reads it only
    // where `q` is null.
    let guarded = "int w(int n) { struct c_q *q = c_make(n); int r; \
        if (!q) return 0; c_drop(q); q = c_make(n); \
        return q && q->cnt > 0; }";
    let (status, keys) = outcome(guarded);
    assert!(!keys.contains(&"unknown_alias:q".to_owned()), "{keys:?}");
    assert!(!keys.contains(&RETURN.to_owned()), "{keys:?}");
    assert_ne!(status, "Safe", "the fixture leaks `q` on its non-null edge");
    let unguarded = "int w(int n) { struct c_q *q = c_make(n); int r; \
        if (!q) return 0; c_drop(q); q = c_make(n); \
        return q || q->cnt > 0; }";
    let (_, keys) = outcome(unguarded);
    assert!(keys.contains(&"unknown_alias:q".to_owned()), "{keys:?}");
}
