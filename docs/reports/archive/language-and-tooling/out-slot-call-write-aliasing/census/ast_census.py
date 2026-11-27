"""AST census for OUT-SLOT-CALL-WRITE-ALIASING.

usage: ast_census.py <report.json> <route> <out.json> <compile-db>...

For every `Safe` row of the report, read the function's definition from the
preprocessed unit (`clang -E -P`, the analyzer's own text), keep the rows
that store through a formal (`*X = V`, `*X op= V`, `X[0] = V`), and read
those functions from Clang's JSON AST (`-ast-dump-filter`): every out-slot
store through a pointer-to-pointer formal (or a pointer formal of any other
pointee, listed apart), and every call after the first such store in source
order (or inside a loop holding one), each actual classified by what it may
designate:

  scalar     a non-pointer actual
  const      a pointer to a const-qualified type (the parameter type Clang
             converted the actual to, or the actual's own type in a variadic
             tail)
  fnptr      a function pointer
  null       a null pointer constant
  string     a string literal
  addr-local `&NAME`, `&NAME.f`, `&NAME[i]` or an array's decay, NAME an
             automatic local or a formal's own variable
  addr-static the same of a static or file-scope object
  local:N    the value of the automatic pointer local N
  formal:N   the value of the pointer formal N (another formal, or the
             slot's own formal)
  slot:N     the value `*N` of a formal's pointee
  member     a pointer read from a member or element
  call       a call's result
  other      anything else

Lexical prefilter, AST decision. Paths under /home/cosmo/Documents/GitHub/
qcore-c in the databases are read through /media/Data/GitHub/quirin-lang.
"""
import json, os, re, shlex, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

HOME = '/home/cosmo/Documents/GitHub/qcore-c'
REAL = '/media/Data/GitHub/quirin-lang'

report_path, route, out_path = sys.argv[1:4]
dbs = sys.argv[4:]
# Rows the parent cards predict `Safe` at the measurement base (`rev:aa3999333`)
# that the report (an earlier binary) has `Unknown`: read as if `Safe`.
ALSO = set(filter(None, os.environ.get('CENSUS_ALSO_SAFE', '').split(',')))


def real(text):
    return text.replace(HOME, REAL)


entries = []
for db in dbs:
    for e in json.load(open(db)):
        entries.append(dict(file=real(e['file']), directory=real(e['directory']),
                            command=real(e['command'])))

HINT = {'cmetrics': 'lib/cmetrics/', 'ctraces': 'lib/ctraces/', 'librdkafka': 'lib/librdkafka',
        'chunkio': 'lib/chunkio/', 'fluent-bit-chunk': 'fluent-bit-clang-source/src/',
        'monkey-libevent': 'monkey-source/'}


def entry_for(library, file):
    hits = [e for e in entries if e['file'].endswith('/' + file)]
    if len(hits) > 1 and library in HINT:
        hits = [e for e in hits if HINT[library] in e['file']] or hits
    if len(hits) > 1:
        hits = [e for e in hits if 'fluent-bit-clang-source/' + file in e['file']] or hits
    return hits[0] if hits else None


def clang_args(e, extra):
    args = shlex.split(e['command'])
    out = [args[0]]
    skip = False
    for a in args[1:]:
        if skip:
            skip = False
            continue
        if a in ('-o', '-MF', '-MT', '-MQ'):
            skip = True
            continue
        if a in ('-c', '-MD', '-MMD') or a == e['file'] or a.endswith('/' + os.path.basename(e['file'])):
            continue
        out.append(a)
    return out + extra + [e['file']]


def run(e, extra):
    r = subprocess.run(clang_args(e, extra), cwd=e['directory'], capture_output=True, text=True,
                       errors='replace')
    return r.stdout


def definition(text, function):
    for m in re.finditer(r'(?m)^[A-Za-z_][^;{}()#]*?\b' + re.escape(function) + r'\s*\(', text):
        depth, i = 0, m.end() - 1
        while i < len(text):
            if text[i] == '(':
                depth += 1
            elif text[i] == ')':
                depth -= 1
                if depth == 0:
                    break
            i += 1
        rest = text[i + 1:i + 400].lstrip()
        if rest.startswith('{'):
            j = open_at = text.index('{', i)
            depth = 0
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


