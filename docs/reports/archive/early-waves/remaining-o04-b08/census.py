"""O04-B08 corpus census.

Read-only. Compares two produced `resource-diagnostics.json` documents and
reports every unit and function status movement, plus the per-unit diagnostic
delta. Reproduce with:

    python3 -B docs/reports/remaining-o04-b08/census.py \
        <before>/resource-diagnostics.json <after>/resource-diagnostics.json
"""

import collections
import json
import pathlib
import sys

b = json.loads(pathlib.Path(sys.argv[1]).read_text())
a = json.loads(pathlib.Path(sys.argv[2]).read_text())


def units(doc):
    return {(u["library"], u["file"]): u for u in doc["units"]}


def funcs(doc):
    return {
        (u["library"], u["file"], f["function"]): f
        for u in doc["units"]
        for f in u.get("functions", [])
    }


bu, au = units(b), units(a)
bf, af = funcs(b), funcs(a)
print("corpus diagnostic_counts before:", b["diagnostic_counts"])
print("corpus diagnostic_counts after: ", a["diagnostic_counts"])
print("units:", len(bu), len(au), " projected functions:", len(bf), len(af))
print("unit status before:", dict(collections.Counter(u["status"] for u in bu.values())))
print("unit status after: ", dict(collections.Counter(u["status"] for u in au.values())))
moves = [k for k in bu if bu[k]["status"] != au[k]["status"]]
print("unit status moves:", len(moves), moves)
print(
    "unit reason_code moves:",
    len([k for k in bu if bu[k].get("reason_code") != au[k].get("reason_code")]),
)
fmoves = [k for k in bf if bf[k]["status"] != af.get(k, {}).get("status")]
print("function status moves:", len(fmoves))
for k in fmoves:
    print("   MOVED", k, bf[k]["status"], "->", af.get(k, {}).get("status"))
print("functions added:", sorted(set(af) - set(bf))[:20])
print("functions removed:", sorted(set(bf) - set(af))[:20])
print()
print("per-unit diagnostic deltas:")
total = collections.Counter()
for k in sorted(bu):
    kinds = set(bu[k]["diagnostic_counts"]) | set(au[k]["diagnostic_counts"])
    delta = {
        kind: au[k]["diagnostic_counts"].get(kind, 0) - bu[k]["diagnostic_counts"].get(kind, 0)
        for kind in kinds
    }
    delta = {kk: vv for kk, vv in delta.items() if vv}
    if delta:
        total.update(delta)
        print(f"   {k[0]}/{k[1]}: {delta}")
print("sum of per-unit deltas:", dict(total))
