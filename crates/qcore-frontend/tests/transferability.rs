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

//! Cards `CT46-SENDSYNC` and `CT47-COPY-CAPTURE`: source-level evidence for
//! the transferability judgment.
//!
//! Three things are checked here.
//!
//! 1. **The admitted set did not move.** Every accepting control below goes
//!    through the real pipeline; nothing the enumeration admitted is
//!    withdrawn and nothing it refused is admitted.
//!
//! 2. **The refusal is the judgment's own** (card `CT47-COPY-CAPTURE`). A
//!    capture both the judgment and the enumeration refuse is now attributed
//!    at the crossing construct — `Q600`, the Concurrency pass, a condition
//!    widening stated in spec §L21 — with a message naming the missing
//!    transfer/share contract and the contracts the kind does carry. Before
//!    that card the same shapes fell through to the enumeration's
//!    executable-subset catch-all at `Q500` (`CEmission`), a statement about
//!    the backend rather than about the boundary; the wave-26 CT lane
//!    measured exactly that attribution gap
//!    (`docs/reports/ct-wire-lane/README.md` §3.8). Every expectation below
//!    is a deliberate pin update recorded in
//!    `docs/reports/ct47-copy-capture/README.md`; the refused set is
//!    unchanged, and the grammar gate (`Q000`) and the catch-all keep their
//!    own text for the shapes that still reach them.
//!
//! 3. **The judgment is not vacuous.** The agreement test at the bottom
//!    quantifies over every reachable (mode, place kind, parameter kind)
//!    triple and shows the derived judgment reproduces the enumeration
//!    exactly — which is why the disagreement branch never fires, and is the
//!    evidence that the composition is behavior-preserving rather than the
//!    assertion that it is.
use qcore_core::task_execution::check_task_module;
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_mutex::{has_source_mutex, lower_source_mutex};
use qcore_frontend::source_tasks::lower_source_tasks;

/// Run one module all the way to a checked task result and return the first
/// diagnostic's `(code, message)`, or `None` if it checks.
fn task_diagnostic(source: &str) -> Option<(String, String)> {
    let parsed = parse_source_program(source).expect("parses");
    let typed = match elaborate_source(&parsed) {
        Ok(typed) => typed,
        Err(errors) => {
            let first = errors
                .first()
                .expect("elaboration rejection has diagnostics");
            return Some((first.code.to_string(), first.message.clone()));
        }
    };
    match lower_source_tasks(&parsed, &typed, "main") {
        Ok(module) => match check_task_module(&module) {
            Ok(_) => None,
            Err(errors) => {
                let first = errors.first().expect("checker rejection has diagnostics");
                Some(("Q600".to_string(), first.message.clone()))
            }
        },
        Err(errors) => {
            let first = errors.first().expect("lowering rejection has diagnostics");
            Some((first.code.to_string(), first.message.clone()))
        }
    }
}

/// The same, for a module the mutex route selects.
fn mutex_diagnostic(source: &str) -> Option<(String, String)> {
    let parsed = parse_source_program(source).expect("parses");
    assert!(
        has_source_mutex(&parsed),
        "this module must select the mutex route"
    );
    let typed = match elaborate_source(&parsed) {
        Ok(typed) => typed,
        Err(errors) => {
            let first = errors
                .first()
                .expect("elaboration rejection has diagnostics");
            return Some((first.code.to_string(), first.message.clone()));
        }
    };
    match lower_source_mutex(&parsed, &typed, "main") {
        Ok(_) => None,
        Err(errors) => {
            let first = errors.first().expect("lowering rejection has diagnostics");
            Some((first.code.to_string(), first.message.clone()))
        }
    }
}

fn assert_refused(source: &str, code: &str, message: &str) {
    let actual = task_diagnostic(source);
    assert_eq!(
        actual,
        Some((code.to_string(), message.to_string())),
        "measured diagnostic for this module"
    );
}

/// A worker taking one owned Buffer, and a `main` that allocates one, moves it
/// in and joins. The accepted control every mutation below starts from.
const OWNED: &str = "#import std.concurrency; concurrency checked;
fn worker(payload: own<Buffer> use one) -> own<Buffer> {
  return move payload;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        let returned = join move handle?;
        drop returned;
        return 0;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 1;
      }
    }
  }
}";

