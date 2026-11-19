"""AG2-UNRESOLVED-ALIAS sub-shape census (lexical) over the user's `current` report.

Reads the report rows (pickled by extract.py) and the prepared Fluent Bit tree
(read-only). For every `unknown_alias` leaf of an Unknown row it derives:

* name class: literal / synthetic / place / formal-pointer / formal-scalar /
  local-pointer / local-scalar / local-array / enum-or-macro-constant / global;
* statement shape at the leaf's origin line;
* cause: primary (unresolved on its own) or secondary (a consequence of
  another refusal of the same row: a refused statement mentioning the name at
  or before the leaf, or an assignment of the name from an uncontracted
  callee's result before the leaf).

Lexical: declarations and parameters are read by regular expression from the
original source text, so every class is an approximation and is labelled so.
"""
import collections, json, pickle, re, sys

# usage: census.py ROWS.pkl PREPARED_TREE/ LEAVES.json
SRC = sys.argv[2].rstrip('/') + '/'
rows = pickle.load(open(sys.argv[1], 'rb'))

_files = {}


def lines_of(path):
    if path not in _files:
        try:
            _files[path] = open(SRC + path, errors='replace').read().split('\n')
        except OSError:
            _files[path] = None
    return _files[path]


def origin(d):
    o = d.get('origin') or ''
    if ':' not in o:
        return None, None
    p, l = o.rsplit(':', 1)
    try:
        return p, int(l)
    except ValueError:
        return None, None


def alias_name(d):
    m = re.search(r'`([^`]*)`', d['description'])
    return m.group(1) if m else '?'


def find_definition(text, fn, before):
    """Line index (0-based) of `fn(` definition header at or before `before`."""
    pat = re.compile(r'(^|[^\w.>])' + re.escape(fn) + r'\s*\(')
    for i in range(min(before, len(text) - 1), -1, -1):
        if pat.search(text[i]):
            # crude: skip calls (a definition header line does not end with ';')
            j = i
            seg = ''
            while j < len(text) and j < i + 15:
                seg += text[j] + '\n'
                if '{' in text[j] or ';' in text[j]:
                    break
                j += 1
            if '{' in seg and not re.search(r'\)\s*;', seg.split('{')[0]):
                return i
    return None


def params_of(text, start):
    seg = '\n'.join(text[start:start + 20])
    k = seg.find('(')
    depth = 0
    out = ''
    for ch in seg[k:]:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
            if depth == 0:
                break
        out += ch
    out = out[1:]
    params = {}
    for part in out.split(','):
        part = part.strip()
        m = re.search(r'(\w+)\s*(\[[^\]]*\])?\s*$', part)
        if not m:
            continue
        name = m.group(1)
        kind = 'pointer' if ('*' in part or m.group(2)) else 'scalar'
        if re.search(r'\(\s*\*', part):
            kind = 'function-pointer'
        params[name] = (kind, part)
    return params


DECL_TYPES = r'(?:const\s+|volatile\s+|static\s+|unsigned\s+|signed\s+|struct\s+\w+\s*|enum\s+\w+\s*|union\s+\w+\s*|[A-Za-z_]\w*\s+)'


def local_kind(body, name):
    """The declared kind of `name` in a body, from declaration lines: the name
    must appear in a declarator (before that declarator's `=`), never in an
    initializer."""
    n = re.compile(r'\b' + re.escape(name) + r'\b')
    for ln in body:
        s = ln.strip().rstrip(';')
        if re.match(r'(return|if|while|for|switch|else|case|goto|do)\b', s):
            continue
        m = re.match(r'^((?:' + DECL_TYPES + r')+)(.*)$', s)
        if not m or '(' in m.group(1):
            continue
        rest = m.group(2)
        depth = 0
        seg = ''
        segs = []
        for ch in rest:
            if ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1
            if ch == ',' and depth == 0:
                segs.append(seg)
                seg = ''
            else:
                seg += ch
        segs.append(seg)
        for seg in segs:
            declarator = seg.split('=')[0]
            if not n.search(declarator):
                continue
            if re.search(r'\b' + re.escape(name) + r'\s*\[', declarator):
                return 'array', s
            if '*' in declarator:
                return 'pointer', s
            if re.fullmatch(r'\s*\(?\s*' + re.escape(name) + r'\s*\)?\s*', declarator):
                return 'scalar', s
    return None, None


