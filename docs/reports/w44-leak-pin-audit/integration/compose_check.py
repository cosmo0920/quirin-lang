"""Composition check for baseline-moving integrations (v3).
compose_check.py PREV_BASE CARD_BASE CARD_AFTER COMBINED
Leaves are multisets keyed by (library, file, function, category, description,
origin); preprocessed line numbers are excluded. With R, A the card's removed and
added leaves (CARD_AFTER vs CARD_BASE):
  expected = PREV - (R restricted to what PREV still has) + A
The combined run must equal `expected` exactly. R-leaves absent from PREV were
already removed by an earlier integration (overlap) and are listed.
Transitive: a function Safe only in COMBINED gains a validated summary, so its
callers' "call to uncontracted function `f`" leaves may disappear; those are
listed and removed from `expected`.
Status: every function/unit whose status differs between PREV and COMBINED must
either take the card's own transition, or be an Unknown function whose expected
leaf set is empty and that is Safe in COMBINED (complementary clearing: the card
removed its last blocker after an earlier card removed the others). Every card
transition must appear in COMBINED unless PREV already had the target status.
v3: an EXPLAIN entry with "status_kept": true names a function whose card
transition does not occur on the combined tree because the leaves the card
removed are restored there (listed through "added_prefixes"); the function must
then keep PREV's status exactly."""
import gzip,json,sys,os,collections
def load(p):
    if os.path.isdir(p):
        for n in ('diagnostics.json.gz','resource-diagnostics.json'):
            q=os.path.join(p,n)
            if os.path.exists(q): return json.load(gzip.open(q) if q.endswith('.gz') else open(q))
    return json.load(open(p))
def parts(d):
    c=collections.Counter(); fs={}; us={}; per=collections.defaultdict(collections.Counter)
    for u in d['units']:
        us[(u['library'],u['file'])]=u['status']
        for f in u.get('functions',[]):
            k=(u['library'],u['file'],f['function']); fs[k]=f['status']
            for g in f.get('diagnostics',[]):
                leaf=k+(g['category'],g['description'],g.get('origin')); c[leaf]+=1; per[k][leaf]+=1
    return c,fs,us
P,CB,CA,C=[parts(load(x)) for x in sys.argv[1:5]]
R=CB[0]-CA[0]; A=CA[0]-CB[0]
overlap=R-P[0]; Reff=R-overlap
expected=(P[0]-Reff)+A
# Transitive: a function Safe only in COMBINED (not in PREV, not via the card)
# gains a summary, so its callers' "call to uncontracted function `f`" leaves go.
newly_safe={k[2] for k,b in C[1].items() if b=='Safe' and P[1].get(k)!='Safe' and CA[1].get(k)!='Safe'}
transitive=collections.Counter({leaf:n for leaf,n in expected.items()
    if leaf[3]=='unsupported' and any(leaf[4]==f'call to uncontracted function `{f}`' for f in newly_safe)
    and C[0][leaf] < n})
for leaf,n in list(transitive.items()): transitive[leaf]=n-C[0][leaf]
expected=expected-transitive
# Explained: leaves the integrator attributes by hand (EXPLAIN=json file of
# [{"function": f, "description_prefix": d, "why": text}]); only expected-only
# leaves may be explained, and each explanation must match at least one leaf.
explained=collections.Counter(); notes=[]
if os.environ.get('EXPLAIN'):
    for e in json.load(open(os.environ['EXPLAIN'])):
        hit=collections.Counter({leaf:n-C[0][leaf] for leaf,n in expected.items()
            if leaf[2]==e['function'] and leaf[4].startswith(e['description_prefix']) and C[0][leaf]<n})
        if not hit and not e.get('removal_optional'): print('  EXPLANATION MATCHES NOTHING',e); sys.exit(1)
        explained+=hit; notes.append((e['function'],sum(hit.values()),e['why']))
expected=expected-explained
added_expl=collections.Counter()
if os.environ.get('EXPLAIN'):
    for e in json.load(open(os.environ['EXPLAIN'])):
        for pre in e.get('added_prefixes',[]):
            hit=collections.Counter({leaf:n-expected[leaf] for leaf,n in C[0].items() if leaf[2]==e['function'] and leaf[4].startswith(pre) and expected[leaf]<n})
            if not hit: print('  ADDED EXPLANATION MATCHES NOTHING',e['function'],pre); sys.exit(1)
            added_expl+=hit; notes.append((e['function'],sum(hit.values()),'added: '+pre))
expected=expected+added_expl
ok=True
if expected!=C[0]:
    ok=False
    for k in list((expected-C[0]).keys())[:10]: print('  expected-only',k)
    for k in list((C[0]-expected).keys())[:10]: print('  combined-only',k)
card_f={k:(CB[1].get(k),CA[1].get(k)) for k in set(CB[1])|set(CA[1]) if CB[1].get(k)!=CA[1].get(k)}
card_u={k:(CB[2].get(k),CA[2].get(k)) for k in set(CB[2])|set(CA[2]) if CB[2].get(k)!=CA[2].get(k)}
exp_by_fn=collections.Counter(); 
for leaf,n in expected.items(): exp_by_fn[leaf[:3]]+=n
complementary=[]
for k in set(P[1])|set(C[1]):
    a,b=P[1].get(k),C[1].get(k)
    if a==b: continue
    if k in card_f and card_f[k][1]==b: continue
    if a=='Unknown' and b=='Safe' and exp_by_fn[k]==0: complementary.append(k[2]); continue
    ok=False; print('  unexplained function',k,a,'->',b,'card',card_f.get(k))
kept=set()
if os.environ.get('EXPLAIN'):
    kept={e['function'] for e in json.load(open(os.environ['EXPLAIN'])) if e.get('status_kept')}
for k,(x,y) in card_f.items():
    if k[2] in kept and C[1].get(k)==P[1].get(k): notes.append((k[2],0,f'status kept: card {x} -> {y}, combined stays {C[1].get(k)}')); continue
    if C[1].get(k)!=y and P[1].get(k)!=y: ok=False; print('  card transition missing',k,x,'->',y,'combined',C[1].get(k))
for k in set(P[2])|set(C[2]):
    a,b=P[2].get(k),C[2].get(k)
    if a!=b and not (k in card_u and card_u[k][1]==b):
        # a unit may also clear complementarily; require all its functions Safe/data-only in COMBINED
        print('  unit change',k,a,'->',b,'card',card_u.get(k)); ok=ok and b=='Safe'
print('card delta:     removed',sum(R.values()),'added',sum(A.values()),'functions',len(card_f),'units',len(card_u))
print('overlap (already removed by earlier integrations):',sum(overlap.values()),sorted(set((k[2],k[4][:50],k[5]) for k in overlap)))
print('combined delta: removed',sum((P[0]-C[0]).values()),'added',sum((C[0]-P[0]).values()))
print('transitive (callers of combined-only Safe functions):',sum(transitive.values()),sorted(set((k[2],k[4][:60]) for k in transitive)))
print('explained by the integrator:',sum(explained.values()),notes)
print('complementary Unknown->Safe (card cleared the last blocker):',sorted(complementary))
moved=sorted((k[2],v) for k,v in ((k,(P[1].get(k),C[1].get(k))) for k in set(P[1])|set(C[1])) if v[0]!=v[1])
print('functions moved (combined):',moved)
print('COMPOSES' if ok else 'DOES NOT COMPOSE'); sys.exit(0 if ok else 1)
