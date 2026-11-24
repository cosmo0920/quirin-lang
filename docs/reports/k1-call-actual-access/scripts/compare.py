#!/usr/bin/env python3
"""compare.py BEFORE.json AFTER.json [REFERENCE.json] OUT.json -- K1 (AG1's compare.py): two
`current` reports of the same units, compared row by row.

Rows are keyed (unit file, function, occurrence). Writes the status totals of
each side, every status move, and every leaf added or removed (by category
and description, with the access refusal counted apart). With REFERENCE (the
user's full run), also checks that BEFORE reproduces REFERENCE's rows for the
same units (status and leaf multiset)."""
import collections
import json
import sys

GATE = "indirect memory access is outside the resource projection"


def rows(path):
    data = json.load(open(path))
    out = {}
    for unit in data["units"]:
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            key = (unit["file"], function["function"], seen[function["function"]])
            leaves = collections.Counter(
                (leaf["category"], leaf["description"], leaf["line"])
                for leaf in function.get("diagnostics", [])
            )
            out[key] = (function["status"], leaves)
    return out


def main() -> int:
    args = sys.argv[1:]
    before, after = rows(args[0]), rows(args[1])
    reference = rows(args[2]) if len(args) == 4 else None
    out_path = args[-1]
    result = {"totals": {}, "moves": [], "leaves": {}}
    for name, side in (("before", before), ("after", after)):
        result["totals"][name] = dict(collections.Counter(status for status, _ in side.values()))
    added = collections.Counter()
    removed = collections.Counter()
    changed_rows = 0
    for key in sorted(set(before) | set(after)):
        b = before.get(key)
        a = after.get(key)
        if b is None or a is None:
            result["moves"].append({"row": list(key), "before": b and b[0], "after": a and a[0]})
            continue
        if b[0] != a[0]:
            result["moves"].append({"row": list(key), "before": b[0], "after": a[0],
                                    "after_leaves": sorted({d for (_, d, _) in a[1]})})
        if b[1] != a[1]:
            changed_rows += 1
            for (category, description, _), count in (b[1] - a[1]).items():
                removed[(category, description)] += count
            for (category, description, _), count in (a[1] - b[1]).items():
                added[(category, description)] += count
    result["leaves"] = {
        "rows_with_changed_leaves": changed_rows,
        "access_removed": removed.get(("unsupported", GATE), 0),
        "access_added": added.get(("unsupported", GATE), 0),
        "removed": [[c, d, n] for (c, d), n in removed.most_common()],
        "added": [[c, d, n] for (c, d), n in added.most_common()],
    }
    if reference is not None:
        files = {key[0] for key in before}
        mismatched = []
        for key, value in before.items():
            ref = reference.get(key)
            if ref is None or ref[0] != value[0] or ref[1] != value[1]:
                mismatched.append(list(key))
        missing = [list(key) for key in reference if key[0] in files and key not in before]
        result["reproduction"] = {
            "rows": len(before),
            "mismatched": len(mismatched),
            "mismatched_rows": mismatched[:200],
            "missing": missing[:200],
        }
    with open(out_path, "w") as handle:
        json.dump(result, handle, indent=1)
    print(json.dumps(result["totals"]), len(result["moves"]), "moves;",
          json.dumps({k: v for k, v in result["leaves"].items() if k not in ("removed", "added")}))
    if reference is not None:
        print("reproduction", {k: v for k, v in result["reproduction"].items() if not isinstance(v, list)})
    return 0


if __name__ == "__main__":
    sys.exit(main())
