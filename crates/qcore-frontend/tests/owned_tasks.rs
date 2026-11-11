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

use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{
    CheckedTaskModule, ScalarOperand, TaskExecutionScript, TaskInstructionKind, TaskTerminatorKind,
    check_task_module, execute_task_module, render_task_observation,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;

const ROUND_TRIP: &str = include_str!("../../../compiler/tests/fixtures/ct/CT01/case.qr");

fn checked(source: &str) -> Result<CheckedTaskModule, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let typed = elaborate_source(&parsed).map_err(|e| format!("{e:?}"))?;
    let module = lower_source_tasks(&parsed, &typed, "main").map_err(|e| format!("{e:?}"))?;
    check_task_module(&module).map_err(|e| format!("{e:?}"))
}

#[test]
fn actual_source_moves_return_the_original_generation_and_failed_create_restores_it() {
    let module = checked(ROUND_TRIP).unwrap();
    for succeeds in [true, false] {
        let result = execute_task_module(
            &module,
            &TaskExecutionScript {
                spawn_outcomes: vec![succeeds],
                allocation_outcomes: vec![true],
            },
        )
        .unwrap();
        assert_eq!(result.root_outcome, TaskOutcome::I32(42));
        let trace = render_task_observation(&result);
        assert_eq!(trace.matches("QCORE_BUFFER_ALLOC 0 1 42\n").count(), 1);
        assert_eq!(trace.matches("QCORE_BUFFER_DROP 0 1\n").count(), 1);
        assert_eq!(result.tasks.len(), if succeeds { 2 } else { 1 });
        if succeeds {
            assert_eq!(result.tasks[1].outcome, TaskOutcome::OwnedBuffer(1));
            assert!(result.tasks[1].joined);
        }
    }
}

#[test]
fn moved_binding_never_revives_when_join_returns_custody_under_a_new_name() {
    for operation in ["let forbidden = qcore_buffer_read(buffer);", "drop buffer;"] {
        for anchor in ["let returned = join move handle?;", "drop returned;"] {
            let source = ROUND_TRIP.replace(anchor, &format!("{anchor} {operation}"));
            let errors = checked(&source).unwrap_err();
            assert!(
                errors.contains("not an initialized owned buffer"),
                "{errors}"
            );
        }
    }
}

#[test]
fn ownership_is_accounted_on_creation_allocation_and_worker_error_exits() {
    for source in [
        ROUND_TRIP.replace("drop returned;", ""),
        ROUND_TRIP.replace("drop buffer;", ""),
        ROUND_TRIP.replace("return move buffer;", "drop buffer; return move buffer;"),
        ROUND_TRIP.replace("return move buffer;", "return 42;"),
        ROUND_TRIP.replace(
            "let buffer = qcore_buffer_new(42)?;",
            "let extra = qcore_buffer_new(7)?; let buffer = qcore_buffer_new(42)?;",
        ),
        ROUND_TRIP
            .replace(
                "-> own<Buffer> { return move buffer; }",
                "-> own<Buffer> effects { throws<Work> } { raise Work::Failed; }",
            )
            .replace(
                "concurrency checked;",
                "concurrency checked; error Work { Failed; }",
            ),
    ] {
        assert!(
            checked(&source).is_err(),
            "accepted ownership loss: {source}"
        );
    }
}

#[test]
fn implicit_copy_wrong_capture_types_and_unquantified_owned_inputs_reject() {
    for source in [
        ROUND_TRIP.replace("worker(move buffer)", "worker(copy buffer)"),
        ROUND_TRIP.replace("worker(move buffer)", "worker(move buffer, move buffer)"),
        ROUND_TRIP.replace("own<Buffer> use one", "own<Buffer>"),
        ROUND_TRIP.replace("own<Buffer> use one", "own<Buffer> use many"),
        ROUND_TRIP.replace("qcore_buffer_read(returned)", "qcore_buffer_read(42)"),
        ROUND_TRIP.replace("qcore_buffer_new(42)", "qcore_buffer_new(42, 7)"),
        ROUND_TRIP.replace("alloc, throws<AllocError>,", "throws<AllocError>,"),
        ROUND_TRIP.replace("alloc, throws<AllocError>,", "alloc,"),
        ROUND_TRIP.replace("let buffer =", "let buffer: i32 ="),
        ROUND_TRIP.replace("drop returned;", "let alias = returned; drop returned;"),
    ] {
        assert!(
            checked(&source).is_err(),
            "accepted unsupported input: {source}"
        );
    }
}

