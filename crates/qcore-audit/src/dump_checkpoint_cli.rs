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

//! Read-only inspector for complete binary audit checkpoints.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::checkpoint::{Checkpoint, TextCheckpoint};

pub const USAGE: &str = "usage: quirin-dump-checkpoint [--full | --section NAME] FILE.qo\n\
Validates all sections before printing JSON. The default is a compact summary.\n\
--full prints all decoded fields; --section prints one decoded section.";

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    path: PathBuf,
    full: bool,
    section: Option<String>,
}

fn parse(arguments: Vec<OsString>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut path = None;
    let mut positional_only = false;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if !positional_only && argument == "--" {
            positional_only = true;
        } else if !positional_only && argument == "--full" {
            if options.full {
                return Err("--full is given twice".into());
            }
            options.full = true;
        } else if !positional_only && argument == "--section" {
            let section = arguments.next().ok_or("--section requires a name")?;
            let section = section
                .into_string()
                .map_err(|_| "section name is not UTF-8")?;
            if !crate::checkpoint::SECTIONS.contains(&section.as_str()) {
                return Err(format!("unknown section `{section}`"));
            }
            if options.section.replace(section).is_some() {
                return Err("--section is given twice".into());
            }
        } else if !positional_only && argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown argument `{}`", argument.to_string_lossy()));
        } else if path.replace(PathBuf::from(argument)).is_some() {
            return Err("exactly one checkpoint is required".into());
        }
    }
    if options.full && options.section.is_some() {
        return Err("--full and --section cannot be combined".into());
    }
    options.path = path.ok_or("a checkpoint file is required")?;
    Ok(options)
}

fn inspect(options: Options) -> Result<String, String> {
    let bytes = std::fs::read(&options.path)
        .map_err(|error| format!("{}: {error}", options.path.display()))?;
    // Parsing validates the prelude, versions, lengths, every digest, and every
    // typed section before any decoded content is published.
    let checkpoint = Checkpoint::from_bytes(&bytes)
        .map_err(|error| format!("{}: {error}", options.path.display()))?;
    let decoded = TextCheckpoint::of(&checkpoint)?;
    let value = if options.full || options.section.is_some() {
        let mut value = serde_json::to_value(&decoded).map_err(|error| error.to_string())?;
        if let Some(section) = options.section {
            value
                .as_object_mut()
                .and_then(|fields| fields.remove(&section))
                .ok_or_else(|| format!("section `{section}` is unavailable"))?
        } else {
            value
        }
    } else {
        serde_json::json!({
            "file": options.path,
            "format_version": decoded.format_version,
            "schema_version": decoded.schema_version,
            "content_sha256": decoded.content_sha256,
            "source_revision": decoded.identity.run.source_revision,
            "checkpoint": decoded.identity.group.label,
            "units": decoded.identity.group.units,
            "result": decoded.result,
            "sections": decoded.manifest,
            "scope": "validated checkpoint contents; integrity checking is not a new safety verdict"
        })
    };
    serde_json::to_string_pretty(&value)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

#[must_use]
pub fn main(arguments: Vec<OsString>) -> ExitCode {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match parse(arguments).and_then(inspect) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("checkpoint dump refused: {error}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
