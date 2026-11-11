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

//! `source.std-io-tasks.v1` — a module that both spawns and prints
//! (card `COMPOSE-CONCURRENCY`).
//!
//! The route matrix, the refusals in both directions, the byte-identity of every
//! route this one does *not* take, and one sanitized native run compared with
//! the composed reference **per stream**.

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::{
    CompileOptions, CompiledModule, SourceExecutionProfile, SourceFile, compile_files,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_with_source_lines,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::collections::BTreeMap;

fn compile(source: &str) -> Result<CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("compose.qcore", source)],
        &CompileOptions::default(),
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .unwrap_err()
        .into_iter()
        .map(|error| error.code.to_owned())
        .collect()
}

/// A spawning entry with `BODY` in its parent, `HEADER` before the marker and
/// `EFFECTS` in its row.
fn module(header: &str, effects: &str, body: &str) -> String {
    format!(
        "#import std.concurrency;
{header}concurrency checked;
fn worker(value: i32) -> i32 {{ return value; }}
fn main() -> i32 effects {{ {effects} }} {{
    let first_value = 42;
    thread scope workers {{
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
{body}
        return first_result;
    }}
}}
"
    )
}

const IMPORTS: &str = "#import std.io;\n#import std.logger;\n#import std.fmt;\n";
const EFFECTS: &str = "io, throws<SpawnError>, spawn, join, may_block";
const TASK_EFFECTS: &str = "throws<SpawnError>, spawn, join, may_block";

// ---------------------------------------------------------------- route matrix

#[test]
fn a_module_that_spawns_and_prints_takes_the_composed_route() {
    let module = compile(&module(
        IMPORTS,
        EFFECTS,
        "        first_result |> print dec;",
    ))
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
    assert!(module.checked.as_composed_tasks().is_some());
    // The composition is not a second task route: the task accessor answers
    // `None`, and a consumer that wants the task half asks the composition for
    // it and sees the placed lines beside it.
    assert!(module.checked.as_scoped_tasks().is_none());
}

#[test]
fn importing_is_not_using_and_the_task_route_is_kept_byte_for_byte() {
    // The conjunction's own boundary: a module that imports all three modules
    // and writes no line is not composing. Its emitted C must be the task
    // route's, byte for byte — which it is by construction, since both travel
    // one lowering and one backend, and is pinned here anyway.
    let bare = compile(&module("", TASK_EFFECTS, "")).unwrap();
    let imported = compile(&module(IMPORTS, TASK_EFFECTS, "")).unwrap();
    assert_eq!(
        bare.execution_profile,
        SourceExecutionProfile::ScopedCopyTasksV1
    );
    assert_eq!(
        imported.execution_profile,
        SourceExecutionProfile::ScopedCopyTasksV1
    );
    assert!(imported.checked.as_scoped_tasks().is_some());
    assert_eq!(bare.c_source, imported.c_source);
}

