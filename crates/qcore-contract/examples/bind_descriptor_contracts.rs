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

//! Bind one run's native descriptor records to declared libc contracts.
//!
//! Usage: bind_descriptor_contracts RECORDS.jsonl MODULE.qcore [MODULE.qcore ...]
//!
//! The modules are loaded together with the resource-v1 loader (their logical
//! name is the file path's last two components, for example
//! `libc/unistd.qv`). Prints one JSON binding report. Exit 0 means the
//! records were bound (contradictions are part of the report); exit 1 means
//! the records or a contract shape were rejected. Never a static certificate.
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_contract::descriptor_contract_binding::bind_descriptor_contracts;
use std::{collections::BTreeMap, env, fs, path::Path, process::ExitCode};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [records_path, modules @ ..] = args.as_slice() else {
        return Err("usage: bind_descriptor_contracts RECORDS.jsonl MODULE.qcore...".into());
    };
    if modules.is_empty() {
        return Err("at least one contract module is required".into());
    }
    let mut sources = BTreeMap::new();
    for module in modules {
        let path = Path::new(module);
        let logical = format!(
            "{}/{}",
            path.parent()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        let text = fs::read_to_string(path).map_err(|e| format!("{module}: {e}"))?;
        if sources.insert(logical.clone(), text).is_some() {
            return Err(format!("duplicate module {logical}"));
        }
    }
    let contracts = load_resource_contract_sources(&sources)?;
    let records = fs::read_to_string(records_path).map_err(|e| format!("{records_path}: {e}"))?;
    let report = bind_descriptor_contracts(&records, &contracts)?;
    println!(
        "{}",
        serde_json::json!({
            "kind": "native_descriptor_contract_binding",
            "modules": sources.keys().collect::<Vec<_>>(),
            "report": report,
        })
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
