#!/usr/bin/env python3
"""Attribute every difference between a BEFORE and an AFTER corpus run to a contract.

Usage: attribute.py BEFORE AFTER [--source SOURCE_ROOT] [--json OUT]

BEFORE and AFTER are artifact directories (`diagnostics.json.gz`) or run
directories (`resource-diagnostics.json`). The new declarations are read from --contracts JSON (symbol to owner).
Unit ordinal is retained so repeated compile configurations are not collapsed. A difference is attributed only if it is one of:

* `removed_contracted`: a BEFORE `call to uncontracted function `NAME`` leaf
  with NAME in CONTRACTED;
* `removed_naming_contracted`: any other BEFORE `unsupported` leaf whose
  description names a contracted callee in backquotes (a nested call);
* `removed_cascade_from_moved_callee`: a BEFORE uncontracted-call leaf naming a
  function that moved to Safe in this run (the existing summary rules);
* `removed_on_contracted_call_line`: a BEFORE `unsupported` leaf whose source
  line (read from `--source`, every candidate file agreeing) calls a
  contracted callee (a condition that is now modelled);
* `added_at_contracted_site`: an AFTER leaf at the line of a removed leaf of
  the kinds above, which that coordinate did not carry BEFORE (what the call
  site says once the callee is declared: a premise refusal, a tail refusal,
  the next refusal a modelled condition exposes);
* a status move of a function whose own leaves changed only as above, credited
  to the callees whose leaves it lost;
* a unit-level status or count change whose functions' changes are all
  attributed.

Anything else is UNATTRIBUTED and makes the script exit 1. Every Safe to
non-Safe move is listed as a correction, and every added `leak` or violation
category leaf is listed for classification by reading.
"""

import argparse
import collections
import gzip
import json
import pathlib
import re
import sys

CONTRACTED = {}

UNC = re.compile(r"call to uncontracted function `([^`]+)`")
VIOLATIONS = {"leak", "double_release", "invalid_release", "use_after_release", "violation"}


NAMED = re.compile(r"`([^`]+)`")


def source_index(root: pathlib.Path):
    index = collections.defaultdict(list)
    for path in root.rglob("*.c"):
        if path.is_file():
            index[path.name].append(path)
    return index


def source_calls(sources, origin):
    """Contracted callees called on the source line of ORIGIN (`path:line`), read
    from every file of the source tree whose path ends in ORIGIN's path. Empty
    unless every candidate file's line calls the same contracted callee."""
    if not sources or not origin or ":" not in origin:
        return []
    rel, line = origin.rsplit(":", 1)
    candidates = [p for p in sources.get(pathlib.PurePath(rel).name, []) if str(p).endswith("/" + rel)]
    found = None
    for path in candidates:
        lines = path.read_text(errors="replace").splitlines()
        text = lines[int(line) - 1] if 0 < int(line) <= len(lines) else ""
        names = sorted(n for n in CONTRACTED if re.search(rf"\b{re.escape(n)}\s*\(", text))
        if found is None:
            found = names
        elif found != names:
            return []
    return found or []


def load(path: pathlib.Path):
    if (path / "diagnostics.json.gz").exists():
        return json.load(gzip.open(path / "diagnostics.json.gz"))
    return json.loads((path / "resource-diagnostics.json").read_text())


def leaf(diag):
    return (diag["category"], diag["description"], diag.get("line"), diag.get("origin"))


