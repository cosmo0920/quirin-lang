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

//! Card CAST-FORMALIZATION (gateway rule 12h): one conversion table, by
//! class, consulted from every route-G position. One control per class and
//! position, and the refusals the table keeps: an access of an incompatible
//! effective type, an integer made a pointer where custody needs an
//! identity, a function/object conversion. `Safe` here is over the synthetic
//! trusted fixture contracts of the CG00 harness and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct T { int x; char *name; }; struct A { long y; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// H-V: access positions read a view by the table's access rule.
// ---------------------------------------------------------------------------

/// A typed view of character storage (C11 6.3.2.3p7, `P-EFFECTIVE-TYPE`), a
/// byte view of an object (6.5p7) and a qualifier conversion (the same type)
/// are read over an untracked root at a member root, a dereference and a
/// subscript base. BEFORE: CG12 read only a `void *` operand, so each was
/// refused.
#[test]
fn views_the_table_admits_are_read_at_access_positions() {
    if !clang_available() {
        return;
    }
    for body in [
        // typed-view, member root, from a character pointer.
        "void subject(char *buf) { int x = ((struct T *) buf)->x; cg_sink(x); }",
        "void subject(unsigned char *buf) { int x = ((struct T *) buf)->x; cg_sink(x); }",
        // typed-view, dereference, from `void *` (CG12's own row).
        "void subject(void *v) { int x = *(int *) v; cg_sink(x); }",
        // byte-view, subscript base.
        "void subject(struct T *t) { char c = ((char *) t)[0]; cg_sink(c); }",
        // qualifier, dereference (CG12 refused `*(int *)p` of an `int *p`).
        "void subject(int *p) { int x = *(const int *) p; cg_sink(x); }",
        "void subject(int *p) { int x = *(int *) p; cg_sink(x); }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

/// A punned view (an incompatible effective type), a view of a tracked
/// identity, a view of a function pointer and an added `volatile` stay
/// refused: the table never certifies them.
#[test]
fn views_the_table_refuses_stay_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(long *p) { int x = *(int *) p; cg_sink(x); }",
        "void subject(struct A *a) { int x = ((struct T *) a)->x; cg_sink(x); }",
        "void subject(void) { char *b = q_alloc(); int x = ((struct T *) b)->x; cg_sink(x); \
         q_free(b); }",
        "void subject(int *p) { int x = *(volatile int *) p; cg_sink(x); }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// H-U: value positions keep the operand's identity.
// ---------------------------------------------------------------------------

/// A byte view at a call actual and at a pointer step, a typed view at an
/// initializer, and a punned view at an assignment value denote the operand's
/// object: the statement is its uncast statement, so a release through the
/// cast releases the operand and a step over it derives from it.
#[test]
fn identity_preserving_casts_at_value_positions_are_their_operands() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { struct T *t = q_alloc(); q_free((void *) t); }",
        "void subject(void) { struct T *t = q_alloc(); q_free((char *) t); }",
        "void subject(struct T *t, int n) { char *p; p = (char *) t + n; cg_sink(p != 0); }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
    // The identity is the operand's: a leak and a use after release through
    // the cast are found. The soundness repair H-U0: BEFORE (main
    // `cb3085365`) the lexical classifier read the borrowed actual
    // `(void *) t` as no place, and the use after release below was `Safe`.
    let run = gs("void subject(void) { struct T *t = q_alloc(); q_observe((void *) t); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs(
        "void subject(void) { struct T *t = q_alloc(); q_free((void *) t); q_observe((void *) t); }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// An integer made a pointer carries no identity, a tracked pointer made an
/// integer escapes, and a function pointer never becomes an object pointer:
/// each keeps a refusal where custody needs an identity.
#[test]
fn conversions_without_an_identity_stay_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(unsigned long v) { void *p = (void *) v; q_free(p); }",
        "void subject(void) { void *p = q_alloc(); unsigned long v = (unsigned long) p; \
         cg_sink((int) v); q_free(p); }",
        "void callee(void); void subject(void) { void *p = (void *) callee; q_free(p); }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

/// `(void) e` evaluates `e` for its effect only.
#[test]
fn a_discarded_value_is_its_operand_effect_only() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(void) { void *p = q_alloc(); (void) p; q_free(p); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { (void) q_alloc(); }");
    assert!(!run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// H-U in a declared variadic tail (`CG26-TAIL-RESIDUAL-ACTUALS`' cast leaves).
// ---------------------------------------------------------------------------

const BLOG: &str = "module contract;\n\
extern fn cg_blog(format: ptr<const, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_borrowed(format);\n\
    ensures normal resource_return_none();\n\
    ensures normal resource_variadic_borrowed_tail();\n\
};\n";

fn with_blog() -> ResourceContracts {
    let mut sources = BTreeMap::new();
    sources.insert("cg_blog.qcore".to_owned(), BLOG.to_owned());
    for (name, text) in clang_gap::contract_sources() {
        sources.insert(name, text);
    }
    load_resource_contract_sources(&sources).expect("fixture contracts load")
}

fn blog(source: &str) -> LegacyFunctionSummary {
    let text = format!(
        "{}\n{S}void cg_blog(const char *format, ...);\n{source}\n",
        clang_gap::DECLARATIONS
    );
    let contracts = with_blog();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            certificate.summary.functions["subject"].clone()
        }
        UnrestrictedLegacyResult::Unsafe(summary) => summary.functions["subject"].clone(),
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary.functions["subject"].clone(),
        other => panic!("no summary: {other:?}"),
    }
}

/// A byte view of a live identity and a scalar conversion of a scalar in a
/// borrowed tail are the operand's borrowed read; the identity's obligations
/// stay (a leak is still found). A descriptor's scalar conversion keeps the
/// refusal: the integer carries an identity.
#[test]
fn casts_in_a_borrowed_tail_are_their_operands() {
    if !clang_available() {
        return;
    }
    let run = blog(
        "void subject(void) { struct T *t = q_alloc(); int n = 3; \
         cg_blog(\"%p %ld\", (void *) t, (long) n); q_free(t); }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{:?}", run.diagnostics);
    let leaking =
        blog("void subject(void) { struct T *t = q_alloc(); cg_blog(\"%p\", (void *) t); }");
    assert!(
        leaking
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{:?}",
        leaking.diagnostics
    );
    let descriptor = blog(
        "void subject(void) { int fd = k_open(\"x\"); cg_blog(\"%ld\", (long) fd); k_close(fd); }",
    );
    assert_ne!(
        descriptor.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptor.diagnostics
    );
}
