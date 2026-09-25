#!/usr/bin/env python3
"""Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: leaf-by-leaf attribution of AFTER vs BEFORE.

Usage: attribute.py BEFORE_RUN AFTER_RUN CENSUS_RUN PREDICTION_JSON OUT_JSON OUT_TXT

* BEFORE_RUN / AFTER_RUN: audit run directories.
* CENSUS_RUN: the census run of the base tree (the probe's own answers).
* PREDICTION_JSON: the frozen census output (`census/census.json`), whose
  `forecast.leaves` is the forecast set.

Every removed and every added leaf is put in exactly one class:

removed
  * `rule` - an `indirect memory access` leaf of the frozen forecast set;
  * `rule:outside-unknown` - an `indirect memory access` leaf whose every
    call-argument census record at its key is admitted by the probe, in a
    function that was not `Unknown` at BEFORE (the forecast counted Unknown
    functions only);
  * `rule-unforecast` - any other removed `indirect memory access` leaf whose
    census records the probe admitted (listed; a forecast miss);
  * `unexplained-indirect` - a removed `indirect memory access` leaf the probe
    did not admit (must be zero or explained);
  * `summary` - a `call to uncontracted function `F`` leaf where `F` moved to
    `Safe` in the same unit (constraint 4's route);
  * `downstream` - any other leaf: binding precision regained where a refused
    call statement became a modeled call, or an edge.
added
  * `exchange` - an `unresolved resource identity` leaf at the coordinate of a
    removed `rule*` leaf of the same function (an obligation refusing);
  * `exchange:call-refusal` - an `unsupported` leaf at such a coordinate: the
    now-modeled call's own refusal (for example an unsanctioned out-slot
    destination, reported as a discarded owned result);
  * `violation` - a `use_after_release` / `invalid_release` / `double_release`
    / `leak` leaf (listed with its witness);
  * `downstream` - anything else.
"""

import collections
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import call_argument_census as cac  # noqa: E402
import shape_census as census  # noqa: E402


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = path.read_bytes()
    if path.suffix == ".gz":
        data = gzip.decompress(data)
    return json.loads(data)


def leaves(doc):
    out = collections.Counter()
    status = {}
    units = {}
    for unit in doc["units"]:
        ukey = (unit.get("library"), unit["file"])
        units[ukey] = unit["status"]
        for function in unit["functions"]:
            fkey = ukey + (function["function"],)
            status[fkey] = function["status"]
            for d in function["diagnostics"]:
                out[(fkey, d["line"], d["category"], d["description"])] += 1
    return out, status, units


