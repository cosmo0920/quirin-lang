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

//! Card VA-LOOP-AND-POINTER-ARG (gateway rule 12c): a `va_arg` in a
//! controlling expression, and a pointer `va_arg` value as a never-owned
//! opaque binding, through route G (`analyze_unrestricted_c11`).
//!
//! Runs over the CG00 fixture contracts (`support/clang_gap.rs`); every
//! `Safe` is over those trusted test assumptions and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::{G, LOOP, clang_available, g};

const STDARG: &str = "#include <stdarg.h>\n";

fn va(source: &str) -> G {
    g(&format!("{STDARG}{source}"))
}

fn protocol(reason: &str) -> String {
    format!("va_list protocol refused: {reason}")
}

const SHAPE: &str = "va_list operation refused: a `va_arg` in a controlling expression that is \
                     not `(NAME = va_arg(L, T))`";

/// `mk_vhost_set` at the tip of Monkey's `va_end` fix (`21a2c190`), its
/// property setter replaced by the borrowing fixture callee `r_borrow_code`:
/// the loop condition is one event per evaluation, then the test; both
/// pointer values are never-owned opaque bindings handed to borrowed
/// positions.
const TIP: &str = "int subject(void *ctx, int vid, ...)\n{\n int ret;\n char *key;\n\
     char *value;\n va_list va;\n if (!ctx) {\n return -1;\n }\n va_start(va, vid);\n\
     while ((key = va_arg(va, char *))) {\n value = va_arg(va, char *);\n\
     if (!value) {\n va_end(va);\n return -1;\n }\n\
     q_observe(key);\n ret = r_borrow_code(value);\n\
     if (ret != 0) {\n va_end(va);\n return -1;\n }\n }\n\
     va_end(va);\n return 0;\n}";

#[test]
fn the_fixed_vhost_loop_is_established() {
    if !clang_available() {
        return;
    }
    let run = va(TIP);
    assert!(run.safe(), "{run:?}");
    assert!(!run.refused(LOOP), "{run:?}");
}

/// The defective master body (no `va_end` before the `!value` return): still
/// refused for the missing `va_end`, now named at the condition too.
#[test]
fn the_defective_vhost_loop_stays_refused_for_va_end() {
    if !clang_available() {
        return;
    }
    let master = TIP.replace(
        "if (!value) {\n va_end(va);\n return -1;\n }",
        "if (!value) {\n return -1;\n }",
    );
    assert_ne!(master, TIP);
    let run = va(&master);
    assert!(run.unknown(), "{run:?}");
    let missing = protocol("`va` may reach a function exit without `va_end`");
    assert!(run.refused(&missing), "{run:?}");
    assert!(!run.refused(LOOP), "{run:?}");
    assert!(
        run.unsupported.iter().filter(|d| **d == missing).count() >= 3,
        "the condition, the stored value and the ends: {run:?}"
    );
}

#[test]
fn a_scalar_va_arg_in_a_condition_is_established() {
    if !clang_available() {
        return;
    }
    let looped = va(
        "int subject(int n, ...) { va_list ap; int x; int total = 0; void *p = q_alloc(); \
         va_start(ap, n); while ((x = va_arg(ap, int))) { total = total + x; } \
         va_end(ap); q_free(p); return total; }",
    );
    assert!(looped.safe(), "{looped:?}");
    let tested = va(
        "int subject(int n, ...) { va_list ap; int x = 0; va_start(ap, n); \
         if ((x = va_arg(ap, int))) { x = x + 1; } va_end(ap); return x; }",
    );
    assert!(tested.safe(), "{tested:?}");
    let repeated = va(
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); \
         do { n = n - 1; } while ((x = va_arg(ap, int))); va_end(ap); return n; }",
    );
    assert!(repeated.safe(), "{repeated:?}");
}

/// The condition's event happens on every evaluation: a `va_end` in the body
/// makes the next evaluation a read after `va_end`.
#[test]
fn the_back_edge_re_evaluates_the_event() {
    if !clang_available() {
        return;
    }
    let run = va(
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); \
         while ((x = va_arg(ap, int))) { va_end(ap); } return n; }",
    );
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&protocol(
            "`ap` may be read after `va_end`, after being passed to a callee, or before `va_start`"
        )),
        "{run:?}"
    );
}

