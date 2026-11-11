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
    CheckedTaskModule, TaskExecutionScript, check_task_module, execute_task_module,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::lower_source_tasks;

fn checked(source: &str) -> Result<CheckedTaskModule, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let typed = elaborate_source(&parsed).map_err(|e| format!("{e:?}"))?;
    let module = lower_source_tasks(&parsed, &typed, "main").map_err(|e| format!("{e:?}"))?;
    check_task_module(&module).map_err(|e| format!("{e:?}"))
}

const RECOVERY: &str = "#import std.concurrency; concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
 let first_value = 42; let second_value = 7;
 thread scope s {
  let first = spawn s worker(copy first_value)?;
  match spawn s worker(copy second_value) {
   Result::Ok { value: second } => { let second_result = join move second?; let first_result = join move first?; return second_result; }
   Result::Err { error: e } => { let first_result = join move first?; return first_result; }
  }
 }
}";

#[test]
fn failed_second_create_can_join_prior_sibling_and_recover() {
    let module = checked(RECOVERY).unwrap();
    for (choices, outcome, tasks) in [
        (vec![true, true], TaskOutcome::I32(7), 3),
        (vec![true, false], TaskOutcome::I32(42), 2),
        (vec![false], TaskOutcome::Error("SpawnError".into()), 1),
    ] {
        let observation = execute_task_module(
            &module,
            &TaskExecutionScript {
                allocation_outcomes: vec![],
                spawn_outcomes: choices,
            },
        )
        .unwrap();
        assert_eq!(observation.root_outcome, outcome);
        assert_eq!(observation.tasks.len(), tasks);
        assert!(
            observation
                .tasks
                .iter()
                .filter(|t| t.parent.is_some())
                .all(|t| t.joined)
        );
    }
    let raised = RECOVERY.replace("return first_result;", "raise e;");
    let module = checked(&raised).unwrap();
    let observation = execute_task_module(
        &module,
        &TaskExecutionScript {
            allocation_outcomes: vec![],
            spawn_outcomes: vec![true, false],
        },
    )
    .unwrap();
    assert_eq!(
        observation.root_outcome,
        TaskOutcome::Error("SpawnError".into())
    );
    assert!(observation.tasks[1].joined);
}

#[test]
fn direct_join_and_existing_result_matches_preserve_worker_error_tags() {
    for scrutinee in ["join move h", "r"] {
        let binding = if scrutinee == "r" {
            "let r = join move h;"
        } else {
            ""
        };
        let source = format!(
            "#import std.concurrency; concurrency checked; error Work {{ First; Second; }} fn worker(x:i32)->i32 effects {{throws<Work>}} {{raise Work::Second;}} fn main()->i32 effects {{throws<SpawnError>,throws<Work>,spawn,join,may_block}} {{let x=1;thread scope s {{let h=spawn s worker(copy x)?;{binding} match {scrutinee} {{Result::Ok {{value:v}} => {{return v;}} Result::Err {{error:e}} => {{raise e;}}}}}}}}"
        );
        let module = checked(&source).unwrap();
        let result = execute_task_module(&module, &TaskExecutionScript::default()).unwrap();
        assert_eq!(
            result.root_outcome,
            TaskOutcome::Error("Work::Second".into())
        );
        assert!(result.tasks[1].joined);
        let handled = source
            .replace("raise e;", "return 9;")
            .replace("throws<SpawnError>,throws<Work>", "throws<SpawnError>");
        let module = checked(&handled).unwrap();
        assert_eq!(
            execute_task_module(&module, &TaskExecutionScript::default())
                .unwrap()
                .root_outcome,
            TaskOutcome::I32(9)
        );
    }
}

#[test]
fn both_arms_continue_without_exporting_arm_locals_or_inventing_phi_values() {
    let source = "#import std.concurrency; concurrency checked; fn worker(x:i32)->unit{return;} fn main()->i32 effects {spawn,join,may_block} {let x=5; thread scope s {match spawn s worker(copy x) {Result::Ok {value:h}=>{join move h;} Result::Err {error:e}=>{let x=9;}}} return x;}";
    let module = checked(source).unwrap();
    for succeeds in [true, false] {
        assert_eq!(
            execute_task_module(
                &module,
                &TaskExecutionScript {
                    allocation_outcomes: vec![],
                    spawn_outcomes: vec![succeeds]
                }
            )
            .unwrap()
            .root_outcome,
            TaskOutcome::I32(5)
        );
    }
    assert!(checked(&source.replace("return x;}", "return h;}")).is_err());
    assert!(checked(&source.replace("return x;}", "raise e;}")).is_err());
}

