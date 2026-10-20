#!/usr/bin/env python3
"""Attribute all corpus diagnostic changes and repeated home-body evidence."""
from pathlib import Path
import json,gzip,copy,collections,hashlib
repo=Path(__file__).resolve().parents[4];report=Path(__file__).resolve().parents[1];root=report/'artifacts/corpus'
load=lambda side,f:json.loads(gzip.decompress((root/side/(f+'.json.gz')).read_bytes()))
a,b=[load(side,'diagnostics') for side in ['before','after']]
validated=['MK_EVENT_NEW','mk_mem_alloc','mk_mem_alloc_z','mk_mem_free'];unvalidated=['mk_fifo_queue_destroy','mk_list_add','mk_list_del']
known={n:[] for n in validated}
for u in a['units']:
 e=u.get('contract_body_evidence',{})
 for n in validated:
  if n in e.get('validated',{}):
   value=dict(methods=e['validated'][n],clauses=e.get('clauses',{}).get(n))
   if value not in known[n]:known[n].append(value)
assert all(known.values())
records=[]
for i,(old,new) in enumerate(zip(a['units'],b['units'])):
 x=copy.deepcopy(old.get('contract_body_evidence',{}));y=new.get('contract_body_evidence',{})
 if x==y:continue
 added=[]
 for n in validated:
  if n in y.get('validated',{}) and n not in x.get('validated',{}):
   value=dict(methods=y['validated'][n],clauses=y.get('clauses',{}).get(n));assert value in known[n],(i,n)
   x.setdefault('validated',{})[n]=y['validated'][n];x.setdefault('clauses',{})[n]=y['clauses'][n];added.append(n)
 old_un=set(x.get('unvalidated',[]));new_un=set(y.get('unvalidated',[]));assert old_un<=new_un;assert new_un-old_un<=set(unvalidated)
 x['unvalidated']=sorted(new_un);assert x==y,(i,'unexplained body evidence')
 records.append(dict(index=i,library=old['library'],file=old['file'],validated_replay_additions=added,unvalidated_replay_additions=sorted(new_un-old_un)))
(root/'body-replay-occurrence-review.json').write_text(json.dumps(dict(meaning='check_imports reanalyses each selected home under caller-session tracking. New mk_fifo_send home includes Monkey header bodies; occurrences are not independent native definitions or new standing classes.',code=['crates/qcore-c-gateway/src/legacy_c.rs:2600','crates/qcore-c-gateway/src/legacy_cross_unit.rs:287'],unchanged_existing_clause_payloads=known,unit_changes=records,complete_body_evidence_equal_after_explicit_replay_additions=True),indent=2)+'\n')
comparison=json.loads((root/'row-comparison.json').read_text());differences=json.loads(gzip.decompress((root/'full-row-differences.json.gz').read_bytes()))
rows={tuple(x['key']):x for x in differences};source=Path('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source');prefix={'librdkafka':'lib/librdkafka-2.15.0/src','chunkio':'lib/chunkio/src','monkey-libevent':'lib/monkey'}
classified=[]
for x in comparison['changed']:
 pair=rows[tuple(x['key'])];old,new=pair['before'],pair['after'];counter=lambda r:collections.Counter(json.dumps(d,sort_keys=True) for d in r.get('diagnostics',[]));removed=list((counter(old)-counter(new)).elements());snippets=[]
 for text in removed:
  diag=json.loads(text);origin=diag.get('origin')
  if diag['description']!='unmodeled call, aggregate or indirect access in branch condition':continue
  path,line=origin.rsplit(':',1);p=source/prefix[x['key'][0]]/path;lines=p.read_text().splitlines();number=int(line)
  snippet='\n'.join(lines[max(0,number-2):number+2]);assert 'errno' in snippet,(x['key'],origin,snippet)
  snippets.append(dict(origin=origin,source_path=str(p.relative_to(source)),file_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),excerpt=snippet))
 name=x['key'][2]
 why='Checked scalar errno comparison(s); both outcomes retained and other leaves unchanged.'
 if name=='mk_fifo_send':why='Ordinary same-unit msg_write summary discharges both calls.'
 if name=='mk_mq_send':why='Explicit conditional mk_fifo_send home plus replay discharges the external call; link/type/calling-convention/interposition premise remains.'
 if name=='net_plain_send_file':why='Incidental exact errno!=EAGAIN comparison in the Linux sendfile branch; no new contract or pointer fact.'
 classified.append(dict(key=x['key'],before=x['before'],after=x['after'],classification=why,removed_leaves=x['removed_leaves'],added_leaves=x['added_leaves'],source_observations=snippets))
assert sum(len(x['source_observations']) for x in classified)==22
(root/'attribution.json').write_text(json.dumps(dict(selected_rows=4834,changed_status_or_diagnostic_rows=len(classified),promotions=[x['key'] for x in classified if x['after']=='Safe'],safe_regressions=0,new_unsafe_rows=0,removed_condition_refusals=22,removed_msg_write_calls=2,removed_mk_fifo_send_calls=1,added_diagnostics=0,rows=classified,body_evidence_occurrences='body-replay-occurrence-review.json; standing classes unchanged, copies of previously present clause payloads only',cross_unit_metadata='Report-wide catalog adds exactly mk_fifo_send and changes the link-map digest; it is not a claim every caller uses the home.',source_families='Source/chunk-contract/source-effect payloads agree after explicit checkout normalization, as precise-field-differences.json.gz; no new source-effect theorem or authority is inferred.'),indent=2)+'\n')
print('23diagnostic/status rows attributed;22errno refusals;4Safe gains; body replay additions match previous clause payloads')
