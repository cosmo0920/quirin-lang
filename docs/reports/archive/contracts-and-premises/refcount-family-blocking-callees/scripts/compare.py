#!/usr/bin/env python3
"""compare.py BEFORE AFTER OUT.json -- REFCOUNT-FAMILY-BLOCKING-CALLEES (adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1) (from
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS' compare.py and attribute_contracts.py):
every moving function between two route reports (a run directory or its
resource-diagnostics.json), with the leaves it lost and gained, attributed to
the card's declarations.

Rows are keyed by (unit file, the unit's occurrence of that file, function,
row index within the unit). A function moves when its status or its multiset
of leaves (line, description) changes. Labels:
* `callee:F` -- a lost leaf names a callee F this card declared;
* `callee-summary:G` -- otherwise, a lost or gained leaf names a function G
  that itself moved to Safe in AFTER (its validated summary now applies);
* `other` -- none of these (read by hand, in the report).
"""
import collections
import json
import pathlib
import re
import sys

DECLARED = [
    # REFCOUNT-FAMILY-BLOCKING-CALLEES: plain declarations between a
    # reference's acquisition and its drop
    "rd_kafka_q_pop",  # declared in a1 only, withdrawn before a2
    "rd_kafka_q_wait_result", "rd_kafka_q_purge0",
    "rd_kafka_toppar_needs_query", "rd_kafka_toppar_fetch_backoff",
]
TEMPLATE = re.compile(r"`[^`]*`")


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
    statuses = [collections.Counter(s for s, _ in table.values()) for table in (before, after)]
    safe_now = {key[2] for key, (status, _) in after.items()
                if status == "Safe" and before.get(key, ("absent",))[0] != "Safe"}
    transitions, by_label = collections.Counter(), collections.Counter()
    removed, added = collections.Counter(), collections.Counter()
    moves = []
    for key in sorted(set(before) | set(after), key=str):
        old = before.get(key, ("absent", collections.Counter()))
        new = after.get(key, ("absent", collections.Counter()))
        if old == new:
            continue
        lost, gained = old[1] - new[1], new[1] - old[1]
        for (_, description), count in lost.items():
            removed[TEMPLATE.sub("`_`", description)] += count
        for (_, description), count in gained.items():
            added[TEMPLATE.sub("`_`", description)] += count
        function = key[2]
        lost_text = " ".join(d for (_, d) in lost)
        labels = sorted({"callee:" + name for name in DECLARED if "`%s`" % name in lost_text})
        if not labels:
            texts = " ".join(d for (_, d) in list(lost) + list(gained))
            labels = sorted({"callee-summary:" + name for name in safe_now
                             if name != function and "`%s`" % name in texts})
        if not labels:
            labels = ["other"]
        if old[0] != new[0]:
            transitions["%s -> %s" % (old[0], new[0])] += 1
            for label in labels:
                by_label[label] += 1
        moves.append({
            "unit": key[0], "occurrence": key[1], "function": function, "row": key[3],
            "status": [old[0], new[0]], "labels": labels,
            "lost": sorted("%s: %s" % (line, d) for (line, d) in lost.elements()),
            "gained": sorted("%s: %s" % (line, d) for (line, d) in gained.elements()),
        })
    summary = {
        "statuses_before": dict(statuses[0]), "statuses_after": dict(statuses[1]),
        "transitions": dict(transitions),
        "status_moves_by_label": dict(by_label.most_common()),
        "removed": dict(removed.most_common()), "added": dict(added.most_common()),
        "moving_rows": len(moves),
        "status_moves": [m for m in moves if m["status"][0] != m["status"][1]],
        "moves": moves,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: summary[k] for k in ("statuses_before", "statuses_after", "transitions",
                                              "moving_rows")}))
    print(json.dumps(summary["status_moves_by_label"]))
    print("removed", json.dumps(summary["removed"]))
    print("added", json.dumps(summary["added"]))
    for move in summary["status_moves"]:
        print("%s::%s %s -> %s %s" % (move["unit"], move["function"], move["status"][0],
                                      move["status"][1], ",".join(move["labels"])))


if __name__ == "__main__":
    main()
