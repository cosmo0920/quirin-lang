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

//! Card **W45-AUDIT-CHECKPOINTS**: `quirin-audit checkpoint` — verify,
//! decompose to YAML or TOML, recompose from text, and compose a report from
//! per-group `.qo` checkpoints.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::checkpoint::{
    Checkpoint, CheckpointReader, IdentitySection, TextFormat, decompose, parse_text,
};
use crate::compose::{ComposeOutputs, Console, compose};

/// Usage text of `quirin-audit checkpoint`.
pub const USAGE: &str = "usage: quirin-audit checkpoint verify FILE.qo...\n       \
quirin-audit checkpoint decompose FILE.qo --format yaml|toml [--output FILE]\n       \
quirin-audit checkpoint recompose TEXT --format yaml|toml --output FILE.qo\n       \
quirin-audit checkpoint compose FILE.qo... [--report PATH] [--monkey-source-report PATH] \
[--binding-directory DIR]";

/// Run a `checkpoint` subcommand; returns the process exit code.
#[must_use]
pub fn main(arguments: Vec<String>) -> ExitCode {
    let Some((command, rest)) = arguments.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let result = match command.as_str() {
        "verify" => verify(rest),
        "decompose" => decompose_command(rest),
        "recompose" => recompose_command(rest),
        "compose" => compose_command(rest),
        other => Err(format!("unknown checkpoint command `{other}`\n{USAGE}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("checkpoint refused: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Split flags with one value each from positional arguments.
fn parse(
    arguments: &[String],
    flags: &[&str],
) -> Result<(Vec<PathBuf>, std::collections::BTreeMap<String, String>), String> {
    let mut positional = Vec::new();
    let mut values = std::collections::BTreeMap::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if let Some(flag) = flags.iter().find(|flag| **flag == argument) {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| format!("{flag} requires a value"))?;
            if values.insert((*flag).to_owned(), value.clone()).is_some() {
                return Err(format!("{flag} is given twice"));
            }
            index += 2;
        } else if argument.starts_with("--") {
            return Err(format!("unknown argument `{argument}`\n{USAGE}"));
        } else {
            positional.push(PathBuf::from(argument));
            index += 1;
        }
    }
    Ok((positional, values))
}

fn verify(arguments: &[String]) -> Result<bool, String> {
    let (files, _) = parse(arguments, &[])?;
    if files.is_empty() {
        return Err(format!("verify needs at least one checkpoint\n{USAGE}"));
    }
    let mut all = true;
    for path in &files {
        let checked = (|| -> Result<String, String> {
            let mut reader = CheckpointReader::open(path)?;
            reader.verify()?;
            let identity = reader.section::<IdentitySection>("identity")?;
            Ok(format!(
                "{}: ok (format {}, schema {}, group `{}`, {} unit(s), content sha256 {})",
                path.display(),
                crate::checkpoint::FORMAT_VERSION,
                crate::checkpoint::SCHEMA_VERSION,
                identity.group.label,
                identity.group.units.len(),
                reader.content_sha256()?
            ))
        })();
        match checked {
            Ok(line) => println!("{line}"),
            Err(error) => {
                all = false;
                eprintln!("{}: refused: {error}", path.display());
            }
        }
    }
    Ok(all)
}

fn format_of(values: &std::collections::BTreeMap<String, String>) -> Result<TextFormat, String> {
    TextFormat::parse(
        values
            .get("--format")
            .ok_or_else(|| format!("--format yaml|toml is required\n{USAGE}"))?,
    )
}

fn single(files: &[PathBuf], what: &str) -> Result<PathBuf, String> {
    match files {
        [one] => Ok(one.clone()),
        _ => Err(format!("{what} takes exactly one input\n{USAGE}")),
    }
}

fn decompose_command(arguments: &[String]) -> Result<bool, String> {
    let (files, values) = parse(arguments, &["--format", "--output"])?;
    let path = single(&files, "decompose")?;
    let format = format_of(&values)?;
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let checkpoint =
        Checkpoint::from_bytes(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = decompose(&checkpoint, format)?;
    match values.get("--output") {
        Some(output) => write(Path::new(output), text.as_bytes())?,
        None => print!("{text}"),
    }
    Ok(true)
}

fn recompose_command(arguments: &[String]) -> Result<bool, String> {
    let (files, values) = parse(arguments, &["--format", "--output"])?;
    let path = single(&files, "recompose")?;
    let format = format_of(&values)?;
    let output = values
        .get("--output")
        .ok_or_else(|| format!("recompose needs --output FILE.qo\n{USAGE}"))?;
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let checkpoint =
        parse_text(&text, format).map_err(|error| format!("{}: {error}", path.display()))?;
    write(Path::new(output), &checkpoint.to_bytes()?)?;
    Ok(true)
}

fn compose_command(arguments: &[String]) -> Result<bool, String> {
    let (files, values) = parse(
        arguments,
        &["--report", "--monkey-source-report", "--binding-directory"],
    )?;
    let path = |flag: &str| values.get(flag).map(PathBuf::from);
    let (report, monkey, binding) = (
        path("--report"),
        path("--monkey-source-report"),
        path("--binding-directory"),
    );
    let summary = compose(
        &files,
        &ComposeOutputs {
            report: report.as_deref(),
            monkey_source_report: monkey.as_deref(),
            binding_directory: binding.as_deref(),
        },
        Console::Full,
    )?;
    eprintln!(
        "composed {} group(s), {} configuration(s); run verdict {} (the join of the group verdicts)",
        summary.groups, summary.units, summary.verdict
    );
    Ok(summary.gateway_failures == 0)
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}
