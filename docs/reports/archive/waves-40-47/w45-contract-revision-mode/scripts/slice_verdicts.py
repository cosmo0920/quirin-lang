#!/usr/bin/env python3
"""Whether a slice run under the revision mode gives the verdicts of an
archived slice run: unit statuses, function statuses and each function's
diagnostic multiset (category, description, line) in unit order.

usage: slice_verdicts.py RUN_REPORT ARCHIVED_REPORT[.gz]

Prints the tallies of both (units Safe / Unsafe / Unknown / data-only;
functions Safe / Unsafe / Unknown / not projected; the diagnostic counts), the
named OR functions' statuses, and every difference. Exit 0 when there is none.
"""
import collections
import gzip
import json
import pathlib
import sys

OR_FUNCTIONS = ("flb_output_task_singleplex_enqueue", "flb_output_task_flush")


def load(path):
    path = pathlib.Path(path)
    data = path.read_bytes()
    if path.suffix == ".gz":
        data = gzip.decompress(data)
    return json.loads(data)


def tally(report):
    units = collections.Counter(u["status"] for u in report["units"])
    functions = collections.Counter()
    for unit in report["units"]:
        functions.update(f["status"] for f in unit["functions"])
        functions["not projected"] += len(unit["not_projected"])
    # Distinct functions by (file, name), as the retry card's report counts
    # them (static and shared configurations agree).
    distinct = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            distinct[(unit["file"], function["function"])] = function["status"]
        for name in unit["not_projected"]:
            distinct.setdefault((unit["file"], name), "not projected")
    by_status = collections.Counter(distinct.values())
    return {
        "units": [units["Safe"], units["Unsafe"], units["Unknown"],
                  units["data_only"] + units["data-only"]],
        "distinct_functions": [by_status["Safe"], by_status["Unsafe"], by_status["Unknown"],
                               by_status["not projected"]],
        "functions": [functions["Safe"], functions["Unsafe"], functions["Unknown"],
                      functions["not projected"]],
        "diagnostic_counts": report.get("diagnostic_counts"),
        "or": {name: sorted({f["status"] for u in report["units"] for f in u["functions"]
                             if f["function"] == name}) for name in OR_FUNCTIONS},
    }


def rows(report):
    result = []
    for unit in report["units"]:
        result.append(("unit", unit["library"], unit["file"], unit["status"],
                       unit.get("reason_code")))
        for function in unit["functions"]:
            diagnostics = sorted((d["category"], d["description"], d["line"])
                                 for d in function["diagnostics"])
            result.append(("function", unit["file"], function["function"],
                           function["status"], json.dumps(diagnostics)))
    return result


def main() -> int:
    run, archived = load(sys.argv[1]), load(sys.argv[2])
    ours, theirs = rows(run), rows(archived)
    differences = [
        {"run": a, "archived": b} for a, b in zip(ours, theirs) if a != b
    ]
    if len(ours) != len(theirs):
        differences.append({"row_count": [len(ours), len(theirs)]})
    result = {
        "run": tally(run),
        "archived": tally(archived),
        "rows_compared": len(ours),
        "differences": differences,
        "verdicts_identical": not differences,
    }
    print(json.dumps(result, indent=2))
    return 0 if not differences else 1


if __name__ == "__main__":
    sys.exit(main())