/// A worker taking one immutable i32 copy.
const COPIED: &str = "#import std.concurrency; concurrency checked;
fn worker(value: i32) -> i32 {
  return value;
}
fn main() -> i32 effects { spawn, throws<SpawnError>, join, may_block } {
  let count = 3;
  thread scope s {
    let handle = spawn s worker(copy count)?;
    let seen = join move handle?;
    return seen;
  }
}";

/// The two accepted controls really are accepted, so every refusal below is a
/// refusal of the mutation and not of the surrounding program.
#[test]
fn the_controls_check() {
    assert_eq!(task_diagnostic(OWNED), None);
    assert_eq!(task_diagnostic(COPIED), None);
}

/// Card CT47-COPY-CAPTURE: the messages the judgment refuses with, one per
/// refusal shape. Pinned verbatim so a drift in the wording is a deliberate
/// change, not an accident.
const COPY_OF_BUFFER: &str = "the transferability judgment refuses this capture: no transfer or \
     share contract admits `copy` of `own<Buffer>` across a spawn boundary; `own<Buffer>` \
     crosses only under `move` (custody transferred to the worker), `read PLACE share N` \
     (a counted read share), or `lend PLACE LOW..HIGH` (a disjoint range write lease)";
const LEND_TO_OWN: &str = "the transferability judgment refuses this capture: `read ... share \
     ...` of `own<Buffer>` is an admitted transfer, but it delivers `borrow<Buffer>` and the \
     worker declares it receives `own<Buffer>`";
const MOVE_TO_LOAN_PARAMETER: &str = "the transferability judgment refuses this capture: `move` \
     of `own<Buffer>` is an admitted transfer, but it delivers `own<Buffer>` and the worker \
     declares it receives `borrow<Buffer>`";
const WRITE_OF_BUFFER: &str = "the transferability judgment refuses this capture: a `write` \
     capture names no transfer or share contract — no kind crosses a spawn boundary under it; \
     `own<Buffer>` crosses only under `move` (custody transferred to the worker), `read PLACE \
     share N` (a counted read share), or `lend PLACE LOW..HIGH` (a disjoint range write lease)";
const MOVE_OF_I32: &str = "the transferability judgment refuses this capture: no transfer or \
     share contract admits `move` of `i32` across a spawn boundary; `i32` crosses only under \
     `copy` (an independent duplicate; no custody crosses)";
const LEND_OF_I32: &str = "the transferability judgment refuses this capture: no transfer or \
     share contract admits `read ... share ...` of `i32` across a spawn boundary; `i32` crosses \
     only under `copy` (an independent duplicate; no custody crosses)";
const WRITE_OF_I32: &str = "the transferability judgment refuses this capture: a `write` capture \
     names no transfer or share contract — no kind crosses a spawn boundary under it; `i32` \
     crosses only under `copy` (an independent duplicate; no custody crosses)";

/// The capture-mode refusals: an affine Buffer may not be duplicated, a scalar
/// may not be moved, and a lend may not be delivered as custody or a move as a
/// loan. Since card CT47-COPY-CAPTURE every one of these is the judgment's own
/// refusal at `Q600`, attributed to the capture and naming the missing
/// contract — a deliberate pin update from the `Q500` catch-all these rows
/// carried before it.
#[test]
fn a_mode_the_kind_does_not_admit_is_refused_by_the_judgment() {
    // Copying affine custody would duplicate an identity.
    assert_refused(
        &OWNED.replace("move buffer", "copy buffer"),
        "Q600",
        COPY_OF_BUFFER,
    );
    // Lending to an `own<Buffer>` parameter: the transfer is admitted, but a
    // lend delivers a loan, not custody.
    assert_refused(
        &OWNED.replace("move buffer", "read buffer share FULL"),
        "Q600",
        LEND_TO_OWN,
    );
    // A `write` capture. The keyword parses and has never had a lowering.
    assert_refused(
        &OWNED.replace("move buffer", "write buffer"),
        "Q600",
        WRITE_OF_BUFFER,
    );
    // Moving a scalar. There is no custody to move.
    assert_refused(
        &COPIED.replace("copy count", "move count"),
        "Q600",
        MOVE_OF_I32,
    );
    // Lending a scalar. A copy already gives the child an independent value,
    // so there is no authority to split.
    assert_refused(
        &COPIED.replace("copy count", "read count share FULL"),
        "Q600",
        LEND_OF_I32,
    );
    // Writing a scalar.
    assert_refused(
        &COPIED.replace("copy count", "write count"),
        "Q600",
        WRITE_OF_I32,
    );
}

