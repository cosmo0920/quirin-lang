//! Card CG00-CLANG-GAP-BASELINE, phase 4 (cards CG20–CG30): executed BEFORE
//! controls for the Clang gap analysis's numeric, control, call, vocabulary
//! and dialect entries. Conventions as in `clang_gap_phase1.rs`; the
//! compile-flag halves of I01, I02 and I06 are in
//! `crates/qcore-audit/tests/clang_gap_flags.rs`, the core halves (N03–N06,
//! W02–W07, W10) in `crates/qcore-core/tests/clang_gap_core.rs`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect, analyze_unrestricted_c11};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{
    DerivedExtent, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

fn uncontracted(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

// ---------------------------------------------------------------------------
// CG20 — numeric domain (N01–N07, N08 transfer, C04, C05)
// ---------------------------------------------------------------------------

#[test]
fn n01_n02_n03_t_keeps_conversions_as_named_casts_only() {
    if !clang_available() {
        return;
    }
    // Widening and narrowing integer conversions are admitted as an
    // `IntegralCast` node: no width, signedness or loss is attached.
    for source in [
        "unsigned long subject(unsigned int a) { return a; }",
        "unsigned int subject(long a) { return (unsigned int)a; }",
        "int subject(unsigned int a) { return (int)a; }",
    ] {
        let summary = t(source).unwrap_or_else(|reason| panic!("{source}: {reason}"));
        let rendered = format!("{:?}", summary.paths());
        assert!(
            rendered.contains("cast: \"IntegralCast\""),
            "{source}: {rendered}"
        );
    }
}

#[test]
fn n04_n05_n06_n07_g_proves_no_arithmetic_property() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(int a) { return a + 2147483647; }",
        "int subject(int a, int b) { return a / b; }",
        "int subject(int a, int b) { return a % b; }",
        "int subject(int a) { return a << 40; }",
        "int subject(int a, unsigned b) { return a < b; }",
    ] {
        assert!(gs(source).safe(), "{source}");
    }
}

#[test]
fn n08_floating_transfer_is_not_modelled() {
    if !clang_available() {
        return;
    }
    assert!(gs("int subject(double d) { return (int)d; }").safe());
    assert!(gs("float subject(double d) { return (float)d; }").safe());
}

/// The route's precision limit publishes a provisional `Unsafe`, not an
/// `Unknown`: every case edge is kept whatever the switch value.
#[test]
fn c04_switch_values_do_not_prune_cases() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(void) { void *p = q_alloc(); int k = 1; \
                  switch (k) { case 1: q_free(p); break; case 2: break; } }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
}

#[test]
fn c05_branch_correlation_is_not_tracked() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void subject(int flag) { void *p = q_alloc(); if (flag) q_free(p); if (!flag) q_free(p); }",
    );
    assert!(
        run.unsafe_() && run.has("Leak") && run.has("DoubleRelease"),
        "{run:?}"
    );
}

// ---------------------------------------------------------------------------
// CG21 — call exits and boundaries (C09–C13, E20, I06)
// ---------------------------------------------------------------------------

#[test]
fn c09_computed_goto_is_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { void *t = &&l; goto *t; l: return a; }"),
        "unmodeled callback expression AddrLabelExpr"
    );
    let run =
        gs("void subject(int n) { void *p = q_alloc(); void *t = &&l; goto *t; l: q_free(p); }");
    assert!(
        run.unknown() && run.diagnostics.first().map(String::as_str) == Some(NOT_PROJECTED),
        "{run:?}"
    );
}

#[test]
fn c10_setjmp_has_no_return_twice_edge() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "#include <setjmp.h>\nvoid subject(jmp_buf b) { void *p = q_alloc(); \
                  if (setjmp(b)) { return; } q_free(p); }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
    assert!(!run.safe());
}

/// Correction to the input: G refuses a `cleanup` local, but through the
/// indirect-access family (IND), not as an attribute.
#[test]
fn c11_cleanup_attributes_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal(
            "void g(int *); int subject(void) { __attribute__((cleanup(g))) int x = 0; return x; }"
        ),
        "unmodeled local initialization/attributes"
    );
    assert!(
        gs("void subject(void) { __attribute__((cleanup(q_free))) void *p = q_alloc(); }")
            .refused(IND)
    );
    assert!(
        gs("void g(int *); void subject(void) { __attribute__((cleanup(g))) int x = 0; }")
            .refused(IND)
    );
}

