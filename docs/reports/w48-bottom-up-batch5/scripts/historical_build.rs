//! Reconstruct complete historical build Debug tuples from held route inputs.
use qcore_audit::compile_db::{CompileEntry, compiler_options_compatibility as compiler_options};
use qcore_c_gateway::{ClangIdentity, legacy_c::{C11TranslationUnit, LegacyFunctionScope}};
use serde_json::{Value,json};
use std::{fs,path::PathBuf};
fn read(path:&Value)->Value { serde_json::from_slice(&fs::read(path.as_str().unwrap()).unwrap()).unwrap() }
fn main(){
 let args=std::env::args().collect::<Vec<_>>();assert_eq!(args.len(),3);
 let spec:Value=serde_json::from_slice(&fs::read(&args[1]).unwrap()).unwrap();
 let base=read(&spec["base_diagnostics"]);let tip=read(&spec["tip_diagnostics"]);
 let bi=read(&spec["base_invocation"]);let ti=read(&spec["tip_invocation"]);
 assert_eq!(bi["source_identity"],ti["source_identity"]);assert_eq!(bi["source_identity"],bi["source_identity_after"]);assert_eq!(ti["source_identity"],ti["source_identity_after"]);
 let bc=PathBuf::from(bi["cwd"].as_str().unwrap());let tc=PathBuf::from(ti["cwd"].as_str().unwrap());assert_ne!(bc,tc);
 let compiler=ClangIdentity::capture().unwrap();let mut records=Vec::new();
 let bu=base["units"].as_array().unwrap();let tu=tip["units"].as_array().unwrap();let entries=spec["entries"].as_array().unwrap();assert_eq!(bu.len(),tu.len());assert_eq!(bu.len(),entries.len());
 for ((b,t),held) in bu.iter().zip(tu).zip(entries){
  for key in ["library","file","compile_configuration"] {assert_eq!(b[key],t[key]);}
  let artifact=format!("{}/{}",b["library"].as_str().unwrap(),b["file"].as_str().unwrap());assert_eq!(held["artifact"],artifact);
  let entry=CompileEntry{directory:held["entry"]["directory"].as_str().unwrap().into(),file:held["entry"]["file"].as_str().unwrap().into(),command:held["entry"]["command"].as_str().unwrap().into()};let raw=fs::read_to_string(&entry.file).unwrap();let options=compiler_options(&entry).unwrap();
  let unit=C11TranslationUnit{source:&raw,include_paths:options.include_paths,definitions:options.definitions,preprocessor_options:options.preprocessor_options,dialect:options.dialect,function_scope:LegacyFunctionScope::MainSourceDefinitions,input:options.input};
  let old=format!("{:?}",(&unit,&bc,&compiler));let new=format!("{:?}",(&unit,&tc,&compiler));let old_hash=qcore_schemas::sha256_hex(old.as_bytes());let new_hash=qcore_schemas::sha256_hex(new.as_bytes());
  assert_eq!(b["functions"].as_array().unwrap().len(),t["functions"].as_array().unwrap().len());
  for (x,y) in b["functions"].as_array().unwrap().iter().zip(t["functions"].as_array().unwrap()){
   assert_eq!(x["function"],y["function"]);assert_eq!(x["hypothesis_context"]["build_identity"],old_hash,"BASE full Debug preimage {artifact}");assert_eq!(y["hypothesis_context"]["build_identity"],new_hash,"TIP full Debug preimage {artifact}");
  }
  let wrong=format!("{:?}",(&unit,PathBuf::from("/unprovided-historical-cwd"),&compiler));assert_ne!(qcore_schemas::sha256_hex(wrong.as_bytes()),old_hash);assert_ne!(qcore_schemas::sha256_hex(wrong.as_bytes()),new_hash);
  records.push(json!({"artifact":artifact,"entry":held["entry"],"raw_source_sha256":qcore_schemas::sha256_hex(raw.as_bytes()),"unit_debug":format!("{unit:?}"),"clang_debug":format!("{compiler:?}"),"base_cwd":bc,"tip_cwd":tc,"base_preimage":old,"tip_preimage":new,"base_build":old_hash,"tip_build":new_hash,"wrong_cwd_negative_rejected":true}));
 }
 let hashes=["base_diagnostics","tip_diagnostics","base_invocation","tip_invocation"].into_iter().map(|k|(k,qcore_schemas::sha256_hex(&fs::read(spec[k].as_str().unwrap()).unwrap()))).collect::<std::collections::BTreeMap<_,_>>();
 fs::write(&args[2],serde_json::to_vec(&json!({"schema":"historical-debug-build.v1","records":records,"input_sha256":hashes,"scope":"Actual full production Debug preimages; sole independently held invocation cwd differs; no source/graph/value inverse."})).unwrap()).unwrap();
}
