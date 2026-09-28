#!/usr/bin/env python3
"""Headline tallies of two corpus runs and the byte identity of their other artifacts.

Usage: tallies.py BEFORE_RUN AFTER_RUN OUT_JSON
"""

import collections
import hashlib
import json
import pathlib
import sys

OTHER = ("resource-diagnostics.chunk-contracts.json", "monkey-source.json",
         "resource-diagnostics.source-effects.json")


def tally(run):
    data = json.loads((run / "resource-diagnostics.json").read_text())
    functions = collections.Counter()
    units = collections.Counter(unit["status"] for unit in data["units"])
    categories = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
            for leaf in entry["diagnostics"]:
                categories[leaf["category"]] += 1
    return {"functions": dict(functions), "units": dict(units), "leaves": dict(categories)}


def sorted_rows(path):
    return hashlib.sha256(b"".join(sorted(path.read_bytes().splitlines(keepends=True)))).hexdigest()


def main():
    before, after, out = map(pathlib.Path, sys.argv[1:4])
    result = {"before": tally(before), "after": tally(after), "identical": {}}
    for name in OTHER:
        result["identical"][name] = (before / name).read_bytes() == (after / name).read_bytes()
    for name in ("parity.jsonl", "parity.jsonl.contexts.jsonl"):
        result["identical"][name + " (sorted rows)"] = sorted_rows(before / name) == sorted_rows(after / name)
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(result, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
