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

//! Card `W48-QR-CLOSURES` (card 4 of the `.qr` language track): closures that
//! capture from the enclosing scope into concurrent workers.
//!
//! `spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }` is lifted by the
//! parser into a **hidden named worker** declared immediately before the
//! function that spawns it — one parameter per capture, the written result and
//! effect row, the closure's body — and the spawn becomes an ordinary spawn of
//! that worker with the written captures. The task route decides each
//! parameter's type from the captured place at the spawn site, so the lifted
//! program checks and emits exactly as its hand-written twin does, and the
//! transferability judgment, the task ledger and the emitter apply unchanged.
//! There is no implicit capture: the hidden worker's scope holds the capture
//! binders and nothing of the enclosing scope.
//!
//! Negative controls first (the brief's list, in its order), then twin
//! identity, then the published examples' native runs on real threads.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

use qcore_compiler::{CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, TaskModule, execute_with_source_lines};

#[cfg(target_os = "linux")]
static NEXT: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("closures.qr", source)],
        &CompileOptions::default(),
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile(source).unwrap_or_else(|errors| panic!("expected admission, got {errors:#?}"))
}

fn refusals(source: &str) -> Vec<(String, String)> {
    match compile(source) {
        Ok(module) => panic!(
            "expected a refusal, admitted with profile {:?}",
            module.execution_profile
        ),
        Err(errors) => errors
            .into_iter()
            .map(|error| (error.code, error.message))
            .collect(),
    }
}

fn refused_with(source: &str, code: &str, needles: &[&str]) {
    let found = refusals(source);
    assert!(
        found.iter().any(|(found, message)| found == code
            && needles.iter().all(|needle| message.contains(needle))),
        "expected {code} mentioning {needles:?}, got {found:#?}"
    );
}

const HEAD: &str = "#import std.concurrency;\nconcurrency checked;\n";
const ROW: &str = "effects { throws<SpawnError>, spawn, join, may_block }";
const OWNER_ROW: &str = "effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, \
                         may_block }";

/// A parent binding `x = 42` and `y = 7`, spawning `spawn` under `?` in scope
/// `s` and returning the joined result.
fn scalar_parent(spawn: &str) -> String {
    format!(
        "{HEAD}fn main() -> i32 {ROW} {{\n    let x = 42;\n    let y = 7;\n    thread scope s {{\n        \
         let h = {spawn}?;\n        let r = join move h?;\n        return r;\n    }}\n}}\n"
    )
}

// ---------------------------------------------------------------------------
// Negative controls (the brief's list, in its order).
// ---------------------------------------------------------------------------

/// No implicit capture: an outer name the capture list does not name does not
/// resolve inside the body, and the refusal names it and says why.
#[test]
fn an_unlisted_outer_name_is_refused_by_name() {
    refused_with(
        &scalar_parent("spawn s |copy x| -> i32 { return y; }"),
        "Q013",
        &[
            "`y` belongs to the scope enclosing this closure",
            "does not capture it",
            "nothing is captured implicitly",
        ],
    );
    // The same body with `y` listed is admitted: the list was the refusal.
    admitted(&scalar_parent(
        "spawn s |copy x, copy y| -> i32 { return y; }",
    ));
    // A name the enclosing scope does not have either keeps the ordinary
    // refusal: only a name the reader can see around the closure is named as
    // an uncaptured one.
    refused_with(
        &scalar_parent("spawn s |copy x| -> i32 { return z; }"),
        "Q013",
        &["unknown name `z`"],
    );
}

