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
                records.push(json!({"file":path,"declaration":d.value.name,
                    "source_sha256":qcore_schemas::sha256_hex(source.as_bytes()),
                    "graph_sha256":qcore_contract::hypothesis::digest(&graph)?,
                    "formal_ids":ids,"entry_requirements":graph.requirements,
                    "source_binding":"open", "evidence":"typed representation only"}));
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
