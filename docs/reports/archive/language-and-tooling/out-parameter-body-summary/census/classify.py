"""Lexical census of the refused out-slot stores (card OUT-PARAMETER-BODY-SUMMARY).

usage: classify.py <sites.json> <fluent-bit root> <out.json>

Reads each site's original source line and its function's definition, and
classifies (1) the slot's pointee type, (2) the stored value, (3) the body's
return shape, (4) the callers' actuals at the slot position. Lexical only:
macro expansions are read from the line the report names.
"""
import json, os, re, sys, collections as C

sites_path, root, out_path = sys.argv[1:4]
rows = json.load(open(sites_path))

SCALAR_WORDS = {'int', 'char', 'short', 'long', 'unsigned', 'signed', 'size_t', 'ssize_t',
                'int8_t', 'int16_t', 'int32_t', 'int64_t', 'uint8_t', 'uint16_t', 'uint32_t',
                'uint64_t', 'off_t', 'double', 'float', 'socklen_t', 'rd_bool_t', 'mk_bool_t'}
ALLOC = re.compile(r'\b(\w*(malloc|calloc|realloc|strdup|strndup|_create|_new|_new0|_dup|_copy|'
                   r'sds\w*|alloc\w*)\w*)\s*\(')
_files = {}


def source(path):
    if path not in _files:
        try:
            _files[path] = open(os.path.join(root, path), errors='replace').read()
        except OSError:
            _files[path] = ''
    return _files[path]


def definition(text, function):
    for m in re.finditer(r'(?m)^[A-Za-z_][^;{}()]*?\b' + re.escape(function) + r'\s*\(', text):
        depth = 0
        i = m.end() - 1
        while i < len(text):
            if text[i] == '(':
                depth += 1
            elif text[i] == ')':
                depth -= 1
                if depth == 0:
                    break
            i += 1
        rest = text[i + 1:i + 200].lstrip()
        if rest.startswith('{'):
            open_at = text.index('{', i)
            depth = 0
            j = open_at
            while j < len(text):
                if text[j] == '{':
                    depth += 1
                elif text[j] == '}':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            return text[m.start():i + 1], text[open_at:j + 1]
    return None, None


def formal_type(signature, name):
    if not signature:
        return None
    params = signature[signature.index('(') + 1:-1]
    for p in re.split(r',(?![^()]*\))', params):
        p = ' '.join(p.split())
        if re.search(r'\b' + re.escape(name) + r'\s*$', p):
            return p[:p.rfind(name)].strip()
    return None


def pointee(type_text, enum_typedefs):
    if type_text is None:
        return 'unknown'
    stars = type_text.count('*')
    words = [w for w in re.findall(r'\w+', type_text) if w not in ('const', 'volatile', 'struct', 'enum', 'union')]
    if stars >= 2:
        return 'pointer'
    if stars == 1:
        if 'void' in words and len(words) == 1:
            return 'void'
        if any(w in SCALAR_WORDS for w in words):
            return 'scalar'
        if 'enum' in type_text or any(w in enum_typedefs for w in words):
            return 'enum'
        if any(w.endswith('_t') for w in words):
            return 'typedef'
        return 'record'
    return 'unknown'


def value_class(slot_kind, rhs, body):
    rhs = rhs.strip().rstrip(';').strip()
    if slot_kind in ('scalar', 'enum'):
        return 'scalar'
    if rhs in ('NULL', '0', '(void *) 0', '((void*)0)'):
        return 'null'
    if rhs.startswith('"'):
        return 'static string'
    if rhs.startswith("'"):
        return 'scalar (char)'
    if re.fullmatch(r'[A-Z][A-Z0-9_]+', rhs):
        return 'scalar (constant)'
    if re.fullmatch(r'\w+', rhs):
        assigns = re.findall(r'\b' + re.escape(rhs) + r'\s*=\s*([^;]+);', body or '')
        if any(ALLOC.search(a) for a in assigns):
            return 'fresh (allocator/constructor local)'
        if any(('->' in a or '&' in a or 'entry' in a or '[' in a) for a in assigns):
            return 'borrowed (interior/element local)'
        if not assigns:
            return 'local/formal (no assignment seen)'
        return 'local (other source)'
    if '?' in rhs:
        return 'conditional'
    if ALLOC.search(rhs):
        return 'fresh (call)'
    return 'other expression'


enum_typedefs = set()
for path in {r['file'] for r in rows}:
    pass

out = []
for r in rows:
    origin_path = r['sites'][0]['origin'].rsplit(':', 1)[0] if r['sites'] and ':' in r['sites'][0]['origin'] else None
    text = source(origin_path) if origin_path else ''
    signature, body = definition(text, r['function'])
    returns = re.findall(r'\breturn\b\s*([^;]*);', body or '')
    literal = all(re.fullmatch(r'-?\s*\d+|[A-Z][A-Z0-9_]*(OK|ERROR|ERR|TRUE|FALSE)\w*', x.strip()) for x in returns) if returns else False
    result = ' '.join(signature[:signature.index(r['function'])].split()) if signature else '?'
    for s in r['sites']:
        slot = s['slot'].lstrip('*')
        macro = slot == '_vp'
        declared = formal_type(signature, slot)
        slot_root = 'macro local' if macro else ('formal' if declared is not None else 'local')
        if declared is None and not macro:
            m = re.search(r'(?m)^\s*((?:const\s+|struct\s+|unsigned\s+)*\w+)\s*(\*+)?\s*' + re.escape(slot) + r'\s*(\[)?', body or '')
            if m:
                declared = m.group(1) + ' ' + (m.group(2) or '') + ('*' if m.group(3) else '')
        kind = 'scalar (macro local)' if macro else pointee(declared, enum_typedefs)
        if kind in ('typedef',):
            # an enum typedef or an opaque record typedef; resolve by the enumerator stored
            rhs = s['text'].split('=', 1)[1] if '=' in s['text'] else ''
            kind = 'enum' if re.fullmatch(r'\s*[A-Z][A-Z0-9_]+\s*;.*', rhs) else 'typedef'
        rhs = s['text'].split('=', 1)[1] if '=' in s['text'] else ''
        rhs = rhs.split('//')[0].split('/*')[0]
        value = 'scalar (macro)' if macro else value_class(kind, rhs, body)
        out.append(dict(library=r['lib'], file=r['file'], function=r['function'],
                        sole=len(r['other']) == 0, slot=s['slot'], origin=s['origin'], root=slot_root,
                        pointee=kind, value=value, result=result,
                        literal_returns=literal, text=s['text']))
json.dump(out, open(out_path, 'w'), indent=1)
by = C.Counter((o['root'], o['pointee'], o['value']) for o in out)
for (r, k, v), n in sorted(by.items(), key=lambda kv: -kv[1]):
    fns = len({(o['file'], o['function']) for o in out if (o['root'], o['pointee'], o['value']) == (r, k, v)})
    sole = len({(o['file'], o['function']) for o in out if (o['root'], o['pointee'], o['value']) == (r, k, v) and o['sole']})
    print(f'{n:4} sites {fns:4} functions {sole:2} sole  root={r:12} pointee={k:22} value={v}')
print('sites', len(out), 'functions', len({(o["file"], o["function"]) for o in out}))