/// Correction to the input: G's refusal of inline asm is not an explicit
/// asm rejection; the token reader reads the statement as a call to an
/// uncontracted function named after its first keyword.
#[test]
fn c12_inline_asm_is_refused_as_a_call() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { __asm__ volatile(\"\" ::: \"memory\"); return a; }"),
        "unmodeled callback statement GCCAsmStmt"
    );
    for (statement, word) in [
        ("__asm__ volatile(\"\" ::: \"memory\");", "volatile"),
        ("__asm__(\"nop\");", "__asm__"),
        ("__asm__ goto(\"\" :::: out);", "goto"),
    ] {
        let run = gs(&format!(
            "void subject(void) {{ void *p = q_alloc(); {statement} out: q_free(p); }}"
        ));
        assert!(run.refused(&uncontracted(word)), "{statement}: {run:?}");
    }
}

#[test]
fn c13_noreturn_is_read_only_as_a_terminal_statement() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal(
            "__attribute__((noreturn)) int die(void); int subject(int a) { int x = die(); return x; }"
        ),
        "nonreturning call requires a terminal statement"
    );
    assert!(t("__attribute__((noreturn)) void die(void); int subject(int a) { if (a) die(); return 0; }").is_ok());
    // G has no noreturn outcome for a call without a contract, in any position.
    let run = gs(
        "__attribute__((noreturn)) int die(void); void subject(int a) { void *p = q_alloc(); int x = a ? die() : 0; q_free(p); }",
    );
    assert!(run.refused(&uncontracted("die")), "{run:?}");
}

#[test]
fn e20_builtins_are_ordinary_uncontracted_calls() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { return __builtin_choose_expr(1, a, 0); }"),
        "unmodeled callback expression ChooseExpr"
    );
    for (source, callee) in [
        (
            "void subject(void) { void *p = q_alloc(); int x = __builtin_choose_expr(1, 1, 0); q_free(p); }",
            "__builtin_choose_expr",
        ),
        (
            "int subject(float f) { return __builtin_bit_cast(int, f); }",
            "__builtin_bit_cast",
        ),
        (
            "int subject(int a) { return __builtin_expect(a, 0); }",
            "__builtin_expect",
        ),
    ] {
        assert!(gs(source).refused(&uncontracted(callee)), "{source}");
    }
}

/// Clarification of the input: without `-fopenmp` (outside the frontend-flag
/// allowlist, see `crates/qcore-audit/tests/clang_gap_flags.rs`) an OpenMP
/// pragma is not an AST node at all, so the audited build's body is the plain
/// block and is read as such.
#[test]
fn i06_extension_pragmas_are_inert_in_the_audited_build() {
    if !clang_available() {
        return;
    }
    assert!(gs("void subject(void) { void *p = q_alloc(); \n#pragma omp parallel\n { q_observe(p); } q_free(p); }").safe());
}

// ---------------------------------------------------------------------------
// CG22 — indirect calls and callbacks (K01 K02)
// ---------------------------------------------------------------------------

#[test]
fn k01_indirect_callees_are_not_resolved() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int f(int); int subject(int a) { return (*f)(a); }"),
        "indirect or unmodeled call"
    );
    assert!(
        gs("void subject(void (*fp)(void *)) { void *p = q_alloc(); fp(p); q_free(p); }")
            .refused(&uncontracted("fp"))
    );
    assert!(
        gs("void subject(struct S *s) { void *p = q_alloc(); s->cb(); q_free(p); }").refused(IND)
    );
}

