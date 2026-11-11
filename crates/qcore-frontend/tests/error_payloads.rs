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

//! Source-level acceptance for the bounded typed error payload (card T06).
//!
//! Every case goes through the real pipeline — parse, elaborate, lower, then
//! `check_task_module` — so a "checks" assertion is an all-paths result of the
//! same checker the driver publishes, and a rejection is that checker's own
//! refusal rather than a lowering convenience. Each expected message is the one
//! actually measured, so a refusal that moves to a different gate becomes a
//! visible change instead of a silently satisfied substring.
//!
//! The admitted profile is exactly one owned Buffer inside one explicitly
//! tagged error, travelling out of one child through its completion capsule and
//! its creator's join. Recursive owned graphs, several payloads, scalar or
//! borrowed payloads, nested workers and payload-carrying rethrow of a bound
//! error are all outside it, and each has a refusal here.
use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{
    CheckedTaskModule, ErrorOperand, TaskExecutionScript, TaskInstructionKind, TaskModule,
    TaskTerminatorKind, check_task_module, execute_task_module,
    execute_task_module_with_worker_allocations, render_task_observation,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;
use std::collections::BTreeMap;

/// The canonical program. The child owns the parent's Buffer, allocates two
/// locals, and on either allocation failure raises a tagged error carrying the
/// parent's Buffer back while releasing whatever it did acquire. The parent
/// takes the payload out of the bound error, reads it and releases it.
const PAYLOAD: &str = "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: first } => {
      match qcore_buffer_new(9) {
        Result::Ok { value: second } => {
          let seen = qcore_buffer_read(payload);
          drop payload;
          drop first;
          drop second;
          return seen;
        }
        Result::Err { error: exhausted } => {
          drop first;
          raise Recovery::Failed(payload);
        }
      }
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        match join move handle {
          Result::Ok { value: seen } => {
            return seen;
          }
          Result::Err { error: failure } => {
            let recovered = qcore_error_payload(failure);
            let value = qcore_buffer_read(recovered);
            drop recovered;
            return value;
          }
        }
      }
      Result::Err { error: refused } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

fn lower(source: &str) -> Result<TaskModule, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let typed = elaborate_source(&parsed).map_err(|e| format!("{e:?}"))?;
    lower_source_tasks(&parsed, &typed, "main").map_err(|e| format!("{e:?}"))
}

/// A fixture this test knows the source surface admits, lowered but not yet
/// checked, so a test can mutate the IR before the checker sees it.
fn lowered(source: &str) -> TaskModule {
    lower(source).expect("the fixture lowers")
}

fn checked(source: &str) -> Result<CheckedTaskModule, String> {
    check_task_module(&lower(source)?).map_err(|e| format!("{e:?}"))
}

/// The refusal a hand-built module measures. Some core gates cannot be reached
/// from this source surface at all — the surface refuses earlier and for a
/// different reason — so they are exercised on IR the lowering produced and a
/// test then mutated, exactly as the existing task fixtures do.
#[track_caller]
fn refuses_module(module: &TaskModule, expected: &str) {
    let errors = format!(
        "{:?}",
        check_task_module(module).expect_err("the checker must refuse this module")
    );
    assert!(
        errors.contains(expected),
        "expected a refusal containing {expected:?}, got:\n{errors}"
    );
}

#[track_caller]
fn rejects(source: &str, expected: &str) {
    let errors = checked(source).expect_err("the profile must refuse this program");
    assert!(
        errors.contains(expected),
        "expected a refusal containing {expected:?}, got:\n{errors}"
    );
}

/// `worker_allocations` keys the injected decisions by the runtime child
/// identity, so the reference run and the native run choose the same failure.
fn run(module: &CheckedTaskModule, spawn: bool, worker: &[bool]) -> String {
    let observation = execute_task_module_with_worker_allocations(
        module,
        &TaskExecutionScript {
            spawn_outcomes: vec![spawn],
            allocation_outcomes: vec![true],
        },
        &if worker.is_empty() {
            BTreeMap::new()
        } else {
            BTreeMap::from([(1u64, worker.to_vec())])
        },
    )
    .unwrap();
    render_task_observation(&observation)
}

