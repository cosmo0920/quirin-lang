#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: sizes of the card's residuals.

usage: residuals.py BEFORE_RUN AFTER_RUN AFTER_PARITY_JSONL [--json OUT]

* operand positions: BEFORE `__assert_fail` / `abort` / `exit` leaves (by
  unit-free function and line) with no AFTER `call-exit-statement` path end
  at the same function and line: call sites the AFTER applies elsewhere, with
  the continuation kept.
* undeclared no-return callees: AFTER leaves `call to uncontracted function
  X` for every X the AFTER census rows resolve `NoReturn`, the functions
  touched and the Unknown functions blocked by them alone.
"""

import argparse
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from compare import load  # noqa: E402

DECLARED = re.compile(r"^call to uncontracted function `(__assert_fail|abort|exit)`$")
UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")


def diagnostics(run):
    path = pathlib.Path(run)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text())
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                yield unit["file"], function["function"], diagnostic


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    path_ends = set()
    no_return = set()
    with args.parity.open(encoding="utf-8") as handle:
        for line in handle:
            if '"call-exit-' not in line:
                continue
            row = json.loads(line)
            if row.get("position") == "call-exit-statement" and row.get("decision") == "PathEnd":
                path_ends.add((row["function"], row["line"]))
            if row.get("position") == "call-exit-census" and row.get("resolution") == "NoReturn":
                no_return.add(row["callee"])
    before_sites = set()
    for _unit, function, diagnostic in diagnostics(args.before):
        if DECLARED.match(diagnostic.get("description") or ""):
            before_sites.add((function, diagnostic.get("line")))
    elsewhere = sorted(site for site in before_sites if site not in path_ends)
    _, after = load(pathlib.Path(args.after))
    leaves = collections.Counter()
    touched = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    for key, (status, counter) in after.items():
        unsupported = [(d, n) for (c, d), n in counter.items() if c == "unsupported"]
        callees = set()
        others = 0
        for description, n in unsupported:
            match = UNCONTRACTED.match(description or "")
            if match and match.group(1) in no_return:
                leaves[match.group(1)] += n
                touched[match.group(1)].add(key)
                callees.add(match.group(1))
            else:
                others += 1
        if status == "Unknown" and callees and not others and not any(
            c == "unknown_alias" for (c, _), _n in counter.items()
        ):
            for callee in callees:
                sole[callee].add(key)
    record = {
        "before_declared_sites": len(before_sites),
        "path_end_sites": len(path_ends),
        "before_sites_not_path_ends": len(elsewhere),
        "before_sites_not_path_ends_list": [f"{f}:{l}" for f, l in elsewhere],
        "undeclared_no_return": {
            callee: {
                "leaves": leaves[callee],
                "functions": len(touched[callee]),
                "unknown_sole": sorted("::".join(k) for k in sole[callee]),
            }
            for callee in sorted(no_return)
        },
    }
    print(json.dumps({k: v for k, v in record.items() if k != "before_sites_not_path_ends_list"},
                     indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
