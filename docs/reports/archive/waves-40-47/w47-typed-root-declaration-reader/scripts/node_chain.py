"""Print the node chain (kind, castKind, opcode, isArrow, name, type) of each traced expression."""
import json, re, sys
seen = set()
for line in open(sys.argv[1], errors='replace'):
    if not line.startswith('W47TR '):
        continue
    m = re.match(r'W47TR hook=(\S+) fn=(\S+) line=(\d+) .*? node=Some\((".*")\)$', line.rstrip('\n'))
    if not m:
        continue
    hook, fn, ln, raw = m.groups()
    if (fn, hook) in seen:
        continue
    seen.add((fn, hook))
    try:
        node = json.loads(json.loads(raw))
    except Exception as error:
        print(fn, hook, 'truncated node', error)
        continue
    print(f'== {fn} [{hook}] line {ln}')

    def walk(n, depth):
        if not isinstance(n, dict) or not n.get('kind'):
            return
        t = n.get('type', {})
        ty = t.get('desugaredQualType') or t.get('qualType')
        extra = {k: n[k] for k in ('castKind', 'opcode', 'isArrow', 'name') if k in n}
        ref = n.get('referencedDecl', {}).get('name')
        print('  ' * depth + f"{n['kind']} {extra} type={ty!r}" + (f' ref={ref}' if ref else ''))
        for c in n.get('inner', []):
            walk(c, depth + 1)
    walk(node, 1)