/// T06 acceptance 1. The payload leaves the child once, inside the completion
/// capsule, and arrives at the parent once, at the join. Before the join the
/// parent cannot touch it; after the join it holds it and releases it.
#[test]
fn the_join_transfers_the_tagged_payload_exactly_once() {
    let module = checked(PAYLOAD).unwrap();
    let worker = module
        .module()
        .functions
        .values()
        .find(|function| function.name == "worker")
        .expect("the worker is lowered");
    assert_eq!(
        worker.error_payloads.iter().collect::<Vec<_>>(),
        ["Recovery::Failed"],
        "the payload row comes from the declaration, not from a use site"
    );
    // The entry declares no payload of its own; its row is the two builtin
    // failures, which never carry one.
    let entry = &module.module().functions[&module.module().entry];
    assert!(entry.error_payloads.is_empty());

    let trace = run(&module, true, &[true, false]);
    let line = |needle: &str| {
        trace
            .lines()
            .position(|entry| entry == needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in:\n{trace}"))
    };
    // One completion, tagged, naming the parent's own generation 1.
    assert_eq!(
        trace
            .matches("QCORE_TASK_RESULT 1 ERROR_BUFFER Recovery::Failed 1\n")
            .count(),
        1,
        "{trace}"
    );
    // The parent's read and release of that generation happen only after the
    // join: a completed-but-unjoined capsule is nobody's to touch.
    assert!(line("QCORE_TASK_JOIN 1") < line("QCORE_BUFFER_READ 0 1 42"));
    assert!(line("QCORE_BUFFER_READ 0 1 42") < line("QCORE_BUFFER_DROP 0 1"));
    assert_eq!(
        trace.matches("QCORE_BUFFER_DROP 0 1\n").count(),
        1,
        "{trace}"
    );
    // Exactly one allocation of generation 1 and exactly one release of it,
    // anywhere in the program, by anybody.
    assert_eq!(trace.matches(" 1 42\n").count(), 2, "{trace}");
    assert_eq!(
        trace.lines().filter(|l| l.ends_with("_DROP 0 1")).count(),
        1,
        "{trace}"
    );
    assert!(!trace.contains("QCORE_BUFFER_DROP 1 1"), "{trace}");
}

/// T06 acceptance 2. A child that acquired locals before failing releases every
/// one of them, and carries out only the identity the payload names.
#[test]
fn partial_worker_acquisition_releases_the_locals_it_does_not_carry() {
    let module = checked(PAYLOAD).unwrap();

    // Second allocation fails: the first local is released inside the child,
    // and only the parent's original generation leaves in the payload.
    let partial = run(&module, true, &[true, false]);
    assert!(
        partial.contains("QCORE_BUFFER_ALLOC 1 4294967297 7\n"),
        "{partial}"
    );
    assert!(
        partial.contains("QCORE_BUFFER_DROP 1 4294967297\n"),
        "{partial}"
    );
    assert!(
        partial.contains("QCORE_BUFFER_ALLOC_FAILED 1\n"),
        "{partial}"
    );
    assert!(
        partial.contains("QCORE_TASK_RESULT 1 ERROR_BUFFER Recovery::Failed 1\n"),
        "{partial}"
    );

    // First allocation fails: nothing local was acquired, and the same payload
    // still leaves. The two failures are distinct observed traces.
    let immediate = run(&module, true, &[false]);
    assert!(!immediate.contains("QCORE_BUFFER_ALLOC 1"), "{immediate}");
    assert!(
        immediate.contains("QCORE_TASK_RESULT 1 ERROR_BUFFER Recovery::Failed 1\n"),
        "{immediate}"
    );
    assert_ne!(partial, immediate);

    // The success path releases both locals and the payload inside the child,
    // so nothing at all crosses the join.
    let success = run(&module, true, &[]);
    assert!(
        success.contains("QCORE_TASK_RESULT 1 I32 42\n"),
        "{success}"
    );
    assert!(success.contains("QCORE_BUFFER_DROP 1 1\n"), "{success}");
    assert!(!success.contains("ERROR_BUFFER"), "{success}");

    // The control: a child that forgets its acquired local on the failing path
    // is refused, and the refusal is about the custody it did not account for.
    rejects(
        &PAYLOAD.replace(
            "          drop first;\n          raise Recovery::Failed(payload);",
            "          raise Recovery::Failed(payload);",
        ),
        "return/error discards a handle, unconsumed Result or owned buffer",
    );
}

/// A failed creation transfers nothing: the parent keeps the Buffer it tried to
/// move, and there is no child, no completion and no payload.
#[test]
fn a_failed_creation_leaves_the_payload_with_the_parent() {
    let module = checked(PAYLOAD).unwrap();
    let trace = run(&module, false, &[]);
    assert!(trace.contains("QCORE_TASK_CREATE_FAILED 1\n"), "{trace}");
    assert!(!trace.contains("ERROR_BUFFER"), "{trace}");
    assert!(trace.contains("QCORE_BUFFER_DROP 0 1\n"), "{trace}");
    assert_eq!(
        execute_task_module(
            &module,
            &TaskExecutionScript {
                spawn_outcomes: vec![false],
                allocation_outcomes: vec![true],
            },
        )
        .unwrap()
        .root_outcome,
        TaskOutcome::I32(0)
    );
}

/// T06 acceptance 1, the other half: a path that neither takes nor delivers the
/// payload is refused. The bound error is not a scalar tag that can be dropped.
#[test]
fn an_error_handler_cannot_discard_an_undelivered_payload() {
    rejects(
        &PAYLOAD.replace(
            "            let recovered = qcore_error_payload(failure);\n            let value = qcore_buffer_read(recovered);\n            drop recovered;\n            return value;",
            "            return 0;",
        ),
        "return/error discards an undelivered error payload",
    );
}

/// A taken payload is an ordinary owned Buffer and keeps the ordinary duty:
/// forgetting it is a leak, on the same path.
#[test]
fn a_taken_payload_is_still_accounted_for_at_every_exit() {
    rejects(
        &PAYLOAD.replace(
            "            drop recovered;\n            return value;",
            "            return value;",
        ),
        "return/error discards a handle, unconsumed Result or owned buffer",
    );
}

/// T06 acceptance 3. The payload cannot be taken twice, so no path can name one
/// identity as two owned values.
#[test]
fn a_payload_cannot_be_taken_twice() {
    rejects(
        &PAYLOAD.replace(
            "            let value = qcore_buffer_read(recovered);",
            "            let again = qcore_error_payload(failure);\n            let value = qcore_buffer_read(recovered);",
        ),
        "qcore_error_payload requires a bound error that still holds an owned payload",
    );
}

/// T06 acceptance 3. A second join of the same handle is refused, so a
/// completion capsule cannot be collected — and its payload transferred —
/// twice.
#[test]
fn a_duplicate_join_cannot_transfer_the_payload_twice() {
    rejects(
        &PAYLOAD.replace(
            "            drop recovered;\n            return value;",
            "            drop recovered;\n            match join move handle {\n              Result::Ok { value: again } => {\n                return again;\n              }\n              Result::Err { error: second } => {\n                let twice = qcore_error_payload(second);\n                drop twice;\n                return value;\n              }\n            }",
        ),
        "join requires a live handle slot",
    );
}

/// Re-raising the bound error would carry the payload onward through a tag
/// alone. That is refused at the source gate, before any custody question.
#[test]
fn a_bound_error_cannot_be_re_raised_while_it_holds_a_payload() {
    // Unreachable from the source surface: a function that catches a payload
    // error and re-raises would have to declare that tag beside `AllocError` or
    // `SpawnError`, and the uniform-row rule below refuses such a declaration
    // first. The core gate is therefore checked on mutated IR, so that widening
    // the source surface later cannot silently admit a payload-carrying
    // rethrow.
    let mut module = lowered(PAYLOAD);
    let entry = module.entry;
    let main = module
        .functions
        .get_mut(&entry)
        .expect("the entry is lowered");
    main.errors.insert("Recovery::Failed".into());
    // The parent's error arm takes the payload; replace that take with a
    // rethrow of the still-loaded binding.
    let arm = main
        .blocks
        .values_mut()
        .find(|block| {
            block
                .instructions
                .iter()
                .any(|i| matches!(i.kind, TaskInstructionKind::TakeErrorPayload { .. }))
        })
        .expect("the parent takes the payload");
    let TaskInstructionKind::TakeErrorPayload { error_slot, .. } = arm
        .instructions
        .iter()
        .find_map(|i| match &i.kind {
            kind @ TaskInstructionKind::TakeErrorPayload { .. } => Some(kind.clone()),
            _ => None,
        })
        .expect("the take is present")
    else {
        unreachable!()
    };
    // Keep only the scope bookkeeping, so the arm raises the bound error
    // directly instead of reading and releasing a payload it never took.
    arm.instructions
        .retain(|i| matches!(i.kind, TaskInstructionKind::ExitScope { .. }));
    arm.terminator.kind = TaskTerminatorKind::Raise {
        error: ErrorOperand::Slot(error_slot),
        payload: None,
    };
    refuses_module(
        &module,
        "a bound error must deliver its payload before it is re-raised",
    );
}

/// `?` has nowhere to name the payload, so it cannot propagate one. This is the
/// refusal, not a silent forward.
#[test]
fn propagation_cannot_carry_an_owned_payload() {
    // The source gate, on the only shape that reaches it: a worker whose whole
    // declared row carries payloads is joined with `?`. The row is uniform, so
    // the declaration itself is admitted and `?` is what refuses.
    rejects(
        "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: scratch } => {
      let seen = qcore_buffer_read(payload);
      drop payload;
      drop scratch;
      return seen;
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<Recovery>, spawn, join, may_block } {
  match qcore_buffer_new(42) {
    Result::Ok { value: buffer } => {
      thread scope s {
        match spawn s worker(move buffer) {
          Result::Ok { value: handle } => {
            let seen = join move handle?;
            return seen;
          }
          Result::Err { error: refused } => {
            drop buffer;
            return 0;
          }
        }
      }
    }
    Result::Err { error: failed } => {
      return 0;
    }
  }
}",
        "an error that carries an owned payload requires an explicit Result match, not `?`",
    );
}