/// A capture mode the transferability judgment refuses is refused with the
/// judgment's own reason — the same judgment a named worker's capture meets.
#[test]
fn a_capture_mode_the_judgment_refuses_is_refused_with_its_reason() {
    // `copy` of a record that owns: the aggregate judgment names the leaf.
    let copy_owning = format!(
        "{HEAD}type Job {{ count: i32, data: own<Buffer> }}\nfn main() -> i32 {OWNER_ROW} {{\n    \
         let d = qcore_buffer_new(1)?;\n    let job = Job {{ count: 1, data: d }};\n    thread scope \
         s {{\n        match spawn s |copy job| -> i32 {{ drop job; return 0; }} {{\n            \
         Result::Ok {{ value: h }} => {{ let r = join move h?; drop job; return r; }}\n            \
         Result::Err {{ error: f }} => {{ drop job; return 1; }}\n        }}\n    }}\n}}\n"
    );
    refused_with(
        &copy_owning,
        "Q600",
        &["transferability judgment", "the leaf `job.data`", "`copy`"],
    );
    // `write` is refused by name, before any kind is asked.
    refused_with(
        &scalar_parent("spawn s |write x| -> i32 { return x; }"),
        "Q600",
        &["a `write` capture names no transfer or share contract"],
    );
    // `move` of a scalar.
    refused_with(
        &scalar_parent("spawn s |move x| -> i32 { return x; }"),
        "Q600",
        &["no transfer or share contract admits `move` of `i32`"],
    );
    // A handle is not a value: no worker parameter could receive it.
    refused_with(
        &scalar_parent(
            "spawn s |copy x| -> i32 { return x; }?;\n        let g = spawn s |copy h| -> i32 { \
             return 0; }",
        ),
        "Q600",
        &["the closure's capture `h` names a task handle"],
    );
}

/// A capture list names each place once, and each binder once.
#[test]
fn a_duplicate_capture_is_refused() {
    refused_with(
        &scalar_parent("spawn s |copy x, copy x as z| -> i32 { return z; }"),
        "Q011",
        &["the closure captures `x` twice"],
    );
    refused_with(
        &scalar_parent("spawn s |copy x, copy y as x| -> i32 { return x; }"),
        "Q011",
        &["two captures of the closure are bound to `x`", "`as NAME`"],
    );
}

/// Two closures leasing overlapping ranges of one Buffer at once: the ledger's
/// own overlap refusal, exactly as for two named workers.
#[test]
fn overlapping_lends_are_refused() {
    let program = |second: &str| {
        format!(
            "{HEAD}fn main() -> i32 {OWNER_ROW} {{\n    let p = qcore_buffer_new(0)?;\n    thread scope \
             s {{\n        match spawn s |lend p 0..3| -> unit {{ let a = qcore_buffer_write(p, 0, 1); \
             return; }} {{\n            Result::Ok {{ value: first }} => {{\n                match \
             spawn s |lend p {second}| -> unit {{ let b = qcore_buffer_write(p, 0, 2); return; }} {{\n                    \
             Result::Ok {{ value: second }} => {{ join move first; join move second; drop p; return 0; \
             }}\n                    Result::Err {{ error: r }} => {{ join move first; drop p; return 1; \
             }}\n                }}\n            }}\n            Result::Err {{ error: f }} => {{ drop p; \
             return 1; }}\n        }}\n    }}\n}}\n"
        )
    };
    refused_with(&program("2..4"), "Q600", &["OverlappingLeases"]);
    // Disjoint ranges of the same Buffer are admitted.
    admitted(&program("3..4"));
}

/// A closure whose returns do not match its declared result type is refused
/// by the worker's own result rule.
#[test]
fn a_result_type_mismatch_is_refused() {
    refused_with(
        &scalar_parent("spawn s |copy x| -> u8 { return x; }"),
        "Q020",
        &["return value does not match the declared task function result"],
    );
    // The result type is required: nothing is inferred.
    refused_with(
        &scalar_parent("spawn s |copy x| { return x; }"),
        "Q000",
        &["a closure declares its result type after its capture list"],
    );
}

