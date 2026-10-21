#!/usr/bin/env python3
"""Acquire externally; require complete own gates then freeze tip and run six audits."""
import hashlib,json,shutil,subprocess,sys,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];P=ROOT/'private/premise-evidence-contracts'
for stage in ['standing-2','provider-1','release-1']:
 rows=json.loads((P/stage/'commands.json').read_bytes());assert rows and all(r['exit']==0 for r in rows),stage
code=subprocess.call([sys.executable,'docs/reports/premise-evidence-contracts/scripts/run_steps.py','examples',str(P/'examples-2')],cwd=ROOT)
assert code==0
assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT), 'freeze complete tracked source before snapshots'
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
archive=P/'tip-inputs.tar';snapshot=P/'tip-inputs';snapshot.mkdir(exist_ok=False)
with archive.open('wb') as stream:subprocess.run(['git','archive',head,'contract','providers','scripts/run_pinned_audit.py','cycle19-resource-contracts.json'],cwd=ROOT,stdout=stream,check=True)
with tarfile.open(archive) as stream:stream.extractall(snapshot,filter='data')
(snapshot/'target/release').mkdir(parents=True)
shutil.copy2(ROOT/'target/release/quirin-audit',snapshot/'target/release/quirin-audit')
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
receipt={'head':head,'binary_sha256':sha(snapshot/'target/release/quirin-audit'),'tip_archive_sha256':sha(archive),'baseline_receipt':json.loads((ROOT/'docs/reports/premise-evidence-contracts/baseline-and-context.json').read_bytes()),'source_binding':'open','all_gate_steps_pass':True}
(P/'immutable-tip.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt),flush=True)
env=P/'environment';env.mkdir()
source='/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
for side in ['base','tip']:
 command=[str(ROOT/'target/debug/examples/premise_environment'),str(P/(side+'-inputs')/'cycle19-resource-contracts.json'),source,str(env/(side+'-environment.json'))]
 with (env/(side+'-environment-hashes.json')).open('wb') as stream:subprocess.run(command,cwd=ROOT,stdout=stream,check=True)
 (env/(side+'-command.json')).write_text(json.dumps(command,indent=2)+'\n')
raise SystemExit(subprocess.call([sys.executable,'docs/reports/premise-evidence-contracts/scripts/measure_all.py'],cwd=ROOT))