/// Any other expression shape holding a `va_arg` is refused by name.
#[test]
fn other_condition_shapes_are_refused_by_name() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); \
         while ((x = va_arg(ap, int)) != 0) { n = n + x; } va_end(ap); return n; }",
        "int subject(int n, ...) { va_list ap; va_start(ap, n); \
         if (va_arg(ap, int)) { n = 1; } va_end(ap); return n; }",
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); \
         switch ((x = va_arg(ap, int))) { case 1: n = 2; break; default: break; } \
         va_end(ap); return n; }",
    ] {
        let run = va(source);
        assert!(run.unknown(), "{source}: {run:?}");
        assert!(run.refused(SHAPE), "{source}: {run:?}");
    }
}

/// A pointer `va_arg` passed to a borrowed callee is admitted (declaration
/// and assignment forms).
#[test]
fn a_pointer_va_arg_passed_to_a_borrowed_callee_is_admitted() {
    if !clang_available() {
        return;
    }
    let declared = va("void subject(int n, ...) { va_list ap; va_start(ap, n); \
         void *p = va_arg(ap, void *); q_observe(p); va_end(ap); }");
    assert!(declared.safe(), "{declared:?}");
    let assigned = va(
        "void subject(int n, ...) { va_list ap; char *s; va_start(ap, n); \
         s = va_arg(ap, char *); if (s) { q_observe(s); } va_end(ap); }",
    );
    assert!(assigned.safe(), "{assigned:?}");
}

/// Never owned: releasing it (directly or through the binding) or storing it
/// into custody is never `Safe`.
#[test]
fn a_pointer_va_arg_is_never_released_or_stored() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(int n, ...) { va_list ap; va_start(ap, n); \
         q_free(va_arg(ap, void *)); va_end(ap); }",
        "void subject(int n, ...) { va_list ap; va_start(ap, n); \
         void *p = va_arg(ap, void *); va_end(ap); q_free(p); }",
        "void subject(int n, ...) { va_list ap; void *owner = q_alloc(); va_start(ap, n); \
         void *p = va_arg(ap, void *); q_store(owner, p); va_end(ap); q_free(owner); }",
        "void subject(int n, ...) { va_list ap; va_start(ap, n); \
         void *p = va_arg(ap, void *); q_transfer(p); va_end(ap); }",
    ] {
        let run = va(source);
        assert!(!run.safe(), "{source}: {run:?}");
        // The direct release keeps the uncontracted `va_arg` actual; every
        // use of the binding is an unresolved alias, never applied.
        assert!(
            run.refused_prefix("nested call `__builtin_va_arg` inside a modeled call")
                || run.has("UnknownAlias { variable: \"p\" }"),
            "{source}: {run:?}"
        );
    }
}

/// Never returned as owned: returning the binding is never `Safe`.
#[test]
fn a_pointer_va_arg_is_never_returned_as_owned() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void *subject(int n, ...) { va_list ap; void *p; va_start(ap, n); \
         p = va_arg(ap, void *); va_end(ap); return p; }",
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("UnknownAlias { variable: \"p\" }"), "{run:?}");
}

/// A rebinding of an owned identity by a `va_arg` pointer loses it: never
/// `Safe`.
#[test]
fn overwriting_an_owned_binding_is_never_safe() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void subject(int n, ...) { va_list ap; void *p = q_alloc(); va_start(ap, n); \
         p = va_arg(ap, void *); va_end(ap); q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// Every other pointer destination keeps a named refusal: a static local, a
/// formal. (A function-pointer `va_arg` is refused before this rule: the
/// classifier reads its type name as a call.)
#[test]
fn other_pointer_destinations_keep_a_named_refusal() {
    if !clang_available() {
        return;
    }
    let refused = "va_list operation refused: the `va_arg` value stored into";
    let function_pointer = va("void subject(int n, ...) { va_list ap; void (*f)(void); \
         va_start(ap, n); f = va_arg(ap, void (*)(void)); va_end(ap); }");
    assert!(!function_pointer.safe(), "{function_pointer:?}");
    for source in [
        "void subject(int n, ...) { va_list ap; static void *keep; va_start(ap, n); \
         keep = va_arg(ap, void *); va_end(ap); }",
        "void subject(void *q, ...) { va_list ap; va_start(ap, q); \
         q = va_arg(ap, void *); va_end(ap); }",
    ] {
        let run = va(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(run.refused_prefix(refused), "{source}: {run:?}");
    }
}
