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

//! Card W47-TYPED-CONST-POINTER-VALUE (gateway rule 10m) — a pointer value
//! whose own type carries a top-level `const` (a pointer member read through
//! a `const` pointee, an element of a `T *const[]` member), and, as its own
//! step, an implicit `void *` bit-cast in a pointer-destination statement.
//!
//! The controls were written before the change and run at the base
//! (`docs/reports/w47-typed-const-pointer-value/`). Each fixture reaches the
//! typed reader only after every older route refused it (an indexed or
//! embedded path; the doubly declared typedef of P3's control is in
//! `access_path_positions.rs`). Every test asserts the
//! status and the complete diagnostic vector of `w`. The fixtures use a
//! generic `c_*` surface: `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

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
extern fn c_peek(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(l);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_use(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_same(a: ptr<const, u8>, b: ptr<const, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(a);\n\
           ensures normal resource_borrowed(b); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef void (*c_fn)(int); \
struct c_leaf { int weight; char *name; }; \
struct c_item { int count; struct c_leaf *leaf; struct c_leaf cells[4]; struct c_leaf *slots[4]; }; \
struct c_conf { void *opaque; char *label; }; \
struct c_box { int flag; struct c_item *item; struct c_conf conf; void *opaque; \
               struct c_leaf *const *pp; }; \
struct c_odd { char *volatile vname; char *const volatile cvname; char *restrict rname; \
               _Atomic(char *) aname; char *const cname; }; \
struct c_odds { struct c_odd rows[2]; }; \
struct c_box *c_make(int n); \
void c_drop(struct c_box *b); \
void c_peek(struct c_leaf *l); \
void c_use(int v); \
int c_same(const char *a, const char *b);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "typed_const_pointer_value.qv".to_owned(),
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
            eprintln!("skipping Clang-dependent const-pointer-value test: {reason}");
            false
        }
    }
}

fn analyze(prefix: &str, body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{prefix}\n{body}");
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
fn outcome_with(prefix: &str, body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(prefix, body);
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

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    outcome_with("", body)
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

/// Step 1: a `T *const` value read through a `const` pointee or as an element
/// of a `T *const[]` member is the same pointer as its unqualified read, in
/// every typed position: a return, a condition operand, a call actual at a
/// `Borrowed` position and a pointer-destination statement. At the base each
/// refused on the value's type alone.
#[test]
fn a_const_qualified_pointer_value_is_read_in_every_position() {
    if !clang_available() {
        return;
    }
    for body in [
        "struct c_leaf *w(const struct c_item *it, int i) { return it->slots[i]; }",
        "void *w(const struct c_box *b) { return b->conf.opaque; }",
        "char *w(const struct c_item *it) { return it->cells[0].name; }",
        "char *w(struct c_odds *s, int i) { return s->rows[i].cname; }",
        "int w(const struct c_item *it, int i) { if (it->slots[i] == 0) return 1; return 0; }",
        "void w(const struct c_item *it, int i) { c_peek(it->slots[i]); }",
        "int w(const struct c_item *it, int i) { struct c_leaf *l = it->slots[i]; c_peek(l); return 0; }",
        "int w(const struct c_item *it, int i) { struct c_leaf *l; l = it->slots[i]; c_peek(l); return 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// Unlisted top-level qualifiers stay refused: `volatile`, `const volatile`,
/// `restrict` and `_Atomic`. Only a lone `const` is erased.
#[test]
fn a_volatile_restrict_or_atomic_pointer_value_refuses() {
    if !clang_available() {
        return;
    }
    for member in ["vname", "cvname", "rname", "aname"] {
        let body = format!("char *w(struct c_odds *s, int i) {{ return s->rows[i].{member}; }}");
        assert_eq!(outcome(&body), unknown(&[RETURN]), "{body}");
    }
}

/// A `T *const *` hop in the middle of a path is read exactly as before: its
/// loaded pointer is the unqualified value Clang's lvalue conversion gives,
/// and the path keeps rule 10k's mixed-path premise. Under a formal root it
/// certifies (as at the base); under a root the body binds it refuses.
#[test]
fn a_const_pointer_middle_hop_keeps_its_premises() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("int w(struct c_box *b) { return b->pp[0]->weight; }"),
        safe()
    );
    // Card CG10 (rule 10t): the tracked root's place is read; its `b->pp`
    // link has no admitted store's evidence, so the flow refuses it.
    let tracked = "int w(int n) { struct c_box *b = c_make(n); int r = 0; if (!b) return 0; \
        if (b->pp[0]->weight) r = 1; c_drop(b); return r; }";
    assert_eq!(outcome(tracked), unknown(&["unknown_alias:b->pp"]));
}

/// Rule 5b's premise is unchanged: a `const`-qualified pointer value whose
/// root the body binds is never handed out, and neither is one whose root is
/// bound from a call in an assignment nested in a condition (closed by
/// `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT`, rule 10o).
#[test]
fn a_const_pointer_value_from_a_bound_root_is_not_safe() {
    if !clang_available() {
        return;
    }
    // The body keeps `b`, so the leak check accuses it (a provisional
    // `Unsafe`, not from this route); the return keeps its refusal.
    for body in [
        "struct c_item *w(int n) { const struct c_box *b = c_make(n); if (!b) return 0; return b->item; }",
        "struct c_leaf *w(int n, int i) { const struct c_box *b = c_make(n); if (!b) return 0; \
            return b->item->slots[i]; }",
    ] {
        assert_eq!(
            outcome(body),
            ("Unsafe", vec!["leak".to_owned(), RETURN.to_owned()]),
            "{body}"
        );
    }
    // Since rule 10o the closure reads the assignment in the condition, so `b`
    // is bound and the return keeps its refusal as well. Flipped by
    // CG04-CONDITION-ACQUISITION (rule 10x): the owned result split on NULL in
    // the condition is lowered, so the branch refusal is gone; the outcome is
    // exactly the assignment-statement pair's (`b = c_make(n); if (!b) …`).
    let nested = "struct c_item *w(int n) { const struct c_box *b; if (!(b = c_make(n))) return 0; \
        return b->item; }";
    assert_eq!(outcome(nested), unknown(&[RETURN]));
    let pair = "struct c_item *w(int n) { const struct c_box *b; b = c_make(n); if (!b) return 0; \
        return b->item; }";
    assert_eq!(outcome(nested), outcome(pair));
}

/// Step 2: `NAME = PATH` where `PATH` is a `void *` member and `NAME` an
/// object pointer is an implicit bit-cast of the same pointer. It is P2's
/// borrowed read; the null test on `NAME` then keeps its identity refusal
/// (rule 10b), as for an uncast pointer.
#[test]
fn a_void_pointer_bit_cast_is_a_borrowed_read() {
    if !clang_available() {
        return;
    }
    let assigned = "int w(const struct c_box *b) { const struct c_leaf *l; \
        if (!(l = b->opaque)) return -1; return l->weight; }";
    assert_eq!(outcome(assigned), unknown(&["unknown_alias:l"]));
    let declared =
        "int w(struct c_box *b) { struct c_leaf *l = b->conf.opaque; c_peek(l); return 0; }";
    assert_eq!(outcome(declared), safe());
}

/// The bit-cast arm admits only an implicit cast from `void *` to an object
/// pointer at the top of a pointer-destination statement: an explicit cast,
/// a function-pointer destination, an explicit cast to an integer, a cast in
/// the middle of a path (in a statement and in a condition) and a bit-cast
/// from another object pointer (in a statement and in a condition
/// assignment) keep the refusal they had at the base.
///
/// CG12 flip (rule 11j): an explicit `void *` view to the destination's own
/// type is read as the implicit one, and a view in the middle of a path over
/// a pointer loaded from a never-bound root is a view step; the three rows
/// below were refused (`GATE`, and `BRANCH` for the condition) BEFORE.
#[test]
fn every_other_cast_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b) { c_fn f = b->conf.opaque; return f != 0; }",
        "int w(struct c_box *b) { long v = (long) b->conf.opaque; return v != 0; }",
        "int w(struct c_box *b, int i) { struct c_leaf *l = (struct c_leaf *) b->item->cells[i].name; c_peek(l); return 0; }",
    ] {
        assert_eq!(outcome(body), unknown(&[GATE]), "{body}");
    }
    let body = "int w(struct c_box *b, int i) { struct c_leaf *l; if (!(l = (struct c_leaf *) b->item->cells[i].name)) return 0; return 1; }";
    assert_eq!(outcome(body), unknown(&[BRANCH]), "{body}");
    for body in [
        "int w(struct c_box *b) { struct c_leaf *l = (struct c_leaf *) b->conf.opaque; c_peek(l); return 0; }",
        "int w(struct c_box *b) { struct c_leaf *l = ((struct c_item *) b->conf.opaque)->leaf; c_peek(l); return 0; }",
        "int w(struct c_box *b, int i) { if (((struct c_item *) b->conf.opaque)->slots[i]) return 1; return 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "CG12 flip: {body}");
    }
    // An explicit view to a type other than the destination's keeps the
    // refusal (a pun between object pointers).
    assert_eq!(
        outcome(
            "int w(struct c_box *b) { struct c_leaf *l = (struct c_leaf *)(struct c_item *) b->conf.opaque; c_peek(l); return 0; }"
        ),
        unknown(&[GATE])
    );
}

