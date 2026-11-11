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

use qcore_contract::local_condition;
use qcore_frontend::frontend::parse_source_program;

fn read(body: &str) -> Result<qcore_schemas::loop_lifetime::LoopLifetimeContract, String> {
    let source = format!(
        "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract {{ requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); {body} }};"
    );
    let p = parse_source_program(&source).map_err(|e| e.to_string())?;
    local_condition::read(&p.declarations[0].value)
}
#[test]
fn complementary_prerequisites_keep_their_own_polarities() {
    let p=read("quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]]; quant not #threaded in { requires resource_premise(instance_config, \"The setter uses this instance's config\"); }: [[local]];").unwrap();
    assert_eq!(p.bindings[0].parameter, 0);
    assert_eq!(p.bindings[0].members, ["is_threaded"]);
    assert!(!p.blocks[0].guard.negated);
    assert!(p.blocks[1].guard.negated);
    assert_eq!(p.blocks[0].clauses.len(), 1);
}
#[test]
fn registered_binder_is_not_a_condition_formal() {
    let p = read("quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;")
        .unwrap();
    assert_eq!(p.registered[0].record, "flb_input_collector");
    assert!(read("quant #threaded = (#coll.instance.is_threaded != 0); quant #threaded in { quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl; }: [[local]];").unwrap_err().contains("host formal"));
}
#[test]
fn unsupported_conditional_effects_and_selectors_refuse() {
    for inner in [
        "requires resource_contract_v1;",
        "ensures normal resource_borrowed(ins);",
        "requires resource_review(\"x\");",
        "quant not #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];",
    ] {
        let text = format!(
            "quant #threaded = (#ins.is_threaded != 0); quant #threaded in {{ {inner} }}: [[local]];"
        );
        assert!(read(&text).is_err(), "{inner}");
    }
}
#[test]
fn misspelled_scope_unbound_and_duplicate_conditions_refuse() {
    for text in [
        "quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[runtime]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded in { }: [[local]];",
    ] {
        assert!(read(text).is_err(), "{text}");
    }
}
#[test]
fn mismatched_footprint_records_and_duplicate_interiors_refuse() {
    for text in [
        "quant registered #c: collector.event |-> config.evl;",
        "quant registered #c: collector.event |-> collector.evl; quant registered #d: collector.event |-> collector.other;",
    ] {
        assert!(read(text).is_err());
    }
}

#[test]
fn resource_loader_withholds_the_entire_scoped_contract_and_replays_it() {
    use qcore_contract::contract_loader::{
        check_source_assumptions, load_resource_contract_sources,
    };
    let source = "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]]; };";
    let env = load_resource_contract_sources(&[("local.qv".into(), source.into())].into()).unwrap();
    assert!(env.contract("f").is_none());
    assert!(env.retaining_contract("f").is_none());
    let lifetime = env
        .lifetime_contract("f")
        .unwrap()
        .lifetime
        .as_ref()
        .unwrap();
    assert_eq!(lifetime.blocks.len(), 1);
    check_source_assumptions(&env).unwrap();
}
#[test]
fn malformed_inner_existing_obligation_is_not_accepted_as_text() {
    use qcore_contract::contract_loader::load_resource_contract_sources;
    let source = "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, invented_mode); }: [[local]]; };";
    assert!(load_resource_contract_sources(&[("local.qv".into(), source.into())].into()).is_err());
}

#[test]
fn scoped_premises_keep_whole_declaration_name_uniqueness() {
    for body in [
        "quant #c = (#ins.is_threaded != 0); quant #c in { requires resource_premise(same, \"A\"); requires resource_premise(same, \"B\"); }: [[local]];",
        "quant #c = (#ins.is_threaded != 0); quant #c in { requires resource_premise(same, \"A\"); }: [[local]]; quant not #c in { requires resource_premise(same, \"B\"); }: [[local]];",
        "requires resource_premise(same, \"A\"); quant #c = (#ins.is_threaded != 0); quant #c in { requires resource_premise(same, \"B\"); }: [[local]];",
    ] {
        let error = read(body).unwrap_err();
        assert!(
            error.contains("premise") && (error.contains("duplicate") || error.contains("repeats")),
            "{error}"
        );
    }
}

fn guarded_environment() -> qcore_schemas::contract::ResourceContracts {
    qcore_contract::contract_loader::load_resource_contract_sources(&[("local.qv".into(), "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_premise(worker, \"The worker protocol holds\"); }: [[local]]; };".into())].into()).unwrap()
}

