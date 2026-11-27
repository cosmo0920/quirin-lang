#!/usr/bin/env python3
"""Tally a `quirin-audit current` tail-pause slice report.

Usage: tally.py REPORT[.gz] BINDINGS_JSON OUT_JSON

Writes unit and distinct-function status tallies (static/shared variants
collapsed; disagreeing variants count as "mixed"), the report's diagnostic
counts, every Safe and every non-Unknown function with its diagnostics, and
the status and leaf categories of every function `bindings.json` names that
lies in a selected file. It reads only the report; it proves nothing about
concurrency.
"""

import collections
import gzip
import json
import sys


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def unit_output(unit):
    words = unit["compile_configuration"]["command"].split()
    return words[words.index("-o") + 1] if "-o" in words else ""


def main():
    report = load(sys.argv[1])
    bindings = load(sys.argv[2])
    rows = {}
    units = collections.Counter()
    unit_rows = []
    for unit in report["units"]:
        units[unit["status"]] += 1
        output = unit_output(unit)
        unit_rows.append(
            {
                "file": unit["file"],
                "output": output,
                "status": unit["status"],
                "functions": len(unit["functions"]),
                "diagnostic_counts": unit["diagnostic_counts"],
            }
        )
        for function in unit["functions"]:
            rows.setdefault((unit["file"], function["function"]), []).append(
                {
                    "output": output,
                    "status": function["status"],
                    "categories": dict(
                        collections.Counter(d["category"] for d in function["diagnostics"])
                    ),
                    "diagnostics": [
                        {k: d.get(k) for k in ("category", "description", "origin")}
                        for d in function["diagnostics"]
                    ],
                }
            )
    distinct = collections.Counter()
    for variants in rows.values():
        statuses = {variant["status"] for variant in variants}
        distinct[next(iter(statuses)) if len(statuses) == 1 else "mixed"] += 1
    notable = {
        f"{file}:{name}": variants
        for (file, name), variants in sorted(rows.items())
        if any(variant["status"] != "Unknown" for variant in variants)
    }
    bound = {}
    for entry in bindings["bindings"]:
        key = (entry["file"], entry["function"])
        if key in rows:
            bound[f"{entry['file']}:{entry['function']}"] = [
                {k: v for k, v in variant.items() if k != "diagnostics"}
                for variant in rows[key]
            ]
    result = {
        "report": sys.argv[1],
        "fluent_bit_revision": report.get("fluent_bit_revision"),
        "units": dict(units),
        "unit_rows": unit_rows,
        "distinct_functions": dict(distinct),
        "function_variants": sum(len(v) for v in rows.values()),
        "diagnostic_counts": report["diagnostic_counts"],
        "non_unknown_functions": notable,
        "bound_functions_in_slice": bound,
    }
    with open(sys.argv[3], "w") as handle:
        json.dump(result, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(json.dumps({k: result[k] for k in ("units", "distinct_functions", "function_variants", "diagnostic_counts")}, sort_keys=True))
    for key, variants in notable.items():
        print(key, [(v["status"], v["categories"]) for v in variants])


if __name__ == "__main__":
    main()
