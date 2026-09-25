#!/usr/bin/env python3
"""Read-only source/build/archive census; output paths must be fresh."""
import argparse,collections,gzip,hashlib,json,pathlib,shlex,subprocess,time
FILES=['plugins/in_storage_backlog/sb.c','src/flb_input_chunk.c','src/flb_storage.c','src/flb_task.c','src/flb_engine.c','src/flb_engine_dispatch.c','src/flb_scheduler.c','src/flb_output.c']
PIN='864c3d97f3589515285a19396ed8dfc85cda08ae';BUILD_PIN='66910c10a4d7eafa810b84229cf2dcf7b0f26f97'
def sha(b):return hashlib.sha256(b).hexdigest()
def git(root,*a):return subprocess.check_output(['git','-C',str(root),*a],text=True).strip()
def main():
 p=argparse.ArgumentParser();p.add_argument('--source',type=pathlib.Path,required=True);p.add_argument('--database',type=pathlib.Path,required=True);p.add_argument('--build-source',type=pathlib.Path,required=True);p.add_argument('--archive',type=pathlib.Path,required=True);p.add_argument('--out',type=pathlib.Path,required=True);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
 source=a.source.resolve();build=a.build_source.resolve();assert git(source,'rev-parse','HEAD')==PIN;assert git(build,'rev-parse','HEAD')==BUILD_PIN
 assert not git(source,'status','--porcelain','--untracked-files=no'),'tracked source checkout modified'
 assert not git(build,'status','--porcelain','--untracked-files=no'),'tracked build-source checkout modified'
 raw=a.archive.read_bytes();d=json.loads(gzip.decompress(raw));assert d['fluent_bit_revision']==BUILD_PIN
 db_bytes=a.database.read_bytes();db=json.loads(db_bytes);rows=[]
 for rel in FILES:
  s=(source/rel).read_bytes();b=(build/rel).read_bytes();matches=[(i,e) for i,e in enumerate(db) if pathlib.Path(e['file']).resolve()==build/rel]
  # The archived inventory names library plus file. These eight basenames are unique in the requested list.
  archived=[u for u in d['units'] if u['file']==pathlib.Path(rel).name and u['library']=='fluent-bit-chunk']
  row={'path':rel,'source_sha256':sha(s),'build_source_sha256':sha(b),'same_bytes_across_revisions':s==b,'build_records':[],'archived_rows':[]}
  for i,e in matches:
   argv=e.get('arguments') or shlex.split(e['command']);assert argv[0]=='/usr/bin/clang',argv[0]
   stripped=[];j=1
   while j<len(argv):
    if argv[j]=='-o':j+=2;continue
    if argv[j]=='-c':j+=1;continue
    stripped.append(argv[j]);j+=1
   for flag in ['-E','-P']:stripped.append(flag)
   cmd=[argv[0],*stripped];name=pathlib.Path(rel).stem+f'-{i}';start=time.time()
   with (a.out/(name+'.i')).open('wb') as stdout,(a.out/(name+'.stderr')).open('wb') as stderr:
    run=subprocess.run(cmd,cwd=e['directory'],stdout=stdout,stderr=stderr,timeout=120)
   (a.out/(name+'.exit')).write_text(str(run.returncode)+'\n')
   rec={'index':i,'entry':e,'entry_sha256':sha(json.dumps(e,sort_keys=True,separators=(',',':')).encode()),'preprocess_command':cmd,'exit':run.returncode,'seconds':round(time.time()-start,3),'preprocessed_sha256':sha((a.out/(name+'.i')).read_bytes()),'log':str(a.out/(name+'.stderr'))}
   row['build_records'].append(rec)
  for u in archived:
   row['archived_rows'].append({'library':u['library'],'file':u['file'],'status':u['status'],'reason_code':u['reason_code'],'functions':len(u['functions']),'function_status_counts':dict(collections.Counter(f['status'] for f in u['functions'])),'diagnostic_counts':u['diagnostic_counts'],'functions_detail':[{'function':f['function'],'status':f['status'],'blocker_categories':dict(collections.Counter(x['category'] for x in f['diagnostics']))} for f in u['functions']]})
  row['coverage']='present in historical archive' if archived else 'absent from historical archive; not an Unknown row'
  rows.append(row)
 result={'schema_version':1,'card':'FB-AUDIT-01-COVERAGE','source':str(source),'source_revision':PIN,'build_source':str(build),'build_revision':BUILD_PIN,'database':str(a.database),'database_sha256':sha(db_bytes),'database_entries':len(db),'archive':str(a.archive),'archive_gzip_sha256':sha(raw),'archive_units':len(d['units']),'scope':'eight requested system files; archived coverage and current source review are separate; preprocessing is not analyzer execution or proof','rows':rows,'summary':{'selected_files':len(rows),'files_with_compile_records':sum(bool(r['build_records']) for r in rows),'compile_records':sum(len(r['build_records']) for r in rows),'preprocess_failed':sum(x['exit']!=0 for r in rows for x in r['build_records']),'same_source_bytes':sum(r['same_bytes_across_revisions'] for r in rows),'files_in_archive':sum(bool(r['archived_rows']) for r in rows),'files_absent_from_archive':sum(not r['archived_rows'] for r in rows)}}
 (a.out/'evidence.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result['summary'],indent=2))
 if result['summary']['preprocess_failed']:raise SystemExit(1)
if __name__=='__main__':main()
