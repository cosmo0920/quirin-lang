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

//! `quirin-audit measure compose` (card **QUIRIN-AUDIT-CONNECT**, Parts B
//! and C): two or more contract bundles composed for one measurement or
//! verdict, into one **composition record** ([`COMPOSITION_SCHEMA`]).
//!
//! ```text
//! quirin-audit --output OUT measure compose --load-path FILE [--load-path FILE]... \
//!     [--bundle CRATE]... [--report REPORT.json] [--libc-names FILE]
//! ```
//!
//! A **bundle** is one contract crate on a load path (a `quirin-provider-*`
//! or `qcore-provider-*` crate of `providers/contract-load-path.json`, or a
//! plugin-SDK user crate of a user load path that extends it), read as data,
//! as the plugin SDK reads user crates: its load-path record (name,
//! namespace, version, declared dependencies, owned modules with their
//! SHA-256, published pins, declared contract dependencies), its
//! `contracts.lock` (every reviewed declaration with its digest and pin
//! stamp) and its `quirin-plugin.json` (checker bindings), each with its
//! digest. Nothing is parsed or analysed here; the loader stays the
//! authority for what is in force (C06.2), and this record composes what the
//! bundles declare.
//!
//! * **Declarations** (`qcore_schemas::connect::compose_bundles`): a libc/POSIX
//!   name declared by any bundle but `quirin-provider-libc` is refused
//!   (`cx_refuse_libc_redeclared`); several declarations of one function are
//!   decided by their pin stamps exactly as C06.6 decides them, and an
//!   undecided set is refused (`cx_refuse_conflicting_declarations`).
//! * **Dependencies** (Part C, `qcore_schemas::connect::resolve`): every
//!   declared contract dependency resolved over the bundles; a held one holds
//!   its dependents, a cycle is held `Unknown`.
//! * **Verdicts** (with `--report`): every function row that rests on a
//!   composed declaration or carries checker links is recomposed as the join
//!   of its own status, the standing of every declaration it rests on, every
//!   dependency hold and every checker link (each link's verdict checked
//!   against its firewall class, rule 10h), each contributor kept with its
//!   own reasons. The recomposed verdict is never below the published one; a
//!   row it raises is listed (`raised`), never lowered.
//!
//! The tool claims nothing through the firewall. The Lean model is
//! `QCore.MeasureConnect`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qcore_schemas::connect::{
    self, BundleDeclaration, CrateFacts, DeclarationFacts, DeclaredDependency, HoldClass,
    LIBC_CRATE,
};
use qcore_schemas::contract_supersession::StampStanding;
use qcore_schemas::library_version::LibraryVersion;
use qcore_schemas::verdict::Verdict;
use quirin_audit_plugin::{Request, Response, Severity};
use serde_json::{Value, json};

use crate::util::{read_json, sha_file, write_json};

/// The composition record's schema.
pub const COMPOSITION_SCHEMA: &str = "quirin-contract-composition/2";

/// The composition record's file name.
pub const COMPOSITION_FILE: &str = "composition.json";

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output OUT measure compose --load-path FILE \
[--load-path FILE]... [--bundle CRATE]... [--report REPORT.json] [--libc-names FILE]\n\
two or more bundles";

/// The parsed flags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposeOptions {
    pub load_paths: Vec<String>,
    pub bundles: Vec<String>,
    pub report: Option<String>,
    pub libc_names: Option<String>,
}

/// Parse the arguments after `measure compose`.
///
/// # Errors
///
/// An unknown, valueless or repeated flag, or no load path.
pub fn parse(arguments: &[String]) -> Result<ComposeOptions, String> {
    let mut options = ComposeOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        let value = arguments
            .get(index + 1)
            .cloned()
            .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))?;
        match flag {
            "--load-path" => options.load_paths.push(value),
            "--bundle" => {
                if options.bundles.contains(&value) {
                    return Err(format!("--bundle {value} is given twice"));
                }
                options.bundles.push(value);
            }
            "--report" if options.report.is_none() => options.report = Some(value),
            "--libc-names" if options.libc_names.is_none() => options.libc_names = Some(value),
            "--report" | "--libc-names" => return Err(format!("{flag} is given twice")),
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
        index += 2;
    }
    if options.load_paths.is_empty() {
        return Err(format!("give at least one --load-path\n{USAGE}"));
    }
    if !options.bundles.is_empty() && options.bundles.len() < 2 {
        return Err(format!("a composition joins two or more bundles\n{USAGE}"));
    }
    Ok(options)
}

