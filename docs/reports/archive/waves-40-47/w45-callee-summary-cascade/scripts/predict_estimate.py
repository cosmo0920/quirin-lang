# Approximate leverage: remove (a) indirect-access leaves whose text is a one-call
# statement CALLEE(B->F) with a release-shaped callee, (b) unresolved identity
# leaves on a release-shaped call of a plain name in the same function that had an (a).
COND=True
import os
LOWER=os.environ.get('LOWER')=='1'
NULLABLE={'free','rd_free','flb_free','mk_mem_free','cfl_sds_destroy','flb_sds_destroy','rd_kafka_mem_free','cmt_sds_destroy'}
import json,re,collections,sys,pathlib
sys.path.insert(0,'docs/reports/w44-corpus-callee-summaries/scripts')
import definition_sites
CALL=re.compile(r"call to uncontracted function `([^`]+)`")
REL=re.compile(r'(free|destroy|release|unref|cleanup|fini|term)',re.I)
d=json.load(open('measure/before/resource-diagnostics.json'))
texts=[]
for p in sorted(pathlib.Path('measure/capture/pp').glob('*-p1.i')):
    t=p.read_text(errors='replace'); texts.append((set(definition_sites.definitions(t)),t))
nodes={}; removed=collections.Counter()
for u in d['units']:
    key=u['library']+'/'+u['file']; names={f['function'] for f in u['functions']}
    m=sorted((t for t in texts if names<=t[0]),key=lambda t:len(t[0]))
    lines=m[0][1].split('\n') if m else []
    for f in u['functions']:
        keep=[]; hit=False
        diags=f['diagnostics']
        for x in diags:
            t=lines[x['line']-1].strip() if 0<x['line']<=len(lines) else ''
            mm=re.fullmatch(r'([A-Za-z_]\w*)\s*\(\s*\(?\s*([A-Za-z_]\w*)\s*->\s*([A-Za-z_]\w*)\s*\)?\s*\)\s*;',t)
            if x['description'].startswith('indirect memory access') and mm and (mm.group(1) in NULLABLE if LOWER else REL.search(mm.group(1))):
                hit=True; removed['member-release']+=1; continue
            if COND and x['description'].startswith('unmodeled call, aggregate or indirect access in branch condition') and re.search(r'\bif\s*\(\s*\(*\s*!?\s*[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*\s*\)*\s*((!=|==)\s*(\(\(void\s*\*\)0\)|NULL|0))?\s*\)*\s*\)',t):
                hit=True; removed['member-null-test']+=1; continue
            keep.append(x)
        if hit:
            k2=[]
            for x in keep:
                t=lines[x['line']-1].strip() if 0<x['line']<=len(lines) else ''
                if x['description'].startswith('unresolved resource identity') and re.fullmatch(r'[A-Za-z_]\w*\s*\(\s*[A-Za-z_]\w*\s*\)\s*;',t) and REL.search(t):
                    removed['poisoned-release']+=1; continue
                k2.append(x)
            keep=k2
        nodes[(key,f['function'])]=(f['status'],keep)
print(removed)
def clean_set(nodes):
    clean={n for n,(s,_) in nodes.items() if s=='Safe'}
    ch=True
    while ch:
        ch=False
        for n,(s,ds) in nodes.items():
            if n in clean or s!='Unknown': continue
            if all((m:=CALL.fullmatch(x['description'])) and (n[0],m.group(1)) in clean for x in ds):
                clean.add(n); ch=True
    return clean
base={n:(s,f['diagnostics']) for u in d['units'] for f in u['functions'] for n,s in [((u['library']+'/'+u['file'],f['function']),f['status'])]}
b=clean_set(base); a=clean_set(nodes)
new=sorted(a-b); print(len(new)); import json; json.dump(new,open(os.environ.get('OUT','/dev/null'),'w'),indent=0); print(new[:80])
