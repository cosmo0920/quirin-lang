#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: every moving function between two
route reports, with the leaves it lost and gained.

Usage: compare.py BEFORE AFTER OUT.json

Rows are keyed by (unit file, the unit's occurrence of that file, function,
row index within the unit), so the system and slice routes' per-configuration
units are compared one for one. A
function moves when its status or its multiset of leaf descriptions changes.
`transitions` counts status moves; `removed` / `added` total the leaf
descriptions (backquoted names masked into families); `attribution` puts each
moving row under the card family it lost (`noreturn` for `rd_kafka_crash`,
`tail:CALLEE`), or `other` when it lost none.
"""
import collections
import json
import pathlib
import re
import sys

TEMPLATE = re.compile(r"`[^`]*`")
TAIL = re.compile(r"^arguments beyond the declared parameters of `(printf|fprintf|snprintf|mk_print|flb_log_print)` are outside the resource projection$")
CRASH = "call to uncontracted function `rd_kafka_crash`"


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def rows(report):
    out = {}
    units = collections.Counter()
    for unit in report["units"]:
        units[unit["file"]] += 1  # the system and slice routes repeat a file per configuration
        seen = collections.Counter()
        for entry in unit["functions"]:
            seen[entry["function"]] += 1
            key = (unit["file"], units[unit["file"]], entry["function"], seen[entry["function"]])
            out[key] = (entry["status"], [
                f"{d['category']}: {d.get('description', '')}" for d in entry.get("diagnostics") or []])
    return out


def main() -> int:
    before, after = rows(load(sys.argv[1])), rows(load(sys.argv[2]))
    moves, transitions = [], collections.Counter()
    removed, added = collections.Counter(), collections.Counter()
    attribution = collections.defaultdict(list)
    for key in sorted(set(before) | set(after)):
        old = before.get(key, (None, []))
        new = after.get(key, (None, []))
        if old[0] == new[0] and sorted(old[1]) == sorted(new[1]):
            continue
        gone = collections.Counter(old[1]) - collections.Counter(new[1])
        came = collections.Counter(new[1]) - collections.Counter(old[1])
        for leaf, n in gone.items():
            removed[TEMPLATE.sub("`X`", leaf)] += n
        for leaf, n in came.items():
            added[TEMPLATE.sub("`X`", leaf)] += n
        families = set()
        for leaf in gone:
            text = leaf.split(": ", 1)[1]
            if text == CRASH:
                families.add("noreturn")
            match = TAIL.match(text)
            if match:
                families.add("tail:" + match.group(1))
        label = "::".join(str(k) for k in key)
        transitions[f"{old[0]} -> {new[0]}"] += 1
        row = {"function": label, "before": old[0], "after": new[0],
               "families": sorted(families) or ["other"],
               "removed": dict(gone), "added": dict(came)}
        moves.append(row)
        for family in row["families"]:
            attribution[family].append(label)
    out = {
        "transitions": dict(transitions),
        "status_moves": [m for m in moves if m["before"] != m["after"]],
        "removed": dict(removed.most_common()),
        "added": dict(added.most_common()),
        "attribution": {k: len(v) for k, v in sorted(attribution.items())},
        "moves": moves,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: out[k] for k in ("transitions", "attribution", "removed", "added")}, indent=1))
    for move in out["status_moves"]:
        print("STATUS", move["function"], move["before"], "->", move["after"], move["families"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