#[test]
fn k02_callback_registration_has_no_capture_relation() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void reg(void (*cb)(void *), void *arg); void cb(void *a); \
                  void subject(void) { void *p = q_alloc(); reg(cb, p); }",
    );
    assert!(run.refused(&uncontracted("reg")), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG23 / CG24 / CG25 — contract vocabulary (K04 K05 K06 K07 K11)
// ---------------------------------------------------------------------------

fn load(clauses: &str) -> Result<(), String> {
    let text = format!(
        "module contract;\n\nextern fn cg_probe(p: ptr<mut, u8>, n: usize) -> ptr<mut, u8>\ncontract {{\n    requires resource_contract_v1;\n{clauses}}};\n"
    );
    load_resource_contract_sources(&BTreeMap::from([("probe.qcore".to_owned(), text)]))
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn k_vocabulary_control_loads() {
    load("    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_interior(p, nullable);\n")
        .expect("the control declaration loads");
}

#[test]
fn k04_no_read_write_frame_clause_exists() {
    let error = load("    ensures normal resource_reads(p);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_none();\n")
        .expect_err("a frame clause is outside resource v1");
    assert!(!error.is_empty());
}

#[test]
fn k05_no_relational_postcondition_exists() {
    let error = load("    ensures normal resource_released_if(p, result == 0);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_none();\n")
        .expect_err("a return-code relation is outside resource v1");
    assert!(!error.is_empty());
}

#[test]
fn k06_derived_results_have_no_offset_or_length() {
    let error = load("    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_interior(p, n, nullable);\n")
        .expect_err("an offset-carrying derived result is outside resource v1");
    assert!(!error.is_empty());
    // The schema's extent is exactly Exact / Interior.
    let extents = |extent: DerivedExtent| match extent {
        DerivedExtent::Exact | DerivedExtent::Interior => (),
    };
    extents(DerivedExtent::Exact);
    // And the result modes are exactly these four.
    let modes = |mode: ResourceReturnMode| match mode {
        ResourceReturnMode::None
        | ResourceReturnMode::Owned { .. }
        | ResourceReturnMode::Reallocated { .. }
        | ResourceReturnMode::Derived { .. } => (),
    };
    modes(ResourceReturnMode::None);
    let parameters = |mode: ResourceParameterMode| match mode {
        ResourceParameterMode::Borrowed
        | ResourceParameterMode::Owned { .. }
        | ResourceParameterMode::Consumed { .. }
        | ResourceParameterMode::Released { .. }
        | ResourceParameterMode::Out { .. }
        | ResourceParameterMode::Stored { .. }
        | ResourceParameterMode::Removed { .. } => (),
    };
    parameters(ResourceParameterMode::Borrowed);
}

/// CG00's BEFORE control for K07 held at `b6a3dc42` (four kinds). Card
/// CG25-RESOURCE-KINDS (rule 11v) adds the `dynamic_library` and in-place
/// `regex` kinds, each with a declared use; an undeclared spelling such as a
/// generic `handle` stays outside resource v1.
#[test]
fn k07_resource_kinds_are_exactly_six() {
    assert_eq!(
        ResourceKind::ALL,
        [
            ResourceKind::Heap,
            ResourceKind::Descriptor,
            ResourceKind::Stream,
            ResourceKind::Directory,
            ResourceKind::DynamicLibrary,
            ResourceKind::Regex,
        ]
    );
    let error = load("    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_owned(nonnull);\n    ensures normal resource_return_kind(handle);\n")
        .expect_err("an undeclared kind is outside resource v1");
    assert!(!error.is_empty());
}

#[test]
fn k11_an_exit_specific_disposition_has_no_summary() {
    if !clang_available() {
        return;
    }
    // Control: a helper that releases on every exit summarises.
    assert!(gs("static void rel(void *p) { q_free(p); } void subject(void) { void *p = q_alloc(); rel(p); }").safe());
    // Released on one exit only: no summary, so the call is uncontracted.
    let run = gs(
        "static void rel(void *p, int c) { if (c) q_free(p); } void subject(void) { void *p = q_alloc(); rel(p, 1); }",
    );
    assert!(run.refused(&uncontracted("rel")), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG26 — variadics (K08 K09 K10)
// ---------------------------------------------------------------------------

/// **Flipped by CG26-VARIADICS (rule 11w)** for route G: a statement's
/// trailing actual under the declared data tail is read as a declared
/// `Borrowed` one (T8), so a live identity there is a borrow and the call is
/// admitted. Route T still refuses a variadic signature.
#[test]
fn k08_variadic_tails_are_data_only() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a, ...) { return a; }"),
        "function signature is outside the selected Unix profile"
    );
    assert!(gs("void subject(void) { void *p = q_alloc(); cg_log(\"%d\", 5); q_free(p); }").safe());
    let run = gs("void subject(void) { void *p = q_alloc(); cg_log(\"%p\", p); q_free(p); }");
    assert!(!run.refused("arguments beyond the declared parameters of `cg_log` are outside the resource projection"), "{run:?}");
    assert!(run.safe(), "{run:?}");
}

#[test]
fn k09_va_list_operations_are_not_modelled() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal(
            "#include <stdarg.h>\nint subject(int a, va_list ap) { return va_arg(ap, int); }"
        ),
        "unmodeled callback expression VAArgExpr"
    );
    let run = gs(
        "#include <stdarg.h>\nvoid subject(int n, ...) { va_list ap; va_start(ap, n); \
                  void *p = va_arg(ap, void *); va_end(ap); q_free(p); }",
    );
    // **Flipped by CG26-VARIADICS (rule 11w)** for route G: the protocol is
    // established, so `va_start`/`va_end` are custody-neutral; a pointer
    // `va_arg` keeps a named refusal and `q_free(p)` stays unresolved.
    for builtin in ["__builtin_va_start", "__builtin_va_arg", "__builtin_va_end"] {
        assert!(!run.refused(&uncontracted(builtin)), "{builtin}: {run:?}");
    }
    assert!(
        run.refused_prefix("va_list operation refused: the `va_arg` value stored into `p`"),
        "{run:?}"
    );
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn k10_format_agreement_is_not_checked() {
    if !clang_available() {
        return;
    }
    assert!(gs("void subject(void) { void *p = q_alloc(); cg_log(\"%s\", 5); q_free(p); }").safe());
}

