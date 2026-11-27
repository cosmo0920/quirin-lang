"""Typed shape of every final refusal the classifier recorded for a leaf.

Usage: shapes.py JOINED.json PP_DIR OUT.json

The statement text is the classifier's own token run; every name's type is
Clang's (the function's VarDecl/ParmVarDecl declarations, from
-ast-dump=json of the same -E -P text). A name declared more than once with
different classes is `ambiguous`.
"""
import collections as C, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

joined = json.load(open(sys.argv[1]))
PP, OUT = sys.argv[2], sys.argv[3]
CLANG = '/usr/lib/llvm-21/bin/clang'


def walk(n):
    yield n
    for c in n.get('inner', []) or []:
        if isinstance(c, dict):
            yield from walk(c)


def tclass(t):
    t = (t.get('desugaredQualType') or t.get('qualType') or '')
    if 'volatile' in t or '_Atomic' in t:
        return 'volatile'
    s = t.replace('const', '').replace('restrict', '').strip()
    if s.endswith('*') or '(*)' in s:
        return 'pointer'
    if '[' in s:
        return 'array'
    if s.startswith(('struct', 'union')):
        return 'record'
    words = s.split()
    if words and all(w in ('signed', 'unsigned', 'short', 'long', 'char', 'int', 'float', 'double', '_Bool', '__int128') for w in words):
        return 'scalar'
    if s.startswith('enum'):
        return 'scalar'
    return 'other:' + s


def names_of(unit, fn):
    src = os.path.join(PP, unit.replace('/', '__') + '.i')
    p = subprocess.run([CLANG, '-x', 'c', '-std=gnu11', '-fsyntax-only', '-w', '-Xclang', '-ast-dump=json',
                        '-Xclang', '-ast-dump-filter=' + fn, src], capture_output=True, text=True)
    dec = json.JSONDecoder()
    text = p.stdout
    i = 0
    found = None
    while i < len(text):
        while i < len(text) and text[i].isspace():
            i += 1
        if i >= len(text):
            break
        o, i = dec.raw_decode(text, i)
        if o.get('kind') == 'FunctionDecl' and o.get('name') == fn and any(
                c.get('kind') == 'CompoundStmt' for c in o.get('inner', [])):
            found = o
    types = C.defaultdict(set)
    storage = C.defaultdict(set)
    if found:
        for n in walk(found):
            if n.get('kind') in ('VarDecl', 'ParmVarDecl') and n.get('name'):
                types[n['name']].add(tclass(n.get('type', {})))
                storage[n['name']].add(n.get('storageClass', 'auto') + ('-parm' if n['kind'] == 'ParmVarDecl' else ''))
    return {k: (sorted(v)[0] if len(v) == 1 else 'ambiguous') for k, v in types.items()}, {k: sorted(v) for k, v in storage.items()}


def operand_class(toks, at, before, types):
    if before:
        j = at - 1
        if j < 0:
            return 'none'
        t = toks[j]
        if t in (']', ')'):
            return 'path'
        if j >= 1 and toks[j - 1] in ('.', '>'):
            return 'path'
        return types.get(t, 'undeclared')
    j = at + 2
    if j >= len(toks):
        return 'none'
    t = toks[j]
    if j + 1 < len(toks) and toks[j + 1] in ('.', '-', '['):
        return 'path'
    if t in ('(', '*'):
        return 'path'
    return types.get(t, 'undeclared')


def features(toks, types):
    f = set()
    for k, t in enumerate(toks):
        if t == '__qcore_literal__':
            f.add('string')
        elif t == 'sizeof':
            f.add('sizeof')
        elif t == '{':
            f.add('brace')
        elif t == '?':
            f.add('conditional')
        elif t in ('.', '>') and k > 0 and (t == '.' or toks[k - 1] == '-'):
            f.add('member')
        elif t == '[':
            f.add('subscript')
        elif types.get(t) == 'pointer':
            f.add('pointer-name')
        elif types.get(t) in ('volatile',):
            f.add('volatile-name')
        elif types.get(t) == 'ambiguous':
            f.add('ambiguous-name')
        elif t == '(' and k > 0 and toks[k - 1][:1].isalpha() and toks[k - 1] not in ('sizeof', 'if', 'while', 'return') and toks[k - 1] not in types:
            f.add('call')
    return f


