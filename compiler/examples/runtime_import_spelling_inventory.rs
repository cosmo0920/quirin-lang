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

//! Record source outcomes and emitted C identities around the #import migration.
//! Input enumeration is tracked-only; this does not claim native execution.
use qcore_compiler::{CompileOptions, SourceFile, compile_files, digest::sha256_hex};
use serde_json::json;
use std::{fs, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example runtime_import_spelling_inventory");
    let tracked = Command::new("git")
        .args(["ls-files", "-z", "--", "*.qr"])
        .output()?;
    if !tracked.status.success() {
        return Err("git ls-files failed".into());
    }
    let mut rows = Vec::new();
    let mut accepted = 0;
    let mut refused = 0;
    for path in tracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|p| !p.is_empty())
    {
        let path = std::str::from_utf8(path)?;
        let source = fs::read_to_string(path)?;
        let outcome = match compile_files(
            &[SourceFile::new(path, &source)],
            &CompileOptions::default(),
        ) {
            Ok(module) => {
                accepted += 1;
                json!({"kind":"accepted", "mode":format!("{:?}",module.mode),
                    "entry":module.entry, "execution_profile":format!("{:?}",module.execution_profile),
                    "result_profile":format!("{:?}",module.result_profile),
                    "declaration_count":module.declaration_count,
                    "c_sha256":module.c_source.as_ref().map(|s|sha256_hex(s.as_bytes())),
                    "c_bytes":module.c_source.as_ref().map(String::len)})
            }
            Err(diagnostics) => {
                refused += 1;
                json!({"kind":"refused", "diagnostics":diagnostics.iter().map(|d|
                    json!({"code":d.code,"pass":format!("{:?}",d.pass),"message":d.message}))
                    .collect::<Vec<_>>()})
            }
        };
        rows.push(
            json!({"path":path,"source_sha256":sha256_hex(source.as_bytes()),"outcome":outcome}),
        );
    }
    if rows.is_empty() {
        return Err("tracked runtime source inventory is empty".into());
    }
    let tip = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    if !tip.status.success() {
        return Err("git rev-parse failed".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
        "source_tip":std::str::from_utf8(&tip.stdout)?.trim(),"files":rows.len(),
        "accepted":accepted,"refused":refused,
        "boundary":"Default snapshot compile outcomes and emitted-C hashes; source hashes intentionally change with spelling. Diagnostics compare code/pass/message, not source spans. No native or C-corpus coverage claim.",
        "rows":rows}))?
    );
    Ok(())
}
