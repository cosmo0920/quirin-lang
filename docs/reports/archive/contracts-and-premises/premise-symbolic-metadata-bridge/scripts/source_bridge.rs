//! Independently reconstruct additive provenance; serialized receipts are output only.
use qcore_contract::premise_metadata_bridge::{self, CheckedBridge};
use qcore_schemas::{
    contract::{ContractInclusionEvidence, ContractModuleIdentity, ResourceContracts},
    premise_flow::View,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn sha(bytes: impl AsRef<[u8]>) -> String {
    qcore_schemas::sha256_hex(bytes.as_ref())
}
fn bytes(path: &Value) -> Vec<u8> {
    fs::read(path.as_str().expect("held path")).unwrap()
}
fn relative(key: &str, root: &Path) -> String {
    if let Some((krate, module)) = key.split_once(':') {
        assert!(!krate.contains('/') && !krate.is_empty());
        return format!("providers/{krate}/contracts/{module}");
    }
    let actual = Path::new(key).canonicalize().expect("held source path");
    actual
        .strip_prefix(root.canonicalize().unwrap())
        .expect("source inside held snapshot")
        .to_string_lossy()
        .into_owned()
}
fn keys(map: &BTreeMap<String, String>, root: &Path) -> BTreeMap<String, String> {
    {
        let out = map
            .keys()
            .map(|k| (relative(k, root), k.clone()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            out.len(),
            map.len(),
            "source key normalization must be injective"
        );
        out
    }
}
fn identity(
    id: &mut ContractModuleIdentity,
    old: &ContractModuleIdentity,
    raw: &BTreeMap<String, (String, String)>,
) {
    let path = format!("providers/{}/contracts/{}", id.crate_name, id.module);
    assert_eq!(id.crate_name, old.crate_name);
    assert_eq!(id.crate_version, old.crate_version);
    assert_eq!(id.module, old.module);
    if id.sha256 != old.sha256 {
        let pair = &raw[&path];
        assert_eq!(id.sha256, pair.1);
        assert_eq!(old.sha256, pair.0);
        id.sha256 = old.sha256.clone();
    }
}
pub struct SourceBridge {
    checked: CheckedBridge,
    modules: BTreeMap<String, View>,
    old_root: PathBuf,
    new_root: PathBuf,
    pub pairs: Vec<Value>,
    pub hashes: Vec<Value>,
    pub file_pairs: Vec<Value>,
    pub bounds_old: String,
    pub bounds_new: String,
    pub bounds_proof: Value,
    pub receipt: Value,
}
impl SourceBridge {
    fn record_source(&mut self, module: &str, before: &str, after: &str, kind: &str) {
        if before == after
            || self
                .pairs
                .iter()
                .any(|p| p["module"] == module && p["before"] == before && p["after"] == after)
        {
            return;
        }
        self.pairs.push(json!({"module":module,"kind":kind,"before":before,"after":after,"before_sha256":sha(before),"after_sha256":sha(after),"proof":"sealed exact deterministic metadata byte insertion/erasure"}));
        self.hashes.push(json!({"before":sha(before),"after":sha(after),"kind":"checked_source_sha256","allowed_field_kinds":["sha256","module_sha256","source_sha256"],"module":module}));
    }
    fn inclusion(
        &mut self,
        before: &BTreeMap<String, ContractInclusionEvidence>,
        after: &BTreeMap<String, ContractInclusionEvidence>,
        raw: &BTreeMap<String, (String, String)>,
    ) {
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        );
        for (key, new) in after {
            let old = &before[key];
            let mut restored = new.clone();
            identity(&mut restored.module, &old.module, raw);
            if new.source != old.source {
                let module = format!(
                    "providers/{}/contracts/{}",
                    old.module.crate_name, old.module.module
                );
                assert_eq!(sha(&old.source), raw[&module].0);
                assert_eq!(sha(&new.source), raw[&module].1);
                self.record_source(&module, &old.source, &new.source, "raw-inclusion-module");
                restored.source = old.source.clone();
            }
            assert_eq!(
                new.included.keys().collect::<Vec<_>>(),
                old.included.keys().collect::<Vec<_>>()
            );
            for (k, parent) in &mut restored.included {
                let expected = &old.included[k];
                let module = format!(
                    "providers/{}/contracts/{}",
                    parent.identity.crate_name, parent.identity.module
                );
                identity(&mut parent.identity, &expected.identity, raw);
                if parent.source != expected.source {
                    assert_eq!(sha(&expected.source), raw[&module].0);
                    assert_eq!(sha(&parent.source), raw[&module].1);
                    self.record_source(
                        &module,
                        &expected.source,
                        &parent.source,
                        "raw-included-parent",
                    );
                    parent.source = expected.source.clone();
                }
            }
            assert_eq!(
                restored.inherited.keys().collect::<Vec<_>>(),
                old.inherited.keys().collect::<Vec<_>>()
            );
            for (name, chain) in &mut restored.inherited {
                let expected = &old.inherited[name];
                assert_eq!(chain.len(), expected.len());
                for (link, old_link) in chain.iter_mut().zip(expected) {
                    assert_eq!(link.declaration, old_link.declaration);
                    identity(&mut link.module, &old_link.module, raw);
                }
            }
            assert_eq!(
                &restored, old,
                "complete inclusion record beyond checked raw/source identities changed"
            );
        }
    }
    pub fn new(spec: &Value, old: &ResourceContracts, new: &ResourceContracts) -> Self {
        let old_root = PathBuf::from(spec["base_inputs_root"].as_str().unwrap());
        let new_root = PathBuf::from(spec["tip_inputs_root"].as_str().unwrap());
        let plan_bytes = bytes(&spec["metadata_plan"]);
        let expected = spec["metadata_plan_sha256"]
            .as_str()
            .expect("independent frozen inventory hash");
        assert_eq!(sha(&plan_bytes), expected);
        let plan: Value = serde_json::from_slice(&plan_bytes).unwrap();
        let mut before = BTreeMap::new();
        let mut after = BTreeMap::new();
        let mut modules = BTreeMap::new();
        let mut raw = BTreeMap::new();
        let mut file_pairs = Vec::new();
        for module in plan["files"].as_array().unwrap() {
            let path = module["path"].as_str().unwrap();
            assert!(
                !Path::new(path).is_absolute()
                    && !Path::new(path)
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
            );
            let b = fs::read_to_string(old_root.join(path)).unwrap();
            let t = fs::read_to_string(new_root.join(path)).unwrap();
            assert_eq!(sha(&b), module["sha256"]);
            let view: View =
                serde_json::from_value(module["declarations"][0]["view"].clone()).unwrap();
            modules.insert(path.into(), view);
            raw.insert(path.into(), (sha(&b), sha(&t)));
            before.insert(path.into(), b);
            after.insert(path.into(), t);
        }
        let checked = premise_metadata_bridge::check(&plan_bytes, expected, &before, &after)
            .expect("actual8rawsource bridge");
        let checked_receipt = serde_json::to_value(&checked).unwrap();
        let mut bridge = Self {
            checked,
            modules,
            old_root,
            new_root,
            pairs: Vec::new(),
            hashes: Vec::new(),
            file_pairs: Vec::new(),
            bounds_old: String::new(),
            bounds_new: String::new(),
            bounds_proof: Value::Null,
            receipt: checked_receipt,
        };
        for (module, b) in &before {
            bridge.record_source(module, b, &after[module], "raw-module");
            file_pairs.push(json!({"relative":module,"before_sha256":sha(b),"after_sha256":sha(&after[module]),"kind":"checked_raw_metadata_source"}));
        }
        bridge.file_pairs = file_pairs;
        qcore_contract::contract_loader::check_source_assumptions(old).unwrap();
        qcore_contract::contract_loader::check_source_assumptions(new).unwrap();
        let bk = keys(old.source_assumptions(), &bridge.old_root);
        let tk = keys(new.source_assumptions(), &bridge.new_root);
        assert_eq!(bk.keys().collect::<Vec<_>>(), tk.keys().collect::<Vec<_>>());
        for (module, bkey) in &bk {
            let tkey = &tk[module];
            let b = &old.source_assumptions()[bkey];
            let t = &new.source_assumptions()[tkey];
            if b != t {
                assert_eq!(bridge.modules[module], View::Resource);
                bridge
                    .checked
                    .check_expanded(module, View::Resource, b, &sha(b), t)
                    .expect("exact actual elaborated source pair");
                bridge.record_source(module, b, t, "loaded-resource-source");
            }
        }
        assert_eq!(
            qcore_contract::hypothesis::replay(old.source_assumptions()).unwrap(),
            qcore_contract::hypothesis::replay(new.source_assumptions()).unwrap(),
            "complete original graphs unchanged"
        );
        bridge.inclusion(old.inclusion_evidence(), new.inclusion_evidence(), &raw);
        // Every file-level pin is derived from independently held actual source/body
        // records; no table supplied by a capture authorizes a digest change.
        let mut contracts = BTreeMap::<(String, String), (String, String)>::new();
        let old_d =
            qcore_contract::contract_revision::declaration_digests(old.source_assumptions())
                .unwrap();
        let new_d =
            qcore_contract::contract_revision::declaration_digests(new.source_assumptions())
                .unwrap();
        assert_eq!(
            old_d.keys().collect::<Vec<_>>(),
            new_d.keys().collect::<Vec<_>>()
        );
        for (name, (module, b)) in &old_d {
            let (tmodule, t) = &new_d[name];
            let path = relative(module, &bridge.old_root);
            assert_eq!(path, relative(tmodule, &bridge.new_root));
            if b != t {
                assert_eq!(bridge.modules[&path], View::Resource);
                contracts.insert((path, name.clone()), (b.clone(), t.clone()));
            }
        }
        let mut bounds = Vec::new();
        let mut bounds_sources = Vec::new();
        let mut pin_records = Vec::new();
        for (root, invpath) in [
            (&bridge.old_root, &spec["base_invocation"]),
            (&bridge.new_root, &spec["tip_invocation"]),
        ] {
            let inv: Value = serde_json::from_slice(&bytes(invpath)).unwrap();
            assert_eq!(
                inv["inputs_snapshot"].as_str(),
                Some(root.to_str().unwrap())
            );
            let source_root = Path::new(inv["source_root"].as_str().unwrap());
            let profile = root.join("contract/profiles/fluent-bit");
            let pf: Value =
                serde_json::from_slice(&fs::read(profile.join("profile.json")).unwrap()).unwrap();
            let manifest = profile.join(pf["resource_contract_manifest"].as_str().unwrap());
            let manifest_text = fs::read_to_string(&manifest).unwrap();
            let load = qcore_audit::bounds_selection::manifest_load_path(&manifest, &manifest_text)
                .unwrap()
                .unwrap();
            let loaded =
                qcore_contract::load_path_manifest::load_contract_load_path_manifest(&load)
                    .unwrap();
            let (mut env, _) = qcore_audit::bounds_selection::load_views(&loaded).unwrap();
            let pins =
                qcore_audit::bounds_selection::apply_bounds_pins(&mut env, &loaded, source_root)
                    .unwrap();
            let sources = loaded
                .bounds_views
                .iter()
                .flat_map(|(krate, view)| {
                    let owner = loaded.load_path.crate_named(krate).unwrap();
                    view.iter().map(move |module| {
                        (format!("{krate}:{module}"), owner.modules[module].clone())
                    })
                })
                .collect::<BTreeMap<_, _>>();
            bounds_sources.push(sources);
            pin_records.push(pins);
            bounds.push(env);
        }
        let b = &bounds[0];
        let t = &bounds[1];
        assert_eq!(
            b.functions.keys().collect::<Vec<_>>(),
            t.functions.keys().collect::<Vec<_>>()
        );
        let mut restored = t.clone();
        restored.sources = b.sources.clone();
        let mut text_records = Vec::new();
        assert_eq!(
            qcore_contract::bounds_hypotheses::replay(&bounds_sources[0]).unwrap(),
            qcore_contract::bounds_hypotheses::replay(&bounds_sources[1]).unwrap()
        );
        for (name, entry) in &b.functions {
            let tip = &t.functions[name];
            let path = relative(&entry.module, &bridge.old_root);
            assert_eq!(path, relative(&tip.module, &bridge.new_root));
            if entry.text_sha256 != tip.text_sha256 {
                assert_eq!(bridge.modules[&path], View::Bounds);
                let before = &bounds_sources[0][&entry.module];
                let after = &bounds_sources[1][&tip.module];
                bridge
                    .checked
                    .check_expanded(&path, View::Bounds, before, &sha(before), after)
                    .expect("exact held bounds metadata source");
                bridge.record_source(&path, before, after, "loaded-bounds-source");
                contracts.insert(
                    (path.clone(), name.clone()),
                    (entry.text_sha256.clone(), tip.text_sha256.clone()),
                );
                text_records.push(json!({"module":path,"function":name,"before_text_sha256":entry.text_sha256,"after_text_sha256":tip.text_sha256,"canonical_sha256":entry.canonical_sha256}));
                restored.functions.get_mut(name).unwrap().text_sha256 = entry.text_sha256.clone();
            }
        }
        assert_eq!(
            &restored, b,
            "complete bounds interpreted/canonical/review/hold semantics unchanged"
        );
        bridge.bounds_old = b.digest();
        bridge.bounds_new = t.digest();
        bridge.bounds_proof = json!({"before":b.digest(),"after":t.digest(),"before_sources":b.sources,"after_sources":t.sources,"refuted":b.refuted,"changed_text_records":text_records,"before_pin_records":pin_records[0],"after_pin_records":pin_records[1],"complete_semantics":"equal after only proven textual-source metadata identities","before_functions_debug":format!("{:?}",b.functions),"after_functions_debug":format!("{:?}",t.functions)});
        bridge.hashes.push(json!({"before":b.digest(),"after":t.digest(),"kind":"bounds_raw_source_environment_digest","allowed_field_kinds":["contracts_sha256","environment_sha256"]}));
        for ((module, function), (before, after)) in &contracts {
            bridge.hashes.push(json!({"before":before,"after":after,"kind":"exact_declaration_token_preimage","allowed_field_kinds":["contract_sha256","text_sha256"],"module":module,"function":function}));
        }
        for file in [
            "providers/qcore-provider-librdkafka/contracts.lock",
            "providers/qcore-provider-librdkafka/bounds-contracts.lock",
            "providers/quirin-provider-fluent-bit/contracts.lock",
            "providers/quirin-provider-libc/contracts.lock",
        ] {
            let bbytes = fs::read(bridge.old_root.join(file)).unwrap();
            let tbytes = fs::read(bridge.new_root.join(file)).unwrap();
            qcore_audit::contract_revision::ContractLock::read(&bridge.old_root.join(file))
                .unwrap();
            qcore_audit::contract_revision::ContractLock::read(&bridge.new_root.join(file))
                .unwrap();
            let bv: Value = serde_json::from_slice(&bbytes).unwrap();
            let mut tv: Value = serde_json::from_slice(&tbytes).unwrap();
            for kind in ["pinned", "flexible"] {
                let old = bv[kind].as_array().unwrap();
                let tip = tv[kind].as_array_mut().unwrap();
                assert_eq!(old.len(), tip.len());
                for (be, te) in old.iter().zip(tip) {
                    assert_eq!(be["function"], te["function"]);
                    assert_eq!(be["module"], te["module"]);
                    if be["contract_sha256"] != te["contract_sha256"] {
                        let module = format!(
                            "{}/contracts/{}",
                            Path::new(file).parent().unwrap().display(),
                            be["module"].as_str().unwrap()
                        );
                        let pair = &contracts[&(module, be["function"].as_str().unwrap().into())];
                        assert_eq!(be["contract_sha256"], pair.0);
                        assert_eq!(te["contract_sha256"], pair.1);
                        te["contract_sha256"] = be["contract_sha256"].clone();
                    }
                }
            }
            assert_eq!(
                bv, tv,
                "whole lock source/revision/body identities and unknown fields unchanged"
            );
            let mut restored_raw = String::from_utf8(tbytes.clone()).unwrap();
            for ((_, _), (before, after)) in &contracts {
                restored_raw = restored_raw.replace(
                    &format!("\"contract_sha256\": \"{after}\""),
                    &format!("\"contract_sha256\": \"{before}\""),
                );
            }
            assert_eq!(
                restored_raw.as_bytes(),
                bbytes.as_slice(),
                "all lock bytes beyond selected contract digests unchanged"
            );
            bridge.file_pairs.push(json!({"relative":file,"before_sha256":sha(&bbytes),"after_sha256":sha(&tbytes),"kind":"typed_lock_only_selected_contract_digest_changes"}));
            bridge.hashes.push(json!({"before":sha(&bbytes),"after":sha(&tbytes),"kind":"complete_validated_lock_file_preimage","allowed_field_kinds":["sha256"]}));
        }
        let file = "providers/qcore-provider-librdkafka/contracts.json";
        let bb = fs::read(bridge.old_root.join(file)).unwrap();
        let tb = fs::read(bridge.new_root.join(file)).unwrap();
        let bv: Value = serde_json::from_slice(&bb).unwrap();
        let mut tv: Value = serde_json::from_slice(&tb).unwrap();
        for (path, (old, new)) in &raw {
            if let Some(relative) = path.strip_prefix("providers/qcore-provider-librdkafka/") {
                assert_eq!(bv["contracts"][relative], old.as_str());
                assert_eq!(tv["contracts"][relative], new.as_str());
                tv["contracts"][relative] = bv["contracts"][relative].clone();
            }
        }
        assert_eq!(
            bv, tv,
            "package inventory only six actual source digests may change"
        );
        let mut restored_raw = String::from_utf8(tb.clone()).unwrap();
        for (path, (before, after)) in &raw {
            if let Some(relative) = path.strip_prefix("providers/qcore-provider-librdkafka/") {
                restored_raw = restored_raw.replace(
                    &format!("\"{relative}\": \"{after}\""),
                    &format!("\"{relative}\": \"{before}\""),
                );
            }
        }
        assert_eq!(
            restored_raw.as_bytes(),
            bb.as_slice(),
            "all inventory bytes beyond six source digests unchanged"
        );
        bridge.file_pairs.push(json!({"relative":file,"before_sha256":sha(&bb),"after_sha256":sha(&tb),"kind":"six_checked_raw_module_digests_only"}));
        bridge.hashes.push(json!({"before":sha(&bb),"after":sha(&tb),"kind":"complete_validated_package_inventory","allowed_field_kinds":["sha256"]}));
        bridge
    }
    pub fn approved_source(&self, before: &str, after: &str) -> bool {
        before == after
            || self
                .pairs
                .iter()
                .any(|p| p["before"] == before && p["after"] == after)
    }
    pub fn json(&self) -> Value {
        json!({"schema":"checked-symbolic-native-input-bridge.v1","checked_raw":self.receipt,"source_pairs":self.pairs,"digest_pairs":self.hashes,"file_pairs":self.file_pairs,"bounds":self.bounds_proof,"bounds_digest_before":self.bounds_old,"bounds_digest_after":self.bounds_new,"all_original_graphs":"exact","source_truth":"open","fields_deleted":0})
    }
}
