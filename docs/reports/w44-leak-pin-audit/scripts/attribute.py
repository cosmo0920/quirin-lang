#!/usr/bin/env python3
"""Attribute every leaf, function and unit delta of the AFTER arbiter.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS CLASSIFICATION.json OUT.json

A leaf is (unit, function, category, preprocessed line, description). Every
removed or added leaf is listed. A removed `leak` leaf must be a class (b) leaf
of the frozen classification (its mechanism is its attribution); a removed
class (a) or (c) leak is reported as collateral. Every other delta is listed by
category and description for the report to attribute by reading. Function and
unit status moves are listed with the leaf deltas of the function or unit.
Exits 1 when a removed leak is not class (b), so collateral cannot pass
silently. Given the refused hand-on overlay (`handoff-classification.json`), a
removed leak must be one the overlay marks `withdrawn`, its `attribution` is
its mechanism, and a leaf marked `withdrawn` that stays is listed as missed.
"""

import collections
import gzip
import json
import pathlib
import re
import sys


def load(path):
    raw = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        raw = gzip.decompress(raw)
    return json.loads(raw)


def leaves(document):
    out = collections.Counter()
    status = {}
    units = {}
    for unit in document["units"]:
        units[unit["file"]] = unit["status"]
        for function in unit["functions"]:
            status[(unit["file"], function["function"])] = function["status"]
            for d in function.get("diagnostics") or []:
                out[(unit["file"], function["function"], d["category"], d["line"], d.get("origin"), d["description"])] += 1
    return out, status, units


def acquired(description):
    match = re.search(r"acquired at (\S+)$", description)
    return match.group(1) if match else "?"


def main() -> int:
    before, before_status, before_units = leaves(load(sys.argv[1]))
    after, after_status, after_units = leaves(load(sys.argv[2]))
    classification = load(sys.argv[3])["classification"]
    by_key = {
        (e["unit"], e["function"], e["line"], e["acquired"]): e for e in classification
    }
    removed = before - after
    added = after - before
    rows = []
    collateral = []
    for (unit, function, category, line, origin, description), count in sorted(removed.items()):
        row = {
            "direction": "removed",
            "unit": unit,
            "function": function,
            "category": category,
            "line": line,
            "origin": origin,
            "description": description,
            "count": count,
        }
        if category == "leak":
            entry = by_key.get((unit, function, line, acquired(description)))
            if entry and "attribution" in entry:
                row["class"], row["mechanism"] = entry["attribution"].split(":", 1)
                row["prior"] = f"{entry['prior_class']}:{entry['prior_mechanism']}"
                if not entry["withdrawn"]:
                    collateral.append(row)
            else:
                row["class"] = entry["class"] if entry else "?"
                row["mechanism"] = entry["mechanism"] if entry else "?"
                if not entry or entry["class"] != "b":
                    collateral.append(row)
        rows.append(row)
    for (unit, function, category, line, origin, description), count in sorted(added.items()):
        rows.append(
            {
                "direction": "added",
                "unit": unit,
                "function": function,
                "category": category,
                "line": line,
                "origin": origin,
                "description": description,
                "count": count,
            }
        )
    remaining = collections.Counter(
        (unit, function, line, acquired(description))
        for (unit, function, category, line, origin, description), count in after.items()
        if category == "leak"
        for _ in range(count)
    )
    missed = [
        {"unit": e["unit"], "function": e["function"], "line": e["line"], "acquired": e["acquired"],
         "attribution": e.get("attribution")}
        for e in classification
        if e.get("withdrawn") and remaining[(e["unit"], e["function"], e["line"], e["acquired"])] > 0
    ]
    moves = [
        {"unit": u, "function": f, "before": before_status.get((u, f)), "after": after_status.get((u, f))}
        for (u, f) in sorted(set(before_status) | set(after_status))
        if before_status.get((u, f)) != after_status.get((u, f))
    ]
    unit_moves = [
        {"unit": u, "before": before_units.get(u), "after": after_units.get(u)}
        for u in sorted(set(before_units) | set(after_units))
        if before_units.get(u) != after_units.get(u)
    ]

    def tally(counter):
        totals = collections.Counter()
        for key, count in counter.items():
            totals[key[2]] += count
        return dict(sorted(totals.items()))

    def status_counts(statuses):
        return dict(sorted(collections.Counter(statuses.values()).items()))

    document = {
        "leaf_totals": {"before": tally(before), "after": tally(after)},
        "removed_by_category": tally(removed),
        "added_by_category": tally(added),
        "removed_leaks_by_mechanism": dict(
            sorted(
                collections.Counter(
                    f"{r['class']}:{r['mechanism']}" for r in rows if r["direction"] == "removed" and r["category"] == "leak"
                ).items()
            )
        ),
        "collateral_leak_removals": collateral,
        "predicted_withdrawals_missed": missed,
        "functions": {"before": status_counts(before_status), "after": status_counts(after_status)},
        "units": {"before": status_counts(before_units), "after": status_counts(after_units)},
        "function_moves": moves,
        "unit_moves": unit_moves,
        "rows": rows,
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(document, indent=1) + "\n")
    print(
        json.dumps(
            {k: document[k] for k in ("leaf_totals", "removed_by_category", "added_by_category", "removed_leaks_by_mechanism", "functions", "units")},
            indent=1,
        )
    )
    print(
        f"function moves {len(moves)}, unit moves {len(unit_moves)}, collateral {len(collateral)}, "
        f"missed {len(missed)}"
    )
    return 1 if collateral else 0


if __name__ == "__main__":
    sys.exit(main())