/// T06 acceptance 3. A scoped read loan is not a value and cannot escape inside
/// a payload; the refusal names the required type rather than silently lending.
#[test]
fn a_scoped_borrow_cannot_escape_inside_an_error_payload() {
    rejects(
        "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: borrow<Buffer>) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: first } => {
      drop first;
      let seen = qcore_buffer_read(payload);
      return seen;
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s worker(read buffer share FULL) {
      Result::Ok { value: handle } => {
        match join move handle {
          Result::Ok { value: seen } => {
            drop buffer;
            return seen;
          }
          Result::Err { error: failure } => {
            let recovered = qcore_error_payload(failure);
            drop recovered;
            drop buffer;
            return 0;
          }
        }
      }
      Result::Err { error: refused } => {
        drop buffer;
        return 0;
      }
    }
  }
}",
        "an error payload requires own<Buffer>",
    );
}

/// A payload the raise no longer owns cannot be handed out: the double transfer
/// is refused where it is written.
#[test]
fn a_released_buffer_cannot_become_a_payload() {
    rejects(
        &PAYLOAD.replace(
            "          drop first;\n          raise Recovery::Failed(payload);",
            "          drop first;\n          drop payload;\n          raise Recovery::Failed(payload);",
        ),
        "is not an initialized owned buffer",
    );
}

