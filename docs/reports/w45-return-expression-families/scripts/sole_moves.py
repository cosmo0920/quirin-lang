#!/usr/bin/env python3
"""Where the functions the return refusal alone blocked stand after the card.

Usage: sole_moves.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS [--json OUT]

A function is sole-blocked when it is Unknown and every one of its
diagnostics is the return-position refusal (the blocker census's definition).
For each BEFORE sole-blocked function, reports its AFTER status and, if it is
still Unknown, the distinct reasons it now carries. Also counts the AFTER
sole-blocked population and the functions newly sole-blocked.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"


def load(path):
    data = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def functions(document):
    out = {}
    for unit in document["units"]:
        for entry in unit["functions"]:
            out[(unit["library"], unit["file"], entry["function"])] = entry
    return out


def reasons(entry):
    return {re.sub(r"`[^`]*`", "`_`", d["description"]) for d in entry.get("diagnostics") or []}


def sole(entries):
    return {key for key, entry in entries.items()
            if entry["status"] == "Unknown" and reasons(entry) == {RETURN}}


def main():
    before = functions(load(sys.argv[1]))
    after = functions(load(sys.argv[2]))
    sole_before = sole(before)
    sole_after = sole(after)
    outcome = collections.Counter()
    rows = []
    for key in sorted(sole_before):
        entry = after[key]
        remaining = sorted(reasons(entry))
        label = entry["status"] if entry["status"] != "Unknown" else (
            "Unknown, still sole" if remaining == [RETURN] else "Unknown, other reasons")
        outcome[label] += 1
        rows.append({"function": list(key), "after": entry["status"], "reasons": remaining})
    summary = {
        "sole_before": len(sole_before),
        "sole_after": len(sole_after),
        "sole_before_outcome": dict(outcome),
        "newly_sole": len(sole_after - sole_before),
    }
    print(json.dumps(summary, indent=1))
    if "--json" in sys.argv:
        out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1])
        out.write_text(json.dumps({"summary": summary, "functions": rows,
                                   "newly_sole": sorted(map(list, sole_after - sole_before))}, indent=1) + "\n")


if __name__ == "__main__":
    main()
