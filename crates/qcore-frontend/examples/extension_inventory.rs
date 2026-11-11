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

//! Tracked source inventory (legacy and current Quirin suffixes). Run from a checkout root:
//! `cargo run -q -p qcore-frontend --example extension_inventory > mapping.tsv`.
//! The index supplies paths and blob identities; the language parsers supply modes.

use qcore_core::native_library::load_native_library_manifest;
use qcore_frontend::contract_inclusion::scan_contract_inclusion;
use qcore_frontend::frontend::{SourceModuleMode, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use std::fs;
use std::process::Command;

fn field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

// A proposed path for a parser refusal is a review candidate, never a parsed
// mode. The token stream lets a reader see the explicit header separately.
fn refused_candidate(stem: &str, source: &str) -> (String, &'static str) {
    let Ok(tokens) = lex(source) else {
        return (String::new(), "unresolved");
    };
    let header = matches!(tokens.first().map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "module")
        && matches!(
            tokens.get(2).map(|token| &token.kind),
            Some(TokenKind::Symbol(Symbol::Semicolon))
        );
    if header
        && matches!(tokens.get(1).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "contract")
    {
        (format!("{stem}.qv"), "declared_header_candidate")
    } else if header
        && matches!(tokens.get(1).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "library" || word == "executable")
    {
        (format!("{stem}.qr"), "declared_header_candidate")
    } else {
        (format!("{stem}.qr"), "modeless_candidate")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--stage",
            "-z",
            "--",
            "*.qcore",
            "*.qcorelib",
            "*.qv",
            "*.qr",
            "*.qrlib",
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!("git ls-files failed: {}", output.status).into());
    }
    if output.stdout.is_empty() {
        return Err("no tracked Quirin inputs; run from a checkout root".into());
    }
    println!("old_path\tindex_blob\tparser_result\tproposed_new_path\tdecision\tarchive\tdetail");
    for entry in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let entry = std::str::from_utf8(entry)?;
        let (metadata, path) = entry.split_once('\t').ok_or("malformed git ls-files row")?;
        let mut parts = metadata.split_whitespace();
        let _file_mode = parts.next().ok_or("missing index mode")?;
        let blob = parts.next().ok_or("missing index blob")?;
        let stage = parts.next().ok_or("missing index stage")?;
        if stage != "0" {
            return Err(format!("unmerged tracked input: {path}").into());
        }
        let source = fs::read_to_string(path)?;
        let archive = path.starts_with("docs/reports/");
        let (result, new_path, decision, detail) = if let Some(stem) = path
            .strip_suffix(".qcorelib")
            .or_else(|| path.strip_suffix(".qrlib"))
        {
            match load_native_library_manifest(&source) {
                Ok(_) => (
                    "abi_manifest",
                    format!("{stem}.qrlib"),
                    "parsed",
                    String::new(),
                ),
                Err(error) => (
                    "abi_parse_error",
                    String::new(),
                    "unresolved",
                    format!("{error:?}"),
                ),
            }
        } else {
            let stem = path
                .strip_suffix(".qcore")
                .or_else(|| path.strip_suffix(".qv"))
                .or_else(|| path.strip_suffix(".qr"))
                .ok_or("unexpected suffix")?;
            match parse_source_program(&source) {
                Ok(program) => {
                    let (mode, suffix) = match program.mode {
                        SourceModuleMode::Contract => ("contract", ".qv"),
                        SourceModuleMode::Executable => ("executable", ".qr"),
                        SourceModuleMode::Library => ("library", ".qr"),
                    };
                    (mode, format!("{stem}{suffix}"), "parsed", String::new())
                }
                Err(error) => {
                    let (candidate, decision) = refused_candidate(stem, &source);
                    let detail = format!(
                        "{} at {}..{}",
                        error.message, error.span.start, error.span.end
                    );
                    if scan_contract_inclusion(&source).is_ok_and(|syntax| !syntax.is_empty()) {
                        (
                            "contract_inclusion_syntax",
                            format!("{stem}.qv"),
                            "syntax_parsed_loader_resolution_required",
                            detail,
                        )
                    } else {
                        ("source_parse_error", candidate, decision, detail)
                    }
                }
            }
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            field(path),
            blob,
            result,
            field(&new_path),
            decision,
            archive,
            field(if detail.is_empty() { "-" } else { &detail })
        );
    }
    Ok(())
}
