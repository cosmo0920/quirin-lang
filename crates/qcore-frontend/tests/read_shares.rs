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

//! Source-level acceptance for complementary fractional read shares (card T05).
//!
//! Every case goes through the real pipeline — parse, elaborate, lower, then
//! `check_task_module` — so an "admitted" assertion is an all-paths result of
//! the same checker the driver publishes, and a refusal is the checker's own,
//! not a lowering convenience. Each expected message is the one actually
//! measured, so a refusal that moves to a different gate is a visible change
//! rather than a silently satisfied substring.
//!
//! This file is about *two readers of one allocation*. It is not about
//! disjoint slicing, write shares, more than two readers, nested workers, or
//! several lent resources at once: each of those is a measured refusal below.
use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{
    CheckedTaskModule, TaskExecutionScript, check_task_module, execute_task_module,
    render_task_observation,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;

/// One allocation, two children, complementary halves, both reading. The
/// parent writes before lending and after both joins, and reads in between —
/// so the write's exclusive-authority requirement is exercised on an admitted
/// path rather than only on a refused one.
const TWO_READERS: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(1)?;
  let prepared = qcore_buffer_write(buffer, 42);
  thread scope s {
    match spawn s reader(read buffer share FIRST_SHARE) {
      Result::Ok { value: first } => {
        match spawn s reader(read buffer share SECOND_SHARE) {
          Result::Ok { value: second } => {
            let during = qcore_buffer_read(buffer);
            let a = join move first?;
            let b = join move second?;
            let restored = qcore_buffer_write(buffer, 7);
            let after = qcore_buffer_read(buffer);
            drop buffer;
            return after;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// `TWO_READERS` with both share placeholders filled in.
fn shares(first: &str, second: &str) -> String {
    TWO_READERS
        .replace("FIRST_SHARE", first)
        .replace("SECOND_SHARE", second)
}

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

#[track_caller]
fn admits(source: &str) -> CheckedTaskModule {
    checked(source).unwrap_or_else(|error| panic!("this program must be admitted:\n{error}"))
}

/// T05 acceptance 1 and 3, on the admitted path. Two positive complementary
/// shares of one allocation give two readers; both read the *same* generation,
/// the allocation is created once and released once, and the parent's own
/// read survives the whole lending window.
#[test]
fn two_complementary_halves_admit_two_readers_of_one_allocation() {
    let module = admits(&shares("512", "512"));
    let result = execute_task_module(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![true, true],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    assert_eq!(result.root_outcome, TaskOutcome::I32(7));
    let trace = render_task_observation(&result);
    let line = |needle: &str| {
        trace
            .lines()
            .position(|entry| entry == needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in:\n{trace}"))
    };

    // One allocation, counted once: two borrowers, one generation, one drop.
    assert_eq!(trace.matches("QCORE_BUFFER_ALLOC ").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_DROP ").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_LOAN 1 1\n").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_LOAN 2 1\n").count(), 1);
    // Both children read generation 1 — the lender's own object, not a copy.
    assert_eq!(trace.matches("QCORE_BUFFER_READ 1 1 42\n").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_READ 2 1 42\n").count(), 1);

    // The write happens before either share leaves, and the second write only
    // after both shares are back.
    assert!(line("QCORE_BUFFER_WRITE 0 1 42") < line("QCORE_BUFFER_LOAN 1 1"));
    assert!(line("QCORE_BUFFER_RETURN 1 1") < line("QCORE_BUFFER_WRITE 0 1 7"));
    assert!(line("QCORE_BUFFER_RETURN 2 1") < line("QCORE_BUFFER_WRITE 0 1 7"));
    // The second write is observable: the parent's later read sees it.
    assert!(line("QCORE_BUFFER_WRITE 0 1 7") < line("QCORE_BUFFER_READ 0 1 7"));
    assert!(line("QCORE_BUFFER_READ 0 1 7") < line("QCORE_BUFFER_DROP 0 1"));
    assert!(result.tasks[1].joined && result.tasks[2].joined);
}

/// T05 acceptance 1: excess allocation of authority is refused, and the
/// refusal reports the outstanding total and the request rather than a vague
/// bound. The complementary case that exactly fills the authority is admitted,
/// so this is a boundary rather than a blanket refusal of second readers.
#[test]
fn lending_more_than_the_remaining_authority_is_refused() {
    rejects(
        &shares("512", "513"),
        "ShareExceedsAuthority { resource: 1, outstanding: 512, requested: 513 }",
    );
    // Removing one unit from the first share leaves room for the larger second.
    admits(&shares("511", "513"));
    // Two whole loans are the same refusal at the extreme.
    rejects(
        &shares("FULL", "FULL"),
        "ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1024 }",
    );
    // And one unit past a full budget is refused just as completely.
    rejects(
        &shares("1024", "1"),
        "ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1 }",
    );
}

/// T05 acceptance 1: a write demands exclusive authority, so it is refused for
/// the whole window in which any share is outstanding — including after only
/// one of the two borrowers has been joined.
#[test]
fn a_write_is_refused_while_any_read_share_is_outstanding() {
    // Between the two creations: one half is out.
    rejects(
        &shares("512", "512").replace(
            "let during = qcore_buffer_read(buffer);",
            "let during = qcore_buffer_write(buffer, 9);",
        ),
        "LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2, 3] }",
    );
    // After one join: the other borrower still holds its half.
    rejects(
        &shares("512", "512").replace(
            "let b = join move second?;",
            "let mid = qcore_buffer_write(buffer, 9); let b = join move second?;",
        ),
        "LoanOutstanding { resource: 1, outstanding: 512, borrowers: [3] }",
    );
    // A borrower has no write authority at all: its binding is not an owner.
    rejects(
        &shares("512", "512").replace(
            "let seen = qcore_buffer_read(payload);",
            "let seen = qcore_buffer_write(payload, 5);",
        ),
        "buffer write requires own<Buffer>; a read loan grants no write authority",
    );
}

/// T05 acceptance 2: full authority is reconstructed only when every share is
/// back. Releasing, moving and re-lending beyond the budget are each refused
/// while one borrower retains a share.
#[test]
fn reconstruction_to_full_authority_is_refused_while_a_share_is_retained() {
    // Release after only the first join.
    rejects(
        &shares("512", "512").replace(
            "let b = join move second?;",
            "drop buffer; let b = join move second?;",
        ),
        "LoanOutstanding { resource: 1, outstanding: 512, borrowers: [3] }",
    );
    // A path that leaves without joining the second borrower cannot exit the
    // scope at all. The release is removed too, so the measured refusal is the
    // scope rule rather than the release rule that would fire first. Pin moved
    // deliberately (card CT24-CLEANUP-PREEMPTED): the scope rule's refusal now
    // carries the exit backstop's own loan judgment — here naming the second
    // borrower alone, because the first join already reclaimed its share.
    rejects(
        &shares("512", "512").replace(
            "let a = join move first?;\n            let b = join move second?;\n            let restored = qcore_buffer_write(buffer, 7);\n            let after = qcore_buffer_read(buffer);\n            drop buffer;\n            return after;",
            "let a = join move first?;\n            return a;",
        ),
        "cleanup preempted — this exit would run the frame's cleanup over resource 1 \
         while task(s) [3] still hold a read share of it",
    );
}

/// One allocation at a time. Two *different* lent resources are outside this
/// profile even when each has authority to spare, and the refusal says so
/// rather than approximating a second budget.
#[test]
fn only_one_allocation_may_be_lent_at_a_time() {
    let source = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let first = qcore_buffer_new(1)?;
  match qcore_buffer_new(2) {
    Result::Ok { value: second } => {
      thread scope s {
        match spawn s reader(read first share 512) {
          Result::Ok { value: a } => {
            match spawn s reader(read second share 512) {
              Result::Ok { value: b } => {
                let x = join move a?;
                let y = join move b?;
                drop first;
                drop second;
                return x;
              }
              Result::Err { error: e2 } => {
                let x = join move a?;
                drop first;
                drop second;
                return x;
              }
            }
          }
          Result::Err { error: e1 } => {
            drop first;
            drop second;
            return 0;
          }
        }
      }
    }
    Result::Err { error: e0 } => {
      drop first;
      return 0;
    }
  }
}";
    rejects(source, "at most one outstanding read loan at a time");
}

/// A third simultaneous reader is a represented refusal, not an unmodelled
/// case, even when the remaining authority would allow it.
#[test]
fn a_third_simultaneous_reader_is_refused_by_the_declared_bound() {
    let source = shares("1", "1").replace(
        "let during = qcore_buffer_read(buffer);",
        "match spawn s reader(read buffer share 1) {
              Result::Ok { value: third } => {
                let c = join move third?;
              }
              Result::Err { error: denied } => {
              }
            }",
    );
    rejects(&source, "TooManyReaders { resource: 1, limit: 2 }");
}

/// The share is a static property of the capture, so lending is refused on
/// its own path rather than by an aggregate over the program. A creation that
/// fails lends nothing, so the *other* arm of the same match keeps exactly the
/// authority it had.
#[test]
fn a_failed_second_creation_returns_only_its_own_share() {
    let module = admits(&shares("512", "512"));
    let result = execute_task_module(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![true, false],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    let trace = render_task_observation(&result);
    // Exactly one loan was opened, and exactly one came back.
    assert_eq!(trace.matches("QCORE_BUFFER_LOAN ").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_RETURN ").count(), 1);
    assert!(trace.contains("QCORE_TASK_CREATE_FAILED 0\n"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_DROP 0 1\n"), "{trace}");
    // Only the first write ran; the failure arm releases without writing again.
    assert_eq!(trace.matches("QCORE_BUFFER_WRITE ").count(), 1);
    assert_eq!(result.tasks.len(), 2);
}

/// A write is an owner's operation over an exact owned name and one i32. Every
/// other spelling is refused rather than reinterpreted.
#[test]
fn the_write_form_is_exact() {
    for (form, expected) in [
        (
            "qcore_buffer_write(buffer)",
            "buffer write requires one owned-buffer name and one i32 value",
        ),
        // Deliberate pin update (card `WRITE-LEND-TASKS-EXERCISE`, wave 36).
        // The three-argument spelling now *means* something — it is the byte
        // write through a range write lease — so this line is refused for the
        // **kind** of `buffer` rather than for its shape. The point the row
        // makes is unchanged and arguably sharper: an owner still cannot reach
        // a byte write, because the whole-payload write remains its only write
        // footprint (`WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`).
        (
            "qcore_buffer_write(buffer, 1, 2)",
            "a byte write requires lend<Buffer>",
        ),
        ("qcore_buffer_write(missing, 1)", "unknown name `missing`"),
    ] {
        rejects(
            &shares("512", "512").replace("qcore_buffer_write(buffer, 42)", form),
            expected,
        );
    }
    // A non-buffer local is refused on the place type, not silently coerced.
    rejects(
        &shares("512", "512").replace(
            "qcore_buffer_write(buffer, 7)",
            "qcore_buffer_write(prepared, 7)",
        ),
        "buffer write requires own<Buffer>; a read loan grants no write authority",
    );
    // A released buffer has no write authority either. The lexical binding
    // still says `own<Buffer>`, so this one is refused by the checker's own
    // custody state rather than by the source type — which is the point: the
    // write is decided by the ledger, on the path that reaches it.
    rejects(
        &shares("512", "512").replace(
            "drop buffer;\n            return after;",
            "drop buffer;\n            let late = qcore_buffer_write(buffer, 3);\n            return after;",
        ),
        "slot 4 is not an initialized owned buffer",
    );
}

/// Pointer copies preserve permission identity: handing the same allocation to
/// two children creates no second identity, and a borrower still cannot turn
/// its share into a value it keeps. Both T04 refusals survive the share
/// algebra unchanged.
#[test]
fn a_share_is_not_a_value_a_borrower_can_keep() {
    for (body, expected) in [
        ("drop payload; return 0;", "drop requires an owned buffer"),
        (
            "let copied = payload; return 0;",
            "a read loan cannot be copied",
        ),
        ("return move payload;", "move return requires own<Buffer>"),
    ] {
        rejects(
            &shares("512", "512").replace(
                "let seen = qcore_buffer_read(payload);\n  return seen;",
                body,
            ),
            expected,
        );
    }
    // One identity: both borrowers' reads name the same generation, and the
    // module records exactly one owned allocation site.
    let module = admits(&shares("512", "512"));
    let allocations = module
        .module()
        .functions
        .values()
        .flat_map(|function| function.blocks.values())
        .filter(|block| {
            matches!(
                block.terminator.kind,
                qcore_core::task_execution::TaskTerminatorKind::TryAllocateBuffer { .. }
            )
        })
        .count();
    assert_eq!(allocations, 1);
}
