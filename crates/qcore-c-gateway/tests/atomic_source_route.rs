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

//! Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the atomic source route
//! through routes G and T on real Clang ASTs. The BEFORE controls these
//! replace are CG00's `w01_…`/`w09_…` in `clang_gap_phase3.rs` (flipped there
//! with the card's name).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const STDATOMIC: &str = "#include <stdatomic.h>\n#include <stdint.h>\n";

fn ga(source: &str) -> G {
    g(&format!("{STDATOMIC}{source}"))
}

fn ta(source: &str) -> String {
    t_refusal(&format!("{STDATOMIC}{source}"))
}

const FENCE: &str = "memory order of atomic `__c11_atomic_thread_fence` is not an integer constant the atomic route evaluates (CG19)";
const CONSUME: &str = "memory_order_consume load `__c11_atomic_load` is not represented by the weak-memory model (CG28)";

/// The corpus shapes of `cmt_atomic_clang.c` (lib/cmetrics), whose only
/// blockers were the uncontracted builtin calls: routed, custody-`Safe`.
#[test]
fn g_routes_the_cmetrics_plain_load_and_store() {
    if !clang_atomic_ast_available() {
        return;
    }
    let load = ga(
        "uint64_t subject(uint64_t *storage) { return __atomic_load_n(storage, __ATOMIC_SEQ_CST); }",
    );
    assert!(load.safe(), "{load:?}");
    let store = ga(
        "void subject(uint64_t *storage, uint64_t new_value) { __atomic_store_n(storage, new_value, __ATOMIC_SEQ_CST); }",
    );
    assert!(store.safe(), "{store:?}");
    // The compare-exchange beside them: refused by name here (CG19), routed
    // since card CG28-WEAK-MEMORY-EVENTS (rule 11t) and custody-`Safe`.
    let cas = ga(
        "int subject(uint64_t *storage, uint64_t old_value, uint64_t new_value) { return __atomic_compare_exchange(storage, &old_value, &new_value, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); }",
    );
    assert!(cas.safe(), "{cas:?}");
}

/// `<stdatomic.h>`'s generic functions, lowered by Clang to the `__c11_`
/// builtins, and a loaded value's scalar destination.
#[test]
fn g_routes_stdatomic_plain_forms() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert!(ga("int subject(atomic_int *a) { return atomic_load(a); }").safe());
    assert!(ga("void subject(atomic_int *a, int v) { atomic_store(a, v); }").safe());
    assert!(ga("int subject(atomic_int *a) { int x = atomic_load(a); return x; }").safe());
    assert!(
        ga("int subject(int *p) { int x; x = __atomic_load_n(p, __ATOMIC_ACQUIRE); return x; }")
            .safe()
    );
    // An enumerator order is not evaluated from the function's AST.
    let explicit =
        ga("int subject(atomic_int *a) { return atomic_load_explicit(a, memory_order_acquire); }");
    assert!(
        explicit.refused("memory order of atomic `__c11_atomic_load` is not an integer constant the atomic route evaluates (CG19)"),
        "{explicit:?}"
    );
    // A routed access still carries the dereference obligation: a released
    // object's address is not read through.
    let released = ga(
        "void subject(void) { int *p = q_alloc(); q_free(p); __atomic_store_n(p, 1, __ATOMIC_RELAXED); }",
    );
    assert!(!released.safe(), "{released:?}");
}

