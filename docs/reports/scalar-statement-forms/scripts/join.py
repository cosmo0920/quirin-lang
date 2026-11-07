"""Join the classifier's own records to the run's leaves.

Usage: join.py ROWS.json PP_DIR CENSUS_DIR OUT.json

A census line is in the coordinates of the driver's own -E -P of the .i
file; it is mapped back to the .i (the run's coordinates) by the order of
non-blank lines, which re-preprocessing preserves (checked per unit).
"""
import collections as C, json, os, subprocess, sys

rows = json.load(open(sys.argv[1]))
pp, cen, out = sys.argv[2], sys.argv[3], sys.argv[4]
CODE = {
    'unsupported assignment expression': 'ASSIGN',
    'unrecognized assignment destination': 'DEST',
    'braced initializer, expression, or declaration is not modeled': 'BRACE',
    'unmodeled increment or decrement expression': 'INCDEC',
    'C operator `sizeof` requires type/operand analysis': 'SIZEOF',
}
statements = C.defaultdict(list)  # (unit, fn, line, form) -> [texts of final refusals]
problems = []
for name in sorted(os.listdir(pp)):
    if not name.endswith('.i'):
        continue
    unit = name[:-2].replace('__', '/')
    stream = os.path.join(cen, name[:-2] + '.jsonl')
    if not os.path.exists(stream):
        problems.append(('no stream', unit))
        continue
    a = open(os.path.join(pp, name), errors='replace').read().split('\n')
    b = subprocess.run(['/usr/lib/llvm-21/bin/clang', '-x', 'c', '-std=gnu11', '-E', '-P', '-'],
                       input='\n'.join(a), capture_output=True, text=True).stdout.split('\n')
    na = [i + 1 for i, l in enumerate(a) if l.strip()]
    nb = [i + 1 for i, l in enumerate(b) if l.strip()]
    if [a[i - 1] for i in na] != [b[i - 1] for i in nb]:
        problems.append(('line map differs', unit))
        continue
    back = dict(zip(nb, na))
    seen = C.defaultdict(lambda: [0, 0])  # key -> [classified, answered]
    for line in open(stream):
        r = json.loads(line)
        key = (r['function'], back.get(r['line'], -r['line']), tuple(r['texts']), r['description'])
        if r['stage'] == 'classified':
            seen[key][0] += 1
        elif r['answered']:
            seen[key][1] += 1
    for (fn, ln, texts, desc), (cl, ans) in seen.items():
        if cl > ans:
            statements[(unit, fn, ln, CODE[desc])].append(' '.join(texts))
joined = []
for r in rows:
    k = (r['unit'], r['fn'], r['line'], r['form'])
    joined.append(dict(r, statements=sorted(set(statements.get(k, [])))))
json.dump(joined, open(out, 'w'), indent=0)
print('problems', problems)
print('leaves', len(joined), 'matched', sum(1 for r in joined if r['statements']))
print(C.Counter((r['form'], len(r['statements'])) for r in joined))
