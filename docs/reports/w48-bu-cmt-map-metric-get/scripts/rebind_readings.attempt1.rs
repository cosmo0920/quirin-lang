//! Exact binding replay for unchanged reading observations and captures.
//! Refuses any delta beyond the independently recorded evidence build version.
//! Native observations remain conditional; this is no source truth transport.
use qcore_c_gateway::reading_site_evidence::{Receipt,bind,validate_environment};
use qcore_schemas::hypothesis::Context;
use serde_json::{Value,json};
use std::{collections::BTreeMap,fs};
fn value(p:&str)->Value {serde_json::from_slice(&fs::read(p).unwrap()).unwrap()}
fn main(){
 let a=std::env::args().collect::<Vec<_>>();assert_eq!(a.len(),8,"BASE_DIAG TIP_DIAG BASE_CONTRACTS TIP_CONTRACTS BASE_COMMIT TIP_COMMIT OUTPUT");
 let (b,t,bc,tc)=(value(&a[1]),value(&a[2]),value(&a[3]),value(&a[4]));
 let sources:BTreeMap<String,String>=serde_json::from_value(bc["sources"].clone()).unwrap();
 assert_eq!(bc["sources"],tc["sources"],"all held contract source assumptions must agree");
 let graphs=qcore_contract::hypothesis::replay(&sources).unwrap();
 let bu=b["units"].as_array().unwrap();let tu=t["units"].as_array().unwrap();assert_eq!(bu.len(),tu.len());
 let mut records=Vec::new();let mut captures=0;
 for (base,tip) in bu.iter().zip(tu){
  for k in ["library","file"] {assert_eq!(base[k],tip[k]);}
  assert_eq!(base.get("reading_site_environments"),tip.get("reading_site_environments"),"captured environment delta requires separate semantic attribution");
  let envs:BTreeMap<String,String>=base.get("reading_site_environments").map(|x|serde_json::from_value(x["environments"].clone()).unwrap()).unwrap_or_default();
  for (sha,text) in &envs {let env=validate_environment(text,sha).unwrap();assert_eq!(env.source_assumptions(),&sources);captures+=1;}
  let bf=base["functions"].as_array().unwrap();let tf=tip["functions"].as_array().unwrap();assert_eq!(bf.len(),tf.len());
  for (before,after) in bf.iter().zip(tf){
   assert_eq!(before["function"],after["function"]);
   let br:Vec<Receipt>=before.get("reading_site_evidence").map(|x|serde_json::from_value(x.clone()).unwrap()).unwrap_or_default();
   let tr:Vec<Receipt>=after.get("reading_site_evidence").map(|x|serde_json::from_value(x.clone()).unwrap()).unwrap_or_default();assert_eq!(br.len(),tr.len());
   if br.is_empty(){continue;}
   assert_eq!(serde_json::to_value(&br).unwrap(),before["reading_site_evidence"]);assert_eq!(serde_json::to_value(&tr).unwrap(),after["reading_site_evidence"]);
   let old:Context=serde_json::from_value(before["hypothesis_context"].clone()).unwrap();let new:Context=serde_json::from_value(after["hypothesis_context"].clone()).unwrap();
   assert_eq!(old.evidence_version,format!("contract-hypothesis-graph.v1:{}",a[5]));assert_eq!(new.evidence_version,format!("contract-hypothesis-graph.v1:{}",a[6]));
   let mut revised=new.clone();revised.evidence_version=old.evidence_version.clone();assert_eq!(revised,old,"other Context coordinate changed");
   let mut remaining=br.clone();
   for r in tr {
    let graph=&graphs[&r.observation.declaration];assert_eq!(bind(&r.observation,&new,graph).unwrap(),r,"complete actual TIP binding");
    assert!(envs.contains_key(&r.observation.environment_sha256));
    let restored=bind(&r.observation,&old,graph).unwrap();let i=remaining.iter().position(|x|x==&restored).expect("observation changed beyond evidence build version");let prior=remaining.remove(i);
    assert_eq!(bind(&prior.observation,&old,graph).unwrap(),prior,"complete actual BASE binding");
    assert_ne!(r,restored,"different recorded versions must differ in binding");
    records.push(json!({"library":base["library"],"file":base["file"],"function":before["function"],"base_id":prior.id,"tip_id":r.id,"base_receipt":prior,"tip_receipt":r,"exact_observation_unchanged":true,"exact_environment_unchanged":true,"actual_bindings_reproduced":true}));
   }
   assert!(remaining.is_empty());
  }
 }
 let hashes=a[1..5].iter().map(|p|(p.clone(),qcore_schemas::sha256_hex(&fs::read(p).unwrap()))).collect::<BTreeMap<_,_>>();
 fs::write(&a[7],serde_json::to_vec_pretty(&json!({"records":records,"canonical_captures_validated":captures,"input_sha256":hashes,"source_binding":"Open","truth_transport":false,"meaning":"Complete production binding replay, only actual evidence build version differs; every observation/capture/graph retained and equal"})).unwrap()).unwrap();
}
