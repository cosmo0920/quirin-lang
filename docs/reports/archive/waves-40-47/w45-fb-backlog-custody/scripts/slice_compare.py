#!/usr/bin/env python3
"""Compare two `quirin-audit current` slice reports function by function.

Usage: slice_compare.py RUN_REPORT BASELINE_REPORT [--functions NAME,...] [--out FILE]
                        [--ignore-lines]

--ignore-lines compares a diagnostic's origin by file only, so a source
revision that moves lines does not count as a difference.

A report may be plain JSON or gzipped. Each function is keyed by
(file, unit output object, function) so the static/shared variants stay
separate. Prints unit and function status tallies for both and every function
whose status or diagnostic multiset (category, description, origin) differs.
With --functions, also prints each named function's rows in both reports.
"""

import collections
import gzip
import json
import sys


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def unit_key(unit):
    command = unit["compile_configuration"]["command"]
    words = command.split()
    output = words[words.index("-o") + 1] if "-o" in words else ""
    return unit["file"], output


def origin_file(origin):
    return origin.rsplit(":", 1)[0] if IGNORE_LINES else origin


IGNORE_LINES = "--ignore-lines" in sys.argv


def functions(report):
    rows = {}
    for unit in report["units"]:
        file, output = unit_key(unit)
        for function in unit["functions"]:
            diagnostics = sorted(
                (d["category"], d["description"], origin_file(d.get("origin") or ""))
                for d in function["diagnostics"]
            )
            rows[(file, output, function["function"])] = (function["status"], diagnostics)
        for name in unit.get("not_projected", []):
            label = name if isinstance(name, str) else json.dumps(name, sort_keys=True)
            rows[(file, output, "not-projected:" + label)] = ("NotProjected", [])
    return rows


def distinct(rows):
    """Status tally over distinct (file, function), static/shared collapsed.

    A function whose variants disagree is counted under "mixed"."""
    seen = {}
    for (file, _output, name), (status, _diagnostics) in rows.items():
        seen.setdefault((file, name), set()).add(status)
    return dict(
        collections.Counter(
            next(iter(statuses)) if len(statuses) == 1 else "mixed" for statuses in seen.values()
        )
    )


def main():
    run_path, base_path = sys.argv[1], sys.argv[2]
    names = []
    out = None
    if "--functions" in sys.argv:
        names = sys.argv[sys.argv.index("--functions") + 1].split(",")
    if "--out" in sys.argv:
        out = sys.argv[sys.argv.index("--out") + 1]
    run, base = load(run_path), load(base_path)
    a, b = functions(run), functions(base)
    result = {
        "run": run_path,
        "baseline": base_path,
        "run_units": dict(collections.Counter(u["status"] for u in run["units"])),
        "baseline_units": dict(collections.Counter(u["status"] for u in base["units"])),
        "run_functions": dict(collections.Counter(v[0] for v in a.values())),
        "baseline_functions": dict(collections.Counter(v[0] for v in b.values())),
        "run_counts": run["diagnostic_counts"],
        "baseline_counts": base["diagnostic_counts"],
        "run_distinct": distinct(a),
        "baseline_distinct": distinct(b),
        "differing": [],
        "named": {},
    }
    for key in sorted(set(a) | set(b)):
        if a.get(key) != b.get(key):
            result["differing"].append(
                {
                    "key": list(key),
                    "run": a.get(key),
                    "baseline": b.get(key),
                }
            )
    for name in names:
        result["named"][name] = {
            "run": {"/".join(k[:2]): v for k, v in a.items() if k[2] == name},
            "baseline": {"/".join(k[:2]): v for k, v in b.items() if k[2] == name},
        }
    text = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if out:
        with open(out, "w") as handle:
            handle.write(text)
    print(
        json.dumps(
            {k: result[k] for k in ("run_units", "baseline_units", "run_functions", "baseline_functions", "run_distinct", "baseline_distinct", "run_counts", "baseline_counts")},
            sort_keys=True,
        )
    )
    print("differing functions:", len(result["differing"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
