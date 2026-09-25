#!/usr/bin/env python3
"""The raw delta between two corpus diagnostics artifacts.

Usage: delta.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS [--json OUT]

Reports function and unit status moves, unit-field changes, the top-level
counts and every leaf removed or added (multiset difference per function over
`(category, description, line)`), each keyed by `(library, file, function)`.
Plain or gzip-compressed inputs.
"""

import collections
import gzip
import json
import pathlib
import sys


def load(path):
    data = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def functions(document):
    out = {}
    units = {}
    for unit in document["units"]:
        units[(unit["library"], unit["file"])] = unit
        for entry in unit["functions"]:
            key = (unit["library"], unit["file"], entry["function"])
            out[key] = entry
    return out, units


def leaves(entry):
    return collections.Counter(
        (d["category"], d["description"], d["line"]) for d in entry.get("diagnostics") or []
    )


def delta(before_path, after_path):
    before = load(before_path)
    after = load(after_path)
    bf, bu = functions(before)
    af, au = functions(after)
    moves = []
    removed = []
    added = []
    for key in sorted(set(bf) | set(af)):
        b = bf.get(key)
        a = af.get(key)
        if b is None or a is None:
            moves.append({"function": list(key), "before": b and b["status"], "after": a and a["status"]})
            continue
        if b["status"] != a["status"]:
            moves.append({"function": list(key), "before": b["status"], "after": a["status"]})
        lb, la = leaves(b), leaves(a)
        for leaf, count in (lb - la).items():
            removed.extend([{"function": list(key), "category": leaf[0], "description": leaf[1], "line": leaf[2]}] * count)
        for leaf, count in (la - lb).items():
            added.extend([{"function": list(key), "category": leaf[0], "description": leaf[1], "line": leaf[2]}] * count)
    unit_changes = []
    for key in sorted(set(bu) | set(au)):
        b = {k: v for k, v in (bu.get(key) or {}).items() if k != "functions"}
        a = {k: v for k, v in (au.get(key) or {}).items() if k != "functions"}
        if b != a:
            unit_changes.append({
                "unit": list(key),
                "fields": sorted(k for k in set(a) | set(b) if a.get(k) != b.get(k)),
                "status": [b.get("status"), a.get("status")],
            })
    top = {
        k: [before.get(k), after.get(k)]
        for k in sorted(set(before) | set(after))
        if k != "units" and before.get(k) != after.get(k)
    }
    return {
        "top_level_changes": top,
        "function_moves": moves,
        "unit_changes": unit_changes,
        "removed": removed,
        "added": added,
    }


def main():
    result = delta(sys.argv[1], sys.argv[2])
    print("top-level changes:", json.dumps(result["top_level_changes"]))
    print("function moves:", len(result["function_moves"]),
          dict(collections.Counter(f"{m['before']} -> {m['after']}" for m in result["function_moves"])))
    print("unit changes:", len(result["unit_changes"]),
          dict(collections.Counter(f"{u['status'][0]} -> {u['status'][1]}" for u in result["unit_changes"])))
    print("removed leaves:", len(result["removed"]))
    for (category, description), count in collections.Counter(
            (r["category"], r["description"] if "`" not in r["description"] else r["description"].split("`")[0] + "`_`")
            for r in result["removed"]).most_common():
        print(f"  -{count:5} {category}: {description}")
    print("added leaves:", len(result["added"]))
    for (category, description), count in collections.Counter(
            (r["category"], r["description"] if "`" not in r["description"] else r["description"].split("`")[0] + "`_`")
            for r in result["added"]).most_common():
        print(f"  +{count:5} {category}: {description}")
    if "--json" in sys.argv:
        out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1])
        out.write_text(json.dumps(result, indent=1) + "\n")


if __name__ == "__main__":
    main()
