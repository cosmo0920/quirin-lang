from pathlib import Path
import datetime,json,os,subprocess,sys
p=Path('private/w48-cmt-member-storage')
freeze=json.loads((p/'semantic-freeze.json').read_text())['head']
r={'acquired_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'state':'development_slot_acquired','head':freeze,'prediction_commit':'08b5546809e99124f384c0d91db7220c9c192520','pid':os.getpid()}
(p/'development-lock-recovery.json').write_text(json.dumps(r,indent=2)+'\n');(p/'development-lock.json').write_text(json.dumps(r,indent=2)+'\n')
commands=[['python3',str(p/'run_standing_gate.py')],['python3',str(p/'run_step.py'),'build-tip-1','env','CARGO_BUILD_JOBS=6','QCORE_AUDIT_BUILD_COMMIT='+freeze,'cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'],['python3',str(p/'run_step.py'),'tip-campaign-1','python3',str(p/'tip_campaign.py')]]
for route in ['corpus','system','slice']:
 commands.append(['python3',str(p/'run_step.py'),'retain-'+route,'python3',str(p/'retain_compare.py'),route])
commands.append(['python3',str(p/'run_step.py'),'post-attribution-1','sh','docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh','python3',str(p/'post_attribution.py'),'replay-v1'])
for route in ['corpus','system','slice']:
 commands.append(['python3',str(p/'run_step.py'),'attribute-'+route,'python3',str(p/'attribute_compare.py'),route])
commands.append(['python3',str(p/'run_step.py'),'accounting-1','python3',str(p/'finish_accounting.py')])
for command in commands:
 print('START',command,flush=True)
 code=subprocess.run(command).returncode
 if code:sys.exit(code)
r.update(state='released',released_at=datetime.datetime.now(datetime.timezone.utc).isoformat());(p/'development-lock-recovery.json').write_text(json.dumps(r,indent=2)+'\n')
