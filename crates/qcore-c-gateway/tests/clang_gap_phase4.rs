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
    // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): both forms are named
    // boundaries in their own function (BEFORE: the function was not
    // projected at all, `NOT_PROJECTED`).
    let run =
        gs("void subject(int n) { void *p = q_alloc(); void *t = &&l; goto *t; l: q_free(p); }");
    assert!(
        run.unknown()
            && run.refused("a computed `goto *` has no modelled target set")
            && run.refused(
                "a label's address (`&&label`) is the operand of a computed goto, which is not \
                 modelled"
            ),
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

/// Correction to the input: G refused a `cleanup` local, but through the
/// indirect-access family (IND), not as an attribute. Card
/// CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o) names the boundary, asked first
/// (BEFORE: `IND`).
const CLEANUP: &str = "a `cleanup` attribute's call at every exit of its scope is not modelled";

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
            .refused(CLEANUP)
    );
    assert!(
        gs("void g(int *); void subject(void) { __attribute__((cleanup(g))) int x = 0; }")
            .refused(CLEANUP)
    );
}

/// Correction to the input: G's refusal of inline asm was not an explicit
/// asm rejection; the token reader read the statement as a call to an
/// uncontracted function named after its first keyword (`volatile`,
/// `__asm__`, `goto`). Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o) names
/// the boundary, asked first.
#[test]
fn c12_inline_asm_is_refused_as_a_call() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int a) { __asm__ volatile(\"\" ::: \"memory\"); return a; }"),
        "unmodeled callback statement GCCAsmStmt"
    );
    const ASM: &str =
        "inline assembly is an opaque boundary: its clobbers and memory effects are not modelled";
    const ASM_GOTO: &str = "`asm goto` is an opaque boundary: its clobbers, memory effects and \
                            label edges are not modelled";
    for (statement, named) in [
        ("__asm__ volatile(\"\" ::: \"memory\");", ASM),
        ("__asm__(\"nop\");", ASM),
        ("__asm__ goto(\"\" :::: out);", ASM_GOTO),
    ] {
        let run = gs(&format!(
            "void subject(void) {{ void *p = q_alloc(); {statement} out: q_free(p); }}"
        ));
        assert!(run.refused(named), "{statement}: {run:?}");
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
    // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): an extension node is
    // named as one, never as an uncontracted call (BEFORE: "call to
    // uncontracted function `__builtin_choose_expr`" / `__builtin_bit_cast`);
    // a builtin that Clang parses as a call stays an ordinary call.
    for (source, named) in [
        (
            "void subject(void) { void *p = q_alloc(); int x = __builtin_choose_expr(1, 1, 0); q_free(p); }",
            "`__builtin_choose_expr` is the Clang extension node ChooseExpr, which no resource \
             contract or route models",
        ),
        (
            "int subject(float f) { return __builtin_bit_cast(int, f); }",
            "`__builtin_bit_cast` is the Clang extension node BuiltinBitCastExpr, which no \
             resource contract or route models",
        ),
    ] {
        assert!(gs(source).refused(named), "{source}: {:?}", gs(source));
    }
    assert!(
        gs("int subject(int a) { return __builtin_expect(a, 0); }")
            .refused(&uncontracted("__builtin_expect"))
    );
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
    // The schema's extent is exactly Exact / Interior / Borrowed (the last
    // added deliberately by card SLOT-MOVE-AND-INTERIOR-BORROW, gateway rule
    // 12y: a result borrowed from its parent; it carries no offset or length
    // either).
    let extents = |extent: DerivedExtent| match extent {
        DerivedExtent::Exact | DerivedExtent::Interior | DerivedExtent::Borrowed => (),
    };
    extents(DerivedExtent::Exact);
    // The fifth mode preserves an existing removed item; like the other
    // modes, it adds no offset or length to the derived extent vocabulary.
    let modes = |mode: ResourceReturnMode| match mode {
        ResourceReturnMode::None
        | ResourceReturnMode::Owned { .. }
        | ResourceReturnMode::Reallocated { .. }
        | ResourceReturnMode::Derived { .. }
        | ResourceReturnMode::Removed { .. } => (),
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
/// generic `handle` stays outside resource v1. Card REFCOUNT-RESOURCE-KIND
/// (rule 13b) adds the reference kind, taken and dropped only by its own
/// clauses and spelled only on an owned result (deliberate update: six kinds
/// to seven).
#[test]
fn k07_resource_kinds_are_exactly_seven() {
    assert_eq!(
        ResourceKind::ALL,
        [
            ResourceKind::Heap,
            ResourceKind::Descriptor,
            ResourceKind::Stream,
            ResourceKind::Directory,
            ResourceKind::DynamicLibrary,
            ResourceKind::Regex,
            ResourceKind::Reference,
        ]
    );
    let error = load("    ensures normal resource_released(p, nonnull);\n    ensures normal resource_borrowed(n);\n    ensures normal resource_kind(p, reference);\n    ensures normal resource_return_none();\n")
        .expect_err("the reference kind is not spelled on a parameter");
    assert!(error.contains("resource_reference_kept"), "{error}");
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

/// Unchanged by CG26-VARIADICS (rule 11w): the data tail still refuses an
/// identity-carrying trailing actual; only a declaration carrying
/// `resource_variadic_borrowed_tail()` reads it as a borrow
/// (`tests/cg26_variadics.rs`). Route T still refuses a variadic signature.
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
    assert!(run.refused("arguments beyond the declared parameters of `cg_log` are outside the resource projection"), "{run:?}");
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
    // **Flipped again by VA-LOOP-AND-POINTER-ARG (rule 12c)**: the pointer
    // value is a never-owned opaque binding (no named refusal); `q_free(p)`
    // is still an unresolved alias, so the function is never `Safe`.
    for builtin in ["__builtin_va_start", "__builtin_va_arg", "__builtin_va_end"] {
        assert!(!run.refused(&uncontracted(builtin)), "{builtin}: {run:?}");
    }
    assert!(
        !run.refused_prefix("va_list operation refused: the `va_arg` value stored into `p`"),
        "{run:?}"
    );
    assert!(run.has("UnknownAlias { variable: \"p\" }"), "{run:?}");
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

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y) added the explicit C17, C23 and
/// C2y identities. C23 spellings are still not C in an older dialect (Clang
/// refuses the unit); under C23 they are registered differences refused by
/// name (`tests/cg30_dialects_flags_types.rs`).
#[test]
fn i01_the_dialect_set_is_seven_explicit_identities() {
    let dialects = |dialect: CCompilerDialect| match dialect {
        CCompilerDialect::C11
        | CCompilerDialect::Gnu99
        | CCompilerDialect::Gnu11
        | CCompilerDialect::Gnu17
        | CCompilerDialect::C17
        | CCompilerDialect::C23
        | CCompilerDialect::C2y => (),
    };
    dialects(CCompilerDialect::default());
    if !clang_available() {
        return;
    }
    for dialect in [
        CCompilerDialect::C11,
        CCompilerDialect::Gnu17,
        CCompilerDialect::C17,
    ] {
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
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): T refuses by name first; G
    // keeps its statement refusal and adds the named one.
    assert_eq!(
        t_refusal("int subject(void) { double _Complex z = 0; return 0; }"),
        "C extension type `_Complex double` (complex) is not adopted (CG30)"
    );
    let run = gs("void subject(void) { double _Complex z = 1.0; }");
    assert!(run.refused(ASSIGNMENT));
    assert!(run.refused("C extension type `_Complex double` (complex) is not adopted (CG30)"));
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

/// Renamed by card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): refused by
/// the atomic route's name rather than as an uncontracted call. Flipped by
/// card CG28-WEAK-MEMORY-EVENTS (rule 11t): each is now a routed event of the
/// weak-memory model, lowered to its custody effect, and route G answers
/// custody-`Safe` (no ordering claim).
#[test]
fn w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls() {
    if !clang_atomic_ast_available() {
        return;
    }
    for source in [
        "int subject(_Atomic int *a) { return __c11_atomic_fetch_add(a, 1, 5); }",
        "int subject(_Atomic int *a, int *e) { return __c11_atomic_compare_exchange_strong(a, e, 1, 5, 5); }",
        "void subject(void) { __c11_atomic_thread_fence(5); }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

/// Correction to the input: G's refusal of `_BitInt` is the indirect-access
/// family (IND), not a scalar-type classification.
#[test]
fn n14_bit_precise_integers_are_refused() {
    if !clang_available() {
        return;
    }
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): refused by name on both
    // routes; G keeps its IND refusal beside the named one.
    assert_eq!(
        t_refusal("int subject(void) { _BitInt(7) x = 0; return 0; }"),
        "C extension type `_BitInt(7)` (bit-precise integer) is not adopted (CG30)"
    );
    let run = gs("void subject(void) { _BitInt(7) x = 3; }");
    assert!(run.refused(IND));
    assert!(
        run.refused("C extension type `_BitInt(7)` (bit-precise integer) is not adopted (CG30)")
    );
}
