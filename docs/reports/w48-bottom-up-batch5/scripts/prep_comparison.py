import pathlib,subprocess,json,hashlib,sys,datetime
root=pathlib.Path.cwd();p=root/'private/w48-batch5';native=pathlib.Path('/home/cosmo/.codex/worktrees/symbolic-cycle3-integration/qcore-c/private/symbolic-decomposition-cycle3/postgate-1');route=sys.argv[1];base=p/('corpus-before-retry-1' if route=='corpus' else route+'-before');tip=native/route/'tip';out=p/'comparisons'/route;out.mkdir(parents=True,exist_ok=True);logs=out/'prep-commands';logs.mkdir(exist_ok=False);records=[];sha=lambda p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def run(name,command):
 record={'name':name,'command':list(map(str,command)),'started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 with (logs/(name+'.log')).open('wb') as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
 record.update(exit=r.returncode,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());records.append(record);(out/'prep-commands.json').write_text(json.dumps(records,indent=2)+'\n');assert r.returncode==0,name
stage=p/'analysis-inputs'/route
run('stage',['python3','docs/reports/w48-bottom-up-batch5/scripts/stage_inputs.py',route,base,tip,stage])
run('raw',['python3','docs/reports/w48-bottom-up-batch5/scripts/retain_compare.py',route,base,tip,out/'raw'])
run('known-identities',['python3','docs/reports/premise-resolution-cycle2/scripts/derive_joint_identities.py',route,stage/'base',stage/'tip',p/'environments/base',p/'environments/tip',out/'known-identities.json'])
helpers=json.loads((native/'helpers.json').read_bytes());helper=pathlib.Path(helpers['helpers']['metadata-replay']['path']);assert sha(helper)==helpers['helpers']['metadata-replay']['sha256']
for side in ['base','tip']:
 run('metadata-'+side,[helper,stage/side/'resource-diagnostics.json',stage/side/'resource-diagnostics.contracts.json',p/'environments'/side/route/(side+'-environment.json'),out/('metadata-'+side+'.json')])
compat=out/'archived-compat';compat.mkdir()
for stem,name in [('diagnostics','resource-diagnostics.json'),('source','monkey-source.json'),('chunk-contracts','resource-diagnostics.chunk-contracts.json'),('source-effects','resource-diagnostics.source-effects.json'),('contracts','resource-diagnostics.contracts.json')]:
 source=out/'raw/base'/(name+'.gz')
 if source.exists():(compat/(stem+'.json.gz')).symlink_to(source)
run('five-artifact',['python3','docs/reports/corpus-repin/diff_artifacts.py','--archived',compat,'--measured',stage/'tip','--output',out/'five-artifact-diff.json','--full'])
run('source-body',['python3','docs/reports/w48-bottom-up-batch5/scripts/attribute_source.py',out/'raw','--source',json.loads((tip/'invocation.json').read_bytes())['source_root']])
print('prepared',route,len(records),'commands')
