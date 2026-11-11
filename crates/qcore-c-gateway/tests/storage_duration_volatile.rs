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

//! Card CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h): the gateway's
//! verdicts on the pinned shapes, bound to
//! `QCore.StorageDuration.shapeVectors`, and the brief's refusal controls.
//! Contracts are the CG00 fixture assumptions (`support/clang_gap.rs`).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const LEAN: &str = include_str!("../../../proof/QCore/StorageDuration.lean");

/// `(name, source)` of every pinned shape; the expected verdict class is
/// Lean's (`shapeVectors`).
const SHAPE_VECTORS: &[(&str, &str)] = &[
    (
        "table-reader",
        "const char *subject(int op) { static const char *names[] = {\"a\", \"b\"}; \
         if (op < 0 || op >= 2) return \"x\"; return names[op]; }",
    ),
    (
        "thread-local-formatter",
        "char *subject(void) { static _Thread_local char ret[4][16]; \
         static _Thread_local int reti = 0; reti = (reti + 1) % 4; cg_log(ret[reti]); \
         return ret[reti]; }",
    ),
    (
        "static-scalar-counter",
        "int subject(void) { static int n; n++; return n; }",
    ),
    (
        "static-cache-allocation",
        "void subject(void) { static void *cache; cache = q_alloc(); }",
    ),
    (
        "static-initializer-per-call",
        "void subject(void) { static void *cache = 0; if (cache) q_free(cache); }",
    ),
    (
        "global-release",
        "void *g_slot; void subject(void) { q_free(g_slot); }",
    ),
    (
        "volatile-sum",
        "int subject(volatile int *v) { return *v + *v; }",
    ),
    (
        "volatile-spin-wait",
        "volatile int g_flag; void subject(void) { void *p = q_alloc(); while (!g_flag) { } \
         q_free(p); }",
    ),
    (
        "volatile-pointer-binding",
        "void subject(void) { void * volatile p = q_alloc(); if (p) q_free(p); }",
    ),
    (
        "volatile-flag-known-zero",
        "void subject(void) { void *p = q_alloc(); volatile int flag = 0; if (flag) q_free(p); \
         q_free(p); }",
    ),
    (
        "volatile-branch-release",
        "volatile int g_flag; void subject(void) { void *p = q_alloc(); \
         if (g_flag) { q_free(p); return; } q_free(p); }",
    ),
];

/// Lean's `shapeVectors`: `(name, expected class)`.
fn lean_shapes() -> Vec<(String, String)> {
    let start = "def shapeVectors : List (String × String × Verdict) := [";
    let body = LEAN.split(start).nth(1).expect("shapeVectors");
    let body = &body[..body.find("\n]").expect("list end")];
    body.lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(|line| {
            let quoted = line.split('"').skip(1).step_by(2).collect::<Vec<_>>();
            let class = line
                .rsplit_once(", .")
                .map(|(_, class)| class.trim_end().trim_end_matches([')', ',']).to_owned())
                .expect("a verdict");
            (quoted[0].to_owned(), class)
        })
        .collect()
}

fn class(run: &G) -> &'static str {
    if run.safe() {
        "safe"
    } else if run.unknown() && !run.has("Leak") {
        "refuse"
    } else {
        "other"
    }
}

#[test]
fn cg15_shape_vectors_match_lean_and_the_gateway() {
    let lean = lean_shapes();
    assert_eq!(
        lean.iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        SHAPE_VECTORS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
    );
    if !clang_available() {
        return;
    }
    for ((name, expected), (_, source)) in lean.iter().zip(SHAPE_VECTORS) {
        let run = g(source);
        assert_eq!(class(&run), expected, "{name}: {run:?}");
    }
}

#[test]
fn a_function_local_static_cache_is_not_a_per_call_leak_nor_safe() {
    if !clang_available() {
        return;
    }
    // The brief's first refusal control: `static void *cache; cache =
    // malloc(n);` is a retained resource, refused by name (the static-storage
    // publication), never a leak of this call and never Safe.
    for source in [
        "void subject(void) { static void *cache; cache = q_alloc(); }",
        "void subject(void) { static void *cache; if (!cache) cache = q_alloc(); }",
        "void subject(void) { void *p = q_alloc(); static void *keep; keep = p; }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{run:?}");
        assert!(!run.has("Leak"), "{run:?}");
        assert!(
            run.refused_prefix("publication of a tracked resource into the static-storage object"),
            "{run:?}"
        );
    }
}

