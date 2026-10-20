#!/usr/bin/env python3
"""Keep raw receipts and compare only explicitly attributed libc graph metadata."""
import argparse
from collections import Counter
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
CARD = ROOT/'docs/reports/premise-decomposition-cycle1b'
spec=importlib.util.spec_from_file_location('graph_comparison',ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py')
old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)

def gz(path,value):
    path.write_bytes(gzip.compress((json.dumps(value,indent=2)+'\n').encode(),mtime=0))

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('route');p.add_argument('before',type=Path);p.add_argument('after',type=Path);p.add_argument('out',type=Path);a=p.parse_args();a.out.mkdir(exist_ok=False,parents=True)
    mapping=json.loads((CARD/'mapping.json').read_text())
    roots=[str(ROOT/'private/premise-decomposition-cycle1b/baseline-source'),'/home/cosmo/.codex/worktrees/w48-bottom-up-batch4/qcore-c',str(ROOT)]
    locks={}
    for label,root in [('before',Path(roots[0])),('after',ROOT)]:
        lock=json.loads((root/'providers/quirin-provider-libc/contracts.lock').read_text());locks[label]={r['function']:r['contract_sha256'] for r in lock['flexible'] if r['function'] in mapping['declarations']}
    lock_bytes={label:(root/'providers/quirin-provider-libc/contracts.lock').read_bytes() for label,root in [('before',Path(roots[0])),('after',ROOT)]}
    restored_lock=lock_bytes['after']
    for function in mapping['declarations']:
        assert restored_lock.count(locks['after'][function].encode())==1
        restored_lock=restored_lock.replace(locks['after'][function].encode(),locks['before'][function].encode())
    assert restored_lock==lock_bytes['before'], 'lock bytes differ beyond the two official flexible identities'
    lock_hash={label:hashlib.sha256(raw).hexdigest() for label,raw in lock_bytes.items()}
    records=[]
    baseline_text=(Path(roots[0])/mapping['module']).read_text()
    migrated_text=(ROOT/mapping['module']).read_text()
    environments={}
    provenance=a.out/'provenance';provenance.mkdir()
    for label,raw in lock_bytes.items():
        (provenance/(label+'-libc-contracts.lock.gz')).write_bytes(gzip.compress(raw,mtime=0))
    (provenance/'lock-attribution.json').write_text(json.dumps({'sha256':lock_hash,'only_changed_functions':sorted(mapping['declarations']),'exact_bytes_equal_after_selected_digest_substitution':True},indent=2)+'\n')
    for label in ['before','after']:
        path=ROOT/('private/premise-decomposition-cycle1b/environment-'+label+'.json')
        if path.exists():
            raw=path.read_bytes();environments[label]={'value':json.loads(raw),'sha256':hashlib.sha256(raw).hexdigest()}
            (provenance/(label+'-environment.json.gz')).write_bytes(gzip.compress(raw,mtime=0))
    source_hash={}
    if environments:
        assert set(environments)=={'before','after'}
        for label,record in environments.items():
            raw=json.dumps(record['value']['source_assumptions'],separators=(',',':'),ensure_ascii=False).encode()
            source_hash[label]=hashlib.sha256(raw).hexdigest()

    def normalize(v):
        if isinstance(v,str):
            for root in roots:v=v.replace(root,'$ANALYZER')
            return v
        if isinstance(v,list):return [normalize(x) for x in v]
        if isinstance(v,dict):
            out={normalize(k):normalize(x) for k,x in v.items()};assert len(out)==len(v),'root normalization collision';return out
        return v
    def restore(v,path=(),owner=None):
        if isinstance(v,list):return [restore(x,path+(i,),owner) for i,x in enumerate(v)]
        if not isinstance(v,dict):
            if isinstance(v,str) and path and str(path[-1]).endswith('/libc/errno.qv') and len(path)>1 and path[-2] in {'sources','contracts'}:
                assert v==migrated_text, path
                records.append({'path':path,'kind':'exact-migrated-retained-module','before_sha256':hashlib.sha256(baseline_text.encode()).hexdigest(),'after_sha256':hashlib.sha256(v.encode()).hexdigest()})
                return baseline_text
            return v
        owner=v.get('function',owner)
        result={}
        for key,value in v.items():
            child=key if key in mapping['declarations'] else owner
            if owner in mapping['declarations'] and key=='statement' and v.get('name')=='nested_call_admission':
                target=mapping['declarations'][owner]
                assert value==target['after_statement'],(path,value)
                records.append({'path':path+(key,),'kind':'exact-scoped-premise-statement','before':target['before_statement'],'after':value});value=target['before_statement']
            if key=='sha256' and len(path)>1 and path[-2]=='locks' and str(path[-1]).endswith('/providers/quirin-provider-libc/contracts.lock'):
                assert v['owner']=='quirin-provider-libc' and value==lock_hash['after']
                records.append({'path':path+(key,),'kind':'exact-two-flexible-entry-lock-file-sha256','before':lock_hash['before'],'after':value})
                value=lock_hash['before']
            if key=='inventory_sha256' and path and path[-1]=='compile_configuration':
                # current.rs and no_build.rs hash the complete configuration
                # before inserting this field. Verify both raw records; the
                # final full-field comparison still rejects any other change.
                before_config=data['before']
                after_config=data['after']
                for component in path:
                    before_config=before_config[component]
                    after_config=after_config[component]
                for label,config in [('before',before_config),('after',after_config)]:
                    body={k:x for k,x in config.items() if k!='inventory_sha256'}
                    exact=hashlib.sha256(json.dumps(body,separators=(',',':'),ensure_ascii=False,sort_keys=True).encode()).hexdigest()
                    assert config['inventory_sha256']==exact,(label,path,exact)
                if value!=before_config['inventory_sha256']:
                    records.append({'path':path+(key,),'kind':'reconstructed-complete-compile-configuration-sha256','before':before_config['inventory_sha256'],'after':value,'all_remaining_configuration_fields_compared':True})
                value=before_config['inventory_sha256']
            if key=='resource_contract_sources':
                assert value==source_hash['after'],(path,value,source_hash)
                records.append({'path':path+(key,),'kind':'reconstructed-source-assumption-map-sha256','before':source_hash['before'],'after':value});value=source_hash['before']
            if key=='home_environment_sha256':
                assert value==environments['after']['sha256'],(path,value,environments['after']['sha256'])
                records.append({'path':path+(key,),'kind':'reconstructed-serialized-resource-environment-sha256','before':environments['before']['sha256'],'after':value});value=environments['before']['sha256']
            if owner in mapping['declarations'] and key=='contract_sha256':
                assert value==locks['after'][owner]
                records.append({'path':path+(key,),'kind':'official-two-function-flexible-identity','before':locks['before'][owner],'after':value});value=locks['before'][owner]
            result[key]=restore(value,path+(key,),child)
        return result
    if environments:
        baseline_text=(Path(roots[0])/mapping['module']).read_text()
        migrated_text=(ROOT/mapping['module']).read_text()
        before=normalize(environments['before']['value'])
        after=normalize(environments['after']['value'])
        selected=[key for key,value in after['source_assumptions'].items() if value==migrated_text]
        assert len(selected)==1, selected
        after['source_assumptions'][selected[0]]=baseline_text
        assert before==after, 'serialized contract environments differ beyond the exact migrated module and analyzer roots'
        (provenance/'environment-attribution.json').write_text(json.dumps({'before_sha256':environments['before']['sha256'],'after_sha256':environments['after']['sha256'],'source_assumption_sha256':source_hash,'only_changed_module':mapping['module'],'module_key':selected[0],'all_other_environment_fields_equal_after_collision_checked_root_normalization':True},indent=2)+'\n')
    names={'diagnostics':'resource-diagnostics.json','contracts':'resource-diagnostics.contracts.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','source':'monkey-source.json','null':a.route+'-null.jsonl','null-summary':a.route+'-null.summary.json','outer':a.route+'-outer.json'}
    data={}
    for label,directory in [('before',a.before),('after',a.after)]:
        inv=json.loads((directory/'invocation.json').read_text());assert inv['exit']==0;assert inv['binary_sha256']==inv['binary_sha256_after'];dest=a.out/label;dest.mkdir();(dest/'invocation.json').write_text(json.dumps(inv,indent=2)+'\n');docs={};manifest={}
        for stem,name in names.items():
            path=directory/name
            if not path.exists():assert stem in {'source','chunk-contracts','source-effects'};continue
            raw=path.read_bytes();(dest/(name+'.gz')).write_bytes(gzip.compress(raw,mtime=0));manifest[name]={'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()};docs[stem]=[json.loads(x) for x in raw.splitlines()] if name.endswith('.jsonl') else json.loads(raw)
        (dest/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');data[label]=docs
    assert data['before'].keys()==data['after'].keys()
    migrated_graphs={}
    for name,expected_atoms in [('__errno_location',3),('strerror',5)]:
        declarations=data['after']['contracts']['contract_dependencies']['declarations']
        if name not in declarations:
            continue  # Explicitly no consultation in this selected route.
        before_graph=data['before']['contracts']['contract_dependencies']['declarations'][name]['hypothesis_graph']
        after_graph=declarations[name]['hypothesis_graph']
        assert len(before_graph['atoms'])==1 and len(before_graph['premises'])==1
        assert len(after_graph['atoms'])==expected_atoms
        assert list(after_graph['premises'])==['nested_call_admission']
        assert after_graph['premises']['nested_call_admission']['children']==sorted(mapping['declarations'][name]['atoms'])
        migrated_graphs[name]={'before_atoms':1,'after_atoms':expected_atoms,'consulting_rows':declarations[name]['rows'],'consulting_safe_rows':declarations[name]['safe_rows']}
    assert '__errno_location' in migrated_graphs

    raw={k:old.differences(data['before'][k],data['after'][k]) for k in data['before']};gz(a.out/'raw-differences.json.gz',raw)
    left=normalize(old.erase(data['before']));right=normalize(restore(old.erase(data['after'])))
    residual={k:old.differences(left[k],right[k]) for k in left};gz(a.out/'attributed-residual-differences.json.gz',residual);gz(a.out/'exact-metadata-attribution.json.gz',records)
    counts=lambda d:dict(Counter(f['status'] for u in d['diagnostics']['units'] for f in u['functions']))
    checks={'static_verdict_counts_equal':counts(left)==counts(right),'full_runtime_projection_equal':left['outer']==right['outer'],'all_null_records_equal':left['null']==right['null'],'null_summary_equal':left['null-summary']==right['null-summary'],'all_remaining_artifacts_equal':all(not x for x in residual.values())}
    result={'route':a.route,'migrated_graphs':migrated_graphs,'before_counts':counts(left),'after_counts':counts(right),'null_rows':len(left['null']),'checks':checks,'raw_differences':{k:len(x) for k,x in raw.items()},'residual_differences':{k:len(x) for k,x in residual.items()},'scoped_metadata_records':len(records),'all_invariant_checks_pass':all(checks.values()),'meaning':'Raw before/after legacy erasure differs. Exact selected premise/module/pin metadata, independently reconstructed source/environment and complete configuration digests are restored; every remaining field is compared.'}
    (a.out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));return 0 if result['all_invariant_checks_pass'] else 1

if __name__=='__main__':raise SystemExit(main())
