from pathlib import Path
import subprocess,json,hashlib
root=Path.cwd();p=root/'private/w48-cmt-member-storage';out=root/'docs/reports/w48-cmt-list-self-init/integration';module='providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv'
# The legacy symbolic checkpoint stays frozen. Retain the exact intentional
# rule37 replacement as a separately pinned, reversible historical seam.
old=subprocess.check_output(['git','show','f02dd8779:'+module],text=True);new=(root/module).read_text();delimiter='extern fn rd_strndup('
before=old[:old.index(delimiter)];after=new[:new.index(delimiter)];assert before!=after
for label,text in [('before',before),('after',after)]:
 (out/f'rule37-memory-{label}-prefix.txt').write_text(text)
sha=lambda text:hashlib.sha256(text.encode()).hexdigest()
(out/'rule37-memory-transition.json').write_text(json.dumps({'module':module,'before_source':'f02dd8779','after_source':'81df98c9d8edb2ca83c2478262c3afb90cadd9d7','before_prefix_sha256':sha(before),'after_prefix_sha256':sha(after),'before_module_sha256':sha(old),'after_module_sha256':sha(new),'meaning':'Exactly rd_calloc/rd_strdup become checked non-null specifications under asserted_allocation, with the accompanying explanatory comments. No frozen checkpoint is changed; actual graph/binding/lifecycle checks still use actual current source.'},indent=2)+'\n')
f=root/'crates/qcore-contract/tests/production_symbolic_adoption.rs';s=f.read_text();needle='    const CJSON: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv";\n';assert s.count(needle)==1
addition=f'''    // Rule 37 intentionally replaces these two inheritance declarations.
    // Recover only the byte-erasure checkpoint through this exact pinned seam;
    // all graph/loader/binding/flow/lifecycle checks still read current bytes.
    const MEMORY: &str = "{module}";
    if path == MEMORY {{
        let before = include_str!("../../../docs/reports/w48-cmt-list-self-init/integration/rule37-memory-before-prefix.txt");
        let after = include_str!("../../../docs/reports/w48-cmt-list-self-init/integration/rule37-memory-after-prefix.txt");
        assert_eq!(qcore_schemas::sha256_hex(before.as_bytes()), "{sha(before)}");
        assert_eq!(qcore_schemas::sha256_hex(after.as_bytes()), "{sha(after)}");
        assert!(current.starts_with(after), "rule37 source prefix changed");
        assert_eq!(current.matches(after).count(), 1);
        return current.replacen(after, before, 1);
    }}
'''
s=s.replace(needle,addition+needle);f.write_text(s)
f=root/'compiler/tests/contract_loader.rs';s=f.read_text();needle='    // Explicitly borrowed trailing arguments were implicit in the old table.\n';assert s.count(needle)==1
addition='''    // Rule 37 deliberately replaces only these nullable inherited modes.
    // Their stronger result is a body-checked specification under the named
    // allocation premise; it must never become an unchecked non-null fact.
    let checked = qcore_compiler::contract_loader::declared_checked_specifications(&contracts).unwrap();
    let premises = qcore_compiler::contract_loader::declared_premises(&contracts).unwrap();
    for name in ["rd_calloc", "rd_strdup"] {
        assert!(checked.contains(name), "{name} must remain body-checked");
        assert!(premises[name].iter().any(|p| p.name == "asserted_allocation"));
        old.insert_contract(name, ResourceFunctionContract::new([], R::Owned { nullable: false }));
    }
'''
s=s.replace(needle,addition+needle);f.write_text(s)
f=root/'scripts/tests/test_contract_corpus.py';s=f.read_text();s=s.replace('            (rdkafka, "rd_calloc", "libc::calloc"),\n','').replace('            (rdkafka, "rd_strdup", "libc::strdup"),\n','');needle='        self.assertEqual(references, [\n';assert s.count(needle)==1;s=s.replace(needle,'        # Rule37: rd_calloc/rd_strdup are checked specifications, not inheritance.\n'+needle);f.write_text(s)
print('Updated exactly three integration expectations; runtime, contracts and proofs unchanged.')
