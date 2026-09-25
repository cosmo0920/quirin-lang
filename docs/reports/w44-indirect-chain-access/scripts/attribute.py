#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: attribute every BEFORE -> AFTER leaf delta.

Usage: attribute.py BEFORE_RUN AFTER_RUN CHAIN_CENSUS_JSON OUT_JSON OUT_TXT

A leaf is `(library, file, function, line, category, description)` of
`resource-diagnostics.json`, counted with multiplicity. `CHAIN_CENSUS_JSON` is
`chain_census.py`'s output over the base census, whose `reach_leaves` names
every leaf the rule's recogniser admits, with its forecast arm.

Every removed and every added leaf is put in exactly one class:

Removed:

* `rule:clear` / `rule:flow` - a forecast leaf (its every census record at the
  key admitted), by arm;
* `downstream` - any other removal in a function that also lost a rule leaf;
* `unattributed` - anything else.

Added:

* `violation` - `invalid_release` / `use_after_release`;
* `exchange` - an `unknown_alias` at the (function, line) of a removed rule
  leaf: the flow half refusing a root or a link;
* `downstream` - any other addition in a function that also lost a rule leaf;
* `unattributed` - anything else.

The attribution is complete when both `unattributed` classes are empty.
"""

import collections
import json
import pathlib
import sys

VIOLATIONS = ("invalid_release", "use_after_release")


def leaves(document):
    out = collections.Counter()
    status = {}
    units = {}
    for unit in document["units"]:
        ukey = (unit.get("library"), unit["file"])
        units[ukey] = unit["status"]
        for function in unit["functions"]:
            fkey = ukey + (function["function"],)
            status[fkey] = function["status"]
            for d in function["diagnostics"]:
                out[fkey + (d["line"], d["category"], d["description"])] += 1
    return out, status, units


def name(fkey, line, description):
    return "/".join(str(p) for p in fkey) + f":{line}:{description}"


def main():
    before_run, after_run = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    reach = json.loads(pathlib.Path(sys.argv[3]).read_text())["reach_leaves"]
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    before, before_status, before_units = leaves(json.load(open(before_run / "resource-diagnostics.json")))
    after, after_status, after_units = leaves(json.load(open(after_run / "resource-diagnostics.json")))
    removed = before - after
    added = after - before

    classes = collections.defaultdict(list)
    rule_functions = set()
    rule_coordinates = set()
    for leaf, count in sorted(removed.items(), key=str):
        fkey, line, category, description = leaf[:3], leaf[3], leaf[4], leaf[5]
        arm = reach.get(name(fkey, line, description))
        if arm is not None and category == "unsupported":
            rule_functions.add(fkey)
            rule_coordinates.add(fkey + (line,))
            classes[f"removed:rule:{arm}"].extend([leaf] * count)
    for leaf, count in sorted(removed.items(), key=str):
        fkey, line, category, description = leaf[:3], leaf[3], leaf[4], leaf[5]
        if reach.get(name(fkey, line, description)) is not None and category == "unsupported":
            continue
        cls = "removed:downstream" if fkey in rule_functions else "removed:unattributed"
        classes[cls].extend([leaf] * count)
    for leaf, count in sorted(added.items(), key=str):
        fkey, line, category = leaf[:3], leaf[3], leaf[4]
        if category in VIOLATIONS:
            cls = "added:violation"
        elif category == "unknown_alias" and fkey + (line,) in rule_coordinates:
            cls = "added:exchange"
        elif fkey in rule_functions:
            cls = "added:downstream"
        else:
            cls = "added:unattributed"
        classes[cls].extend([leaf] * count)

    moves = collections.Counter(
        (before_status.get(f), after_status.get(f))
        for f in set(before_status) | set(after_status)
        if before_status.get(f) != after_status.get(f)
    )
    unit_moves = collections.Counter(
        (before_units.get(u), after_units.get(u))
        for u in set(before_units) | set(after_units)
        if before_units.get(u) != after_units.get(u)
    )
    forecast_not_removed = sorted(
        leaf for leaf in reach
        if not any(
            name(l[:3], l[3], l[5]) == leaf for l in classes["removed:rule:clear"] + classes["removed:rule:flow"]
        )
    )
    categories = lambda counter: dict(collections.Counter(leaf[4] for leaf in counter.elements()))  # noqa: E731
    result = {
        "removed_total": sum(removed.values()),
        "added_total": sum(added.values()),
        "removed_by_category": categories(removed),
        "added_by_category": categories(added),
        "classes": {cls: len(items) for cls, items in sorted(classes.items())},
        "attribution_complete": not classes["removed:unattributed"] and not classes["added:unattributed"],
        "function_moves": {f"{a} -> {b}": n for (a, b), n in sorted(moves.items(), key=str)},
        "function_movers": sorted(
            "/".join(map(str, f)) + f": {before_status.get(f)} -> {after_status.get(f)}"
            for f in set(before_status) | set(after_status)
            if before_status.get(f) != after_status.get(f)
        ),
        "unit_moves": {f"{a} -> {b}": n for (a, b), n in sorted(unit_moves.items(), key=str)},
        "forecast_leaves_not_removed": forecast_not_removed,
        "leaves": {
            cls: ["/".join(map(str, leaf[:3])) + f":{leaf[3]}:{leaf[4]}:{leaf[5]}" for leaf in items]
            for cls, items in sorted(classes.items())
        },
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    lines = [
        "W44-INDIRECT-CHAIN-ACCESS attribution (BEFORE -> AFTER)",
        "",
        f"removed {result['removed_total']} {result['removed_by_category']}",
        f"added   {result['added_total']} {result['added_by_category']}",
        f"attribution_complete: {result['attribution_complete']}",
        "",
        "classes:",
    ]
    lines.extend(f"  {cls:24} {n:5}" for cls, n in result["classes"].items())
    lines.append("")
    lines.append(f"function moves: {result['function_moves']}")
    lines.extend("  " + mover for mover in result["function_movers"])
    lines.append(f"unit moves: {result['unit_moves']}")
    lines.append(f"forecast leaves not removed: {len(forecast_not_removed)}")
    lines.extend("  " + leaf for leaf in forecast_not_removed)
    lines.append("")
    for cls in sorted(classes):
        if cls.startswith("removed:rule"):
            continue
        lines.append(f"{cls}:")
        lines.extend("  " + leaf for leaf in result["leaves"][cls])
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:40]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