#[test]
fn malformed_patterns_scrutinees_and_unhandled_custody_fail_closed() {
    for (from, to) in [
        ("Result::Err { error: e }", "Result::Ok { value: e }"),
        ("Result::Err { error: e }", "Result::Err { value: e }"),
        (
            "Result::Err { error: e }",
            "Result::Err { error: e, extra: z }",
        ),
        (
            "match spawn s worker(copy second_value)",
            "match spawn s worker(copy second_value)?",
        ),
        (
            "match spawn s worker(copy second_value)",
            "match spawn s worker(copy second_value) + 1",
        ),
        (
            "let first_result = join move first?; return first_result;",
            "return 1;",
        ),
        (
            "let second_result = join move second?;",
            "let second_result = 1;",
        ),
        ("return first_result;", "return second;"),
    ] {
        assert!(checked(&RECOVERY.replace(from, to)).is_err(), "{to}");
    }
    let error_not_permitted = RECOVERY
        .replace("return first_result;", "raise e;")
        .replace("throws<SpawnError>, ", "");
    assert!(checked(&error_not_permitted).is_err());
}

#[test]
fn nested_handlers_and_same_name_binders_keep_their_lexical_identity() {
    let mut body = "return 11;".to_owned();
    for _ in 0..8 {
        body = format!(
            "match spawn s worker(copy x) {{Result::Ok {{value:h}}=>{{join move h; {body}}} Result::Err {{error:e}}=>{{return 12;}}}}"
        );
    }
    let source = format!(
        "#import std.concurrency; concurrency checked;fn worker(x:i32)->unit{{return;}}fn main()->i32 effects{{spawn,join,may_block}}{{let x=1;thread scope s {{{body}}}}}"
    );
    let module = checked(&source).unwrap();
    let succeeded = execute_task_module(&module, &TaskExecutionScript::default()).unwrap();
    assert_eq!(succeeded.root_outcome, TaskOutcome::I32(11));
    assert_eq!(succeeded.tasks.len(), 9);
    let failed = execute_task_module(
        &module,
        &TaskExecutionScript {
            allocation_outcomes: vec![],
            spawn_outcomes: vec![true, true, false],
        },
    )
    .unwrap();
    assert_eq!(failed.root_outcome, TaskOutcome::I32(12));
    assert_eq!(failed.tasks.len(), 3);
    assert!(
        failed
            .tasks
            .iter()
            .filter(|t| t.parent.is_some())
            .all(|t| t.joined)
    );
}

#[test]
fn match_error_arm_can_precede_success_and_one_arm_can_continue() {
    let source = "#import std.concurrency; concurrency checked;fn worker(x:i32)->unit{return;}fn main()->i32 effects{spawn,join,may_block}{let x=4;thread scope s{match spawn s worker(copy x){Result::Err{error:e}=>{return 8;}Result::Ok{value:h}=>{join move h;}}}return x;}";
    let module = checked(source).unwrap();
    for (succeeds, value) in [(true, 4), (false, 8)] {
        assert_eq!(
            execute_task_module(
                &module,
                &TaskExecutionScript {
                    allocation_outcomes: vec![],
                    spawn_outcomes: vec![succeeds]
                }
            )
            .unwrap()
            .root_outcome,
            TaskOutcome::I32(value)
        );
    }
    let doubled_result = "#import std.concurrency; concurrency checked;fn worker(x:i32)->i32{return x;}fn main()->i32 effects{throws<SpawnError>,spawn,join,may_block}{let x=4;thread scope s{let h=spawn s worker(copy x)?;let r=join move h;match r{Result::Ok{value:v}=>{}Result::Err{error:e}=>{}}match r{Result::Ok{value:v}=>{return v;}Result::Err{error:e}=>{return 0;}}}}";
    assert!(checked(doubled_result).is_err());
}