/// A closure is a spawn target only: stored in a binding or passed to a
/// worker, it is a closure as a value, which is not admitted.
#[test]
fn a_closure_at_a_non_spawn_position_is_refused() {
    let stored = format!(
        "{HEAD}fn main() -> i32 {ROW} {{\n    let x = 42;\n    let c = |copy x| -> i32 {{ return x; \
         }};\n    return 0;\n}}\n"
    );
    refused_with(
        &stored,
        "Q038",
        &["a closure is a spawn target only", "W48-QR-CLOSURE-VALUES"],
    );
    refused_with(
        &format!(
            "{HEAD}fn w(v: i32) -> i32 {{ return v; }}\n{}",
            scalar_parent("spawn s w(|copy x| -> i32 { return x; })").trim_start_matches(HEAD)
        ),
        "Q000",
        &["a closure is a spawn target only", "W48-QR-CLOSURE-VALUES"],
    );
}

/// A capture list never touches `[` or `]`: `[|` and `|]` are the quant
/// delimiters, so a bracket beside a bar is refused rather than read.
#[test]
fn a_capture_list_adjacent_to_a_bracket_is_refused() {
    for spawn in [
        "spawn s [|copy x|] -> i32 { return x; }",
        "spawn s |copy x|] -> i32 { return x; }",
    ] {
        refused_with(&scalar_parent(spawn), "Q000", &["never touches `[` or `]`"]);
    }
}

/// Recursion and nested spawn inside a closure follow the worker-leaf rules:
/// a closure that spawns (here, another closure in its own scope) is a
/// task-bearing worker and is refused, at the closure's own span.
#[test]
fn a_closure_that_spawns_is_refused_by_the_worker_leaf_rule() {
    let source = scalar_parent(
        "spawn s |copy x| -> i32 effects { throws<SpawnError>, spawn, join, may_block } {\n            \
         thread scope t {\n                let g = spawn t |copy x| -> i32 { return x; }?;\n                \
         let q = join move g?;\n                return q;\n            }\n        }",
    );
    refused_with(
        &source,
        "Q600",
        &["spawned workers must be first-order leaf functions"],
    );
    // The outer scope is not the closure's either: using it is an uncaptured
    // outer name.
    refused_with(
        &scalar_parent(
            "spawn s |copy x| -> i32 { thread scope t { let g = spawn s |copy x| -> i32 { return \
             x; }?; let q = join move g?; return q; } }",
        ),
        "Q013",
        &["`s` belongs to the scope enclosing this closure"],
    );
}

/// A closure on a route that lowers no closure is refused by name before that
/// route reads it.
#[test]
fn a_closure_off_the_scoped_task_routes_is_refused_by_name() {
    let epoch = format!(
        "{HEAD}fn main() -> i32 {OWNER_ROW} {{\n    thread scope plan epoch {{\n        bound 65536 \
         effort 1000;\n        overhead scope 512 thread 128;\n        thread scope worker {{\n            \
         copy;\n            peak 1024;\n            effort 10;\n            retained 64;\n            \
         join(drop);\n        }}\n    }}\n    let value = 42;\n    thread scope s {{\n        let \
         handle = spawn s |copy value| -> i32 {{ return value; }}?;\n        let r = join move \
         handle?;\n        return r;\n    }}\n}}\n"
    );
    refused_with(&epoch, "Q600", &["W48-QR-CLOSURE-EPOCH"]);
    let mutex = "#import std.concurrency;\nconcurrency checked;\nfn main() -> i32 effects { \
                 throws<SpawnError>, spawn, join, may_block } {\n    let x = 42;\n    thread scope s \
                 {\n        let buf = protected 4;\n        let m = mutex guards buf;\n        let h = \
                 spawn s |copy x| -> i32 { return x; }?;\n        let r = join move h?;\n        \
                 with lock m as g { read buf with g; }\n        return r;\n    }\n}\n";
    refused_with(mutex, "Q600", &["W48-QR-CLOSURE-MUTEX"]);
}

// ---------------------------------------------------------------------------
// The lift and twin identity.
// ---------------------------------------------------------------------------

