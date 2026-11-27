"""Composition check for baseline-moving integrations.
compose_check.py PREV_BASE CARD_BASE CARD_AFTER COMBINED
  PREV_BASE  : combined baseline before this integration (e.g. indirect artifacts dir)
  CARD_BASE  : the baseline the card measured against (card's BEFORE)
  CARD_AFTER : the card's archived AFTER artifacts dir
  COMBINED   : this integration's run dir (resource-diagnostics.json) or artifacts dir
Holds iff (COMBINED - PREV_BASE) == (CARD_AFTER - CARD_BASE) as multisets of
(library, file, function, category, description, origin) leaves, and function /
unit status changes agree. Line numbers are excluded (preprocessed coordinates)."""
import gzip,json,sys,os,collections
def load(p):
    if os.path.isdir(p):
        for n in ('diagnostics.json.gz','resource-diagnostics.json'):
            q=os.path.join(p,n)
            if os.path.exists(q): return json.load(gzip.open(q) if q.endswith('.gz') else open(q))
    return json.load(open(p))
def leaves(d):
    c=collections.Counter(); fs={}; us={}
    for u in d['units']:
        us[(u['library'],u['file'])]=u['status']
        for f in u.get('functions',[]):
            fs[(u['library'],u['file'],f['function'])]=f['status']
            for g in f.get('diagnostics',[]):
                c[(u['library'],u['file'],f['function'],g['category'],g['description'],g.get('origin'))]+=1
    return c,fs,us
def delta(a,b):
    (ca,fa,ua),(cb,fb,ub)=leaves(a),leaves(b)
    rem=ca-cb; add=cb-ca
    fch={k:(fa.get(k),fb.get(k)) for k in set(fa)|set(fb) if fa.get(k)!=fb.get(k)}
    uch={k:(ua.get(k),ub.get(k)) for k in set(ua)|set(ub) if ua.get(k)!=ub.get(k)}
    return rem,add,fch,uch
P,CB,CA,C=[load(x) for x in sys.argv[1:5]]
r1,a1,f1,u1=delta(CB,CA); r2,a2,f2,u2=delta(P,C)
print('card delta:     removed',sum(r1.values()),'added',sum(a1.values()),'functions',len(f1),'units',len(u1))
print('combined delta: removed',sum(r2.values()),'added',sum(a2.values()),'functions',len(f2),'units',len(u2))
ok=(r1==r2 and a1==a2 and f1==f2 and u1==u2)
if not ok:
    for name,x,y in (('removed',r1,r2),('added',a1,a2)):
        for k in list((x-y).keys())[:8]: print('  card-only',name,k)
        for k in list((y-x).keys())[:8]: print('  combined-only',name,k)
    for k in set(f1)|set(f2):
        if f1.get(k)!=f2.get(k): print('  function',k,'card',f1.get(k),'combined',f2.get(k))
    for k in set(u1)|set(u2):
        if u1.get(k)!=u2.get(k): print('  unit',k,'card',u1.get(k),'combined',u2.get(k))
print('functions moved (combined):',sorted((k[2],v) for k,v in f2.items()))
print('COMPOSES' if ok else 'DOES NOT COMPOSE'); sys.exit(0 if ok else 1)
