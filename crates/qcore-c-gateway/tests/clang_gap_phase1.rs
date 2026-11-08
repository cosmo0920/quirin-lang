//! Card CG00-CLANG-GAP-BASELINE, phase 1 (cards CG01–CG09): executed BEFORE
//! controls for the Clang gap analysis's expression, call-application and
//! typed-source control entries (`docs/plans/clang-gap-cards/README.md` §2).
//!
//! Each test is named after the entry it pins and asserts the route's
//! **current** outcome: the refusal (or limit) the implementation card will
//! flip deliberately. Where the analysis's claim did not reproduce, the test
//! asserts what actually happens and the report lists the entry under
//! "corrections to the input". G is the custody gateway (`legacy_c`); T is
//! `typed_source.rs`. `Safe` is over synthetic trusted fixture contracts.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

/// A record with every field shape the controls read or store through.
const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

const NESTED: &str = "nested call `";
const NESTED_TAIL: &str = "` inside a modeled call is outside the resource projection";

fn nested(callee: &str) -> String {
    format!("{NESTED}{callee}{NESTED_TAIL}")
}

// ---------------------------------------------------------------------------
// CG01 — T operator closure (E01 E02 E03(T) E14)
// ---------------------------------------------------------------------------

/// The refusal `import_source_functions` gives for a node CG01 added to the
/// importer but a legacy consumer does not classify.
fn legacy(node: &str) -> String {
    format!("typed-source node `{node}` is not classified by the legacy typed-source consumer")
}

// After CG01-T-OPERATOR-CLOSURE (merged at 7e7ba401) the importer reads every
// E01/E02/E03(T)/E14 node; only `import_monkey_wrappers` admits them, and the
// legacy consumer behind `import_source_functions` refuses each by name. These
// controls pin that post-CG01 state (the BEFORE refusals are in CG01's report).

#[test]
fn e01_t_legacy_consumer_refuses_six_binary_operators_by_name() {
    if !clang_available() {
        return;
    }
    for operator in ["-", "<=", ">=", "^", "<<", ">>"] {
        let source = format!("int subject(int a, int b) {{ return a {operator} b; }}");
        assert_eq!(t_refusal(&source), legacy(&format!("binary {operator}")));
    }
    // Control: `+` is in T's allowlist.
    assert!(t("int subject(int a, int b) { return a + b; }").is_ok());
    // G: the typed scalar reader takes all six over a never-bound field root.
    let run = gs(
        "int subject(struct S *s) { int x = s->a - s->b; x = s->a <= s->b; \
                  x = s->a >= s->b; x = s->a ^ s->b; x = s->a << s->b; x = s->a >> s->b; return x; }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn e02_t_legacy_consumer_refuses_unary_plus_and_complement_by_name() {
    if !clang_available() {
        return;
    }
    for operator in ["+", "~"] {
        let source = format!("int subject(int a) {{ return {operator}a; }}");
        assert_eq!(t_refusal(&source), legacy(&format!("unary {operator}")));
    }
    assert!(t("int subject(int a) { return -a; }").is_ok());
    let run = gs("int subject(struct S *s) { int x = +s->a; x = ~s->a; return x; }");
    assert!(run.safe(), "{run:?}");
}

#[test]
fn e03_t_legacy_consumer_admits_only_int_or_assign() {
    if !clang_available() {
        return;
    }
    for operator in ["+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "^="] {
        let source = format!("int subject(int a) {{ int x = 1; x {operator} a; return x; }}");
        assert_eq!(t_refusal(&source), legacy(&format!("compound {operator}")));
    }
    assert!(t("int subject(int a) { int x = 1; x |= a; return x; }").is_ok());
}

/// Correction to the input: the analysis says G "already has scalar compound
/// assignments". On local scalars eight of the ten did; `*=` and `&=` were
/// refused as an unrecognized destination, and every one of them was refused
/// (IND) once its right operand read a field.
///
/// **Flipped by CG02-ELABORATION-CORE (rule 10s):** the elaborator is asked
/// after those refusals, so `x *= a`, `x &= a`, `x op= s->a` for all ten, and
/// the field destination `s->a += 1` are now admitted (a single place
/// evaluation: one typed read, one typed write). The eight older admissions
/// keep their older route.
#[test]
fn e03_g_scalar_compound_assignments_are_partial() {
    if !clang_available() {
        return;
    }
    for operator in ["+=", "-=", "/=", "%=", "<<=", ">>=", "^=", "|="] {
        let run = gs(&format!(
            "int subject(int a) {{ int x = 1; x {operator} a; return x; }}"
        ));
        assert!(run.safe(), "{operator}: {run:?}");
    }
    // CG02 flip: formerly `DESTINATION`.
    for operator in ["*=", "&="] {
        let run = gs(&format!(
            "int subject(int a) {{ int x = 1; x {operator} a; return x; }}"
        ));
        assert!(run.safe(), "{operator}: {run:?}");
    }
    // CG02 flip: formerly `IND`.
    for operator in ["+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "^=", "|="] {
        let run = gs(&format!(
            "int subject(struct S *s) {{ int x = 1; x {operator} s->a; return x; }}"
        ));
        assert!(run.safe(), "{operator}: {run:?}");
    }
    // CG02 flip: a field destination, formerly `DESTINATION`.
    assert!(gs("void subject(struct S *s) { s->a += 1; }").safe());
    // The typed obligations still stand: a released base refuses the access.
    let run = gs("void subject(void) { struct S *s = q_alloc(); q_free(s); s->a += 1; }");
    assert!(!run.safe(), "{run:?}");
    // Pointer steps by a literal are the admitted pointer form.
    assert!(
        gs("void subject(void) { char *p = q_alloc(); char *q = p; q += 1; q_free(p); }").safe()
    );
}

#[test]
fn e14_t_legacy_consumer_refuses_floating_and_character_literals_by_name() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(void) { return 1.5 == 0.0; }"),
        legacy("literal floating")
    );
    assert_eq!(
        t_refusal("int subject(void) { return 'a'; }"),
        legacy("literal character")
    );
}

