#!/usr/bin/env python3
"""Card W43-INDEX-TRAVERSAL: attribute every BEFORE -> AFTER leaf delta.

Usage: attribute.py BEFORE_RUN AFTER_RUN CENSUS_RUN OUT_JSON OUT_TXT

A leaf is `(library, file, function, line, category, description)` of
`resource-diagnostics.json`. Every removed and every added leaf is put in
exactly one class; a leaf no rule explains lands in `unattributed`, which the
report must then explain by hand (the attribution is complete only when that
class is empty or each member is listed with its reason).

Removed leaves, by the base census record at the same (function, line):

* `rule:condition`, `rule:step`, `rule:gated-step` - the leaf's refusal is
  the recognised shape (`predict.py`'s lexical classes);
* `rule:scalar-alias` - an `unknown_alias` of a declared scalar, or of the
  source of a recognised copy at the line;
* `rule:unforecast` - a refusal of a family the rules intercept, at a line
  where the census recognised some but not all records (or none), which the
  rule admitted anyway;
* `summary` - a `call to uncontracted function F` leaf where `F` moved
  Unknown -> Safe and so gained a validated W4 summary;
* `downstream` - any other removal in a function whose leaves also moved by
  a rule (a call leaf a new summary decided, a node no longer reached, a
  leak whose refusal changed);
* `unattributed` - anything else.

Added leaves:

* `violation` - `invalid_release` / `use_after_release`;
* `exchange` - an `unknown_alias` at the (function, line) of a removed rule
  leaf: the flow half refusing a base;
* `exchange:scalar-join` - an `unknown_alias` naming a declared scalar
  elsewhere: the scalar-fact join left no binding for a generic copy to read;
* `leak` - a leak leaf;
* `downstream` - any other addition in a function with a rule removal;
* `unattributed` - anything else.
"""

import collections
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

# This card's own forecast module (the indirect card's scripts directory holds
# a `predict.py` of its own, so it is loaded by path).
_spec = importlib.util.spec_from_file_location("traversal_predict", HERE / "predict.py")
predict = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(predict)

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