/// Delivering a move to a `borrow<Buffer>` parameter. The judgment separates
/// this from the mode refusals — the transfer is fine, the delivered kind is
/// not — and since card CT47-COPY-CAPTURE its refusal says exactly that.
#[test]
fn a_move_delivered_to_a_loan_parameter_is_refused() {
    // The reader is a well-formed loan worker; only the capture mode is wrong.
    const MOVE_TO_LOAN: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s reader(move buffer) {
      Result::Ok { value: handle } => {
        let seen = join move handle?;
        drop buffer;
        return seen;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 1;
      }
    }
  }
}";
    // Replacing only the capture mode with `read ... share FULL` checks, so
    // the refusal below is the mode's and not the module's.
    assert_eq!(
        task_diagnostic(&MOVE_TO_LOAN.replace("move buffer)", "read buffer share FULL)")),
        None
    );
    assert_refused(MOVE_TO_LOAN, "Q600", MOVE_TO_LOAN_PARAMETER);
}

/// CT47's own boundary, the three shapes the wave-26 CT lane measured
/// (`docs/reports/ct-wire-lane/README.md` §3.8), re-measured after card
/// CT47-COPY-CAPTURE.
///
/// The middle shape is the one that moved: a `copy` capture of an owned
/// handle — the nearest expressible shape to CT47's raw-pointer bit-copy,
/// this language having no raw pointer type — is now refused **by the
/// transferability judgment**, attributed to the capture at the crossing
/// construct, with the missing transfer/share contract named. Before the
/// card it fell through to the backend's executable-subset catch-all
/// (`Q500`), which is the attribution the lane refused to map CT47 on. The
/// other two shapes are pinned unchanged: no declared mode still does not
/// parse (there is no automatic mode at a thread boundary), and the declared
/// read share is still admitted.
#[test]
fn ct47s_measured_shapes_a_copy_capture_is_refused_by_the_judgment() {
    const READER: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s reader(read buffer share FULL) {
      Result::Ok { value: handle } => {
        let seen = join move handle?;
        drop buffer;
        return seen;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 1;
      }
    }
  }
}";
    // Shape 3: the declared read share is admitted, exactly as measured.
    assert_eq!(task_diagnostic(READER), None);
    // Shape 1: a capture with no declared mode does not parse. The grammar
    // gate is untouched by this card and keeps its own text.
    let modeless = READER.replace("read buffer share FULL", "buffer");
    let parse_error = qcore_frontend::frontend::parse_source_program(&modeless)
        .expect_err("a modeless capture must not parse");
    assert_eq!(
        parse_error.message,
        "capture requires copy, move, read ... share, lend ... LOW..HIGH, or write"
    );
    // Shape 2: the bit-copy of an owned handle, refused by the judgment at
    // the boundary — no longer by the backend.
    assert_refused(
        &READER.replace("read buffer share FULL", "copy buffer"),
        "Q600",
        COPY_OF_BUFFER,
    );
}

