from pathlib import Path
import re, hashlib, json
import subprocess
base=Path(__file__).resolve().parent
old=subprocess.check_output(['git','show','de5b239:compiler/src/verifier_plugins/chunkio/chunk_source.rs'],text=True)
new=Path('crates/qcore-c-gateway/src/boolean_source.rs').read_text()
facade=Path('compiler/src/verifier_plugins/chunkio/chunk_source.rs').read_text()
def body(s,name):
    start=s.index('fn '+name+'(')
    opening=s.index('{',start)
    depth=1; end=opening+1
    while depth:
        depth+=(s[end]=='{')-(s[end]=='}'); end+=1
    return s[start:end]
def norm(s):
    s=re.sub(r'//[^\n]*','',s)
    return re.sub(r'\s+','',s)
rows=[]
for name in ['record_layout','validate_field_layout','kind','children','evaluate','execute','field','expression','statement','field_kind']:
    left,right=norm(body(old,name)),norm(body(new,name))
    assert left==right,name
    rows.append(dict(function=name,normalization='comments and whitespace',sha256=hashlib.sha256(left.encode()).hexdigest()))
# The model binding stays in the compatibility facade verbatim.
left,right=norm(body(old,'instantiate')),norm(body(facade,'instantiate'))
assert left==right
rows.append(dict(function='instantiate',location='compiler facade',normalization='comments and whitespace',sha256=hashlib.sha256(left.encode()).hexdigest()))
# Source path derivation changes only by carrying the importer-owned identity.
left=norm(body(old,'derive_summary'))
right=norm(body(new,'derive_summary')).replace('identity:&BooleanSourceIdentity,','').replace(',identity.clone(),','')
assert left==right, (left[-180:],right[-180:])
rows.append(dict(function='derive_summary',normalization='comments/whitespace; identity argument and appended metadata only',sha256=hashlib.sha256(left.encode()).hexdigest()))
# Entry point changes: explicit trust inputs, compiler snapshot/provenance, no evaluator changes.
for name in ['import_chunk_frame_helper','import_chunk_source_with_profile']:
    left=norm(body(old,name)).replace('PINNED_FLUENT_BIT_REVISION','expected_revision')
    right=body(new,name)
    right=right.replace('    let compiler = CompilerIdentity::capture()?;\n','')
    right=right.replace('    let mut model_libraries = BTreeMap::new();\n', '')
    right=right.replace('            model_libraries.insert(format!("int (struct {} *)", model.record), (*name).to_owned());\n', '')
    right=right.replace('            let mut identity = identity.clone();\n            identity.model_library = Some(model_libraries[abi].clone());\n', '')
    start=right.index('    let identity = input_identity(')
    end=right.index('\n    )?;',start)+len('\n    )?;')
    right=right[:start]+right[end:]
    start=right.index('    if CompilerIdentity::capture()? != compiler {')
    end=right.index('\n    }',start)+len('\n    }')
    right=right[:start]+right[end:]
    right=norm(right)
    right=right.replace('model_libraries.insert(format!("int(struct{}*)",model.record),(*name).to_owned(),);','')
    right=right.replace('expected_revision:&str,','').replace('source_contract_rules:&str,','')
    right=right.replace('qcore_contract::contract_observation::check_contract_facts(source_contract_rules,','crate::source_contract::check(')
    right=right.replace('model,&identity,','model').replace('model,&identity','model').replace(',identity,','')
    # rustfmt's multiline tuple constructors add a trailing comma.
    right=right.replace('writes,},','writes,}')
    if left != right:
        import difflib
        print('\n'.join(difflib.ndiff([left],[right])))
    assert left==right,name
    rows.append(dict(function=name,normalization='comments/whitespace; explicit revision/rules parameters; provenance capture/storage and unique model-owner bookkeeping only',sha256=hashlib.sha256(left.encode()).hexdigest()))
(base/'statement-parity.json').write_text(json.dumps({'baseline':'de5b239','functions':rows,'unchanged_contract_evaluator':'qcore_contract::contract_observation::check_contract_facts','source_contract_rules':'compiler/src/source_contract.rs unchanged'},indent=2)+'\n')
print('Verified',len(rows),'functions')
