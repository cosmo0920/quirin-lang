#!/usr/bin/env python3
"""Every changed leaf of every changed function between two corpus runs.

Usage: changed_leaves.py BEFORE_RUN AFTER_RUN OUT_JSONL

One row per (function, leaf) whose multiplicity differs, with `change`
(`removed`/`added`), the leaf's category, description, internal line and origin.
"""

import collections
import json
import pathlib
import sys


def index(run):
    data = json.loads((run / "resource-diagnostics.json").read_text())
    out = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            out[f"{unit['file']}::{entry['function']}"] = (
                entry["status"],
                collections.Counter((d["category"], d["description"], d["line"], d.get("origin"))
                                    for d in entry["diagnostics"]))
    return out


def main():
    before, after = index(pathlib.Path(sys.argv[1])), index(pathlib.Path(sys.argv[2]))
    rows = []
    for key in sorted(set(before) | set(after)):
        b_status, b = before.get(key, (None, collections.Counter()))
        a_status, a = after.get(key, (None, collections.Counter()))
        for change, diff in (("removed", b - a), ("added", a - b)):
            for (category, description, line, origin), count in sorted(diff.items(), key=str):
                rows.append({"function": key, "status": f"{b_status} -> {a_status}", "change": change,
                             "category": category, "description": description, "line": line,
                             "origin": origin, "count": count})
    pathlib.Path(sys.argv[3]).write_text("".join(json.dumps(r) + "\n" for r in rows))
    for r in rows:
        print(r["change"], r["function"], r["status"], r["category"], r["origin"], r["description"][:90])


if __name__ == "__main__":
    main()
