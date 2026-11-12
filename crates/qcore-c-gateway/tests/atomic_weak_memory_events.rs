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

//! Card CG28-WEAK-MEMORY-EVENTS (gateway rule 11t): read-modify-writes,
//! compare-exchange and fences routed from C source to the core weak-memory
//! model, through routes G and T on real Clang ASTs. The BEFORE controls these
//! replace are CG00's `w02_w03_w04_…` in `clang_gap_phase4.rs` and CG19's
//! `g_refuses_rmw_cas_fence_and_consume_by_name` (both flipped there with the
//! card's name). Route G's `Safe` here is custody under the declared
//! contracts; it carries no ordering, race-freedom or liveness claim (the
//! route's single-graph assumption).

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

fn assert_safe(source: &str) {
    let run = ga(source);
    assert!(run.safe(), "{source}: {run:?}");
}

fn assert_refused(source: &str, refusal: &str) {
    let run = ga(source);
    assert!(run.unknown() && run.refused(refusal), "{source}: {run:?}");
}

/// The corpus shape of `cmt_atomic_gcc.c` (lib/cmetrics): the generic
/// compare-exchange of `storage` with the addresses of two scalar formals,
/// its result returned. BEFORE (CG19): refused by name as an unrepresented
/// compare-exchange.
#[test]
fn g_routes_the_cmetrics_compare_exchange() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert_safe(
        "int subject(uint64_t *storage, uint64_t old_value, uint64_t new_value) { return __atomic_compare_exchange(storage, &old_value, &new_value, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); }",
    );
}

/// Read-modify-writes: a statement, a scalar destination, a `return`, every
/// order but consume, and the `_Atomic` compound assignment and increment
/// through a pointer (BEFORE: each refused by name).
#[test]
fn g_routes_read_modify_writes() {
    if !clang_atomic_ast_available() {
        return;
    }
    for source in [
        "void subject(atomic_int *a) { atomic_fetch_add(a, 1); }",
        "int subject(atomic_int *a) { int old = atomic_fetch_add(a, 1); return old; }",
        "int subject(atomic_int *a) { return atomic_fetch_add(a, 1); }",
        "int subject(atomic_int *a) { return atomic_exchange(a, 3); }",
        "int subject(int *p) { return __atomic_fetch_or(p, 4, __ATOMIC_RELAXED); }",
        "int subject(int *p) { return __atomic_add_fetch(p, 1, __ATOMIC_ACQ_REL); }",
        "int subject(int *p) { return __atomic_exchange_n(p, 5, __ATOMIC_RELEASE); }",
        "void subject(_Atomic int *a) { (*a)++; }",
        "void subject(_Atomic int *a) { *a += 2; }",
        "void subject(_Atomic int *a) { --(*a); }",
    ] {
        assert_safe(source);
    }
}

/// Compare-exchange: strong and weak, by `&local`, `&formal` and a pointer
/// formal as `expected`, and the weak form's retry loop.
#[test]
fn g_routes_compare_exchange_and_the_weak_loop() {
    if !clang_atomic_ast_available() {
        return;
    }
    for source in [
        "int subject(atomic_int *a) { int e = 0; return atomic_compare_exchange_strong(a, &e, 1); }",
        "int subject(atomic_int *a, int *e) { return atomic_compare_exchange_weak(a, e, 1); }",
        "int subject(int *p, int e) { return __atomic_compare_exchange_n(p, &e, 1, 1, __ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE); }",
        "int subject(int *p, int *e, int d) { return __atomic_compare_exchange(p, e, &d, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED); }",
        // The weak form may fail spuriously: the retry loop C programs write.
        "int subject(atomic_int *a) { int e = 0; int ok; do { ok = atomic_compare_exchange_weak(a, &e, 1); } while (!ok); return e; }",
    ] {
        assert_safe(source);
    }
}

/// Fences touch no object: a thread or signal fence with a literal order is
/// custody-neutral. An enumerator order stays unevaluated in G
/// (CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T).
#[test]
fn g_routes_fences() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { __atomic_thread_fence(__ATOMIC_SEQ_CST); }",
        "void subject(void) { __c11_atomic_thread_fence(3); }",
        "void subject(void) { __atomic_signal_fence(__ATOMIC_ACQUIRE); }",
        "void subject(void) { atomic_signal_fence(2); }",
        // A consume fence is an acquire fence (C11 7.17.4.1p2).
        "void subject(void) { __atomic_thread_fence(__ATOMIC_CONSUME); }",
    ] {
        assert_safe(source);
    }
    assert_refused(
        "void subject(void) { atomic_thread_fence(memory_order_release); }",
        "memory order of atomic `__c11_atomic_thread_fence` is not an integer constant the atomic route evaluates (CG19)",
    );
}

