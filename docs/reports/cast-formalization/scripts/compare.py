#!/usr/bin/env python3
"""CAST-FORMALIZATION: every moving function between two route reports, with
the leaves it lost and gained, attributed to the card's hooks.

Usage: compare.py BEFORE AFTER AFTER-PARITY.jsonl OUT.json

Rows are keyed by (unit file, the unit's occurrence of that file, function,
row index within the unit), so the system and slice routes' per-configuration
units are compared one for one. A function moves when its status or its
multiset of leaves (line, description) changes.

Attribution, from the AFTER parity stream:
* `H-U0` -- the function has a `cast-semantics-value` row answered
  `uncast-actuals-admitted` or `uncast-actuals-refused` (the call-actual
  soundness repair replaced the classifier's answer);
* `H-U` -- a row answered `uncast-admitted` or `uncast-tail-admitted`;
* `H-V` -- a lost leaf sits on a line holding an explicit cast of an
  access-admitting class (`typed-view`, `byte-view`, `qualifier`) at a member
  root, dereference or subscript base (`cast-semantics-census` rows);
* `H-SR5` -- a gained leaf is a `return` refusal on a line holding a cast
  under a `*` (read by hand);
* `other` -- none of these (read by hand).
A function may carry several labels.
"""
import collections
import json
import pathlib
import re
import sys

TEMPLATE = re.compile(r"`[^`]*`")
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
ACCESS = {"member-root", "dereference", "subscript-base"}
VIEWS = {"typed-view", "byte-view", "qualifier"}


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def rows(report):
    out = {}
    units = collections.Counter()
    for unit in report["units"]:
        units[unit["file"]] += 1
        seen = collections.Counter()
        for entry in unit["functions"]:
            seen[entry["function"]] += 1
            key = (unit["file"], units[unit["file"]], entry["function"], seen[entry["function"]])
            leaves = collections.Counter(
                (d.get("line"), d.get("description", "")) for d in entry.get("diagnostics") or [])
            out[key] = (entry["status"], leaves)
    return out


def main():
    before, after = rows(load(sys.argv[1])), rows(load(sys.argv[2]))
    hooks = collections.defaultdict(set)
    access_lines = collections.defaultdict(set)
    for line in open(sys.argv[3], encoding="utf-8"):
        if '"cast-semantics-value"' in line:
            row = json.loads(line)
            answer = row["answer"]
            if answer.startswith("uncast-actuals"):
                hooks[row["function"]].add("H-U0")
            elif answer in ("uncast-admitted", "uncast-tail-admitted"):
                hooks[row["function"]].add("H-U")
        elif '"cast-semantics-census"' in line:
            row = json.loads(line)
            if row["class"] in VIEWS and row["cast_position"] in ACCESS:
                for at in (row["line"], row["statement_line"]):
                    access_lines[row["function"]].add(at)
    transitions = collections.Counter()
    removed, added = collections.Counter(), collections.Counter()
    moves, attribution = [], collections.Counter()
    for key in sorted(set(before) | set(after), key=str):
        old = before.get(key, ("absent", collections.Counter()))
        new = after.get(key, ("absent", collections.Counter()))
        if old == new:
            continue
        lost = old[1] - new[1]
        gained = new[1] - old[1]
        for (_, description), count in lost.items():
            removed[TEMPLATE.sub("`_`", description)] += count
        for (_, description), count in gained.items():
            added[TEMPLATE.sub("`_`", description)] += count
        function = key[2]
        labels = set(hooks.get(function, set()))
        if any(line in access_lines.get(function, set()) for (line, _) in lost):
            labels.add("H-V")
        if not labels:
            labels.add("other")
        if old[0] != new[0]:
            transitions["%s -> %s" % (old[0], new[0])] += 1
        for label in labels:
            attribution[label] += 1
        moves.append({
            "unit": key[0], "occurrence": key[1], "function": function, "row": key[3],
            "status": [old[0], new[0]], "labels": sorted(labels),
            "lost": sorted("%s: %s" % (line, d) for (line, d) in lost.elements()),
            "gained": sorted("%s: %s" % (line, d) for (line, d) in gained.elements()),
        })
    summary = {
        "transitions": dict(transitions),
        "status_moves": [m for m in moves if m["status"][0] != m["status"][1]],
        "removed": dict(removed.most_common()),
        "added": dict(added.most_common()),
        "attribution": dict(attribution),
        "moving_rows": len(moves),
        "moves": moves,
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: summary[k] for k in ("transitions", "attribution", "moving_rows")}))
    for move in summary["status_moves"]:
        print("%s::%s %s -> %s %s" % (move["unit"], move["function"], move["status"][0],
                                      move["status"][1], ",".join(move["labels"])))


if __name__ == "__main__":
    main()
