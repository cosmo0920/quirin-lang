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

//! Reproducible single-unit transport control for AUDIT-IR-CLANG-TRANSPORT.
//! Run with a JSON configuration; both modes retain the same complete AST.
use qcore_c_gateway::{legacy_c::C11TranslationUnit, typed_c_ast};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Deserialize)]
struct Config {
    mode: String,
    source: String,
    output: String,
    temporary_directory: String,
}

fn canonicalize(value: &mut Value, identities: &mut BTreeMap<String, String>) {
    match value {
        Value::String(text)
            if text.starts_with("0x") && text[2..].chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            let next = format!("identity-{}", identities.len());
            *text = identities.entry(text.clone()).or_insert(next).clone();
        }
        Value::Array(values) => {
            for value in values {
                canonicalize(value, identities);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                canonicalize(value, identities);
            }
        }
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example clang_transport_measure");
    let config: Config = serde_json::from_slice(&std::fs::read(
        std::env::args().nth(1).ok_or("configuration missing")?,
    )?)?;
    // The executable has no threads yet. Both control modes use this setting.
    unsafe {
        std::env::set_var("TMPDIR", &config.temporary_directory);
    }
    let source = std::fs::read_to_string(&config.source)?;
    let mut ast: Value = match config.mode.as_str() {
        "before" => {
            let mut child = Command::new("clang")
                .args([
                    "-x",
                    "c",
                    "-std=c11",
                    "-fsyntax-only",
                    "-Xclang",
                    "-ast-dump=json",
                    "-",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let mut stdin = child.stdin.take().ok_or("stdin missing")?;
            std::thread::scope(|scope| -> Result<Value, Box<dyn std::error::Error>> {
                let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
                let output = child.wait_with_output()?;
                writer.join().map_err(|_| "writer panic")??;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
                }
                Ok(serde_json::from_slice(&output.stdout)?)
            })?
        }
        "after" => typed_c_ast::translation_unit_ast(
            &source,
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
        )?,
        _ => return Err("unknown mode".into()),
    };
    canonicalize(&mut ast, &mut BTreeMap::new());
    let mut output = std::io::BufWriter::new(std::fs::File::create(&config.output)?);
    serde_json::to_writer(&mut output, &ast)?;
    output.flush()?;
    Ok(())
}