/// The parser's lift: a hidden worker named `ENCLOSING#closureN`, declared
/// immediately before the function that spawns it, its parameters the capture
/// binders (root name, last field, or `as NAME`), and the spawn naming it.
#[test]
fn the_parser_lifts_a_closure_into_a_hidden_worker_before_its_function() {
    use qcore_frontend::frontend::{SourceTaskExpressionKind, parse_source_program};
    let source = format!(
        "{HEAD}type P {{ a: i32, b: i32 }}\nfn main() -> i32 {ROW} {{\n    let x = 42;\n    let p = P \
         {{ a: 1, b: 2 }};\n    thread scope s {{\n        let h = spawn s |copy x as v, copy p.b| -> \
         i32 {{ return v; }}?;\n        let r = join move h?;\n        let g = spawn s |copy p| -> i32 \
         {{ let q = p.a; return q; }}?;\n        let t = join move g?;\n        return r;\n    \
         }}\n}}\n"
    );
    let program = parse_source_program(&source).unwrap();
    let names: Vec<&str> = program
        .declarations
        .iter()
        .skip(1)
        .map(|declaration| declaration.value.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["P", "main#closure1", "main#closure2", "main"],
        "the hidden workers precede `main`, in source order"
    );
    let first = program.closure("main#closure1").unwrap();
    let binders: Vec<&str> = first.binders.iter().map(|b| b.value.as_str()).collect();
    assert_eq!(binders, ["v", "b"]);
    assert_eq!(
        &source[first.captures_span.start..first.captures_span.end],
        "|copy x as v, copy p.b|"
    );
    assert!(source[first.span.start..first.span.end].ends_with("{ return v; }"));
    let hidden = &program.declarations[2].value;
    assert_eq!(hidden.parameters.len(), 2);
    assert_eq!(hidden.result_type.as_ref().unwrap().source, "i32");
    let spawns: Vec<&str> = program
        .task_expressions()
        .iter()
        .filter_map(|task| match &task.kind {
            SourceTaskExpressionKind::Spawn { worker, .. } => Some(worker.value.as_str()),
            SourceTaskExpressionKind::Join { .. } => None,
        })
        .collect();
    assert_eq!(spawns, ["main#closure1", "main#closure2"]);
    // A source identifier cannot be spelled with `#`: no declaration the
    // reader writes can take the hidden name.
    let clash = format!("{HEAD}fn main#closure1() -> i32 {{ return 0; }}\n");
    assert!(parse_source_program(&clash).map_or(true, |program| {
        program
            .declarations
            .iter()
            .all(|declaration| !declaration.value.name.contains('#'))
    }));
}

/// The checked task module with every name and source position erased: what
/// must agree between a closure program and its hand-written twin.
fn shape(module: &TaskModule) -> String {
    let mut module = module.clone();
    module.source_len = 0;
    for function in module.functions.values_mut() {
        function.name = String::new();
    }
    let text = format!("{module:?}");
    // Every `TaskSpan { start: N, end: M }` names a source position.
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("TaskSpan { start: ") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let close = tail.find('}').unwrap();
        out.push_str("TaskSpan");
        rest = &tail[close + 1..];
    }
    out.push_str(rest);
    out
}

fn task_module(module: &CompiledModule) -> TaskModule {
    if let Some(composed) = module.checked.as_composed_tasks() {
        return composed.tasks().module().clone();
    }
    match &module.checked {
        qcore_compiler::CheckedSourceModule::ScopedTasks(checked) => checked.module().clone(),
        other => panic!("not a task module: {other:?}"),
    }
}