/// Correction to the input: G reads a floating literal and a character literal
/// in a comparison or a return, but a `char` local initialised from a
/// character literal binds the literal as an unresolved resource identity.
#[test]
fn e14_g_reads_literals_except_a_character_initializer() {
    if !clang_available() {
        return;
    }
    assert!(gs("double subject(void) { double d = 1.5; return d; }").safe());
    assert!(gs("int subject(void) { return 'a'; }").safe());
    assert!(gs("int subject(struct S *s) { return s->a == 'a'; }").safe());
    let run = gs("int subject(void) { char c = 'a'; return c; }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.has("UnknownAlias { variable: \"__qcore_literal__\" }"));
}

// ---------------------------------------------------------------------------
// CG02 — elaboration core (E03 values, E04, E05, E08, E12 order model)
// ---------------------------------------------------------------------------

/// The refusal route T gives a value form after CG02 (rule 10s): named, since
/// no typed-source consumer decides one.
fn t_value_form(form: &str) -> String {
    format!("typed-source value form `{form}` is decided by no typed-source consumer (CG02)")
}

/// **Flipped by CG02-ELABORATION-CORE:** T now names the form; G admits both
/// chains and the returned assignment. `p = (q = q_alloc())` lowers to the
/// classifier's own `q = q_alloc()` then `p = q` (the two stores are to
/// distinct objects, so their unsequenced order is immaterial), and
/// `return (p = q_alloc())` to `p = q_alloc()` then `return p`, which escapes
/// the acquired object exactly as the two statements would.
#[test]
fn e04_assignment_as_a_value_is_refused_on_both_routes() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { int x; int y; x = (y = a); return x; }"),
        t_value_form("assignment value")
    );
    assert_eq!(
        t_refusal("int subject(int a) { int x; return (x = a); }"),
        t_value_form("assignment value")
    );
    // G: a chained owned assignment, and an assignment returned as a value.
    let run = gs("void subject(void) { void *p; void *q; p = (q = q_alloc()); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    // CFG nodes are pushed last statement first.
    assert_eq!(run.calls, ["q_free", "q_alloc"]);
    let run = gs("void *subject(void) { void *p; return (p = q_alloc()); }");
    assert!(run.safe(), "{run:?}");
    // A scalar chain beside a live resource.
    let run = gs("int subject(void) { void *p = q_alloc(); int x; int y; \
                  x = (y = r_borrow_code(p)); q_free(p); return x; }");
    assert!(run.safe(), "{run:?}");
    // The obligations of the pieces stand: the chain's acquisition leaks.
    let run = gs("void subject(void) { void *p; void *q; p = (q = q_alloc()); }");
    assert!(!run.safe(), "{run:?}");
    // The same place on both sides of an unsequenced pair stays refused.
    let run = gs("int subject(struct S *s) { s->a = (s->a = 1); return s->a; }");
    assert!(!run.safe(), "{run:?}");
    // Finding (CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED) discharged by card
    // CHAINED-ASSIGNMENT-INNER-DESTINATION (rule 5i, amended): the older
    // scalar route admitted `x = (x = 1)` as one scalar write before the
    // elaborator was asked. The chain guard now refuses that admission by
    // name, and the elaborator's order model refuses the same-object pair.
    let run = gs("int subject(int x) { x = (x = 1); return x; }");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(
            "chained assignment writes an inner destination too; it is read only as its \
             sequential assignments (C11 6.5.16p3)"
        ),
        "{run:?}"
    );
}