#[test]
fn guarded_table_is_exclusive_and_cannot_be_erased_by_an_overlay_or_forged_partition() {
    use qcore_contract::contract_loader::check_source_assumptions;
    use qcore_schemas::contract::{
        ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    };
    let env = guarded_environment();
    assert!(env.retaining().is_empty());
    assert_eq!(env.guarded_retaining().len(), 1);
    check_source_assumptions(&env).unwrap();
    let mut overlay = env.clone();
    overlay.insert_contract(
        "f",
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None),
    );
    assert!(overlay.contract("f").is_none());
    check_source_assumptions(&overlay).unwrap();
    assert_eq!(
        serde_json::to_value(&overlay).unwrap(),
        serde_json::to_value(&env).unwrap()
    );
    let mut missing_identity = serde_json::to_value(&env).unwrap();
    missing_identity["declared_guarded"] = serde_json::json!([]);
    let decoded: ResourceContracts = serde_json::from_value(missing_identity).unwrap();
    assert!(check_source_assumptions(&decoded).is_err());
    let mut forged = serde_json::to_value(&env).unwrap();
    forged["retaining"] = forged["guarded_retaining"].take();
    forged["guarded_retaining"] = serde_json::json!({});
    let decoded: ResourceContracts = serde_json::from_value(forged.clone()).unwrap();
    assert!(check_source_assumptions(&decoded).is_err());
    forged["source_assumptions"] = serde_json::json!({});
    let decoded: ResourceContracts = serde_json::from_value(forged).unwrap();
    assert!(
        check_source_assumptions(&decoded).is_err(),
        "partition must be checked even without text assumptions"
    );
    assert!(
        !serde_json::to_string(&ResourceContracts::default())
            .unwrap()
            .contains("guarded_retaining")
    );
}

#[test]
fn guarded_declarations_follow_withdrawal_carry_and_pin_hold_restoration() {
    use qcore_contract::contract_loader::check_source_assumptions;
    use qcore_schemas::{contract::ResourceContracts, contract_revision::PinMismatchReason};
    let env = guarded_environment();
    let mut copy = ResourceContracts::default();
    copy.carry_side_declarations(&env, |_| false);
    assert!(copy.retaining().is_empty());
    assert!(copy.lifetime_contract("f").is_some());
    copy.withdraw_declaration("f");
    assert!(copy.lifetime_contract("f").is_none());
    assert!(copy.has_guarded_interface("f"));
    copy.insert_contract("f", env.lifetime_contract("f").unwrap().contract.clone());
    assert!(copy.contract("f").is_none());
    assert!(copy.functions().is_empty());
    copy.carry_side_declarations(&env, |name| name == "f");
    assert!(copy.lifetime_contract("f").is_none());
    let mut held = env.clone();
    held.withhold_for_pin_mismatch(
        "f",
        "local.qv".into(),
        PinMismatchReason::NoLock,
        None,
        "unreviewed local fixture".into(),
    )
    .unwrap();
    assert!(held.lifetime_contract("f").is_none());
    assert!(held.retaining().is_empty());
    let mut imported = held.clone();
    imported.set_inferred_imports(
        [(
            "f".into(),
            env.lifetime_contract("f").unwrap().contract.clone(),
        )]
        .into(),
    );
    assert!(imported.contract("f").is_none());
    check_source_assumptions(&imported).unwrap();
    check_source_assumptions(&held).unwrap();
    let restored = held.with_human_holds_in_force().unwrap();
    assert!(restored.retaining().is_empty());
    assert!(restored.guarded_retaining().contains_key("f"));
    check_source_assumptions(&restored).unwrap();
}

#[test]
fn inheritance_preserves_the_guarded_table_and_replays_its_source_identity() {
    use qcore_contract::contract_inclusion::{
        ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
    };
    use qcore_contract::contract_loader::{
        PublishedModuleRef, check_source_assumptions, load_published_resource_contracts,
    };
    let parent = guarded_environment().source_assumptions()["local.qv"]
        .replace("extern fn f(", "extern fn original(");
    let child = "module contract; #include \"quirin-provider-base:base/local.qv\"; extern fn child(ins: ptr<mut, u8>) -> i32 contract inherit base::original;";
    let krate =
        |name: &str, dependencies: Vec<String>, module: &str, text: String, published: bool| {
            PublishedContractCrate {
                name: name.into(),
                namespace: derive_contract_namespace(name).unwrap(),
                version: "0.1.0".into(),
                dependencies: dependencies.into_iter().collect(),
                modules: [(module.into(), text)].into(),
                published: if published {
                    [module.into()].into()
                } else {
                    Default::default()
                },
            }
        };
    let path = ContractLoadPath::new([
        krate(
            "quirin-provider-base",
            vec![],
            "base/local.qv",
            parent,
            true,
        ),
        krate(
            "quirin-provider-child",
            vec!["quirin-provider-base".into()],
            "child/local.qv",
            child.into(),
            false,
        ),
    ])
    .unwrap();
    let env = load_published_resource_contracts(
        &path,
        &[PublishedModuleRef {
            crate_name: "quirin-provider-child".into(),
            module: "child/local.qv".into(),
        }],
    )
    .unwrap();
    assert!(env.retaining().is_empty());
    assert!(env.guarded_retaining().contains_key("child"));
    assert_eq!(
        env.lifetime_contract("child")
            .unwrap()
            .lifetime
            .as_ref()
            .unwrap()
            .blocks
            .len(),
        1
    );
    check_source_assumptions(&env).unwrap();
}
