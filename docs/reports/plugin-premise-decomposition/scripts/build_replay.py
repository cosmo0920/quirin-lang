#!/usr/bin/env python3
"""Compile report-owned replay against the exact libraries of an actual binary.

Cargo's fingerprint dependency receipts select each rlib; newest-file guesses
and an implicit unique-library assumption are refused. This is build identity,
not source correspondence, formal validity or premise authority.
"""
import argparse,hashlib,json,subprocess
from pathlib import Path


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path);p.add_argument('--binary',type=Path,required=True);p.add_argument('--extern',dest='crates',nargs='+',required=True);a=p.parse_args();binary=a.binary.resolve();directory=binary.parent;fingerprints=directory/'.fingerprint';deps=directory/'deps';digest=sha(binary)
 matches=[]
 for fingerprint in fingerprints.glob('qcore-audit-*/bin-quirin-audit.json'):
  suffix=fingerprint.parent.name.removeprefix('qcore-audit-');artifact=deps/('quirin_audit-'+suffix)
  if artifact.is_file() and sha(artifact)==digest:matches.append(fingerprint)
 assert len(matches)==1,('actual analyzer has no unique held Cargo receipt',matches)
 fingerprint=matches[0];metadata=json.loads(fingerprint.read_text());dependencies={row[1]:row[3] for row in metadata['deps']};libraries={};command=['rustc','--edition=2024',str(a.source.resolve()),'-L','dependency='+str(deps)]
 for name in a.crates:
  expected=dependencies[name];found=[]
  for candidate in fingerprints.glob('*/lib-'+name):
   if int.from_bytes(bytes.fromhex(candidate.read_text().strip()),'little')==expected:found.append(candidate)
  assert len(found)==1,(name,expected,found)
  receipt=found[0];suffix=receipt.parent.name.rsplit('-',1)[1];library=deps/('lib'+name+'-'+suffix+'.rlib');assert library.is_file();libraries[name]={'path':str(library),'sha256':sha(library),'cargo_dependency_fingerprint':expected,'receipt':str(receipt),'receipt_sha256':sha(receipt)};command+=['--extern',name+'='+str(library)]
 a.output.parent.mkdir(parents=True,exist_ok=True);assert not a.output.exists();command+=['-o',str(a.output.resolve())]
 log=a.output.with_suffix('.build.log')
 with log.open('wb') as stream:code=subprocess.run(command,stdout=stream,stderr=subprocess.STDOUT).returncode
 record={'command':command,'exit':code,'source_sha256':sha(a.source),'actual_analyzer_sha256':digest,'cargo_receipt':str(fingerprint),'cargo_receipt_sha256':sha(fingerprint),'libraries':libraries,'log':str(log),'meaning':'Exact actual-binary dependency identity only; no truth or source binding'}
 a.output.with_suffix('.build.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'exit':code,'libraries':len(libraries)}));return code


if __name__=='__main__':raise SystemExit(main())
