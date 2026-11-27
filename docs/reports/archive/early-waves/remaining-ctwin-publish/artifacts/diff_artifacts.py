#!/usr/bin/env python3
"""CTWIN-PUBLISH corpus artifact comparison (card-private measurement tool).

usage: diff_artifacts.py <left> <right> [--left-baseline] [--right-baseline]

`left`/`right` are either a produced reports directory or an archived
`artifacts/` directory (with `--*-baseline`). Prints a structural summary:
byte identity per artifact, unit verdict moves, per-function status moves and
per-diagnostic leaf moves.
"""

import gzip
import hashlib
import json
import os
import sys

FILES = {
    "diagnostics": ("resource-diagnostics.json", "diagnostics.json.gz"),
    "contracts": ("resource-diagnostics.contracts.json", "contracts.json.gz"),
    "chunk-contracts": (
        "resource-diagnostics.chunk-contracts.json",
        "chunk-contracts.json.gz",
    ),
    "source-effects": (
        "resource-diagnostics.source-effects.json",
        "source-effects.json.gz",
    ),
    "source": ("monkey-source.json", "source.json.gz"),
}


def load(root, key, baseline):
    plain, archived = FILES[key]
    path = os.path.join(root, archived if baseline else plain)
    if not os.path.exists(path):
        return None
    opener = gzip.open if baseline else open
    with opener(path, "rb") as handle:
        return handle.read()


def unit_key(unit):
    return f"{unit.get('library', '')}/{unit.get('file', '')}"


def leaves(unit):
    """Every diagnostic of a unit as a comparable tuple."""
    out = []
    for function in unit.get("functions", []):
        for diagnostic in function.get("diagnostics", []):
            out.append(
                (
                    function.get("function"),
                    diagnostic.get("line"),
                    diagnostic.get("category"),
                    diagnostic.get("description"),
                )
            )
    return out


def compare_units(left, right):
    lu = {unit_key(u): u for u in json.loads(left).get("units", [])}
    ru = {unit_key(u): u for u in json.loads(right).get("units", [])}
    verdict_moves, function_moves, leaf_moves = [], [], []
    for key in sorted(set(lu) | set(ru)):
        a, b = lu.get(key), ru.get(key)
        if a is None or b is None:
            verdict_moves.append({"unit": key, "left": bool(a), "right": bool(b)})
            continue
        if a.get("status") != b.get("status") or a.get("reason") != b.get("reason"):
            verdict_moves.append(
                {
                    "unit": key,
                    "left": a.get("status"),
                    "right": b.get("status"),
                    "left_reason": a.get("reason"),
                    "right_reason": b.get("reason"),
                }
            )
        la = {f.get("function"): f.get("status") for f in a.get("functions", [])}
        lb = {f.get("function"): f.get("status") for f in b.get("functions", [])}
        moves = {
            name: [la.get(name), lb.get(name)]
            for name in sorted(set(la) | set(lb))
            if la.get(name) != lb.get(name)
        }
        if moves:
            function_moves.append({"unit": key, "functions": moves})
        al, bl = leaves(a), leaves(b)
        if al != bl:
            removed = [x for x in al if x not in bl]
            added = [x for x in bl if x not in al]
            leaf_moves.append(
                {
                    "unit": key,
                    "removed": len(removed),
                    "added": len(added),
                    "removed_sample": removed[:6],
                    "added_sample": added[:6],
                }
            )
    return verdict_moves, function_moves, leaf_moves


def status_totals(payload):
    totals = {}
    for unit in json.loads(payload).get("units", []):
        totals[unit.get("status")] = totals.get(unit.get("status"), 0) + 1
    return totals


def main():
    left_root, right_root = sys.argv[1], sys.argv[2]
    left_baseline = "--left-baseline" in sys.argv
    right_baseline = "--right-baseline" in sys.argv
    report = {}
    for key in FILES:
        left = load(left_root, key, left_baseline)
        right = load(right_root, key, right_baseline)
        entry = {
            "left_sha256": hashlib.sha256(left).hexdigest() if left else None,
            "right_sha256": hashlib.sha256(right).hexdigest() if right else None,
            "identical": left == right,
        }
        if key == "diagnostics" and left and right:
            entry["left_totals"] = status_totals(left)
            entry["right_totals"] = status_totals(right)
        if not entry["identical"] and left and right and key != "source":
            try:
                verdicts, functions, leafs = compare_units(left, right)
                entry["unit_verdict_moves"] = verdicts
                entry["function_status_moves"] = functions
                entry["diagnostic_leaf_moves"] = leafs
                entry["changed_units"] = len(
                    {m["unit"] for m in leafs}
                    | {m["unit"] for m in verdicts}
                    | {m["unit"] for m in functions}
                )
            except (json.JSONDecodeError, AttributeError, TypeError) as error:
                entry["structural_comparison_error"] = str(error)
        report[key] = entry
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()
