"""Every K2 leaf at 11-24 (all rows) with its source text and its row's other blockers."""
import json, sys, collections, pickle, importlib.util, gzip, re
rec, src_root, out = sys.argv[1:4]
r = json.load(open(rec))
cache = {}


def text_of(origin, span=1):
    path, ln = origin.rsplit(":", 1)
    if path not in cache:
        try:
            cache[path] = open(f"{src_root}/{path}", errors="replace").read().splitlines()
        except FileNotFoundError:
            cache[path] = []
    lines = cache[path]
    ln = int(ln)
    return " ".join(l.strip() for l in lines[ln - 1:ln - 1 + span]) if 0 < ln <= len(lines) else "<missing>"


# Re-derive the leaves from the frozen-style rows: recensus.py kept only frozen/freed rows,
# so this script is given the full per-row dump.
rows = json.load(open(sys.argv[4]))
outrows = []
for row in rows:
    nonk2 = collections.Counter(b for b in row["others"])
    for fam, sub, origin, desc in row["k2"]:
        outrows.append({"function": row["function"], "unit": row["unit"], "status": row["status"], "fam": fam,
                        "sub": sub, "origin": origin, "text": text_of(origin, 2), "others": dict(nonk2)})
json.dump(outrows, open(out, "w"), indent=1)
for x in sorted(outrows, key=lambda x: (x["fam"], x["text"])):
    print(x["status"][0], len(x["others"]), x["fam"], x["sub"] or "", x["origin"], "|", x["text"][:150])
