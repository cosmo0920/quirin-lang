//! Independent exact guarded-v2 source and official-pin provenance.
use qcore_contract::guarded_metadata_bridge::{self, Plan};
use qcore_schemas::contract::ResourceContracts;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};
const PLAN_SHA: &str = "f19cb3d1f20df83b5eb1396d1baf9dadeb9d6ca7dd3570edba10e42eb817219e";
pub struct SourceBridge { before: String, after: String, receipt: Value }
fn sha(bytes:&[u8])->String {qcore_schemas::sha256_hex(bytes)}
impl SourceBridge {
 pub fn new(spec:&Value,old:&ResourceContracts,new:&ResourceContracts)->Self {
  let plan_bytes=fs::read(spec["guarded_metadata_plan"].as_str().unwrap()).unwrap();
  assert_eq!(sha(&plan_bytes),PLAN_SHA,"independently held guarded source plan");
  let p:Plan=serde_json::from_slice(&plan_bytes).unwrap();
  let select=|sources:&BTreeMap<String,String>|{
   let values=sources.iter().filter(|(key,_)|key.ends_with(&p.module)).collect::<Vec<_>>();assert_eq!(values.len(),1,"exact one production module");values[0].1.clone()
  };
  let before=select(old.source_assumptions());let after=select(new.source_assumptions());
  let held=BTreeMap::from([(p.module.clone(),before.clone())]);let presented=BTreeMap::from([(p.module.clone(),after.clone())]);
  let checked=guarded_metadata_bridge::check(&plan_bytes,PLAN_SHA,&held,&presented).unwrap();
  assert_eq!(checked.recover_before(&presented).unwrap(),held);
  let mut bad=presented.clone();bad.get_mut(&p.module).unwrap().push('\n');assert!(checked.recover_before(&bad).is_err());
  let old_root=Path::new(spec["base_inputs_root"].as_str().unwrap());let new_root=Path::new(spec["tip_inputs_root"].as_str().unwrap());
  let mut files=Vec::new();
  for relative in [&p.module,"providers/qcore-provider-librdkafka/contracts.lock","providers/qcore-provider-librdkafka/contracts.json"] {
   let b=fs::read(old_root.join(relative)).unwrap();let t=fs::read(new_root.join(relative)).unwrap();assert_ne!(b,t);
   if relative==&p.module {assert_eq!(b,before.as_bytes());assert_eq!(t,after.as_bytes());}
   else {
    let bv:Value=serde_json::from_slice(&b).unwrap();let mut tv:Value=serde_json::from_slice(&t).unwrap();
    if relative.ends_with("contracts.lock") {
     let ob=bv["pinned"].as_array().unwrap().iter().find(|e|e["function"]==p.declaration).unwrap();
     let nt=tv["pinned"].as_array_mut().unwrap().iter_mut().find(|e|e["function"]==p.declaration).unwrap();
     let bd=qcore_contract::contract_revision::declaration_digests(&held).unwrap();let td=qcore_contract::contract_revision::declaration_digests(&presented).unwrap();
     assert_eq!(ob["contract_sha256"],bd[&p.declaration].1);assert_eq!(nt["contract_sha256"],td[&p.declaration].1);
     nt["contract_sha256"]=ob["contract_sha256"].clone();
    } else {
     assert_eq!(bv["contracts"]["contracts/librdkafka/cjson.qv"],sha(before.as_bytes()));
     assert_eq!(tv["contracts"]["contracts/librdkafka/cjson.qv"],sha(after.as_bytes()));
     tv["contracts"]["contracts/librdkafka/cjson.qv"]=bv["contracts"]["contracts/librdkafka/cjson.qv"].clone();
    }
    assert_eq!(bv,tv,"whole official pin/inventory differs beyond checked metadata identity");
   }
   files.push(json!({"relative":relative,"before_sha256":sha(&b),"after_sha256":sha(&t),"scope":"exact guarded source recovery or complete official JSON identity reconstruction; original C sites and all unrelated fields exact"}));
  }
  let receipt=json!({"plan_sha256":PLAN_SHA,"checked":checked,"source_module":p.module,"before_text":before,"after_text":after,"changed_runtime_files":files,"wrong_tip_source_negative_rejected":true,"source_truth":"open"});
  Self {before,after,receipt}
 }
 pub fn approved_source(&self,before:&str,after:&str)->bool {before==after || before==self.before && after==self.after}
 pub fn json(&self)->Value {self.receipt.clone()}
}
