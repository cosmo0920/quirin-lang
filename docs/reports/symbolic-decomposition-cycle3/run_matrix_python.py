#!/usr/bin/env python3
"""Run authoritative Python against an already committed, unchanged matrix."""
import datetime, fcntl, hashlib, json, re, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
MATRIX=ROOT/'docs/reviews/r0_3/acceptance_matrix.json'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT).decode().strip()
def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
out=ROOT/'private/symbolic-decomposition-cycle3/matrix-python-1'; out.mkdir(parents=True,exist_ok=False)
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 head=git('rev-parse','HEAD'); before=sha(MATRIX)
 assert not git('status','--porcelain')
 assert subprocess.check_output(['git','show','HEAD:docs/reviews/r0_3/acceptance_matrix.json'],cwd=ROOT)==MATRIX.read_bytes()
 command=[sys.executable,'-m','unittest','discover','-s','scripts/tests']; start=now()
 with (out/'python.log').open('wb') as log: result=subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT)
 text=(out/'python.log').read_text(); count=re.search(r'Ran (\d+) tests?',text)
 stable=head==git('rev-parse','HEAD') and not git('status','--porcelain') and before==sha(MATRIX)
 receipt=dict(command=command,cwd=str(ROOT),source=head,started=start,ended=now(),exit=result.returncode,test_count=int(count[1]) if count else None,matrix_sha256_before=before,matrix_sha256_after=sha(MATRIX),matrix_committed_before=True,source_and_matrix_stable=stable,log=str(out/'python.log'),log_sha256=sha(out/'python.log'))
 (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n'); print(json.dumps(receipt,indent=2),flush=True)
 assert result.returncode==0 and count and stable
