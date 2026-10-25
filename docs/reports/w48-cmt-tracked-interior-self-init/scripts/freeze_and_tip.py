from pathlib import Path
import subprocess,json,datetime,os,hashlib
p=Path('private/w48-cmt-tracked-interior');head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
g=json.load(open(p/'latest-gate.json'));assert g['exit']==0 and g['head']==head, 'green root gate required';v=json.load(open(p/'provider-gate/gate.json'));assert v['exit']==0 and v['head']==head, 'green provider gate required'
freeze={'head':head,'base':json.load(open(p/'config.json'))['base'],'prediction_commit':'f18d9606562627719ac41ce3986abf9c87791c12','contract_scope_prediction_commit':'08da23123aec69859150d9e94501509ffaa57bcb','created':datetime.datetime.now(datetime.timezone.utc).isoformat(),'root_gate':g,'provider_gate':str(p/'provider-gate/gate.json'),'canonical_qv_sha256':hashlib.sha256(Path('providers/quirin-provider-cfl/contracts/cfl/intrusive_list.qv').read_bytes()).hexdigest(),'native_inputs':'identical frozen BASE snapshot; canonical declared-model change independently validated, no interpreted view change'}
f=p/'semantic-freeze.json';assert not f.exists();f.write_text(json.dumps(freeze,indent=2)+'\n')
for name,cmd in [('build-tip-1',['env','CARGO_BUILD_JOBS=6','QCORE_AUDIT_BUILD_COMMIT='+head,'cargo','build','--release','-p','qcore-audit','--bin','quirin-audit']),('tip-campaign-1',['python3',str(p/'tip_campaign.py')])]:
 code=subprocess.run(['python3',str(p/'run_step.py'),name,*cmd]).returncode
 if code:raise SystemExit(code)