/// The entry has no external custodian, so it cannot declare a payload row —
/// exactly as it cannot declare an owned result.
#[test]
fn the_task_entry_cannot_declare_an_error_payload() {
    // `main` declares only `Recovery`, so its row is uniform and the
    // declaration gate admits it; the refusal is the entry rule itself.
    rejects(
        "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: scratch } => {
      let seen = qcore_buffer_read(payload);
      drop payload;
      drop scratch;
      return seen;
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<Recovery>, spawn, join, may_block } {
  match qcore_buffer_new(42) {
    Result::Ok { value: buffer } => {
      raise Recovery::Failed(buffer);
    }
    Result::Err { error: failed } => {
      return 0;
    }
  }
}",
        "owned task entry error payloads have no external custodian in this profile",
    );
}

/// The declaration decides, in both directions: a payload variant must be
/// raised with one and a payload-free variant must not.
#[test]
fn a_raise_must_agree_with_the_declared_variant() {
    rejects(
        &PAYLOAD.replace(
            "      raise Recovery::Failed(payload);\n    }\n  }\n}",
            "      drop payload;\n      raise Recovery::Failed;\n    }\n  }\n}",
        ),
        "raised payload disagrees with the declared error variant",
    );
    rejects(
        &PAYLOAD.replace(
            "error Recovery { Failed(own<Buffer>); }",
            "error Recovery { Failed; }",
        ),
        "raised payload disagrees with the declared error variant",
    );
}

