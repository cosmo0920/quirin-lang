use qcore_c_gateway::legacy_c::*;
fn main(){
 let mut c=ResourceContracts::new([("q_alloc",ResourceApiKind::Acquire),("q_free",ResourceApiKind::Release)]);
 for (name,mode) in [("q_store",ResourceParameterMode::Stored{owner_parameter:0,nullable:false}),("q_remove",ResourceParameterMode::Removed{owner_parameter:0,nullable:false})] {c.insert_contract(name,ResourceFunctionContract::new([ResourceParameterMode::Owned{nullable:false},mode],ResourceReturnMode::None));}
 let src="void *q_alloc(void);void q_free(void*);void q_store(void*,void*);void q_remove(void*,void*); void subject(int flag){void*o=q_alloc();void*p=q_alloc();void*q=q_alloc();q_store(o,p);q_store(o,q);void*x;if(flag){x=p;}else{x=q;}q_remove(o,x);q_free(p);q_free(q);q_free(o);}";
 let out=analyze_unrestricted_c11(&C11TranslationUnit{source:src,..Default::default()},"subject",&c).unwrap();
 match out{UnrestrictedLegacyResult::Safe(cert)=>println!("SAFE certificate recheck: {:?}; summaries: {:?}",check_legacy_safety_certificate(&cert),cert.summaries),other=>println!("{:?}",other)};
}