/// **Flipped by CG02-ELABORATION-CORE** where the value is a scalar and no
/// call takes it: `int j = i++;` (a `ForgetScalar` for `i`, then for `j`) and
/// `int j = s->a++;` (the typed read and write of `s->a`, then `j`). T names
/// the form. **Flipped by CG03-CALL-SUMMARY-INSTANTIATION** (rule 10u):
/// `cg_sink(i++)` is the increment, sequenced before the call, then the call's
/// one contract application.
#[test]
fn e05_value_producing_increments_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { int j = a++; return j; }"),
        t_value_form("post increment")
    );
    assert_eq!(
        t_refusal("int subject(int a) { int j = ++a; return j; }"),
        t_value_form("pre increment")
    );
    assert!(t("int subject(int a) { a++; return a; }").is_ok());
    // G: the value of `i++` in an initializer (flipped by CG02) and an actual
    // (flipped by CG03-CALL-SUMMARY-INSTANTIATION, rule 10u: the increment is
    // sequenced before the call and the scalar value is data at a borrowed
    // formal; BEFORE this was the elaborator's `EffectfulActual` refusal).
    let run = gs("void subject(void) { void *p = q_alloc(); int i = 0; int j = i++; q_free(p); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { void *p = q_alloc(); int i = 0; cg_sink(i++); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    assert!(gs("int subject(struct S *s) { int j = s->a++; return j; }").safe());
    // Standalone scalar and pointer steps are admitted.
    assert!(
        gs("void subject(void) { void *p = q_alloc(); int i = 0; i++; ++i; i--; q_free(p); }")
            .safe()
    );
    assert!(gs("void subject(void) { char *p = q_alloc(); char *q = p; q++; q_free(p); }").safe());
    // A pointer step inside a larger value stays refused (pointer arithmetic,
    // CG16); the older stepped-access route owns `int c = *q++;` itself.
    let run = gs(
        "int subject(void) { char *p = q_alloc(); char *q = p; int c = 0; \
                  c = q[0] + *q++; q_free(p); return c; }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// **Flipped by CG02-ELABORATION-CORE:** each comma operand is a statement of
/// its own, decided by the existing lowering, in order; the last one gives the
/// value. T names the form.
#[test]
fn e08_comma_is_refused_in_value_positions() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a, int b) { return (a, b); }"),
        t_value_form("comma")
    );
    let run =
        gs("void subject(void) { void *p = q_alloc(); int x = (q_observe(p), 1); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    assert_eq!(run.calls, ["q_free", "q_observe", "q_alloc"]);
    let run = gs("int subject(void) { void *p = q_alloc(); return (q_free(p), 0); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { void *p = q_alloc(); q_observe(p), q_free(p); }");
    assert!(run.safe(), "{run:?}");
    assert_eq!(run.calls, ["q_free", "q_observe", "q_alloc"]);
    // The sequence is checked: a use after the release in the right operand.
    let run = gs("void subject(void) { void *p = q_alloc(); q_free(p), q_observe(p); }");
    assert!(!run.safe(), "{run:?}");
    // A returned comma whose left operand releases the returned object.
    let run = gs("void *subject(void) { void *p = q_alloc(); return (q_free(p), p); }");
    assert!(!run.safe(), "{run:?}");
}

/// CG02's refusal controls: C leaves these pairs unsequenced on one object
/// (undefined behaviour), so the order model refuses them and G keeps its
/// refusal byte for byte (`elaboration::tests` pins the model's reason).
#[test]
fn e12_unsequenced_same_place_pairs_stay_refused() {
    if !clang_available() {
        return;
    }
    let run = gs("int subject(int i) { i = i++; return i; }");
    assert!(run.refused(INCREMENT), "{run:?}");
    let run = gs("int subject(int i, int *a) { a[i] = i++; return i; }");
    assert!(run.refused(INCREMENT), "{run:?}");
    let run = gs("int cg_f(int, int); int subject(int x) { return cg_f(x++, x); }");
    assert!(!run.safe(), "{run:?}");
    // The disjoint pair is admitted.
    assert!(gs("int subject(int i) { int j = 0; j = i++; return j + i; }").safe());
}

#[test]
fn e12_t_refuses_unsequenced_calls_and_reads() {
    if !clang_available() {
        return;
    }
    const UNSEQUENCED: &str = "unsequenced call/read operands require an explicit ordering model";
    assert_eq!(
        t_refusal("int f(int); int g(int); int subject(int a) { return f(a) + g(a); }"),
        UNSEQUENCED
    );
    assert_eq!(
        t_refusal("int f(int); int subject(int *a) { return f(1) + *a; }"),
        UNSEQUENCED
    );
    // Sequenced by statements, the same calls are admitted.
    assert!(t("int f(int); int g(int); int subject(int a) { int x = f(a); int y = g(a); return x + y; }").is_ok());
}

// ---------------------------------------------------------------------------
// CG03 — call-summary instantiation (K12, K03)
// ---------------------------------------------------------------------------