/// librdkafka's `rd_kafka_topic_partition_cmp` / `…_cmp_opaque` shape. This
/// card pinned the wrapper `Safe` on a comparator summary (`Owned`, `Owned`)
/// validated with neither formal protected, while the comparator's own row,
/// analysed with that summary in force, protected `_a`/`_b` and refused.
/// `W47-SUMMARY-VALIDATION-PROTECTED-FORMALS` (gateway rule 4b) now validates
/// the summary on the reported premises: it refuses, and the wrapper's call is
/// uncontracted. The fixture keeps this card's const-pointer reads.
#[test]
fn a_comparator_summary_is_withdrawn_where_its_own_row_refuses() {
    if !clang_available() {
        return;
    }
    let comparator = "struct c_tp { char *topic; int partition; }; \
        int cmp(const void *_a, const void *_b) { \
        const struct c_tp *a = _a; const struct c_tp *b = _b; \
        int r = c_same(a->topic, b->topic); if (r) return r; \
        else return (int)((a->partition) < (b->partition) ? -1 : ((a->partition) > (b->partition))); }";
    let result = analyze(
        comparator,
        "static int w(const void *_a, const void *_b, void *opaque) { return cmp(_a, _b); }",
    );
    let rows = &program(&result).functions;
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // comparator's row under its summary no longer refuses (the `?:` arm's
    // reads of the bound copies are read on their own edge), so rule 4b
    // validates the summary on the premises that row reads and the wrapper's
    // call is contracted. BEFORE: the wrapper refused the uncontracted call
    // and the comparator's contract-free row carried four `unknown_alias`
    // leaves (`_a`, `_b`, `a`, `b`).
    assert_eq!(format!("{:?}", rows["cmp"].status), "Safe");
    assert!(
        rows["cmp"].diagnostics.is_empty(),
        "{:?}",
        rows["cmp"].diagnostics
    );
    assert_eq!(format!("{:?}", rows["w"].status), "Safe");
    assert!(
        rows["w"].diagnostics.is_empty(),
        "{:?}",
        rows["w"].diagnostics
    );
}