/// One bundle as read.
#[derive(Clone, Debug)]
pub struct Bundle {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub root: PathBuf,
    pub dependencies: BTreeSet<String>,
    pub identity: Value,
    pub declarations: Vec<BundleDeclaration>,
    pub contract_dependencies: Vec<DeclaredDependency>,
    pub checkers: Value,
}

/// Every crate record of a load-path manifest and of the manifest it
/// extends, with the directory its paths are relative to.
fn manifest_crates(path: &Path) -> Result<Vec<(PathBuf, Value, String)>, String> {
    let manifest = read_json(path)?;
    if manifest["schema_version"] != json!(1) {
        return Err(format!("{}: not a schema 1 load path", path.display()));
    }
    let base = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let digest = sha_file(path)?;
    let mut out = Vec::new();
    if let Some(extends) = manifest["extends"].as_str() {
        out.extend(manifest_crates(&base.join(extends))?);
    }
    for record in manifest["crates"].as_array().into_iter().flatten() {
        out.push((base.clone(), record.clone(), digest.clone()));
    }
    Ok(out)
}

fn read_bundle(base: &Path, record: &Value, manifest_sha256: &str) -> Result<Bundle, String> {
    let name = record["name"]
        .as_str()
        .ok_or("a load-path crate without a name")?
        .to_owned();
    let contracts = base.join(
        record["contracts"]
            .as_str()
            .ok_or_else(|| format!("{name}: no contracts directory"))?,
    );
    let root = contracts
        .parent()
        .ok_or_else(|| format!("{name}: a contracts directory without a parent"))?
        .to_path_buf();
    let mut modules = BTreeMap::new();
    for module in record["modules"].as_array().into_iter().flatten() {
        let module = module.as_str().unwrap_or_default();
        modules.insert(module.to_owned(), sha_file(&contracts.join(module))?);
    }
    for (module, pin) in record["published"].as_object().into_iter().flatten() {
        if modules.get(module) != Some(&pin.as_str().unwrap_or_default().to_owned()) {
            return Err(format!(
                "{name}: published module {module} differs from its pin; the load path is stale"
            ));
        }
    }
    let lock_path = root.join("contracts.lock");
    let (lock, lock_sha256) = if lock_path.is_file() {
        (read_json(&lock_path)?, json!(sha_file(&lock_path)?))
    } else {
        (Value::Null, Value::Null)
    };
    let plugin_path = root.join("quirin-plugin.json");
    let (checkers, plugin_sha256) = if plugin_path.is_file() {
        (
            read_json(&plugin_path)?["checkers"].clone(),
            json!(sha_file(&plugin_path)?),
        )
    } else {
        (Value::Null, Value::Null)
    };
    let mut declarations = Vec::new();
    for table in ["pinned", "flexible", "tip"] {
        for entry in lock[table].as_array().into_iter().flatten() {
            let (Some(function), Some(module), Some(digest)) = (
                entry["function"].as_str(),
                entry["module"].as_str(),
                entry["contract_sha256"].as_str(),
            ) else {
                return Err(format!("{name}: a malformed {table} lock entry"));
            };
            declarations.push(BundleDeclaration {
                bundle: name.clone(),
                function: function.to_owned(),
                module: format!("{name}:{module}"),
                contract_sha256: digest.to_owned(),
                stamp: StampStanding::of_entry(entry["pinned_at"].as_str()),
            });
        }
    }
    let contract_dependencies: Vec<DeclaredDependency> = match record.get("contract_dependencies") {
        None => Vec::new(),
        Some(entries) => serde_json::from_value(entries.clone())
            .map_err(|error| format!("{name}: contract_dependencies: {error}"))?,
    };
    let contracts_sha256 = qcore_schemas::sha256_hex(json!(modules).to_string().as_bytes());
    Ok(Bundle {
        namespace: record["namespace"].as_str().unwrap_or_default().to_owned(),
        version: record["version"].as_str().unwrap_or_default().to_owned(),
        dependencies: record["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        identity: json!({
            "name": name,
            "namespace": record["namespace"],
            "version": record["version"],
            "load_path_sha256": manifest_sha256,
            "contracts_sha256": contracts_sha256,
            "modules": modules,
            "lock_sha256": lock_sha256,
            "plugin_manifest_sha256": plugin_sha256,
            "contract_dependencies": record.get("contract_dependencies").cloned()
                .unwrap_or_else(|| json!([])),
        }),
        name,
        root,
        declarations,
        contract_dependencies,
        checkers,
    })
}