// ---------------------------------------------------------------------------
// CG27 — loops, recursion and budgets (C06 C07 C08)
// ---------------------------------------------------------------------------

#[test]
fn c06_collection_loops_have_no_invariant() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void subject(int n) { void *a[4]; int i; for (i = 0; i < 4; i++) a[i] = q_alloc(); \
                  for (i = 0; i < 4; i++) q_free(a[i]); }",
    );
    assert!(
        run.refused("discarded owned result of resource API `q_alloc`"),
        "{run:?}"
    );
    assert!(run.refused(IND));
    // Control: loops over one scalar-indexed body are read.
    assert!(gs("void subject(int n) { int i; for (i = 0; i < n; i++) { void *p = q_alloc(); q_free(p); } }").safe());
}

/// Partial correction to the input: neutral recursion with two recursive
/// calls (and mutual recursion) is admitted; recursion that passes a tracked
/// resource has no summary.
#[test]
fn c07_recursion_passing_a_resource_has_no_summary() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void subject(void *p, int n) { if (n == 0) { q_free(p); return; } subject(p, n - 1); }",
    );
    assert!(run.refused(&uncontracted("subject")), "{run:?}");
    assert!(gs("void subject(int n) { if (n <= 0) return; void *p = q_alloc(); subject(n - 1); subject(n - 2); q_free(p); }").safe());
    assert!(gs("void other(int n); void subject(int n) { void *p = q_alloc(); if (n > 0) other(n - 1); q_free(p); } \
                void other(int n) { if (n > 0) subject(n - 1); }").safe());
}

#[test]
fn c08_budgets_are_reported_never_safe() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal(
            "int f(int); int subject(int a) { if (a) f(1); if (a) f(2); if (a) f(3); if (a) f(4); \
                   if (a) f(5); if (a) f(6); if (a) f(7); return 0; }"
        ),
        "callback path limit exceeded"
    );
    let run = gs(
        "void subject(int c0, int c1, int c2, int c3, int c4, int c5, int c6, int c7, int c8) { \
                  void *x = 0; void *p0 = q_alloc(); void *p1 = q_alloc(); void *p2 = q_alloc(); void *p3 = q_alloc(); \
                  void *p4 = q_alloc(); void *p5 = q_alloc(); void *p6 = q_alloc(); void *p7 = q_alloc(); void *p8 = q_alloc(); \
                  if (c0) x = p0; if (c1) x = p1; if (c2) x = p2; if (c3) x = p3; if (c4) x = p4; if (c5) x = p5; \
                  if (c6) x = p6; if (c7) x = p7; if (c8) x = p8; q_observe(x); \
                  q_free(p0); q_free(p1); q_free(p2); q_free(p3); q_free(p4); q_free(p5); q_free(p6); q_free(p7); q_free(p8); }",
    );
    assert!(
        run.refused("alias set exceeded 8 targets and was widened"),
        "{run:?}"
    );
    assert!(!run.safe());
}

