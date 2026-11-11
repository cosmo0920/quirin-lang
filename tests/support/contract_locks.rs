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

//! Card CONTRACT-COUNT-PINS-FROM-LOCKS: the aggregate contract counts the
//! corpus tests used to pin as literals outside `providers/`, derived from the
//! providers' generated `contract-inventory.lock` files and
//! `providers/contract-load-path.json`.
//!
//! One reader for every Rust test that asserts an aggregate (the compiler's
//! `contract_loader.rs`, the gateway's `neighbours.rs` and `prod_contracts.rs`,
//! the audit host's `provider_selection.rs`); `scripts/contract_corpus.py`
//! `lock_totals` is the same reader for the Python suite, field for field, and
//! both are held to the cases in `contract_locks_cases.json` beside this file.
//! A test compares what it loads or walks with these totals, so it still
//! fails when the loaded set and the locks disagree; a contract card moves
//! the totals by syncing its own lock (`python3 providers/contracts.py sync`),
//! a reviewed diff inside `providers/` (hard invariant 5). The per-file lock
//! check stays `compiler/tests/qcore_contract_corpus.rs`'s.
//!
//! The reader refuses a lock it cannot read as the load path states it: a
//! missing or unreadable lock, another schema or crate name, a resource view
//! that differs from the load path's or names no locked file, an unknown
//! resource table, or a module whose membership in the resource view
//! disagrees with its declarations' tables.
#![allow(dead_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde_json::Value;

/// The lock schema this reader reads (`INVENTORY_SCHEMA` in
/// `providers/contracts.py`).
pub const LOCK_SCHEMA: u64 = 2;
/// The resource-contract tables a declaration of a module the v1 loader
/// accepts lands in, as the lock spells them.
pub const RESOURCE_TABLES: [&str; 4] = ["functions", "pending_review", "relational", "retaining"];

/// The aggregate counts the inventory locks state. Field names are
/// `contract_corpus.LOCK_TOTAL_FIELDS`'.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LockTotals {
    /// Load-path crates (each with its lock).
    pub crates: usize,
    /// Every locked contract module (the tracked corpus, by
    /// `qcore_contract_corpus.rs`'s per-file check).
    pub corpus_modules: usize,
    /// Locked modules of `qcore-provider-*` crates.
    pub qcore_provider_modules: usize,
    /// Locked modules of `quirin-provider-*` crates.
    pub quirin_provider_modules: usize,
    /// Σ of the crates' resource views: the modules the resource-contract
    /// loader accepts, one source assumption each when all are selected.
    pub resource_view_modules: usize,
    /// `extern fn` declarations with a resource table (the v1 declarations).
    pub v1_declarations: usize,
    /// `extern fn` declarations without one, plus every declaration that is
    /// not `extern fn` (`other_declarations`).
    pub rejected_declarations: usize,
    /// v1 declarations per table.
    pub functions: usize,
    pub pending_review: usize,
    pub relational: usize,
    pub retaining: usize,
}

impl LockTotals {
    /// The totals as `(field, value)` pairs, in `LOCK_TOTAL_FIELDS` order.
    pub fn fields(&self) -> [(&'static str, usize); 11] {
        [
            ("crates", self.crates),
            ("corpus_modules", self.corpus_modules),
            ("qcore_provider_modules", self.qcore_provider_modules),
            ("quirin_provider_modules", self.quirin_provider_modules),
            ("resource_view_modules", self.resource_view_modules),
            ("v1_declarations", self.v1_declarations),
            ("rejected_declarations", self.rejected_declarations),
            ("functions", self.functions),
            ("pending_review", self.pending_review),
            ("relational", self.relational),
            ("retaining", self.retaining),
        ]
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

fn strings<'a>(value: &'a Value, what: &str) -> Result<Vec<&'a str>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{what} is not a list"))?
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or_else(|| format!("{what} holds a non-string"))
        })
        .collect()
}

/// The aggregate counts of `repository`'s inventory locks, panicking on a
/// lock [`try_lock_totals`] refuses.
pub fn lock_totals(repository: &Path) -> LockTotals {
    try_lock_totals(repository).unwrap_or_else(|error| panic!("{error}"))
}

/// The aggregate counts of `repository`'s inventory locks, one per crate of
/// `providers/contract-load-path.json`, or the refusal.
pub fn try_lock_totals(repository: &Path) -> Result<LockTotals, String> {
    try_lock_contents(repository).map(|(totals, _)| totals)
}

