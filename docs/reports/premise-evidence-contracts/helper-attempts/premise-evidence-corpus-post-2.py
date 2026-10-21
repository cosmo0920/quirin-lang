import json,time,subprocess,sys,hashlib
from pathlib import Path
root=Path("/home/cosmo/.codex/worktrees/premise-decomposition-cycle1/qcore-c")
p=root/"private/premise-evidence-contracts"
out=p/"post-2";out.mkdir(exist_ok=False)
while True:
 paths=[p/"measure-1/corpus"/side/"invocation.json" for side in ["base","tip"]]
 if all(q.exists() and "exit" in json.loads(q.read_text()) for q in paths):break
 time.sleep(30)
command=[sys.executable,"/tmp/premise-evidence-final-compare-2.py","corpus"]
with (out/"corpus.log").open("wb") as log:code=subprocess.run(command,cwd=root,stdout=log,stderr=subprocess.STDOUT).returncode
receipt={"command":command,"exit":code,"log":str(out/"corpus.log"),"helper_sha256":hashlib.sha256(Path(command[1]).read_bytes()).hexdigest()}
(out/"commands.json").write_text(json.dumps([receipt],indent=2)+"\n")
print(json.dumps(receipt),flush=True)
sys.exit(code)
