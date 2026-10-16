#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: compare.py ROUTE BEFORE AFTER
BEFORE_NULL AFTER_NULL OUT.json

Adapted from W47-LOOP-OWNER-CONTRACTS' compare.py: every function row whose
status or leaf multiset differs between two runs (`resource-diagnostics.json`
or a directory holding one), with the leaves added and removed; unit and row
status tallies; and the null property's verdict tallies with every row whose
null verdict moved. Read-only."""
import collections
import gzip
import hashlib
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
        return json.load(handle), hashlib.sha256(path.read_bytes()).hexdigest()


def index(data):
    # A file compiled in two configurations (the static and shared variants)
    # is two units: each row is keyed by the file, its configuration's
    # ordinal among that file's units, and the function.
    functions, units = {}, {}
    seen = collections.Counter()
    for unit in data["units"]:
        ordinal = seen[unit["file"]]
        seen[unit["file"]] += 1
        name = unit["file"] if ordinal == 0 else f"{unit['file']}#{ordinal}"
        units[name] = unit.get("status")
        for function in unit.get("functions", []):
            key = (name, function["function"])
            leaves = collections.Counter(
                (d.get("category"), d.get("description"), d.get("line"))
                for d in function.get("diagnostics", []))
            functions[key] = (function.get("status"), leaves)
    return functions, units


def null_rows(path):
    rows = {}
    with open(path) as f:
        for line in f:
            n = json.loads(line)
            verdict = n["null"]["verdict"]
            if verdict == "Safe":
                verdict = "Safe-unconditional" if n["null"].get("scope") == "unconditional" else "Safe-conditional"
            key = (n["file"], n["function"])
            rows.setdefault(key, collections.Counter())[verdict] += 1
    return rows


def null_tally(rows):
    total = collections.Counter()
    for counter in rows.values():
        total.update(counter)
    return dict(sorted(total.items()))


def main():
    route, before_path, after_path, before_null, after_null, out = sys.argv[1:7]
    (before_data, before_sha), (after_data, after_sha) = load(before_path), load(after_path)
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
            "file": key[0], "function": key[1], "status": [b[0], a[0]],
            "added": sorted([list(x) + [n] for x, n in added.items()], key=str),
            "removed": sorted([list(x) + [n] for x, n in removed.items()], key=str),
        })
    nb, na = null_rows(before_null), null_rows(after_null)
    null_moves = [
        {"file": k[0], "function": k[1], "before": dict(nb.get(k, {})), "after": dict(na.get(k, {}))}
        for k in sorted(set(nb) | set(na)) if nb.get(k) != na.get(k)
    ]

    def statuses(functions):
        return dict(sorted(collections.Counter(s for s, _ in functions.values()).items()))

    report = {
        "route": route,
        "before": {"report_sha256": before_sha, "rows": statuses(before), "null": null_tally(nb)},
        "after": {"report_sha256": after_sha, "rows": statuses(after), "null": null_tally(na)},
        "unit_status_moves": {f: [before_units.get(f), after_units.get(f)]
                              for f in sorted(set(before_units) | set(after_units))
                              if before_units.get(f) != after_units.get(f)},
        "status_moves": [m for m in moved if m["status"][0] != m["status"][1]],
        "leaf_only_moves": [m for m in moved if m["status"][0] == m["status"][1]],
        "null_moves": null_moves,
    }
    pathlib.Path(out).write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({
        "route": route,
        "before": report["before"], "after": report["after"],
        "status_moves": [(m["file"], m["function"], m["status"]) for m in report["status_moves"]],
        "leaf_only_moves": len(report["leaf_only_moves"]),
        "null_moves": len(null_moves),
    }, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
