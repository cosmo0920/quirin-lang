#!/usr/bin/env python3
"""CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (from CAST-FORMALIZATION's compare.py):
every moving function between two route reports, with the leaves it lost and
gained, attributed to the card's rules.

Usage: compare.py BEFORE AFTER AFTER-PARITY.jsonl OUT.json

Rows are keyed by (unit file, the unit's occurrence of that file, function,
row index within the unit), so the system and slice routes' per-configuration
units are compared one for one. A function moves when its status or its
multiset of leaves (line, description) changes.

Attribution, from the AFTER parity stream (the rules' own rows):
* `12i` -- a `return-call-access` row of the function answered admitted (rule
  12i read a refused return's call);
* `12j` -- a `typed-place` row of the function carries a rule-12j place
  admitted (a `.`-only scalar place on an automatic record object);
* `callee-summary` -- a lost or gained leaf names a callee whose own function
  moved to Safe in AFTER (its validated summary now applies);
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
    for line in open(sys.argv[3], encoding="utf-8"):
        if '"return-call-access"' in line:
            row = json.loads(line)
            if row.get("new_admitted"):
                hooks[row["function"]].add("12i")
        elif '"rule":"12j"' in line or '"rule": "12j"' in line:
            row = json.loads(line)
            if any(place.get("rule") == "12j" and place.get("admitted")
                   for place in row.get("places") or []):
                hooks[row["function"]].add("12j")
    safe_now = {key[2] for key, (status, _) in after.items()
                if status == "Safe" and before.get(key, ("absent",))[0] != "Safe"}
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
        if any(f"`{callee}`" in description for (_, description) in list(lost) + list(gained)
               for callee in safe_now if callee != function):
            labels.add("callee-summary")
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
