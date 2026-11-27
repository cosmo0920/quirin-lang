"""Caller census of the refused out slots: how each call site passes the slot.

usage: callers.py <census.json> <fluent-bit root> <out.json>
"""
import json, os, re, sys, collections as C

census_path, fb_root, out_path = sys.argv[1:4]
census = json.load(open(census_path))
sources = {}
for top in ('src', 'plugins', 'lib'):
    for dirpath, _, files in os.walk(os.path.join(fb_root, top)):
        if '/tests' in dirpath or '/examples' in dirpath:
            continue
        for name in files:
            if name.endswith('.c'):
                path = os.path.join(dirpath, name)
                sources[os.path.relpath(path, fb_root)] = open(path, errors='replace').read()


def args_at(text, open_paren):
    depth, start, out, i = 0, open_paren + 1, [], open_paren
    while i < len(text):
        ch = text[i]
        if ch in '([{':
            depth += 1
        elif ch in ')]}':
            depth -= 1
            if depth == 0:
                out.append(text[start:i])
                return out
        elif ch == ',' and depth == 1:
            out.append(text[start:i])
            start = i + 1
        i += 1
    return None


def signature_index(function, slot, defining_file):
    text = sources.get(defining_file, '')
    m = re.search(r'(?m)^[A-Za-z_][^;{}()]*?\b' + re.escape(function) + r'\s*\(', text)
    if not m:
        return None
    params = args_at(text, m.end() - 1) or []
    for index, p in enumerate(params):
        if re.search(r'\b' + re.escape(slot) + r'\s*$', ' '.join(p.split())):
            return index
    return None


def classify(actual):
    a = ' '.join(actual.split())
    if a in ('NULL', '0'):
        return 'NULL'
    if re.fullmatch(r'&\s*\w+', a):
        return '&local'
    if re.fullmatch(r'&\s*[\w.\->\[\]]+', a):
        return '&member'
    if re.fullmatch(r'\w+', a):
        return 'forwarded pointer'
    return 'other'


rows = {}
for o in census:
    if o['root'] != 'formal':
        continue
    key = (o['file'], o['function'], o['slot'])
    if key in rows:
        continue
    slot = o['slot'].lstrip('*')
    index = signature_index(o['function'], slot, o['file'])
    kinds = C.Counter()
    same_unit = 0
    if index is not None:
        call = re.compile(r'(?<![\w.>])' + re.escape(o['function']) + r'\s*\(')
        for path, text in sources.items():
            for m in call.finditer(text):
                # skip the definition and prototypes
                line_start = text.rfind('\n', 0, m.start()) + 1
                prefix = text[line_start:m.start()]
                if re.match(r'^[A-Za-z_][\w\s\*]*$', prefix) and not re.search(r'\b(return|if|while)\b|=', prefix):
                    continue
                args = args_at(text, m.end() - 1)
                if not args or index >= len(args):
                    continue
                kinds[classify(args[index])] += 1
                same_unit += path == o['file']
    rows[key] = dict(file=o['file'], function=o['function'], slot=o['slot'], index=index,
                     calls=sum(kinds.values()), same_unit=same_unit, actuals=dict(kinds))
json.dump(list(rows.values()), open(out_path, 'w'), indent=1)
total = C.Counter()
for r in rows.values():
    total.update(r['actuals'])
print('slots', len(rows), 'call sites', sum(total.values()), dict(total))
print('slots with no call site found', sum(1 for r in rows.values() if r['calls'] == 0))
print('call sites in the defining file', sum(r['same_unit'] for r in rows.values()))
