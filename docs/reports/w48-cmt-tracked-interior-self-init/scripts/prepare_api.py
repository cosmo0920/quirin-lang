from pathlib import Path
import subprocess
p=Path('private/w48-cmt-tracked-interior')
for route in ['corpus','plan','system','slice']:
 assert (p/('tip-'+route+'.exit')).read_text().strip()=='0'
commands=[('api-sync-1',['python3','providers/contracts.py','sync']),('api-rust-1',['cargo','test','-p','qcore-contract','--test','cmetrics_api_models','--','--nocapture']),('api-corpus-1',['cargo','test','-p','qcore-compiler','--test','qcore_contract_corpus']),('api-lean-1',['python3',str(p/'focus_api_lean.py')])]
codes=[]
for name,cmd in commands:
 codes.append(subprocess.run(['python3',str(p/'run_step.py'),name,*cmd]).returncode)
raise SystemExit(int(any(codes)))
