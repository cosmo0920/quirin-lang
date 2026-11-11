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

//! Card CT24-CLEANUP-PREEMPTED (wave 30): the preempting scope refusal now
//! carries the exit rule's own loan judgment where the frame's cleanup would
//! destroy a lent owner before its borrower's join — the construct CT24 names.
//!
//! The wave-26 CT lane (`docs/reports/ct-wire-lane/README.md` §3.5) measured
//! CT24's distinguishing fixture unwritable: the cleanup path is refused first,
//! on the task obligation (`ScopeBusy`), so the refusal was never attributable
//! to a cleanup destroying an owner while a child is unjoined. The judgment
//! that names that construct already existed — the `exit` rule's "lent to an
//! unjoined child" backstop, recorded as redundant because the scope rule
//! preempts it — and its result was discarded in favour of the bare debug
//! rendering. This card re-emits the preempting refusal from that judgment,
//! on the wave-29 CT47 precedent: same code (`Q600`), same span, same refused
//! set, message attributable to the destroy-before-join.
//!
//! Every case goes through the real pipeline — parse, elaborate, lower, then
//! `check_task_module` — and every expected message is the one measured.
use qcore_core::task_execution::{CheckedTaskModule, check_task_module};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;

/// The CT24 frame: a parent-owned Buffer, a child holding a counted read
/// share, and a fallible allocation inside the thread scope whose `?` is the
/// early exit that would run the frame's cleanup over the owner.
const CLEANUP_BEFORE_JOIN: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s reader(read buffer share 512) {
      Result::Ok { value: handle } => {
        let second = qcore_buffer_new(7)?;
        drop second;
        let seen = join move handle?;
        drop buffer;
        return seen;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// The wave-26 lane's measured refusal for the CT24 `?` shape was the bare
/// `ScopeBusy(5)`. The refusal is unmoved — same code, same span, same
/// refused program — and its message now carries the exit rule's own loan
/// judgment, so it is attributable to the construct CT24 names.
const PREEMPTED_CLEANUP: &str = "ScopeBusy(5): cleanup preempted — this exit would run the \
frame's cleanup over resource 1 while task(s) [2] still hold a read share of it, destroying \
the owner before the child's join; the scope refuses the exit with that loan outstanding";

fn checked(source: &str) -> Result<CheckedTaskModule, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let typed = elaborate_source(&parsed).map_err(|e| format!("{e:?}"))?;
    let module = lower_source_tasks(&parsed, &typed, "main").map_err(|e| format!("{e:?}"))?;
    check_task_module(&module).map_err(|e| format!("{e:?}"))
}

#[track_caller]
fn rejects(source: &str, expected: &str) {
    let errors = checked(source).expect_err("the profile must refuse this program");
    assert!(
        errors.contains(expected),
        "expected a refusal containing {expected:?}, got:\n{errors}"
    );
}

/// The CT24 shape itself: an early exit (`?`) inside the thread scope while
/// the child is unjoined and holds a read share of the frame's owner. Before
/// this card the refusal was the bare `ScopeBusy(5)` — the wave-26 lane's
/// measurement, never attributable to the cleanup or the owner. It now names
/// the destroy-before-join, from the same judgment the exit backstop reads.
#[test]
fn ct24s_preempted_cleanup_is_refused_for_its_own_reason() {
    rejects(CLEANUP_BEFORE_JOIN, PREEMPTED_CLEANUP);
}

/// Attribution control (wave-26 shape 1, byte-identical): the explicit `drop`
/// before the join keeps its own refusal, attributable to the drop — which is
/// exactly why the lane declined to map CT24 on it. This card moves nothing
/// about it.
#[test]
fn the_explicit_drop_shape_keeps_its_loan_refusal_unchanged() {
    let source = CLEANUP_BEFORE_JOIN.replace(
        "let second = qcore_buffer_new(7)?;\n        drop second;\n        let seen",
        "drop buffer;\n        let seen",
    );
    // The later drop is now a double release on the same path, so the measured
    // refusal is the first one reached: the drop against the outstanding loan.
    rejects(
        &source,
        "LoanOutstanding { resource: 1, outstanding: 512, borrowers: [2] }",
    );
}

/// Rendering control: a busy scope whose unjoined child holds no loan keeps
/// the bare `ScopeBusy(..)` rendering byte for byte — there is no owner whose
/// cleanup the exit preempts, so nothing is attributed.
#[test]
fn a_busy_scope_without_a_lent_owner_keeps_the_bare_rendering() {
    let source = "#import std.concurrency; concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let value = 42;
  thread scope s {
    match spawn s worker(copy value) {
      Result::Ok { value: handle } => {
        let second = qcore_buffer_new(7)?;
        drop second;
        let seen = join move handle?;
        return seen;
      }
      Result::Err { error: failure } => {
        return 0;
      }
    }
  }
}";
    let errors = checked(source).expect_err("the busy scope still refuses the early exit");
    assert!(errors.contains("ScopeBusy(4)"), "{errors}");
    assert!(!errors.contains("cleanup preempted"), "{errors}");
}

/// The same construct at an explicit `return`: an early return's teardown is
/// the same preempted cleanup point, so it carries the same attribution.
#[test]
fn an_early_return_that_abandons_the_borrower_names_the_same_construct() {
    let source = CLEANUP_BEFORE_JOIN.replace(
        "let second = qcore_buffer_new(7)?;\n        drop second;\n        let seen = join move handle?;\n        drop buffer;\n        return seen;",
        "return 7;",
    );
    rejects(&source, PREEMPTED_CLEANUP);
}

/// Boundary control: joining the borrower and releasing the owner before the
/// fallible statement is exactly what flips the verdict — the same program
/// with the join and the release ahead of the `?` is admitted whole.
#[test]
fn joining_the_borrower_before_the_early_exit_is_admitted() {
    let source = CLEANUP_BEFORE_JOIN.replace(
        "let second = qcore_buffer_new(7)?;\n        drop second;\n        let seen = join move handle?;\n        drop buffer;\n        return seen;",
        "let seen = join move handle?;\n        drop buffer;\n        let second = qcore_buffer_new(7)?;\n        drop second;\n        return seen;",
    );
    checked(&source).unwrap();
}
