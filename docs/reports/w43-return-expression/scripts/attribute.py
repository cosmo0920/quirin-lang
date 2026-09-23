#!/usr/bin/env python3
"""Attribute every delta between a BEFORE and an AFTER corpus diagnostics run.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS CENSUS_JSON [--json OUT]

CENSUS_JSON is `return_census.py --json` output over the BEFORE run: it
classifies every BEFORE return-expression leaf into a shape family. The card's
rule lowers exactly the whole-call families A and B *without* a cast.

A delta is attributed only if it is one of:

* `removed_lowered` — a BEFORE return-expression leaf the census classifies in
  the lowered families;
* `added_exchange` — an AFTER `unsupported` leaf at the internal coordinate
  `(unit, function, line)` of a `removed_lowered` leaf, which that coordinate
  did not carry BEFORE;
* `absorbed` — a `removed_lowered` coordinate that gains no new leaf because
  the classifier's refusal is identical to a leaf the coordinate already had
  (the diagnostics are a set per function).

Any other difference — a removed leaf outside the lowered families, an added
leaf at another coordinate or in another category, a status move, a unit
field change, a diagnostic-count change the leaves do not explain — is
reported as UNATTRIBUTED and makes the script exit 1.
"""

import collections
import json
import pathlib
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"


def lowered_family(leaf):
    return leaf["family"][:2] in ("A.", "B.") and "under a cast" not in leaf["detail"]


def index(data):
    units, functions, leaves = {}, {}, collections.Counter()
    unit_fields = {}
    for unit in data["units"]:
        ukey = (unit["library"], unit["file"])
        units[ukey] = unit.get("status")
        unit_fields[ukey] = {k: v for k, v in unit.items() if k not in ("functions", "diagnostic_counts")}
        for entry in unit.get("functions", []):
            fkey = (*ukey, entry["function"])
            functions[fkey] = entry.get("status")
            for diagnostic in entry.get("diagnostics") or []:
                leaves[(*fkey, diagnostic["line"], diagnostic["category"], diagnostic["description"],
                        diagnostic.get("origin"))] += 1
    return units, unit_fields, functions, leaves


def main() -> int:
    before = json.loads(pathlib.Path(sys.argv[1]).read_text())
    after = json.loads(pathlib.Path(sys.argv[2]).read_text())
    census = json.loads(pathlib.Path(sys.argv[3]).read_text())
    out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1]) if "--json" in sys.argv else None

    bu, bfields, bf, bl = index(before)
    au, afields, af, al = index(after)

    lowered_coordinates = collections.Counter()
    for leaf in census["leaves"]:
        if lowered_family(leaf):
            lowered_coordinates[(leaf["library"], leaf["file"], leaf["function"], leaf["line"])] += 1

    removed = bl - al
    added = al - bl
    unattributed = []
    removed_lowered = collections.Counter()
    for key, count in removed.items():
        coordinate = key[:4]
        if key[5] == RETURN and key[4] == "unsupported" and coordinate in lowered_coordinates:
            removed_lowered[key] += count
        else:
            unattributed.append({"kind": "removed", "leaf": list(key), "count": count})
    removed_coordinates = {key[:4] for key in removed_lowered}
    exchange_descriptions = collections.Counter()
    added_exchange = 0
    for key, count in added.items():
        coordinate = key[:4]
        if key[4] == "unsupported" and coordinate in removed_coordinates:
            added_exchange += count
            exchange_descriptions[
                key[5] if "`" not in key[5] else key[5].split("`")[0] + "`…`" + key[5].split("`")[-1]
            ] += count
        else:
            unattributed.append({"kind": "added", "leaf": list(key), "count": count})
    coordinates_with_addition = {key[:4] for key in added if key[:4] in removed_coordinates}
    absorbed = []
    for coordinate in sorted(removed_coordinates - coordinates_with_addition):
        existing = [key for key in al if key[:4] == coordinate and key[4] == "unsupported"]
        if existing:
            absorbed.append({"coordinate": list(coordinate), "kept": [key[5] for key in existing]})
        else:
            unattributed.append({"kind": "removed without replacement", "coordinate": list(coordinate)})

    census_lowered = sum(lowered_coordinates.values())
    removed_lowered_total = sum(removed_lowered.values())
    if removed_lowered_total != census_lowered:
        unattributed.append(
            {"kind": "census mismatch", "census_lowered": census_lowered, "removed": removed_lowered_total}
        )

    function_moves = [
        {"function": list(k), "before": bf.get(k), "after": af.get(k)}
        for k in sorted(bf.keys() | af.keys()) if bf.get(k) != af.get(k)
    ]
    unit_moves = [
        {"unit": list(k), "before": bu.get(k), "after": au.get(k)}
        for k in sorted(bu.keys() | au.keys()) if bu.get(k) != au.get(k)
    ]
    unit_field_changes = [
        {"unit": list(k), "before": bfields.get(k), "after": afields.get(k)}
        for k in sorted(bfields.keys() | afields.keys()) if bfields.get(k) != afields.get(k)
    ]
    for move in function_moves:
        unattributed.append({"kind": "function status move", **move})
    for move in unit_moves:
        unattributed.append({"kind": "unit status move", **move})
    other_top = sorted(
        k for k in before.keys() | after.keys()
        if k not in ("units", "diagnostic_counts") and before.get(k) != after.get(k)
    )
    for key in other_top:
        unattributed.append({"kind": "top-level field", "field": key})

    functions_touched = {key[:3] for key in removed_lowered}
    sole_before = set()
    for leaf in census["leaves"]:
        if leaf["status"] == "Unknown" and leaf["sole"]:
            sole_before.add((leaf["library"], leaf["file"], leaf["function"]))

    def distinct_reasons(data, fkey):
        for unit in data["units"]:
            if (unit["library"], unit["file"]) != fkey[:2]:
                continue
            for entry in unit["functions"]:
                if entry["function"] == fkey[2]:
                    return {d["description"] for d in entry.get("diagnostics") or []}
        return set()

    still_return = sum(1 for f in sole_before if RETURN in distinct_reasons(after, f))
    result = {
        "diagnostic_counts": {"before": before["diagnostic_counts"], "after": after["diagnostic_counts"]},
        "function_counts": {
            "before": dict(collections.Counter(bf.values())),
            "after": dict(collections.Counter(af.values())),
        },
        "unit_counts": {
            "before": dict(collections.Counter(bu.values())),
            "after": dict(collections.Counter(au.values())),
        },
        "leaves_removed": sum(removed.values()),
        "leaves_added": sum(added.values()),
        "removed_lowered": removed_lowered_total,
        "census_lowered": census_lowered,
        "added_exchange": added_exchange,
        "absorbed_coordinates": len(absorbed),
        "absorbed": absorbed,
        "exchange_descriptions": dict(exchange_descriptions.most_common()),
        "functions_losing_a_return_leaf": len(functions_touched),
        "function_moves": function_moves,
        "unit_moves": unit_moves,
        "unit_field_changes": unit_field_changes,
        "sole_blocked_before": len(sole_before),
        "sole_blocked_still_carrying_the_return_refusal": still_return,
        "unattributed": unattributed,
        "attribution_complete": not unattributed and not unit_field_changes,
    }
    text = json.dumps(result, indent=1)
    if out:
        out.write_text(text + "\n")
    summary = {k: v for k, v in result.items() if k not in ("absorbed", "unattributed", "unit_field_changes")}
    summary["unattributed_count"] = len(unattributed)
    summary["unit_field_change_count"] = len(unit_field_changes)
    print(json.dumps(summary, indent=1))
    return 0 if result["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