/// Every binding that is not a value is refused at the boundary. A thread
/// scope, a task handle, a `Result` and a bound error are each `Never` in the
/// judgment, and each keeps the type check's own message.
#[test]
fn a_binding_that_is_not_a_value_never_crosses() {
    // A `thread scope` name captured as if it were an i32.
    assert_refused(
        &COPIED.replace("copy count", "copy s"),
        "Q020",
        "capture type does not match worker parameter",
    );
    // A task handle captured into a second creation.
    let handle_capture = "#import std.concurrency; concurrency checked;
fn worker(value: i32) -> i32 {
  return value;
}
fn main() -> i32 effects { spawn, throws<SpawnError>, join, may_block } {
  let count = 3;
  thread scope s {
    let first = spawn s worker(copy count)?;
    let second = spawn s worker(copy first)?;
    let a = join move first?;
    let b = join move second?;
    return a;
  }
}";
    assert_refused(
        handle_capture,
        "Q020",
        "capture type does not match worker parameter",
    );
    // A `Result` binding cannot be captured because it cannot be *bound*: an
    // allocation that is neither propagated with `?` nor matched is refused
    // before any capture is considered. So the judgment's `TaskResult` arm is
    // unreachable from source on this route, which is why it is classified
    // rather than omitted.
    assert_refused(
        "#import std.concurrency; concurrency checked;
fn worker(value: i32) -> i32 {
  return value;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, throws<SpawnError>, join, may_block } {
  let count = 3;
  let outcome = qcore_buffer_new(1);
  thread scope s {
    let handle = spawn s worker(copy outcome)?;
    let seen = join move handle?;
    return seen;
  }
}",
        "Q500",
        "buffer allocation requires ? or explicit Result match",
    );
    // A bound error value captured on the failure arm.
    let error_capture = "#import std.concurrency; concurrency checked;
fn worker(value: i32) -> i32 {
  return value;
}
fn main() -> i32 effects { spawn, throws<SpawnError>, join, may_block } {
  let count = 3;
  thread scope s {
    match spawn s worker(copy count) {
      Result::Ok { value: handle } => {
        let seen = join move handle?;
        return seen;
      }
      Result::Err { error: failure } => {
        let second = spawn s worker(copy failure)?;
        let seen = join move second?;
        return seen;
      }
    }
  }
}";
    assert_refused(
        error_capture,
        "Q020",
        "capture type does not match worker parameter",
    );
}

/// A compound capture list mixing kinds. The admitted capture is admitted and
/// the refused one is refused in the same creation, so the boundary decides
/// per capture rather than per creation.
#[test]
fn a_compound_capture_refuses_only_the_bad_component() {
    const MIXED: &str = "#import std.concurrency; concurrency checked;
fn worker(value: i32, payload: own<Buffer> use one) -> own<Buffer> {
  return move payload;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let count = 3;
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s worker(copy count, move buffer) {
      Result::Ok { value: handle } => {
        let returned = join move handle?;
        drop returned;
        return 0;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 1;
      }
    }
  }
}";
    // The mixed control checks: a copy and a move in one capture list.
    assert_eq!(task_diagnostic(MIXED), None);
    // Swapping only the second component's mode refuses that component, by
    // the judgment, naming that component's own kind.
    assert_refused(
        &MIXED.replace("copy count, move buffer", "copy count, copy buffer"),
        "Q600",
        COPY_OF_BUFFER,
    );
    // Swapping only the first refuses that one instead, at the same gate.
    assert_refused(
        &MIXED.replace("copy count, move buffer", "move count, move buffer"),
        "Q600",
        MOVE_OF_I32,
    );
    // Reversing the two components is a place/parameter type disagreement on
    // the first, which the type check reaches first.
    assert_refused(
        &MIXED.replace("copy count, move buffer", "copy buffer, move count"),
        "Q020",
        "capture type does not match worker parameter",
    );
}

/// Guards and facts, on the task route. The measured result is stronger than
/// "refused at the capture boundary": a guard can never be *named* on this
/// route at all.
///
/// A guard binder comes only from a `with lock` body, and the mutex it locks
/// comes only from `let m = mutex guards r;` — which selects the mutex route
/// for the whole module. So on the task route the mutex name is unbound and
/// the refusal is ordinary name resolution, and a `footprint` declaration is
/// refused by the statement grammar. Neither ever reaches a capture. Both
/// diagnostics are exactly the ones measured before this card.
#[test]
fn a_guard_can_never_be_named_on_the_task_route() {
    assert_refused(
        &COPIED.replace(
            "let count = 3;",
            "let count = 3;\n  with lock m as g {\n    read r;\n  }",
        ),
        "Q013",
        "unknown name `m`",
    );
    assert_refused(
        &COPIED.replace("let count = 3;", "let count = 3;\n  let r = footprint 0 8;"),
        "Q500",
        "task scalar expressions require an i32 literal or immutable scalar binding",
    );
}

