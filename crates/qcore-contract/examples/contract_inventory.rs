//! Card CONTRACT-LOCKS (user decision 1 of 2026-10-24): print, as JSON, the
//! parse-derived inventory of every module of every crate on a contract load
//! path. `providers/contracts.py` runs this to generate (`sync`) and check
//! (`check`) each crate's `contract-inventory.lock`, beside the SHA-256 it
//! computes itself and the views the load path records;
//! `compiler/tests/qcore_contract_corpus.rs` recomputes the same facts with its
//! own reading and compares them with the locks.
//!
//! Per module (logical path, relative to the crate's `contracts/`):
//! - `declarations`: every `extern fn` declaration in source order, with the
//!   number of clauses of its **elaborated** contract (what the loaders load);
//! - `inherits`: every `contract inherit` site as written, `(child, parent)`;
//! - `premises`: `qcore_contract::contract_loader::module_premises` of the
//!   elaborated text, function to premise names and origins in order;
//! - `premise_bindings`: every `// qcore-premise-binding-manifest-v1
//!   DECLARATION PREMISE SHA256` line of the written text, in order;
//! - per declaration, `resource_table` (card CONTRACT-COUNT-PINS-FROM-LOCKS):
//!   the table of the resource-contract environment the declaration lands in
//!   when the resource-contract v1 loader loads the module alone
//!   (`functions`, `pending_review`, `relational` or `retaining`), or `null`
//!   when that loader refuses the module; and `resource_mode`, the loaded
//!   function-map contract (serialized `ResourceFunctionContract`) of a
//!   `functions` declaration, `null` for every other;
//! - `other_declarations` (same card): the names of the module's
//!   declarations that are not `extern fn` (`resource type`, `error`, `quant
//!   axis`, ...), in source order.
//!
//! The aggregate counts the corpus tests used to pin as literals (modules,
//! resource-v1 and rejected declarations, the loader's tables) are sums of
//! these fields (`tests/support/contract_locks.rs`).
//!
//! A module that does not parse, elaborate or read its premises is an error,
//! never an empty record.

use std::{collections::BTreeMap, path::PathBuf};

use qcore_contract::{
    contract_inclusion::elaborate_contract_module,
    contract_loader::{load_resource_contract_sources, module_premises},
    load_path_manifest::load_contract_load_path_manifest,
};
use qcore_frontend::{
    contract_inclusion::{contains_contract_inclusion_syntax, scan_contract_inclusion},
    frontend::{DeclarationKind, parse_source_program},
};
use serde_json::{Value, json};

const BINDING_MARKER: &str = "// qcore-premise-binding-manifest-v1 ";