/// What stays refused, each by name: consume on a read-modify-write or a
/// compare-exchange (never strengthened to acquire), an invalid or too strong
/// failure order, an unevaluated weak flag, the legacy `__sync` builtins, the
/// generic by-pointer exchange, an operand the route does not read as data,
/// and an `_Atomic` read-modify-write of a named object.
#[test]
fn g_refuses_consume_invalid_orders_and_legacy_forms_by_name() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert_refused(
        "int subject(int *p) { return __atomic_fetch_add(p, 1, __ATOMIC_CONSUME); }",
        "memory_order_consume on `__atomic_fetch_add` is outside the weak-memory model: dependency ordering is not modelled and acquire would be a strengthening (CG28)",
    );
    assert_refused(
        "int subject(int *p, int e) { return __atomic_compare_exchange_n(p, &e, 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_CONSUME); }",
        "memory_order_consume on `__atomic_compare_exchange_n` is outside the weak-memory model: dependency ordering is not modelled and acquire would be a strengthening (CG28)",
    );
    let invalid = "memory order of atomic `__atomic_compare_exchange_n` is not valid for its operation (C11 7.17.7)";
    // Failure stronger than success, and a release failure order.
    assert_refused(
        "int subject(int *p, int e) { return __atomic_compare_exchange_n(p, &e, 1, 0, __ATOMIC_RELAXED, __ATOMIC_ACQUIRE); }",
        invalid,
    );
    assert_refused(
        "int subject(int *p, int e) { return __atomic_compare_exchange_n(p, &e, 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELEASE); }",
        invalid,
    );
    assert_refused(
        "int subject(int *p, int e, int w) { return __atomic_compare_exchange_n(p, &e, 1, w, 5, 5); }",
        "weak flag of compare-exchange `__atomic_compare_exchange_n` is not an integer constant the atomic route evaluates (CG28)",
    );
    assert_refused(
        "void subject(void) { __sync_synchronize(); }",
        "legacy `__sync_synchronize` is a GCC full barrier with no C11 memory order and is not routed to the weak-memory model (CG28)",
    );
    // Clang names the sized `__sync` builtin, which no table lists. `long long`
    // is 8 bytes on LP64 and LLP64 alike (`long` is 4 on Windows, which names
    // `__sync_fetch_and_add_4`).
    assert_refused(
        "long long subject(long long *p) { return __sync_fetch_and_add(p, 1); }",
        "unlisted atomic form `__sync_fetch_and_add_8` (CG19)",
    );
    assert_refused(
        "int subject(int *p, int v) { int r; __atomic_exchange(p, &v, &r, 5); return r; }",
        "generic by-pointer atomic `__atomic_exchange` is not routed to the weak-memory model (CG19)",
    );
    assert_refused(
        "int subject(atomic_int *a) { return atomic_fetch_add(a, opaque()); }",
        "atomic `__c11_atomic_fetch_add` has an operand shape the atomic route does not read (CG19)",
    );
    assert_refused(
        "_Atomic int counter; void subject(void) { counter++; }",
        "atomic `_Atomic increment or decrement` has an operand shape the atomic route does not read (CG19)",
    );
    // A pointer-valued exchange is a custody transfer the route does not read.
    let pointer =
        ga("void *subject(void **slot, void *v) { return __atomic_exchange_n(slot, v, 5); }");
    assert!(!pointer.safe(), "{pointer:?}");
}

/// A routed access keeps the dereference obligation of its object and of a
/// compare-exchange's `expected` pointer: a released object is not read or
/// written through.
#[test]
fn g_keeps_the_dereference_obligations() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { int *p = q_alloc(); q_free(p); __atomic_fetch_add(p, 1, 0); }",
        "int subject(int *p) { int *e = q_alloc(); q_free(e); return __atomic_compare_exchange_n(p, e, 1, 0, 5, 5); }",
        "int subject(int *e) { int *p = q_alloc(); q_free(p); return __atomic_compare_exchange_n(p, e, 1, 0, 5, 5); }",
    ] {
        let run = ga(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
}

/// Route T names a routed read-modify-write or compare-exchange by the
/// route's answer (BEFORE: a read-modify-write was refused by name as
/// unrepresented); no typed-source consumer decides one.
#[test]
fn t_names_routed_events() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert_eq!(
        ta("int subject(int *p) { return __atomic_fetch_add(p, 1, 5); }"),
        "atomic `__atomic_fetch_add` is routed to the weak-memory model, which no typed-source consumer decides (CG19)"
    );
    assert_eq!(
        ta("int subject(int *p, int *e) { return __atomic_compare_exchange_n(p, e, 1, 0, 5, 5); }"),
        "atomic `__atomic_compare_exchange_n` is routed to the weak-memory model, which no typed-source consumer decides (CG19)"
    );
    assert_eq!(
        ta("int subject(int *p) { return __atomic_fetch_add(p, 1, 1); }"),
        "memory_order_consume on `__atomic_fetch_add` is outside the weak-memory model: dependency ordering is not modelled and acquire would be a strengthening (CG28)"
    );
}
