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

//! Explicit held-source replay of every recorded libc observation domain.
//! INPUT_ROOT CHECKOUT REPO DIAGNOSTICS INVOCATION FRESH_OUTPUT_DIRECTORY EXPECTED_GRAPH_EVIDENCE_VERSION.
//! Refused domains are retained and never counted as source-bound successes.
use qcore_c_gateway::{
    legacy_c::{C11TranslationUnit, LegacyFunctionScope, prepare_translation_unit},
    reading_site_domain::{self, HeldDomain},
    reading_site_evidence::{BINDINGS, Receipt},
};
use qcore_schemas::hypothesis::Context;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
fn read(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| e.to_string())
}
fn reconstruct_imports(
    invocation: &Value,
    sidecar: &Value,
    data: &Value,
    root: &qcore_schemas::contract::ResourceContracts,
    main_entries: &[qcore_audit::compile_db::CompileEntry],
    auxiliary_entries: &[qcore_audit::compile_db::CompileEntry],
    out: &Path,
) -> Result<
    (
        qcore_schemas::contract::ResourceContracts,
        Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
        Value,
    ),
    String,
> {
    let Some(path) = invocation["link_map"].as_str() else {
        if sidecar.get("cross_unit_link_resolution").is_some() {
            return Err("recorded cross-unit premise lacks independent actual link map".into());
        }
        return Ok((root.clone(), Vec::new(), json!({"imports":"none"})));
    };
    let links = qcore_audit::cross_unit_links::LinkMap::load(Path::new(path))?;
    if invocation["link_map_sha256"].as_str() != Some(&links.digest)
        || invocation["link_map_sha256_after"].as_str() != Some(&links.digest)
        || sidecar["cross_unit_link_resolution"]["map_sha256"].as_str() != Some(&links.digest)
        || sidecar["cross_unit_link_resolution"]["premise"].as_str() != Some(&links.premise)
    {
        return Err("actual held link-map bytes/premise differ".into());
    }
    let requested = links.requested();
    let units = data["units"].as_array().ok_or("recorded census absent")?;
    let mut definitions = Vec::new();
    let mut exports = Vec::new();
    let mut source_receipts = Vec::new();
    for u in units {
        let library = u["library"].as_str().ok_or("census library absent")?;
        let file = u["file"].as_str().ok_or("census file absent")?;
        let key = format!("{library}/{file}");
        if u["enumeration"]["completeness"] != "established" {
            return Err(format!("held complete definition census refused: {key}"));
        }
        let names: Vec<String> =
            serde_json::from_value(u["source_functions"].clone()).map_err(|e| e.to_string())?;
        definitions.extend(
            names
                .iter()
                .filter(|n| requested.contains(*n))
                .map(|n| (key.clone(), n.clone())),
        );
        let selected = links.names_for(&key);
        if selected.is_empty() {
            continue;
        }
        let entries = if library == "monkey-libevent" {
            auxiliary_entries
        } else {
            main_entries
        };
        let candidates = entries
            .iter()
            .filter(|e| e.file.ends_with(file))
            .collect::<Vec<_>>();
        let [entry] = candidates.as_slice() else {
            return Err(format!(
                "home actual DB selection ambiguous or absent: {key}"
            ));
        };
        let raw = fs::read_to_string(&entry.file).map_err(|e| e.to_string())?;
        let options = qcore_audit::compile_db::compiler_options_compatibility(entry)?;
        let unit = C11TranslationUnit {
            source: &raw,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        };
        let prepared = prepare_translation_unit(&unit);
        let pp = prepared.preprocessed_source().map_err(|e| e.to_string())?;
        if !prepared.inventory().is_complete()
            || prepared.inventory().definitions() != names.as_slice()
        {
            return Err(format!(
                "independent home complete inventory differs: {key}"
            ));
        }
        let found = qcore_c_gateway::legacy_c::export_cross_unit_homes(
            &prepared,
            root,
            &selected,
            &links.digest,
        )?;
        for export in &found {
            let records = sidecar["cross_unit_link_resolution"]["imported"]
                .as_array()
                .ok_or("recorded imports absent")?;
            let matches = records
                .iter()
                .filter(|r| r["function"].as_str() == Some(export.home.function.as_str()))
                .collect::<Vec<_>>();
            let [record] = matches.as_slice() else {
                return Err("recorded selected home absent/duplicate".into());
            };
            let id = export.home.import_identity();
            if record["home_unit"] != key
                || record["home_environment_sha256"] != id.environment_sha256
                || record["home_raw_source_sha256"] != id.raw_source_sha256
                || record["home_preprocessed_source_sha256"] != id.preprocessed_source_sha256
            {
                return Err(format!(
                    "independent export differs from actual selected home: {} actual={} expected={}",
                    export.home.function,
                    serde_json::to_string(&id).unwrap(),
                    record
                ));
            }
        }
        fs::write(
            out.join(format!("home-{}.prepared.c", source_receipts.len())),
            pp,
        )
        .map_err(|e| e.to_string())?;
        source_receipts.push(json!({"unit":key,"file":entry.file,"directory":entry.directory,"command":entry.command,"raw_sha256":qcore_schemas::sha256_hex(raw.as_bytes()),"preprocessed_sha256":qcore_schemas::sha256_hex(pp.as_bytes()),"requested":selected,"independently_exported":found.iter().map(|e|json!({"function":e.home.function,"contract":e.contract,"identity":e.home.import_identity()})).collect::<Vec<_>>()}));
        exports.extend(found.into_iter().map(|export| (key.clone(), export)));
    }
    let (environment, homes) = links.select(exports, definitions.clone(), root);
    let selected_names = homes
        .iter()
        .map(|h| h.function.clone())
        .collect::<BTreeSet<_>>();
    let recorded_names = sidecar["cross_unit_link_resolution"]["imported"]
        .as_array()
        .ok_or("recorded imports absent")?
        .iter()
        .map(|r| {
            r["function"]
                .as_str()
                .map(str::to_owned)
                .ok_or("import function absent")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if selected_names != recorded_names {
        return Err(
            "independent selected home set differs from complete actual imported set".into(),
        );
    }
    let receipt = json!({"actual_link_map":path,"sha256":links.digest,"premise":links.premise,"independent_home_source_exports":source_receipts,"retained_complete_definition_census":definitions,"census_prerequisite":"independently held recorded complete selected-unit census; other unit bodies not re-enumerated by this bounded helper","actual_link_interposition_truth":"open","run_environment_sha256":qcore_schemas::sha256_hex(&serde_json::to_vec(&environment).map_err(|e|e.to_string())?)});
    Ok((environment, homes, receipt))
}

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example libc_local_source_domains");
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 8 {
        return Err("INPUT_ROOT CHECKOUT REPO DIAGNOSTICS INVOCATION FRESH_OUTPUT_DIRECTORY EXPECTED_GRAPH_EVIDENCE_VERSION".into());
    }
    let inputs = Path::new(&args[1]);
    let checkout = Path::new(&args[2]);
    let repo = Path::new(&args[3]);
    let diagnostics = Path::new(&args[4]);
    let out = PathBuf::from(&args[6]);
    fs::create_dir(&out).map_err(|e| e.to_string())?;
    let profile_dir = inputs.join("contract/profiles/fluent-bit");
    let profile: Value = serde_json::from_slice(&read(profile_dir.join("profile.json"))?)
        .map_err(|e| e.to_string())?;
    let manifest = profile_dir.join(
        profile["resource_contract_manifest"]
            .as_str()
            .ok_or("profile manifest absent")?,
    );
    let text = String::from_utf8(read(&manifest)?).map_err(|e| e.to_string())?;
    let mut contracts =
        qcore_contract::contract_loader::load_resource_contract_manifest(&manifest)?;
    qcore_audit::contract_revision::apply_pins(
        &mut contracts,
        &qcore_audit::contract_revision::ModuleOwners::for_manifest(&manifest, &text)?,
        checkout,
    )?;
    let sidecar_bytes = read(diagnostics.with_extension("contracts.json"))?;
    let sidecar: Value = serde_json::from_slice(&sidecar_bytes).map_err(|e| e.to_string())?;
    let mut applied = qcore_schemas::contract_refutation::RefutationTable::new();
    for r in sidecar["refuted_contracts"]["entries"]
        .as_array()
        .ok_or("refutation entries absent")?
    {
        if r["applied"] == true {
            applied.insert(
                r["function"].as_str().ok_or("function absent")?.into(),
                serde_json::from_value(r["refutations"].clone()).map_err(|e| e.to_string())?,
            );
        }
    }
    if sidecar["refuted_contracts"]["table_sha256"].as_str()
        != Some(qcore_schemas::contract_refutation::table_digest(&applied).as_str())
    {
        return Err("applied refutation table differs".into());
    }
    contracts = qcore_audit::refuted_contracts::with_table(&contracts, &applied);
    let graphs = qcore_contract::hypothesis::replay(contracts.source_assumptions())?;
    let formal: Value = serde_json::from_str(BINDINGS).map_err(|e| e.to_string())?;
    let artifacts: BTreeMap<String, Vec<u8>> = formal["artifacts"]
        .as_object()
        .ok_or("artifacts absent")?
        .values()
        .map(|v| {
            let p = v["path"].as_str().ok_or("path absent")?;
            Ok((p.to_owned(), read(repo.join(p))?))
        })
        .collect::<Result<_, String>>()?;
    let invocation_bytes = read(&args[5])?;
    let invocation: Value = serde_json::from_slice(&invocation_bytes).map_err(|e| e.to_string())?;
    let configured = invocation["compile_database"].as_str().is_some();
    let database = invocation["compile_database"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or_else(|| checkout.join("build/compile_commands.json"));
    let dbbytes = read(&database)?;
    if configured
        && invocation["compile_database_sha256"].as_str()
            != Some(qcore_schemas::sha256_hex(&dbbytes).as_str())
    {
        return Err("held compile DB differs from actual invocation".into());
    }
    let entries = qcore_audit::compile_db::compile_entries(
        std::str::from_utf8(&dbbytes).map_err(|e| e.to_string())?,
    )?;
    let mut database_receipts = vec![
        json!({"path":database,"sha256":qcore_schemas::sha256_hex(&dbbytes),"selection":"main"}),
    ];
    let mut auxiliary_entries = Vec::new();
    let mut auxiliary_database = None::<PathBuf>;
    let mut auxiliary_sha = None::<String>;
    let actual_clang_spelling = if configured {
        invocation["environment"]["QCORE_CLANG"]
            .as_str()
            .ok_or("actual configured Clang selection absent")?
            .to_owned()
    } else {
        let nested_path = invocation["retained_actual_driver_receipt"]
            .as_str()
            .ok_or("actual nested pinned driver receipt path absent")?;
        let nested_bytes = read(nested_path)?;
        let nested: Value = serde_json::from_slice(&nested_bytes).map_err(|e| e.to_string())?;
        if invocation["actual_driver_receipt"] != nested
            || nested["binary_sha256"] != invocation["binary_sha256"]
        {
            return Err("actual nested receipt/executable digest differs from independently held outer receipt".into());
        }
        let command = nested["command"]
            .as_array()
            .ok_or("actual pinned command absent")?;
        let words = command
            .iter()
            .map(|v| v.as_str().ok_or("actual command word absent"))
            .collect::<Result<Vec<_>, _>>()?;
        if words.get(1) != Some(&"pinned")
            || words.first().copied() != invocation["binary"].as_str()
            || !words.contains(&checkout.to_string_lossy().as_ref())
        {
            return Err("actual pinned binary/source command does not match held target".into());
        }
        let databases = words
            .iter()
            .filter(|word| word.ends_with("compile_commands.json"))
            .copied()
            .collect::<Vec<_>>();
        let [auxiliary] = databases.as_slice() else {
            return Err(
                "actual pinned command must identify exactly one explicit auxiliary database"
                    .into(),
            );
        };
        let aux_path = PathBuf::from(auxiliary);
        if aux_path == database {
            return Err("actual main/auxiliary database identities collide".into());
        }
        let aux_bytes = read(&aux_path)?;
        let aux_hash = qcore_schemas::sha256_hex(&aux_bytes);
        for (path, sha) in [
            (&database, qcore_schemas::sha256_hex(&dbbytes)),
            (&aux_path, aux_hash.clone()),
        ] {
            let spelled = path.to_str().ok_or("database path not UTF8")?;
            if invocation["external_inputs_before"][spelled].as_str() != Some(sha.as_str())
                || invocation["external_inputs_after"][spelled].as_str() != Some(sha.as_str())
            {
                return Err(format!(
                    "actual database lexical path/hash not independently bound before and after: {spelled}"
                ));
            }
        }
        auxiliary_entries = qcore_audit::compile_db::compile_entries(
            std::str::from_utf8(&aux_bytes).map_err(|e| e.to_string())?,
        )?;
        auxiliary_database = Some(aux_path.clone());
        auxiliary_sha = Some(aux_hash.clone());
        database_receipts.push(json!({"path":aux_path,"sha256":aux_hash,"selection":"actual-monkey-libevent-command","nested_receipt_path":nested_path,"nested_receipt_sha256":qcore_schemas::sha256_hex(&nested_bytes)}));
        nested["environment"]["QCORE_CLANG"]
            .as_str()
            .ok_or("actual inner Clang spelling absent")?
            .to_owned()
    };
    let raw = read(diagnostics)?;
    let data: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let source_digest = qcore_schemas::sha256_hex(
        serde_json::to_string(contracts.source_assumptions())
            .map_err(|e| e.to_string())?
            .as_bytes(),
    );
    let compiler = qcore_c_gateway::ClangIdentity::capture()?;
    if compiler.program() != actual_clang_spelling {
        return Err(
            "actual active Clang spelling differs from held actual route environment".into(),
        );
    }

    let actual_profile = qcore_contract::profile_loader::load_profile_directory(&profile_dir)
        .map_err(|e| e.to_string())?;
    let profile_sha = actual_profile.manifest_digest().to_owned();
    let (run_contracts, run_homes, imported_environment_replay) = if configured {
        (
            contracts.clone(),
            Vec::new(),
            json!({"mode":"configured-no-import-replay"}),
        )
    } else {
        reconstruct_imports(
            &invocation,
            &sidecar,
            &data,
            &contracts,
            &entries,
            &auxiliary_entries,
            &out,
        )?
    };
    let mut dependent_records = Vec::new();
    for (ui, u) in data["units"]
        .as_array()
        .ok_or("units absent")?
        .iter()
        .enumerate()
    {
        for row in u["functions"].as_array().ok_or("functions absent")? {
            if let Some(context) = row.get("hypothesis_context") {
                let declarations = row["contract_dependencies"]["hypothesis_dependencies"]
                    .as_object()
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default();
                dependent_records.push(reading_site_domain::DependentRecord {
                    id: format!(
                        "{}:{}:{}",
                        ui,
                        u["file"].as_str().unwrap_or(""),
                        row["function"].as_str().unwrap_or("")
                    ),
                    context: serde_json::from_value(context.clone()).map_err(|e| e.to_string())?,
                    declarations,
                });
            }
        }
    }
    let mut results = Vec::new();
    let mut all_records = 0;
    let mut checked_records = 0;
    for (number, u) in data["units"]
        .as_array()
        .ok_or("units absent")?
        .iter()
        .enumerate()
    {
        let mut expected = Vec::<Receipt>::new();
        let mut contexts = BTreeMap::<String, Context>::new();
        for row in u["functions"].as_array().ok_or("functions absent")? {
            for r in row["reading_site_evidence"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let receipt: Receipt =
                    serde_json::from_value(r.clone()).map_err(|e| e.to_string())?;
                let ctx: Context = serde_json::from_value(row["hypothesis_context"].clone())
                    .map_err(|e| e.to_string())?;
                let artifact = format!(
                    "{}/{}#{}",
                    u["library"].as_str().ok_or("library absent")?,
                    u["file"].as_str().ok_or("file absent")?,
                    row["function"].as_str().ok_or("function absent")?
                );
                if ctx.artifact != artifact
                    || ctx.property != "resource-custody.v1"
                    || ctx.bounds != "existing gateway budgets; no new hypothesis discharge"
                    || ctx.evidence_version != args[7]
                {
                    return Err("actual row artifact/property/bounds or independently held binary evidence version differs".into());
                }
                if receipt.context != ctx {
                    return Err("receipt vs independently held row Context differs".into());
                }
                let deps = &row["contract_dependencies"]["hypothesis_dependencies"]
                    [&receipt.observation.declaration];
                if !receipt.atom_sha256.keys().all(|a| {
                    deps.as_array()
                        .is_some_and(|d| d.iter().any(|v| v.as_str() == Some(a)))
                }) {
                    return Err("actual consultation atom closure missing".into());
                }
                if let Some(captured) = u.get("reading_site_environments") {
                    let environments: BTreeMap<String, String> =
                        serde_json::from_value(captured["environments"].clone())
                            .map_err(|e| e.to_string())?;
                    qcore_c_gateway::reading_site_evidence::validate_observation_environment(
                        &receipt.observation,
                        &environments,
                    )?;
                }
                contexts.insert(receipt.observation.function.clone(), ctx);
                expected.push(receipt);
            }
        }
        if expected.is_empty() {
            continue;
        }
        all_records += expected.len();
        let owners = expected
            .iter()
            .map(|r| r.observation.function.clone())
            .collect::<BTreeSet<_>>();
        let declarations = expected
            .iter()
            .map(|r| r.observation.declaration.clone())
            .collect::<BTreeSet<_>>();
        let file = u["file"].as_str().ok_or("file absent")?;
        let cfg = &u["compile_configuration"];
        let uses_auxiliary = !configured && u["library"].as_str() == Some("monkey-libevent");
        let selected_entries = if uses_auxiliary {
            &auxiliary_entries
        } else {
            &entries
        };
        let selected_database = if uses_auxiliary {
            auxiliary_database
                .as_ref()
                .ok_or("actual Monkey database absent")?
        } else {
            &database
        };
        let selected_database_sha = if uses_auxiliary {
            auxiliary_sha
                .clone()
                .ok_or("actual Monkey database hash absent")?
        } else {
            qcore_schemas::sha256_hex(&dbbytes)
        };
        let candidates = selected_entries
            .iter()
            .filter(|e| {
                if configured {
                    Some(e.file.to_string_lossy().as_ref()) == cfg["file"].as_str()
                        && Some(e.directory.to_string_lossy().as_ref()) == cfg["directory"].as_str()
                        && Some(e.command.as_str()) == cfg["command"].as_str()
                } else {
                    e.file.ends_with(file)
                }
            })
            .collect::<Vec<_>>();
        let mut attempts = Vec::new();
        let mut success = false;
        for (candidate, entry) in candidates.iter().enumerate() {
            let checked = (|| -> Result<Value, String> {
                let source_bytes = read(&entry.file)?;
                let source = std::str::from_utf8(&source_bytes).map_err(|e| e.to_string())?;
                let rawsha = qcore_schemas::sha256_hex(&source_bytes);
                if configured && cfg["raw_source_sha256"].as_str() != Some(rawsha.as_str()) {
                    return Err("held raw source differs".into());
                }
                let options = if configured {
                    qcore_audit::compile_db::compiler_options(entry)?
                } else {
                    qcore_audit::compile_db::compiler_options_compatibility(entry)?
                };
                let unit = C11TranslationUnit {
                    source,
                    include_paths: options.include_paths,
                    definitions: options.definitions,
                    preprocessor_options: options.preprocessor_options,
                    dialect: options.dialect,
                    function_scope: LegacyFunctionScope::MainSourceDefinitions,
                    input: options.input,
                };
                let prepared = prepare_translation_unit(&unit);
                let pp = prepared.preprocessed_source().map_err(|e| e.to_string())?;
                let selected: Vec<String> =
                    serde_json::from_str(&contexts.values().next().unwrap().scope)
                        .map_err(|e| e.to_string())?;
                if prepared.inventory().definitions() != selected.as_slice() {
                    return Err("actual prepared selected definition inventory differs".into());
                }
                if contexts
                    .values()
                    .any(|ctx| ctx.source_identity != qcore_schemas::sha256_hex(pp.as_bytes()))
                {
                    return Err("actual preprocessing does not match held source Context".into());
                }
                let build_replay = if configured {
                    let dependencies: BTreeMap<String, String> =
                        serde_json::from_value(cfg["dependencies"].clone())
                            .map_err(|e| e.to_string())?;
                    if dependencies.get("resource_contract_sources") != Some(&source_digest)
                        || dependencies.get("source_profile") != Some(&profile_sha)
                        || cfg["compiler"]
                            != serde_json::to_value(&compiler).map_err(|e| e.to_string())?
                    {
                        return Err("actual source/profile/compiler dependencies differ from held compile task".into());
                    }
                    let task = qcore_audit::compile_db::compile_task_identity(
                        entry,
                        &unit,
                        qcore_audit::compile_db::EffectiveInput::Accepted(pp),
                        &compiler,
                        dependencies,
                    )?;
                    let digest = task.digest()?;
                    if serde_json::to_value(&task).map_err(|e| e.to_string())?
                        != cfg["compile_task_identity"]
                        || contexts.values().any(|c| {
                            c.build_identity != digest
                                || c.semantic_profile != profile_sha
                                || c.assumptions != source_digest
                        })
                    {
                        return Err("actual complete compile-task/Context preimage differs".into());
                    }
                    json!({"kind":"actual-production-compile-task-constructor","task":task,"digest":digest})
                } else {
                    let cwd = PathBuf::from(
                        invocation
                            .get("process_cwd")
                            .or_else(|| invocation.get("cwd"))
                            .and_then(Value::as_str)
                            .ok_or("historical process cwd absent")?,
                    );
                    let build = format!("{:?}", (&unit, &cwd, &compiler));
                    let digest = qcore_schemas::sha256_hex(build.as_bytes());
                    if contexts.values().any(|c| {
                        c.build_identity != digest
                            || c.semantic_profile != "legacy-pinned-resource-profile"
                            || c.assumptions != source_digest
                    }) {
                        return Err(
                            "actual historical unit/cwd/compiler build preimage differs".into()
                        );
                    }
                    let wrong = format!(
                        "{:?}",
                        (&unit, PathBuf::from("/wrong-context-cwd"), &compiler)
                    );
                    if qcore_schemas::sha256_hex(wrong.as_bytes()) == digest {
                        return Err("wrong cwd negative did not differ".into());
                    }
                    json!({"kind":"unchanged-production-historical-debug-preimage","digest":digest,"actual_recorded_process_cwd":cwd,"wrong_cwd_negative_rejected":true})
                };
                let compile=serde_json::to_string(&json!({"file":entry.file,"directory":entry.directory,"command":entry.command,"database_path":selected_database,"database_sha256":selected_database_sha,"transport":unit.input,"dialect":format!("{:?}",unit.dialect)})).unwrap();
                fs::write(
                    out.join(format!("unit-{number}-candidate-{candidate}.prepared.c")),
                    pp,
                )
                .map_err(|e| e.to_string())?;
                let caller_contracts = qcore_c_gateway::legacy_c::cross_unit_for_caller(
                    pp,
                    &run_contracts,
                    &run_homes,
                )
                .environment;
                let held = HeldDomain {
                    source: pp,
                    raw_source: &source_bytes,
                    raw_sha256: &rawsha,
                    compile_configuration: &compile,
                    expected_compile_configuration: &compile,
                    entry: owners.iter().next().unwrap(),
                    contracts: &caller_contracts,
                    selected: Some(&selected),
                    dialect: unit.dialect,
                    graphs: &graphs,
                    contexts: &contexts,
                    owners: &owners,
                    declarations: &declarations,
                    expected: &expected,
                    binding_manifest: BINDINGS.as_bytes(),
                    formal_artifacts: &artifacts,
                };
                let result = reading_site_domain::check(&held, &expected)?;
                let mut impacts = Vec::new();
                for receipt in &expected {
                    for kind in [
                        reading_site_domain::FailureKind::SourceMismatch,
                        reading_site_domain::FailureKind::RefutedPrerequisite,
                        reading_site_domain::FailureKind::FinitePredicateCounterexample,
                    ] {
                        impacts.push(result.impact(
                            &receipt.id,
                            &contexts[&receipt.observation.function],
                            kind,
                            &dependent_records,
                        )?);
                    }
                }
                let mut truncated = expected.clone();
                truncated.pop();
                if reading_site_domain::check(&held, &truncated).is_ok() {
                    return Err("truncated-domain negative did not refuse".into());
                }
                Ok(
                    json!({"checked":result,"actual_build_context_reconstruction":build_replay,"hypothetical_scope_impact_controls":impacts,"actual_compile_configuration":serde_json::from_str::<Value>(&compile).unwrap(),"held_definition_inventory":selected,"truncated_domain_negative_rejected":true}),
                )
            })();
            attempts.push(json!({"candidate":candidate,"file":entry.file,"result":checked}));
            if checked.is_ok() {
                checked_records += expected.len();
                success = true;
                break;
            }
        }
        results.push(json!({"unit_index":number,"library":u["library"],"file":file,"held_observation_count":expected.len(),"owners":owners,"declarations":declarations,"candidate_count":candidates.len(),"complete_selected_recorded_domain_reproduced":success,"attempts":attempts,"universal_atoms":"open"}));
    }
    let result = json!({"schema":"libc-local-source-domain-production.v1","diagnostics_sha256":qcore_schemas::sha256_hex(&raw),"invocation_sha256":qcore_schemas::sha256_hex(&invocation_bytes),"sidecar_sha256":qcore_schemas::sha256_hex(&sidecar_bytes),"actual_compile_database":database,"actual_compile_database_sha256":qcore_schemas::sha256_hex(&dbbytes),"independently_held_database_receipts":database_receipts,"actual_clang_spelling":actual_clang_spelling,"imported_environment_replay":imported_environment_replay,"held_recorded_observations":all_records,"source_reproduced_observations":checked_records,"all_selected_recorded_domains_reproduced":all_records>0&&all_records==checked_records,"units":results,"universal_atoms":"open","unproved_boundaries":["complete relevant C site universe","external library behavior","compiler execution attestation","whole-C extraction","Rust-Lean correspondence"]});
    fs::write(
        out.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "source-bound {checked_records}/{all_records} held observations; all failures retained; universal atoms Open"
    );
    if all_records == 0 || all_records != checked_records {
        return Err(
            "not every selected recorded domain reproduced; see retained result.json".into(),
        );
    }
    Ok(())
}
