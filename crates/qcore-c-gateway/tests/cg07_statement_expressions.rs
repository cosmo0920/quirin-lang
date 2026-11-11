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

//! Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (gateway rule 11g): Clang's
//! selected `_Generic` association and GNU statement expressions on route G.
//! `Safe` is over the synthetic CG00 fixture contracts.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// E17: Clang's selected association
// ---------------------------------------------------------------------------

/// Only the selected association is evaluated: its release happens, an
/// unselected association's release does not.
#[test]
fn only_the_selected_association_is_evaluated() {
    if !clang_available() {
        return;
    }
    // Selected: the release is evaluated, so no leak.
    let run = gs(
        "void subject(int a) { void *p = q_alloc(); int x = _Generic(a, int: r_release_code(p), default: 0); }",
    );
    assert!(run.safe(), "{run:?}");
    // Unselected: the release is not evaluated, so the later free is the
    // only one (no double release).
    let run = gs(
        "void subject(void) { void *p = q_alloc(); int x = _Generic(0, long: r_release_code(p), default: 0); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // Unselected: without the later free the allocation leaks.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); int x = _Generic(0, long: r_release_code(p), default: 0); }",
    );
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    // The controlling expression is never evaluated: its release is no event.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); int x = _Generic(r_release_code(p), int: 1, default: 2); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // The selected association in a return and as a statement.
    let run = gs(
        "int subject(void *p, int a) { return _Generic(a, int: r_borrow_code(p), default: 0); }",
    );
    assert!(run.safe(), "{run:?}");
    let run = gs(
        "void subject(int a) { void *p = q_alloc(); _Generic(a, long: q_observe(p), default: q_free(p)); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// A selection route 1 already lowers keeps route 1's lowering.
#[test]
fn selection_invariant_selections_keep_route_one() {
    if !clang_available() {
        return;
    }
    let run = gs(
        "void subject(void) { void *p = q_alloc(); int x = _Generic(0, int: r_borrow_code(p), default: r_borrow_code(p)); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// E19: statement expressions
// ---------------------------------------------------------------------------

/// The admitted positions: discarded (glibc `assert`'s shape, a comma
/// chain), an initializer, an assignment to a named object, a return.
#[test]
fn statement_expressions_in_admitted_positions() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void *p) { ({ q_observe(p); }); }",
        "void subject(int a) { ((void) sizeof (a), __extension__ ({ if (a) ; else cg_sink(a); })); }",
        "int subject(void *p) { int x = ({ int t = r_borrow_code(p); t + 1; }); return x; }",
        "int subject(void *p) { int x; x = __extension__ ({ int t = r_borrow_code(p); t; }); return x; }",
        "int subject(void *p) { return ({ int t = r_borrow_code(p); t; }); }",
        "void subject(void) { void *p = q_alloc(); int x = ({ q_free(p); 0; }); }",
        "void subject(void) { void *p = ({ void *q = q_alloc(); q; }); q_free(p); }",
        "void subject(int a) { int x = ({ int t = ({ int u = a; u; }); t; }); }",
        "int subject(int a) { int x = ({ int t = a; t; }); int y = ({ int t = x; t; }); return y; }",
        // librdkafka's `RD_KAFKAP_STR_DUPA (&dst, src)`: `*(&NAME) = ({ … })`.
        "int subject(void *p) { int x; (*((&x)) = (__extension__ ({ int t = r_borrow_code(p); t; }))); return x; }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

/// `({ int *q = malloc(4); q; })` with no release is a leak; the inner
/// release is a real release (a second one is a double release).
#[test]
fn inner_custody_composes_into_the_outer_scope() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(void) { void *p = ({ void *q = q_alloc(); q; }); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs("void subject(void) { ({ void *q = q_alloc(); q; }); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run =
        gs("void subject(void) { void *p = q_alloc(); int x = ({ q_free(p); 0; }); q_free(p); }");
    assert!(run.unsafe_() && run.has("DoubleRelease"), "{run:?}");
    // A return from inside the block leaves the function.
    let run = gs(
        "int subject(int a) { void *p = q_alloc(); int x = ({ if (a) { q_free(p); return 1; } 0; }); q_free(p); return x; }",
    );
    assert!(run.safe(), "{run:?}");
}

/// Refusal controls: an escape of an inner object, a shadowing inner name,
/// an operand position, a static inner object.
#[test]
fn statement_expression_refusals() {
    if !clang_available() {
        return;
    }
    // `({ int t; &t; })`: the value would point into storage whose lifetime
    // ends at the block.
    let run = gs("int *subject(void) { int *p = ({ int t = 1; &t; }); return p; }");
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(INITIALIZER), "{run:?}");
    let run = gs("void subject(void) { int *p; p = ({ int a[2]; a; }); }");
    assert!(!run.safe(), "{run:?}");
    // An inner name that shadows an outer object.
    let run = gs(
        "int subject(void) { void *t = q_alloc(); int x = ({ int t = 1; t; }); q_free(t); return x; }",
    );
    assert!(!run.safe(), "{run:?}");
    // Operand positions stay refused.
    for source in [
        "void subject(int a) { int x = a + ({ int t = a; t; }); }",
        "void subject(int a) { cg_sink(({ int t = a; t; })); }",
        "void subject(int a) { if (({ int t = a; t; })) cg_sink(a); }",
        "int subject(int a) { static int s; int x = ({ static int k; k; }); return x; }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
}

/// glibc 2.43's `assert (E)`, the corpus's largest family: the unevaluated
/// `sizeof` operand is no event, the block is its `if`, and the only leaf
/// left is the call to `__assert_fail`, which does not return and which no
/// contract declares (`W43-LIBC-NORETURN`, card CG21).
#[test]
fn glibc_assert_leaves_only_the_assert_fail_call() {
    if !clang_available() {
        return;
    }
    const ASSERT: &str = "extern _Bool __assert_single_arg (_Bool); \
        extern void __assert_fail (const char *a, const char *f, unsigned int l, const char *fn) \
        __attribute__ ((__noreturn__));\n";
    let run = gs(&format!(
        "{ASSERT}void subject(struct S *s) {{ ((void) sizeof (__assert_single_arg (s->a == 1)), \
         __extension__ ({{ if (s->a == 1) ; else __assert_fail (\"s->a == 1\", \"f.c\", 1, \
         __extension__ __PRETTY_FUNCTION__); }})); }}"
    ));
    assert!(run.unknown(), "{run:?}");
    assert_eq!(
        run.unsupported,
        vec!["call to uncontracted function `__assert_fail`".to_owned()],
        "{run:?}"
    );
}

/// Unsafe is never introduced where the source has none, and fail-closed
/// stays: an uncontracted call inside a block keeps its own refusal.
#[test]
fn inner_refusals_keep_their_own_reason() {
    if !clang_available() {
        return;
    }
    let run =
        gs("void opaque_fail(int a); void subject(int a) { ({ if (a) ; else opaque_fail(a); }); }");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused("call to uncontracted function `opaque_fail`")
            || run.refused_prefix("call to uncontracted function"),
        "{run:?}"
    );
}
