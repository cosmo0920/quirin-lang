#!/usr/bin/env python3
"""runtime_summary.py RUN_DIR -- size the strict pair's caller candidates
over every runtime test file (runtime_tests.sh): per file, the census exit,
the roots with a declared call, the violating and not-established readings
by clause and site, and the conforming / undetermined roots. Only readings
whose caller is a test file's own function count as test-side candidates;
readings in library callers (src/, plugins/) are listed apart. Read-only;
prints JSON.
"""
import json
import pathlib
import sys
from collections import Counter


def clause(text):
    for name in ("L1", "L3", "L4", "L5", "L6"):
        if f"({name} " in text:
            return name
    return "other"


def main():
    run = pathlib.Path(sys.argv[1])
    files = [line.strip() for line in (run / "files.txt").read_text().splitlines() if line.strip()]
    out = {"files": len(files), "per_file": {}, "totals": Counter(), "library_callers": Counter(),
           "failed": []}
    for name in files:
        stem = name[:-2]
        exit_code = (run / f"{stem}.exit").read_text().strip() if (run / f"{stem}.exit").exists() else "missing"
        if exit_code != "0":
            out["failed"].append({"file": name, "exit": exit_code})
            continue
        doc = json.loads((run / f"{stem}.json").read_text())
        lc = doc.get("lifecycle", {})
        calls = lc.get("calls", [])
        test_file = f"tests/runtime/{name}"
        row = {"calls": len(calls), "roots": len(lc.get("conformance", {})),
               "violating_roots": 0, "sites": Counter(), "clauses": Counter(),
               "not_established": 0}
        roots = set()
        for call in calls:
            refusals = list(call["violations"]) + list(call.get("not_established", []))
            if not refusals:
                continue
            if call["file"] != test_file:
                for v in refusals:
                    out["library_callers"][f'{call["callee"]} {call["file"]}:{call["line"]} {clause(v)}'] += 1
                continue
            roots.add(call["root"])
            row["sites"][f'{call["callee"]} :{call["line"]}'] += 1
            for v in call["violations"]:
                row["clauses"][clause(v)] += 1
            row["not_established"] += len(call.get("not_established", []))
        row["violating_roots"] = len(roots)
        if row["calls"]:
            out["per_file"][name] = {k: (dict(v) if isinstance(v, Counter) else v) for k, v in row.items()}
            out["totals"]["files_with_calls"] += 1
            out["totals"]["roots"] += row["roots"]
            out["totals"]["violating_roots"] += row["violating_roots"]
            out["totals"]["readings"] += row["calls"]
            out["totals"]["violating_readings"] += sum(row["sites"].values())
            out["totals"]["not_established"] += row["not_established"]
            out["totals"]["distinct_sites"] += len(row["sites"])
            if row["violating_roots"]:
                out["totals"]["files_with_violations"] += 1
            for c, n in row["clauses"].items():
                out["totals"][f"clause_{c}"] += n
    out["totals"] = dict(out["totals"])
    out["library_callers"] = dict(out["library_callers"])
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()
