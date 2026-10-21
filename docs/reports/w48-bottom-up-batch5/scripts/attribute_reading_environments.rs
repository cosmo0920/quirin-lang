//! Joint-cycle exact reconstruction; both sides retain canonical capture bytes.
//! No metadata atom inverse or capture-field deletion is admitted.
//! INPUT holds old/new root environments, actual diagnostics, and build commits.
//! Only independently held root source/inclusion metadata may be reversed here.
//! Every reconstructed effective environment must hit the actual BASE hash.
use qcore_c_gateway::reading_site_evidence::{Receipt, bind, publication_environments, validate_environment};
use qcore_schemas::{contract::ResourceContracts, hypothesis::Context};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
fn read(path: &Value) -> Vec<u8> { fs::read(path.as_str().expect("explicit file path")).unwrap() }
fn value(path: &Value) -> Value { serde_json::from_slice(&read(path)).unwrap() }
fn typed(value: &Value) -> ResourceContracts { let result:ResourceContracts=serde_json::from_value(value.clone()).unwrap(); assert_eq!(serde_json::to_value(&result).unwrap(),*value,"complete environment roundtrip; no ignored fields"); result }
fn digest(v: &ResourceContracts) -> String { qcore_schemas::sha256_hex(&serde_json::to_vec(v).unwrap()) }
fn source_digest(v: &BTreeMap<String,String>) -> String { qcore_schemas::sha256_hex(&serde_json::to_vec(v).unwrap()) }
// Only the two existing diagnostic provenance records have snapshot paths.
// Every changed scalar must match both independently held roots and the same
// literal suffix under their explicitly supplied immutable snapshot prefixes.
fn reverse_snapshot_paths(environment: &mut ResourceContracts, old: &ResourceContracts,
    new: &ResourceContracts, old_root: &str, new_root: &str) {
    let before=serde_json::to_value(old).unwrap();
    let after=serde_json::to_value(new).unwrap();
    let mut value=serde_json::to_value(&*environment).unwrap();
    for kind in ["pending_review","pin_mismatch"] {
        let Some(records)=value.get_mut(kind).and_then(Value::as_object_mut) else {continue;};
        for (name,record) in records {
            for field in ["module","lock"] {
                let Some(expected_new)=after.get(kind).and_then(|v|v.get(name)).and_then(|v|v.get(field)) else {continue;};
                let expected_old=&before[kind][name][field];
                if expected_new==expected_old {continue;}
                let new_path=expected_new.as_str().expect("provenance path scalar");
                let suffix=new_path.strip_prefix(new_root).expect("new provenance has actual TIP snapshot prefix");
                assert!(suffix.starts_with('/'));
                assert_eq!(expected_old.as_str(),Some(format!("{old_root}{suffix}").as_str()),"provenance path suffix differs");
                assert!(record.get(field)==Some(expected_new),"captured provenance differs from held TIP field");
                record[field]=expected_old.clone();
            }
        }
    }
    *environment=serde_json::from_value(value).unwrap();
}
fn replay_build(base:&Value,tip:&Value,bc:&Context,tc:&Context,old:&ResourceContracts,new:&ResourceContracts,historical:Option<&Value>) {
    // The pinned historical route publishes no configured compile-task object.
    // It needs no identity substitution: require the entire available record
    // and the independently held Context build coordinate to be unchanged.
    if base.get("compile_task_identity").is_none() && tip.get("compile_task_identity").is_none() {
        assert_eq!(base,tip,"historical configuration changed; independent producer replay required");
        if bc.build_identity!=tc.build_identity {
            let proof=historical.expect("explicit independently replayed historical build proof");
            let records=proof["records"].as_array().unwrap();
            let matched=records.iter().filter(|r|r["artifact"].as_str()==Some(bc.artifact.split_once('#').unwrap().0)).collect::<Vec<_>>();assert_eq!(matched.len(),1);let r=matched[0];
            assert_eq!(qcore_schemas::sha256_hex(r["base_preimage"].as_str().unwrap().as_bytes()),bc.build_identity);
            assert_eq!(qcore_schemas::sha256_hex(r["tip_preimage"].as_str().unwrap().as_bytes()),tc.build_identity);
            assert_eq!(r["base_build"],bc.build_identity);assert_eq!(r["tip_build"],tc.build_identity);
            for side in ["base","tip"] {
                let expected=format!("({}, {:?}, {})",r["unit_debug"].as_str().unwrap(),std::path::PathBuf::from(r[format!("{side}_cwd")].as_str().unwrap()),r["clang_debug"].as_str().unwrap());
                assert_eq!(expected,r[format!("{side}_preimage")].as_str().unwrap(),"complete typed Debug tuple");
            }
            assert_eq!(r["wrong_cwd_negative_rejected"],true);
        }
        return;
    }
    use qcore_audit::compile_db::{CompileEntry,compiler_options,compile_task_identity,EffectiveInput};
    use qcore_c_gateway::{ClangIdentity,legacy_c::{C11TranslationUnit,LegacyFunctionScope,prepare_translation_unit}};
    let entry=CompileEntry{directory:base["directory"].as_str().unwrap().into(),file:base["file"].as_str().unwrap().into(),command:base["command"].as_str().unwrap().into()};
    for field in ["directory","file","command","compiler","raw_source_sha256","source_revision"] {assert_eq!(base[field],tip[field],"compile source field changed");}
    let raw=fs::read_to_string(&entry.file).unwrap();
    let options=compiler_options(&entry).unwrap();
    let unit=C11TranslationUnit{source:&raw,include_paths:options.include_paths,definitions:options.definitions,preprocessor_options:options.preprocessor_options,dialect:options.dialect,function_scope:LegacyFunctionScope::MainSourceDefinitions,input:options.input};
    let prepared=prepare_translation_unit(&unit);
    let pp=prepared.preprocessed_source().unwrap();
    let compiler=ClangIdentity::capture().unwrap();
    assert_eq!(serde_json::to_value(&compiler).unwrap(),base["compiler"]);
    let mut tasks=Vec::new();
    for (cfg,context,root) in [(base,bc,old),(tip,tc,new)] {
        let dependencies:BTreeMap<String,String>=serde_json::from_value(cfg["dependencies"].clone()).unwrap();
        assert_eq!(dependencies["resource_contract_sources"],source_digest(root.source_assumptions()));
        let task=compile_task_identity(&entry,&unit,EffectiveInput::Accepted(pp),&compiler,dependencies).unwrap();
        assert_eq!(serde_json::to_value(&task).unwrap(),cfg["compile_task_identity"],"production compile-task reconstruction");
        assert_eq!(task.digest().unwrap(),context.build_identity,"actual complete typed task digest");
        let mut inventory=cfg.clone(); let published=inventory.as_object_mut().unwrap().remove("inventory_sha256").unwrap();
        assert_eq!(qcore_schemas::sha256_hex(&serde_json::to_vec(&inventory).unwrap()),published.as_str().unwrap(),"complete compile inventory preimage");
        tasks.push(serde_json::to_value(&task).unwrap());
    }
    tasks[1]["dependencies"]["resource_contract_sources"]=tasks[0]["dependencies"]["resource_contract_sources"].clone();
    assert_eq!(tasks[0],tasks[1],"compile task has unexplained delta beyond held root-source identity");
}
fn main() {
    let args=std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(),3,"INPUT OUTPUT");
    let spec:Value=serde_json::from_slice(&fs::read(&args[1]).unwrap()).unwrap();
    let old=typed(&value(&spec["base_environment"]));
    let new=typed(&value(&spec["tip_environment"]));
    // No semantic, import, settlement, contract, or source authority substitution.
    let mut erased_new=new.clone();
    erased_new.set_source_assumptions(old.source_assumptions().clone());
    erased_new.set_inclusion_evidence(old.inclusion_evidence().clone());
    let old_root=spec["base_inputs_root"].as_str().unwrap();
    let new_root=spec["tip_inputs_root"].as_str().unwrap();
    reverse_snapshot_paths(&mut erased_new,&old,&new,old_root,new_root);
    assert!(serde_json::to_value(&erased_new).unwrap()==serde_json::to_value(&old).unwrap(),"root delta exceeds held source/inclusion and exact snapshot-path metadata; needs independent attribution");
    let mut matched_sources=0;
    for (old_key,old_text) in old.source_assumptions() {
        let new_key=old_key.strip_prefix(old_root).map(|suffix|format!("{new_root}{suffix}")).unwrap_or_else(||old_key.clone());
        assert_eq!(new.source_assumptions().get(&new_key),Some(old_text),"no source/atom/text inverse: all exact held module values agree");
        matched_sources+=1;
    }
    assert_eq!(matched_sources,new.source_assumptions().len(),"no extra source module");
    let old_graphs=qcore_contract::hypothesis::replay(old.source_assumptions()).unwrap();
    let new_graphs=qcore_contract::hypothesis::replay(new.source_assumptions()).unwrap();
    assert_eq!(old_graphs,new_graphs,"complete existing graphs must be exactly unchanged; no graph inverse");
    let historical=spec.get("historical_build_proof").map(value);
    if let Some(proof)=&historical {
        for key in ["base_diagnostics","tip_diagnostics","base_invocation","tip_invocation"] {
            assert_eq!(proof["input_sha256"][key],qcore_schemas::sha256_hex(&read(&spec[key])));
        }
    }
    let before=value(&spec["base_diagnostics"]);
    let after=value(&spec["tip_diagnostics"]);
    let old_contracts=value(&spec["base_contracts"]);
    let new_contracts=value(&spec["tip_contracts"]);
    for (sidecar,environment) in [(&old_contracts,&old),(&new_contracts,&new)] {
        let source_field=sidecar.get("sources").or_else(||sidecar.get("contracts")).unwrap();
        assert_eq!(source_field,&json!(environment.source_assumptions()));

    }
    // current_route and pinned_route export homes under the held pre-import root.
    // Check every actual selected home before deriving the sole permitted delta.
    let old_table=qcore_audit::refuted_contracts::table(before["units"].as_array().unwrap());
    let new_table=qcore_audit::refuted_contracts::table(after["units"].as_array().unwrap());
    assert_eq!(old_table,new_table,"actual body-refutation payload must be unchanged");
    assert_eq!(old_contracts["refuted_contracts"],new_contracts["refuted_contracts"],"complete applied refutation section");
    assert_eq!(qcore_schemas::contract_refutation::table_digest(&old_table),old_contracts["refuted_contracts"]["table_sha256"].as_str().unwrap());
    let old_home_environment=qcore_audit::refuted_contracts::with_table(&old,&old_table);
    let new_home_environment=qcore_audit::refuted_contracts::with_table(&new,&new_table);
    let old_home_root=digest(&old_home_environment);
    let new_home_root=digest(&new_home_environment);
    let home_records=|sidecar:&Value| -> BTreeMap<String,Value> {
        let mut out=BTreeMap::new();
        for entry in sidecar["cross_unit_link_resolution"]["imported"].as_array().into_iter().flatten() {
            assert!(out.insert(entry["function"].as_str().unwrap().to_owned(),entry.clone()).is_none());
        }
        out
    };
    let old_homes=home_records(&old_contracts);
    let new_homes=home_records(&new_contracts);
    assert_eq!(old_homes.keys().collect::<Vec<_>>(),new_homes.keys().collect::<Vec<_>>());
    for (function,new_home) in &new_homes {
        assert_eq!(new_home["home_environment_sha256"],new_home_root);
        assert_eq!(old_homes[function]["home_environment_sha256"],old_home_root);
        let mut reconstructed=new_home.clone();
        reconstructed["home_environment_sha256"]=json!(old_home_root);
        assert_eq!(reconstructed,old_homes[function],"home identity or selected link premise changed beyond exact root environment");
    }
    let mut old_cross=old_contracts["cross_unit_link_resolution"].clone();
    let new_cross=&new_contracts["cross_unit_link_resolution"];
    if let Some(imported)=old_cross.get_mut("imported").and_then(Value::as_array_mut) {
        for entry in imported {entry["home_environment_sha256"]=json!(new_home_root);}
    }
    assert_eq!(&old_cross,new_cross,"complete link-resolution premise/selection must agree");
    let bu=before["units"].as_array().unwrap();
    let tu=after["units"].as_array().unwrap();
    assert_eq!(bu.len(),tu.len());
    let mut records=Vec::new();
    let mut environments=BTreeMap::<String,String>::new();
    for (base,tip) in bu.iter().zip(tu) {
        for key in ["library","file"] {assert_eq!(base[key],tip[key],"unit alignment");}
        let bf=base["functions"].as_array().unwrap();
        let tf=tip["functions"].as_array().unwrap();
        assert_eq!(bf.len(),tf.len());
        let captured:BTreeMap<String,String>=tip.get("reading_site_environments")
            .map(|v|serde_json::from_value(v["environments"].clone()).unwrap()).unwrap_or_default();
        let base_captured:BTreeMap<String,String>=base.get("reading_site_environments")
            .map(|v|serde_json::from_value(v["environments"].clone()).unwrap()).unwrap_or_default();
        let mut bound_base=Vec::new();
        let mut bound=Vec::new();
        for (b,t) in bf.iter().zip(tf) {
            assert_eq!(b["function"],t["function"],"function alignment");
            let mut base_receipts:Vec<Receipt>=b.get("reading_site_evidence").map(|v|serde_json::from_value(v.clone()).unwrap()).unwrap_or_default();
            let tip_receipts:Vec<Receipt>=t.get("reading_site_evidence").map(|v|serde_json::from_value(v.clone()).unwrap()).unwrap_or_default();
            if let Some(raw)=b.get("reading_site_evidence") {assert_eq!(serde_json::to_value(&base_receipts).unwrap(),*raw,"complete BASE raw receipt roundtrip");}
            if let Some(raw)=t.get("reading_site_evidence") {assert_eq!(serde_json::to_value(&tip_receipts).unwrap(),*raw,"complete TIP raw receipt roundtrip");}
            assert_eq!(base_receipts.len(),tip_receipts.len());
            if tip_receipts.is_empty() {continue;}
            let bc:Context=serde_json::from_value(b["hypothesis_context"].clone()).unwrap();
            let tc:Context=serde_json::from_value(t["hypothesis_context"].clone()).unwrap();
            assert_eq!(bc.assumptions,source_digest(old.source_assumptions()));
            assert_eq!(tc.assumptions,source_digest(new.source_assumptions()));
            assert_eq!(bc.evidence_version,format!("contract-hypothesis-graph.v1:{}",spec["base_build_commit"].as_str().unwrap()));
            assert_eq!(tc.evidence_version,format!("contract-hypothesis-graph.v1:{}",spec["tip_build_commit"].as_str().unwrap()));
            replay_build(&base["compile_configuration"],&tip["compile_configuration"],&bc,&tc,&old,&new,historical.as_ref());
            let mut reversed_context=tc.clone();
            reversed_context.build_identity=bc.build_identity.clone();
            reversed_context.assumptions=bc.assumptions.clone();
            reversed_context.evidence_version=bc.evidence_version.clone();
            assert_eq!(reversed_context,bc,"unexplained Context coordinate; no field erasure");
            for receipt in tip_receipts {
                let declaration=&receipt.observation.declaration;
                assert_eq!(bind(&receipt.observation,&tc,&new_graphs[declaration]).unwrap(),receipt,"actual TIP binding");
                let bytes=&captured[&receipt.observation.environment_sha256];
                let mut reconstructed=validate_environment(bytes,&receipt.observation.environment_sha256).unwrap();
                assert_eq!(reconstructed.source_assumptions(),new.source_assumptions(),"effective source metadata differs from held TIP root");
                let carries_inclusion=!reconstructed.inclusion_evidence().is_empty();
                if carries_inclusion {assert!(reconstructed.inclusion_evidence()==new.inclusion_evidence(),"effective inclusion metadata differs from held TIP root");}
                reconstructed.set_source_assumptions(old.source_assumptions().clone());
                if carries_inclusion {reconstructed.set_inclusion_evidence(old.inclusion_evidence().clone());}
                reverse_snapshot_paths(&mut reconstructed,&old,&new,old_root,new_root);
                let mut imports=reconstructed.cross_unit_import_identities().clone();
                for (function,identity) in &mut imports {
                    let home=&new_homes[function];
                    assert_eq!(identity.environment_sha256,new_home_root);
                    assert_eq!(identity.raw_source_sha256,home["home_raw_source_sha256"]);
                    assert_eq!(identity.preprocessed_source_sha256,home["home_preprocessed_source_sha256"]);
                    identity.environment_sha256=old_home_root.clone();
                }
                reconstructed.set_cross_unit_import_identities(imports);
                let canonical=serde_json::to_string(&reconstructed).unwrap();
                let old_sha=qcore_schemas::sha256_hex(canonical.as_bytes());
                let mut observation=receipt.observation.clone();
                observation.environment_sha256=old_sha.clone();
                let rebuilt=bind(&observation,&bc,&old_graphs[declaration]).unwrap();
                let exact=base_receipts.iter().position(|r|r==&rebuilt);
                let mut semantic=Vec::new();
                let position=if let Some(position)=exact {position} else {
                    let allowed=spec.get("semantic_summary_contracts").expect("independently source-reviewed summary changes");
                    let unit_artifact=bc.artifact.split_once('#').unwrap().0;
                    assert_eq!(allowed["unit_artifact"],unit_artifact);
                    for (name,contract) in allowed["contracts"].as_object().unwrap() {
                        assert!(!new.functions().contains_key(name),"not a new declared authority");
                        let body=tip["functions"].as_array().unwrap().iter().find(|r|r["function"]==*name).unwrap();
                        assert_eq!(body["status"],"Safe","summary needs actual native body standing");
                        assert_eq!(body["diagnostics"],json!([]));
                        assert_eq!(contract["result"],"none");
                        assert!(contract["parameters"].as_array().unwrap().iter().all(|p|p=="borrowed"));
                    }
                    let candidate_value=serde_json::to_value(&reconstructed).unwrap();
                    let mut options=Vec::new();
                    for (i,prior) in base_receipts.iter().enumerate() {
                        let mut paired=receipt.observation.clone();paired.environment_sha256=prior.observation.environment_sha256.clone();
                        if paired!=prior.observation {continue;}
                        let old_bytes=&base_captured[&prior.observation.environment_sha256];
                        let old_env=validate_environment(old_bytes,&prior.observation.environment_sha256).unwrap();
                        let old_value=serde_json::to_value(&old_env).unwrap();
                        let mut metadata_only=candidate_value.clone();metadata_only["functions"]=old_value["functions"].clone();
                        if metadata_only!=old_value {continue;}
                        let mut changes=Vec::new();let names=old_value["functions"].as_object().unwrap().keys().chain(candidate_value["functions"].as_object().unwrap().keys()).cloned().collect::<std::collections::BTreeSet<_>>();
                        let mut valid=true;
                        for name in &names {
                            let before=&old_value["functions"][name];let after=&candidate_value["functions"][name];
                            if before==after {continue;}
                            let Some(expected)=allowed["contracts"].get(name) else {valid=false;break;};
                            if !before.is_null() && before!=expected || !after.is_null() && after!=expected {valid=false;break;}
                            changes.push(json!({"function":name,"before":before,"after":after,"native_tip_body":"Safe with no diagnostics; bounded validated inferred-summary publication; source-binding Open"}));
                        }
                        if valid {options.push((changes.len(),prior.id.clone(),i,changes));}
                    }
                    options.sort_by(|a,b|(a.0,&a.1).cmp(&(b.0,&b.1)));
                    let (_,_,i,changes)=options.into_iter().next().expect("unexplained canonical environment beyond actual source-reviewed summaries");semantic=changes;i
                };
                let actual=base_receipts.remove(position);
                assert_eq!(bind(&actual.observation,&bc,&old_graphs[declaration]).unwrap(),actual,"actual BASE binding");
                let actual_canonical=&base_captured[&actual.observation.environment_sha256];
                validate_environment(actual_canonical,&actual.observation.environment_sha256).unwrap();
                if semantic.is_empty() {assert_eq!(actual_canonical,&canonical,"identity-only canonical capture");}
                bound_base.push(actual.clone());
                environments.insert(old_sha.clone(),canonical.clone());
                records.push(json!({"artifact":tc.artifact,"function":b["function"],"base_receipt":actual.id,"tip_receipt":receipt.id,"base_environment":actual.observation.environment_sha256,"tip_environment":receipt.observation.environment_sha256,"normalized_tip_environment":old_sha,"normalized_tip_receipt":rebuilt,"semantic_function_changes":semantic,"non_environment_observation_payload":"exact paired multiset","receipt_replay":"actual unchanged bind both sides; metadata-normalized TIP semantics retained","disposition":"open"}));
                bound.push(receipt);
            }
            assert!(base_receipts.is_empty());
        }
        assert_eq!(publication_environments(&bound_base,&base_captured).unwrap(),base_captured,"BASE captures must exactly match actual bound domain");
        assert_eq!(publication_environments(&bound,&captured).unwrap(),captured,"published captures must be exactly the bound receipt domain");
    }
    let hashes=["base_environment","tip_environment","base_diagnostics","tip_diagnostics","base_contracts","tip_contracts"].into_iter().map(|key|(key,qcore_schemas::sha256_hex(&read(&spec[key])))).collect::<BTreeMap<_,_>>();
    fs::write(&args[2],serde_json::to_vec_pretty(&json!({"status":"exact canonical reconstruction and complete old/new receipt replay","input_sha256":hashes,"receipts":records,"normalized_tip_environments":environments,"source_binding":"open","truth_transport":false})).unwrap()).unwrap();
}