def main():
    before_run, after_run, census_run = (pathlib.Path(p) for p in sys.argv[1:4])
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    before_doc = json.load(open(before_run / "resource-diagnostics.json"))
    after_doc = json.load(open(after_run / "resource-diagnostics.json"))
    before, before_status, before_units = leaves(before_doc)
    after, after_status, after_units = leaves(after_doc)

    _, _, unit_keys = census.load_document(census_run / "resource-diagnostics.json")
    stream = predict.load(census_run / "traversal-census.jsonl")
    per_seq = {}
    for seq, records in stream.items():
        keys = collections.defaultdict(list)
        names = collections.defaultdict(lambda: {"scalars": set(), "pointers": set()})
        for record in records:
            if record["kind"] == "function":
                names[record["function"]]["scalars"].update(record["scalars"])
                names[record["function"]]["pointers"].update(record["pointers"])
            else:
                keys[(record["function"], record["line"])].append(record)
        per_seq[seq] = (keys, names)
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l) in keys} for seq, (keys, _) in per_seq.items()}],
        unit_keys,
    )

    def census_label(leaf):
        library, file, function, line, category, description = leaf
        seq = assignment.get((library, file))
        keys, names = per_seq.get(seq, ({}, {}))
        records = keys.get((function, line), [])
        declared = names.get(function, {"scalars": set(), "pointers": set()})
        match = predict.CONDITION.fullmatch(description)
        if match:
            conditions = [
                r for r in records
                if r["kind"] == "condition" and r["position"] in predict.POSITION[match.group(1)]
            ]
            # `typed_bases` is `[]` for a condition whose only accesses are
            # `S.F` (no base): admitted, so the test is `is not None`. The
            # frozen forecast tested truthiness and so missed those; see the
            # report.
            if conditions and all(r["typed_bases"] is not None for r in conditions):
                return "rule:condition"
            return "rule:unforecast"
        if description in predict.CONSULTED or description == predict.GATE:
            statements = [
                r for r in records if r["kind"] == "statement" and r["outcome"] == description
            ]
            if statements and all(
                r["recognised"] in ("pointer-step", "stepped-access") for r in statements
            ):
                return "rule:gated-step" if description == predict.GATE else "rule:step"
            return "rule:unforecast"
        alias = predict.ALIAS.fullmatch(description)
        if alias:
            name = alias.group(1)
            copies = [
                r for r in records
                if r["kind"] == "statement"
                and r["recognised"] in ("pointer-copy", "scalar-copy")
                and r["step"]
                and r["step"][1] == name
            ]
            if name in declared["scalars"] or copies:
                return "rule:scalar-alias"
        return None

    newly_safe = {
        fkey[2]
        for fkey, status in after_status.items()
        if status == "Safe" and before_status.get(fkey) == "Unknown"
    }

    def declared_scalar(leaf, name):
        seq = assignment.get((leaf[0], leaf[1]))
        _, names = per_seq.get(seq, ({}, {}))
        return name in names.get(leaf[2], {"scalars": set()})["scalars"]

    removed = before - after
    added = after - before
    classes = collections.Counter()
    rows = []
    rule_functions = set()
    rule_coordinates = set()
    pending_removed = []
    for leaf, count in sorted(removed.items()):
        label = census_label(leaf)
        if label:
            rule_functions.add(leaf[:3])
            rule_coordinates.add(leaf[:4])
            classes[("removed", label)] += count
            rows.append({"delta": "removed", "class": label, "leaf": list(leaf), "count": count})
        else:
            pending_removed.append((leaf, count))
    for leaf, count in pending_removed:
        callee = re.fullmatch(r"call to uncontracted function `([^`]*)`", leaf[5])
        if callee and callee.group(1) in newly_safe:
            label = "summary"
        else:
            label = "downstream" if leaf[:3] in rule_functions else "unattributed"
        classes[("removed", label)] += count
        rows.append({"delta": "removed", "class": label, "leaf": list(leaf), "count": count})
    for leaf, count in sorted(added.items()):
        category = leaf[4]
        if category in VIOLATIONS:
            label = "violation"
        elif category == "unknown_alias" and leaf[:4] in rule_coordinates:
            label = "exchange"
        elif category == "unknown_alias" and declared_scalar(
            leaf, predict.ALIAS.fullmatch(leaf[5]).group(1)
        ):
            # A declared scalar's fact joined with no fact is now no fact; a
            # pre-existing copy of it into a place the rule does not type
            # (the generic `Alias`) then meets an unbound source.
            label = "exchange:scalar-join"
        elif category == "leak":
            label = "leak"
        elif leaf[:3] in rule_functions:
            label = "downstream"
        else:
            label = "unattributed"
        classes[("added", label)] += count
        rows.append({"delta": "added", "class": label, "leaf": list(leaf), "count": count})

    moves = collections.Counter()
    moved = []
    for fkey, status in after_status.items():
        old = before_status.get(fkey)
        if old != status:
            moves[(old, status)] += 1
            moved.append({"function": list(fkey), "before": old, "after": status})
    unit_moves = collections.Counter()
    moved_units = []
    for ukey, status in after_units.items():
        if before_units.get(ukey) != status:
            unit_moves[(before_units.get(ukey), status)] += 1
            moved_units.append({"unit": list(ukey), "before": before_units.get(ukey), "after": status})

    def counts(document):
        return document["diagnostic_counts"]

    forecast = json.load(open(HERE.parent / "census" / "predict.json"))
    forecast_functions = set(forecast["functions_all_classes"])
    to_safe = {
        "/".join(str(part) for part in row["function"])
        for row in moved
        if row["before"] == "Unknown" and row["after"] == "Safe"
    }
    result = {
        "before": str(before_run),
        "after": str(after_run),
        "census": str(census_run),
        "diagnostic_counts_before": counts(before_doc),
        "diagnostic_counts_after": counts(after_doc),
        "classes": {f"{d}:{c}": n for (d, c), n in sorted(classes.items())},
        "function_moves": {f"{a} -> {b}": n for (a, b), n in sorted(moves.items(), key=str)},
        "unit_moves": {f"{a} -> {b}": n for (a, b), n in sorted(unit_moves.items(), key=str)},
        "moved_functions": moved,
        "moved_units": moved_units,
        "forecast_to_safe_hit": sorted(to_safe & forecast_functions),
        "forecast_to_safe_missed": sorted(forecast_functions - to_safe),
        "unforecast_to_safe": sorted(to_safe - forecast_functions),
        "attribution_complete": classes[("removed", "unattributed")] == 0
        and classes[("added", "unattributed")] == 0,
        "rows": rows,
    }
    out_json.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    lines = [
        "W43-INDEX-TRAVERSAL attribution (BEFORE -> AFTER)",
        "",
        f"diagnostic counts: {counts(before_doc)} -> {counts(after_doc)}",
        "",
        "Leaf classes:",
    ]
    lines += [f"  {d:8} {c:20} {n:6}" for (d, c), n in sorted(classes.items())]
    lines += ["", "Function status moves:"]
    lines += [f"  {a} -> {b}: {n}" for (a, b), n in sorted(moves.items(), key=str)]
    lines += ["", "Unit status moves:"]
    lines += [f"  {a} -> {b}: {n}" for (a, b), n in sorted(unit_moves.items(), key=str)]
    lines += ["", "Moved functions:"]
    lines += [
        f"  {'/'.join(map(str, row['function']))}: {row['before']} -> {row['after']}"
        for row in moved
    ]
    lines += ["", f"Forecast Unknown->Safe hit: {len(result['forecast_to_safe_hit'])}"]
    lines += [f"  missed: {name}" for name in result["forecast_to_safe_missed"]]
    lines += [f"  unforecast: {name}" for name in result["unforecast_to_safe"]]
    lines += ["", "Violations, leaks and unattributed leaves:"]
    for row in rows:
        if row["class"] in ("violation", "leak", "unattributed") or row["leaf"][4] == "leak":
            lines.append(f"  {row['delta']:7} {row['class']:12} {row['leaf']}")
    lines += ["", f"attribution_complete: {result['attribution_complete']}"]
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:60]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
