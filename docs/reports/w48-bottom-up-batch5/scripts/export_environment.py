#!/usr/bin/env python3
"""Reconstruct an exact held contract root; never relabel postchecks as prechecks."""
import argparse,datetime,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--invocation',type=Path,required=True)
p.add_argument('--helper',type=Path,required=True)
p.add_argument('--side',choices=['base','tip'],required=True)
p.add_argument('--out',type=Path,required=True)
p.add_argument('--archive-postcheck',type=Path)
a=p.parse_args(); sha=lambda b:hashlib.sha256(b).hexdigest();raw=a.invocation.read_bytes();inv=json.loads(raw)
assert inv['exit']==0 and 'identity_error' not in inv
snapshot=Path(inv['inputs_snapshot']);binary=Path(inv['binary']);assert sha(binary.read_bytes())==inv['binary_sha256']==inv['binary_sha256_after']
for key in ['source_identity','runner_sha256','profile_sha256','link_map_sha256','report_driver_sha256']:
 assert inv[key]==inv[key+'_after'],key
if 'inputs_before' in inv:
 assert inv['inputs_before']==inv['inputs_after'];expected=inv['inputs_before'];scope='actual native before/after full archive manifests'
 reference={'invocation':str(a.invocation.resolve()),'invocation_sha256':sha(raw)}
else:
 assert a.archive_postcheck,'initial configured routes require independently retained archive postcheck'
 postraw=a.archive_postcheck.read_bytes();post=json.loads(postraw)
 assert post['all_exact_git_archive_bytes_equal'] and post['archive_files']==post['actual_files']
 expected=post['archive_files'];scope=post['timing']
 reference={'archive_postcheck':str(a.archive_postcheck.resolve()),'sha256':sha(postraw),'reference':post['reference'],'captured_at':post['captured_at']}
assert a.out.is_absolute() and '/private/' in str(a.out);a.out.mkdir(parents=True,exist_ok=False)
def held_archive():
 for rel,digest in expected.items():assert sha((snapshot/rel).read_bytes())==digest,rel
 assert sha(binary.read_bytes())==inv['binary_sha256']
held_archive()
source=inv.get('source_root')
if source is None:
 command=inv['command'];assert '--checkout' in command;source=command[command.index('--checkout')+1]
sidecar_candidates=[a.invocation.parent/'resource-diagnostics.contracts.json',a.invocation.parent/'corpus/resource-diagnostics.contracts.json'];sidecars=[x for x in sidecar_candidates if x.exists()];assert len(sidecars)==1
sidecar_raw=sidecars[0].read_bytes();sidecar=json.loads(sidecar_raw)
manifest=snapshot/'contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json';output=a.out/(a.side+'-environment.json');command=[str(a.helper.resolve()),str(manifest),source,str(output)]
record={'command':command,'helper_sha256':sha(a.helper.read_bytes()),'bound_invocation_sha256':sha(raw),'archive_evidence_scope':scope,'archive_evidence_reference':reference,'archive_files_rechecked_at_export':len(expected),'manifest_sha256':sha(manifest.read_bytes()),'native_sidecar_sha256':sha(sidecar_raw),'source_root_derivation':'actual invocation.source_root or explicit --checkout argument','started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
with (a.out/(a.side+'-environment-hashes.json')).open('wb') as stdout,(a.out/(a.side+'-stderr.log')).open('wb') as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr)
record.update(exit=r.returncode,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());(a.out/'command.json').write_text(json.dumps(record,indent=2)+'\n');assert r.returncode==0
exported=output.read_bytes();environment=json.loads(exported);hashes=json.loads((a.out/(a.side+'-environment-hashes.json')).read_bytes());assert sha(exported)==hashes['environment_sha256']
assert environment['source_assumptions']==sidecar.get('sources',sidecar.get('contracts'))
held_archive();assert a.invocation.read_bytes()==raw and sidecars[0].read_bytes()==sidecar_raw
record.update(environment_sha256=sha(exported),complete_source_map_matches_actual_native_sidecar=True,held_archive_rechecked_before_after_export=True,scope='Native contract-root reconstruction and exact sidecar match; not retroactive native input attestation or proof of source premise truth.')
(a.out/'command.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2))