#[test]
fn explicit_second_allocation_failure_releases_prior_owner() {
    let source = "#import std.concurrency; concurrency checked; fn main()->i32 effects {alloc,throws<AllocError>} {
        let first=qcore_buffer_new(7)?;
        match qcore_buffer_new(42) {
            Result::Ok {value:second} => {let value=qcore_buffer_read(second);drop second;drop first;return value;}
            Result::Err {error:e} => {let value=qcore_buffer_read(first);drop first;return value;}
        }
    }";
    let module = checked(source).unwrap();
    for (allocations, outcome) in [
        (vec![true, true], TaskOutcome::I32(42)),
        (vec![true, false], TaskOutcome::I32(7)),
        (vec![false], TaskOutcome::Error("AllocError".into())),
    ] {
        let result = execute_task_module(
            &module,
            &TaskExecutionScript {
                spawn_outcomes: vec![],
                allocation_outcomes: allocations,
            },
        )
        .unwrap();
        assert_eq!(result.root_outcome, outcome);
    }
    assert!(checked(&source.replace("drop first;return value;", "return value;")).is_err());
}

#[test]
fn two_owned_inputs_return_one_original_and_release_the_other_on_every_path() {
    let source = "#import std.concurrency; concurrency checked;
    fn worker(first:own<Buffer> use one,second:own<Buffer> use one)->own<Buffer> {drop first;return move second;}
    fn main()->i32 effects {alloc,throws<AllocError>,spawn,join,may_block} {
      let first=qcore_buffer_new(7)?;
      match qcore_buffer_new(42) {
       Result::Ok {value:second} => { thread scope s {
        match spawn s worker(move first,move second) {
         Result::Ok {value:h} => {let returned=join move h?;let value=qcore_buffer_read(returned);drop returned;return value;}
         Result::Err {error:e} => {drop first;drop second;return 9;}
        }
       }}
       Result::Err {error:e} => {drop first;return 8;}
      }
    }";
    let module = checked(source).unwrap();
    let result = execute_task_module(&module, &TaskExecutionScript::default()).unwrap();
    assert_eq!(result.tasks[1].outcome, TaskOutcome::OwnedBuffer(2));
    assert_eq!(result.root_outcome, TaskOutcome::I32(42));
    assert!(render_task_observation(&result).contains("QCORE_BUFFER_DROP 1 1\n"));
    assert!(
        checked(&source.replace(
            "worker(move first,move second)",
            "worker(move first,move first)"
        ))
        .unwrap_err()
        .contains("duplicate owned capture")
    );
}

#[test]
fn raw_ir_cannot_forge_scalar_access_to_owned_identity_or_skip_its_drop() {
    let checked_module = checked(ROUND_TRIP).unwrap();
    let mut module = checked_module.module().clone();
    let worker = module
        .functions
        .values_mut()
        .find(|f| f.name == "worker")
        .unwrap();
    let owned_slot = worker.parameters[0].0;
    worker
        .blocks
        .get_mut(&worker.entry)
        .unwrap()
        .terminator
        .kind = TaskTerminatorKind::Return {
        value: ScalarOperand::Slot(owned_slot),
    };
    assert!(check_task_module(&module).is_err());
    let mut module = checked_module.module().clone();
    for block in module
        .functions
        .get_mut(&module.entry)
        .unwrap()
        .blocks
        .values_mut()
    {
        block
            .instructions
            .retain(|i| !matches!(i.kind, TaskInstructionKind::DropBuffer { .. }));
    }
    assert!(check_task_module(&module).is_err());
}

#[test]
fn leaf_allocation_is_checked_while_owned_root_results_and_uncalled_bad_bodies_reject() {
    assert!(checked("#import std.concurrency; concurrency checked; fn worker()->i32 effects {alloc,throws<AllocError>} {let b=qcore_buffer_new(1)?;drop b;return 1;} fn main()->i32 {return 0;}").is_ok());
    for source in [
        "#import std.concurrency; concurrency checked; fn main()->own<Buffer> effects {alloc,throws<AllocError>} {let b=qcore_buffer_new(1)?;return move b;}",
        "#import std.concurrency; concurrency checked; fn worker()->i32 effects {alloc,throws<AllocError>} {let b=qcore_buffer_new(1)?;return 1;} fn main()->i32 {return 0;}",
        "#import std.concurrency; concurrency checked; fn worker(b:own<Buffer> use one)->i32 {return 1;} fn main()->i32 {let b=qcore_buffer_new(1)?;drop b;return 0;}",
    ] {
        assert!(checked(source).is_err(), "{source}");
    }
}
