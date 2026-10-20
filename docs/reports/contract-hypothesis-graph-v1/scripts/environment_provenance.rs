use std::{fs,path::Path};
use qcore_contract::contract_loader::load_resource_contract_manifest;
fn main(){let a:Vec<_>=std::env::args().collect();let manifest=Path::new(&a[1]);let mut env=load_resource_contract_manifest(manifest).unwrap();let text=fs::read_to_string(manifest).unwrap();qcore_audit::contract_revision::apply_pins(&mut env,&qcore_audit::contract_revision::ModuleOwners::for_manifest(manifest,&text).unwrap(),Path::new(&a[2])).unwrap();let bytes=serde_json::to_vec(&env).unwrap();println!("{}",qcore_contract::profile_loader::sha256_hex(&bytes));fs::write(&a[3],bytes).unwrap();}