#[test]
fn k12_one_contract_is_accepted_in_one_position_and_refused_in_another() {
    if !clang_available() {
        return;
    }
    // `r_wrap` (borrowed parameter, owned result) as a statement: applied.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); void *q = r_wrap(p); q_free(q); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // Returned: applied (the leak of `p` is real).
    let run = gs("void *subject(void) { void *p = q_alloc(); return r_wrap(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    // In a condition: refused (CG04).
    let run = gs("void subject(void) { void *p = q_alloc(); if (r_wrap(p)) { } q_free(p); }");
    assert!(run.refused(BRANCH), "{run:?}");
    // As an actual: applied since CG05-NESTED-CALL-EFFECTS (rule 10y); the
    // wrapper's custody, only borrowed, is a reported leak.
    let run = gs("void subject(void) { void *p = q_alloc(); q_observe(r_wrap(p)); q_free(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
}

#[test]
fn k03_complex_actual_places_are_not_substituted() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s) { q_free(s->in.p); }",
        "void subject(struct S *s, int i) { q_free(s->slots[i]); }",
        "void subject(struct S *s) { q_make(&s->p); }",
    ] {
        let run = gs(body);
        assert!(run.refused(IND), "{body}: {run:?}");
    }
    // Control: address-of a local slot is the admitted Out destination.
    assert!(gs("void subject(void) { void *p; q_make(&p); q_free(p); }").safe());
}

// ---------------------------------------------------------------------------
// CG04 — condition acquisition (E09 E13)
// ---------------------------------------------------------------------------

/// **Flipped by CG04-CONDITION-ACQUISITION (rule 10x)** for the three
/// branch forms: the acquisition, its binding, the NULL (or sentinel) split
/// and the merge are lowered together where the condition stands, and each is
/// `Safe` exactly as its statement pair is. The loop form kept its refusal
/// until **CG04-LOOP-SITE-GENERATIONS** gave the flow generation-indexed sites
/// and lifted the placement guard: it is now admitted, applied once in the
/// condition, and decided as its statement pair `p = q_alloc(); while (p != 0)
/// { q_free(p); p = q_alloc(); }` is: the value route calls the exit edge of a
/// non-null result infeasible, the edge is retained (rule
/// SCALAR-ROUTE-EXIT-EDGE), and neither is `Safe`.
#[test]
fn e09_an_owned_acquisition_in_a_condition_is_refused_and_not_hoisted() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p; if ((p = q_alloc()) != 0) { q_free(p); }",
        "void *p; if ((p = q_maybe_alloc()) != 0) { q_free(p); }",
        "int fd; if ((fd = k_open(\"x\")) >= 0) { k_close(fd); }",
    ] {
        let run = gs(&format!("void subject(void) {{ {body} }}"));
        assert!(run.safe(), "{body}: {run:?}");
        // Applied once, where the condition stands.
        assert_eq!(
            run.calls
                .iter()
                .filter(|c| *c == "q_alloc" || *c == "q_maybe_alloc" || *c == "k_open")
                .count(),
            1,
            "{body}"
        );
    }
    let body = "void *p; while ((p = q_alloc()) != 0) { q_free(p); }";
    let run = gs(&format!("void subject(void) {{ {body} }}"));
    assert!(!run.refused(LOOP), "{body}: {run:?}");
    assert!(!run.safe());
    let pair = gs(
        "void subject(void) { void *p = q_alloc(); while (p != 0) { q_free(p); p = q_alloc(); } }",
    );
    assert!(!pair.safe(), "{pair:?}");
    // The acquisition was not lifted out of the condition: applied once, in
    // the condition's plan.
    assert_eq!(run.calls.iter().filter(|c| *c == "q_alloc").count(), 1);
    // The normalised form is admitted.
    assert!(gs("void subject(void) { void *p = q_alloc(); if (p != 0) { q_free(p); } }").safe());
}