def index(data):
    functions = {}
    units = {}
    for ordinal, unit in enumerate(data["units"]):
        unit_key = f"{ordinal}:{unit['file']}"
        units[unit_key] = {k: v for k, v in unit.items() if k not in ("functions",)}
        for entry in unit["functions"]:
            key = (unit_key, entry["function"])
            functions[key] = (entry.get("status"), collections.Counter(leaf(d) for d in entry.get("diagnostics") or []))
    return units, functions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("--json")
    parser.add_argument("--contracts", required=True, help="JSON object of added symbols and owner modules")
    parser.add_argument("--source", help="the pinned Fluent Bit source, read to attribute a condition leaf")
    args = parser.parse_args()
    CONTRACTED.update(json.loads(pathlib.Path(args.contracts).read_text()))
    before_units, before = index(load(pathlib.Path(args.before)))
    after_units, after = index(load(pathlib.Path(args.after)))
    sources = source_index(pathlib.Path(args.source)) if args.source else {}
    moved_safe = {
        key[1]
        for key in set(before) & set(after)
        if before[key][0] != "Safe" and after[key][0] == "Safe"
    }
    unattributed = []
    removed_other = collections.Counter()
    removed_on_line = collections.Counter()
    cascade = collections.Counter()
    removed = collections.Counter()
    added = collections.Counter()
    moves = []
    corrections = []
    new_violations = []
    changed_functions = set()
    if set(before) != set(after):
        unattributed.append({"kind": "function set differs",
                             "only_before": sorted(map(list, set(before) - set(after))),
                             "only_after": sorted(map(list, set(after) - set(before)))})
    for key in sorted(set(before) & set(after)):
        b_status, b_leaves = before[key]
        a_status, a_leaves = after[key]
        gone = b_leaves - a_leaves
        new = a_leaves - b_leaves
        if not gone and not new and b_status == a_status:
            continue
        changed_functions.add(key)
        credited = set()
        contracted_lines = set()
        ok = True
        for item, count in gone.items():
            match = UNC.fullmatch(item[1])
            named = [n for n in NAMED.findall(item[1]) if n in CONTRACTED]
            on_line = source_calls(sources, item[3])
            if item[0] == "unsupported" and match and match.group(1) in CONTRACTED:
                removed[match.group(1)] += count
                credited.add(match.group(1))
                contracted_lines.add(item[2])
            elif item[0] == "unsupported" and named:
                removed_other[(named[0], re.sub(r"`[^`]*`", "`X`", item[1]))] += count
                credited.add(named[0])
                contracted_lines.add(item[2])
            elif item[0] == "unsupported" and match and match.group(1) in moved_safe:
                cascade[(match.group(1), f"{key[0]}::{key[1]}")] += count
                credited.add(f"cascade:{match.group(1)}")
            elif item[0] == "unsupported" and on_line:
                removed_on_line[(on_line[0], item[1])] += count
                credited.add(on_line[0])
                contracted_lines.add(item[2])
            else:
                ok = False
                unattributed.append({"kind": "removed leaf", "function": list(key), "leaf": list(item), "count": count})
        b_lines = {item[2] for item in b_leaves}
        for item, count in new.items():
            if item[0] in VIOLATIONS:
                new_violations.append({"function": list(key), "leaf": list(item), "count": count})
            if item[2] in contracted_lines:
                added[(item[0], re.sub(r"`[^`]*`", "`X`", item[1]))] += count
                continue
            ok = False
            unattributed.append({"kind": "added leaf", "function": list(key), "leaf": list(item), "count": count,
                                 "coordinate_had_leaves_before": item[2] in b_lines})
        if b_status != a_status:
            move = {"function": f"{key[0]}::{key[1]}", "before": b_status, "after": a_status,
                    "moved_by": sorted(credited), "attributed": ok}
            moves.append(move)
            if b_status == "Safe":
                corrections.append(move)
            if not ok:
                unattributed.append({"kind": "status move with unattributed leaves", **move})
    unit_changes = []
    for name in sorted(set(before_units) | set(after_units)):
        b = before_units.get(name)
        a = after_units.get(name)
        if b == a:
            continue
        unit_changes.append({"unit": name, "before_status": (b or {}).get("status"), "after_status": (a or {}).get("status"),
                             "fields": sorted(k for k in set(b or {}) | set(a or {}) if (b or {}).get(k) != (a or {}).get(k))})
    counts = {
        "before": collections.Counter(s for s, _ in before.values()),
        "after": collections.Counter(s for s, _ in after.values()),
    }
    report = {
        "function_status_before": dict(counts["before"]),
        "function_status_after": dict(counts["after"]),
        "unit_status_before": dict(collections.Counter(u.get("status") for u in before_units.values())),
        "unit_status_after": dict(collections.Counter(u.get("status") for u in after_units.values())),
        "changed_functions": len(changed_functions),
        "removed_contracted_leaves": dict(sorted(removed.items())),
        "removed_contracted_total": sum(removed.values()),
        "removed_naming_contracted": [{"callee": c, "description": d, "count": n} for (c, d), n in sorted(removed_other.items())],
        "removed_on_contracted_call_line": [{"callee": c, "description": d, "count": n} for (c, d), n in sorted(removed_on_line.items())],
        "removed_cascade_from_moved_callee": [{"callee": c, "caller": f, "count": n} for (c, f), n in sorted(cascade.items())],
        "added_at_contracted_sites": [{"category": c, "description": d, "count": n} for (c, d), n in sorted(added.items())],
        "status_moves": moves,
        "corrections": corrections,
        "new_leak_or_violation_leaves": new_violations,
        "unit_changes": unit_changes,
        "unattributed": unattributed,
    }
    text = json.dumps(report, indent=1, sort_keys=True)
    if args.json:
        pathlib.Path(args.json).write_text(text + "\n")
    print(text)
    return 1 if unattributed else 0


if __name__ == "__main__":
    sys.exit(main())
