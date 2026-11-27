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

//! Card FB-CHUNK-LIFECYCLE probe: a `static` function whose integer formal
//! every call in the unit fixes to a constant, the shape of Fluent Bit's
//! `sb_remove_chunk_from_segregated_backlog(target_chunk, backlog, destroy)`
//! (`plugins/in_storage_backlog/sb.c:214-246`, its only call passing
//! `FLB_TRUE`). Base verdicts pinned. Conventions as in `clang_gap_phase1.rs`:
//! `Safe` here is over the synthetic `q_*` fixture assumptions and promotes
//! nothing; an `Unsafe` is a provisional verdict over the same fixtures.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

/// The probed body: an owned object released only on the `destroy` arm.
const BODY: &str = "static void subject(int destroy) \
     { void *p = q_alloc(); if (destroy) q_free(p); }";

fn with_callers(body: &str, callers: &str) -> G {
    g(&format!("{body}\n{callers}"))
}

#[test]
fn base_the_unconstrained_formal_keeps_the_false_arm() {
    if !clang_available() {
        return;
    }
    // Every call passes 1, so the false arm is not reachable in the unit;
    // the function-scope reading still reports the leak on it.
    for callers in [
        "void caller(void) { subject(1); }",
        "void caller(void) { subject(1); subject(1); }",
        "void caller(void) { subject(1); } void other(void) { subject((1)); }",
    ] {
        let row = with_callers(BODY, callers);
        assert!(row.unsafe_(), "{callers}: {row:?}");
    }
}

#[test]
fn base_every_other_shape_is_unsafe() {
    if !clang_available() {
        return;
    }
    for callers in [
        "void caller(void) { subject(0); }",
        "void caller(void) { subject(1); subject(0); }",
        "void caller(int x) { subject(x); }",
        "void caller(void) { void (*fp)(int) = subject; fp(1); }",
        "",
    ] {
        let row = with_callers(BODY, callers);
        assert!(row.unsafe_(), "{callers}: {row:?}");
    }
}
