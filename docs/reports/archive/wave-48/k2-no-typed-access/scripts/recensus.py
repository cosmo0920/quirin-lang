"""K2 re-census: carry the 11-07 census tags onto the 11-24 report's leaves.

usage: recensus.py CENSUS_ARTIFACTS R1124_PICKLE ACCESS_CENSUS_PY K2_DRAFT OUT_JSON
"""
import collections, gzip, json, pickle, sys, importlib.util, re

art, pkl, acpy, draft, out, offs = sys.argv[1:7]
OFF = {k: v for k, v in json.load(open(offs)).items() if k.startswith("lib/monkey/")}
spec = importlib.util.spec_from_file_location("ac", acpy)
AC = importlib.util.module_from_spec(spec)
spec.loader.exec_module(AC)
UNC = re.compile(r"call to uncontracted function `([^`]*)`")

leaf_tags = {}
with gzip.open(f"{art}/shape-leaves.jsonl.gz", "rt") as h:
    h.readline()
    for line in h:
        r = json.loads(line)
        leaf_tags[(r["file"], r["function"], r["family"], r["sub"], r["line"])] = frozenset(r["tags"])

old_rows = []
with gzip.open(f"{art}/shape-functions.jsonl.gz", "rt") as h:
    for line in h:
        old_rows.append(json.loads(line))

frozen = []
text = open(draft).read()
block = text.split("## Frozen candidate list")[1].split("```text")[1].split("```")[0]
for l in block.strip().splitlines():
    lib, f, fn = l.split("::")
    frozen.append((lib, f, fn))

units = pickle.load(open(pkl, "rb"))
defined = set()
for d in units:
    for f in d["functions"]:
        defined.add(f["function"])

K2 = frozenset({"none"})
stats = collections.Counter()
rows = []
for ui, d in enumerate(units):
    for f in d["functions"]:
        items = []
        for cat, desc, line, origin in f["diagnostics"]:
            if cat == "unknown_alias":
                items.append(("ALIAS", None, line, origin, desc))
                continue
            if cat != "unsupported":
                items.append((cat.upper(), None, line, origin, desc))
                continue
            fam, sub = AC.family(desc)
            if fam:
                items.append((fam, sub, line, origin, desc))
                continue
            m = UNC.match(desc)
            if m:
                items.append(("UNC-RUN" if m.group(1) in defined else "UNC-EXT", m.group(1), line, origin, desc))
            else:
                items.append(("OTHER", None, line, origin, desc))
        tagged = []
        for fam, sub, line, origin, desc in items:
            if fam in ("IND", "COND", "RET"):
                t = leaf_tags.get((d["file"], f["function"], fam, sub, line - OFF.get(d["file"], 0)))
                if t is None:
                    stats["access-leaf-untagged"] += 1
                    t = frozenset({"new-or-moved"})
                else:
                    stats["access-leaf-tagged"] += 1
                tagged.append((fam, sub, line, origin, desc, t))
            else:
                tagged.append((fam, sub, line, origin, desc, None))
        rows.append({"ui": ui, "file": d["file"], "library": d["library"], "function": f["function"],
                     "status": f["status"], "items": tagged})

unknown = [r for r in rows if r["status"] == "Unknown"]
k2_leaves = [(r, it) for r in rows for it in r["items"] if it[5] == K2]
k2_unknown_leaves = [(r, it) for r in unknown for it in r["items"] if it[5] == K2]
touched = {id(r) for r, _ in k2_unknown_leaves}
freed = [r for r in unknown if r["items"] and all(it[5] == K2 for it in r["items"])]
fam_counts = collections.Counter((it[0], it[1]) for _, it in k2_unknown_leaves)

# 11-07 figures, recomputed from the artifact for the same definitions.
old_unknown = [r for r in old_rows if r["status"] == "Unknown"]
old_k2_leaves = sum(1 for r in old_rows for fam, tags, _ in r["items"] if fam in ("IND", "COND", "RET") and tags == ["none"])
old_touched = sum(1 for r in old_unknown if any(fam in ("IND", "COND", "RET") and tags == ["none"] for fam, tags, _ in r["items"]))
old_freed = [r for r in old_unknown if r["items"] and all(fam in ("IND", "COND", "RET") and tags == ["none"] for fam, tags, _ in r["items"])]

# Frozen list status at 11-24.
by_fn = collections.defaultdict(list)
for r in rows:
    by_fn[(r["file"], r["function"])].append(r)
frozen_out = []
for lib, file, fn in frozen:
    rs = by_fn.get((file, fn), [])
    entry = {"function": f"{lib}::{file}::{fn}", "rows": []}
    for r in rs:
        blockers = collections.Counter()
        for fam, sub, line, origin, desc, t in r["items"]:
            if t is not None:
                blockers[f"{fam}:{'+'.join(sorted(t))}"] += 1
            elif fam in ("UNC-RUN", "UNC-EXT"):
                blockers[f"{fam}:{sub}"] += 1
            else:
                blockers[f"{fam}"] += 1
        entry["rows"].append({"unit": r["ui"], "status": r["status"], "blockers": dict(blockers),
                              "k2_sites": [[fam, sub, origin, desc] for fam, sub, line, origin, desc, t in r["items"] if t == K2],
                              "other": [[fam, sub, origin, desc] for fam, sub, line, origin, desc, t in r["items"] if t != K2]})
    frozen_out.append(entry)

result = {
    "match": dict(stats),
    "old": {"unknown_rows": len(old_unknown), "k2_leaves": old_k2_leaves, "touched_unknown_rows": old_touched,
            "freed_alone_rows": len(old_freed),
            "freed_alone_distinct": len({(r["file"], r["function"]) for r in old_freed})},
    "new": {"rows": len(rows), "unknown_rows": len(unknown), "k2_leaves": len(k2_leaves),
            "k2_leaves_on_unknown": len(k2_unknown_leaves), "touched_unknown_rows": len(touched),
            "freed_alone_rows": len(freed),
            "freed_alone_distinct": len({(r["file"], r["function"]) for r in freed}),
            "leaves_by_family": {f"{a}:{b}": c for (a, b), c in fam_counts.most_common()}},
    "freed_alone": [{"function": f"{r['library']}::{r['file']}::{r['function']}", "unit": r["ui"],
                     "sites": [[fam, sub, origin, desc] for fam, sub, line, origin, desc, t in r["items"]]} for r in freed],
    "frozen": frozen_out,
}
allk2 = []
for r_ in rows:
    k = [[fam, sub, origin, desc] for fam, sub, line, origin, desc, t in r_["items"] if t == K2]
    if k:
        o = []
        for fam, sub, line, origin, desc, t in r_["items"]:
            if t == K2:
                continue
            o.append(f"{fam}:{'+'.join(sorted(t))}" if t is not None else (f"{fam}:{sub}" if fam.startswith("UNC") else fam))
        allk2.append({"function": f"{r_['library']}::{r_['file']}::{r_['function']}", "unit": r_["ui"], "status": r_["status"], "k2": k, "others": o})
json.dump(allk2, open(out.replace(".json", "-rows.json"), "w"), indent=1)
json.dump(result, open(out, "w"), indent=1)
print(json.dumps({k: result[k] for k in ("match", "old", "new")}, indent=1))
