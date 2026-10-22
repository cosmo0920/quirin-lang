#!/usr/bin/env python3
"""Collect correction receipts; preserve the earlier focused evidence unchanged."""
from pathlib import Path
import hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[3]
CARD=ROOT/'docs/reports/scheduler-lifetime-contracts'
OUT=ROOT/'private/scheduler-lifetime-contracts/tls-role-fix-1'
PATCH='1b315995a835ac7f1829bcac95fc8dd1ed80060c'
COMMANDS={
'rust':'cargo test -p qcore-contract --test scheduler_lifetime --no-fail-fast',
'source':'cargo run -p qcore-contract --example scheduler_lifetime_source -- private/scheduler-lifetime-contracts/source',
'fmt':'cargo fmt --all -- --check','clippy':'cargo clippy -p qcore-contract --all-targets -- -D warnings',
'lean-build':'lake build QCore.SchedulerLifetime','lean-audit':'lake env lean AuditSchedulerLifetime.lean'}
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
before=json.loads((OUT/'inputs-before.json').read_text());after=json.loads((OUT/'inputs-after.json').read_text())
assert before==after and int((OUT/'input-guard.exit').read_text())==0
for path,expected in before.items():
    assert hashlib.sha256(subprocess.check_output(['git','show',PATCH+':'+path],cwd=ROOT)).hexdigest()==expected,path
    assert digest(ROOT/path)==expected,path
rows=[]
for stage,command in COMMANDS.items():
    log=OUT/(stage+'.log');status=int((OUT/(stage+'.exit')).read_text());assert status==0,(stage,status)
    rows.append({'command':command,'cwd':str(ROOT/'proof' if stage.startswith('lean-') else ROOT),
        'exit':status,'private_log':str(log),'sha256':digest(log)})
assert '13 passed; 0 failed' in (OUT/'rust.log').read_text()
lean=(OUT/'lean-audit.log').read_text();assert lean.count("'QCore.SchedulerLifetime.")==13
assert 'sorryAx' not in lean and 'warning' not in lean
source=json.loads(next(line for line in (OUT/'source.log').read_text().splitlines() if line.startswith('{')))
assert source['context_refusals']==9 and source['source_refusals']==15 and source['positive_model_controls']==1
payload={'correction':'Current-thread TLS must carry a live unselected modeled parent role; callee/params tokens refuse even in empty cleanup.',
'model_source':PATCH,'measured_input_hashes':before,'input_guard_exit':0,
'inputs_before':{'path':str(OUT/'inputs-before.json'),'sha256':digest(OUT/'inputs-before.json')},
'inputs_after':{'path':str(OUT/'inputs-after.json'),'sha256':digest(OUT/'inputs-after.json')},
'producer':{'path':'docs/reports/scheduler-lifetime-contracts/tls-role-fix-driver.sh','sha256':digest(CARD/'tls-role-fix-driver.sh')},
'rust_passed':13,'lean_audited_theorems':13,'source_result':source,
'parallel_lock':'/tmp/qcore-c-card-validation-20261022.lock','rows':rows,
'prior_receipts':'focused-receipts.json remains unchanged; old source-role admission hole is disclosed in README.md',
'boundary':'finite model/source artifact binding only; actual C TLS/callback/alias/list/deployment truth Open; root final frozen standing gate still required'}
(CARD/'tls-role-fix-receipts.json').write_text(json.dumps(payload,indent=2)+'\n')
