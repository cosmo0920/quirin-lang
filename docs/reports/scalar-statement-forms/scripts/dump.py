import json, collections as C, sys

R = '/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json'
SRC = '/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/'
d = json.load(open(R))
FORMS = {
    'unsupported assignment expression': 'ASSIGN',
    'unrecognized assignment destination': 'DEST',
    'braced initializer, expression, or declaration is not modeled': 'BRACE',
    'unmodeled increment or decrement expression': 'INCDEC',
    'C operator `sizeof` requires type/operand analysis': 'SIZEOF',
}
cache = {}


def line(o):
    f, l = o.rsplit(':', 1)
    if f not in cache:
        try:
            cache[f] = open(SRC + f, errors='replace').read().split('\n')
        except Exception:
            cache[f] = None
    L = cache[f]
    if L is None:
        return None
    return L[int(l) - 1]


rows = []
for u in d['units']:
    for f in u['functions']:
        descs = [x['description'] for x in f.get('diagnostics', [])]
        for x in f.get('diagnostics', []):
            k = FORMS.get(x['description'])
            if not k:
                continue
            o = x.get('origin') or ''
            rows.append(dict(form=k, unit=u['file'], ustatus=u['status'], fn=f['function'],
                             fstatus=f['status'], origin=o, line=x['line'],
                             text=(line(o) or '').strip() if o else '',
                             others=sorted(set(descs) - {x['description']})))
json.dump(rows, open(sys.argv[1], 'w'), indent=0)
print(C.Counter(r['form'] for r in rows))
print(C.Counter(bool(r['origin']) for r in rows))
