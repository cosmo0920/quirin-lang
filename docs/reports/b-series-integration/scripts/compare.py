"""Compare two raw diagnostic JSON artifacts, preserving leaf multiplicities.

Print every changed unit/function verdict and added/removed internal-coordinate
leaf. This computes deltas; it does not decide whether an admission is sound.
"""
import collections, json, pathlib, sys
before=json.loads(pathlib.Path(sys.argv[1]).read_text());after=json.loads(pathlib.Path(sys.argv[2]).read_text())
def indices(data):
 units={}; funcs={}; leaves=collections.Counter()
 for u in data['units']:
  uk=(u['library'],u['file']); units[uk]=u.get('status')
  for f in u.get('functions',[]):
   fk=(*uk,f['function']); funcs[fk]=f.get('status')
   for d in f.get('diagnostics',[]): leaves[(*fk,d.get('line'),d.get('category'),d.get('description'))]+=1
 return units,funcs,leaves
bu,bf,bl=indices(before); au,af,al=indices(after)
def delta(b,a): return [{'key':k,'before':b.get(k),'after':a.get(k)} for k in sorted(b.keys()|a.keys()) if b.get(k)!=a.get(k)]
def rows(counter): return [{'key':k,'count':v} for k,v in sorted(counter.items())]
x={'unit_counts':{'before':dict(collections.Counter(bu.values())),'after':dict(collections.Counter(au.values()))},'function_counts':{'before':dict(collections.Counter(bf.values())),'after':dict(collections.Counter(af.values()))},'diagnostic_counts':{'before':before.get('diagnostic_counts'),'after':after.get('diagnostic_counts')},'unit_moves':delta(bu,au),'function_moves':delta(bf,af),'leaves_removed':rows(bl-al),'leaves_added':rows(al-bl)}
print(json.dumps(x,indent=2))