def main():
    before_doc = load(sys.argv[1])
    after_doc = load(sys.argv[2])
    census_run = pathlib.Path(sys.argv[3])
    prediction = json.load(open(sys.argv[4]))
    out_json = pathlib.Path(sys.argv[5])
    out_txt = pathlib.Path(sys.argv[6])
    before, before_status, before_units = leaves(before_doc)
    after, after_status, after_units = leaves(after_doc)
    forecast = {(f, line) for f, line in prediction["forecast"]["leaves"]}

    _, _, unit_keys = census.load_document(census_run / "resource-diagnostics.json")
    indirect = census.load_stream(census_run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        census_run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    calls = cac.load_calls(census_run / "call-argument-census.jsonl")
    assignment = census.match_units([indirect, statements, calls], unit_keys)

    def probe_admitted(fkey, line):
        seq = assignment.get(fkey[:2])
        if seq is None:
            return False
        key = (fkey[2], line, census.INDIRECT)
        records = calls.get(seq, {}).get(key, [])
        return bool(records) and all(cac.record_class(r)[0].startswith("admitted:") for r in records)

    def fkey_of(fkey):
        # The forecast was written as "library/file/function".
        return "/".join(str(p) for p in fkey)

    newly_safe = {
        (fkey[:2], fkey[2])
        for fkey, status in after_status.items()
        if status == "Safe" and before_status.get(fkey) != "Safe"
    }
    removed = before - after
    added = after - before
    classes = collections.Counter()
    rows = collections.defaultdict(list)
    rule_coordinates = set()
    for (fkey, line, category, description), n in sorted(removed.items(), key=str):
        if description == census.INDIRECT:
            if (fkey_of(fkey), line) in forecast:
                cls = "removed:rule"
            elif probe_admitted(fkey, line):
                cls = (
                    "removed:rule:outside-unknown"
                    if before_status.get(fkey) != "Unknown"
                    else "removed:rule-unforecast"
                )
            else:
                cls = "removed:unexplained-indirect"
            if cls != "removed:unexplained-indirect":
                rule_coordinates.add((fkey, line))
        elif (
            description.startswith("call to uncontracted function `")
            and (fkey[:2], description.split("`")[1]) in newly_safe
        ):
            cls = "removed:summary"
        else:
            cls = "removed:downstream"
        classes[cls] += n
        rows[cls].append(["/".join(map(str, fkey)), line, category, description, n])
    for (fkey, line, category, description), n in sorted(added.items(), key=str):
        if category == "unknown_alias" and (fkey, line) in rule_coordinates:
            cls = "added:exchange"
        elif category == "unsupported" and (fkey, line) in rule_coordinates:
            cls = "added:exchange:call-refusal"
        elif category in ("use_after_release", "invalid_release", "double_release", "leak"):
            cls = "added:violation"
        else:
            cls = "added:downstream"
        classes[cls] += n
        rows[cls].append(["/".join(map(str, fkey)), line, category, description, n])

    moves = collections.Counter()
    moved = []
    for fkey in sorted(set(before_status) | set(after_status), key=str):
        b, a = before_status.get(fkey), after_status.get(fkey)
        if b != a:
            moves[f"{b} -> {a}"] += 1
            moved.append(["/".join(map(str, fkey)), b, a])
    unit_moves = [
        ["/".join(map(str, k)), before_units.get(k), after_units.get(k)]
        for k in sorted(set(before_units) | set(after_units), key=str)
        if before_units.get(k) != after_units.get(k)
    ]

    def category_counts(counter):
        c = collections.Counter()
        for (fkey, line, category, description), n in counter.items():
            c[category] += n
        return c

    forecast_hit = sum(1 for row in rows["removed:rule"])
    result = {
        "before_counts": before_doc["diagnostic_counts"],
        "after_counts": after_doc["diagnostic_counts"],
        "removed_by_category": dict(category_counts(removed)),
        "added_by_category": dict(category_counts(added)),
        "classes": dict(classes),
        "forecast_leaves": len(forecast),
        "forecast_leaves_removed": forecast_hit,
        "forecast_leaves_kept": sorted(
            [f, line] for f, line in forecast
            if not any(r[0] == f and r[1] == line for r in rows["removed:rule"])
        ),
        "function_moves": dict(moves),
        "functions_moved": moved,
        "unit_moves": unit_moves,
        "rows": dict(rows),
        "attribution_complete": sum(classes.values()) == sum(removed.values()) + sum(added.values()),
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    lines = [
        f"before {result['before_counts']}",
        f"after  {result['after_counts']}",
        f"removed by category {result['removed_by_category']}",
        f"added by category {result['added_by_category']}",
        "",
        "classes:",
    ]
    for k, v in sorted(classes.items()):
        lines.append(f"  {v:6}  {k}")
    lines += [
        "",
        f"forecast set: {len(forecast)} leaves, {forecast_hit} removed, "
        f"{len(result['forecast_leaves_kept'])} kept:",
    ]
    for f, line in result["forecast_leaves_kept"]:
        lines.append(f"  {f} L{line}")
    lines += ["", f"function moves {dict(moves)}"]
    for row in moved:
        lines.append(f"  {row[1]} -> {row[2]}  {row[0]}")
    lines += ["", f"unit moves: {unit_moves}"]
    for cls in sorted(rows):
        if cls == "removed:rule":
            continue
        lines += ["", f"{cls} ({classes[cls]}):"]
        for row in rows.get(cls, [])[:300]:
            lines.append(f"  {row[0]} L{row[1]} {row[2]}: {row[3]} x{row[4]}")
    lines += ["", f"attribution_complete: {result['attribution_complete']}"]
    out_txt.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