/// CT46's own shape, measured. A module that declares a mutex and also spawns
/// a worker with a capture selects the mutex route, and that route refuses the
/// task profile's `spawn`/`join` expressions outright: its own child-task form
/// carries no captures at all.
///
/// This is the boundary this card closes at. The judgment classifies
/// `own<Buffer>` as sendable-but-not-freely-shareable and names
/// `ShareProtocol::CheckedMutex` as the discipline that would share it, and
/// there is no source form that asks for it — so the refusal below is not a
/// transferability decision at all.
#[test]
fn a_mutex_protected_payload_crossing_a_task_boundary_has_no_source_form() {
    let shared_wrapper = "#import std.concurrency; concurrency checked;
fn worker(payload: own<Buffer> use one) -> own<Buffer> {
  return move payload;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let r = footprint 0 8;
  let m = mutex guards r;
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        let returned = join move handle?;
        drop returned;
        return 0;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 1;
      }
    }
  }
}";
    assert_eq!(
        mutex_diagnostic(shared_wrapper),
        Some((
            "Q600".to_string(),
            "the scoped task profile's `spawn`/`join` expressions are outside \
             source.mutex.guard.v1; the profile refuses completion-capsule composition. \
             Its own child-task form is a nested `thread scope NAME { … }`"
                .to_string()
        )),
        "the refusal is the route's, and it names its own captureless child form"
    );
}

/// **The identity evidence.** The derived judgment reproduces the existing
/// enumeration on every reachable triple, so the composition's disagreement
/// branch is unreachable and the judgment-first refusal (card
/// CT47-COPY-CAPTURE) fires for exactly the set the catch-all used to.
///
/// The enumeration is written out here independently of the production copy —
/// if the production one changes, this test disagrees rather than following
/// it.
#[test]
fn the_derived_judgment_reproduces_the_enumeration_exactly() {
    use qcore_core::transfer::{
        AdmittedTransfer, ShareProtocol, TransferKind, TransferMode, admits_capture,
    };

    // The elaborated parameter types a worker can declare, plus `Unit`, which
    // the parameter grammar never produces but the judgment still answers for.
    let parameters = [
        TransferKind::I32,
        TransferKind::OwnedBuffer,
        TransferKind::ReadLoan,
        TransferKind::Unit,
    ];
    let modes = [
        TransferMode::Copy,
        TransferMode::Move,
        TransferMode::Lend,
        TransferMode::Write,
    ];

    let mut agreed = 0usize;
    let mut admitted = Vec::new();
    for parameter in parameters {
        // `spawn_inputs` derives the place type from the parameter type: a
        // lend is the one capture whose two sides deliberately differ.
        let place = if parameter == TransferKind::ReadLoan {
            TransferKind::OwnedBuffer
        } else {
            parameter
        };
        for mode in modes {
            // The production enumeration, restated independently.
            let enumerated: Option<AdmittedTransfer> = match (mode, parameter) {
                (TransferMode::Copy, TransferKind::I32) => Some(AdmittedTransfer::Duplicate),
                (TransferMode::Move, TransferKind::OwnedBuffer) => {
                    Some(AdmittedTransfer::MoveCustody)
                }
                (TransferMode::Lend, TransferKind::ReadLoan) => Some(AdmittedTransfer::LendShare(
                    ShareProtocol::CountedReadShares,
                )),
                _ => None,
            };
            let derived = admits_capture(place, mode, parameter).ok();
            assert_eq!(
                derived, enumerated,
                "derived judgment and enumeration disagree at \
                 place={place:?} mode={mode:?} parameter={parameter:?}"
            );
            agreed += 1;
            if enumerated.is_some() {
                admitted.push((place, mode, parameter));
            }
        }
    }
    assert_eq!(agreed, 16, "every reachable (mode, parameter) pair");
    assert_eq!(
        admitted.len(),
        3,
        "and the agreement is not vacuous: three of the sixteen admit"
    );
}