def shape(form, text, types, storage):
    toks = text.split()
    f = features(toks, types)
    if form == 'INCDEC':
        ops = [k for k in range(len(toks) - 1) if toks[k] == toks[k + 1] and toks[k] in '+-']
        classes = []
        for k in ops:
            post = k > 0 and (toks[k - 1][:1].isalnum() or toks[k - 1][:1] == '_' or toks[k - 1] in (']', ')'))
            classes.append(operand_class(toks, k, post, types))
        whole = len(toks) == 3 and ops in ([0], [1])
        ctx = 'statement' if whole else ('comma' if ',' in toks and '=' not in toks and 'call' not in f else ('in-store' if '=' in toks else ('call-actual' if 'call' in f else 'nested')))
        return 'incdec', '+'.join(sorted(set(classes))), ctx, ','.join(sorted(f))
    if form == 'BRACE':
        if toks and toks[0] in ('typedef', 'struct', 'union', 'enum') and '=' not in toks[:toks.index('{')] if '{' in toks else False:
            kind = 'type-declaration'
        elif '( {' in text:
            kind = 'statement-expression'
        elif ') {' in text:
            kind = 'compound-literal'
        elif '= {' in text:
            kind = 'initializer-list'
        else:
            kind = 'other-brace'
        return 'brace', kind, '', ','.join(sorted(f - {'brace'}))
    if form in ('ASSIGN', 'DEST'):
        if '=' not in toks:
            return form.lower(), 'no-equal', '', ','.join(sorted(f))
        e = toks.index('=')
        left = toks[:e]
        op = ''
        if left and left[-1] in '+-*/%&|^<>':
            op = left[-1] + '='
            left = left[:-1]
            if op in ('<=', '>=') and left and left[-1] in '<>':
                left = left[:-1]
        decl = len(left) >= 2 and left[-1][:1].isalpha() and left[-2][:1].isalpha() or (len(left) >= 2 and left[-2] in ('*', 'const'))
        dest = left[-1] if left else ''
        if left and (left[-1] in (']', ')') or '.' in left or '>' in left or left[0] == '*'):
            dclass = 'path'
        else:
            dclass = types.get(dest, 'undeclared')
        right = toks[e + 1:]
        rf = features(right, types)
        if ',' in toks and decl:
            kind = 'multi-declarator'
        else:
            kind = ('declaration' if decl else 'assignment') + (' ' + op if op else '')
        return form.lower(), kind, dclass, ','.join(sorted(rf))
    if form == 'SIZEOF':
        return 'sizeof', ('declaration' if toks and '=' in toks and toks[0][:1].isalpha() and toks[1][:1].isalpha() else 'statement'), '', ','.join(sorted(f - {'sizeof'}))
    return form, '', '', ''


keys = sorted({(r['unit'], r['fn']) for r in joined if r['statements']})
with ThreadPoolExecutor(max_workers=6) as ex:
    named = dict(zip(keys, ex.map(lambda k: names_of(*k), keys)))
out = []
for r in joined:
    types, storage = named.get((r['unit'], r['fn']), ({}, {}))
    out.append(dict(r, shapes=[[s] + list(shape(r['form'], s, types, storage)) for s in r['statements']]))
json.dump(out, open(OUT, 'w'), indent=0)
for form in ['ASSIGN', 'DEST', 'BRACE', 'INCDEC', 'SIZEOF']:
    c = C.Counter()
    for r in out:
        if r['form'] != form:
            continue
        if not r['shapes']:
            c[('(no final refusal recorded)',)] += 1
        for s in r['shapes']:
            c[tuple(s[1:])] += 1
    print('=====', form)
    for k, v in c.most_common(40):
        print('  ', v, k)
