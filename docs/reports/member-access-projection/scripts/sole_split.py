"""Split the single-premise sole-blocker functions by the custody shape of
their leaves (the callee's name class and the root kind), for the README."""
import collections, gzip, json, re, sys

IND = "indirect memory access is outside the resource projection"
rows = [json.loads(l) for l in gzip.open(sys.argv[1], "rt")]
RELEASE = re.compile(r"(^|_)(free|destroy|fclose|close|clear|unref|release)$|^rd_free$|^free$")
by = collections.defaultdict(list)
for r in rows:
    if set(r["families"]) <= {IND} and r["status"] == "Unknown":
        by[(r["file"], r["function"])].append(r)


def shape(leaf):
    paths = leaf["paths"]
    for p in paths:
        callee = p.get("callee")
        if leaf["premise"].startswith("read:pointer") and p["value"] in ("pointer",) and callee and RELEASE.search(callee):
            if any(s["op"] == "[]" for s in p["steps"]):
                return "element released"
            return f"member released ({p['root_kind']} parent)"
    if leaf["premise"].startswith("store:pointer"):
        return f"pointer stored into a member ({leaf['premise'][14:]})"
    if leaf["premise"] == "address:sub-object":
        callees = {p.get("callee") for p in paths if p["value"] == "address"}
        if any(c and RELEASE.search(c) for c in callees):
            return "sub-object address released/cleared"
        return "sub-object address at another position"
    if leaf["premise"] == "read:pointer-member":
        return "pointer member read at another position"
    return leaf["premise"]


table = collections.Counter()
for key, leaves in by.items():
    premises = {l["premise"] for l in leaves}
    if len(premises) != 1:
        table[("several premises", "")] += 1
        continue
    shapes = sorted({shape(l) for l in leaves})
    table[(next(iter(premises)), " + ".join(shapes))] += 1
for (premise, shapes), count in sorted(table.items(), key=lambda kv: -kv[1]):
    print(f"| `{premise}` | {shapes} | {count} |")
