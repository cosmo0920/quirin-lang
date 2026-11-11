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

//! Source-level acceptance for the bounded scoped read loan (card T04).
//!
//! Every case here goes through the real pipeline — parse, elaborate, lower,
//! then `check_task_module` — so a "checks" assertion is an all-paths result of
//! the same checker the driver publishes, and a rejection is the checker's own
//! refusal rather than a lowering convenience. Each expected message is the one
//! actually measured, so a refusal that moves to a different gate is a visible
//! change rather than a silently satisfied substring.
use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{
    CheckedTaskModule, ScalarType, TaskExecutionScript, check_task_module, execute_task_module,
    render_task_observation,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;

/// One parent-owned Buffer, one scoped child read loan, an explicit
/// join-dependent return of authority, and a parent read taken *while* the loan
/// is outstanding. Both creation outcomes are handled explicitly.
const READ_LOAN: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s reader(read buffer share FULL) {
      Result::Ok { value: handle } => {
        let during = qcore_buffer_read(buffer);
        let seen = join move handle?;
        let after = qcore_buffer_read(buffer);
        drop buffer;
        return seen;
      }
      Result::Err { error: failure } => {
        let restored = qcore_buffer_read(buffer);
        drop buffer;
        return restored;
      }
    }
  }
}";

/// Two owned Buffers and two nested creations, so the second lending attempt is
/// reached on a path whose other exits are all well formed. The second
/// allocation is discriminated explicitly, so no path leaks the first Buffer
/// and the measured refusal is the lending one.
const TWO_LENDERS: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let first = qcore_buffer_new(1)?;
  match qcore_buffer_new(2) {
    Result::Ok { value: second } => {
      thread scope s {
        match spawn s reader(read first share FULL) {
          Result::Ok { value: a } => {
            match spawn s reader(read SECOND_LENDER share FULL) {
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

#[test]
fn one_scoped_read_loan_checks_and_returns_authority_at_join() {
    let module = checked(READ_LOAN).unwrap();
    let worker = module
        .module()
        .functions
        .values()
        .find(|function| function.name == "reader")
        .expect("the worker is lowered");
    assert_eq!(worker.parameters.len(), 1);
    assert_eq!(worker.parameters[0].1, ScalarType::READ_LOAN);

    // Successful creation: the parent reads its own buffer while the child
    // holds the loan, the join closes it, and only then is the buffer released.
    let result = execute_task_module(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![true],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    assert_eq!(result.root_outcome, TaskOutcome::I32(42));
    let trace = render_task_observation(&result);
    let line = |needle: &str| {
        trace
            .lines()
            .position(|entry| entry == needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in:\n{trace}"))
    };
    assert!(line("QCORE_TASK_SPAWN 1 0") < line("QCORE_BUFFER_LOAN 1 1"));
    // The parent reads while the loan is outstanding; the borrower's own read
    // is released at join; the parent reads again after authority returns.
    assert!(line("QCORE_BUFFER_LOAN 1 1") < line("QCORE_BUFFER_READ 0 1 42"));
    assert!(line("QCORE_TASK_JOIN 1") < line("QCORE_BUFFER_RETURN 1 1"));
    assert!(line("QCORE_BUFFER_RETURN 1 1") < line("QCORE_BUFFER_DROP 0 1"));
    assert_eq!(trace.matches("QCORE_BUFFER_READ 0 1 42\n").count(), 2);
    assert_eq!(trace.matches("QCORE_BUFFER_READ 1 1 42\n").count(), 1);
    assert_eq!(trace.matches("QCORE_BUFFER_DROP 0 1\n").count(), 1);
    assert!(result.tasks[1].joined);
}

/// T04 acceptance 2, at source level: a creation that failed opened no loan, so
/// the parent's capture state is exactly what it was — including the exclusive
/// authority to release it.
#[test]
fn failed_creation_restores_the_lender_without_opening_a_loan() {
    let module = checked(READ_LOAN).unwrap();
    let result = execute_task_module(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![false],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    assert_eq!(result.root_outcome, TaskOutcome::I32(42));
    let trace = render_task_observation(&result);
    assert!(!trace.contains("QCORE_BUFFER_LOAN"), "{trace}");
    assert!(!trace.contains("QCORE_BUFFER_RETURN"), "{trace}");
    assert!(trace.contains("QCORE_TASK_CREATE_FAILED 0\n"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_READ 0 1 42\n"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_DROP 0 1\n"), "{trace}");
    assert_eq!(result.tasks.len(), 1);
}

/// T04 acceptance 1. Release and re-lending are distinct authorities the owner
/// does not have while the loan is outstanding, and each is refused on the path
/// that reaches it rather than dropped from the projection.
#[test]
fn the_lender_cannot_release_or_relend_while_the_loan_is_outstanding() {
    let released = READ_LOAN.replace(
        "let during = qcore_buffer_read(buffer);",
        "let during = qcore_buffer_read(buffer); drop buffer;",
    );
    rejects(
        &released,
        "LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2] }",
    );

    // The same resource twice, each time for the whole authority. Card T05
    // re-pinned this expectation deliberately: with counted shares, a second
    // whole loan is an over-allocation of authority rather than a demand for
    // exclusive authority, and the refusal now reports the exact amounts. The
    // direction is unchanged — the program is still refused.
    rejects(
        &TWO_LENDERS.replace("SECOND_LENDER", "first"),
        "ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1024 }",
    );
    // A different resource: the structural bound refuses it instead.
    rejects(
        &TWO_LENDERS.replace("SECOND_LENDER", "second"),
        "at most one outstanding read loan at a time",
    );
}

/// An outstanding loan keeps its scope busy, so a path that never joins its
/// borrower cannot leave. The scope rule reaches this first; the exit rule's
/// own loan check is a redundant backstop, recorded as such. **Pin moved
/// deliberately** (card CT24-CLEANUP-PREEMPTED): the preempting refusal now
/// carries that backstop's own judgment — the frame's cleanup would destroy
/// the lent owner before the borrower's join — instead of discarding it; the
/// refusal itself is unmoved (same code, same span, same refused program).
#[test]
fn a_path_that_never_joins_its_borrower_cannot_leave_the_scope() {
    let source = READ_LOAN.replace(
        "let seen = join move handle?;\n        let after = qcore_buffer_read(buffer);\n        drop buffer;\n        return seen;",
        "return 7;",
    );
    rejects(
        &source,
        "ScopeBusy(5): cleanup preempted — this exit would run the frame's cleanup over \
         resource 1 while task(s) [2] still hold a read share of it, destroying the owner \
         before the child's join; the scope refuses the exit with that loan outstanding",
    );
}

/// The borrower holds read authority and nothing else. Each case is a different
/// way to keep, widen or discharge a loan the borrower does not own.
#[test]
fn the_borrower_can_only_read_its_loan() {
    for (body, expected) in [
        ("drop payload; return 0;", "drop requires an owned buffer"),
        (
            "let copied = payload; return 0;",
            "a read loan cannot be copied",
        ),
        ("return move payload;", "move return requires own<Buffer>"),
    ] {
        let source = READ_LOAN.replace(
            "let seen = qcore_buffer_read(payload);\n  return seen;",
            body,
        );
        rejects(&source, expected);
    }
    // The admitted body is exactly the read.
    checked(READ_LOAN).unwrap();
}

/// A loan cannot outlive the creation that opened it, so it is refused as a
/// result type and as an entry parameter before any path is explored.
#[test]
fn a_read_loan_is_not_a_result_type_and_not_an_entry_parameter() {
    rejects(
        &READ_LOAN.replace(
            "fn reader(payload: borrow<Buffer>) -> i32",
            "fn reader(payload: borrow<Buffer>) -> borrow<Buffer>",
        ),
        "task functions support i32, unit and own<Buffer> results",
    );
    rejects(
        &READ_LOAN.replace("fn main() -> i32", "fn main(stray: borrow<Buffer>) -> i32"),
        "entry must have no parameters",
    );
}

/// Card T04 admitted `share FULL` alone and refused every fraction. Card T05
/// counts shares instead, so this expectation was **re-pinned deliberately**:
/// a positive fraction of the full authority is now admitted, and only a share
/// that is not a fraction of it — zero, over-full, or a name with no declared
/// constant — is refused. The refusals below are the ones actually measured.
#[test]
fn a_read_share_must_be_a_positive_fraction_of_the_full_authority() {
    // Admitted: a single unit, a sum, and the whole authority spelled out.
    for share in [
        "share 1",
        "share (1 + 1)",
        "share 1024",
        "share (512 + 512)",
    ] {
        checked(&READ_LOAN.replace("share FULL", share))
            .unwrap_or_else(|error| panic!("`{share}` must be admitted, got:\n{error}"));
    }
    for (share, expected) in [
        (
            "share FULL + FULL",
            "at most FULL (1024); this capture requests 2048",
        ),
        ("share 0", "at most FULL (1024); this capture requests 0"),
        (
            "share 1025",
            "at most FULL (1024); this capture requests 1025",
        ),
        // A name that resolves is still not a share: this profile has no
        // declared share constants, so it is refused rather than evaluated.
        (
            "share buffer",
            "named read shares require a declared share constant",
        ),
    ] {
        rejects(&READ_LOAN.replace("share FULL", share), expected);
    }
    // A name that does not resolve is refused earlier still, by ordinary name
    // resolution, so no share expression can smuggle in an unbound identifier.
    rejects(
        &READ_LOAN.replace("share FULL", "share HALF"),
        "unknown name `HALF`",
    );
}

/// Capture mode and parameter type must agree in both directions, and a loan
/// carries no usage grade in this profile.
#[test]
fn capture_mode_and_parameter_type_must_agree() {
    // Since card CT47-COPY-CAPTURE these three are the transferability
    // judgment's own refusals, each naming its shape: a deliberate pin update
    // from the `Q500` executable-subset catch-all they carried before it.
    rejects(
        &READ_LOAN.replace("read buffer share FULL", "move buffer"),
        "`move` of `own<Buffer>` is an admitted transfer, but it delivers `own<Buffer>` \
         and the worker declares it receives `borrow<Buffer>`",
    );
    rejects(
        &READ_LOAN.replace("read buffer share FULL", "copy buffer"),
        "no transfer or share contract admits `copy` of `own<Buffer>` across a spawn boundary",
    );
    rejects(
        &READ_LOAN.replace("payload: borrow<Buffer>", "payload: own<Buffer> use one"),
        "`read ... share ...` of `own<Buffer>` is an admitted transfer, but it delivers \
         `borrow<Buffer>` and the worker declares it receives `own<Buffer>`",
    );
    rejects(
        &READ_LOAN.replace("payload: borrow<Buffer>", "payload: borrow<Buffer> use one"),
        "task parameters require i32, own<Buffer> use one, an ungraded borrow<Buffer>, or an \
         ungraded lend<Buffer>",
    );
    // Only an owned Buffer can be lent: a scalar place is refused on the place
    // type, before the capture mode is even considered.
    rejects(
        &READ_LOAN
            .replace(
                "let buffer = qcore_buffer_new(42)?;",
                "let buffer = qcore_buffer_new(42)?;\n  let count = 3;",
            )
            .replace("read buffer share FULL", "read count share FULL"),
        "capture type does not match worker parameter",
    );
}

/// Two independent reasons there is no transitive or nested borrowing here: a
/// loan is not a lendable place, and a worker cannot create tasks at all. The
/// first refusal is the one this program reaches.
#[test]
fn a_borrower_cannot_lend_its_loan_onward() {
    let source = "#import std.concurrency; concurrency checked;
fn inner(deeper: borrow<Buffer>) -> i32 {
  let v = qcore_buffer_read(deeper);
  return v;
}
fn reader(payload: borrow<Buffer>) -> i32 effects { spawn, throws<SpawnError>, join, may_block } {
  thread scope t {
    let nested = spawn t inner(read payload share FULL)?;
    let v = join move nested?;
    return v;
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s reader(read buffer share FULL) {
      Result::Ok { value: handle } => {
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
    // The loan itself is not a lendable place: the nested capture's place type
    // is `borrow<Buffer>` where an owned Buffer is required.
    rejects(source, "capture type does not match worker parameter");
    // And even a nested creation that lends nothing is refused, because a
    // spawned worker is a first-order leaf in this profile.
    let nested_copy = "#import std.concurrency; concurrency checked;
fn inner(nothing: i32) -> i32 {
  return nothing;
}
fn reader(payload: borrow<Buffer>) -> i32 effects { spawn, join, may_block } {
  let seen = qcore_buffer_read(payload);
  thread scope t {
    match spawn t inner(copy seen) {
      Result::Ok { value: nested } => {
        let v = join move nested?;
        return v;
      }
      Result::Err { error: refused } => {
        return 0;
      }
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s reader(read buffer share FULL) {
      Result::Ok { value: handle } => {
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
    rejects(nested_copy, "first-order leaf functions");
}

/// One creation may both move one Buffer and lend another. The moved one is
/// released by the borrower; the lent one comes back to the lender at join.
#[test]
fn one_creation_can_move_one_buffer_and_lend_another() {
    let source = "#import std.concurrency; concurrency checked;
fn reader(taken: own<Buffer> use one, lent: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(lent);
  drop taken;
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let kept = qcore_buffer_new(42)?;
  match qcore_buffer_new(7) {
    Result::Ok { value: given } => {
      thread scope s {
        match spawn s reader(move given, read kept share FULL) {
          Result::Ok { value: handle } => {
            let seen = join move handle?;
            drop kept;
            return seen;
          }
          Result::Err { error: failure } => {
            drop kept;
            drop given;
            return 0;
          }
        }
      }
    }
    Result::Err { error: shortfall } => {
      drop kept;
      return 0;
    }
  }
}";
    let module = checked(source).unwrap();
    let result = execute_task_module(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![true],
            allocation_outcomes: vec![true, true],
        },
    )
    .unwrap();
    let trace = render_task_observation(&result);
    assert!(trace.contains("QCORE_BUFFER_LOAN 1 1\n"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_RETURN 1 1\n"), "{trace}");
    // The borrower released the moved Buffer; the lender released its own.
    assert!(trace.contains("QCORE_BUFFER_DROP 1 2\n"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_DROP 0 1\n"), "{trace}");
    assert_eq!(result.root_outcome, TaskOutcome::I32(42));

    // The lender still cannot release the moved Buffer on the success path.
    rejects(
        &source.replace(
            "drop kept;\n            return seen;",
            "drop kept; drop given; return seen;",
        ),
        "not an initialized owned buffer",
    );
}
