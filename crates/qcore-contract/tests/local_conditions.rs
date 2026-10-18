use qcore_contract::local_condition;
use qcore_frontend::frontend::parse_source_program;

fn read(body:&str)->Result<qcore_schemas::loop_lifetime::LoopLifetimeContract,String>{
    let source=format!("module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract {{ requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); {body} }};");
    let p=parse_source_program(&source).map_err(|e|e.to_string())?;
    local_condition::read(&p.declarations[0].value)
}
#[test] fn complementary_prerequisites_keep_their_own_polarities(){
    let p=read("quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]]; quant not #threaded in { requires resource_premise(instance_config, \"The setter uses this instance's config\"); }: [[local]];").unwrap();
    assert_eq!(p.bindings[0].parameter,0);assert_eq!(p.bindings[0].members,["is_threaded"]);
    assert!(!p.blocks[0].guard.negated);assert!(p.blocks[1].guard.negated);assert_eq!(p.blocks[0].clauses.len(),1);
}
#[test] fn registered_binder_is_not_a_condition_formal(){
    let p=read("quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;").unwrap();assert_eq!(p.registered[0].record,"flb_input_collector");
    assert!(read("quant #threaded = (#coll.instance.is_threaded != 0); quant #threaded in { quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl; }: [[local]];").unwrap_err().contains("host formal"));
}
#[test] fn unsupported_conditional_effects_and_selectors_refuse(){
    for inner in ["requires resource_contract_v1;","ensures normal resource_borrowed(ins);","requires resource_review(\"x\");","quant not #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];"]{
        let text=format!("quant #threaded = (#ins.is_threaded != 0); quant #threaded in {{ {inner} }}: [[local]];");assert!(read(&text).is_err(),"{inner}");
    }
}
#[test] fn misspelled_scope_unbound_and_duplicate_conditions_refuse(){
    for text in [
        "quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[runtime]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]];",
        "quant #threaded = (#ins.is_threaded != 0); quant #threaded in { }: [[local]];",
    ]{assert!(read(text).is_err(),"{text}");}
}
#[test] fn mismatched_footprint_records_and_duplicate_interiors_refuse(){
    for text in ["quant registered #c: collector.event |-> config.evl;","quant registered #c: collector.event |-> collector.evl; quant registered #d: collector.event |-> collector.other;"]{assert!(read(text).is_err());}
}

#[test] fn resource_loader_withholds_the_entire_scoped_contract_and_replays_it(){
    use qcore_contract::contract_loader::{load_resource_contract_sources,check_source_assumptions};
    let source="module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, mutate); }: [[local]]; };";
    let env=load_resource_contract_sources(&[("local.qv".into(),source.into())].into()).unwrap();
    assert!(env.contract("f").is_none());assert!(env.retaining_contract("f").is_none());
    let lifetime=env.lifetime_contract("f").unwrap().lifetime.as_ref().unwrap();assert_eq!(lifetime.blocks.len(),1);
    check_source_assumptions(&env).unwrap();
}
#[test] fn malformed_inner_existing_obligation_is_not_accepted_as_text(){
    use qcore_contract::contract_loader::load_resource_contract_sources;
    let source="module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in { requires resource_loop_owner(ins, invented_mode); }: [[local]]; };";
    assert!(load_resource_contract_sources(&[("local.qv".into(),source.into())].into()).is_err());
}
