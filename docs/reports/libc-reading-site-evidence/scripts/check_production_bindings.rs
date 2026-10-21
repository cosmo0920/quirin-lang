//! Reference/binding replay only. This does not claim production C reanalysis.
use qcore_c_gateway::reading_site_evidence::{BINDINGS, Receipt, bind, resolve_bindings};
use qcore_contract::{contract_loader::load_resource_contract_manifest, hypothesis};
use qcore_schemas::hypothesis::Context;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 6, "INPUT_ROOT CHECKOUT REPO DIAGNOSTICS OUTPUT");
    let inputs = Path::new(&args[1]);
    let repo = Path::new(&args[3]);
    let profile_directory = inputs.join("contract/profiles/fluent-bit");
    let profile: Value =
        serde_json::from_slice(&fs::read(profile_directory.join("profile.json")).unwrap()).unwrap();
    let manifest = profile_directory.join(profile["resource_contract_manifest"].as_str().unwrap());
    assert_eq!(
        fs::canonicalize(&manifest).unwrap(),
        fs::canonicalize(inputs.join("cycle19-resource-contracts.json")).unwrap()
    );
    let mut contracts = load_resource_contract_manifest(&manifest).unwrap();
    let text = fs::read_to_string(&manifest).unwrap();
    qcore_audit::contract_revision::apply_pins(
        &mut contracts,
        &qcore_audit::contract_revision::ModuleOwners::for_manifest(&manifest, &text).unwrap(),
        Path::new(&args[2]),
    )
    .unwrap();
    let graphs = hypothesis::replay(contracts.source_assumptions()).unwrap();
    let held_manifest =
        fs::read(repo.join("docs/reports/libc-reading-site-evidence/bindings.json")).unwrap();
    assert_eq!(held_manifest, BINDINGS.as_bytes());
    let bindings: Value = serde_json::from_slice(&held_manifest).unwrap();
    let artifacts: BTreeMap<String, Vec<u8>> = bindings["artifacts"]
        .as_object()
        .unwrap()
        .values()
        .map(|r| {
            let path = r["path"].as_str().unwrap();
            (path.to_owned(), fs::read(repo.join(path)).unwrap())
        })
        .collect();
    resolve_bindings(&held_manifest, &artifacts).unwrap();
    let leaf_manifest_bytes =
        fs::read(repo.join("docs/reports/libc-reading-site-evidence/formal-leaves.json")).unwrap();
    let leaf_manifest: Value = serde_json::from_slice(&leaf_manifest_bytes).unwrap();
    assert_eq!(
        leaf_manifest["parent_binding_manifest_sha256"].as_str(),
        Some(qcore_schemas::sha256_hex(&held_manifest).as_str())
    );
    let leaf_source = fs::read(repo.join(leaf_manifest["source"].as_str().unwrap())).unwrap();
    assert_eq!(
        leaf_manifest["source_sha256"].as_str(),
        Some(qcore_schemas::sha256_hex(&leaf_source).as_str())
    );
    let parent_source =
        fs::read(repo.join(leaf_manifest["parent_model_source"].as_str().unwrap())).unwrap();
    assert_eq!(
        leaf_manifest["parent_model_source_sha256"].as_str(),
        Some(qcore_schemas::sha256_hex(&parent_source).as_str())
    );
    for (id, row) in bindings["propositions"].as_object().unwrap() {
        let leaf = &leaf_manifest["leaves"][id];
        assert_eq!(leaf["declaration"], row["declaration"]);
        assert_eq!(leaf["atom"], row["atom"]);
        let name = leaf["definition"]
            .as_str()
            .unwrap()
            .rsplit('.')
            .next()
            .unwrap();
        assert!(
            std::str::from_utf8(&leaf_source)
                .unwrap()
                .contains(&format!("def {name} ("))
        );
        assert_eq!(leaf["disposition"], "open");
    }
    let raw = fs::read(&args[4]).unwrap();
    let data: Value = serde_json::from_slice(&raw).unwrap();
    let sidecar_path = Path::new(&args[4])
        .parent()
        .unwrap()
        .join("resource-diagnostics.contracts.json");
    let sidecar_bytes = fs::read(&sidecar_path).unwrap();
    let sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
    let mut applied = qcore_schemas::contract_refutation::RefutationTable::new();
    for entry in sidecar["refuted_contracts"]["entries"].as_array().unwrap() {
        if entry["applied"] == true {
            applied.insert(
                entry["function"].as_str().unwrap().to_owned(),
                serde_json::from_value(entry["refutations"].clone()).unwrap(),
            );
        }
    }
    assert_eq!(
        sidecar["refuted_contracts"]["table_sha256"].as_str(),
        Some(qcore_schemas::contract_refutation::table_digest(&applied).as_str())
    );
    contracts = qcore_audit::refuted_contracts::with_table(&contracts, &applied);
    let mut results = Vec::new();
    let mut source_replays = Vec::new();
    let mut replayed = std::collections::BTreeSet::new();
    let mut attempted = std::collections::BTreeSet::new();
    let invocation_path = Path::new(&args[4])
        .parent()
        .unwrap()
        .join("invocation.json");
    let invocation: Value = serde_json::from_slice(&fs::read(invocation_path).unwrap()).unwrap();
    let entries = invocation["compile_database"].as_str().map(|path| {
        let bytes = fs::read(path).unwrap();
        assert_eq!(
            invocation["compile_database_sha256"].as_str(),
            Some(qcore_schemas::sha256_hex(&bytes).as_str())
        );
        qcore_audit::compile_db::compile_entries(std::str::from_utf8(&bytes).unwrap()).unwrap()
    });
    for unit in data["units"].as_array().unwrap() {
        for row in unit["functions"].as_array().unwrap() {
            let held_context: Context =
                serde_json::from_value(row["hypothesis_context"].clone()).unwrap();
            for value in row["reading_site_evidence"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let receipt: Receipt = serde_json::from_value(value.clone()).unwrap();
                let declaration = &receipt.observation.declaration;
                assert_eq!(
                    bind(&receipt.observation, &held_context, &graphs[declaration]).unwrap(),
                    receipt
                );
                assert_eq!(
                    contracts.contract(declaration),
                    Some(&receipt.observation.contract)
                );
                for id in &receipt.observation.prerequisite_ids {
                    assert!(
                        bindings["prerequisites"].get(id).is_some(),
                        "unresolved prerequisite {id}"
                    );
                }
                for id in &receipt.observation.model_ids {
                    assert!(
                        bindings["models"].get(id).is_some(),
                        "unresolved model {id}"
                    );
                }
                let route = serde_json::to_value(&receipt.observation.route).unwrap();
                for (atom, id) in &receipt.proposition_ids {
                    let definition = bindings["propositions"]
                        .get(id)
                        .expect("unresolved proposition");
                    assert_eq!(
                        definition["declaration"].as_str(),
                        Some(declaration.as_str())
                    );
                    assert_eq!(definition["atom"].as_str(), Some(atom.as_str()));
                    for predicate in definition["local_predicate_ids"][route.as_str().unwrap()]
                        .as_array()
                        .expect("route has no local predicate binding")
                    {
                        assert_eq!(
                            receipt
                                .observation
                                .checked_predicates
                                .get(predicate.as_str().unwrap()),
                            Some(&true)
                        );
                    }
                }
                let actual_dependencies = row["contract_dependencies"]["hypothesis_dependencies"]
                    [declaration]
                    .as_array()
                    .unwrap();
                assert!(
                    receipt
                        .atom_sha256
                        .keys()
                        .all(|a| actual_dependencies.iter().any(|v| v.as_str() == Some(a)))
                );
                // Bounded configured production source reanalysis. Inputs come
                // from the independently retained DB/raw source, not receipt
                // fields. A mismatch remains a refused replay, never refutation.
                if let Some(entries) = &entries {
                    let key = (
                        declaration.clone(),
                        format!(
                            "{}#{}",
                            unit["file"].as_str().unwrap(),
                            held_context.build_identity
                        ),
                    );
                    if !replayed.contains(declaration)
                        && attempted
                            .iter()
                            .filter(|key: &&(String, String)| &key.0 == declaration)
                            .count()
                            < 3
                        && attempted.insert(key)
                    {
                        let run = (|| -> Result<Value, String> {
                            use qcore_c_gateway::legacy_c::{
                                C11TranslationUnit, LegacyFunctionScope, prepare_translation_unit,
                            };
                            let configuration = &unit["compile_configuration"];
                            let entry = entries
                                .iter()
                                .find(|entry| {
                                    entry.file.to_string_lossy()
                                        == configuration["file"].as_str().unwrap()
                                        && entry.directory.to_string_lossy()
                                            == configuration["directory"].as_str().unwrap()
                                        && entry.command
                                            == configuration["command"].as_str().unwrap()
                                })
                                .ok_or("actual DB configuration absent")?;
                            let raw = fs::read(&entry.file).map_err(|e| e.to_string())?;
                            if configuration["raw_source_sha256"].as_str()
                                != Some(qcore_schemas::sha256_hex(&raw).as_str())
                            {
                                return Err("independently held raw source differs".into());
                            }
                            let source = std::str::from_utf8(&raw).map_err(|e| e.to_string())?;
                            let options = qcore_audit::compile_db::compiler_options(entry)?;
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
                            let source =
                                prepared.preprocessed_source().map_err(|e| e.to_string())?;
                            let selected: Vec<String> = serde_json::from_str(&held_context.scope)
                                .map_err(|e| e.to_string())?;
                            if qcore_schemas::sha256_hex(source.as_bytes())
                                != held_context.source_identity
                            {
                                return Err(
                                    "fresh independently prepared source differs from held Context"
                                        .into(),
                                );
                            }
                            if prepared.inventory().definitions() != selected.as_slice() {
                                return Err(
                                    "fresh prepared definition inventory differs from held scope"
                                        .into(),
                                );
                            }
                            let single = [receipt.clone()];
                            let direct = qcore_c_gateway::reading_site_evidence::replay(
                                source,
                                &receipt.observation.function,
                                &contracts,
                                Some(&selected),
                                unit.dialect,
                                &graphs,
                                &held_context,
                                &single,
                                &held_manifest,
                                &artifacts,
                            );
                            let mut prepared_fallback = false;
                            if direct.is_err() {
                                // The original prepared-input API additionally
                                // retains the actual caller-held request. Require
                                // exact source, scope, observation and consultation;
                                // never accept a conveniently different record.
                                let recording =
                                    qcore_c_gateway::contract_dependencies::Recording::start();
                                qcore_c_gateway::legacy_c::analyze_prepared_translation_unit(
                                    &prepared,
                                    &receipt.observation.function,
                                    &contracts,
                                )
                                .map_err(|e| e.to_string())?;
                                let fresh = recording.finish();
                                if !fresh.reading_sites.contains(&receipt.observation)
                                    || !fresh
                                        .functions
                                        .get(&receipt.observation.function)
                                        .is_some_and(|log| log.found.contains(declaration))
                                {
                                    return Err(format!(
                                        "direct replay refused: {:?}; exact original-request prepared replay also did not reproduce observation",
                                        direct.err()
                                    ));
                                }
                                prepared_fallback = true;
                            }
                            let changed = format!("{source} ");
                            assert!(
                                qcore_c_gateway::reading_site_evidence::replay(
                                    &changed,
                                    &receipt.observation.function,
                                    &contracts,
                                    Some(&selected),
                                    unit.dialect,
                                    &graphs,
                                    &held_context,
                                    &single,
                                    &held_manifest,
                                    &artifacts
                                )
                                .is_err()
                            );
                            Ok(
                                json!({"held_raw_source_sha256":qcore_schemas::sha256_hex(&raw),"held_preprocessed_source_sha256":qcore_schemas::sha256_hex(source.as_bytes()),"raw_source":entry.file,"actual_db_command":entry.command,"actual_db_directory":entry.directory,"independent_source_and_actual_consultation_reproduced":true,"changed_source_negative_rejected":true,"coverage":"one selected production observation only","direct_replay_result":direct,"original_request_prepared_replay_used":prepared_fallback}),
                            )
                        })();
                        if run.is_ok() {
                            replayed.insert(declaration.clone());
                        }
                        source_replays.push(json!({"receipt":receipt.id,"declaration":declaration,"result":run,"not_a_C_execution_or_universal_proof":true}));
                    }
                }
                results.push(json!({"receipt":receipt.id,"owner":row["function"],"declaration":declaration,"exact_graph_atom_context_binding":true,"loaded_selected_contract_equal":true,"all_formal_prerequisite_references_resolved":true,"local_predicate_reference_edges_resolved":true,"actual_consultation_atoms_present":true,"this_binding_record_claims_source_reanalysis":false,"universal_disposition":"open"}));
            }
        }
    }
    fs::write(&args[5], serde_json::to_vec_pretty(&json!({"schema":"libc-production-reference-replay.v1","diagnostics_sha256":qcore_schemas::sha256_hex(&raw),"binding_manifest_sha256":qcore_schemas::sha256_hex(&held_manifest),"formal_leaf_manifest_sha256":qcore_schemas::sha256_hex(&leaf_manifest_bytes),"independently_held_sidecar_sha256":qcore_schemas::sha256_hex(&sidecar_bytes),"reconstructed_applied_refutation_table_sha256":qcore_schemas::contract_refutation::table_digest(&applied),"checked_receipts":results.len(),"records":results,"bounded_production_source_replays":source_replays,"source_replayed_declarations":replayed,"meaning":"Independent retained graph, selected contract, full Context and formal-artifact binding checks only. Reference records alone confer no source-reanalysis claim. Bounded production_source_replays separately report exact held-source/request successes or conservative refusals for selected observations only; no universal discharge or C execution is proved."})).unwrap()).unwrap();
}
