"""Continue the explicitly authorized integration under the same acquired lock.

The root prepares and commits each reviewed source/matrix state before writing
its readiness signal. Heavy commands remain descendants of the lock owner.
"""
from pathlib import Path
import json,subprocess,time,datetime,sys
p=Path('private/w48-cmt-member-storage')
def write(name,value):(p/name).write_text(json.dumps(value,indent=2)+'\n')
def wait(name):
 signal=p/name
 while not signal.exists():time.sleep(1)
 r=json.loads(signal.read_text());head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
 assert r['head']==head and not subprocess.check_output(['git','status','--porcelain'],text=True).strip(),r
 return r
write('native-ready-for-integration.json',{'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'native_comparison_complete':True})
print('Native attribution complete; holding acquired slot for authorized integration.',flush=True)
wait('integration-ready.json')
code=subprocess.run(['python3',str(p/'integration_gate.py')]).returncode
write('integration-gate-complete.json',{'exit':code,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
if code:sys.exit(code)
wait('integration-matrix-ready.json')
code=subprocess.run(['python3',str(p/'run_step.py'),'post-matrix-python-1','python3','-m','unittest','discover','-s','scripts/tests']).returncode
write('integration-matrix-complete.json',{'exit':code,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
sys.exit(code)