/// Partial correction to the input: of the condition-leaf forms E13 lists,
/// increments, `sizeof`, expression callees and field reads in a leaf are
/// refused, but nested connectives, a scalar cast of a call, an assigned
/// scalar call and a selection-invariant `_Generic` are admitted.
///
/// **Flipped by CG04-CONDITION-ACQUISITION (rule 10x):** a type operator whose
/// operand holds no call is residual text of its leaf
/// (`r_borrow_code(p) == sizeof(int)`), and an increment in a call-free leaf
/// of its own is the elaborator's. An increment beside a call in one leaf
/// (CG05), a field read in a call's residual (CG10's root premise), an
/// uncontracted call and an expression callee (CG22) stay refused.
#[test]
fn e13_condition_leaf_composition_is_partial() {
    if !clang_available() {
        return;
    }
    for condition in [
        "r_borrow_code(p) == sizeof(int)",
        "i++ < 4 && r_borrow_code(p)",
    ] {
        let run = gs(&format!(
            "void subject(struct S *s) {{ void *p = q_alloc(); int i = 0; if ({condition}) {{ }} q_free(p); }}"
        ));
        assert!(run.safe(), "{condition}: {run:?}");
    }
    for condition in [
        "r_borrow_code(p) == i++",
        "r_borrow_code(p) == s->a",
        "!(r_borrow_code(p) == 0 || (r_borrow_code(p) == 1 && opaque()))",
    ] {
        let run = gs(&format!(
            "void subject(struct S *s) {{ void *p = q_alloc(); int i = 0; if ({condition}) {{ }} q_free(p); }}"
        ));
        assert!(run.refused(BRANCH), "{condition}: {run:?}");
    }
    let run =
        gs("void subject(int (*f)(void *)) { void *p = q_alloc(); if (f(p)) { } q_free(p); }");
    assert!(run.refused(BRANCH), "{run:?}");
    for condition in [
        "(long)r_borrow_code(p)",
        "(a || b) && r_borrow_code(p)",
        "(c = r_borrow_code(p)) && c > 1",
        "_Generic(0, int: r_borrow_code(p), default: r_borrow_code(p))",
    ] {
        let run = gs(&format!(
            "void subject(int a, int b) {{ void *p = q_alloc(); int c; if ({condition}) {{ }} q_free(p); }}"
        ));
        assert!(run.safe(), "{condition}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// CG05 — nested-call effects (E10 E11 E12)
// ---------------------------------------------------------------------------

// Flipped deliberately by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): the
// E10/E11 controls now pin the composition, and E12's operator-initializer
// control the scalar store; the non-commuting siblings stay refused.

#[test]
fn e10_non_identity_effects_of_a_nested_call_are_composed() {
    if !clang_available() {
        return;
    }
    // Example B: `sink(release_and_status(p))`: the inner release, then the
    // outer entry.
    let run = gs("void subject(void) { void *p = q_alloc(); cg_sink(r_release_code(p)); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { void *p = q_alloc(); cg_sink(r_consume_code(p)); }");
    assert!(run.safe(), "{run:?}");
    // An owned inner result, bound and released by the outer call.
    let run = gs("void subject(void) { void *p = q_alloc(); q_free(r_wrap(p)); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    // Control: a borrowed inner call with a `None` result is composed.
    assert!(
        gs("void subject(void) { void *p = q_alloc(); cg_sink(r_borrow_code(p)); q_free(p); }")
            .safe()
    );
}

#[test]
fn e11_deeper_and_partial_nested_calls_are_composed() {
    if !clang_available() {
        return;
    }
    // Depth two: applied inside-out, and the wrapper's fresh custody, only
    // borrowed, is a reported leak.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(p))); q_free(p); }",
    );
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    assert!(!run.refused(&nested("r_borrow_code")), "{run:?}");
    let run =
        gs("void subject(void) { void *p = q_alloc(); cg_sink(r_borrow_code(p) + 1); q_free(p); }");
    assert!(run.safe(), "{run:?}");
}

#[test]
fn e12_g_sibling_calls_compose_only_when_their_footprints_commute() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void subject(void) { void *p = q_alloc(); void *q = q_alloc(); \
                  cg_sink2(r_release_code(p), r_release_code(q)); }",
    );
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
    let run = gs(
        "void subject(void) { void *p = q_alloc(); cg_sink2(r_release_code(p), r_borrow_code(p)); }",
    );
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
    // Two identity siblings as actuals are admitted.
    assert!(gs("void subject(void) { void *p = q_alloc(); cg_sink2(r_borrow_code(p), r_borrow_code(p)); q_free(p); }").safe());
    // Under an operator in an initializer they are now composed too.
    let run = gs(
        "int subject(void) { void *p = q_alloc(); int x = r_borrow_code(p) + r_borrow_code(p); q_free(p); return x; }",
    );
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG06 — value-producing connectives and `?:` (E06 E07)
// ---------------------------------------------------------------------------

#[test]
fn e06_short_circuit_values_are_refused_outside_conditions() {
    if !clang_available() {
        return;
    }
    const VALUES: &str = "short-circuit values require a supported branch condition";
    assert_eq!(
        t_refusal("int subject(int a, int b) { int ok = a && b; return ok; }"),
        VALUES
    );
    assert_eq!(
        t_refusal("int subject(int a, int b) { return a || b; }"),
        VALUES
    );
    assert!(t("int subject(int a, int b) { if (a && b) return 1; return 0; }").is_ok());
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // initializer and the actual are lowered by branching on the connective
    // (G only; T above is unchanged). BEFORE: "resource call with a formal
    // embedded in an unmodeled expression" and the nested-call refusal.
    let run = gs(
        "void subject(int flag) { void *p = q_alloc(); int ok = flag && r_borrow_code(p); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    let run = gs(
        "void subject(int flag) { void *p = q_alloc(); cg_sink(flag || r_borrow_code(p)); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // Unchanged: an uncontracted call on one edge keeps the refusal.
    let run =
        gs("int subject(int flag) { void *p = q_alloc(); q_free(p); return flag && opaque(); }");
    assert!(run.refused(RETURN), "{run:?}");
    // Control: the same connective in a condition.
    assert!(
        gs(
            "void subject(int a) { void *p = q_alloc(); if (a && r_borrow_code(p)) { } q_free(p); }"
        )
        .safe()
    );
}

#[test]
fn e07_conditional_values_are_refused_outside_conditions() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a, int b) { return a ? b : 0; }"),
        "unmodeled callback expression ConditionalOperator"
    );
    assert_eq!(
        t_refusal("int subject(int a, int b) { return a ?: b; }"),
        "unmodeled callback expression BinaryConditionalOperator"
    );
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z), G only:
    // * the pointer `?:` initializer is lowered by branching; the flow's
    //   existing join then makes `q` possibly null with `p`'s target, which
    //   marks `p` nullable, exactly as `if (flag) q = p; else q = 0;` does
    //   (BEFORE: the assignment refusal). Still Unknown, never Safe;
    // * the return selects exactly one release per edge: Safe (BEFORE: the
    //   return refusal);
    // * the actual's call is applied on one edge only: Safe (BEFORE: the
    //   nested-call refusal).
    let run =
        gs("void subject(int flag) { void *p = q_alloc(); void *q = flag ? p : 0; q_free(p); }");
    assert!(
        run.refused("nullable resource `p` used by a non-null contract"),
        "{run:?}"
    );
    let branched = gs(
        "void subject(int flag) { void *p = q_alloc(); void *q; if (flag) q = p; else q = 0; q_free(p); }",
    );
    assert_eq!(branched.diagnostics, run.diagnostics);
    let run = gs(
        "int subject(int flag) { void *p = q_alloc(); return flag ? r_release_code(p) : r_consume_code(p); }",
    );
    assert!(run.safe(), "{run:?}");
    let run = gs(
        "void subject(int flag) { void *p = q_alloc(); cg_sink(flag ? r_borrow_code(p) : 0); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // Controls: a top-level condition and a scalar GNU `?:` initializer.
    assert!(gs("void subject(int flag) { void *p = q_alloc(); if (flag ? r_release_code(p) : r_consume_code(p)) { } }").safe());
    assert!(
        gs("void subject(int flag) { void *p = q_alloc(); int x = flag ?: 1; q_free(p); }").safe()
    );
}

