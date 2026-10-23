#!/usr/bin/env python3
"""Retain every changed row and diagnostic leaf; never collapse replay rows."""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path)
a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=False)
read=lambda path:json.loads(path.read_bytes())
def rows(d):
 result={}
 for index,u in enumerate(d['units']):
  for f in u['functions']:
   key=(index,u['library'],u['file'],f['function'])
   assert key not in result,key
   result[key]=f
 return result
base,tip=read(a.base),read(a.tip)
b,t=rows(base),rows(tip)
assert b.keys()==t.keys(),'denominator changed; attribute inventory before comparing'
assert [(u['library'],u['file']) for u in base['units']]==[(u['library'],u['file']) for u in tip['units']]
def leaves(f):
 return collections.Counter(json.dumps(x,sort_keys=True) for x in f['diagnostics'])
changes=[]
for key in b:
 before,after=b[key],t[key]
 if before==after:continue
 lb,lt=leaves(before),leaves(after)
 changes.append({'key':list(key),'status_before':before['status'],'status_after':after['status'],
  'removed_leaves':[json.loads(x) for x,n in (lb-lt).items() for _ in range(n)],
  'added_leaves':[json.loads(x) for x,n in (lt-lb).items() for _ in range(n)],
  'before':before,'after':after})
with gzip.open(a.out/'complete-changed-rows.json.gz','wt') as f:json.dump(changes,f,sort_keys=True)
status=[{'key':x['key'],'before':x['status_before'],'after':x['status_after']} for x in changes if x['status_before']!=x['status_after']]
report={'inputs':{side:{'path':str(path.resolve()),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()} for side,path in [('base',a.base),('tip',a.tip)]},
 'denominator':len(b),'before':dict(collections.Counter(x['status'] for x in b.values())),
 'after':dict(collections.Counter(x['status'] for x in t.values())), 'status_changes':status,
 'changed_rows':len(changes),'leaf_changes':[{'key':x['key'],'removed':x['removed_leaves'],'added':x['added_leaves']} for x in changes if x['removed_leaves'] or x['added_leaves']],
 'complete_rows':'complete-changed-rows.json.gz','all_original_fields_retained':True,
 'attribution':'Pending semantic review of every changed row; this script does not assign causes.'}
(a.out/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['denominator','before','after','changed_rows','status_changes']}))
