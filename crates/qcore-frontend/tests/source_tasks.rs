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

use qcore_core::task_execution::{
    TaskInstructionKind, TaskModule, TaskTerminatorKind, check_task_module,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_tasks::{has_source_tasks, lower_source_tasks};

const WORKER: &str = "fn worker(x: i32) -> i32 { return x; }";
const EFFECTS: &str = "effects { throws<SpawnError>, spawn, join, may_block }";

fn lower(source: &str) -> Result<TaskModule, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let typed = elaborate_source(&parsed).map_err(|e| format!("{e:?}"))?;
    lower_source_tasks(&parsed, &typed, "main").map_err(|e| format!("{e:?}"))
}

fn module(body: &str) -> String {
    format!(
        "#import std.concurrency; concurrency checked; {WORKER} fn main() -> i32 {EFFECTS} {{ let x: i32 = -7; {body} }}"
    )
}

#[test]
fn scalar_copy_source_lowers_to_dynamic_creation_and_explicit_collection() {
    let source = module(
        "thread scope s { let h = spawn s worker(copy x)?; let result = join move h; let value = result?; return value; }",
    );
    let module = lower(&source).unwrap();
    check_task_module(&module).unwrap();
    let main = &module.functions[&module.entry];
    assert!(
        main.blocks
            .values()
            .any(|b| matches!(b.terminator.kind, TaskTerminatorKind::TrySpawn { .. }))
    );
    assert!(
        main.blocks
            .values()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(i.kind, TaskInstructionKind::UnwrapInfallible { .. }))
    );
    assert!(
        main.blocks
            .values()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.kind,
                TaskInstructionKind::Join {
                    outcome_slot: Some(_),
                    ..
                }
            ))
    );
    for instruction in main.blocks.values().flat_map(|b| &b.instructions) {
        assert!(!source[instruction.span.start..instruction.span.end].is_empty());
    }
}

#[test]
fn direct_join_propagation_and_infallible_unit_discard_are_supported() {
    check_task_module(&lower(&module("thread scope s { let h = spawn s worker(copy x)?; let value: i32 = join move h?; return value; }")).unwrap()).unwrap();
    let source = format!(
        "#import std.concurrency; concurrency checked; fn worker(x: i32) -> unit {{ return; }} fn main() -> unit {EFFECTS} {{ let x = 1; thread scope s {{ let h = spawn s worker(copy x)?; join move h; }} return; }}"
    );
    check_task_module(&lower(&source).unwrap()).unwrap();
}

#[test]
fn declared_worker_error_is_preserved_as_a_distinct_outcome() {
    let source = "#import std.concurrency; concurrency checked; error WorkError { Failed; } fn worker(x: i32) -> i32 effects { throws<WorkError> } { raise WorkError::Failed; } fn main() -> i32 effects { throws<SpawnError>, throws<WorkError>, spawn, join, may_block } { let x = 3; thread scope s { let h = spawn s worker(copy x)?; let r = join move h; let value = r?; return value; } }";
    let ir = lower(source).unwrap();
    check_task_module(&ir).unwrap();
    let worker = ir.functions.values().find(|f| f.name == "worker").unwrap();
    assert_eq!(
        worker.errors.iter().cloned().collect::<Vec<_>>(),
        vec!["WorkError::Failed"]
    );
    assert!(
        lower(&source.replace(
            "throws<SpawnError>, throws<WorkError>",
            "throws<SpawnError>"
        ))
        .is_err()
    );
}

