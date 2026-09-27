#!/usr/bin/env python3
"""Join every changed diagnostic to the side-by-side statement coordinates.

Usage: attribute.py CHANGED_FUNCTIONS_GZ PARITY_JSONL OUTPUT_DIR.
"""

import collections
import gzip
import hashlib
import io
import json
import pathlib
import sys


def writer(path):
    return io.TextIOWrapper(gzip.GzipFile(filename=str(path), mode="wb", mtime=0), encoding="utf-8")


def main():
    changes, parity, out = map(pathlib.Path, sys.argv[1:4])
    out.mkdir(parents=True, exist_ok=False)
    direct = collections.defaultdict(set)
    direct_by_function = collections.defaultdict(set)
    with parity.open() as stream:
        for line in stream:
            row = json.loads(line)
            if row["old_route"] == "old_refusal" and row["new_admitted"]:
                key = (row["function"], row["line"])
                direct[key].add(row["source_sha256"])
                direct_by_function[row["function"]].add(row["line"])
    totals = collections.Counter()
    families = collections.Counter()
    rows = []
    with gzip.open(changes, "rt") as stream:
        for line in stream:
            function = json.loads(line)
            for side in ("removed", "added"):
                for diagnostic in function[side]:
                    key = (function["function"], diagnostic["line"])
                    exact_sources = sorted(direct[key])
                    if exact_sources:
                        relationship = "newly_admitted_statement_same_line"
                    elif direct_by_function[function["function"]]:
                        relationship = "same_function_downstream_or_summary"
                    else:
                        relationship = "no_new_statement_in_function"
                    row = {
                        "library": function["library"],
                        "file": function["file"],
                        "function": function["function"],
                        "verdict_before": function["before"],
                        "verdict_after": function["after"],
                        "side": side,
                        "diagnostic": diagnostic,
                        "relationship": relationship,
                        "matching_source_sha256": exact_sources,
                        "new_statement_lines_in_function": sorted(direct_by_function[function["function"]]),
                    }
                    rows.append(row)
                    totals[(side, diagnostic["category"], relationship)] += diagnostic["count"]
                    families[(side, diagnostic["description"], relationship)] += diagnostic["count"]
    with writer(out / "changed-leaf-attribution.jsonl.gz") as stream:
        for row in sorted(rows, key=lambda r: (r["library"], r["file"], r["function"], r["side"], r["diagnostic"]["line"], r["diagnostic"]["description"])):
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    result = {
        "schema": "qcore.w46-access-path-statements-attribution.v1",
        "changed_leaf_rows": len(rows),
        "weighted_counts": [
            {"side": side, "category": category, "relationship": relationship, "count": count}
            for (side, category, relationship), count in sorted(totals.items())
        ],
        "families": [
            {"side": side, "description": description, "relationship": relationship, "count": count}
            for (side, description, relationship), count in sorted(families.items())
        ],
        "attribution_sha256": hashlib.sha256((out / "changed-leaf-attribution.jsonl.gz").read_bytes()).hexdigest(),
    }
    (out / "attribution.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["weighted_counts"], indent=2))


if __name__ == "__main__":
    main()
