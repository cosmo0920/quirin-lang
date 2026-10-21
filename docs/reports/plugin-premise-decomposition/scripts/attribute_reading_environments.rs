//! Report-owned explicit reconstruction, not a consumer of premise truth.
//! INPUT holds old/new root environments, actual diagnostics, and build commits.
//! Only independently held root source/inclusion metadata may be reversed here.
//! Every reconstructed effective environment must hit the actual BASE hash.
use qcore_c_gateway::reading_site_evidence::{Receipt, bind, publication_environments, validate_environment};
use qcore_schemas::{contract::ResourceContracts, hypothesis::Context};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
fn read(path: &Value) -> Vec<u8> { fs::read(path.as_str().expect("explicit file path")).unwrap() }
fn value(path: &Value) -> Value { serde_json::from_slice(&read(path)).unwrap() }
fn typed(value: &Value) -> ResourceContracts { serde_json::from_value(value.clone()).unwrap() }
fn digest<T: serde::Serialize>(v: &T) -> String { qcore_schemas::sha256_hex(&serde_json::to_vec(v).unwrap()) }
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
    assert_eq!(serde_json::to_value(&erased_new).unwrap(),serde_json::to_value(&old).unwrap(),"root delta exceeds held source/inclusion metadata; needs independent producer attribution");
    let old_graphs=qcore_contract::hypothesis::replay(old.source_assumptions()).unwrap();
    let new_graphs=qcore_contract::hypothesis::replay(new.source_assumptions()).unwrap();
    let before=value(&spec["base_diagnostics"]);
    let after=value(&spec["tip_diagnostics"]);
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
        let mut bound=Vec::new();
        for (b,t) in bf.iter().zip(tf) {
            assert_eq!(b["function"],t["function"],"function alignment");
            let mut base_receipts:Vec<Receipt>=b.get("reading_site_evidence").map(|v|serde_json::from_value(v.clone()).unwrap()).unwrap_or_default();
            let tip_receipts:Vec<Receipt>=t.get("reading_site_evidence").map(|v|serde_json::from_value(v.clone()).unwrap()).unwrap_or_default();
            assert_eq!(base_receipts.len(),tip_receipts.len());
            if tip_receipts.is_empty() {continue;}
            let bc:Context=serde_json::from_value(b["hypothesis_context"].clone()).unwrap();
            let tc:Context=serde_json::from_value(t["hypothesis_context"].clone()).unwrap();
            assert_eq!(bc.assumptions,digest(old.source_assumptions()));
            assert_eq!(tc.assumptions,digest(new.source_assumptions()));
            assert_eq!(bc.evidence_version,format!("contract-hypothesis-graph.v1:{}",spec["base_build_commit"].as_str().unwrap()));
            assert_eq!(tc.evidence_version,format!("contract-hypothesis-graph.v1:{}",spec["tip_build_commit"].as_str().unwrap()));
            let mut reversed_context=tc.clone();
            reversed_context.assumptions=bc.assumptions.clone();
            reversed_context.evidence_version=bc.evidence_version.clone();
            assert_eq!(reversed_context,bc,"unexplained Context coordinate; no field erasure");
            for receipt in tip_receipts {
                let declaration=&receipt.observation.declaration;
                assert_eq!(bind(&receipt.observation,&tc,&new_graphs[declaration]).unwrap(),receipt,"actual TIP binding");
                let bytes=&captured[&receipt.observation.environment_sha256];
                let mut reconstructed=validate_environment(bytes,&receipt.observation.environment_sha256).unwrap();
                assert_eq!(reconstructed.source_assumptions(),new.source_assumptions(),"effective source metadata differs from held TIP root");
                assert_eq!(reconstructed.inclusion_evidence(),new.inclusion_evidence(),"effective inclusion metadata differs from held TIP root");
                reconstructed.set_source_assumptions(old.source_assumptions().clone());
                reconstructed.set_inclusion_evidence(old.inclusion_evidence().clone());
                let canonical=serde_json::to_string(&reconstructed).unwrap();
                let old_sha=qcore_schemas::sha256_hex(canonical.as_bytes());
                let mut observation=receipt.observation.clone();
                observation.environment_sha256=old_sha.clone();
                let rebuilt=bind(&observation,&bc,&old_graphs[declaration]).unwrap();
                let position=base_receipts.iter().position(|r|r==&rebuilt).expect("no exact BASE receipt: unexplained effective environment or observation field");
                let actual=base_receipts.remove(position);
                // Both preimages are independently checked, not merely a root-hash map.
                validate_environment(&canonical,&actual.observation.environment_sha256).unwrap();
                environments.insert(old_sha.clone(),canonical);
                records.push(json!({"artifact":tc.artifact,"function":b["function"],"base_receipt":actual.id,"tip_receipt":receipt.id,"base_environment":old_sha,"tip_environment":receipt.observation.environment_sha256,"non_environment_observation_payload":"exact","receipt_replay":"actual unchanged bind both sides","disposition":"open"}));
                bound.push(receipt);
            }
            assert!(base_receipts.is_empty());
        }
        assert_eq!(publication_environments(&bound,&captured).unwrap(),captured,"published captures must be exactly the bound receipt domain");
    }
    let hashes=["base_environment","tip_environment","base_diagnostics","tip_diagnostics"].into_iter().map(|key|(key,qcore_schemas::sha256_hex(&read(&spec[key])))).collect::<BTreeMap<_,_>>();
    fs::write(&args[2],serde_json::to_vec_pretty(&json!({"status":"exact canonical reconstruction and complete old/new receipt replay","input_sha256":hashes,"receipts":records,"reconstructed_base_environments":environments,"source_binding":"open","truth_transport":false})).unwrap()).unwrap();
}
