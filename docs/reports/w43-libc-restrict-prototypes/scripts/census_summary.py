#!/usr/bin/env python3
"""Summarise the declared-ABI censuses written by census.sh.

Usage: census_summary.py OUT.json LABEL...

For each LABEL: per-name outcome counts for the base reader (old) and this
card's reader (new), and every (unit, name) row whose outcome moved between
the two readers.
"""
import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
CENSUS = ROOT / "measure/census"


def rows(path):
    table = {}
    for line in path.read_text().splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            table[(pathlib.Path(parts[0]).name, parts[1])] = parts[2]
    return table


def per_name(table):
    counts = collections.defaultdict(collections.Counter)
    for (_, name), outcome in table.items():
        counts[name][outcome] += 1
    return {name: dict(sorted(c.items())) for name, c in sorted(counts.items())}


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    summary = {}
    for label in sys.argv[2:]:
        old_path, new_path = CENSUS / f"{label}-old.txt", CENSUS / f"{label}-new.txt"
        old, new = rows(old_path), rows(new_path)
        moved = collections.Counter()
        examples = {}
        for key in sorted(set(old) | set(new)):
            a, b = old.get(key, "absent"), new.get(key, "absent")
            if a != b:
                moved[(key[1], a, b)] += 1
                examples.setdefault((key[1], a, b), key[0])
        summary[label] = {
            "rows_old": len(old),
            "rows_new": len(new),
            "old_tool_output_head": old_path.read_text().splitlines()[:2] if not old else None,
            "per_name_old": per_name(old),
            "per_name_new": per_name(new),
            "moved": [
                {"name": n, "old": a, "new": b, "rows": c, "example_unit": examples[(n, a, b)]}
                for (n, a, b), c in sorted(moved.items())
            ],
        }
    out.write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    for label, record in summary.items():
        print(label, record["rows_old"], record["rows_new"], "moved", sum(m["rows"] for m in record["moved"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
