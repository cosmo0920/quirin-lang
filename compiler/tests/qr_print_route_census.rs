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

//! Card `QR-PRINT-ROUTE-MATCHES-DECLARATION`: the route-selection census.
//!
//! Every tracked Quirin source in the repository (`.qr`, `.qv`, and the
//! legacy `.qcore`/`.qcorelib`, listed with `git ls-files` per constraint 29)
//! is compiled as a root with filesystem import loading, once without a
//! recorded target and once with a 64-bit one, and its outcome is written as
//! one JSON line: the module mode, execution profile and checked-module
//! variant (which name the route), the entry, the counts, the emitted C in
//! full, and on refusal every formatted diagnostic. It also records whether
//! the file's lexical tokens hold the pair `print (` (the token test the card
//! replaces).
//!
//! The census is the card's BEFORE/AFTER evidence that no existing source
//! changes route or output: it is run on the base tree and on the card's tree
//! and the two streams are compared byte for byte
//! (`docs/reports/qr-print-route-matches-declaration/scripts/census_compare.py`).
//! The second test (always run) pins the selector's classification of every
//! tracked source holding the pair. The census writes a file only when
//! `QR_ROUTE_CENSUS_OUT` names one, so it is `#[ignore]`d and run on purpose:
//!
//! ```sh
//! QR_ROUTE_CENSUS_OUT=private/qrpr/census-NAME.jsonl \
//!   cargo test -p qcore-compiler --test qr_print_route_census -- --ignored
//! ```
use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CheckedSourceModule, CompileOptions, CompiledModule, FileDiagnostic, SourceFile,
    compile_files_with_filesystem_for_target, format_diagnostic, parse_source_program,
};
use std::io::Write;
use std::path::{Path, PathBuf};

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate sits directly under the repository root")
        .to_path_buf()
}

/// The tracked Quirin sources, sorted (constraint 29: `git ls-files`, never a
/// walk). Submodule content is not listed by the helper.
fn tracked_sources(root: &Path) -> Vec<PathBuf> {
    tracked_files::tracked_files(root)
        .into_iter()
        .filter(|path| {
            path.extension().is_some_and(|extension| {
                ["qr", "qv", "qcore", "qcorelib"]
                    .iter()
                    .any(|wanted| extension == *wanted)
            })
        })
        .collect()
}

fn variant(checked: &CheckedSourceModule) -> &'static str {
    match checked {
        CheckedSourceModule::ScalarCalls(_) => "ScalarCalls",
        CheckedSourceModule::Sequential(_) => "Sequential",
        CheckedSourceModule::ScopedTasks(_) => "ScopedTasks",
        CheckedSourceModule::BitfieldTasks(_) => "BitfieldTasks",
        CheckedSourceModule::CheckedMutexGuards(_) => "CheckedMutexGuards",
        CheckedSourceModule::CheckedEpochBound(_) => "CheckedEpochBound",
        CheckedSourceModule::ComposedEpochTasks(_) => "ComposedEpochTasks",
        CheckedSourceModule::ExternCalls(_) => "ExternCalls",
        CheckedSourceModule::StandardIo(_) => "StandardIo",
        CheckedSourceModule::Composed(_) => "Composed",
        CheckedSourceModule::ComposedTasks(_) => "ComposedTasks",
        CheckedSourceModule::StandardBuffer(_) => "StandardBuffer",
        CheckedSourceModule::Event(_) => "Event",
        CheckedSourceModule::StandardString(_) => "StandardString",
        CheckedSourceModule::StandardSocket(_) => "StandardSocket",
        CheckedSourceModule::StandardHttp(_) => "StandardHttp",
        CheckedSourceModule::Ffi(_) => "Ffi",
    }
}

fn outcome(result: Result<CompiledModule, Vec<FileDiagnostic>>) -> serde_json::Value {
    match result {
        Ok(module) => serde_json::json!({
            "outcome": "admitted",
            "mode": format!("{:?}", module.mode),
            "profile": format!("{:?}", module.execution_profile),
            "checked": variant(&module.checked),
            "functions": module.checked.function_count(),
            "declarations": module.declaration_count,
            "result_profile": format!("{:?}", module.result_profile),
            "entry": module.entry,
            "target_assumption": module.target_assumption().map(|target| format!("{target:?}")),
            "c_source": module.c_source,
        }),
        Err(diagnostics) => serde_json::json!({
            "outcome": "refused",
            "diagnostics": diagnostics.iter().map(format_diagnostic).collect::<Vec<_>>(),
        }),
    }
}