/// A closure program and its hand-written twin: both check; the emitted C,
/// the profile, the declaration count, the entry and the result profile are
/// identical; the checked task modules agree except in names and source
/// positions; the reference runs agree.
fn assert_twins(label: &str, closure: &str, twin: &str) {
    let lifted = compile(closure).unwrap_or_else(|errors| panic!("{label}: {errors:#?}"));
    let written = compile(twin).unwrap_or_else(|errors| panic!("{label} twin: {errors:#?}"));
    assert_eq!(lifted.c_source, written.c_source, "{label}: emitted C");
    assert!(lifted.c_source.is_some(), "{label}: emits C");
    assert_eq!(
        lifted.execution_profile, written.execution_profile,
        "{label}"
    );
    assert_eq!(
        lifted.declaration_count, written.declaration_count,
        "{label}"
    );
    assert_eq!(lifted.entry, written.entry, "{label}");
    // The result profile names each function; a hidden worker is named for
    // its closure and the twin's worker by its writer, so the profiles are
    // compared through each name's function identity, which the two share.
    let (lifted_tasks, written_tasks) = (task_module(&lifted), task_module(&written));
    let by_identity = |module: &CompiledModule, tasks: &TaskModule| {
        let identity = |name: &String| {
            tasks
                .functions
                .iter()
                .find(|(_, function)| &function.name == name)
                .map(|(id, _)| *id)
                .unwrap_or_else(|| panic!("{label}: `{name}` is not a task function"))
        };
        let profile = &module.result_profile;
        (
            profile
                .scalar_results
                .iter()
                .map(|(name, ty)| (identity(name), format!("{ty:?}")))
                .collect::<BTreeMap<_, _>>(),
            profile
                .owned_buffer_results
                .iter()
                .map(identity)
                .collect::<Vec<_>>(),
            format!("{:?}", profile.custody_payloads),
        )
    };
    assert_eq!(
        by_identity(&lifted, &lifted_tasks),
        by_identity(&written, &written_tasks),
        "{label}: result profile"
    );
    assert_eq!(
        shape(&lifted_tasks),
        shape(&written_tasks),
        "{label}: task IR"
    );
    if let (Some(a), Some(b)) = (
        lifted.checked.as_composed_tasks(),
        written.checked.as_composed_tasks(),
    ) {
        let run = |composed: &qcore_core::composed_tasks::CheckedComposedTaskModule| {
            execute_with_source_lines(
                composed.tasks(),
                &TaskExecutionScript::default(),
                &BTreeMap::new(),
                composed.sites(),
            )
            .unwrap()
            .projections()
        };
        assert_eq!(run(a), run(b), "{label}: reference run");
    }
}

const CLOSURES: &str = include_str!("../../examples/threads/closures.qr");
const CLOSURES_TWIN: &str = include_str!("../../examples/threads/closures_hand_written.qr");
const ERRORS: &str = include_str!("../../examples/threads/closure_errors.qr");
const ERRORS_TWIN: &str = include_str!("../../examples/threads/closure_errors_hand_written.qr");

/// The published examples and their published twins.
#[test]
fn the_published_examples_are_byte_identical_to_their_twins() {
    assert_twins("closures", CLOSURES, CLOSURES_TWIN);
    assert_twins("closure_errors", ERRORS, ERRORS_TWIN);
}

