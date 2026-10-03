#!/usr/bin/env python3
"""attribution.py BEFORE AFTER AFTER-PARITY.jsonl OUT.json -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, task 1: rule 12k's re-attribution
measured on its own.

Asserts what the rule promises and reports it: between BEFORE (the base
binary) and AFTER (the base plus rule 12k only),

* no function's status moves (any move is printed and the exit is 1);
* every changed leaf is one `indirect memory access is outside the resource
  projection` leaf that became one ``call to uncontracted function `F` ``
  leaf at the same line and origin (anything else is printed, exit 1).

Then, over the AFTER parity stream's rule-12k rows (`position:
"uncontracted-access"`, the last row per (function, line, form)), the
readings by answer, and per callee the leaves re-attributed and the functions
whose every leaf is now a re-attributed one or another uncontracted-call leaf.

Rows are keyed as in compare.py: (unit file, the unit's occurrence of that
file, function, row index within the unit).
"""
import collections
import json
import pathlib
import re
import sys

IND = "indirect memory access is outside the resource projection"
UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")


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
            out[key] = (entry["status"], [
                (d.get("line"), d.get("origin"), d.get("category"), d.get("description", ""))
                for d in entry.get("diagnostics") or []])
    return out


def main() -> int:
    before, after = rows(load(sys.argv[1])), rows(load(sys.argv[2]))
    problems = []
    moved_leaves = collections.Counter()
    per_callee_functions = collections.defaultdict(set)
    functions_changed = 0
    if set(before) != set(after):
        problems.append("row keys differ")
    for key in sorted(set(before) & set(after), key=str):
        (old_status, old), (new_status, new) = before[key], after[key]
        if old_status != new_status:
            problems.append("status %s %s -> %s" % (key, old_status, new_status))
        if old == new:
            continue
        functions_changed += 1
        gained = collections.Counter(new) - collections.Counter(old)
        for leaf, count in gained.items():
            match = UNCONTRACTED.match(leaf[3])
            if match:
                moved_leaves[match.group(1)] += count
                per_callee_functions[match.group(1)].add(key)
    problems += recheck(before, after)
    readings = collections.Counter()
    last = {}
    with open(sys.argv[3], encoding="utf-8") as stream:
        for line in stream:
            if '"uncontracted-access"' not in line:
                continue
            row = json.loads(line)
            last[(row["function"], row["line"], row["form"], " ".join(row["texts"]))] = row
    for row in last.values():
        answer = row["answer"]
        if not row["attributed"] and answer.startswith("call to "):
            answer = re.sub(r"`[^`]*`", "`_`", answer)
        readings[(row["form"], answer)] += 1
    record = {
        "status_moves": [p for p in problems if p.startswith("status")],
        "problems": problems,
        "functions_with_changed_leaves": functions_changed,
        "leaves_reattributed": sum(moved_leaves.values()),
        "leaves_by_callee": dict(moved_leaves.most_common()),
        "functions_by_callee": {k: len(v) for k, v in sorted(
            per_callee_functions.items(), key=lambda item: -len(item[1]))},
        "readings": {"%s: %s" % key: n for key, n in readings.most_common()},
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: record[k] for k in (
        "status_moves", "functions_with_changed_leaves", "leaves_reattributed")}))
    print("problems:", len(problems))
    for problem in problems[:20]:
        print(" ", problem)
    return 1 if problems else 0


def recheck(before, after):
    problems = []
    for key in sorted(set(before) & set(after), key=str):
        old, new = collections.Counter(before[key][1]), collections.Counter(after[key][1])
        if old == new:
            continue
        lost, gained = old - new, new - old
        lost_by_place = collections.Counter()
        for leaf, count in lost.items():
            if leaf[3] != IND:
                problems.append("lost %s %r" % (key, leaf))
            lost_by_place[leaf[:3]] += count
        for leaf, count in gained.items():
            if not UNCONTRACTED.match(leaf[3]):
                problems.append("gained %s %r" % (key, leaf))
            lost_by_place[leaf[:3]] -= count
        if any(lost_by_place.values()):
            problems.append("unbalanced %s" % (key,))
    return problems


if __name__ == "__main__":
    raise SystemExit(main())
