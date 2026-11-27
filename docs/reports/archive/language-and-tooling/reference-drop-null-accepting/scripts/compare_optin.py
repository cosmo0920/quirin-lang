#!/usr/bin/env python3
"""Compare two `quirin-audit current` resource reports function row by row
(card REFERENCE-DROP-NULL-ACCEPTING, the opt-in OpenSSL run).

    python3 compare_optin.py BASE.json TIP.json [OUT.json]

A row is keyed by (unit file, unit index, function). For each
row present on either side it records the status and the diagnostics (kind and
text, line-independent text kept as written); a row whose status differs is a
move, one whose diagnostics differ only is a content change. Reads files only.
"""
import json
import sys
from collections import Counter


def rows(path):
    report = json.load(open(path))
    out = {}
    for index, unit in enumerate(report["units"]):
        for row in unit.get("functions", []):
            key = f"{unit['file']}#{index}::{row['function']}"
            out[key] = {
                "status": row.get("status", unit.get("status")),
                "diagnostics": sorted(json.dumps(d, sort_keys=True) for d in row.get("diagnostics", [])),
            }
    units = {f"{unit['file']}#{i}": unit.get("status")
             for i, unit in enumerate(report["units"])}
    return out, units


def main():
    base, base_units = rows(sys.argv[1])
    tip, tip_units = rows(sys.argv[2])
    moves, changed = [], []
    for key in sorted(set(base) | set(tip)):
        one, two = base.get(key), tip.get(key)
        if one is None or two is None:
            moves.append({"row": key, "base": one and one["status"], "tip": two and two["status"]})
            continue
        if one["status"] != two["status"]:
            moves.append({"row": key, "base": one["status"], "tip": two["status"]})
        elif one["diagnostics"] != two["diagnostics"]:
            changed.append({
                "row": key,
                "removed": sorted(set(one["diagnostics"]) - set(two["diagnostics"])),
                "added": sorted(set(two["diagnostics"]) - set(one["diagnostics"])),
            })
    result = {
        "base_rows": len(base), "tip_rows": len(tip),
        "base_status": Counter(r["status"] for r in base.values()),
        "tip_status": Counter(r["status"] for r in tip.values()),
        "base_units": base_units, "tip_units": tip_units,
        "moves": moves, "content_changes": changed,
    }
    text = json.dumps(result, indent=1, sort_keys=True, default=dict)
    if len(sys.argv) > 3:
        open(sys.argv[3], "w").write(text + "\n")
    print(json.dumps({k: result[k] for k in ("base_rows", "tip_rows", "base_status", "tip_status")},
                     default=dict))
    print(f"moves {len(moves)}, content changes {len(changed)}")


if __name__ == "__main__":
    main()
