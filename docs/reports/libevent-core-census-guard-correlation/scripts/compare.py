#!/usr/bin/env python3
"""compare.py BASE_REPORT TIP_REPORT [--out FILE]

Row-by-row comparison of two `libevent-core` configured-audit reports
(`quirin-audit CONFIG.json` resource reports). For each unit (keyed by
library and file): status, reason code, enumeration (completeness, refusal
grounds, absent_from_raw_scan), source_functions, not_projected; for each
function row (keyed by unit and function): status and the full row content.
Every difference is listed; nothing is summed away. Totals are counted from
each report's own unit rows.
"""
import argparse
import json
from collections import Counter


def units(path):
    report = json.load(open(path))
    return report, {(u["library"], u["file"]): u for u in report["units"]}


def totals(rows):
    return dict(Counter(u["status"] for u in rows.values()))


def census(unit):
    e = unit.get("enumeration", {})
    return {
        "completeness": e.get("completeness"),
        "refusals": [r["ground"] for r in e.get("refusals", [])],
        "absent_from_raw_scan": e.get("absent_from_raw_scan", []),
        "unanalyzable": e.get("unanalyzable", []),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("base")
    parser.add_argument("tip")
    parser.add_argument("--out")
    args = parser.parse_args()
    base_report, base = units(args.base)
    tip_report, tip = units(args.tip)
    result = {
        "base": args.base,
        "tip": args.tip,
        "totals": {"base": totals(base), "tip": totals(tip)},
        "absent_from_raw_scan_total": {
            "base": sum(len(census(u)["absent_from_raw_scan"]) for u in base.values()),
            "tip": sum(len(census(u)["absent_from_raw_scan"]) for u in tip.values()),
        },
        "refused_units": {
            "base": sorted(k[1] for k, u in base.items() if census(u)["completeness"] == "refused"),
            "tip": sorted(k[1] for k, u in tip.items() if census(u)["completeness"] == "refused"),
        },
        "unmatched_units": {
            "base_only": sorted(k[1] for k in base.keys() - tip.keys()),
            "tip_only": sorted(k[1] for k in tip.keys() - base.keys()),
        },
        "top_level_keys_changed": sorted(
            k for k in set(base_report) | set(tip_report)
            if k != "units" and base_report.get(k) != tip_report.get(k)
        ),
        "units": [],
    }
    for key in sorted(base.keys() & tip.keys()):
        b, t = base[key], tip[key]
        if b == t:
            continue
        entry = {"unit": f"{key[0]}::{key[1]}", "changed_fields": sorted(
            f for f in set(b) | set(t) if b.get(f) != t.get(f))}
        for field in ("status", "reason_code", "reason"):
            if b.get(field) != t.get(field):
                entry[field] = {"base": b.get(field), "tip": t.get(field)}
        if census(b) != census(t):
            entry["enumeration"] = {"base": census(b), "tip": census(t)}
        for field in ("source_functions", "not_projected"):
            bs, ts = set(b.get(field, [])), set(t.get(field, []))
            if bs != ts:
                entry[field] = {"added": sorted(ts - bs), "removed": sorted(bs - ts),
                                "base_count": len(bs), "tip_count": len(ts)}
        bf = {f["function"]: f for f in b.get("functions", [])}
        tf = {f["function"]: f for f in t.get("functions", [])}
        rows = []
        for name in sorted(bf.keys() | tf.keys()):
            x, y = bf.get(name), tf.get(name)
            if x == y:
                continue
            rows.append({
                "function": name,
                "status": {"base": x and x["status"], "tip": y and y["status"]},
                "moved": (x and x["status"]) != (y and y["status"]),
                "changed_fields": sorted(
                    f for f in set(x or {}) | set(y or {}) if (x or {}).get(f) != (y or {}).get(f)),
            })
        entry["function_rows_changed"] = rows
        result["units"].append(entry)
    result["function_rows_moved"] = sum(
        1 for u in result["units"] for r in u["function_rows_changed"] if r["moved"])
    result["unit_rows_moved"] = sum(1 for u in result["units"] if "status" in u)
    text = json.dumps(result, indent=1) + "\n"
    if args.out:
        open(args.out, "w").write(text)
    print(text)


if __name__ == "__main__":
    main()