#[test]
#[ignore = "writes the route census only when QR_ROUTE_CENSUS_OUT is set; run on purpose"]
fn route_census_of_every_tracked_source() {
    let Some(out) = std::env::var_os("QR_ROUTE_CENSUS_OUT") else {
        return;
    };
    let root = repository_root();
    let target = RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "clang 21".into(),
    };
    let mut lines = Vec::new();
    for path in tracked_sources(&root) {
        let relative = path
            .strip_prefix(&root)
            .expect("a tracked path lies under the root")
            .display()
            .to_string();
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let lexical_print_pair = parse_source_program(&source).ok().map(|parsed| {
            parsed.lexical_tokens().windows(2).any(|pair| {
                matches!(&pair[0].kind, qcore_compiler::lexer::TokenKind::Word(word) if word == "print")
                    && pair[1].kind
                        == qcore_compiler::lexer::TokenKind::Symbol(
                            qcore_compiler::lexer::Symbol::LeftParen,
                        )
            })
        });
        for (label, recorded) in [("no-target", None), ("target-64", Some(&target))] {
            let file = SourceFile::new(path.display().to_string(), source.clone());
            let result = std::panic::catch_unwind(|| {
                compile_files_with_filesystem_for_target(
                    std::slice::from_ref(&file),
                    &CompileOptions::default(),
                    recorded,
                )
            });
            let mut row = match result {
                Ok(result) => outcome(result),
                Err(_) => serde_json::json!({ "outcome": "panicked" }),
            };
            row["path"] = relative.clone().into();
            row["config"] = label.into();
            row["lexical_print_pair"] = serde_json::json!(lexical_print_pair);
            lines.push(serde_json::to_string(&row).expect("a census row serializes"));
        }
    }
    let mut file = std::fs::File::create(&out).expect("the census output is writable");
    for line in lines {
        writeln!(file, "{line}").expect("the census row is written");
    }
}

/// The selector over every tracked source (always run). Every file whose
/// tokens hold the pair `print (` is classified; the files where the pair is a
/// statement-head call in a function body, and the files where it is not a
/// call at all (a declaration name), are pinned by name, and no tracked file
/// is ambiguous. A new file holding the pair must be added here deliberately.
#[test]
fn every_tracked_print_pair_is_classified_as_pinned() {
    use qcore_frontend::source_scalar_calls::{PrintCallSelection, print_call_selection};
    let root = repository_root();
    let mut selected = Vec::new();
    let mut declaration_only = Vec::new();
    let mut ambiguous = Vec::new();
    for path in tracked_sources(&root) {
        let source = std::fs::read_to_string(&path).expect("a tracked source is readable");
        let Ok(parsed) = parse_source_program(&source) else {
            continue;
        };
        let pair = parsed.lexical_tokens().windows(2).any(|pair| {
            matches!(&pair[0].kind, qcore_compiler::lexer::TokenKind::Word(word) if word == "print")
                && pair[1].kind
                    == qcore_compiler::lexer::TokenKind::Symbol(
                        qcore_compiler::lexer::Symbol::LeftParen,
                    )
        });
        let relative = path
            .strip_prefix(&root)
            .expect("a tracked path lies under the root")
            .display()
            .to_string();
        match print_call_selection(&parsed) {
            PrintCallSelection::Selected => selected.push(relative),
            PrintCallSelection::Ambiguous(_) => ambiguous.push(relative),
            PrintCallSelection::Absent if pair => declaration_only.push(relative),
            PrintCallSelection::Absent => {}
        }
    }
    assert_eq!(selected, ["examples/scalar/printing.qr"]);
    assert_eq!(
        declaration_only,
        [
            "docs/reports/premise-decomposition-cycle1/proposed-cjson.qv",
            "providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv",
        ]
    );
    assert!(ambiguous.is_empty(), "{ambiguous:?}");
}
