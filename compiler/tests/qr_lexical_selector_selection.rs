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

//! Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: every token-stream route
//! selector skips a declaration's own name, a parameter's name and a contract
//! clause (`qcore_frontend::route_selection::SelectorScope`; a record's name
//! stays read by constraint 14's `AGGREGATE-SELECTOR-WORDS`), so such a name
//! never selects; a selecting use of a word the module also declares refuses
//! at selection; and a `module contract;` module is never compiled on a
//! source-only route. The census is `qr_lexical_selector_census.rs`; the Lean
//! model is `QCore.RouteSelection` (`AuditRouteSelection`).
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files, format_diagnostic, frontend::SourceModuleMode,
};

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let path = if source.starts_with("module contract;") {
        "selection.qv"
    } else {
        "selection.qr"
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

fn admitted(source: &str) -> CompiledModule {
    compile(source).unwrap_or_else(|diagnostics| panic!("{}", text(&diagnostics)))
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

/// The print card's probe (`artifacts/probe/buffer_new.qv`): refused `Q015`
/// at the declaration name before this card.
const CONTRACT_DECLARING_BUFFER_NEW: &str = "module contract;

extern fn qcore_buffer_new(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};
";

#[test]
fn a_contract_declaring_qcore_buffer_new_compiles_as_a_contract() {
    let module = admitted(CONTRACT_DECLARING_BUFFER_NEW);
    assert_eq!(module.mode, SourceModuleMode::Contract);
    assert!(module.entry.is_none());
    assert!(module.c_source.is_none());
}

#[test]
fn concurrency_words_as_declaration_names_need_no_import() {
    for word in [
        "qcore_buffer_new",
        "qcore_buffer_read",
        "qcore_buffer_write",
        "qcore_error_payload",
        "spawn",
        "join",
    ] {
        let module = admitted(&format!(
            "extern fn {word}(x: i32) -> i32;\n\nfn main() -> i32 {{\n    return 0;\n}}\n"
        ));
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::SequentialCleanupV1,
            "{word}"
        );
        let module = admitted(&format!(
            "fn {word}() -> i32 {{\n    return 0;\n}}\n\nfn main() -> i32 {{\n    return 0;\n}}\n"
        ));
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::SequentialCleanupV1,
            "{word}"
        );
    }
}

#[test]
fn an_error_named_epoch_does_not_select_the_epoch_route() {
    let module = admitted("error epoch { Failed; }\n\nfn main() -> i32 {\n    return 0;\n}\n");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}

/// Constraint 14's `AGGREGATE-SELECTOR-WORDS` stands: a record is the one
/// declaration whose name stays read, so a record named after a selector word
/// is still refused at its declaration (`compiler/tests/source_record.rs`
/// pins the `Q015` half), and a record named `epoch` still reaches the epoch
/// route, which refuses it.
#[test]
fn a_record_named_epoch_still_reaches_the_epoch_route() {
    let text = refusal(
        "type epoch { lo: i32, hi: i32 }\n\nfn one(x: i32) -> i32 {\n    return x + 1;\n}\n\nfn main() -> i32 {\n    return one(1);\n}\n",
    );
    assert!(text.contains("Q600"), "{text}");
}

#[test]
fn a_declared_qcore_acquire_does_not_deselect_the_scalar_route() {
    let module = admitted(
        "fn qcore_acquire() -> i32 {\n    return 0;\n}\n\nfn one(x: i32) -> i32 {\n    return x + 1;\n}\n\nfn main() -> i32 {\n    return one(1);\n}\n",
    );
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
}

/// The scope is not the body alone: an effect row is where a function is
/// written, and `spawn` there still requires the import (154 tracked files
/// spell `effects { …, spawn, join }`).
#[test]
fn an_effect_row_still_needs_the_concurrency_import() {
    let text = refusal("fn main() -> i32 effects { spawn, join } {\n    return 0;\n}\n");
    assert!(text.contains("Q015"), "{text}");
    assert!(text.contains("std.concurrency"), "{text}");
}

#[test]
fn a_body_use_still_selects() {
    let text = refusal(
        "error Failure { Failed; }\n\nfn main() -> i32 {\n    let buffer = qcore_buffer_new(42);\n    return 0;\n}\n",
    );
    assert!(text.contains("Q015"), "{text}");
}

#[test]
fn a_use_beside_a_declaration_of_its_word_is_refused_at_selection() {
    let text = refusal(
        "error Failure { Failed; }\n\nfn qcore_acquire() -> i32 {\n    return 0;\n}\n\nfn main() -> i32 {\n    let a = qcore_acquire(1, true, Failure);\n    drop a;\n    return 0;\n}\n",
    );
    assert!(text.contains("Q020"), "{text}");
    assert!(
        text.contains("route selection: this use of the selector word `qcore_acquire`"),
        "{text}"
    );
}

#[test]
fn the_same_use_without_the_declaration_is_routed() {
    let module = admitted(
        "error Failure { Failed; }\n\nfn main() -> i32 {\n    let a = qcore_acquire(1, true, Failure);\n    drop a;\n    return 0;\n}\n",
    );
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}

/// The generalised contract guard: a contract module that the buffer route
/// would select (a record and an entry-free module) is refused naming it.
#[test]
fn a_contract_module_is_never_compiled_on_a_source_only_route() {
    let text = refusal(
        "module contract;\n\ntype R { lo: i32, hi: i32 }\n\nfn main() -> i32 {\n    return 0;\n}\n",
    );
    assert!(text.contains("Q020"), "{text}");
    assert!(
        text.contains(
            "source.buffer.v1: a `module contract;` module declares contracts and is never compiled on this route"
        ),
        "{text}"
    );
}

/// A contract module on no source-only route still compiles (the sequential
/// route reads contract modules), with a source function beside it.
#[test]
fn a_contract_module_with_a_plain_function_still_compiles() {
    let module = admitted(
        "module contract;\n\nextern fn spawn(size: usize) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(size);\n    ensures normal resource_return_owned(nullable);\n};\n\nfn main() -> i32 {\n    return 0;\n}\n",
    );
    assert_eq!(module.mode, SourceModuleMode::Contract);
}
