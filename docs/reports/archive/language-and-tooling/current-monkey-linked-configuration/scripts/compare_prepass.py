#!/usr/bin/env python3
"""What the standalone Monkey units add to the run's member pre-pass.

usage: compare_prepass.py A_PREPASS B_PREPASS OUT_JSON

A_PREPASS and B_PREPASS are the probes' `QCORE_MEMBER_PREPASS_SECTION` dumps
(A: the compile database plus the standalone units, as at base; B: the
compile database alone, as under --monkey-linked). For the lifetime
protocols it reports each protocol whose standing differs, by its reasons
only in one run, and the per-unit summaries only in one run. Auxiliary.
"""
import json
import sys

a_path, b_path, out = sys.argv[1:4]
a = json.load(open(a_path))
b = json.load(open(b_path))
la, lb = a["lifetime_protocols"], b["lifetime_protocols"]


def key(protocol):
    return json.dumps({k: protocol[k] for k in ("host", "protocol", "guard")}, sort_keys=True)


pa = {key(p): p for p in la["protocols"]}
pb = {key(p): p for p in lb["protocols"]}
protocols = []
for k in sorted(set(pa) | set(pb)):
    x, y = pa.get(k), pb.get(k)
    if x == y:
        continue
    entry = {"protocol": json.loads(k)}
    if x is None or y is None:
        entry["only_in"] = "standalone" if y is None else "linked"
    else:
        sx, sy = x["standing"], y["standing"]
        kind_x, kind_y = next(iter(sx)), next(iter(sy))
        entry["standing"] = {"standalone": kind_x, "linked": kind_y}
        rx = set(sx[kind_x].get("reasons", [])) if isinstance(sx[kind_x], dict) else set()
        ry = set(sy[kind_y].get("reasons", [])) if isinstance(sy[kind_y], dict) else set()
        entry["reasons_only_standalone"] = sorted(rx - ry)
        entry["reasons_only_linked"] = sorted(ry - rx)
        entry["premise_equal"] = (
            isinstance(sx[kind_x], dict)
            and isinstance(sy[kind_y], dict)
            and sx[kind_x].get("premise") == sy[kind_y].get("premise")
        )
    protocols.append(entry)
summaries = {
    "only_standalone": sorted(set(la["summaries"]) - set(lb["summaries"])),
    "only_linked": sorted(set(lb["summaries"]) - set(la["summaries"])),
    "differing": sorted(k for k in set(la["summaries"]) & set(lb["summaries"]) if la["summaries"][k] != lb["summaries"][k]),
}
other = {k: "equal" if la.get(k) == lb.get(k) else "differs" for k in la if k not in ("protocols", "summaries")}
result = {
    "units": {"standalone": a.get("units"), "linked": b.get("units")},
    "read": {"standalone": a.get("read"), "linked": b.get("read")},
    "protocols_differing": protocols,
    "summaries": {k: len(v) for k, v in summaries.items()},
    "summary_keys": summaries,
    "other_lifetime_keys": other,
    "other_section_keys_differing": sorted(k for k in set(a) | set(b) if k not in ("units", "read", "lifetime_protocols") and a.get(k) != b.get(k)),
}
json.dump(result, open(out, "w"), indent=1, sort_keys=True)
print(json.dumps({k: v for k, v in result.items() if k != "summary_keys"}, indent=1)[:4000])
