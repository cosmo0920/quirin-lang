#!/usr/bin/env python3
"""CFL-MPACK-EXTERNAL-DECLARATIONS: in-scope callees on each route report.

    python3 routes.py REPORT [REPORT ...]

For each `resource-diagnostics.json` given (the baseline's corpus, system and
slice reports, or the user's `current` run), lists the functions with a
`call to uncontracted function` leaf naming a CFL or mpack callee, and those
whose every leaf is such a call (the only rows that could move).
"""
import collections
import json
import re
import sys
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")
PREFIXES = ("cfl_", "mpack_")


def scan(path):
    report = json.loads(Path(path).read_text())
    rows = []
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves = function["diagnostics"]
            named = [UNCONTRACTED.match(d["description"]) for d in leaves]
            scoped = sorted({m.group(1) for m in named if m and m.group(1).startswith(PREFIXES)})
            if not scoped:
                continue
            others = [d["description"] for d, m in zip(leaves, named)
                      if not (m and m.group(1).startswith(PREFIXES))]
            rows.append({
                "library": unit["library"], "file": unit["file"], "function": function["function"],
                "status": function["status"], "scoped": scoped, "other_leaves": others,
            })
    return rows


def main():
    for path in sys.argv[1:]:
        rows = scan(path)
        only = [r for r in rows if not r["other_leaves"]]
        callees = collections.Counter(c for r in rows for c in r["scoped"])
        print(f"== {path}")
        print(f"functions with an in-scope leaf: {len(rows)}; only in-scope leaves: {len(only)}")
        for r in only:
            print(f"  ONLY {r['library']}::{r['file']}::{r['function']} [{r['status']}] {r['scoped']}")
        print("  callees:", dict(callees.most_common()))


if __name__ == "__main__":
    main()