def formals(signature):
    params = signature[signature.index('(') + 1:-1]
    names = []
    for p in re.split(r',(?![^()]*\))', params):
        m = re.search(r'(\w+)\s*(\[[^\]]*\])?\s*$', p.strip())
        if m and '*' in p:
            names.append(m.group(1))
    return names


def stores_through(body, names):
    for n in names:
        if re.search(r'\*\s*\(?\s*' + re.escape(n) + r'\s*\)?\s*(?:[-+*/%&|^]|<<|>>)?=(?!=)', body):
            return True
        if re.search(r'\b' + re.escape(n) + r'\s*\[\s*0\s*\]\s*=(?!=)', body):
            return True
    return False


# ---------------------------------------------------------------- AST reading

def strip(node):
    while node.get('kind') in ('ImplicitCastExpr', 'ParenExpr', 'CStyleCastExpr') and node.get('inner'):
        if node.get('kind') == 'ImplicitCastExpr' and node.get('castKind') == 'ArrayToPointerDecay':
            return node
        node = node['inner'][0]
    return node


def qual(node):
    t = node.get('type') or {}
    return t.get('desugaredQualType') or t.get('qualType') or ''


QUALIFIERS = ('const', 'volatile', 'restrict', '__restrict')


def unqualified(text):
    """`text` with its top-level qualifiers removed (`char *const` -> `char *`)."""
    text = text.strip()
    changed = True
    while changed:
        changed = False
        for q in QUALIFIERS:
            if text.endswith(' ' + q) or text.endswith('*' + q):
                text = text[:-len(q)].strip()
                changed = True
    return text


def is_pointer(text):
    return unqualified(text).endswith('*') or '(*)' in text


def pointee_const(text):
    """Whether the object a pointer of type `text` designates is const-qualified."""
    t = unqualified(text)
    if not t.endswith('*'):
        return False
    inner = t[:-1].strip()
    if '*' in inner:
        # `T *const *`: the pointee is a const pointer; `T **`: a mutable one.
        return unqualified(inner) != inner and any(inner.endswith(q) for q in ('const',))
    return 'const' in inner.replace('*', ' ').split()


def offset(node):
    r = node.get('range', {}).get('begin', {})
    for k in ('expansionLoc', 'spellingLoc'):
        if k in r and 'offset' in r[k]:
            return r[k]['offset']
    return r.get('offset')


def base_decl(node):
    """The declaration an lvalue `NAME`, `NAME.f`, `NAME[i]` designates storage of."""
    node = strip(node)
    k = node.get('kind')
    if k == 'DeclRefExpr':
        return node.get('referencedDecl', {})
    if k == 'MemberExpr' and not node.get('isArrow'):
        return base_decl(node['inner'][0])
    if k == 'ArraySubscriptExpr':
        b = node['inner'][0]
        bs = strip(b)
        if b.get('castKind') == 'ArrayToPointerDecay' or (b.get('kind') == 'ImplicitCastExpr' and b.get('castKind') == 'ArrayToPointerDecay'):
            return base_decl(b['inner'][0])
        if bs.get('kind') == 'ImplicitCastExpr' and bs.get('castKind') == 'ArrayToPointerDecay':
            return base_decl(bs['inner'][0])
        return None
    return None


def classify(arg, decls):
    t = qual(arg)
    if not is_pointer(t):
        return 'scalar'
    if '(*)' in t and '(' in t:
        return 'fnptr'
    if pointee_const(t):
        return 'const'
    n = strip(arg)
    k = n.get('kind')
    if k == 'ImplicitCastExpr' and n.get('castKind') == 'ArrayToPointerDecay':
        d = base_decl(n['inner'][0])
        return storage(d, decls)
    if k == 'StringLiteral':
        return 'string'
    if k in ('IntegerLiteral',) or (k == 'ImplicitCastExpr' and n.get('castKind') == 'NullToPointer'):
        return 'null'
    if k == 'GNUNullExpr':
        return 'null'
    if k == 'UnaryOperator' and n.get('opcode') == '&':
        d = base_decl(n['inner'][0])
        if d is None:
            inner = strip(n['inner'][0])
            if inner.get('kind') == 'MemberExpr' and inner.get('isArrow'):
                return 'member'
            if inner.get('kind') == 'ArraySubscriptExpr':
                return pointer_value(strip(inner['inner'][0]), decls)
            if inner.get('kind') == 'UnaryOperator' and inner.get('opcode') == '*':
                return pointer_value(strip(inner['inner'][0]), decls)
            return 'other'
        return storage(d, decls)
    return pointer_value(n, decls)