#[test]
fn a_printing_module_that_does_not_spawn_keeps_the_standard_io_route() {
    let module = compile(
        "#import std.io;
#import std.fmt;
fn main() -> i32 effects { io } {
    let n = 42;
    print dec n;
    return;
}
",
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
}

/// **No synthesized line moved.** The emitted C of a composed module is the task
/// route's emitted C with exactly the source lines inserted — which is why the
/// F02 emission digests, the CT fixtures and every pinned trace byte are
/// untouched by this card.
#[test]
fn the_composed_emission_is_the_task_emission_with_lines_inserted() {
    let plain = compile(&module("", TASK_EFFECTS, "")).unwrap();
    let composed = compile(&module(
        IMPORTS,
        EFFECTS,
        "        first_result |> print dec;\n        first_result |> log info hex;\n        flush;",
    ))
    .unwrap();
    let composed_c = composed.c_source.as_ref().unwrap();
    let inserted = [
        "    QCORE_IO_WRITE(stdout, \"%\" PRId32 \"\\n\", s_6.value.scalar);",
        "    QCORE_IO_WRITE(stderr, \"[INFO] 0x%08\" PRIX32 \"\\n\", (uint32_t)(s_6.value.scalar));",
        "    QCORE_IO_FLUSH(stderr);",
    ];
    for statement in inserted {
        assert!(
            composed_c.contains(statement),
            "the composed emission is missing {statement}"
        );
    }
    let plain_c = plain.c_source.as_ref().unwrap();
    for diagnostic in [
        "QCORE_DIAG_TASK_SPAWN",
        "QCORE_DIAG_TASK_JOIN",
        "QCORE_DIAG_TASK_RESULT",
    ] {
        assert!(plain_c.contains(diagnostic));
        assert!(composed_c.contains(diagnostic));
    }
    assert!(!plain_c.contains("QCORE_IO_WRITE("));
}

// ---------------------------------------------- the concurrency exclusions kept

#[test]
fn extern_calls_beside_a_spawn_keep_the_task_routes_refusal() {
    // `STD-IO-COMPOSE` recorded these two exclusions and this card does not lift
    // them: the conjunction excludes a module that also extern-calls, so both
    // shapes fall through to the task route exactly as before.
    const EXTERN_AND_SPAWN: &str = "#import std.concurrency;
#import std.extern;
concurrency checked;
extern fn qcore_extern_scale(value: i32) -> i32;
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    qcore_extern_scale(first_value);
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
        return first_result;
    }
}
";
    assert!(
        codes(EXTERN_AND_SPAWN).contains(&"Q600".to_owned())
            || codes(EXTERN_AND_SPAWN).contains(&"Q500".to_owned()),
        "extern + spawn no longer keeps a task-route refusal: {:?}",
        codes(EXTERN_AND_SPAWN)
    );

    let all_three = EXTERN_AND_SPAWN
        .replace(
            "#import std.extern;",
            "#import std.extern;\n#import std.io;\n#import std.fmt;",
        )
        .replace(
            "        let first_result = join move first?;",
            "        let first_result = join move first?;\n        first_result |> print dec;",
        )
        .replace(
            "effects { throws<SpawnError>",
            "effects { io, throws<SpawnError>",
        );
    let all_three = codes(&all_three);
    assert!(
        !all_three.is_empty(),
        "print + extern-call + spawn is admitted, which this card does not do"
    );
    assert!(
        all_three.iter().all(|code| code != "Q050"),
        "print + extern-call + spawn reached the composed task route: {all_three:?}"
    );
}

// ------------------------------------------------------- the admitted vocabulary

#[test]
fn every_rendering_level_and_barrier_is_admitted_in_the_parent() {
    let module = compile(&module(
        IMPORTS,
        EFFECTS,
        "        first_result |> print dec;
        first_result |> print hex;
        first_result |> log info dec;
        first_result |> log warn hex;
        log fatal dec -1;
        print dec 7;
        flush;",
    ))
    .unwrap();
    let composed = module.checked.as_composed_tasks().unwrap();
    assert_eq!(composed.sites().len(), 7);
    assert_eq!(composed.certificate().flushes, 1);
    assert_eq!(composed.certificate().levels.len(), 3);
    assert_eq!(composed.certificate().renderings.len(), 2);
    assert_eq!(composed.certificate().streams.len(), 2);
}

#[test]
fn a_line_before_any_spawn_is_admitted() {
    // The conservative reading of the boundary, kept as its own pin: a line in
    // the parent before the scope is opened.
    let source = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    first_value |> log info dec;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
        return first_result;
    }
}
";
    let module = compile(source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
}

// --------------------------------------------------------------- the refusals

#[test]
fn a_line_in_a_worker_body_refuses() {
    let source = module(IMPORTS, EFFECTS, "        first_result |> print dec;").replace(
        "fn worker(value: i32) -> i32 { return value; }",
        "fn worker(value: i32) -> i32 { value |> print dec; return value; }",
    );
    assert_eq!(codes(&source), vec!["Q050"]);
}

#[test]
fn a_borrowed_render_refuses() {
    let source = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }
