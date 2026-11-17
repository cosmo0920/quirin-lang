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

//! Card THREAD-LOCAL-CUSTODY-FREE-READ (gateway rule 54): a whole read of a
//! thread-local pointer object copied into a local is a custody-free value,
//! the reading `pthread_getspecific`'s result already gets. The brief's
//! fixtures: a plain getter is `Safe`; a getter that also frees through it, a
//! write to the thread-local object, `&` of it and a member read of a
//! thread-local record are refused; a caller that releases the getter's
//! result still answers for it. Contracts are the CG00 fixture assumptions
//! (`support/clang_gap.rs`).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const LEAN: &str = include_str!("../../../proof/QCore/ThreadLocalRead.lean");

/// Refused, never accused: `Unknown` with no leak, double release or
/// use-after-release candidate.
fn refused(run: &G) -> bool {
    run.unknown() && !run.has("Leak") && !run.has("DoubleRelease") && !run.has("UseAfterRelease")
}

#[test]
fn a_plain_thread_local_getter_is_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        // `FLB_TLS_GET` with `FLB_HAVE_C_TLS` (`flb_engine_evl_get`).
        "struct s; __thread struct s *t_ctx; \
         struct s *subject(void) { struct s *c; c = t_ctx; return c; }",
        // The initializer form.
        "struct s; __thread struct s *t_ctx; \
         struct s *subject(void) { struct s *c = t_ctx; return c; }",
        // C11's spelling, and a typedef of a record pointer (`flb_ctx_t`).
        "typedef struct s s_t; _Thread_local s_t *t_ctx; \
         s_t *subject(void) { s_t *c; c = t_ctx; return c; }",
        // An `extern` declaration in a header and the definition.
        "extern __thread void *t_ctx; __thread void *t_ctx; \
         void *subject(void) { void *c; c = t_ctx; return c; }",
        // A null test and a copy between locals.
        "__thread void *t_ctx; void *subject(void) { void *c; void *d; c = t_ctx; \
         if (c == 0) return 0; d = c; return d; }",
        // A block-scope thread-local object.
        "void *subject(void) { static __thread void *t_ctx; void *c; c = t_ctx; return c; }",
    ] {
        let run = g(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

#[test]
fn a_getter_that_also_frees_through_it_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "__thread void *t_ctx; void subject(void) { void *c; c = t_ctx; q_free(c); }",
        "__thread void *t_ctx; void *subject(void) { void *c; c = t_ctx; q_free(c); return c; }",
        "__thread void *t_ctx; void subject(void) { q_free(t_ctx); }",
        "__thread void *t_ctx; void subject(void) { void *c; c = t_ctx; q_transfer(c); }",
        // A store through the copy, and a call actual of it, withhold the
        // reading too: nothing the body does through the value is admitted.
        "struct r { int x; }; __thread struct r *t_ctx; \
         struct r *subject(void) { struct r *c; c = t_ctx; c->x = 1; return c; }",
        "__thread void *t_ctx; void *subject(void) { void *c; c = t_ctx; q_observe(c); return c; }",
    ] {
        let run = g(source);
        assert!(refused(&run), "{source}: {run:?}");
    }
}

#[test]
fn a_write_to_the_thread_local_object_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        // A take-from-thread-local: the copy would hide a transfer.
        "__thread void *t_ctx; void *subject(void) { void *c; c = t_ctx; t_ctx = 0; return c; }",
        "__thread void *t_ctx; void *subject(void *p) { void *c; c = t_ctx; t_ctx = p; return c; }",
        "__thread char *t_ctx; char *subject(void) { char *c; c = t_ctx; t_ctx++; return c; }",
    ] {
        let run = g(source);
        assert!(refused(&run), "{source}: {run:?}");
    }
}

#[test]
fn the_address_of_the_thread_local_object_is_refused() {
    if !clang_available() {
        return;
    }
    let source = "__thread void *t_ctx; void *subject(void) { void **a = &t_ctx; void *c; \
                  c = t_ctx; (void)a; return c; }";
    let run = g(source);
    assert!(refused(&run), "{run:?}");
}

#[test]
fn a_member_of_a_thread_local_record_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "struct r { void *p; }; __thread struct r t_rec; \
         void *subject(void) { void *c; c = t_rec.p; return c; }",
        "struct r { void *p; }; __thread struct r *t_rec; \
         void *subject(void) { void *c; c = t_rec->p; return c; }",
        // An array is no pointer object.
        "__thread char t_buf[16]; char *subject(void) { char *c; c = t_buf; return c; }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(!run.unsafe_(), "{source}: {run:?}");
    }
}

#[test]
fn a_caller_that_releases_the_result_still_answers_for_it() {
    if !clang_available() {
        return;
    }
    let source = "__thread void *t_ctx; \
                  void *get(void) { void *c; c = t_ctx; return c; } \
                  void subject(void) { void *p = get(); q_free(p); }";
    let getter = g_function(source, "get");
    assert!(getter.safe(), "{getter:?}");
    let caller = g_function(source, "subject");
    assert!(refused(&caller), "{caller:?}");
}

#[test]
fn a_static_object_and_a_formal_destination_keep_the_old_answer() {
    if !clang_available() {
        return;
    }
    for source in [
        // Outside the card: a static (not thread-local) object.
        "struct s; static struct s *g_ctx; \
         struct s *subject(void) { struct s *c; c = g_ctx; return c; }",
        "struct s; struct s *g_ctx; struct s *subject(void) { struct s *c; c = g_ctx; return c; }",
        // A formal destination is not an automatic local.
        "__thread void *t_ctx; void *subject(void *p) { p = t_ctx; return p; }",
    ] {
        let run = g(source);
        assert!(refused(&run), "{source}: {run:?}");
        assert!(run.has("UnknownAlias"), "{source}: {run:?}");
    }
}

#[test]
fn a_shadowed_name_is_not_read_as_thread_local() {
    if !clang_available() {
        return;
    }
    let source = "__thread void *t_ctx; void *subject(void) { void *c; \
                  { void *t_ctx = q_alloc(); q_free(t_ctx); } c = t_ctx; return c; }";
    let run = g(source);
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn the_lean_decision_vectors_match_the_shared_table() {
    let lean = LEAN.replace("\r\n", "\n");
    let block = lean
        .split("def decisionVectors ")
        .nth(1)
        .expect("decisionVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(|field| field.trim().trim_start_matches('.').to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let expected: Vec<Vec<String>> = include_str!("../src/thread_local_read.rs")
        .split("pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[")
        .nth(1)
        .expect("the Rust table")
        .split("];")
        .next()
        .expect("end of the Rust table")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            line.trim_start_matches('(')
                .trim_end_matches("),")
                .split(',')
                .map(|field| field.trim().trim_matches('"').to_owned())
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), 22);
    assert_eq!(rows, expected);
}
