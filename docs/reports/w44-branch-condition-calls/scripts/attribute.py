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
import gzip
import json
import pathlib
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"


NOT_A_CALLEE = {
    "sizeof", "_Alignof", "alignof", "__alignof", "__alignof__", "_Generic", "__typeof",
    "__typeof__", "typeof", "__builtin_offsetof", "__builtin_types_compatible_p",
    "__builtin_va_arg", "__extension__",
}


def rule_shape(tokens):
    """`return_call::whole_expression_call` plus `NOT_A_CALLEE`, over the
    analyzer's own token view: an identifier, `(`, and the matching `)` as the
    last token. No parenthesis stripping and no cast skipping, exactly as the
    Rust rule reads it."""
    if len(tokens) < 3 or tokens[1] != "(":
        return False
    first = tokens[0]
    if not (first[:1].isalpha() or first[:1] == "_") or first in NOT_A_CALLEE:
        return False
    depth = 0
    for index in range(1, len(tokens)):
        if tokens[index] == "(":
            depth += 1
        elif tokens[index] == ")":
            depth -= 1
            if depth == 0:
                return index == len(tokens) - 1
    return False


def lowered_family(leaf):
    """The census forecast family: A or B without a cast."""
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
    def load(path):
        data = pathlib.Path(path).read_bytes()
        return json.loads(gzip.decompress(data) if path.endswith(".gz") else data)

    before = load(sys.argv[1])
    after = load(sys.argv[2])
    census = load(sys.argv[3])
    out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1]) if "--json" in sys.argv else None

    bu, bfields, bf, bl = index(before)
    au, afields, af, al = index(after)

    # The attribution key is the rule's own shape predicate over the BEFORE
    # token view; the census family forecast is compared separately.
    lowered_coordinates = collections.Counter()
    forecast_coordinates = collections.Counter()
    for leaf in census["leaves"]:
        coordinate = (leaf["library"], leaf["file"], leaf["function"], leaf["line"])
        if rule_shape(leaf["expression"].split(" ")):
            lowered_coordinates[coordinate] += 1
        if lowered_family(leaf):
            forecast_coordinates[coordinate] += 1

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
    exchanges = [
        {
            "coordinate": list(coordinate),
            "origin": next(key[6] for key in removed_lowered if key[:4] == coordinate),
            "added": sorted(key[5] for key in added if key[:4] == coordinate),
        }
        for coordinate in sorted(removed_coordinates)
    ]
    absorbed = []
    for coordinate in sorted(removed_coordinates - coordinates_with_addition):
        existing = [key for key in al if key[:4] == coordinate and key[4] == "unsupported"]
        if existing:
            absorbed.append({"coordinate": list(coordinate), "kept": [key[5] for key in existing]})
        else:
            unattributed.append({"kind": "removed without replacement", "coordinate": list(coordinate)})

    census_lowered = sum(lowered_coordinates.values())
    removed_lowered_total = sum(removed_lowered.values())
    # A rule-shape leaf that survives is one whose synthetic assignment the
    # classifier did not refuse; it must be listed, never silently accepted.
    rule_shape_kept = sorted(
        list(coordinate) for coordinate in lowered_coordinates if coordinate not in removed_coordinates
    )
    forecast = {
        "forecast_lowered": sum(forecast_coordinates.values()),
        "measured_lowered": removed_lowered_total,
        "lowered_but_not_forecast": sorted(
            list(c) for c in removed_coordinates if c not in forecast_coordinates
        ),
        "forecast_but_not_lowered": sorted(
            list(c) for c in forecast_coordinates if c not in removed_coordinates
        ),
    }

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
        "rule_shape_leaves_before": census_lowered,
        "rule_shape_kept": rule_shape_kept,
        "forecast": forecast,
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
        "exchanges": exchanges,
    }
    text = json.dumps(result, indent=1)
    if out:
        out.write_text(text + "\n")
    summary = {
        k: v
        for k, v in result.items()
        if k not in ("absorbed", "unattributed", "unit_field_changes", "exchanges")
    }
    summary["unattributed_count"] = len(unattributed)
    summary["unit_field_change_count"] = len(unit_field_changes)
    print(json.dumps(summary, indent=1))
    return 0 if result["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
