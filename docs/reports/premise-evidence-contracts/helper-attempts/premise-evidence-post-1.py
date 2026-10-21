import json,time,subprocess,sys
from pathlib import Path
root=Path("/home/cosmo/.codex/worktrees/premise-decomposition-cycle1/qcore-c")
private=root/"private/premise-evidence-contracts"
out=private/"post-1";out.mkdir(exist_ok=False)
receipts=[]
for route in ["system","slice","corpus"]:
 while True:
  paths=[private/"measure-1"/route/side/"invocation.json" for side in ["base","tip"]]
  if all(p.exists() and "exit" in json.loads(p.read_text()) for p in paths):break
  time.sleep(15)
 command=[sys.executable,"docs/reports/premise-evidence-contracts/scripts/final_compare.py",route]
 with (out/(route+".log")).open("wb") as log:code=subprocess.run(command,cwd=root,stdout=log,stderr=subprocess.STDOUT).returncode
 receipts.append({"command":command,"exit":code,"log":str(out/(route+".log"))})
 (out/"commands.json").write_text(json.dumps(receipts,indent=2)+"\n")
 print(json.dumps(receipts[-1]),flush=True)
 if code:sys.exit(code)
