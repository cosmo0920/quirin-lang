import pickle, sys, json, gzip, collections, importlib.util
art, pkl, acpy = sys.argv[1:4]
spec = importlib.util.spec_from_file_location("ac", acpy)
AC = importlib.util.module_from_spec(spec)
spec.loader.exec_module(AC)
old = collections.defaultdict(set)
with gzip.open(f"{art}/shape-leaves.jsonl.gz", "rt") as h:
    h.readline()
    for line in h:
        r = json.loads(line)
        old[r["file"]].add((r["function"], r["family"], r["sub"], r["line"]))
units = pickle.load(open(pkl, "rb"))
new = collections.defaultdict(set)
for d in units:
    for f in d["functions"]:
        for cat, desc, line, origin in f["diagnostics"]:
            if cat != "unsupported":
                continue
            fam, sub = AC.family(desc)
            if fam:
                new[d["file"]].add((f["function"], fam, sub, line))
res = {}
for file in sorted(new):
    if file not in old:
        continue
    o = old[file]
    best = collections.Counter()
    olines = collections.defaultdict(set)
    for fn, fam, sub, l in o:
        olines[(fn, fam, sub)].add(l)
    for fn, fam, sub, l in new[file]:
        for ol in olines.get((fn, fam, sub), ()):
            best[l - ol] += 1
    if not best:
        continue
    off, hits = best.most_common(1)[0]
    exact = sum(1 for x in new[file] if x in o)
    if off != 0 or exact < len(new[file]):
        res[file] = (off, hits, exact, len(new[file]), len(o))
for k, v in res.items():
    print(k, "offset", v[0], "hits", v[1], "exact", v[2], "new", v[3], "old", v[4])
json.dump({k: v[0] for k, v in res.items()}, open(sys.argv[4], "w"))
