#!/usr/bin/env python3
"""Account for empty Monkey lifetime-summary keys by exact __TIME__ replay.

The fallback descriptor is the unchanged loop_lifetime_source::read_unit
Rust Debug prefix for AnonymousText/Gnu17 with three empty option vectors.
No artifact is rewritten and no semantic field is excluded by this check.
"""
import argparse,datetime,gzip,hashlib,json,re,shlex,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('route',choices=['system','slice','corpus']);p.add_argument('comparison',type=Path)
p.add_argument('--residual',type=Path)
a=p.parse_args();residual_path=a.residual or a.comparison/'remaining-differences.json.gz'
d=json.loads(gzip.decompress(residual_path.read_bytes()))
invocation=json.loads((a.comparison/'tip/invocation.json').read_text())
audit_command=invocation['command']
if a.route=='corpus':
 # pinned_route.rs:98 reads the source root's production database; the
 # explicit Monkey library DB is appended later and has no mk_bin/monkey.c.
 audit_command=invocation['actual_driver_receipt']['command']
 assert audit_command[:3]==[invocation['binary'],'pinned','--jobs']
 assert Path(audit_command[4])==Path(invocation['source_root'])
 database=Path(audit_command[4])/'build/compile_commands.json'
else:
 database=Path(audit_command[audit_command.index('--compile-database')+1])
producer_sources={}
base_inv=json.loads((a.comparison/'base/invocation.json').read_bytes())
for path in ['crates/qcore-c-gateway/src/loop_lifetime_source.rs','crates/qcore-c-gateway/src/legacy_c.rs']:
 before=subprocess.check_output(['git','show',base_inv['analyzer_source']+':'+path]);after=subprocess.check_output(['git','show',invocation['analyzer_source']+':'+path]);assert before==after,('empty-summary producer changed',path);producer_sources[path]=hashlib.sha256(before).hexdigest()
raw_database=database.read_bytes()
for held_inv in [base_inv,invocation]:
 expected=held_inv['external_inputs_before'][str(database)] if a.route=='corpus' else held_inv['compile_database_sha256']
 assert hashlib.sha256(raw_database).hexdigest()==expected,'actual compile database differs from held audit identity'
entries=[entry for entry in json.loads(raw_database) if entry['file'].endswith('/lib/monkey/mk_bin/monkey.c')];assert len(entries)==1
entry=entries[0];words=shlex.split(entry['command']);command=[words[0]];skip=False
for word in words[1:]:
 if skip:skip=False;continue
 if word in ['-c','-MD','-MMD','-MP','-M','-MM','-MG']:continue
 if word in ['-o','-MF','-MT','-MQ']:skip=True;continue
 if (word.startswith('-o') and len(word)>2) or any(word.startswith(flag) and len(word)>3 for flag in ['-MF','-MT','-MQ']):continue
 command.append(word)
command+=['-E','-P']
assert '-fPIE' in command
assert command[-3].endswith('/lib/monkey/mk_bin/monkey.c')
result=subprocess.run(command,cwd=entry['directory'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
record={'command':command,'cwd':entry['directory'],'exit':result.returncode,'compile_database':str(database),'resolved_compile_database':str(database.resolve()),'bound_audit_command':audit_command,'compile_database_sha256':hashlib.sha256(raw_database).hexdigest(),'source_sha256':hashlib.sha256(Path(entry['file']).read_bytes()).hexdigest(),'fresh_execution':True,'unchanged_producer_sources':producer_sources}
(a.comparison/'time-preprocess-command.json').write_text(json.dumps(record,indent=2)+'\n');(a.comparison/'time-preprocess-stderr.log').write_bytes(result.stderr)
(a.comparison/'time-source.c.gz').write_bytes(gzip.compress(result.stdout,mtime=0))
assert result.returncode==0
changes=d['contracts'];assert all(not v for k,v in d.items() if k!='contracts')
paths=[['callback_members','lifetime_protocols','evidence','summaries'],['callback_members','prepass','lifetime_protocols','summaries']]
assert len(changes)==4 and all(c['path'][:-1] in paths for c in changes)
targets={c['path'][-1] for c in changes};assert len(targets)==2
assert all((c.get('before'),c.get('after')) in [(None,{}),({},None)] for c in changes)
s=result.stdout;m=re.search(rb'(static const char MONKEY_BUILT\[\] = ")([^"]+)(" " " ")([^"]+)(";)',s);assert m
# As read_facts_in does after compiler_options rejects -fPIE, own command -E -P,
# then C11TranslationUnit { source, dialect:Gnu17, ..Default::default() }.
prefix=b'AnonymousText\0Gnu17\0[]\0[]\0[]\0'+s[:m.start(4)];suffix=s[m.end(4):]
found={};windows=[]
for label in ['base','tip']:
 inv=json.loads((a.comparison/label/'invocation.json').read_text());begin=datetime.datetime.fromisoformat(inv['started']);end=datetime.datetime.fromisoformat(inv['ended']);windows.append([inv['started'],inv['ended']])
 for sec in range(int(begin.timestamp()),int(end.timestamp())+1):
  t=datetime.datetime.fromtimestamp(sec,datetime.timezone(datetime.timedelta(hours=9))).strftime('%H:%M:%S');h=hashlib.sha256(prefix+t.encode()+suffix).hexdigest()
  if h in targets: found[h]=t
assert set(found)==targets,(found,targets)
record={'meaning':'four retained raw leaf differences are one empty-summary identity remap, duplicated in evidence/prepass; changing only MONKEY_BUILT __TIME__ reproduces both complete source/config SHA256 keys','source':'lib/monkey/mk_bin/monkey.c','preprocessed_bytes':len(s),'preprocessed_sha256':hashlib.sha256(s).hexdigest(),'descriptor_prefix_hex':b'AnonymousText\0Gnu17\0[]\0[]\0[]\0'.hex(),'timezone':'Asia/Tokyo (+09:00)','date_literal':m.group(2).decode(),'matches':found,'measurement_windows_utc':windows,'preprocess':record,'residual_input':str(residual_path),'semantic_summary_before':{},'semantic_summary_after':{},'all_other_complete_reconstructed_fields_equal':True}
(a.comparison/'provenance-attribution.json').write_text(json.dumps(record,indent=2)+'\n');(a.comparison/'time-source.c.gz').write_bytes(gzip.compress(s,mtime=0));print(json.dumps({'matches':found,'all_residuals_attributed':True}))

comparison=json.loads((a.comparison/'comparison.json').read_bytes())
assert comparison['existing_reading_observations_exactly_reconstructed']>=0
assert comparison['full_null_equal'] and comparison['full_runtime_equal']
final=dict(comparison)
final.update({'all_existing_fields_equal_after_exact_time_identity_attribution':True,
 'all_fields_retained':True,'full_null_equal':True,'full_runtime_equal':True,
 'time_identity_proof_sha256':hashlib.sha256((a.comparison/'provenance-attribution.json').read_bytes()).hexdigest(),
 'unexplained_remaining_leaf_counts':{key:0 for key in d},
 'qualification':'Only exact independently replayed empty-summary source keys are paired; no semantic payload or capture is deleted.'})
(a.comparison/'final-comparison.json').write_text(json.dumps(final,indent=2)+'\n')