/// Read every bundle the load paths name (all of them when `selected` is
/// empty).
///
/// # Errors
///
/// An unreadable manifest, lock or module; a crate named twice with two
/// records; a selected crate on no load path.
pub fn read_bundles(load_paths: &[PathBuf], selected: &[String]) -> Result<Vec<Bundle>, String> {
    let mut records: BTreeMap<String, (PathBuf, Value, String)> = BTreeMap::new();
    for path in load_paths {
        for (base, record, digest) in manifest_crates(path)? {
            let name = record["name"].as_str().unwrap_or_default().to_owned();
            if let Some((known_base, known, _)) = records.get(&name) {
                if *known != record || *known_base != base {
                    return Err(format!(
                        "crate {name} is on two load paths with two records; a bundle has one \
                         identity"
                    ));
                }
                continue;
            }
            records.insert(name, (base, record, digest));
        }
    }
    for name in selected {
        if !records.contains_key(name) {
            return Err(format!("--bundle {name} is on no load path"));
        }
    }
    records
        .iter()
        .filter(|(name, _)| selected.is_empty() || selected.contains(name))
        .map(|(_, (base, record, digest))| read_bundle(base, record, digest))
        .collect()
}

fn libc_names(bundles: &[Bundle], file: Option<&Path>) -> Result<Option<BTreeSet<String>>, String> {
    let path = match file {
        Some(path) => Some(path.to_path_buf()),
        None => bundles
            .iter()
            .find(|bundle| bundle.name == LIBC_CRATE)
            .map(|bundle| bundle.root.join("libc-names.txt"))
            .filter(|path| path.is_file()),
    };
    let Some(path) = path else {
        return Ok(None);
    };
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut names: BTreeSet<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    for bundle in bundles.iter().filter(|bundle| bundle.name == LIBC_CRATE) {
        names.extend(bundle.declarations.iter().map(|d| d.function.clone()));
    }
    if names.is_empty() {
        return Err(format!("{}: an empty libc name list", path.display()));
    }
    Ok(Some(names))
}