/// Every capture mode, scalar and record, against its twin: the parameter a
/// closure's capture binds is the captured place's type, or its loan or lease.
#[test]
fn every_capture_mode_matches_its_twin() {
    const REC: &str = "type Pair { a: u16, buf: own<Buffer<u8>> }\ntype Two { a: i32, b: i64 }\n";
    let program = |workers: &str, spawn: &str, ok: &str| {
        format!(
            "{HEAD}{REC}{workers}fn main() -> i32 effects {{ alloc, throws<AllocError>, spawn, join, \
             may_block }} {{\n    let seed: u8 = 7;\n    let b = qcore_buffer_new(seed)?;\n    let a: \
             u16 = 3;\n    let p = Pair {{ a: a, buf: b }};\n    let wide: i64 = 9;\n    let two = Two \
             {{ a: 5, b: wide }};\n    match qcore_buffer_new(seed) {{\n    Result::Ok {{ value: c }} => \
             {{\n    thread scope s {{\n        match spawn s {spawn} {{\n            Result::Ok {{ value: \
             h }} => {{\n{ok}\n            }}\n            Result::Err {{ error: f }} => {{\n                \
             drop p;\n                drop c;\n                return 1;\n            }}\n        }}\n    \
             }}\n    }}\n    Result::Err {{ error: e }} => {{\n        drop p;\n        return 1;\n    \
             }}\n    }}\n}}\n"
        )
    };
    let keep_all = "                let r = join move h?;\n                drop p;\n                \
                    drop c;\n                return 0;";
    let cases: [(&str, &str, &str, &str, &str); 7] = [
        (
            "copy of a scalar field and a whole record of scalars",
            "|copy p.a, copy two| -> unit { let x = two.a; return; }",
            "fn w(a: u16, two: Two) -> unit {\n    let x = two.a;\n    return;\n}\n",
            "w(copy p.a, copy two)",
            keep_all,
        ),
        (
            "move of an owned Buffer and a read share of another",
            "|move c, read p.buf share 512| -> unit { let v = qcore_buffer_read(buf); drop c; \
             return; }",
            "fn w(c: own<Buffer<u8>> use one, buf: borrow<Buffer<u8>>) -> unit {\n    let v = \
             qcore_buffer_read(buf);\n    drop c;\n    return;\n}\n",
            "w(move c, read p.buf share 512)",
            "                let r = join move h?;\n                drop p;\n                return 0;",
        ),
        (
            "a range lease of an owned field",
            "|lend p.buf 0..1| -> unit { let z = qcore_buffer_write(buf, 0, 9); return; }",
            "fn w(buf: lend<Buffer<u8>>) -> unit {\n    let z = qcore_buffer_write(buf, 0, 9);\n    \
             return;\n}\n",
            "w(lend p.buf 0..1)",
            keep_all,
        ),
        (
            "a whole record that owns, moved and returned",
            "|move p| -> Pair { return move p; }",
            "fn w(p: Pair use one) -> Pair {\n    return move p;\n}\n",
            "w(move p)",
            "                let back = join move h?;\n                drop back;\n                \
             drop c;\n                return 0;",
        ),
        (
            "an owned field moved out, leaving a hole the parent does not use",
            "|move p.buf as data| -> u8 { let v = qcore_buffer_read(data); drop data; return v; }",
            "fn w(data: own<Buffer<u8>> use one) -> u8 {\n    let v = qcore_buffer_read(data);\n    \
             drop data;\n    return v;\n}\n",
            "w(move p.buf)",
            "                let r = join move h?;\n                drop c;\n                return 0;",
        ),
        (
            "an empty capture list",
            "|| -> i32 { return 5; }",
            "fn w() -> i32 {\n    return 5;\n}\n",
            "w()",
            keep_all,
        ),
        (
            "two captures bound under `as`",
            "|copy two.a as first, copy p.a as second| -> i32 { return first; }",
            "fn w(first: i32, second: u16) -> i32 {\n    return first;\n}\n",
            "w(copy two.a, copy p.a)",
            keep_all,
        ),
    ];
    for (label, closure, worker, call, ok) in cases {
        let lifted =
            program("", &format!("s {closure}"), ok).replace("match spawn s s ", "match spawn s ");
        let twin =
            program(worker, &format!("s {call}"), ok).replace("match spawn s s ", "match spawn s ");
        assert_twins(label, &lifted, &twin);
    }
}

// ---------------------------------------------------------------------------
// The published examples on real threads.
// ---------------------------------------------------------------------------

const EXAMPLES: [(&str, &str); 2] = [("closures", CLOSURES), ("closure_errors", ERRORS)];

fn example(name: &str, source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new(
            format!("examples/threads/{name}.qr"),
            source,
        )],
        &CompileOptions::default(),
    )
    .unwrap_or_else(|errors| panic!("{name} does not check: {errors:#?}"))
}