#[test]
fn a_branch_on_a_volatile_value_that_decides_a_release_is_refused() {
    if !clang_available() {
        return;
    }
    // The brief's second refusal control: `if (*vp) free(p);`, the branch
    // value volatile. Refused (the typed reader refuses the volatile read in
    // the condition), never Safe.
    for source in [
        "void subject(volatile int *vp, void *p) { if (*vp) q_free(p); }",
        "void subject(volatile int *vp) { void *p = q_alloc(); if (*vp) { q_free(p); return; } \
         q_free(p); }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{run:?}");
    }
    // The same decision on a volatile object the route does read is refused
    // by rule 11h itself.
    let run = g(
        "volatile int g_flag; void subject(void) { void *p = q_alloc(); \
         if (g_flag) { q_free(p); return; } q_free(p); }",
    );
    assert!(
        run.refused_prefix("custody decision on a volatile read"),
        "{run:?}"
    );
    // A branch whose arms carry no custody stays admitted.
    let run = g(
        "volatile int g_flag; void subject(void) { void *p = q_alloc(); \
         if (g_flag) { cg_sink(1); } q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn soundness_repairs_that_were_safe_on_the_base() {
    if !clang_available() {
        return;
    }
    // Both were `Safe` before rule 11h: the static initializer re-applied on
    // every call pruned the release; the volatile pointer carried its
    // identity and null fact.
    let run = g("void subject(void) { static void *cache = 0; if (cache) q_free(cache); }");
    assert!(!run.safe(), "{run:?}");
    let run = g("void subject(void) { void * volatile p = q_alloc(); void *q = p; q_free(q); }");
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.refused_prefix("custody through the volatile object `p`"),
        "{run:?}"
    );
    // A known-zero volatile flag no longer prunes the arm that releases.
    let run = g(
        "void subject(void) { void *p = q_alloc(); volatile int flag = 0; \
         if (flag) q_free(p); q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn static_addresses_and_reads_carry_no_identity() {
    if !clang_available() {
        return;
    }
    // A static table whose statement also defines its record tag is no
    // event either.
    let run = g(
        "int subject(void) { static const struct { const char *pfx; int v; } \
         map[] = {{\"a\", 1}, {\"b\", 2}}; return 0; }",
    );
    assert!(run.safe(), "{run:?}");
    // Reading an element of the untagged table is the typed place's (an
    // untagged aggregate is refused there, as before).
    let run = g(
        "int subject(int i) { static const struct { const char *pfx; int v; } \
         map[] = {{\"a\", 1}, {\"b\", 2}}; return map[i & 1].v; }",
    );
    assert!(!run.refused(INITIALIZER), "{run:?}");
    // A static array's address handed out or formatted into: no identity.
    assert!(g("char *subject(int i) { static char ret[4][16]; return ret[i & 3]; }").safe());
    assert!(g("char *subject(void) { static char buf[16]; return &buf[2]; }").safe());
    // An automatic array's address is not a static address.
    let run = g("char *subject(int i) { char ret[4][16]; return ret[i & 3]; }");
    assert!(!run.safe(), "{run:?}");
    // A custody-taking use of a static address or read refuses.
    let run = g("void subject(void) { static char buf[16]; q_free(buf); }");
    assert!(!run.safe(), "{run:?}");
    // An index with a side effect is not a static address.
    let run = g("char *subject(int i) { static char ret[4][16]; return ret[i++ & 3]; }");
    assert!(!run.safe(), "{run:?}");
    // A volatile static array is not read as an address with no effect.
    let run =
        g("volatile char *subject(int i) { static volatile char ret[4][16]; return ret[i & 3]; }");
    assert!(!run.safe(), "{run:?}");
}

/// `CT41-VOLATILE-COMPLEMENT-UNMEASURED`: the three uses of `volatile` for its
/// C11 purposes, each measured through route G and pinned as it answers.
#[test]
fn the_volatile_complement_shapes_answer_as_measured() {
    if !clang_available() {
        return;
    }
    // A memory-mapped device register: a store to and a poll of a fixed
    // address. Refused (the fixed-address place is no place route G reads).
    let run = g(
        "void subject(void) { *(volatile unsigned *)0x40000000UL = 1u; \
         while (*(volatile unsigned *)0x40000004UL & 1u) { } }",
    );
    assert!(run.unknown() && run.refused(DESTINATION), "{run:?}");
    let run = g(
        "void subject(void) { volatile unsigned *reg = (volatile unsigned *)0x40000000UL; \
         *reg = 1u; while (*reg & 1u) { } }",
    );
    assert!(run.unknown(), "{run:?}");
    // An object a signal handler writes: a spin-wait on it is custody-neutral
    // and admitted; a release it decides is refused by rule 11h.
    let flag = "typedef int sig_atomic_t; static volatile sig_atomic_t stop_flag; \
        void on_signal(int s) { (void)s; stop_flag = 1; }";
    let run = g(&format!(
        "{flag} void subject(void) {{ void *p = q_alloc(); while (!stop_flag) {{ }} q_free(p); }}"
    ));
    assert!(run.safe(), "{run:?}");
    let run = g(&format!(
        "{flag} void subject(void) {{ void *p = q_alloc(); if (stop_flag) {{ q_free(p); return; }} \
         q_free(p); }}"
    ));
    assert!(
        run.refused_prefix("custody decision on a volatile read"),
        "{run:?}"
    );
    // An object live across `setjmp`: refused (the volatile binding by rule
    // 11h; `setjmp` itself is an uncontracted call in the condition).
    let jump = "typedef long jmp_buf[8]; int setjmp(jmp_buf env); jmp_buf env;";
    let run = g(&format!(
        "{jump} void subject(void) {{ void * volatile p = q_alloc(); \
         if (setjmp(env)) {{ q_free(p); return; }} q_free(p); }}"
    ));
    assert!(
        run.refused_prefix("custody through the volatile object `p`"),
        "{run:?}"
    );
    let run = g(&format!(
        "{jump} void subject(void) {{ volatile int n = 0; if (setjmp(env)) {{ cg_sink(n); return; }} \
         n = 1; }}"
    ));
    assert!(run.unknown(), "{run:?}");
}
