#!/usr/bin/env python3
"""Copy the census's published evidence into the report and write its tables.

usage: publish.py WORK_DIR OUT_DIR

Per scope: `leaves.jsonl.gz` (every unknown_alias leaf with class, group,
cause and facts), `summary.json`, `unmatched.json`, `functions-class.json`,
`functions-group.json`; and `tables.json`: per discharge group the leaves,
first leaves (the earliest leaf of a (function, name) pair), functions with
any leaf of the group, functions whose every alias leaf is in it
(`exclusive`), blocked by it alone, with exactly one other family, and the
closure gain beyond the baseline. Refuses (exit 3) when the class or group
totals do not sum to the report's `unknown_alias` count.
"""

import collections
import gzip
import json
import pathlib
import shutil
import sys


def main():
    work, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    tables = {}
    bad = []
    for scope in ("corpus", "system"):
        source = work / scope
        target = out / scope
        target.mkdir(exist_ok=True)
        for name in ("leaves.jsonl.gz", "summary.json", "unmatched.json", "functions-class.json",
                     "functions-group.json"):
            shutil.copyfile(source / name, target / name)
        summary = json.loads((source / "summary.json").read_text())
        rows = [json.loads(line) for line in gzip.open(source / "leaves.jsonl.gz", "rt")][1:]
        functions = json.loads((source / "functions-group.json").read_text())
        total = summary["report_unknown_alias"]
        if sum(summary["classes"].values()) != total or sum(summary["groups"].values()) != total \
                or len(rows) != total:
            bad.append(scope)
        leaves = collections.Counter(row["group"] for row in rows)
        first = collections.Counter(row["group"] for row in rows if row["first"])
        table = {}
        for group in sorted(leaves, key=lambda g: -leaves[g]):
            ranking = functions["ranking"].get(group, {})
            closure = functions.get("closure", {}).get(group, {})
            table[group] = {
                "leaves": leaves[group],
                "first_leaves": first[group],
                "functions_any": functions["functions_per_class"].get(group, 0),
                "functions_exclusive": ranking.get("exclusive", 0),
                "alone": ranking.get("alone", 0),
                "plus_one_family": ranking.get("plus_one", 0),
                "plus_one_by_family": {k[5:]: v for k, v in ranking.items() if k.startswith("plus_") and k != "plus_one"},
                "closure_same_unit": closure.get("same-unit"),
                "closure_cross_unit": closure.get("cross-unit"),
            }
        tables[scope] = {
            "unknown_alias": total,
            "unclassified": summary["classes"].get("unclassified", 0),
            "classes": summary["classes"],
            "groups": table,
            "unknown_functions": functions["unknown_functions"],
            "unknown_with_alias": functions["unknown_with_alias"],
            "alone_total": functions["alone_total"],
            "alias_plus_one_family_by_family": functions["alias_plus_one_family_by_family"],
            "every_class_closure": functions.get("closure", {}).get("<every class>"),
            "closure_baseline": functions.get("closure_baseline"),
        }
    (out / "tables.json").write_text(json.dumps(tables, indent=1) + "\n")
    print(json.dumps({s: {"unknown_alias": t["unknown_alias"], "unclassified": t["unclassified"],
                          "alone_total": t["alone_total"]} for s, t in tables.items()}, indent=1))
    if bad:
        print(f"totals do not sum in {bad}", file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
