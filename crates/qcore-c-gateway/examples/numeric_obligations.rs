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

//! Card **CG20-NUMERIC-DOMAIN** measurement tool: decide the C integer
//! arithmetic property (gateway rule 11q) for the functions of one compiled
//! file, with that file's own recorded compile command.
//!
//! ```sh
//! cargo run -p qcore-c-gateway --example numeric_obligations -- \
//!     compile_commands.json src/flb_scheduler.c random_uniform [--target TRIPLE]
//! ```
//!
//! It runs the recorded compiler with the command's own flags (`-c`, `-o` and
//! the input removed; `-fsyntax-only` and a JSON AST filtered to declarations
//! whose name contains the third argument added), queries the same compiler
//! for its target integer model, and prints the production report
//! (`c_numeric_obligations::report`) as JSON. A filtered dump carries no
//! enumerator declarations, so every enumerator is its type's full range here
//! (conservative). It publishes no custody verdict.

use std::process::Command;

use qcore_c_gateway::c_numeric::TargetIntegerModel;
use qcore_c_gateway::c_numeric_obligations::report;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example numeric_obligations");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [database, file, filter, rest @ ..] = arguments.as_slice() else {
        return Err(
            "usage: compile_commands.json FILE-SUFFIX NAME-FILTER [--target TRIPLE]".into(),
        );
    };
    let target = match rest {
        [] => None,
        [flag, triple] if flag == "--target" => Some(triple.as_str()),
        _ => return Err("only `--target TRIPLE` may follow".into()),
    };
    let entries: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(database)?)?;
    let entry = entries
        .iter()
        .find(|entry| {
            entry["file"]
                .as_str()
                .is_some_and(|f| f.ends_with(file.as_str()))
        })
        .ok_or("no compile command for that file")?;
    let words = entry["command"]
        .as_str()
        .ok_or("entry without a `command`")?
        .split_whitespace()
        .collect::<Vec<_>>();
    let input = entry["file"].as_str().unwrap_or_default();
    let mut flags = Vec::new();
    let mut index = 1;
    while index < words.len() {
        match words[index] {
            "-c" => {}
            "-o" => index += 1,
            word if word == input || word.ends_with(file.as_str()) => {}
            word => flags.push(word.to_owned()),
        }
        index += 1;
    }
    let compiler = words.first().ok_or("empty command")?;
    let directory = entry["directory"].as_str().unwrap_or(".");
    let mut model_command = Command::new(compiler);
    model_command.current_dir(directory).args(&flags);
    let model = TargetIntegerModel::query(model_command, target)?;
    let mut ast_command = Command::new(compiler);
    ast_command.current_dir(directory).args(&flags);
    if let Some(target) = target {
        ast_command.arg(format!("--target={target}"));
    }
    let output = ast_command
        .args([
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter",
            "-Xclang",
            filter,
            input,
        ])
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    let declarations = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()?;
    let unit = serde_json::json!({ "kind": "TranslationUnitDecl", "inner": declarations });
    println!("{}", serde_json::to_string_pretty(&report(model, &unit))?);
    Ok(())
}
