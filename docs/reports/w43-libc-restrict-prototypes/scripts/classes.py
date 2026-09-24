#!/usr/bin/env python3
"""Split this card's arbiter movement into its attribution classes.

Usage: classes.py BEFORE READER DECL ATTRIBUTION.json OUT.json

BEFORE is the base binary's run; READER is this card's binary with the base
declarations (the array-bound reader change alone); DECL is the same binary
with the ten declarations. ATTRIBUTION.json is
`w43-libc-contracts/scripts/attribute.py READER DECL ...` with the ten
declarations as its module list.

Classes:
  (a) reader change alone     READER vs BEFORE, every artifact compared
  (b) declarations            DECL vs READER, per declared function: the leaves
                              each name decided or renamed, and the functions
                              that moved
  (c) violation candidates    every added leak / double_release /
                              invalid_release / use_after_release leaf

Exits 0 only when the attribution is complete, class (a) is byte-identical and
class (c) is empty.
"""

import collections
import json
import pathlib
import re
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
CANDIDATES = ("leak", "double_release", "invalid_release", "use_after_release")
NAMES = ["stat", "strftime", "gettimeofday", "getsockopt", "getpeername", "dlsym",
         "cnd_timedwait", "pthread_sigmask", "regexec", "regerror"]


def load(directory):
    return json.loads((pathlib.Path(directory) / "resource-diagnostics.json").read_text())


def statuses(data):
    functions, units = collections.Counter(), collections.Counter()
    table = {}
    for unit in data["units"]:
        units[unit["status"]] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
            table[(unit["library"], unit["file"], entry["function"])] = entry["status"]
    return dict(sorted(functions.items())), dict(sorted(units.items())), table


def owner(leaf):
    """The declared name a changed leaf belongs to, read from its text."""
    text = leaf["description"] + " " + " ".join(leaf.get("line_calls") or [])
    found = [n for n in NAMES if re.search(r"(?<![A-Za-z0-9_])" + n + r"(?![A-Za-z0-9_])", text)]
    return found


def main() -> int:
    before, reader, decl, attribution_path, out = sys.argv[1:6]
    record = {}
    identical = {a: (pathlib.Path(before) / a).read_bytes() == (pathlib.Path(reader) / a).read_bytes()
                 for a in ARTIFACTS}
    record["a_reader_change_alone"] = {"byte_identical": identical, "all_identical": all(identical.values())}

    attribution = json.loads(pathlib.Path(attribution_path).read_text())
    summary = attribution["summary"]
    fr, ur, tr = statuses(load(reader))
    fd, ud, td = statuses(load(decl))
    moves = sorted([f"{k[0]}/{k[1]}::{k[2]}", tr[k], td[k]] for k in tr if tr[k] != td[k])
    per_name = collections.defaultdict(lambda: collections.Counter())
    functions_by_name = collections.defaultdict(set)
    for leaf in attribution["leaves"]:
        names = owner(leaf) or ["(newly Safe callee or same-function)"]
        key = f"{leaf['direction']} {leaf['class']}"
        for name in names:
            per_name[name][key] += leaf["count"]
            functions_by_name[name].add(f"{leaf['unit']}::{leaf['function']}")
    record["b_declarations"] = {
        "functions_reader": fr, "functions_decl": fd,
        "units_reader": ur, "units_decl": ud,
        "function_moves": moves,
        "diagnostic_counts": summary["diagnostic_counts"],
        "by_class": summary["by_class"],
        "per_name": {n: dict(c) for n, c in sorted(per_name.items())},
        "functions_by_name": {n: sorted(v) for n, v in sorted(functions_by_name.items())},
        "attribution_complete": summary["attribution_complete"],
        "unattributed": summary["unattributed"],
        "unit_status_moves": [u for u in summary["unit_changes"] if u["status"][0] != u["status"][1]],
    }
    candidates = [leaf for leaf in attribution["leaves"]
                  if leaf["direction"] == "added" and leaf["category"] in CANDIDATES]
    record["c_violation_candidates"] = candidates
    pathlib.Path(out).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({"a": record["a_reader_change_alone"]["all_identical"],
                      "moves": moves, "candidates": len(candidates),
                      "complete": summary["attribution_complete"]}, indent=1))
    ok = record["a_reader_change_alone"]["all_identical"] and summary["attribution_complete"] and not candidates
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
