#!/usr/bin/env python3
"""Verify all completed BASE receipts without hiding the earlier disk incident."""
import collections,datetime,hashlib,json
from pathlib import Path
root=Path.cwd();p=root/'private/w48-batch5';sha=lambda b:hashlib.sha256(b).hexdigest();post=json.loads((p/'full-input-postcheck.json').read_bytes());records={}
for route,directory in [('system',p/'system-before'),('slice',p/'slice-before'),('corpus',p/'corpus-before-retry-1')]:
 raw=(directory/'invocation.json').read_bytes();inv=json.loads(raw);assert inv['exit']==0 and 'identity_error'not in inv
 for key in ['source_identity','runner_sha256','profile_sha256','link_map_sha256','report_driver_sha256','binary_sha256']+([] if route=='corpus' else ['compile_database_sha256','selection_sha256']):assert inv[key]==inv[key+'_after'],(route,key)
 assert sha(Path(inv['binary']).read_bytes())==inv['binary_sha256']
 artifacts=directory/'corpus' if route=='corpus' else directory;d=json.loads((artifacts/'resource-diagnostics.json').read_bytes());rows=[r for u in d['units'] for r in u['functions']];null=[json.loads(x) for x in (directory/(route+'-null.jsonl')).read_bytes().splitlines() if x]
 rec={'invocation':str(directory/'invocation.json'),'invocation_sha256':sha(raw),'exit':inv['exit'],'started':inv['started'],'ended':inv['ended'],'binary_sha256':inv['binary_sha256'],'collector_head':inv['head'],'units':len(d['units']),'custody_rows':len(rows),'custody':dict(collections.Counter(r['status'] for r in rows)),'null_rows':len(null),'actual_evidence_versions':sorted({r['hypothesis_context']['evidence_version'] for r in rows if 'hypothesis_context'in r}),'private_raw_artifacts':str(artifacts),'unsafe_qualification':'provisional; not an upstream defect claim'}
 if route=='corpus':
  assert inv['inputs_before']==inv['inputs_after']==post['archive_files'];assert inv['external_inputs_before']==inv['external_inputs_after'];assert inv['package_inputs_before']==inv['package_inputs_after'];actual=json.loads((artifacts/'run.json').read_bytes());assert actual==inv['actual_driver_receipt'];assert actual['exit']==0 and actual['binary_sha256']==inv['binary_sha256']
  for key,value in inv['driver_environment_overrides'].items():assert actual['environment'][key]==value
  rec.update(full_native_archive_before_after_equal_independently_held_git_archive=True,archive_files=len(inv['inputs_before']),external_compile_databases=inv['external_inputs_before'],provider_package_files=inv['package_inputs_before'],actual_wrapper_exit=actual['exit'],actual_wrapper_environment=actual['environment'],free_bytes_before=inv['free_bytes_before'],free_bytes_after=inv['free_bytes_after'])
 else:rec['input_scope']='Original per-file before/after guards plus independently timed full archive postcheck; no retroactive full native pre-manifest claim.'
 records[route]=rec
out={'checked_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'routes':records,'excluded_attempts':{'corpus-before':{'analyzer_exit':-2,'outer_exit':254,'reason':'host ENOSPC; interrupted, no final report; excluded'},'focused-tip-1':{'exit':143,'reason':'terminated while queued; compilation never started'}},'source_binding':'Open','truth_transport':False}
(p/'base-completion.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
