"""Attribute every leaf delta of compare.py's output to rule 63.

exchange : a generic condition leaf removed and a named one added at the same
           (file, function, line)  -- rule 63 (2)
admission: a generic condition leaf removed with no leaf added at its line  -- rule 63 (1)
other    : everything else, listed in full
"""
import json, sys
from collections import Counter, defaultdict

c = json.load(open(sys.argv[1]))
PREFIX = "unmodeled call, aggregate or indirect access in "
removed = defaultdict(list)
added = defaultdict(list)
for f, fn, cat, desc, line, n in c["removed"]:
    removed[(f, fn, line)] += [(cat, desc)] * n
for f, fn, cat, desc, line, n in c["added"]:
    added[(f, fn, line)] += [(cat, desc)] * n
kinds = Counter()
others = []
for key in sorted(set(removed) | set(added), key=str):
    r, a = list(removed.get(key, [])), list(added.get(key, []))
    named = [x for x in a if x[0] == "unsupported" and x[1].startswith("call to uncontracted function") and " in " in x[1]]
    generic = [x for x in r if x[0] == "unsupported" and x[1].startswith(PREFIX)]
    while named and generic:
        n, g = named.pop(), generic.pop()
        a.remove(n); r.remove(g)
        kinds["exchange " + g[1][len(PREFIX):]] += 1
    for g in [x for x in r if x[0] == "unsupported" and x[1].startswith(PREFIX)]:
        if not a:
            r.remove(g)
            kinds["admission " + g[1][len(PREFIX):]] += 1
    if r or a:
        others.append({"key": key, "removed": r, "added": a})
for k, n in sorted(kinds.items()):
    print(f"{n:6d} {k}")
print("other deltas:", len(others))
for o in others:
    print(json.dumps(o))
exchanged_rows = set()