def storage(d, decls):
    if not d:
        return 'other'
    if d.get('kind') == 'ParmVarDecl':
        return 'addr-local'
    info = decls.get(d.get('id'))
    if info is None:
        return 'addr-static'
    return 'addr-local' if info['automatic'] else 'addr-static'


def pointer_value(n, decls):
    n = strip(n)
    k = n.get('kind')
    if k == 'DeclRefExpr':
        d = n.get('referencedDecl', {})
        if d.get('kind') == 'ParmVarDecl':
            return 'formal:' + d.get('name', '?')
        info = decls.get(d.get('id'))
        if info and info['automatic']:
            return 'local:' + d.get('name', '?')
        return 'static:' + d.get('name', '?')
    if k == 'BinaryOperator' and n.get('opcode') in ('+', '-'):
        a, b = n['inner'][0], n['inner'][1]
        return pointer_value(a if is_pointer(qual(a)) else b, decls)
    if k == 'UnaryOperator' and n.get('opcode') == '*':
        inner = strip(n['inner'][0])
        if inner.get('kind') == 'DeclRefExpr' and inner.get('referencedDecl', {}).get('kind') == 'ParmVarDecl':
            return 'slot:' + inner['referencedDecl'].get('name', '?')
        return 'other'
    if k in ('MemberExpr', 'ArraySubscriptExpr'):
        return 'member'
    if k == 'CallExpr':
        return 'call'
    if k == 'ConditionalOperator':
        return '|'.join(sorted({pointer_value(c, decls) for c in n['inner'][1:]}))
    return 'other'


def walk(node, visit, loops=()):
    visit(node, loops)
    k = node.get('kind')
    inner_loops = loops + (node,) if k in ('ForStmt', 'WhileStmt', 'DoStmt') else loops
    for c in node.get('inner', []) or []:
        if isinstance(c, dict):
            walk(c, visit, inner_loops)


def callee_name(call):
    c = strip(call['inner'][0])
    if c.get('kind') == 'DeclRefExpr':
        return c.get('referencedDecl', {}).get('name')
    return None


def read_function(ast_text, function):
    fn = None
    for chunk in split_json(ast_text):
        if chunk.get('kind') == 'FunctionDecl' and chunk.get('name') == function and any(
                c.get('kind') == 'CompoundStmt' for c in chunk.get('inner', [])):
            fn = chunk
    return fn


def split_json(text):
    # -ast-dump-filter prints `Dumping NAME:` then one JSON object per decl.
    dec = json.JSONDecoder()
    i = 0
    while True:
        j = text.find('{', i)
        if j < 0:
            return
        try:
            obj, end = dec.raw_decode(text, j)
        except ValueError:
            i = j + 1
            continue
        yield obj
        i = end


def analyse(fn, source_lines):
    params = [c for c in fn.get('inner', []) if c.get('kind') == 'ParmVarDecl']
    formal_types = {p.get('name'): qual(p) for p in params}
    decls = {}

    def collect(node, loops):
        if node.get('kind') == 'VarDecl':
            automatic = node.get('storageClass') not in ('static', 'extern')
            decls[node.get('id')] = dict(name=node.get('name'), automatic=automatic, type=qual(node))
    walk(fn, collect)
    stores, calls = [], []

    def visit(node, loops):
        k = node.get('kind')
        if k in ('BinaryOperator', 'CompoundAssignOperator') and (k == 'CompoundAssignOperator' or node.get('opcode') == '='):
            lhs = strip(node['inner'][0])
            target = None
            if lhs.get('kind') == 'UnaryOperator' and lhs.get('opcode') == '*':
                b = strip(lhs['inner'][0])
                if b.get('kind') == 'DeclRefExpr' and b.get('referencedDecl', {}).get('kind') == 'ParmVarDecl':
                    target = b['referencedDecl']['name']
            elif lhs.get('kind') == 'ArraySubscriptExpr':
                b = strip(lhs['inner'][0])
                idx = strip(lhs['inner'][1])
                if (b.get('kind') == 'DeclRefExpr' and b.get('referencedDecl', {}).get('kind') == 'ParmVarDecl'
                        and idx.get('kind') == 'IntegerLiteral' and idx.get('value') == '0'):
                    target = b['referencedDecl']['name']
            if target:
                stores.append(dict(formal=target, type=formal_types.get(target, ''), offset=offset(node),
                                   value_type=qual(node['inner'][0]),
                                   loops=[offset(l) for l in loops]))
        if k == 'CallExpr':
            args = node.get('inner', [])[1:]
            calls.append(dict(callee=callee_name(node), offset=offset(node),
                              loops=[(offset(l), offset_end(l)) for l in loops],
                              actuals=[dict(type=qual(a), reach=classify(a, decls)) for a in args]))
    walk(fn, visit)
    return formal_types, stores, calls