/// A declaration's resource table and, in the function map, its mode.
type TableAndMode = (&'static str, Value);

/// Card CONTRACT-COUNT-PINS-FROM-LOCKS: the resource-contract table each
/// `extern fn` of a module lands in when the resource-contract v1 loader loads
/// the module alone, with its function-map mode (the loaded
/// `ResourceFunctionContract`, serialized) when that table is `functions`; or
/// `None` when the loader refuses the module. A declared name in no table or
/// in two is an error, never a guess.
fn resource_tables(
    logical: &str,
    text: &str,
    names: &[String],
) -> Result<Option<BTreeMap<String, TableAndMode>>, String> {
    let Ok(contracts) =
        load_resource_contract_sources(&BTreeMap::from([(logical.to_owned(), text.to_owned())]))
    else {
        return Ok(None);
    };
    let mut tables = BTreeMap::new();
    for name in names {
        let found = [
            ("functions", contracts.functions().contains_key(name)),
            (
                "pending_review",
                contracts.pending_review().contains_key(name),
            ),
            ("relational", contracts.relational().contains_key(name)),
            ("retaining", contracts.retaining().contains_key(name)),
        ]
        .into_iter()
        .filter_map(|(table, present)| present.then_some(table))
        .collect::<Vec<_>>();
        let [table] = found[..] else {
            return Err(format!(
                "{name} is in the resource tables {found:?}, not exactly one"
            ));
        };
        let mode = match contracts.functions().get(name) {
            Some(contract) => serde_json::to_value(contract).map_err(|e| e.to_string())?,
            None => Value::Null,
        };
        tables.insert(name.clone(), (table, mode));
    }
    Ok(Some(tables))
}

fn module_record(
    load_path: &qcore_contract::contract_inclusion::ContractLoadPath,
    krate: &str,
    logical: &str,
    written: &str,
) -> Result<Value, String> {
    let at = |error: String| format!("{krate}:{logical}: {error}");
    let text = if contains_contract_inclusion_syntax(written) {
        elaborate_contract_module(load_path, krate, logical)
            .map_err(|e| at(e.to_string()))?
            .text
    } else {
        written.to_owned()
    };
    let parsed = parse_source_program(&text).map_err(|e| at(e.to_string()))?;
    let mut declarations = Vec::new();
    let mut other_declarations = Vec::new();
    for declaration in &parsed.declarations {
        if declaration.value.kind != DeclarationKind::ExternFunction {
            other_declarations.push(declaration.value.name.clone());
            continue;
        }
        let contract = declaration
            .value
            .function_contract
            .as_ref()
            .ok_or_else(|| at(format!("{} has no contract", declaration.value.name)))?;
        declarations.push((declaration.value.name.clone(), contract.clauses.len()));
    }
    let names = declarations
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let tables = resource_tables(logical, &text, &names).map_err(at)?;
    let declarations = declarations
        .into_iter()
        .map(|(name, clauses)| {
            let (table, mode) = match tables.as_ref() {
                Some(tables) => (Some(tables[&name].0), tables[&name].1.clone()),
                None => (None, Value::Null),
            };
            json!({
                "name": name,
                "clauses": clauses,
                "resource_table": table,
                "resource_mode": mode,
            })
        })
        .collect::<Vec<_>>();
    let mut inherits = Vec::new();
    let syntax = scan_contract_inclusion(written).map_err(|e| at(e.to_string()))?;
    for site in &syntax.inherits {
        // The child is the `extern fn` whose signature the `contract` token
        // closes.
        let head = &written[..site.contract_span.start];
        let start = head
            .rfind("extern fn ")
            .ok_or_else(|| at("an inheriting site names no declaration".into()))?;
        let child = head[start + "extern fn ".len()..]
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()
            .unwrap_or_default();
        let parent = match &site.namespace {
            Some(namespace) => format!("{namespace}::{}", site.parent),
            None => site.parent.clone(),
        };
        inherits.push(json!({"child": child, "parent": parent}));
    }
    let mut premises = BTreeMap::new();
    for (function, list) in module_premises(&text).map_err(at)? {
        let list = list
            .iter()
            .map(|p| json!({"name": p.name, "origin": p.origin.as_str()}))
            .collect::<Vec<_>>();
        premises.insert(function, list);
    }
    let mut bindings = Vec::new();
    for line in written.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix(BINDING_MARKER) {
            let fields = rest.split_whitespace().collect::<Vec<_>>();
            let [declaration, premise, sha256] = fields[..] else {
                return Err(at(format!("malformed premise binding pin `{line}`")));
            };
            bindings.push(json!({
                "declaration": declaration,
                "premise": premise,
                "sha256": sha256,
            }));
        }
    }
    Ok(json!({
        "declarations": declarations,
        "inherits": inherits,
        "premises": premises,
        "premise_bindings": bindings,
        "other_declarations": other_declarations,
    }))
}

fn main() -> Result<(), String> {
    let usage = "usage: contract_inventory LOAD_PATH.json";
    let mut arguments = std::env::args_os().skip(1);
    let load_path = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let manifest =
        load_contract_load_path_manifest(&PathBuf::from(load_path)).map_err(|e| e.to_string())?;
    let mut crates = BTreeMap::new();
    for krate in manifest.load_path.crates() {
        let mut modules = BTreeMap::new();
        for (logical, written) in &krate.modules {
            modules.insert(
                logical.clone(),
                module_record(&manifest.load_path, &krate.name, logical, written)?,
            );
        }
        crates.insert(krate.name.clone(), json!({"modules": modules}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema_version": 2, "crates": crates}))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}