/// The function-map mode each lock records (`resource_mode`, the serialized
/// `ResourceFunctionContract` of a `functions` declaration), by function name,
/// panicking on a lock [`try_lock_totals`] refuses. A function-map name in
/// two locked modules is refused too.
pub fn function_modes(repository: &Path) -> BTreeMap<String, Value> {
    try_lock_contents(repository)
        .map(|(_, modes)| modes)
        .unwrap_or_else(|error| panic!("{error}"))
}

/// The totals and the function-map modes, or the refusal.
pub fn try_lock_contents(
    repository: &Path,
) -> Result<(LockTotals, BTreeMap<String, Value>), String> {
    let mut modes = BTreeMap::new();
    let load_path_file = repository.join("providers/contract-load-path.json");
    let load_path = read_json(&load_path_file)?;
    let base = load_path_file.parent().unwrap();
    let mut totals = LockTotals::default();
    for record in load_path["crates"]
        .as_array()
        .ok_or("the load path lists no crates")?
    {
        let name = record["name"]
            .as_str()
            .ok_or("a crate record names no crate")?;
        let contracts = record["contracts"]
            .as_str()
            .ok_or_else(|| format!("{name}: no contracts directory"))?;
        let lock_file = base
            .join(contracts)
            .parent()
            .unwrap()
            .join("contract-inventory.lock");
        let lock = read_json(&lock_file)?;
        if lock["schema_version"] != LOCK_SCHEMA || lock["crate"] != name {
            return Err(format!(
                "{}: not a schema {LOCK_SCHEMA} lock of {name}",
                lock_file.display()
            ));
        }
        if lock["resource_view"] != record["resource_view"] {
            return Err(format!(
                "{name}: the lock's resource view is not the load path's"
            ));
        }
        let files = lock["files"]
            .as_object()
            .ok_or_else(|| format!("{name}: the lock lists no files"))?;
        let view = strings(&lock["resource_view"], "resource_view")?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let unlocked = view
            .iter()
            .filter(|module| !files.contains_key(**module))
            .collect::<Vec<_>>();
        if !unlocked.is_empty() {
            return Err(format!("{name}: view modules {unlocked:?} are not locked"));
        }
        totals.crates += 1;
        totals.corpus_modules += files.len();
        if name.starts_with("qcore-provider-") {
            totals.qcore_provider_modules += files.len();
        } else if name.starts_with("quirin-provider-") {
            totals.quirin_provider_modules += files.len();
        }
        totals.resource_view_modules += view.len();
        for (logical, file) in files {
            let mut tables = Vec::new();
            for declaration in file["declarations"]
                .as_array()
                .ok_or_else(|| format!("{name}:{logical}: no declarations"))?
            {
                let table = match &declaration["resource_table"] {
                    Value::Null => None,
                    Value::String(table) if RESOURCE_TABLES.contains(&table.as_str()) => {
                        Some(table.as_str())
                    }
                    other => {
                        return Err(format!("{name}:{logical}: resource tables [{other}]"));
                    }
                };
                // A function-map declaration carries its loaded mode; no
                // other does.
                let mode = &declaration["resource_mode"];
                if mode.is_null() == (table == Some("functions")) {
                    return Err(format!(
                        "{name}:{logical}: a resource mode outside the function map, or none \
                         inside it"
                    ));
                }
                if table == Some("functions") {
                    let function = declaration["name"]
                        .as_str()
                        .ok_or_else(|| format!("{name}:{logical}: a declaration names nothing"))?;
                    if modes.insert(function.to_owned(), mode.clone()).is_some() {
                        return Err(format!(
                            "{name}:{logical}: {function} is in the function map twice"
                        ));
                    }
                }
                tables.push(table);
            }
            let others = strings(&file["other_declarations"], "other_declarations")?.len();
            let accepted = !tables.is_empty() && tables.iter().all(Option::is_some) && others == 0;
            if view.contains(logical.as_str()) != accepted {
                return Err(format!(
                    "{name}:{logical}: resource-view membership and resource tables disagree"
                ));
            }
            totals.rejected_declarations += others;
            for table in tables {
                let Some(table) = table else {
                    totals.rejected_declarations += 1;
                    continue;
                };
                totals.v1_declarations += 1;
                *match table {
                    "functions" => &mut totals.functions,
                    "pending_review" => &mut totals.pending_review,
                    "relational" => &mut totals.relational,
                    _ => &mut totals.retaining,
                } += 1;
            }
        }
    }
    Ok((totals, modes))
}
