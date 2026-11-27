#!/usr/bin/env python3
"""RETENTION-UNMATCHED-REMOVAL-POISONS: census.py OUT.json ROUTE=REPORT...

Read-only. For each route's resource report (`resource-diagnostics.json`, or a
directory holding one), every function row whose leaves this card can change:

* `loop`: a leaf naming `mk_event_loop_create` or `mk_event_loop_destroy`
  (today `call to uncontracted function ...`); the two declarations this card
  adds replace it;
* `retention`: a rule-12t leaf (`retention-undecided`,
  `retention-released-while-held`), the leaves the poisoning can rename
  (an exit after an unmatched removal reads `retention-removal-poisoned`).

Each row is listed with its status and those leaves. Nothing else is read."""
import collections
import json
import pathlib
import sys

LOOP = ("mk_event_loop_create", "mk_event_loop_destroy")
RETENTION = ("retention-undecided", "retention-released-while-held", "retention-removal-poisoned")


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def rows(data):
    out = {"loop": [], "retention": []}
    seen = collections.Counter()
    for unit in data["units"]:
        ordinal = seen[unit["file"]]
        seen[unit["file"]] += 1
        name = unit["file"] if ordinal == 0 else f"{unit['file']}#{ordinal}"
        for function in unit.get("functions", []):
            leaves = [d.get("description") or "" for d in function.get("diagnostics", [])]
            for group, keys in (("loop", LOOP), ("retention", RETENTION)):
                hits = sorted({leaf for leaf in leaves if any(key in leaf for key in keys)})
                if hits:
                    out[group].append({"file": name, "function": function["function"],
                                       "status": function.get("status"), "leaves": hits})
    return out


def main():
    dest = pathlib.Path(sys.argv[1])
    result = {}
    for argument in sys.argv[2:]:
        route, report = argument.split("=", 1)
        result[route] = rows(load(report))
        print(route, {group: len(found) for group, found in result[route].items()})
    dest.write_text(json.dumps(result, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
