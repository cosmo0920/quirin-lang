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

//! Card `QR-PRINT-ROUTE-MATCHES-DECLARATION`: `source.scalar-calls.v1` is
//! selected by a `print (` **call** at a statement head inside a source
//! function body, never by the token pair alone; an ambiguous pair refuses at
//! the selection point; and a `module contract;` module is never compiled on
//! that route (the secondary guard at `compile_scalar_calls`).
//!
//! The Lean model is `QCore.PrintRouteSelection` (`AuditPrintRouteSelection`).
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files, format_diagnostic, frontend::SourceModuleMode,
};

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    // A `module contract;` source is a `.qv` file (`Q403` otherwise).
    let path = if source.starts_with("module contract;") {
        "selection.qv"
    } else {
        "selection.qr"
    };
    compile_files(&[SourceFile::new(path, source)], &CompileOptions::default())
}

fn refusal(source: &str) -> String {
    match compile(source) {
        Ok(module) => panic!(
            "expected a refusal, admitted on {:?}",
            module.execution_profile
        ),
        Err(diagnostics) => diagnostics
            .iter()
            .map(format_diagnostic)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// The withdrawn round-3 declaration's shape: a contract module that declares
/// a C function named `print`. Before the card it was lowered on the
/// scalar-calls route and refused at its first `extern fn` (`Q020`).
const CONTRACT_DECLARING_PRINT: &str = "module contract;

extern fn print(item: ptr<const, u8>, format: i32, hooks: ptr<const, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(item);
    ensures normal resource_borrowed(format);
    ensures normal resource_borrowed(hooks);
    ensures normal resource_return_owned(nullable);
};
";

#[test]
fn a_contract_declaring_print_compiles_as_a_contract() {
    let module = compile(CONTRACT_DECLARING_PRINT).unwrap_or_else(|diagnostics| {
        panic!(
            "{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert_eq!(module.mode, SourceModuleMode::Contract);
    assert_ne!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
    assert!(module.entry.is_none());
    assert!(module.c_source.is_none());
}

#[test]
fn a_statement_head_call_still_selects_the_route() {
    let module = compile(
        "#import std.io;\n\nfn sum(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n\nfn main() -> i32 {\n    let f = sum(20, 22);\n    print(f);\n    return f;\n}\n",
    )
    .expect("the printing example is admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
}

#[test]
fn a_call_in_a_nested_block_still_selects_the_route() {
    let module = compile(
        "#import std.io;\n\nfn main() -> i32 {\n    match 1 {\n        1 => { print(7); return 0; }\n        _ => { return 1; }\n    }\n}\n",
    );
    // Whatever the route decides about the match, it is the scalar-calls
    // route that decides it: the call after `{` is a statement head.
    match module {
        Ok(module) => {
            assert_eq!(
                module.execution_profile,
                SourceExecutionProfile::ScalarCallsV1
            )
        }
        Err(diagnostics) => assert!(
            diagnostics
                .iter()
                .any(|d| d.message.starts_with("source.scalar-calls.v1:")),
            "{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

#[test]
fn print_as_a_value_is_refused_at_selection() {
    let text =
        refusal("#import std.io;\n\nfn main() -> i32 {\n    let x = print(1);\n    return x;\n}\n");
    assert!(text.contains("Q020"), "{text}");
    assert!(text.contains("only at a statement head"), "{text}");
}

#[test]
fn a_call_beside_a_declared_print_is_refused_at_selection() {
    let text = refusal(
        "#import std.io;\n\nfn print(x: i32) -> i32 {\n    return x;\n}\n\nfn main() -> i32 {\n    print(1);\n    return 0;\n}\n",
    );
    assert!(text.contains("Q020"), "{text}");
    assert!(text.contains("also declares `print`"), "{text}");
}

#[test]
fn a_comment_or_a_declaration_name_does_not_select() {
    // `print (` only in a comment and as an extern declaration's name: no
    // call, so the module is not the scalar-calls route's (and is not refused
    // with that route's extern-declaration refusal).
    let outcome = compile(
        "// print(1);\nextern fn print(x: i32) -> i32;\n\nfn main() -> i32 {\n    return 0;\n}\n",
    );
    match outcome {
        Ok(module) => {
            assert_ne!(
                module.execution_profile,
                SourceExecutionProfile::ScalarCallsV1
            )
        }
        Err(diagnostics) => {
            let text = diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!text.contains("source.scalar-calls.v1"), "{text}");
        }
    }
}

#[test]
fn a_contract_module_is_never_compiled_on_the_scalar_route() {
    // Two source functions with a call between them select the scalar-calls
    // route by `has_scalar_calls`; in a contract module the guard refuses.
    let text = refusal(
        "module contract;\n\nfn one(x: i32) -> i32 {\n    return x + 1;\n}\n\nfn two() -> i32 {\n    return one(1);\n}\n",
    );
    assert!(text.contains("Q020"), "{text}");
    assert!(
        text.contains("a `module contract;` module declares contracts"),
        "{text}"
    );
}