#[test]
fn same_name_handles_and_empty_scope_binders_keep_exact_identities() {
    let source = module(
        "thread scope empty { } thread scope s { let h = spawn s worker(copy x)?; let value = join move h?; thread scope t { let h = spawn t worker(copy value)?; let value = join move h?; return value; } }",
    );
    let ir = lower(&source).unwrap();
    check_task_module(&ir).unwrap();
    let scopes = ir.functions[&ir.entry]
        .blocks
        .values()
        .flat_map(|b| &b.instructions)
        .filter_map(|i| match i.kind {
            TaskInstructionKind::EnterScope { scope } => Some(scope),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(scopes.len(), 3);
}

#[test]
fn detection_covers_uncalled_and_empty_scopes_without_admitting_other_forms() {
    let parsed = parse_source_program(
        "#import std.concurrency; fn main() -> unit { return; } fn unused() -> unit { thread scope s { } return; }",
    )
    .unwrap();
    assert!(has_source_tasks(&parsed));
    let parsed = parse_source_program("fn main() -> unit { return; }").unwrap();
    assert!(!has_source_tasks(&parsed));
}

#[test]
fn whole_statement_consumption_rejects_ignored_suffixes_and_multiple_tasks() {
    for body in [
        "thread scope s { let h = spawn s worker(copy x)? + 1; return 0; }",
        "thread scope s { let h = spawn s worker(copy x)? spawn s worker(copy x)?; return 0; }",
        "thread scope s junk { let h = spawn s worker(copy x)?; let r = join move h?; return r; }",
        "thread scope s { let h = spawn s worker(copy x)?; let r = join move h? + 1; return r; }",
        "thread scope s { let h = spawn s worker(copy x)?; return join move h?; }",
        // A chained expression is still refused: a value statement is one
        // operation (card `W48-QR-SCALAR-TYPES-R4`).
        "let y = x + 1 + 2; return y;",
    ] {
        assert!(lower(&module(body)).is_err(), "accepted {body}");
    }
    // Card `W48-QR-SCALAR-TYPES-R4` deliberately widens one shape this test
    // pinned as refused: `let y = x + 1;` over the entry's known `x` is a value
    // statement, folded at check time to one instruction with no error edge.
    let widened = lower(&module("let y = x + 1; return y;")).unwrap();
    check_task_module(&widened).unwrap();
    assert!(
        widened.functions[&widened.entry]
            .blocks
            .values()
            .any(|block| block
                .instructions
                .iter()
                .any(|i| matches!(i.kind, TaskInstructionKind::Compute { .. })))
    );
    // Over a run-time value (the worker's parameter) it needs the row's
    // `throws<ArithmeticError>`, and without it stays refused.
    let run_time = module("return x;").replace(
        "fn worker(x: i32) -> i32 { return x; }",
        "fn worker(x: i32) -> i32 { let y = x + 1; return y; }",
    );
    assert!(
        lower(&run_time)
            .unwrap_err()
            .contains("throws<ArithmeticError>")
    );
}

#[test]
fn type_effect_capture_and_all_definition_gates_remain_closed() {
    let valid = module(
        "thread scope s { let h = spawn s worker(copy x)?; let value = join move h?; return value; }",
    );
    for bad in [
        valid.replace("concurrency checked;", ""),
        valid.replace("spawn, join", "join"),
        valid.replace("join, may_block", "join"),
        valid.replace("throws<SpawnError>, ", ""),
        valid.replace("copy x", "move x"),
        valid.replace("copy x", "read x share FULL"),
        valid.replace("copy x", "copy x[0]"),
        valid.replace("let h =", "let h: i32 ="),
        valid.replace("let x: i32 = -7", "let x: unit = -7"),
        valid.replace("x: i32) -> i32", "x: bool) -> i32"),
        valid.replace("let value = join move h?", "let value = join move x?"),
        valid.replace("worker(copy x)?", "worker(copy x)"),
        valid.replace("return value;", "return h;"),
        format!(
            "{valid} fn bad() -> i32 {{ let p = qcore_acquire(1, true, SpawnError); return 0; }}"
        ),
        format!("{valid} const global: i32 = 9;"),
        format!("{valid} extern fn foreign() -> unit;"),
        valid.replace(
            "fn worker(x: i32) -> i32",
            "fn worker(x: i32 use one) -> i32",
        ),
        valid.replace(
            "fn worker(x: i32) -> i32",
            "fn worker(x: i32) -> i32 contract { requires true; }",
        ),
    ] {
        assert!(lower(&bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn core_rejects_all_path_handle_and_result_obligations_from_actual_source() {
    for body in [
        "thread scope s { let h = spawn s worker(copy x)?; return 0; }",
        "thread scope s { let h = spawn s worker(copy x)?; let r = join move h; return 0; }",
        "thread scope s { let h = spawn s worker(copy x)?; let h2 = spawn s worker(copy x)?; let a = join move h?; let b = join move h2?; return b; }",
        "thread scope s { let h = spawn s worker(copy x)?; let a = join move h?; let b = join move h?; return b; }",
    ] {
        let ir = lower(&module(body)).unwrap();
        assert!(check_task_module(&ir).is_err(), "accepted {body}");
    }
}

#[test]
fn independent_source_and_symbol_metadata_check_rejects_forgery() {
    let source = module(
        "thread scope s { let h = spawn s worker(copy x)?; let value = join move h?; return value; }",
    );
    let parsed = parse_source_program(&source).unwrap();
    let mut typed = elaborate_source(&parsed).unwrap();
    typed
        .declarations
        .iter_mut()
        .find(|d| d.name == "main")
        .unwrap()
        .statements[0]
        .binding = Some(qcore_frontend::frontend::SymbolId(999));
    assert!(lower_source_tasks(&parsed, &typed, "main").is_err());
    let mut parsed = parsed;
    parsed.declarations.retain(|d| d.value.name != "worker");
    assert!(
        lower_source_tasks(
            &parsed,
            &elaborate_source(&parse_source_program(&source).unwrap()).unwrap(),
            "main"
        )
        .is_err()
    );
}

#[test]
fn thread_scope_cannot_replace_an_earlier_body_with_a_later_structural_block() {
    let source = "#import std.concurrency; concurrency checked; fn main() -> i32 { thread scope s { qcore_acquire(1, true, SpawnError); } ignored { } return 0; }";
    assert!(lower(source).is_err(), "an earlier scope body was erased");
}

#[test]
fn declared_error_family_and_unused_effect_permissions_are_preserved() {
    let source = "#import std.concurrency; concurrency checked; error WorkError { First; Second; } fn worker(x: i32) -> i32 effects { throws<WorkError>, join, may_block } { raise WorkError::Second; } fn main() -> i32 effects { throws<SpawnError>, throws<WorkError>, spawn, join, may_block } { let x = 0; thread scope s { let h = spawn s worker(copy x)?; let value = join move h?; return value; } }";
    let ir = lower(source).unwrap();
    check_task_module(&ir).unwrap();
    let worker = ir.functions.values().find(|f| f.name == "worker").unwrap();
    assert_eq!(
        worker.errors.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["WorkError::First", "WorkError::Second"]
    );
    assert!(worker.declared_effects.join && worker.declared_effects.may_block);
}