/// The reference projections under a creation script.
fn reference(module: &CompiledModule, spawn_outcomes: Vec<bool>) -> (String, String) {
    let composed = module.checked.as_composed_tasks().unwrap();
    execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript {
            spawn_outcomes,
            allocation_outcomes: Vec::new(),
        },
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap()
    .projections()
}

/// The three builds every native row of the track uses.
#[cfg(target_os = "linux")]
const BUILDS: [(&str, &[&str]); 3] = [
    (
        "O0-ubsan",
        &[
            "-O0",
            "-g",
            "-fsanitize=undefined,integer",
            "-fno-sanitize-recover=all",
        ],
    ),
    (
        "O2-ubsan",
        &[
            "-O2",
            "-g",
            "-fsanitize=undefined,integer",
            "-fno-sanitize-recover=all",
        ],
    ),
    ("O2", &["-O2"]),
];

#[cfg(target_os = "linux")]
struct Run {
    stdout: String,
    stderr: String,
    status: Option<i32>,
}

/// Build the emitted C with `-std=c11 -Wall -Wextra -Werror -pedantic` and the
/// build's flags (after `prelude`, which the spawn-failure row uses to refuse
/// every creation through the runtime's own `QCORE_TASK_CREATE` seam), and run
/// it.
#[cfg(target_os = "linux")]
fn build_and_run(c: &str, flags: &[&str], prelude: Option<&str>) -> Result<Run, String> {
    let dir = std::env::temp_dir().join(format!(
        "qcore-closures-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let source = match prelude {
        Some(prelude) => format!("{prelude}{c}"),
        None => c.to_owned(),
    };
    std::fs::write(dir.join("main.c"), source).unwrap();
    let out = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()))
        .args([
            "-std=c11",
            "-pthread",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
        ])
        .args(flags)
        .arg(dir.join("main.c"))
        .arg("-o")
        .arg(dir.join("main"))
        .output()
        .expect("Clang is required for this native check");
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let run = Command::new(dir.join("main"))
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Ok(Run {
        stdout: String::from_utf8(run.stdout).unwrap(),
        stderr: String::from_utf8(run.stderr).unwrap(),
        status: run.status.code(),
    })
}

/// A prelude that refuses every task creation through the runtime's own
/// `QCORE_TASK_CREATE` seam, so the program takes its spawn-failure arm.
#[cfg(target_os = "linux")]
const REFUSE_CREATION: &str = "#include <threads.h>\nstatic int qcore_refuse_create(thrd_t *thread, \
     int (*function)(void *), void *argument) {\n    (void)thread; (void)function; (void)argument;\n    \
     return thrd_error;\n}\n#define QCORE_TASK_CREATE qcore_refuse_create\n";

/// Each example, three builds: stdout and stderr equal the reference, exit 0,
/// no sanitizer report; `closures` also with every creation refused, against
/// the reference run whose first creation fails (the record stays the
/// parent's). ThreadSanitizer rows follow. The summary goes to the report's
/// `native-closures.json` when `QCORE_W48_CLOSURES_NATIVE_SUMMARY` names a
/// file.
#[cfg(target_os = "linux")]
#[test]
fn the_published_examples_run_clean_under_three_builds() {
    let mut summary = Vec::new();
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let c = module.c_source.as_deref().unwrap();
        let mut cases = vec![("created", reference(&module, Vec::new()), None)];
        if name == "closures" {
            cases.push((
                "creation-refused",
                reference(&module, vec![false]),
                Some(REFUSE_CREATION),
            ));
        }
        for (arm, (stdout, stderr), prelude) in cases {
            for (label, flags) in BUILDS {
                let run = build_and_run(c, flags, prelude).unwrap_or_else(|error| {
                    panic!("{name} {label}: the emitted C does not build: {error}")
                });
                assert_eq!(run.status, Some(0), "{name} {arm} {label}: {}", run.stderr);
                assert_eq!(run.stdout, stdout, "{name} {arm} {label}");
                assert_eq!(
                    run.stderr, stderr,
                    "{name} {arm} {label}: a sanitizer report or a changed line"
                );
                summary.push(format!(
                    "{{\"program\": \"examples/threads/{name}.qr\", \"arm\": \"{arm}\", \"build\": \
                     \"{label}\", \"exit\": 0, \"stdout_lines\": {}, \"stderr_lines\": {}, \
                     \"sanitizer_reports\": 0}}",
                    stdout.lines().count(),
                    stderr.lines().count()
                ));
            }
        }
    }
    summary.extend(tsan_rows());
    if let Ok(path) = std::env::var("QCORE_W48_CLOSURES_NATIVE_SUMMARY") {
        let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler().unwrap();
        std::fs::write(
            path,
            format!(
                "{{\n \"compiler\": \"{compiler}\",\n \"triple\": \"{triple}\",\n \"flags\": \"-std=c11 \
                 -pthread -Wall -Wextra -Werror -pedantic\",\n \"runs\": [\n  {}\n ]\n}}\n",
                summary.join(",\n  ")
            ),
        )
        .unwrap();
    }
}