// ---------------------------------------------------------------------------
// CG07 — `_Generic` and statement expressions (E17 E19)
// ---------------------------------------------------------------------------

/// **Flipped by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g):** G's
/// selection-invariance route (W44) refused differing associations as a call
/// to an uncontracted function `_Generic`; Clang's selected association is
/// now its fallback, so both selections below read the selected call (the
/// release is evaluated where it is selected). Route T is unchanged.
#[test]
fn e17_only_selection_invariant_generics_are_read() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { return _Generic(a, int: 1, default: 2); }"),
        "unmodeled callback expression GenericSelectionExpr"
    );
    // CG07 flip: formerly `call to uncontracted function \`_Generic\``.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); int x = _Generic(0, int: r_borrow_code(p), default: 0); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    let run = gs(
        "void subject(int a) { void *p = q_alloc(); int x = _Generic(a, int: r_release_code(p), default: 0); }",
    );
    assert!(run.safe(), "{run:?}");
    // Control: identical associations (route 1, unchanged).
    assert!(gs("void subject(void) { void *p = q_alloc(); int x = _Generic(0, int: r_borrow_code(p), default: r_borrow_code(p)); q_free(p); }").safe());
}

/// **Flipped by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g):** a
/// statement expression that is a declaration's whole initializer is lowered
/// as its block's statements, then the declaration of its last expression.
/// Route T is unchanged.
#[test]
fn e19_statement_expressions_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { return ({ int t = a; t; }); }"),
        "unmodeled callback expression StmtExpr"
    );
    // CG07 flip: formerly `INITIALIZER`.
    for body in [
        "void *p = q_alloc(); int x = ({ int t = r_borrow_code(p); t; }); q_free(p);",
        "void *p = q_alloc(); int x = ({ q_free(p); 0; });",
    ] {
        let run = gs(&format!("void subject(void) {{ {body} }}"));
        assert!(run.safe(), "{body}: {run:?}");
    }
    // Still refused: an operand position.
    let run = gs("void subject(void) { void *p = q_alloc(); int x = 1 + ({ q_free(p); 0; }); }");
    assert!(run.refused(INITIALIZER), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG08 — casts and type queries (E15 E16 N11 M18 N08)
// ---------------------------------------------------------------------------

#[test]
fn e15_t_cast_kinds_outside_the_six_are_refused() {
    if !clang_available() {
        return;
    }
    for (source, reason) in [
        (
            "unsigned long subject(void *p) { return (unsigned long)p; }",
            "unmodeled callback expression CStyleCastExpr",
        ),
        (
            "int subject(int a) { return (_Bool)a; }",
            "unmodeled callback expression CStyleCastExpr",
        ),
        (
            "int subject(int a) { return (int)(double)a; }",
            "unmodeled callback expression CStyleCastExpr",
        ),
        (
            "int f(int); int subject(int a) { (void)f(a); return 0; }",
            "unmodeled callback statement CStyleCastExpr",
        ),
    ] {
        assert_eq!(t_refusal(source), reason, "{source}");
    }
    // G refuses pointer<->integer casts in assignments.
    let run = gs(
        "unsigned long subject(void) { void *p = q_alloc(); unsigned long v = (unsigned long)p; q_free(p); return v; }",
    );
    assert!(run.refused(ASSIGNMENT), "{run:?}");
    let run = gs("void subject(unsigned long v) { void *p = (void *)v; q_free(p); }");
    assert!(run.refused(ASSIGNMENT), "{run:?}");
}

/// Partial correction to the input: G's `sizeof(*p)` reading holds in a call
/// actual; in an initializer it is refused (IND) and in a return (RET).
#[test]
fn e16_type_queries_are_read_only_in_fixed_shapes() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("unsigned long subject(int a) { return sizeof(a); }"),
        "unmodeled callback expression UnaryExprOrTypeTraitExpr"
    );
    assert_eq!(
        t_refusal("unsigned long subject(void) { return _Alignof(int); }"),
        "unmodeled callback expression UnaryExprOrTypeTraitExpr"
    );
    assert!(t("unsigned long subject(void) { return sizeof(int); }").is_ok());
    assert!(
        gs("void subject(struct S *s) { void *p = q_alloc(); cg_sink(sizeof(*s)); q_free(p); }")
            .safe()
    );
    // Flipped by CG08-CASTS-AND-TYPE-QUERIES (rule 11b): the initializer and
    // the return read the unevaluated operand from Clang's node (no event),
    // and the evaluated VLA operand's size expression `n` is read once.
    assert!(
        gs("unsigned long subject(struct S *s) { unsigned long z = sizeof(*s); return z; }").safe()
    );
    assert!(gs("unsigned long subject(struct S *s) { return sizeof(*s); }").safe());
    let run = gs(
        "unsigned long subject(int n) { void *p = q_alloc(); unsigned long z = sizeof(int[n]); q_free(p); return z; }",
    );
    assert!(run.safe(), "{run:?}");
    // An evaluated operand with an effect stays refused.
    let run = gs(
        "unsigned long subject(int n) { void *p = q_alloc(); unsigned long z = sizeof(int[n++]); q_free(p); return z; }",
    );
    assert!(run.unknown(), "{run:?}");
}

