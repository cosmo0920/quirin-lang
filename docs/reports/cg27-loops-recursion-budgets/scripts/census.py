#!/usr/bin/env python3
"""CG27-LOOPS-RECURSION-BUDGETS: census of the non-Safe functions a run's
diagnostics attribute to loop shapes, recursion and budget exhaustion.

usage: census.py RUN [--json OUT]   (a run directory or a diagnostics file, .gz accepted)

A function is counted in a family when one of its diagnostics matches the
family; "alone" counts the functions whose every non-Safe leaf belongs to that
one family (an upper bound on what lifting the family could free, since a
lifted leaf may expose another). Recursion is read from the uncontracted-call
leaves: a function whose uncontracted callee is itself is self-recursive; a
pair of functions in one unit, each naming the other as an uncontracted
callee, is mutual (longer cycles are not reconstructed from leaves, so this
is a lower bound). The counted-user pass's loop refusal is the loop family's
`the body contains a loop` leaf (the report carries no clause records).
"""

import argparse
import collections
import gzip
import json
import pathlib
import re

LOOP = (
    "in loop condition",
    "in for-loop condition",
    "in do-while condition",
    "malformed for-loop clauses",
    "for-loop clause with a call",
    "loop re-acquisition",
    "loop exit edge retained",
    "the body contains a loop",
    "is called inside a loop",
    "hazard inside a loop",
    "intrusive-list iteration",
)
BUDGET = (
    "alias set exceeded",
    "more than 512 paths",
    "path budget was exceeded",
    "step budget",
    "symbolic expression limit exceeded",
    "short-circuit path limit exceeded",
    "callback path limit exceeded",
    "callback operation limit exceeded",
    "leaves exceed",
    "recurses or nests too deeply",
)
RECURSIVE_DIRECT = ("its body is in a recursive component", "recursive call to")
UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def load(run: pathlib.Path):
    if run.is_dir():
        candidate = run / "resource-diagnostics.json"
        run = candidate if candidate.exists() else run / "resource-diagnostics.json.gz"
    raw = run.read_bytes()
    if run.suffix == ".gz":
        raw = gzip.decompress(raw)
    return json.loads(raw)


def family(description: str, function: str, unit_self: dict) -> str:
    if any(fragment in description for fragment in LOOP):
        return "loop"
    if any(fragment in description for fragment in BUDGET):
        return "budget"
    if any(fragment in description for fragment in RECURSIVE_DIRECT):
        return "recursion"
    match = UNCONTRACTED.search(description)
    if match:
        callee = match.group(1)
        if callee == function:
            return "recursion"
        if callee in unit_self.get(function, ()):
            return "recursion"
    return "other"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = load(args.run)
    counts = collections.Counter()
    alone = collections.Counter()
    leaves = collections.Counter()
    descriptions = collections.Counter()
    members = collections.defaultdict(list)
    only = collections.defaultdict(list)
    seen = set()
    for unit in data["units"]:
        # The uncontracted callees each function of this unit names.
        callees = collections.defaultdict(set)
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                match = UNCONTRACTED.search(diagnostic.get("description") or "")
                if match:
                    callees[match.group(1)].add(function["function"])
        for function in unit["functions"]:
            name = function["function"]
            # De-duplicated by (unit, function), as compare.py's totals are:
            # a system or slice report lists one file once per configuration.
            if (unit["file"], name) in seen:
                continue
            seen.add((unit["file"], name))
            if function["status"] == "Safe":
                continue
            families = collections.Counter()
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description") or diagnostic.get("category") or ""
                kind = family(description, name, callees)
                families[kind] += 1
                if kind != "other":
                    leaves[kind] += 1
                    descriptions[(kind, re.sub(r"`[^`]*`", "`…`", description))] += 1
            for kind in families:
                if kind == "other":
                    continue
                counts[kind] += 1
                members[kind].append(f"{unit['file']}::{name} ({function['status']})")
                if set(families) == {kind}:
                    alone[kind] += 1
                    only[kind].append({
                        "function": f"{unit['file']}::{name}",
                        "status": function["status"],
                        "leaves": sorted(
                            d.get("description") or d.get("category") or ""
                            for d in function.get("diagnostics", [])
                        ),
                    })
    record = {
        "functions": dict(counts),
        "functions_blocked_only_by_family": dict(alone),
        "leaves": dict(leaves),
        "descriptions": {f"{k}: {d}": n for (k, d), n in descriptions.most_common()},
        "blocked_only_by_family": dict(only),
        "members": {kind: sorted(names) for kind, names in members.items()},
    }
    printable = {key: value for key, value in record.items() if key != "members"}
    print(json.dumps(printable, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
