#!/usr/bin/env python3
"""compare.py BEFORE AFTER AFTER-PARITY.jsonl OUT.json [BEFORE-PARITY.jsonl] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (from the first round's
compare.py): every moving function between two route reports, with the leaves
it lost and gained, attributed to the card's rules and declarations.

Rows are keyed by (unit file, the unit's occurrence of that file, function,
row index within the unit), so the system and slice routes' per-configuration
units are compared one for one. A function moves when its status or its
multiset of leaves (line, description) changes.

Attribution, from the AFTER parity stream (the rules' own rows) and the AFTER
row's `contract_dependencies`:
* `14a` / `14b` -- a `residual-access` row of the function answered admitted;
* `14c` / `14d` -- a `call-argument` row of the function admitted an actual
  the BEFORE parity stream never admitted at that line (given), spelled
  `& ... [ ... ]` (an element address, 14c) or otherwise (an interior offset,
  14d); without the BEFORE stream, only the 14c spelling is labelled;
* `errno-print` -- the AFTER row rests on `cio_errno_print` or
  `ctr_errno_print` (the two declarations this card adds);
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
DECLARED = {"cio_errno_print", "ctr_errno_print"}


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
            declarations = set((entry.get("contract_dependencies") or {}).get("declarations") or [])
            out[key] = (entry["status"], leaves, declarations)
    return out


def admitted_actuals(path):
    out = set()
    for line in open(path, encoding="utf-8"):
        if '"call-argument"' not in line or '"new_admitted":true' not in line.replace(" ", ""):
            continue
        row = json.loads(line)
        if row.get("position") == "call-argument" and row.get("new_admitted"):
            out.add((row["function"], row.get("line"), tuple(row.get("texts") or [])))
    return out


def main():
    before, after = rows(load(sys.argv[1])), rows(load(sys.argv[2]))
    hooks = collections.defaultdict(set)
    for line in open(sys.argv[3], encoding="utf-8"):
        if '"residual-access"' in line:
            row = json.loads(line)
            if row.get("position") == "residual-access" and row.get("new_admitted"):
                hooks[row["function"]].add(row.get("rule"))
    new_actuals = admitted_actuals(sys.argv[3])
    if len(sys.argv) > 5:
        new_actuals -= admitted_actuals(sys.argv[5])
    for function, _, texts in new_actuals:
        element = texts[:1] == ("&",) and "[" in texts
        if element:
            hooks[function].add("14c")
        elif len(sys.argv) > 5:
            hooks[function].add("14d")
    safe_now = {key[2] for key, (status, _, _) in after.items()
                if status == "Safe" and before.get(key, ("absent",))[0] != "Safe"}
    transitions = collections.Counter()
    removed, added = collections.Counter(), collections.Counter()
    moves, attribution = [], collections.Counter()
    empty = ("absent", collections.Counter(), set())
    for key in sorted(set(before) | set(after), key=str):
        old = before.get(key, empty)
        new = after.get(key, empty)
        if old[:2] == new[:2]:
            continue
        lost = old[1] - new[1]
        gained = new[1] - old[1]
        for (_, description), count in lost.items():
            removed[TEMPLATE.sub("`_`", description)] += count
        for (_, description), count in gained.items():
            added[TEMPLATE.sub("`_`", description)] += count
        function = key[2]
        labels = set(hooks.get(function, set()))
        if new[2] & DECLARED:
            labels.add("errno-print")
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
    status_attribution = collections.Counter(
        label for move in moves if move["status"][0] != move["status"][1] for label in move["labels"])
    summary = {
        "transitions": dict(transitions),
        "status_moves": [m for m in moves if m["status"][0] != m["status"][1]],
        "removed": dict(removed.most_common()),
        "added": dict(added.most_common()),
        "attribution": dict(attribution),
        "status_attribution": dict(status_attribution),
        "moving_rows": len(moves),
        "moves": moves,
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: summary[k] for k in ("transitions", "attribution", "status_attribution",
                                              "moving_rows")}))
    for move in summary["status_moves"]:
        print("%s::%s %s -> %s %s" % (move["unit"], move["function"], move["status"][0],
                                      move["status"][1], ",".join(move["labels"])))


if __name__ == "__main__":
    main()