#[test]
fn n11_type_queries_carry_no_evaluated_value() {
    if !clang_available() {
        return;
    }
    // T keeps `sizeof(int)` symbolic: the value is a `SizeOf` node, never a
    // host-evaluated number.
    let summary = t("unsigned long subject(void) { return sizeof(int); }").expect("admitted");
    let rendered = format!("{:?}", summary.paths());
    assert!(
        rendered.contains("SizeOf { argument_type: \"int\" }"),
        "{rendered}"
    );
    assert!(!rendered.contains("Integer { value: \"4\" }"), "{rendered}");
}

#[test]
fn m18_pointer_integer_round_trips_are_not_modelled() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("unsigned long subject(void *p) { return (unsigned long)p; }"),
        "unmodeled callback expression CStyleCastExpr"
    );
    // G reads a tag test and a tagged pointer as custody-neutral scalars: no
    // width, loss or provenance is carried.
    assert!(gs("unsigned long subject(void *p) { return (unsigned long)p & 7; }").safe());
    assert!(gs("void *subject(void *p) { return (void *)((unsigned long)p | 1); }").safe());
}

#[test]
fn n08_floating_conversions_have_no_representation_in_t() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { return (int)(double)a; }"),
        "unmodeled callback expression CStyleCastExpr"
    );
    // G reads the conversion as a neutral scalar: no range or rounding fact.
    assert!(gs("int subject(double d) { return (int)d; }").safe());
}

// ---------------------------------------------------------------------------
// CG09 — typed-source control (C01 C02 C03)
// ---------------------------------------------------------------------------

// After CG09-TYPED-SOURCE-CONTROL (rule 11l) route T reads every C01–C03
// statement. The legacy consumer behind `import_source_functions` refuses each
// by name; an admitting consumer (`t_control`) lowers the acyclic forms, and
// every back-edge stays refused in every consumer (no invariant or summary:
// CG27). The BEFORE refusals are in CG09's report.

