"""Compare every native row and raw JSON leaf without masking digests or paths."""
from pathlib import Path
import argparse,collections,gzip,hashlib,json,subprocess,sys

def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,obj):
 with gzip.open(p,'wt') if p.suffix=='.gz' else p.open('w') as f: json.dump(obj,f,sort_keys=True,indent=1);f.write('\n')
def diff(b,t,path=()):
 if type(b)!=type(t):yield {'path':path,'base':b,'tip':t}
 elif isinstance(b,dict):
  for k in sorted(set(b)|set(t)):
   if k not in b or k not in t:yield {'path':(*path,k),'base_present':k in b,'tip_present':k in t,'base':b.get(k),'tip':t.get(k)}
   else:yield from diff(b[k],t[k],(*path,k))
 elif isinstance(b,list):
  if len(b)!=len(t):yield {'path':path,'base':b,'tip':t}
  else:
   for i,(x,y) in enumerate(zip(b,t)):yield from diff(x,y,(*path,i))
 elif b!=t:yield {'path':path,'base':b,'tip':t}
def report_dir(p,route):return p/'corpus' if route=='corpus' else p
def tally(d):return dict(collections.Counter(f['status'] for u in d['units'] for f in u.get('functions',[])))
def leaf_counter(row):return collections.Counter(json.dumps(x,sort_keys=True) for x in row.get('diagnostics',[]))
def decoded(counter):return [{'diagnostic':json.loads(k),'multiplicity':v} for k,v in sorted(counter.items())]

p=argparse.ArgumentParser(description=__doc__);p.add_argument('route',choices=['corpus','system','slice']);p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);p.add_argument('--repo',type=Path,default=Path.cwd());a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
for directory in [a.base,a.tip]:
 assert json.loads((directory/'invocation.json').read_text()).get('exit')==0,(directory,'native route incomplete or failed')
bd,td=[report_dir(x,a.route) for x in [a.base,a.tip]]
b,t=[json.loads((x/'resource-diagnostics.json').read_text()) for x in [bd,td]]
assert len(b['units'])==len(t['units'])
changes=[];status=[];leaf_changes=[];unit_changes=[];unit_status_changes=[]
for ui,(bu,tu) in enumerate(zip(b['units'],t['units'])):
 assert (bu['library'],bu['file'])==(tu['library'],tu['file'])
 fields=['status','reason','reason_code','source_functions','not_projected','diagnostic_counts']
 old_unit={k:bu.get(k) for k in fields};new_unit={k:tu.get(k) for k in fields}
 if old_unit!=new_unit:unit_changes.append({'unit_index':ui,'library':bu['library'],'file':bu['file'],'base':old_unit,'tip':new_unit})
 if bu.get('status')!=tu.get('status'):unit_status_changes.append({'unit_index':ui,'library':bu['library'],'file':bu['file'],'base':bu.get('status'),'tip':tu.get('status'),'base_reason':bu.get('reason'),'tip_reason':tu.get('reason')})
 br={r['function']:r for r in bu.get('functions',[])};tr={r['function']:r for r in tu.get('functions',[])}
 assert len(br)==len(bu.get('functions',[])) and len(tr)==len(tu.get('functions',[])) and br.keys()==tr.keys()
 for name in br:
  old,new=br[name],tr[name];key={'unit_index':ui,'library':bu['library'],'file':bu['file'],'function':name}
  if old!=new:changes.append(key|{'base':old,'tip':new})
  if old['status']!=new['status']:status.append(key|{'base':old,'tip':new})
  bc,tc=leaf_counter(old),leaf_counter(new)
  if bc!=tc:leaf_changes.append(key|{'base_status':old['status'],'tip_status':new['status'],'removed':decoded(bc-tc),'added':decoded(tc-bc)})
