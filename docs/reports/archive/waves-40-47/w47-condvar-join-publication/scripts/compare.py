#!/usr/bin/env python3
"""compare.py BEFORE AFTER [OUT.json]: every function whose status or leaf
multiset differs between two runs (resource-diagnostics.json[.gz] or a
directory holding one), with the leaves added and removed, plus unit and
function status tallies and diagnostic counts for both."""
import collections
import gzip
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        gz = path / "resource-diagnostics.json.gz"
        path = gz if gz.exists() else path / "resource-diagnostics.json"
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def index(data):
    functions, units = {}, {}
    for unit in data["units"]:
        units[unit["file"]] = unit.get("status")
        for function in unit.get("functions", []):
            key = (unit["file"], function["function"])
            leaves = collections.Counter(
                (d.get("category"), d.get("description"), d.get("line"))
                for d in function.get("diagnostics", []))
            functions[key] = (function.get("status"), leaves)
    return functions, units


def tally(functions, units, data):
    unique = {}
    for (file, name), (status, _) in functions.items():
        unique.setdefault(name, set()).add(status)
    return {
        "units": dict(collections.Counter(units.values())),
        "function_rows": dict(collections.Counter(s for s, _ in functions.values())),
        "diagnostic_counts": data.get("diagnostic_counts"),
    }


def main():
    before_data, after_data = load(sys.argv[1]), load(sys.argv[2])
    before, before_units = index(before_data)
    after, after_units = index(after_data)
    moved = []
    for key in sorted(set(before) | set(after)):
        b = before.get(key, (None, collections.Counter()))
        a = after.get(key, (None, collections.Counter()))
        if b == a:
            continue
        added = a[1] - b[1]
        removed = b[1] - a[1]
        moved.append({
            "file": key[0], "function": key[1],
            "status": [b[0], a[0]],
            "added": sorted([list(x) + [n] for x, n in added.items()], key=str),
            "removed": sorted([list(x) + [n] for x, n in removed.items()], key=str),
        })
    unit_moves = {f: [before_units.get(f), after_units.get(f)]
                  for f in sorted(set(before_units) | set(after_units))
                  if before_units.get(f) != after_units.get(f)}
    report = {
        "before": tally(before, before_units, before_data),
        "after": tally(after, after_units, after_data),
        "unit_status_moves": unit_moves,
        "status_moves": [m for m in moved if m["status"][0] != m["status"][1]],
        "leaf_only_moves": [m for m in moved if m["status"][0] == m["status"][1]],
    }
    text = json.dumps(report, indent=1) + "\n"
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text)
    print(json.dumps({k: (len(v) if isinstance(v, (list, dict)) and k.endswith("moves") else v)
                      for k, v in report.items()}, indent=1))


if __name__ == "__main__":
    main()
