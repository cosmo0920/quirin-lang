#!/usr/bin/env python3
"""Run explicit-input two-envelope reference checks; no source truth is inferred.

Preparation uses draft model/audit bytes explicitly; final invocation supplies
registered production files and a freshly built card helper. Every run is fresh.
"""
from pathlib import Path
import argparse, copy, datetime, hashlib, json, subprocess

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]

def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def canonical(value): return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--audit', type=Path, required=True)
    args = parser.parse_args(); out = args.out.resolve(); out.mkdir(parents=True, exist_ok=False)
    design = json.loads((REPORT/'formal-catalog-design.json').read_text())
    freeze = json.loads((REPORT/'outer-identity-freeze.json').read_text())
    model = args.model.resolve(); audit = args.audit.resolve()
    model_key = str(model.relative_to(REPO)); audit_key = str(audit.relative_to(REPO))
    ref = lambda key, path: {'path': key, 'sha256': sha(path)}
    env = design['environment']; sources = {}; artifacts = {model_key:str(model),audit_key:str(audit)}
    for envelope in freeze['envelopes']:
        owner, relative = envelope['module'].split('/contracts/')
        key = owner.split('/')[-1]+':'+relative
        sources[key] = str(REPO/envelope['module'])
    candidate = REPO/design['candidate_inventory']['path']; artifacts[str(candidate.relative_to(REPO))] = str(candidate)
    source_root = Path(freeze['source_routes']['corpus']['root']); loader_refs=[]
    for relative, expected in sorted(freeze['source_routes']['corpus']['files'].items()):
        path = source_root/relative; assert sha(path)==expected
        key = 'upstream/66910c10/'+relative; artifacts[key]=str(path); loader_refs.append(ref(key,path))
    catalog = {'version':1,'source_binding':'open','environment':env,
        'source_modules':{key:sha(path) for key,path in sources.items()},
        'candidate_inventory':ref(str(candidate.relative_to(REPO)),candidate),
        'loader_sources':loader_refs,'envelopes':{}}
    for item in design['outer_catalogs']:
        held = next(x for x in freeze['envelopes'] if x['name']==item['name'])
        atoms={}
        for atom in item['atoms']:
            atoms[atom['id']]={key:copy.deepcopy(atom[key]) for key in ['proposition','quantifiers','prerequisites','conclusion','mapping','source_binding']}
            atoms[atom['id']].update(environment=env['id'],model=ref(model_key,model),audit=ref(audit_key,audit))
        catalog['envelopes'][item['name']]={'definition':held['defining_declaration'],
            'hosting_loaders':sorted(held['hosting_loaders']),
            'outer_sha256':held['outer_digest_from_original_official_loader_text_hashes'],'atoms':atoms}
    expected=out/'held-expected.json'; expected.write_bytes(canonical(catalog))
    (out/'source-index.json').write_text(json.dumps(sources,indent=2)+'\n')
    (out/'artifact-index.json').write_text(json.dumps(artifacts,indent=2)+'\n')
    records=[]
    def run(name, value, wanted=1, held=None, source_override=None, artifact_override=None):
        directory=out/name; directory.mkdir()
        presented=directory/'presented.json'; presented.write_bytes(canonical(value))
        hold=expected
        if held is not None:hold=directory/'held.json';hold.write_bytes(canonical(held))
        si=out/'source-index.json'; ai=out/'artifact-index.json'
        if source_override is not None:si=directory/'sources.json';si.write_text(json.dumps(source_override))
        if artifact_override is not None:ai=directory/'artifacts.json';ai.write_text(json.dumps(artifact_override))
        result_file=directory/'result.json';cmd=[str(args.binary.resolve()),str(hold),str(presented),str(si),str(ai),str(result_file)]
        start=datetime.datetime.now(datetime.timezone.utc).isoformat()
        with (directory/'console.log').open('w') as stream:result=subprocess.run(cmd,cwd=REPO,stdout=stream,stderr=subprocess.STDOUT)
        record={'name':name,'command':cmd,'exit':result.returncode,'expected_exit':wanted,'started':start,
            'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(directory/'console.log')}
        records.append(record);(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
        assert result.returncode==wanted,(name,result.returncode)
        assert result_file.exists()==(wanted==0)
        if wanted==0:
            result=json.loads(result_file.read_text());assert result['source_binding']=='open'
            assert not {'disposition','verdict','authority'} & result.keys()
    run('actual-two-envelopes',catalog,0)
    changed=copy.deepcopy(catalog);changed['source_binding']='supported';run('unsupported-source-truth-even-if-expected-changed',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['version']=999;run('unsupported-version-even-if-expected-changed',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['rk_plugin_library']['definition']='flb_plugin_load';run('wrong-rk-definition',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['flb_external_plugin']['hosting_loaders']=['flb_plugin_load'];run('missing-go-host',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['rk_plugin_library']['outer_sha256']='0'*64;run('wrong-outer-digest',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['environment']['id']='foreign';run('foreign-manifest-environment',changed)
    first_atom=next(iter(catalog['envelopes']['rk_plugin_library']['atoms']))
    changed=copy.deepcopy(catalog);changed['envelopes']['rk_plugin_library']['atoms'][first_atom]['prerequisites']=[];run('missing-prerequisite',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['rk_plugin_library']['atoms'][first_atom]['model']['sha256']='0'*64;run('wrong-model-hash',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['rk_plugin_library']['atoms'][first_atom]['audit']['path']='../foreign';run('unsafe-audit-reference',changed,held=changed)
    changed=copy.deepcopy(catalog);changed['envelopes']['flb_external_plugin']['atoms']={};run('empty-flb-atoms',changed,held=changed)
    missing=dict(artifacts);missing.pop(model_key);run('missing-model-input',catalog,artifact_override=missing)
    missing=dict(artifacts);missing.pop(str(candidate.relative_to(REPO)));run('missing-candidate-input',catalog,artifact_override=missing)
    missing=dict(sources);missing.pop(next(iter(missing)));run('missing-outer-source',catalog,source_override=missing)
    altered=out/'altered-candidate.json';altered.write_bytes(candidate.read_bytes()+b'\n');different=dict(artifacts);different[str(candidate.relative_to(REPO))]=str(altered);run('altered-candidate-bytes',catalog,artifact_override=different)
    run('actual-two-envelopes-after-controls',catalog,0)
    # Actual creator source contains an ignored return; reference integrity must
    # still yield Open, and a forged source-supported catalog is rejected above.
    proxy=(source_root/'src/flb_plugin_proxy.c').read_text();assert '    flb_plugin_proxy_register(proxy, config);' in proxy
    assert '    return proxy;' in proxy
    summary={'schema':1,'positive_controls':2,'negative_controls':len(records)-2,
        'all_expected_outcomes':True,'binary_sha256':sha(args.binary),
        'model_sha256':sha(model),'audit_sha256':sha(audit),'held_catalog_sha256':sha(expected),
        'outer_source_digests':catalog['source_modules'],
        'ignored-registration-return':{'source':'src/flb_plugin_proxy.c','sha256':sha(source_root/'src/flb_plugin_proxy.c'),
            'reference_check_disposition':'open','meaning':'Absence of a checked return proves neither successful registration nor failed old envelope. No Supported source evidence or upstream Unsafe is minted.'},
        'meaning':'Reference integrity checks only; source/model/prose/environment binding and all component truths remain Open. Final production model/audit + independently held Graph/Context checks are separate.'}
    (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary))

if __name__=='__main__':main()