/// The refusal controls the card names: `atomic_fetch_add`,
/// `atomic_compare_exchange_strong`, `atomic_thread_fence` and a consume
/// load are refused by name, never read as neutral calls. Flipped by card
/// CG28-WEAK-MEMORY-EVENTS (rule 11t): the read-modify-write and the
/// compare-exchange are routed and custody-`Safe`
/// (`tests/atomic_weak_memory_events.rs`); the fence with an enumerator order
/// is now refused for its unevaluated order; the consume load stays refused.
#[test]
fn g_refuses_rmw_cas_fence_and_consume_by_name() {
    if !clang_atomic_ast_available() {
        return;
    }
    for source in [
        "void subject(atomic_int *a) { atomic_fetch_add(a, 1); }",
        "int subject(atomic_int *a) { int old = atomic_fetch_add(a, 1); return old; }",
        "int subject(atomic_int *a) { return atomic_fetch_add(a, 1); }",
        "int subject(atomic_int *a) { int e = 0; return atomic_compare_exchange_strong(a, &e, 1); }",
    ] {
        let run = ga(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
    let fence = ga("void subject(void) { atomic_thread_fence(memory_order_seq_cst); }");
    assert!(fence.unknown() && fence.refused(FENCE), "{fence:?}");
    let consume =
        ga("int subject(atomic_int *a) { return atomic_load_explicit(a, memory_order_consume); }");
    assert!(consume.unknown() && consume.refused(CONSUME), "{consume:?}");
    let consume_literal =
        ga("int subject(int *p) { return __atomic_load_n(p, __ATOMIC_CONSUME); }");
    assert!(
        consume_literal.refused("memory_order_consume load `__atomic_load_n` is not represented by the weak-memory model (CG28)"),
        "{consume_literal:?}"
    );
    // An order C11 does not allow for the operation.
    let release_load = ga("int subject(int *p) { return __atomic_load_n(p, __ATOMIC_RELEASE); }");
    assert!(
        release_load.refused(
            "memory order of atomic `__atomic_load_n` is not valid for its operation (C11 7.17.7)"
        ),
        "{release_load:?}"
    );
    // A pointer-valued atomic is not custody-neutral data.
    let pointer =
        ga("void *subject(void **slot) { return __atomic_load_n(slot, __ATOMIC_ACQUIRE); }");
    assert!(!pointer.safe(), "{pointer:?}");
}

/// A `volatile` object that is not atomic is not this route's: G's answer is
/// the older route's (CG15 owns volatile), and T keeps its volatile arm.
#[test]
fn volatile_that_is_not_atomic_is_untouched() {
    if !clang_available() {
        return;
    }
    assert!(g("int subject(volatile int *v) { return *v; }").safe());
    assert_eq!(
        t_refusal("int subject(volatile int *p) { return *p; }"),
        "qualified access requires additional synchronization/alias evidence"
    );
    assert_eq!(
        t_refusal("int subject(int * restrict p) { return *p; }"),
        "qualified access requires additional synchronization/alias evidence"
    );
}

/// Route T discriminates `_Atomic` from `volatile` and names every atomic
/// builtin by the route's answer (absorbs
/// CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING).
#[test]
fn t_discriminates_atomic_from_volatile() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(_Atomic int *p) { return *p; }"),
        "atomic-qualified type `int (_Atomic(int) *)` is read by the atomic source route, which no typed-source consumer decides (CG19)"
    );
    // `volatile _Atomic` is atomic: the atomic arm, not the volatile one.
    assert!(
        t_refusal("int subject(volatile _Atomic int *p) { return *p; }")
            .starts_with("atomic-qualified type `")
    );
    assert_eq!(
        t_refusal("int subject(int *p) { return __atomic_load_n(p, 5); }"),
        "atomic `__atomic_load_n` is routed to the weak-memory model, which no typed-source consumer decides (CG19)"
    );
    // Card CG28 (rule 11t): a read-modify-write is now routed, and named so.
    assert_eq!(
        ta("int subject(int *p) { return __atomic_fetch_add(p, 1, 5); }"),
        "atomic `__atomic_fetch_add` is routed to the weak-memory model, which no typed-source consumer decides (CG19)"
    );
}

/// `_Atomic` operator forms in G: a simple assignment through a pointer is
/// the plain store's custody effect (BEFORE: `unsupported assignment
/// expression`); a compound assignment or increment is a read-modify-write,
/// refused by the route's name (BEFORE: the older routes' refusals).
#[test]
fn g_routes_qualified_stores_and_names_qualified_rmw() {
    if !clang_available() {
        return;
    }
    assert!(g("void subject(_Atomic int *a) { *a = 2; }").safe());
    // A formal-valued store keeps the older route's own answer: the classifier
    // does not refuse it, so the hook is not asked, and the flow's out-parameter
    // check refuses it (for `int *` the same store is `Safe`). Registered as
    // CG19-ATOMIC-FORMAL-VALUE-STORE-OUT-PARAMETER.
    let run = g("void subject(_Atomic int *a, int v) { *a = v; }");
    assert!(
        run.unknown() && run.refused("store through `*a` without an out-parameter contract"),
        "{run:?}"
    );
    // Card CG28 (rule 11t): the compound assignment and increment through a
    // pointer are routed read-modify-writes (custody-`Safe`); of a named
    // `_Atomic` object, the operand shape is not read.
    let run = g("void subject(_Atomic int *a) { (*a)++; }");
    assert!(run.safe(), "{run:?}");
    let run = g("void subject(_Atomic int *a) { *a += 2; }");
    assert!(run.safe(), "{run:?}");
    let run = g("_Atomic int counter; void subject(void) { counter++; }");
    assert!(
        run.unknown()
            && run.refused(
                "atomic `_Atomic increment or decrement` has an operand shape the atomic route does not read (CG19)"
            ),
        "{run:?}"
    );
    // A store of a value the route does not read as data stays refused.
    let run = g("void subject(_Atomic int *a) { *a = opaque(); }");
    assert!(run.unknown(), "{run:?}");
}
