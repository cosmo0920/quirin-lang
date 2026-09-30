#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK (copied from CG10-BOUND-UNTRACKED-ROOTS, from CG04-LOOP-SITE-GENERATIONS, from CG06, itself from CG03-CALL-SUMMARY-INSTANTIATION): attribute every verdict move between two corpus runs.

Usage: compare.py BEFORE AFTER [--json OUT]   (a run directory or a diagnostics file, .gz accepted)

Reads each run's resource-diagnostics.json and reports, per function keyed by
(unit, function): the status move, and the change in each diagnostic kind and
unsupported description. Prints the totals, every move, and the leaf deltas by
description; with --json, writes the same as a record.
"""

import argparse
import collections
import gzip
import json
import pathlib


def load(run: pathlib.Path):
    """A run directory, or a resource-diagnostics.json file (gzip accepted)."""
    if run.is_dir():
        run = run / "resource-diagnostics.json"
        if not run.exists():
            run = run.with_name("resource-diagnostics.json.gz")
    raw = run.read_bytes()
    if run.suffix == ".gz":
        raw = gzip.decompress(raw)
    data = json.loads(raw)
    functions = {}
    for unit in data["units"]:
        name = unit["file"]
        for function in unit["functions"]:
            key = (name, function["function"])
            leaves = collections.Counter()
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description")
                if diagnostic.get("category") == "unknown_alias":
                    description = "unresolved resource identity"
                leaves[(diagnostic.get("category"), description)] += 1
            functions[key] = (function["status"], leaves)
    return data, functions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("after", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    _, before = load(args.before)
    _, after = load(args.after)
    totals = {
        "before": dict(collections.Counter(s for s, _ in before.values())),
        "after": dict(collections.Counter(s for s, _ in after.values())),
    }
    moves = []
    leaf_delta = collections.Counter()
    changed_unknown = 0
    for key in sorted(set(before) | set(after), key=str):
        b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
        a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
        delta = collections.Counter(a_leaves)
        delta.subtract(b_leaves)
        delta = {f"{k[0]}: {k[1]}" if k[1] else k[0]: v for k, v in delta.items() if v}
        for label, value in delta.items():
            leaf_delta[label] += value
        if b_status != a_status:
            moves.append({"unit": key[0], "function": key[1], "before": b_status,
                          "after": a_status, "leaf_delta": delta})
        elif delta:
            changed_unknown += 1
    record = {
        "totals": totals,
        "moves": moves,
        "move_counts": dict(collections.Counter(f"{m['before']} -> {m['after']}" for m in moves)),
        "same_status_functions_with_changed_diagnostics": changed_unknown,
        "leaf_delta": dict(sorted(leaf_delta.items(), key=lambda item: item[1])),
    }
    print(json.dumps(record, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
