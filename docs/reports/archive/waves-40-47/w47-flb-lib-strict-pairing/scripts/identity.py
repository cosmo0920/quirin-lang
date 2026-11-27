#!/usr/bin/env python3
"""identity.py PHASE_DIR SLICE_DIR BASELINE_DIR -- compare one phase's three
arbiter runs (card W47-FLB-LIB-LIFECYCLE) with the archived baseline
(docs/reports/w47-loop-owner-instance-flow/integration/artifacts/): the
SHA-256 of each resource-diagnostics.json, the function and unit status
tallies, and every (unit, function) whose status or leaf multiset differs.
Read-only; prints JSON.
"""
import gzip
import hashlib
import json
import pathlib
import sys
from collections import Counter


def raw(path):
    path = pathlib.Path(path)
    if path.is_dir():
        gz = path / "resource-diagnostics.json.gz"
        path = gz if gz.exists() else path / "resource-diagnostics.json"
    data = (gzip.open if path.suffix == ".gz" else open)(path, "rb").read()
    return data


def rows(data):
    doc = json.loads(data)
    status, units, leaves = {}, {}, {}
    for unit in doc["units"]:
        units[unit["file"]] = unit.get("status")
        for function in unit.get("functions", []):
            key = (unit["file"], function["function"])
            status[key] = function.get("status")
            leaves[key] = Counter(
                (d.get("category"), d.get("description"), d.get("line"))
                for d in function.get("diagnostics", [])
            )
    return status, units, leaves, doc.get("diagnostic_counts")


def compare(run, base):
    a, b = raw(run), raw(base)
    sa, ua, la, ca = rows(a)
    sb, ub, lb, cb = rows(b)
    moved = []
    for key in sorted(set(sa) | set(sb)):
        if sa.get(key) != sb.get(key) or la.get(key) != lb.get(key):
            gone = (lb.get(key, Counter()) - la.get(key, Counter()))
            new = (la.get(key, Counter()) - lb.get(key, Counter()))
            moved.append({
                "unit": key[0], "function": key[1],
                "status": [sb.get(key), sa.get(key)],
                "leaves_removed": [list(x) for x in sorted(gone.elements(), key=str)],
                "leaves_added": [list(x) for x in sorted(new.elements(), key=str)],
            })
    units = [u for u in sorted(set(ua) | set(ub)) if ua.get(u) != ub.get(u)]
    return {
        "sha256": [hashlib.sha256(b).hexdigest(), hashlib.sha256(a).hexdigest()],
        "identical": a == b,
        "functions": [dict(Counter(sb.values())), dict(Counter(sa.values()))],
        "units": [dict(Counter(ub.values())), dict(Counter(ua.values()))],
        "diagnostic_counts": [cb, ca],
        "moved_functions": moved,
        "moved_units": units,
    }


def main():
    phase, slice_dir, baseline = map(pathlib.Path, sys.argv[1:4])
    out = {
        "corpus": compare(phase / "corpus", baseline / "corpus"),
        "system": compare(phase / "system", baseline / "system"),
        "slice": compare(slice_dir, baseline / "slice"),
    }
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()
