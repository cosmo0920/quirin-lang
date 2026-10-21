"""Add only complete independently reconstructed producer identity preimages."""
import argparse,collections,copy,hashlib,json,pathlib
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('out',type=pathlib.Path);a=p.parse_args();P=pathlib.Path('private/plugin-premise-decomposition');read=lambda f:json.loads(f.read_bytes());js=lambda v:json.dumps(v,ensure_ascii=False,separators=(',',':'),sort_keys=True);sha=lambda v:hashlib.sha256(v.encode()).hexdigest();k=read(P/('known-'+a.route+'-4.json'));known=k['exact_strings'];evidence={}
def add(after,before,why):
 if after==before:return
 if after in known:assert known[after]==before
 known[after]=before;evidence.setdefault(after,[]).append(why)
cfg=read(P/('configuration-'+a.route+'-2.json'))
for after,before in cfg['exact_strings'].items():add(after,before,'complete source-pinned configured task and configuration preimage replay')
nulls={s:read(P/'null-environments'/(a.route+'-'+s+'.json')) for s in ['base','tip']}
assert nulls['base']['pin_holds']==nulls['tip']['pin_holds'] and nulls['base']['refuted']==nulls['tip']['refuted']
assert {key:known.get(value,value) for key,value in nulls['tip']['sources'].items()}==nulls['base']['sources']
for s,x in nulls.items():assert sha(x['descriptor'])==x['digest']
add(nulls['tip']['digest'],nulls['base']['digest'],'actual production bounds selection/pin replay and full descriptor canonical bytes')
rows={s:[json.loads(line) for line in (P/'measure-1'/a.route/s/(a.route+'-null.jsonl')).read_bytes().splitlines() if line] for s in nulls};assert len(rows['base'])==len(rows['tip']);diagnostics={s:read(P/'measure-1'/a.route/s/'resource-diagnostics.json') for s in nulls}
contexts={s:collections.defaultdict(collections.deque) for s in nulls}
for s,d in diagnostics.items():
 for u in d['units']:
  for r in u['functions']:
   if 'hypothesis_context' in r:contexts[s][(u['library'],u['file'],r['function'])].append(r['hypothesis_context'])
records=[]
for b,t in zip(rows['base'],rows['tip']):
 assert tuple(b[k] for k in ['library','file','function'])==tuple(t[k] for k in ['library','file','function']);key=tuple(b[k] for k in ['library','file','function'])
 for s,r in [('base',b),('tip',t)]:
  assert r['identity']['contracts_sha256']==nulls[s]['digest'];assert r['identity']['resource_contracts_sha256']==k['environment_hashes'][s]['function_table_sha256']
  if 'hypothesis_context' not in r:continue
  resource=contexts[s][key].popleft();derived=sha(js({'custody':resource['assumptions'],'null_identity':r['identity']}));assert derived==r['hypothesis_context']['assumptions'],(s,key,'null assumption preimage');scope=js({'scope':r['null']['scope'],'assumptions':r['null']['assumptions']});assert scope==r['hypothesis_context']['scope'];assert r['hypothesis_context']['property']=='c.null.deref.v1'
 if 'hypothesis_context' in b:
  assert 'hypothesis_context' in t
  bi,ti=copy.deepcopy(b['identity']),copy.deepcopy(t['identity']);ti['contracts_sha256']=nulls['base']['digest'];assert bi==ti,(key,'other null identity delta needs exact producer proof')
  add(t['hypothesis_context']['assumptions'],b['hypothesis_context']['assumptions'],'actual full null identity + actual paired custody source assumptions canonical SHA preimages, no Context transport')
  records.append({'artifact':key,'base_assumptions':b['hypothesis_context']['assumptions'],'tip_assumptions':t['hypothesis_context']['assumptions'],'both_actual_composite_preimages_recomputed':True})
k['derived_identity_proofs']={'configuration':cfg,'null_selection_and_pins':nulls,'null_context_preimages':records,'added_exact_string_justifications':evidence};a.out.write_text(json.dumps(k,indent=2)+'\n');print(a.route,len(records),len(known))
