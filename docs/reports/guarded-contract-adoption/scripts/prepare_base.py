#!/usr/bin/env python3
"""Prepare exact BASE source archive without compiling or taking native locks."""
import datetime,hashlib,json,subprocess,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
SOURCE='31c5ed500d9d4a1448a623b839e1644adb19efd1'
out=ROOT/'private/guarded-contract-adoption/native-base-build-1';out.mkdir(parents=True,exist_ok=False)
archive=out/'source.tar'
with archive.open('wb') as f:r=subprocess.run(['git','archive',SOURCE],cwd=ROOT,stdout=f)
assert r.returncode==0
checkout=out/'source';checkout.mkdir()
with tarfile.open(archive) as f:f.extractall(checkout,filter='data')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
files={str(p.relative_to(checkout)):sha(p) for p in sorted(checkout.rglob('*')) if p.is_file()}
record={'source':SOURCE,'archive_command':['git','archive',SOURCE],'archive_exit':r.returncode,'archive_sha256':sha(archive),'archive':str(archive),'checkout':str(checkout),'files':files,'prepared':datetime.datetime.now(datetime.timezone.utc).isoformat(),'compilation':'pending under shared validation lock; no build or native result claimed'}
(out/'source-receipt.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'source':SOURCE,'checkout':str(checkout),'archive_files':len(files),'archive_sha256':sha(archive)}))
