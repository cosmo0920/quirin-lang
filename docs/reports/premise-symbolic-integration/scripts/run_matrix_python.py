import pathlib,subprocess,json,time,hashlib,fcntl,sys
p=pathlib.Path('private/premise-symbolic-integration')/sys.argv[1];p.mkdir()
with open('/tmp/qcore-c-card-validation-20261021.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert not subprocess.check_output(['git','status','--porcelain'],text=True)
 matrix=pathlib.Path('docs/reviews/r0_3/acceptance_matrix.json');before=hashlib.sha256(matrix.read_bytes()).hexdigest();start=time.time()
 with (p/'unittest.log').open('w') as out:r=subprocess.run(['python3','-m','unittest','discover','-s','scripts/tests'],stdout=out,stderr=subprocess.STDOUT)
 receipt={'command':['python3','-m','unittest','discover','-s','scripts/tests'],'head':head,'exit':r.returncode,'elapsed_seconds':time.time()-start,'matrix_sha256_before':before,'matrix_sha256_after':hashlib.sha256(matrix.read_bytes()).hexdigest(),'private_log':str(p/'unittest.log')};(p/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));print((p/'unittest.log').read_text()[-600:]);sys.exit(r.returncode)
