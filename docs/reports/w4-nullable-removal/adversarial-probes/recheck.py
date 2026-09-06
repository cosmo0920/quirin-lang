"""Recheck the archived source probes against an explicitly supplied gateway rlib.

Usage: python3 recheck.py /absolute/path/to/libqcore_c_gateway-HASH.rlib /tmp/probe-results
Build the current gateway first with cargo build -p qcore-c-gateway.
"""
import hashlib,json,subprocess,sys,tempfile
from pathlib import Path
root=Path(__file__).resolve().parent
rlib=Path(sys.argv[1]).resolve()
output=Path(sys.argv[2]).resolve();output.mkdir(parents=True,exist_ok=True)
manifest=json.loads((root/'recheck.json').read_text())
results={}
with tempfile.TemporaryDirectory(prefix='qcore-custody-recheck-') as temporary:
    binary=Path(temporary)/'probe'
    for name,record in manifest['cases'].items():
        source=root/record['source']
        assert hashlib.sha256(source.read_bytes()).hexdigest()==record['source_sha256']
        subprocess.run(['rustc','--edition','2024','-C','strip=debuginfo',str(source),'-L','dependency='+str(rlib.parent),'--extern','qcore_c_gateway='+str(rlib),'-o',str(binary)],check=True)
        result=subprocess.run([str(binary)],capture_output=True,text=True,check=True)
        (output/(name+'.log')).write_text(result.stdout+result.stderr)
        results[name]={'safe':result.stdout.startswith('SAFE'),'class':result.stdout.split(None,1)[0].split('(',1)[0]}
        binary.unlink()
(output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
assert all(not result['safe'] for result in results.values()),results
print(json.dumps(results,indent=2))