write(a.out/'unit-verdict-and-projection-changes.json',unit_changes);write(a.out/'unit-status-changes.json',unit_status_changes)
write(a.out/'changed-rows.json.gz',changes);write(a.out/'verdict-changes.json',status);write(a.out/'diagnostic-leaf-trades.json',leaf_changes)
receipts=[];raw_counts={};stems={'diagnostics':'resource-diagnostics.json','source':'monkey-source.json','contracts':'resource-diagnostics.contracts.json','source-effects':'resource-diagnostics.source-effects.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json'}
archive=a.out/'base-artifacts';archive.mkdir()
for stem,name in stems.items():
 bp,tp=bd/name,td/name;record={'artifact':name,'base_present':bp.exists(),'tip_present':tp.exists()}
 for side,f in [('base',bp),('tip',tp)]:
  if f.exists():record[side]={'path':str(f.resolve()),'bytes':f.stat().st_size,'sha256':digest(f)}
 receipts.append(record)
 if bp.exists():
  with gzip.open(archive/(stem+'.json.gz'),'wb') as out:out.write(bp.read_bytes())
 if bp.exists() and tp.exists():
  differences=list(diff(json.loads(bp.read_bytes()),json.loads(tp.read_bytes())))
  write(a.out/(stem+'-raw-leaf-differences.json.gz'),differences);raw_counts[name]=len(differences)
root=a.repo.resolve()
cmd=[sys.executable,str(root/'docs/reports/corpus-repin/diff_artifacts.py'),'--archived',str(archive),'--measured',str(td),'--output',str(a.out/'five-artifact-diff.json'),'--full']
with (a.out/'five-artifact-diff.log').open('w') as out:r=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT)
(a.out/'five-artifact-diff.exit').write_text(str(r.returncode)+'\n');assert r.returncode==0
nulls=[]
for side,where in [('base',a.base),('tip',a.tip)]:
 f=where/(a.route+'-null.jsonl');nulls.append([json.loads(x) for x in f.read_text().splitlines()]);receipts.append({'artifact':f.name,'side':side,'path':str(f.resolve()),'bytes':f.stat().st_size,'sha256':digest(f)})
assert len(nulls[0])==len(nulls[1]);null_changes=[];null_dictionary_changes=[]
for i,(x,y) in enumerate(zip(*nulls)):
 assert (x['file'],x['function'])==(y['file'],y['function'])
 if x!=y:null_changes.append({'row_index':i,'file':x['file'],'function':x['function'],'differences':list(diff(x,y))})
 if x.get('null')!=y.get('null'):null_dictionary_changes.append({'row_index':i,'base':x.get('null'),'tip':y.get('null')})
write(a.out/'null-full-record-differences.json.gz',null_changes);write(a.out/'null-dictionary-differences.json.gz',null_dictionary_changes)
for name in [a.route+'-null.summary.json',a.route+'-outer.json']:
 bp,tp=a.base/name,a.tip/name;differences=list(diff(json.loads(bp.read_text()),json.loads(tp.read_text())));write(a.out/(name+'.raw-differences.json.gz'),differences);raw_counts[name]=len(differences)
 for side,f in [('base',bp),('tip',tp)]:receipts.append({'artifact':name,'side':side,'path':str(f.resolve()),'bytes':f.stat().st_size,'sha256':digest(f)})
summary={'route':a.route,'units':len(b['units']),'unit_status_counts':{side:dict(collections.Counter(u.get('status') for u in doc['units'])) for side,doc in [('base',b),('tip',t)]},'unit_status_changes':unit_status_changes,'source_inventory_counts':{side:{'raw_source_function_entries':sum(len(u.get('source_functions',[])) for u in doc['units']),'analyzed_function_rows':sum(len(u.get('functions',[])) for u in doc['units']),'not_projected_entries':sum(len(u.get('not_projected',[])) for u in doc['units'])} for side,doc in [('base',b),('tip',t)]},'denominator_note':'Function tallies describe analyzed rows only, not whole-source safety coverage. Every selected unit remains in unit counts; raw source names and not_projected inventories are reported separately, including unread or compiled-out source. No safety percentage is inferred.','counts':{'base':tally(b),'tip':tally(t)},'diagnostic_counts':{'base':b['diagnostic_counts'],'tip':t['diagnostic_counts']},'status_changes':[{k:v for k,v in r.items() if k not in ['base','tip']}|{'base':r['base']['status'],'tip':r['tip']['status']} for r in status],'changed_rows':len(changes),'diagnostic_trade_rows':len(leaf_changes),'null_rows':len(nulls[0]),'null_full_record_changes':len(null_changes),'null_dictionary_changes':len(null_dictionary_changes),'raw_difference_counts':raw_counts,'fields_masked':0,'fields_erased':0,'attribution_status':'Raw differences retained; interpretation and attribution must be written after inspection.','artifact_receipts':receipts}
write(a.out/'summary.json',summary);print(json.dumps({k:v for k,v in summary.items() if k!='artifact_receipts'},indent=2))
