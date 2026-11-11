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

//! Card `ROUTE-SELECTION-ORDER-RESTATED`: the driver's route order is written
//! once (`qcore-verification`'s `driver::source_routes::SOURCE_ROUTES`), and
//! the closure refusal, the contract guard, the selection refusal and the
//! lowering all read the one entry selected from it. These fixtures drive the
//! arms that read the table; the table's order, flags and closure rules are
//! pinned by its own unit tests, and the Lean model is
//! `QCore.RouteSelection`'s selector table (`AuditRouteSelection`).
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files, format_diagnostic,
    frontend::SourceModuleMode,
};

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let path = if source.starts_with("module contract;") {
        "table.qv"
    } else {
        "table.qr"
    };
    compile_files(&[SourceFile::new(path, source)], &CompileOptions::default())
}

fn text(diagnostics: &[FileDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(format_diagnostic)
        .collect::<Vec<_>>()
        .join("\n")
}

fn refusal(source: &str) -> String {
    match compile(source) {
        Ok(module) => panic!(
            "expected a refusal, admitted on {:?}",
            module.execution_profile
        ),
        Err(diagnostics) => text(&diagnostics),
    }
}

fn guard(route: &str) -> String {
    format!(
        "{route}: a `module contract;` module declares contracts and is never compiled on this route"
    )
}

/// A body that dispatches and sockets and does nothing else: the
/// socket-and-event composition entry (`SOCKET-EVENT-COMPOSE`), lowered by
/// the socket route.
const DISPATCH_AND_SOCKET: &str = "fn main() -> unit effects { io, alloc } {
    event open;
    let listener = tcp;
    bind listener;
    listen listener;
    watch inbound listener read;
    dispatch idle;
    unwatch inbound;
    event close;
    close listener;
    return;
}
";

/// The guard names the route the table would lower the module on. Before
/// this card the guard's restated list had no entry for the composition and
/// named the next one, `source.event.v1`.
#[test]
fn a_contract_module_that_dispatches_and_sockets_is_refused_naming_the_socket_route() {
    let text = refusal(&format!(
        "module contract;\n\n#import std.socket;\n#import std.event;\n\n{DISPATCH_AND_SOCKET}"
    ));
    assert!(text.contains("Q020"), "{text}");
    assert!(text.contains(&guard("source.socket.v1")), "{text}");
    assert!(!text.contains("source.event.v1"), "{text}");
}

/// The same body in an executable module is lowered by that entry: the
/// socket route admits it (the selection the guard read is the one lowered).
#[test]
fn the_same_body_in_an_executable_module_is_lowered_on_the_socket_route() {
    let module = compile(&format!(
        "#import std.socket;\n#import std.event;\n\n{DISPATCH_AND_SOCKET}"
    ))
    .unwrap_or_else(|diagnostics| panic!("{}", text(&diagnostics)));
    assert_eq!(module.mode, SourceModuleMode::Executable);
    assert_eq!(
        format!("{:?}", module.execution_profile),
        "StandardSocketV1",
        "{:?}",
        module.execution_profile
    );
}

/// A contract module whose route is the scoped-task route is refused naming
/// it: the entry carries the guard's name, so the guard and the lowering read
/// one row.
#[test]
fn a_contract_module_on_the_task_route_is_refused_naming_it() {
    let text = refusal(
        "module contract;\n\n#import std.concurrency;\n\nfn main() -> i32 {\n    let b = qcore_buffer_new(4);\n    return 0;\n}\n",
    );
    assert!(text.contains("Q020"), "{text}");
    assert!(text.contains(&guard("the scoped-task route")), "{text}");
}

/// A contract module that only the sequential route selects is compiled
/// there: the one entry that is not source-only reads contract modules.
#[test]
fn a_contract_module_on_the_sequential_route_compiles() {
    let module = compile("module contract;\n\nfn main() -> i32 {\n    return 0;\n}\n")
        .unwrap_or_else(|diagnostics| panic!("{}", text(&diagnostics)));
    assert_eq!(module.mode, SourceModuleMode::Contract);
    assert_eq!(
        format!("{:?}", module.execution_profile),
        "SequentialCleanupV1"
    );
}

/// A closure spawned in scope `s`, joined and returned.
const CLOSURE_SCOPE: &str = "    let x = 42;\n    thread scope s {\n        let h = spawn s |copy x| -> i32 { return x; }?;\n        let r = join move h?;\n        return r;\n    }\n";

/// The closure refusal reads the selected entry's rule: the FFI entry, which
/// stands before the task entries, refuses a closure by its residual. (The
/// epoch and mutex entries' rules are driven by `closures.rs`.)
#[test]
fn a_closure_on_the_ffi_route_meets_that_entry_rule() {
    let text = refusal(&format!(
        "#import std.ffi.unsafe;\n#import std.concurrency;\nconcurrency checked;\n\nfn main() -> i32 effects {{ throws<SpawnError>, spawn, join, may_block }} {{\n{CLOSURE_SCOPE}}}\n"
    ));
    assert!(text.contains("Q600"), "{text}");
    assert!(text.contains("W48-QR-TASK-FFI-STRUCT-WORKER"), "{text}");
}

/// The field-footprint entry's rule: a layout record and a field-write
/// capture select it ahead of the task entries, and a closure beside them is
/// refused by that entry's residual.
#[test]
fn a_closure_on_the_field_footprint_route_meets_that_entry_rule() {
    let text = refusal(
        "#import std.concurrency;\nconcurrency checked;\n\ntype Pair {\n    left: field<0, 4>,\n    right: field<4, 4>\n}\n\nfn writer(value: i32) -> unit effects { mut } {\n    write value;\n    return;\n}\n\nfn main() -> unit effects { throws<SpawnError>, spawn, join, mut, may_block } {\n    let pair = Pair;\n    let x = 42;\n    thread scope workers {\n        let left = spawn workers writer(write pair.left)?;\n        let c = spawn workers |copy x| -> i32 { return x; }?;\n        let left_done = join move left?;\n        let c_done = join move c?;\n    }\n    return;\n}\n",
    );
    assert!(text.contains("Q600"), "{text}");
    assert!(text.contains("W48-QR-CLOSURE-ROUTES"), "{text}");
}

/// A closure on a task entry is hosted: the selected entry's rule lets it
/// through to the scoped-task lowering.
#[test]
fn a_closure_on_the_task_route_is_hosted() {
    let module = compile(&format!(
        "#import std.concurrency;\nconcurrency checked;\n\nfn main() -> i32 effects {{ throws<SpawnError>, spawn, join, may_block }} {{\n{CLOSURE_SCOPE}}}\n"
    ))
    .unwrap_or_else(|diagnostics| panic!("{}", text(&diagnostics)));
    assert_eq!(module.mode, SourceModuleMode::Executable);
}