/// The composition of `bundles`, with a report's rows recomposed when given.
///
/// # Errors
///
/// No libc name list while a bundle other than the libc crate is composed
/// (ownership could not be checked), or a malformed report.
#[allow(clippy::too_many_lines)]
pub fn compose(
    bundles: &[Bundle],
    libc_file: Option<&Path>,
    report: Option<(&Path, &Value)>,
) -> Result<Value, String> {
    if bundles.len() < 2 {
        return Err("a composition joins two or more bundles".into());
    }
    let names = libc_names(bundles, libc_file)?;
    let Some(names) = names else {
        return Err(format!(
            "no libc name list ({LIBC_CRATE} is not composed and no --libc-names is given): \
             libc ownership (cx_refuse_libc_redeclared) cannot be checked"
        ));
    };
    let declarations: Vec<BundleDeclaration> = bundles
        .iter()
        .flat_map(|bundle| bundle.declarations.iter().cloned())
        .collect();
    let composed = connect::compose_bundles(&declarations, &names);
    // Part C: the environment the dependencies resolve against.
    let mut environment: BTreeMap<String, CrateFacts> = BTreeMap::new();
    for bundle in bundles {
        let mut facts = CrateFacts {
            version: LibraryVersion::parse(&bundle.version).ok(),
            dependencies: bundle.dependencies.clone(),
            declarations: BTreeMap::new(),
        };
        for declaration in &bundle.declarations {
            let entry = &composed[&declaration.function];
            let in_force = entry
                .in_force
                .as_ref()
                .is_some_and(|d| d.bundle == bundle.name);
            let refused = entry.refused.iter().any(|(d, _)| d.bundle == bundle.name);
            if in_force || refused {
                facts.declarations.insert(
                    declaration.function.clone(),
                    DeclarationFacts {
                        sha256: declaration.contract_sha256.clone(),
                        held: refused.then_some(HoldClass::Unknown),
                    },
                );
            }
        }
        environment.insert(bundle.name.clone(), facts);
    }
    let dependencies: Vec<DeclaredDependency> = bundles
        .iter()
        .flat_map(|bundle| bundle.contract_dependencies.iter().cloned())
        .collect();
    let mut static_refusals = Vec::new();
    for dependency in &dependencies {
        if let Err(why) = dependency.validate(Some(&names)) {
            static_refusals.push(json!({"dependency": dependency.id(), "why": why}));
        }
    }
    let resolution = connect::resolve(&dependencies, &environment, Some(&names));
    let holds: BTreeMap<String, Vec<(String, qcore_schemas::verdict::ReasonedVerdict)>> = {
        let mut holds: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for standing in resolution.held() {
            holds
                .entry(standing.function.clone())
                .or_default()
                .push((standing.crate_name.clone(), connect::hold_verdict(standing)));
        }
        holds
    };
    let declaration_records: BTreeMap<&String, Value> = composed
        .iter()
        .filter(|(_, entry)| {
            entry.refused.len() + entry.superseded.len() + usize::from(entry.in_force.is_some()) > 1
                || !entry.refused.is_empty()
        })
        .map(|(function, entry)| {
            (
                function,
                json!({
                    "in_force": entry.in_force,
                    "superseded": entry.superseded,
                    "refused": entry.refused.iter().map(|(d, r)| json!({"declaration": d,
                        "formula": r.formula, "detail": r.detail})).collect::<Vec<_>>(),
                    "standing": entry.standing().verdict(),
                    "reasons": entry.standing().reasons(),
                }),
            )
        })
        .collect();
    let mut rows = Vec::new();
    let mut raised = Vec::new();
    let mut misclassified = Vec::new();
    if let Some((_, report)) = report {
        for unit in report["units"].as_array().into_iter().flatten() {
            for function in unit["functions"].as_array().into_iter().flatten() {
                let name = function["function"].as_str().unwrap_or_default();
                let published: Verdict = function["status"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(Verdict::Unknown);
                let host: Verdict = function["host_status"]
                    .as_str()
                    .or_else(|| function["status"].as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(Verdict::Unknown);
                let rests_on: Vec<String> = function["contract_dependencies"]["declarations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                let mut contributors = vec![json!({"source": "host", "verdict": host})];
                for declaration in &rests_on {
                    if let Some(entry) = composed.get(declaration) {
                        let standing = entry.standing();
                        if standing.verdict() != Verdict::Safe {
                            contributors.push(json!({"source": "declarations",
                                "declaration": declaration, "verdict": standing.verdict(),
                                "reasons": standing.reasons()}));
                        }
                    }
                    for (crate_name, hold) in holds.get(declaration).into_iter().flatten() {
                        contributors.push(json!({"source": "dependency-hold",
                            "declaration": declaration, "crate": crate_name,
                            "verdict": hold.verdict(), "reasons": hold.reasons()}));
                    }
                }
                for link in function["plugin_links"].as_array().into_iter().flatten() {
                    let claim = link["answer"]["claim"].as_str().unwrap_or_default();
                    let recorded: Option<Verdict> =
                        link["verdict"].as_str().and_then(|s| s.parse().ok());
                    let expected = connect::class_verdict(claim);
                    let checked = match (recorded, expected) {
                        (Some(recorded), Some(expected)) if recorded == expected => recorded,
                        _ => {
                            misclassified.push(json!({"function": name, "link": link,
                                "formula": "cx_refuse_misclassified_record"}));
                            Verdict::Unknown
                        }
                    };
                    contributors.push(json!({"source": format!("{}:{}",
                            link["crate"].as_str().unwrap_or_default(),
                            link["plugin"].as_str().unwrap_or_default()),
                        "callee": link["callee"], "claim": claim, "verdict": checked,
                        "reasons": link["reasons"], "own_reasons": link["answer"]["reasons"]}));
                }
                if contributors.len() == 1 {
                    continue;
                }
                let joined = Verdict::fold(
                    contributors
                        .iter()
                        .filter_map(|c| c["verdict"].as_str().and_then(|s| s.parse().ok())),
                );
                let row = json!({"unit": {"library": unit["library"], "file": unit["file"]},
                    "function": name, "published": published, "composed": joined,
                    "contributors": contributors});
                if joined > published {
                    raised.push(json!({"function": name, "published": published,
                        "composed": joined, "formula": "cx_composed_bounds_every_member"}));
                }
                rows.push(row);
            }
        }
    }
    Ok(json!({
        "schema": COMPOSITION_SCHEMA,
        "tool": crate::tool(),
        "bundles": bundles.iter().map(|bundle| (bundle.name.clone(), json!({
            "identity": bundle.identity,
            "declarations": bundle.declarations.len(),
            "checkers": bundle.checkers,
        }))).collect::<BTreeMap<_, _>>(),
        "libc_owner": LIBC_CRATE,
        "declarations": {
            "functions": composed.len(),
            "in_force": composed.values().filter(|e| e.in_force.is_some()).count(),
            "refused": composed.values().map(|e| e.refused.len()).sum::<usize>(),
            "superseded": composed.values().map(|e| e.superseded.len()).sum::<usize>(),
            "shared_or_refused": declaration_records,
        },
        "dependency_graph": {
            "vocabulary": connect::LINKAGE_VOCABULARY,
            "dependencies": dependencies.iter().map(|d| json!({
                "id": d.id(), "dependent": d.dependent, "dependent_crate": d.dependent_crate,
                "target_crate": d.target_crate, "target_function": d.target_function,
                "since": d.since.map(|v| v.to_string()), "removed": d.removed.map(|v| v.to_string()),
                "sha256": d.sha256, "form": d.form.as_str(), "vocabulary": d.vocabulary,
                "link": d.link(), "clause": d.clause(),
                "target_identity": environment.get(&d.target_crate).map(|facts| json!({
                    "version": facts.version.map(|v| v.to_string()),
                    "sha256": facts.declarations.get(&d.target_function).map(|f| f.sha256.clone()),
                })),
            })).collect::<Vec<_>>(),
            "static_refusals": static_refusals,
            "cycles": resolution.cycles.iter().map(|(c, f)| format!("{c}:{f}")).collect::<Vec<_>>(),
            "held": resolution.held().map(|standing| json!({
                "crate": standing.crate_name, "function": standing.function,
                "class": standing.held, "verdict": connect::hold_verdict(standing).verdict(),
                "causes": standing.causes.iter().map(|(id, failure)| json!({"dependency": id,
                    "failure": failure.as_str(), "formula": failure.formula(), "detail": failure}))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        },
        "report": report.map(|(path, _)| json!({"path": path.display().to_string(),
            "sha256": sha_file(path).ok()})),
        "rows": rows,
        "raised": raised,
        "misclassified_records": misclassified,
        "formulas": connect::FORMULAS.iter()
            .filter(|(id, _)| id.starts_with("cx_refuse_libc") || id.starts_with("cx_refuse_conflicting")
                || id.starts_with("cx_refused") || id.starts_with("cx_composed")
                || id.starts_with("cx_every_reason") || id.starts_with("cx_uncertified")
                || id.starts_with("cx_checker") || id.starts_with("cx_refuse_misclassified")
                || id.starts_with("cx_satisfied") || id.starts_with("cx_every_check")
                || id.starts_with("cx_unsatisfied") || id.starts_with("cx_held")
                || id.starts_with("cx_cycle") || id.ends_with("_is_unknown")
                || id.starts_with("cx_digest") || id.starts_with("cx_hold")
                || id.starts_with("cx_pending_hold"))
            .map(|(id, text)| (*id, *text))
            .collect::<BTreeMap<_, _>>(),
    }))
}

/// Answer `measure compose`.
///
/// # Errors
///
/// A malformed argument, an unreadable bundle or report.
pub fn answer(request: &Request, arguments: &[String]) -> Result<Response, String> {
    let options = parse(arguments)?;
    let output = request
        .output_directory()
        .canonicalize()
        .map_err(|error| format!("{}: {error}", request.output_directory().display()))?;
    let load_paths: Vec<PathBuf> = options
        .load_paths
        .iter()
        .map(|path| request.resolve_argument(path))
        .collect();
    let bundles = read_bundles(&load_paths, &options.bundles)?;
    let report_path = options
        .report
        .as_ref()
        .map(|path| request.resolve_argument(path));
    let report = report_path
        .as_ref()
        .map(|path| crate::chain::read_record(path))
        .transpose()?;
    let libc_file = options
        .libc_names
        .as_ref()
        .map(|path| request.resolve_argument(path));
    let record = compose(
        &bundles,
        libc_file.as_deref(),
        report_path.as_deref().zip(report.as_ref()),
    )?;
    write_json(&output.join(COMPOSITION_FILE), &record)?;
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new(COMPOSITION_FILE), "contract-composition")?;
    let refused = record["declarations"]["refused"].as_u64().unwrap_or(0);
    let held = record["dependency_graph"]["held"]
        .as_array()
        .map_or(0, Vec::len);
    let static_refusals = record["dependency_graph"]["static_refusals"]
        .as_array()
        .map_or(0, Vec::len);
    if refused + held as u64 + static_refusals as u64 > 0 {
        response.push_diagnostic(
            Severity::Warning,
            "composition-refusals",
            format!(
                "{refused} refused declaration(s), {held} held dependency standing(s), \
                 {static_refusals} malformed dependency clause(s); each is named in {}",
                output.join(COMPOSITION_FILE).display()
            ),
        );
    }
    Ok(response)
}