/// T over one function `subject` under a consumer admitting every acyclic
/// control form (as the Monkey wrapper projection does).
fn t_control(source: &str) -> Result<qcore_c_gateway::typed_source::SourceSummary, String> {
    use qcore_c_gateway::legacy_c::C11TranslationUnit;
    use qcore_c_gateway::typed_source::{
        ControlAdmission, SourceImportProfile, SourceResult,
        import_source_functions_admitting_control,
    };
    use qcore_c_gateway::typed_source_nodes::NodeAdmission;
    const ACYCLIC: ControlAdmission = ControlAdmission::new(
        "CG00 control consumer",
        &["switch", "break", "continue", "goto"],
    );
    let results = import_source_functions_admitting_control(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "cg00",
        &["subject"],
        &SourceImportProfile {
            records: &[],
            sizeof_types: T_SIZEOF,
            signature_constraints: &[],
            restricted_calls: &[],
        },
        &NodeAdmission::LEGACY,
        &ACYCLIC,
    )
    .unwrap();
    match results.into_iter().next() {
        Some(SourceResult::Summary(summary)) => Ok(summary),
        Some(SourceResult::Unknown { reason, .. }) => Err(reason),
        None => panic!("T returned no result"),
    }
}

fn not_admitted(form: &str, consumer: &str) -> String {
    format!("typed-source control `{form}` is not admitted by the {consumer}")
}

fn back_edge(form: &str, consumer: &str) -> String {
    format!(
        "typed-source control `{form}` has a back-edge, and the {consumer} supplies no \
         invariant or summary for it (CG27)"
    )
}

const LEGACY_T: &str = "legacy typed-source consumer";

#[test]
fn c01_t_reads_general_loops_and_refuses_each_back_edge_by_name() {
    if !clang_available() {
        return;
    }
    let while_loop = "int subject(int a) { while (a) { a = 0; } return a; }";
    let for_loop = "int subject(int a) { int i; for (i = 0; i < a; i++) { a = 0; } return a; }";
    let do_loop = "int subject(int a) { do { a = 0; } while (a); return a; }";
    assert_eq!(t_refusal(while_loop), back_edge("loop while", LEGACY_T));
    assert_eq!(
        t_refusal(for_loop),
        format!(
            "{}; not a canonical scan: scan body requires one conditional early return",
            back_edge("loop for", LEGACY_T)
        )
    );
    assert_eq!(t_refusal(do_loop), back_edge("loop do", LEGACY_T));
    // An admitting consumer refuses them too: no invariant or summary exists.
    assert_eq!(
        t_control(while_loop).unwrap_err(),
        back_edge("loop while", "CG00 control consumer")
    );
    assert!(
        t_control(for_loop)
            .unwrap_err()
            .starts_with(&back_edge("loop for", "CG00 control consumer"))
    );
    assert_eq!(
        t_control(do_loop).unwrap_err(),
        back_edge("loop do", "CG00 control consumer")
    );
    // G has general loop CFGs.
    assert!(
        gs("void subject(int n) { void *p = q_alloc(); while (n > 0) { n--; } q_free(p); }").safe()
    );
}

#[test]
fn c02_t_lowers_switch_where_the_consumer_admits_it() {
    if !clang_available() {
        return;
    }
    let source = "int subject(int a) { switch (a) { case 1: return 1; default: return 0; } }";
    assert_eq!(t_refusal(source), not_admitted("switch", LEGACY_T));
    // One path per label: `a == 1` taken, and `a == 1` not taken (default).
    assert_eq!(t_control(source).unwrap().paths().len(), 2);
    assert!(gs("void subject(int n) { void *p = q_alloc(); switch (n) { case 1: break; default: break; } q_free(p); }").safe());
}

#[test]
fn c03_t_lowers_forward_jumps_where_the_consumer_admits_them() {
    if !clang_available() {
        return;
    }
    let goto = "int subject(int a) { if (a) goto out; a = 1; out: return a; }";
    assert_eq!(t_refusal(goto), not_admitted("goto", LEGACY_T));
    assert_eq!(t_control(goto).unwrap().paths().len(), 2);
    // A `break` in a loop: the loop is refused first (its back-edge).
    let loop_break = "int subject(int a) { int i; for (i = 0; i < a; i++) { break; } return a; }";
    assert_eq!(
        t_refusal(loop_break),
        format!(
            "{}; not a canonical scan: {}",
            back_edge("loop for", LEGACY_T),
            not_admitted("break", LEGACY_T)
        )
    );
    // A `break` out of a `switch` or `do … while (0)` is acyclic and lowered.
    let switch_break = "int subject(int a) { switch (a) { case 1: break; } return a; }";
    assert_eq!(t_refusal(switch_break), not_admitted("switch", LEGACY_T));
    assert_eq!(t_control(switch_break).unwrap().paths().len(), 2);
    assert_eq!(
        t_control("int subject(int a) { do { if (a) break; a = 1; } while (0); return a; }")
            .unwrap()
            .paths()
            .len(),
        2
    );
    assert!(
        gs("void subject(int n) { void *p = q_alloc(); if (n) goto out; n = 1; out: q_free(p); }")
            .safe()
    );
}