/// The payload row is uniform across a declared error row, so a joined error
/// always holds exactly one Buffer and the parent's take is total. A mixed row
/// is a represented refusal, not a silently widened profile.
#[test]
fn a_declared_error_row_cannot_mix_payload_and_payload_free_variants() {
    rejects(
        &PAYLOAD.replace(
            "error Recovery { Failed(own<Buffer>); }",
            "error Recovery { Failed(own<Buffer>); Exhausted; }",
        ),
        "a declared error row must carry an owned payload on every variant or on none",
    );
}

/// Only one owned Buffer is admitted as a payload. Every other shape of
/// declaration is refused at the declaration, before any body is read.
#[test]
fn only_one_owned_buffer_is_an_admitted_payload_shape() {
    for declaration in [
        "error Recovery { Failed(i32); }",
        "error Recovery { Failed(borrow<Buffer>); }",
        "error Recovery { Failed(); }",
        "error Recovery { Failed(own<Buffer>, own<Buffer>); }",
        "error Recovery { Failed(own<Buffer>) }",
        "error Recovery { Failed(own<Buffer> use one); }",
        "error Recovery { Failed(own<Buffer>); Failed(own<Buffer>); }",
    ] {
        rejects(
            &PAYLOAD.replace("error Recovery { Failed(own<Buffer>); }", declaration),
            "payload-free or carry exactly one own<Buffer>",
        );
    }
}

/// `qcore_error_payload` is an intrinsic, not a name: it cannot be shadowed by
/// a declaration, and it cannot be applied to something that is not a bound
/// typed error.
#[test]
fn the_payload_intrinsic_is_reserved_and_typed() {
    rejects(
        &PAYLOAD.replace(
            "            let recovered = qcore_error_payload(failure);",
            "            let recovered = qcore_error_payload(handle);",
        ),
        "qcore_error_payload requires a bound error that still holds an owned payload",
    );
    // The intrinsic is recognized by the head word of the initializer and is a
    // reserved source word, so it resolves without being declared anywhere and
    // a same-named local cannot capture it: the shadowing program checks and
    // executes exactly like the original.
    let shadowed = checked(&PAYLOAD.replace(
        "            let recovered = qcore_error_payload(failure);",
        "            let qcore_error_payload = 1;\n            let recovered = qcore_error_payload(failure);",
    ))
    .expect("a same-named local does not capture the intrinsic");
    assert_eq!(
        run(&shadowed, true, &[false]),
        run(&checked(PAYLOAD).unwrap(), true, &[false])
    );
}

/// A worker may hand the same Buffer back as a normal result on one path and
/// inside a tagged error on another. Both are exactly one transfer, and the
/// checker keeps them apart per path.
#[test]
fn one_buffer_can_be_a_result_on_one_path_and_a_payload_on_another() {
    const EITHER: &str = "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> own<Buffer> effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: scratch } => {
      drop scratch;
      return move payload;
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        match join move handle {
          Result::Ok { value: returned } => {
            let seen = qcore_buffer_read(returned);
            drop returned;
            return seen;
          }
          Result::Err { error: failure } => {
            let recovered = qcore_error_payload(failure);
            let seen = qcore_buffer_read(recovered);
            drop recovered;
            return seen;
          }
        }
      }
      Result::Err { error: refused } => {
        drop buffer;
        return 0;
      }
    }
  }
}";
    let module = checked(EITHER).unwrap();
    let normal = run(&module, true, &[]);
    assert!(
        normal.contains("QCORE_TASK_RESULT 1 BUFFER 1\n"),
        "{normal}"
    );
    let raised = run(&module, true, &[false]);
    assert!(
        raised.contains("QCORE_TASK_RESULT 1 ERROR_BUFFER Recovery::Failed 1\n"),
        "{raised}"
    );
    // Either way the identity is the parent's original generation, it is read
    // once after the join and released exactly once.
    for trace in [&normal, &raised] {
        assert_eq!(trace.matches("QCORE_BUFFER_READ 0 1 42\n").count(), 1);
        assert_eq!(trace.matches("QCORE_BUFFER_DROP 0 1\n").count(), 1);
    }
}
