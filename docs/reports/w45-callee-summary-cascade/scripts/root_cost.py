import json,collections,re,sys
sys.path.insert(0,'measure/census')
CALL=re.compile(r"call to uncontracted function `([^`]+)`")
norm=lambda s: re.sub(r'`[^`]*`','X',s)
d=json.load(open('measure/before/resource-diagnostics.json'))
import gzip
sites=json.load(gzip.open('docs/reports/w44-corpus-callee-summaries/census/definition-sites.json.gz','rt'))['units']
scope={}
for line in gzip.open('docs/reports/w44-corpus-callee-summaries/census/scope-census.jsonl.gz','rt'):
    row=json.loads(line); scope.setdefault(tuple(sorted(row['selected'])),row)
text=collections.defaultdict(dict)
for r in map(json.loads,open('measure/census/cone-lines.jsonl')):
    text[(r['unit'],r['function'])][r['line']]=r['text']
roots=[]
for u in d['units']:
    key=u['library']+'/'+u['file']
    fns={f['function']:f for f in u['functions']}
    sc={c['name']:c for c in scope.get(tuple(sorted(fns)),{'callees':[]})['callees']}
    for f in u['functions']:
        if f['status']!='Unknown' or not f['diagnostics'] or not all(CALL.fullmatch(x['description']) for x in f['diagnostics']): continue
        seen=set(); leaves=[]; external=set(); work=[f['function']]
        while work:
            n=work.pop()
            if n in seen: continue
            seen.add(n)
            for x in fns[n]['diagnostics']:
                m=CALL.fullmatch(x['description'])
                if m:
                    c=m.group(1)
                    if c in fns:
                        if fns[c]['status']=='Unknown': work.append(c)
                        elif fns[c]['status']!='Safe': external.add(c+':unsafe')
                    elif c in sc: external.add(c+':hdr-'+sc[c]['outcome'])
                    else:
                        s=sites.get(key,{}).get('definitions',{}).get(c)
                        external.add(c+(':hdr' if s and s['file']!='<stdin>' else ':ext'))
                elif n!=f['function']:
                    leaves.append((n,norm(x['description']),text[(key,n)].get(x['line'],'?')))
        roots.append({'unit':key,'fn':f['function'],'cost':len(leaves),'external':sorted(external),'leaves':leaves,'callees':len(seen)-1})
json.dump(roots,open('measure/census/root-cost.json','w'),indent=1)
internal=[r for r in roots if not r['external']]
print('roots',len(roots),'internal-only',len(internal))
h=collections.Counter(min(r['cost'],50)//5*5 for r in internal); print(sorted(h.items()))
fam=collections.Counter()
for r in internal:
    if r['cost']<=10:
        for l in r['leaves']: fam[l[1]]+=1
print(fam.most_common(12))
ext=collections.Counter(e.split(':',1)[1] for r in roots for e in r['external'])
print(ext)