// ---------------------------------------------------------------------------
// CG30 — dialects, flags and types (I01 I02 I03 I05 N12 N13 N14)
// ---------------------------------------------------------------------------

#[test]
fn i01_the_dialect_set_is_c11_and_three_gnu_modes() {
    let dialects = |dialect: CCompilerDialect| match dialect {
        CCompilerDialect::C11
        | CCompilerDialect::Gnu99
        | CCompilerDialect::Gnu11
        | CCompilerDialect::Gnu17 => (),
    };
    dialects(CCompilerDialect::default());
    if !clang_available() {
        return;
    }
    // C23 spellings are not C in any admitted dialect: Clang refuses the unit.
    for dialect in [CCompilerDialect::C11, CCompilerDialect::Gnu17] {
        let source = "int subject(void *p) { return p == nullptr; }";
        let outcome = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                dialect,
                ..Default::default()
            },
            "subject",
            &contracts(),
        );
        assert!(outcome.is_err(), "{dialect:?}");
    }
}

#[test]
fn i03_type_profiles_do_not_canonicalise_typedefs() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("typedef int myint; myint subject(myint a) { return a; }"),
        "result typedef outside unsigned scalar profile"
    );
}

/// Partial correction to the input: T admits GNU `__typeof__` over a profile
/// type; G refuses it (IND). `nullptr` and `true` are refused by Clang itself
/// under every admitted dialect (see I01).
#[test]
fn i05_typeof_is_admitted_by_t_and_refused_by_g() {
    if !clang_available() {
        return;
    }
    assert!(t("int subject(int a) { __typeof__(a) b = a; return b; }").is_ok());
    assert!(gs("int subject(int a) { __typeof__(a) b = a; return b; }").refused(IND));
}

#[test]
fn n12_complex_values_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(void) { double _Complex z = 0; return 0; }"),
        "local outside scalar/pointer callback profile"
    );
    assert!(gs("void subject(void) { double _Complex z = 1.0; }").refused(ASSIGNMENT));
}

#[test]
fn n13_vector_values_are_refused() {
    if !clang_available() {
        return;
    }
    assert!(gs("typedef int v4 __attribute__((vector_size(16))); void subject(v4 a, v4 b) { v4 c = a + b; }")
        .refused(ASSIGNMENT));
}

// ---------------------------------------------------------------------------
// CG28 — weak-memory events (W02 W03 W04; the core halves are in qcore-core)
// ---------------------------------------------------------------------------

/// Renamed by card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): still
/// refused (`Unknown`), now by the atomic route's name rather than as an
/// uncontracted call. CG28 owns representing them.
#[test]
fn w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls() {
    if !clang_available() {
        return;
    }
    for (source, refusal) in [
        (
            "int subject(_Atomic int *a) { return __c11_atomic_fetch_add(a, 1, 5); }",
            "atomic read-modify-write `__c11_atomic_fetch_add` is not represented by the weak-memory model (CG28)",
        ),
        (
            "int subject(_Atomic int *a, int *e) { return __c11_atomic_compare_exchange_strong(a, e, 1, 5, 5); }",
            "atomic compare-exchange `__c11_atomic_compare_exchange_strong` is not represented by the weak-memory model (CG28)",
        ),
        (
            "void subject(void) { __c11_atomic_thread_fence(5); }",
            "atomic fence `__c11_atomic_thread_fence` is not represented by the weak-memory model (CG28)",
        ),
    ] {
        let run = gs(source);
        assert!(run.unknown() && run.refused(refusal), "{source}: {run:?}");
    }
}

/// Correction to the input: G's refusal of `_BitInt` is the indirect-access
/// family (IND), not a scalar-type classification.
#[test]
fn n14_bit_precise_integers_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(void) { _BitInt(7) x = 0; return 0; }"),
        "local outside scalar/pointer callback profile"
    );
    assert!(gs("void subject(void) { _BitInt(7) x = 3; }").refused(IND));
}