#[cfg(target_os = "linux")]
#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;

/// ThreadSanitizer over both examples, through the existing pthread binding
/// (`TASK-ROUTE-TSAN-PTHREAD-BINDING`): each unforced, and the concurrent
/// example forced with both closures live together and forced in reverse.
/// Each row is clean — no report of any kind — and prints the reference
/// stdout. Returns the summary rows.
#[cfg(target_os = "linux")]
fn tsan_rows() -> Vec<String> {
    use thread_sanitizer::{Row, Schedule, ScheduleEvent};
    let mut rows = Vec::new();
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let c = module.c_source.as_deref().unwrap();
        let (stdout, _) = reference(&module, Vec::new());
        let schedules: &[(Schedule, usize)] = if name == "closures" {
            &[
                (Schedule::Unforced, 0),
                (Schedule::ForcedTogether, 2),
                (Schedule::ForcedReverse, 2),
            ]
        } else {
            &[(Schedule::Unforced, 0)]
        };
        for (schedule, cohort) in schedules {
            let run = Row {
                tag: name,
                source: c,
                schedule: *schedule,
                cohort: *cohort,
                ..Row::default()
            }
            .execute()
            .unwrap_or_else(|error| panic!("{name} tsan: {error}"));
            assert!(
                run.clean,
                "{name} {}: ThreadSanitizer reported:\n{}",
                schedule.tag(),
                run.stderr
            );
            assert_eq!(run.races, 0);
            if schedule.is_forced() {
                let started = run
                    .schedule
                    .iter()
                    .filter(|event| matches!(event, ScheduleEvent::Start(_)))
                    .count();
                assert_eq!(
                    started,
                    *cohort,
                    "{name} {}: {:?}",
                    schedule.tag(),
                    run.schedule
                );
            }
            assert_eq!(run.stdout, stdout, "{name} {}", schedule.tag());
            rows.push(format!(
                "{{\"program\": \"examples/threads/{name}.qr\", \"arm\": \"created\", \"build\": \
                 \"tsan-{}\", \"exit\": 0, \"stdout_lines\": {}, \"sanitizer_reports\": 0}}",
                schedule.tag(),
                stdout.lines().count()
            ));
        }
    }
    rows
}

/// The examples page publishes each example's transcript, read out of the
/// page and compared with the reference.
#[test]
fn the_threads_page_publishes_the_closure_transcripts() {
    let page = std::fs::read_to_string(root().join("examples/threads/README.md"))
        .unwrap()
        .replace("\r\n", "\n");
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let (stdout, _) = reference(&module, Vec::new());
        let marker = format!("<!-- transcript:{name} -->\n\n```text\n");
        let block = page
            .split_once(marker.as_str())
            .unwrap_or_else(|| panic!("the `{name}` transcript is not on the page"))
            .1
            .split_once("```")
            .unwrap()
            .0;
        assert_eq!(block, stdout, "{name}");
    }
}
