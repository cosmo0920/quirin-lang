//! Read-only actual production bounds selection/pin replay, no truth consumer.
use std::{fs,path::Path};
use serde_json::{Value,json};
fn main(){
 let a:Vec<String>=std::env::args().collect();assert_eq!(a.len(),4);
 let manifest=Path::new(&a[1]);let text=fs::read_to_string(manifest).unwrap();
 let path=qcore_audit::bounds_selection::manifest_load_path(manifest,&text).unwrap().unwrap();
 let load=qcore_contract::load_path_manifest::load_contract_load_path_manifest(&path).unwrap();
 let (mut env,modules)=qcore_audit::bounds_selection::load_views(&load).unwrap();
 let pins=qcore_audit::bounds_selection::apply_bounds_pins(&mut env,&load,Path::new(&a[2])).unwrap();
 let mut descriptor=String::from(qcore_contract::bounds_contract::BOUNDS_CONTRACT_VERSION);
 for (path,sha) in &env.sources {descriptor.push_str(&format!("\n{path} {sha}"));}
 for name in &env.refuted {descriptor.push_str(&format!("\nrefuted {name}"));}
 let mut holds=serde_json::Map::new();
 for (name,c) in &env.functions {if let Some(hold)=&c.pin_hold {descriptor.push_str(&format!("\npin-held {name} {}",hold.reason));holds.insert(name.clone(),json!(hold.reason));}}
 assert_eq!(qcore_schemas::sha256_hex(descriptor.as_bytes()),env.digest());
 let result:Value=json!({"digest":env.digest(),"descriptor":descriptor,"sources":env.sources,"refuted":env.refuted,"pin_holds":holds,"selection_modules":modules,"pins":pins,"claim":"actual production selection and pin replay; no source truth"});
 fs::write(&a[3],serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}
