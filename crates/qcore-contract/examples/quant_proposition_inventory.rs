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

//! Inspect typed model hypotheses only. This command grants no C call permission.
use qcore_frontend::frontend::{SourceContractClauseKind, SourceModuleMode, parse_source_program};
use serde_json::json;
use std::{collections::BTreeSet, env, fs};
fn run() -> Result<(), String> {
    let mut records = Vec::new();
    for path in env::args().skip(1) {
        let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let program = parse_source_program(&source).map_err(|e| e.to_string())?;
        if program.mode != SourceModuleMode::Contract {
            return Err("contract mode required".into());
        }
        for d in program.declarations {
            let Some(contract) = d.value.function_contract.as_ref() else {
                continue;
            };
            let mut ids = BTreeSet::new();
            for clause in &contract.clauses {
                let id = match &clause.value.kind {
                    SourceContractClauseKind::QuantCondition(selector) => {
                        qcore_frontend::quant_condition::parse(
                            selector.clone(),
                            &clause.value.expression_source,
                        )
                        .map_err(|error| {
                            format!(
                                "{path}:{}:{}: {error}",
                                d.value.name,
                                clause
                                    .value
                                    .expression_source
                                    .split('(')
                                    .next()
                                    .unwrap_or("condition")
                            )
                        })?
                        .proposition
                        .id
                    }
                    SourceContractClauseKind::QuantProposition => {
                        qcore_frontend::quant_proposition::parse(&clause.value.expression_source)?
                            .id
                    }
                    SourceContractClauseKind::QuantEffect(_) => {
                        qcore_frontend::quant_effect::parse(&clause.value.expression_source)?.id
                    }
                    SourceContractClauseKind::QuantTheorem => {
                        qcore_frontend::quant_theorem::parse(&clause.value.expression_source)?.id
                    }
                    _ => continue,
                };
                if !ids.insert(id) {
                    return Err(format!("{path}:{}: duplicate formal ID", d.value.name));
                }
            }
            if !ids.is_empty() {
                let graph = qcore_contract::hypothesis::declaration(&d.value, &[])
                    .map_err(|e| format!("{path}:{}: {e}", d.value.name))?;
                let conditions = qcore_contract::quant_condition::declaration(&d.value)?;
                records.push(
                    json!({"conditions":conditions,"file":path,"declaration":d.value.name,
                    "source_sha256":qcore_schemas::sha256_hex(source.as_bytes()),
                    "graph_sha256":qcore_contract::hypothesis::digest(&graph)?,
                    "formal_ids":ids,"entry_requirements":graph.requirements,
                    "source_binding":"open", "evidence":"typed representation only"}),
                );
            }
        }
    }
    if records.is_empty() {
        return Err("no formal hypotheses selected".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&records).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