def shape_at(line, name):
    if line is None:
        return 'no-source'
    n = r'\b' + re.escape(name) + r'\b'
    s = line.strip()
    if not re.search(n, s) and name != '__qcore_literal__':
        return 'name-not-on-line'
    if name == '__qcore_literal__':
        if re.search(r'=\s*"', s):
            return 'copy-of-literal'
        if re.search(r'\w+\s*\(.*"', s):
            return 'literal-actual'
        return 'literal-other'
    if re.match(r'return\b', s):
        return 'return'
    if re.match(r'(if|while|for|switch|do)\b', s) or re.match(r'\}?\s*while\b', s):
        return 'condition'
    if re.search(r'=\s*(\([^()]*\)\s*)?' + n + r'\s*;', s):
        return 'copy-source'
    if re.search(r'^\**\s*' + n + r'\s*=[^=]', s) or re.search(r'[\*\s]' + n + r'\s*=[^=]', s):
        return 'assigned'
    if re.search(n + r'\s*(->|\.|\[)', s):
        if re.search(r'\w+\s*\(', s):
            return 'member-in-call'
        return 'member-access'
    if re.search(r'\w+\s*\([^;]*' + n, s):
        return 'call-actual'
    return 'other'


def classify_name(name, params, body):
    if name == '__qcore_literal__':
        return 'literal'
    if name.startswith('__qcore_') or name in ('__tmpstr', '_vp', '__ctx'):
        return 'synthetic:' + re.sub(r'\d+', 'N', name)
    if not re.fullmatch(r'[A-Za-z_]\w*', name):
        return 'place'
    if name in params:
        return 'formal-' + params[name][0]
    k, _ = local_kind(body, name)
    if k:
        return 'local-' + k
    if re.fullmatch(r'[A-Z][A-Z0-9_]*', name) or re.fullmatch(r'[A-Z0-9]+_\w+', name):
        return 'constant'
    return 'global-or-undeclared'


out = []
unit_ids = {}
for u, f in rows:
    uid = unit_ids.setdefault(id(u), len(unit_ids))
    if f['status'] != 'Unknown':
        continue
    ds = f['diagnostics']
    leaves = [d for d in ds if d['category'] == 'unknown_alias']
    if not leaves:
        continue
    others = [d for d in ds if d['category'] != 'unknown_alias']
    fn = f['function']
    paths = [origin(d)[0] for d in leaves if origin(d)[0]]
    path = collections.Counter(paths).most_common(1)[0][0] if paths else None
    text = lines_of(path) if path else None
    first = min(origin(d)[1] for d in leaves if origin(d)[0] == path) if path else None
    params, body, start = {}, [], None
    if text:
        start = find_definition(text, fn, first - 1)
        if start is not None:
            params = params_of(text, start)
            last = max([origin(d)[1] for d in ds if origin(d)[0] == path] + [first])
            body = text[start:last + 1]
    uncontracted = {re.sub(r'^.*`([^`]*)`.*$', r'\1', d['description']) for d in others if d['category'] == 'unsupported' and 'uncontracted' in d['description']}
    # uncontracted callee names as reported (category is 'unsupported'? check both)
    for d in others:
        m = re.match(r'call to (?:the )?uncontracted (?:function )?`([^`]+)`', d['description'])
        if m:
            uncontracted.add(m.group(1))
    for d in leaves:
        name = alias_name(d)
        p, l = origin(d)
        line = text[l - 1] if (text and p == path and l and l <= len(text)) else None
        cls = classify_name(name, params, body)
        shp = shape_at(line, name)
        # secondary causes
        cause = 'primary'
        why = ''
        base = re.split(r'[/\-\[.]', name)[0] if cls == 'place' else name
        nre = re.compile(r'\b' + re.escape(base) + r'\b')
        for o in others:
            op, ol = origin(o)
            if op != p or ol is None or l is None or ol > l:
                continue
            otext = text[ol - 1] if (text and ol <= len(text)) else ''
            if ol == l:
                cause, why = 'secondary', 'same-line-refusal'
                break
            if nre.search(otext):
                cause, why = 'secondary', 'earlier-refusal-mentions-name'
                break
        if cause == 'primary' and cls.startswith('formal') and text and start is not None:
            body_text = '\n'.join(text[start:(l or start) + 1])
            for callee in uncontracted:
                if re.search(r'\b' + re.escape(callee) + r'\s*\([^;]*\b' + re.escape(base) + r'\b', body_text):
                    cause, why = 'secondary', 'formal-handed-to-uncontracted-callee'
                    break
        if cause == 'primary' and text and start is not None and l:
            for i in range(start, l - 1):
                s = text[i]
                m = re.search(r'\b' + re.escape(base) + r'\s*=\s*(?:\([^()]*\)\s*)?(\w+)\s*\(', s)
                if m and m.group(1) not in ('sizeof',):
                    callee = m.group(1)
                    if any(callee == c or callee in c for c in uncontracted) or any(callee in (o['description']) for o in others):
                        cause, why = 'secondary', 'assigned-from-refused-call'
                        break
        out.append(dict(unit=uid, lib=u['library'], file=u['file'], fn=fn, name=name, cls=cls, shape=shp,
                        cause=cause, why=why, origin=d.get('origin'), line=(line or '').strip()[:140],
                        nother=len(others)))

json.dump(out, open(sys.argv[3], 'w'))
print('leaves', len(out))