fn main() -> i32 effects { io, alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    log info dec borrow buffer;
    thread scope context {
        match spawn context worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
";
    assert_eq!(codes(source), vec!["Q050"]);
}

#[test]
fn rendering_something_that_is_not_an_i32_refuses() {
    // A resource, a handle and a result each refuse at the composition rather
    // than being rendered as a number no model computed. The custody discipline
    // is the same one `STD-MODULES` established, over the task value namespace:
    // there is no operand form that could carry a buffer, a loan or a handle.
    let buffer = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }
fn main() -> i32 effects { io, alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    log info dec buffer;
    thread scope context {
        match spawn context worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
";
    assert_eq!(codes(buffer), vec!["Q050"]);

    let handle = module(IMPORTS, EFFECTS, "").replace(
        "        let first_result = join move first?;",
        "        first |> log info dec;\n        let first_result = join move first?;",
    );
    assert_eq!(codes(&handle), vec!["Q050"]);
}

#[test]
fn rendering_a_name_nothing_binds_refuses_at_name_resolution() {
    // Measured, and the third route to record it: name resolution runs over the
    // parsed body before any lowering, so a use before its binding is caught
    // there. The composition's own operand rule is kept as the fail-closed
    // backstop.
    let source = module(IMPORTS, EFFECTS, "        nothing |> log info dec;");
    assert_eq!(codes(&source), vec!["Q013"]);
}

#[test]
fn writing_without_declaring_io_refuses_at_the_join() {
    let source = module(
        IMPORTS,
        TASK_EFFECTS,
        "        first_result |> log info dec;",
    );
    assert_eq!(codes(&source), vec!["Q050"]);
}

#[test]
fn each_capability_still_needs_its_own_import() {
    for (imports, body) in [
        (
            "#import std.fmt;\n#import std.logger;\n",
            "        first_result |> print dec;",
        ),
        (
            "#import std.fmt;\n#import std.io;\n",
            "        first_result |> log info dec;",
        ),
        ("#import std.fmt;\n#import std.io;\n", "        flush;"),
        (
            "#import std.io;\n#import std.logger;\n",
            "        first_result |> print dec;",
        ),
    ] {
        let source = module(imports, EFFECTS, body);
        assert_eq!(
            codes(&source),
            vec!["Q015"],
            "the import gate no longer runs before this route: {source}"
        );
    }
}

#[test]
fn a_shape_this_grammar_does_not_model_keeps_the_standard_io_routes_refusal() {
    // The composition states no grammar of its own: the three line-producing
    // statements are read by `source.std-io.v1`'s own parser, so a malformed one
    // refuses under that route's code at that route's own site.
    for body in [
        "        flush 1;",
        "        print dec;",
        "        log dec first_result;",
        "        print first_result;",
    ] {
        let source = module(IMPORTS, EFFECTS, body);
        assert_eq!(
            codes(&source),
            vec!["Q030"],
            "expected the standard-IO shape refusal for `{body}`"
        );
    }
}

#[test]
fn an_unreserved_word_in_a_rendering_position_refuses_at_name_resolution() {
    // Carried over from `STD-MODULES` §6.2 unchanged: an unreserved word is
    // collected as a referenced name and refuses earlier. Fail-closed, and
    // recorded rather than re-attributed.
    let source = module(IMPORTS, EFFECTS, "        print oct first_result;");
    assert_eq!(codes(&source), vec!["Q013"]);
}

// ------------------------------------------------------------- native evidence

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn the_composed_program_matches_the_reference_per_stream() {
    use std::fs;
    use std::process::Command;

    let module = compile(&module(
        IMPORTS,
        EFFECTS,
        "        first_result |> log info dec;
        first_result |> print hex;
        log warn dec -1;
        flush;",
    ))
    .unwrap();
    let composed = module.checked.as_composed_tasks().unwrap();
    let (out, err) = execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap()
    .projections();

    let directory =
        std::env::temp_dir().join(format!("qcore-task-io-compose-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    fs::write(
        &c,
        diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
    )
    .unwrap();
    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-g",
        "-O0",
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
    ]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let leaks = if cfg!(target_os = "linux") { 1 } else { 0 };
    let output = Command::new(&executable)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), out);
    assert_eq!(String::from_utf8(output.stderr).unwrap(), err);
    fs::remove_dir_all(directory).unwrap();
}
