//! Card W47-TYPED-CONST-POINTER-VALUE (gateway rule 10m) — a pointer value
//! whose own type carries a top-level `const` (a pointer member read through
//! a `const` pointee, an element of a `T *const[]` member), and, as its own
//! step, an implicit `void *` bit-cast in a pointer-destination statement.
//!
//! The controls were written before the change and run at the base
//! (`docs/reports/w47-typed-const-pointer-value/`). Each fixture reaches the
//! typed reader only after every older route refused it (an indexed or
//! embedded path, or a doubly declared typedef). Every test asserts the
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
           ensures normal resource_return_none(); };";

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
void c_use(int v);";

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

/// The P3 control's getter (`W47-ACCESS-PATH-POSITIONS`): with the typedef
/// declared twice (C11 permits it; librdkafka does it for `rd_kafka_Node_t`)
/// the older lexical route refuses `node->host` and the typed reader decides.
/// Its value is `char *const` through the `const` pointee, which this card
/// admits, so the getter certifies with and without the `const`.
#[test]
fn a_redeclared_typedef_getter_reads_its_const_pointer_value() {
    if !clang_available() {
        return;
    }
    let twice = "typedef struct g_s g_t; typedef struct g_s { char *host; } g_t;";
    assert_eq!(
        outcome_with(twice, "const char *w(const g_t *node) { return node->host; }"),
        safe()
    );
    assert_eq!(
        outcome_with(twice, "char *w(g_t *node) { return node->host; }"),
        safe()
    );
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
    let tracked = "int w(int n) { struct c_box *b = c_make(n); int r = 0; if (!b) return 0; \
        if (b->pp[0]->weight) r = 1; c_drop(b); return r; }";
    assert_eq!(outcome(tracked), unknown(&[BRANCH]));
}

/// Rule 5b's premise is unchanged: a `const`-qualified pointer value whose
/// root the body binds is never handed out, and neither is one whose root is
/// bound from a call in an assignment nested in a condition (the disputed
/// class `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT` tracks).
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
    let nested = "struct c_item *w(int n) { const struct c_box *b; if (!(b = c_make(n))) return 0; \
        return b->item; }";
    assert_eq!(outcome(nested), unknown(&[BRANCH]));
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
    let declared = "int w(struct c_box *b) { struct c_leaf *l = b->conf.opaque; c_peek(l); return 0; }";
    assert_eq!(outcome(declared), safe());
}

/// The bit-cast arm admits only an implicit cast from `void *` to an object
/// pointer at the top of a pointer-destination statement: an explicit cast,
/// a function-pointer destination, an explicit cast to an integer, a cast in
/// the middle of a path (in a statement and in a condition) and a bit-cast
/// from another object pointer (in a statement and in a condition
/// assignment) keep the refusal they had at the base.
#[test]
fn every_other_cast_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b) { struct c_leaf *l = (struct c_leaf *) b->conf.opaque; c_peek(l); return 0; }",
        "int w(struct c_box *b) { c_fn f = b->conf.opaque; return f != 0; }",
        "int w(struct c_box *b) { long v = (long) b->conf.opaque; return v != 0; }",
        "int w(struct c_box *b) { struct c_leaf *l = ((struct c_item *) b->conf.opaque)->leaf; c_peek(l); return 0; }",
        "int w(struct c_box *b, int i) { struct c_leaf *l = b->item->cells[i].name; c_peek(l); return 0; }",
    ] {
        assert_eq!(outcome(body), unknown(&[GATE]), "{body}");
    }
    for body in [
        "int w(struct c_box *b, int i) { if (((struct c_item *) b->conf.opaque)->slots[i]) return 1; return 0; }",
        "int w(struct c_box *b, int i) { struct c_leaf *l; if (!(l = b->item->cells[i].name)) return 0; return 1; }",
    ] {
        assert_eq!(outcome(body), unknown(&[BRANCH]), "{body}");
    }
}
