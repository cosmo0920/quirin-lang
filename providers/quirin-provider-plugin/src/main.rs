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

//! `quirin-plugin`: the SDK's command line (card W45-USER-CONTRACT-PLUGINS).
//!
//! ```text
//! quirin-plugin new DIR --name quirin-provider-NAME
//! quirin-plugin check CRATE_DIR...
//! quirin-plugin load-path --extends SHIPPED.json --output USER.json CRATE_DIR...
//! quirin-plugin package --executable PATH --output DIR
//! quirin-plugin verify PACKAGE_DIR
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use quirin_provider_plugin::{load_path, package, template, user_crate::UserCrate};

const USAGE: &str = "usage: quirin-plugin new DIR --name quirin-provider-NAME\n\
       quirin-plugin check CRATE_DIR...\n\
       quirin-plugin load-path --extends SHIPPED.json --output USER.json CRATE_DIR...\n\
       quirin-plugin package --executable PATH --output DIR\n\
       quirin-plugin verify PACKAGE_DIR";

fn flag(arguments: &mut Vec<String>, name: &str) -> Option<String> {
    let index = arguments.iter().position(|argument| argument == name)?;
    let value = arguments.get(index + 1).cloned();
    arguments.drain(index..(index + 2).min(arguments.len()));
    value
}

fn run(mut arguments: Vec<String>) -> Result<(), String> {
    let command = if arguments.is_empty() {
        String::new()
    } else {
        arguments.remove(0)
    };
    match command.as_str() {
        "new" => {
            let name = flag(&mut arguments, "--name").ok_or(USAGE)?;
            let [directory] = arguments.as_slice() else {
                return Err(USAGE.into());
            };
            template::instantiate(Path::new(directory), &name)?;
            println!("created {directory} ({name})");
        }
        "check" => {
            if arguments.is_empty() {
                return Err(USAGE.into());
            }
            for directory in &arguments {
                let krate = UserCrate::read(Path::new(directory))?;
                println!(
                    "{}: {} module(s), {} checker(s), {} override(s)",
                    krate.name,
                    krate.modules.len(),
                    krate.manifest.checkers.len(),
                    krate.manifest.overrides.len()
                );
            }
        }
        "load-path" => {
            let extends = flag(&mut arguments, "--extends").ok_or(USAGE)?;
            let output = PathBuf::from(flag(&mut arguments, "--output").ok_or(USAGE)?);
            if arguments.is_empty() {
                return Err(USAGE.into());
            }
            let crates = arguments
                .iter()
                .map(|directory| UserCrate::read(Path::new(directory)))
                .collect::<Result<Vec<_>, _>>()?;
            let base = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let manifest = load_path::user_load_path(&extends, base, &crates)?;
            let mut bytes =
                serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
            bytes.push(b'\n');
            std::fs::write(&output, bytes).map_err(|error| error.to_string())?;
            println!(
                "wrote {}; now run `quirin-audit plugin views --load-path {} --write`",
                output.display(),
                output.display()
            );
        }
        "package" => {
            let executable = flag(&mut arguments, "--executable").ok_or(USAGE)?;
            let output = flag(&mut arguments, "--output").ok_or(USAGE)?;
            if !arguments.is_empty() {
                return Err(USAGE.into());
            }
            let manifest = package::build(Path::new(&executable), Path::new(&output))?;
            println!(
                "packaged {} at {output} (sha256 {})",
                manifest.plugin_id, manifest.package_digest
            );
        }
        "verify" => {
            let [directory] = arguments.as_slice() else {
                return Err(USAGE.into());
            };
            let manifest = package::verify(Path::new(directory))?;
            println!(
                "verified {} ({})",
                manifest.plugin_id, manifest.package_digest
            );
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
