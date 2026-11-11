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

//! Actual held cJSON source -> typed projection -> adapter-owned native execution
//! -> sealed finite equations -> four proposed flows. No universal truth.
use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect, CSourceInput};
use qcore_schemas::hypothesis::Graph;
use quirin_source_analysis::cjson_local_binding as binding;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example cjson_local_source");
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("SOURCE_C HEADER FRESH_OUTPUT_DIRECTORY".into());
    }
    let out = PathBuf::from(&args[3]);
    fs::create_dir(&out).map_err(|e| e.to_string())?;
    let out = fs::canonicalize(out).map_err(|e| e.to_string())?;
    let raw = fs::read(&args[1]).map_err(|e| e.to_string())?;
    let header = fs::read(&args[2]).map_err(|e| e.to_string())?;
    fs::write(out.join("cJSON.c"), &raw).map_err(|e| e.to_string())?;
    fs::write(out.join("cJSON.h"), &header).map_err(|e| e.to_string())?;
    let source = std::str::from_utf8(&raw).map_err(|e| e.to_string())?;
    let unit = C11TranslationUnit {
        source,
        include_paths: vec![out.to_string_lossy().into_owned()],
        dialect: CCompilerDialect::C11,
        input: CSourceInput::DeclaredFile {
            spelled: "cJSON.c".into(),
            directory: out.clone(),
        },
        ..Default::default()
    };
    let build = binding::native_build_configuration(&out)?;
    let context = binding::native_context(&unit, &build)?;
    let graph: Graph = serde_json::from_str(include_str!(
        "../../../docs/reports/premise-evidence-contracts/expected-graph.json"
    ))
    .map_err(|e| e.to_string())?;
    let checked_source = binding::check_source(
        &unit,
        &qcore_schemas::sha256_hex(&raw),
        &context,
        &context,
        &graph,
        include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean"),
        binding::ENVIRONMENT,
    )?;
    let checked = binding::execute_native_domain(checked_source, &build)?;
    fs::write(
        out.join("checked.json"),
        serde_json::to_vec_pretty(&checked).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let flows = [
        "entry_and_post",
        "entry_or_post",
        "post_except_incomplete",
        "post_terminal",
    ]
    .iter()
    .map(|entry| checked.evaluate_scoped_flow(&context, entry))
    .collect::<Result<Vec<_>, _>>()?;
    fs::write(
        out.join("flows.json"),
        serde_json::to_vec_pretty(&flows).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(checked.case_count(), 11);
    println!(
        "checked11actual native heap cases and4scoped proposed flows; universal atoms and no-writer trace remain Open; {}",
        Path::new(&args[3]).display()
    );
    Ok(())
}