def offset_end(node):
    r = node.get('range', {}).get('end', {})
    for k in ('expansionLoc', 'spellingLoc'):
        if k in r and 'offset' in r[k]:
            return r[k]['offset']
    return r.get('offset')


def pointer_slot(type_text):
    t = type_text.replace(' ', '')
    return t.endswith('**') or t.endswith('*const*')


def do_unit(item):
    (library, file), functions = item
    e = entry_for(library, file)
    if e is None:
        return [dict(library=library, file=file, function=f, error='no compile entry') for f in functions]
    pre = run(e, ['-E', '-P'])
    rows = []
    for f in functions:
        sig, body = definition(pre, f)
        # A definition the lexical reader does not find (an attribute or a
        # pasted name before the parenthesis) is read from the AST directly.
        # A function with a `T **` formal is read too: it may hand the slot
        # on to a callee's out position instead of storing through it.
        if (sig is not None and not stores_through(body, formals(sig))
                and not re.search(r'\*\s*\*\s*(const\s+)?\w+\s*[,)]', sig)):
            continue
        text = run(e, ['-fsyntax-only', '-Xclang', '-ast-dump=json', '-Xclang', '-ast-dump-filter=' + f])
        fn = read_function(text, f)
        if fn is None:
            rows.append(dict(library=library, file=file, function=f, error='no AST'))
            continue
        formal_types, stores, calls = analyse(fn, None)
        # Handing a pointer-to-pointer formal to a call is a slot write by
        # the callee (an `Out` position) or a read of it: listed as a store.
        for c in calls:
            for a in c['actuals']:
                name = a['reach'][len('formal:'):] if a['reach'].startswith('formal:') else None
                if name and pointer_slot(formal_types.get(name, '')):
                    stores.append(dict(formal=name, type=formal_types[name], offset=c['offset'],
                                       value_type='handed to ' + str(c['callee']), loops=[]))
        if not stores:
            continue
        first = min(s['offset'] for s in stores if s['offset'] is not None)
        loop_spans = {tuple(l) for c in calls for l in c['loops']}
        store_offsets = [s['offset'] for s in stores]

        def after(c):
            if c['offset'] is not None and c['offset'] > first:
                return True
            return any(a is not None and b is not None and any(a <= so <= b for so in store_offsets)
                       for a, b in c['loops'])
        later = [c for c in calls if after(c)]
        rows.append(dict(library=library, file=file, function=f, formals=formal_types,
                         stores=[dict(formal=s['formal'], type=s['type'], value_type=s['value_type'])
                                 for s in stores],
                         calls=[dict(callee=c['callee'], actuals=c['actuals']) for c in later]))
    return rows


report = json.load(open(report_path))
work = {}
for u in report['units']:
    safe = [f['function'] for f in u['functions']
            if f.get('status') == 'Safe' or f['function'] in ALSO]
    # Every validated summary any row reads (header-defined inline functions
    # have no row of their own) is read too: a withdrawn one moves its readers.
    for f in u['functions']:
        safe.extend((f.get('contract_dependencies') or {}).get('summaries') or [])
    if safe:
        work.setdefault((u['library'], u['file']), set()).update(safe)
work = {k: sorted(v) for k, v in work.items()}
with ThreadPoolExecutor(int(os.environ.get('CENSUS_JOBS', '6'))) as pool:
    results = [r for rows in pool.map(do_unit, sorted(work.items())) for r in rows]
json.dump(dict(route=route, report=report_path, safe_rows=sum(len(v) for v in work.values()),
               rows=results), open(out_path, 'w'), indent=1)
print(route, 'safe', sum(len(v) for v in work.values()), 'storing', len([r for r in results if 'error' not in r]),
      'errors', len([r for r in results if 'error' in r]))
