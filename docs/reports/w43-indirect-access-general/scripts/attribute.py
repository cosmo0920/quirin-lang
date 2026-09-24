#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: leaf-by-leaf attribution of AFTER vs BEFORE.

Usage: attribute.py BEFORE_RUN AFTER_RUN CENSUS_RUN OUT_JSON OUT_TXT

* BEFORE_RUN / AFTER_RUN: audit run directories (`resource-diagnostics.json`).
* CENSUS_RUN: the BEFORE-tree census run (statement and indirect streams),
  whose statements the forecast mirror (`typed_scalar.py`) classifies.

Every removed and every added leaf is put in exactly one class:

removed
  * `rule` - an intercepted refusal at a coordinate whose every census
    statement the mirror admits (the lexical half of the admission);
  * `rule-unforecast` - an intercepted refusal the mirror did not admit or the
    census did not explain, whose statements carry no call (a forecast miss of
    the rule, listed);
  * `unreached` - an intercepted refusal one of whose statements carries a
    call, which the rule never admits: the node is no longer reached
    (an edge became infeasible), attributed apart from the rule;
  * `summary` - a `call to uncontracted function `F`` leaf where `F` is a
    function of the same unit that moved to `Safe` (constraint 4's route);
  * `downstream` - any other leaf (a leak, an alias) that disappeared:
    binding precision regained when a blanket or framed refusal became an
    admitted statement, or an edge.
added
  * `exchange` - an `unresolved resource identity` leaf at a coordinate of a
    removed `rule` or `rule-unforecast` leaf of the same function (the flow
    half refusing);
  * `violation` - a `use_after_release` / `invalid_release` leaf;
  * `downstream` - anything else.

Function and unit status moves are listed in full.
"""

import collections
import gzip
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import predict  # noqa: E402
import shape_census as census  # noqa: E402
import typed_scalar  # noqa: E402


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
    out_json = pathlib.Path(sys.argv[4])
    out_txt = pathlib.Path(sys.argv[5])
    before, before_status, before_units = leaves(before_doc)
    after, after_status, after_units = leaves(after_doc)

    _, functions, unit_keys = census.load_document(census_run / "resource-diagnostics.json")
    indirect = census.load_stream(census_run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        census_run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    assignment = census.match_units([indirect, statements], unit_keys)

    def mirror(fkey, line, description):
        seq = assignment.get(fkey[:2])
        key = (fkey[2], line, description)
        records = []
        if seq is not None:
            records = statements.get(seq, {}).get(key, [])
            if description == census.INDIRECT:
                records = indirect.get(seq, {}).get(key, []) or records
        if not records:
            return None
        plans = [typed_scalar.classify(record) for record in records]
        return plans if all(plans) else False

    def mirror_records(fkey, line, description):
        seq = assignment.get(fkey[:2])
        key = (fkey[2], line, description)
        if seq is None:
            return []
        return statements.get(seq, {}).get(key, []) + indirect.get(seq, {}).get(key, [])

    newly_safe = {
        (fkey[:2], fkey[2])
        for fkey, status in after_status.items()
        if status == "Safe" and before_status.get(fkey) != "Safe"
    }
    removed = before - after
    added = after - before
    classes = collections.Counter()
    rows = collections.defaultdict(list)
    shapes = collections.Counter()
    rule_coordinates = set()
    for (fkey, line, category, description), n in sorted(removed.items()):
        if predict.intercepted(description):
            plans = mirror(fkey, line, description)
            if plans:
                cls = "removed:rule"
                rule_coordinates.add((fkey, line))
                for plan in plans:
                    shapes[(plan["shape"], census.normalize(description))] += 0
                shapes[(plans[0]["shape"], census.normalize(description))] += n
            elif plans is False and any(
                census.has_call(record["texts"]) for record in mirror_records(fkey, line, description)
            ):
                cls = "removed:unreached"
            else:
                cls = "removed:rule-unforecast"
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
    for (fkey, line, category, description), n in sorted(added.items()):
        if description.startswith("unresolved resource identity") and (fkey, line) in rule_coordinates:
            cls = "added:exchange"
        elif category in ("use_after_release", "invalid_release"):
            cls = "added:violation"
        else:
            cls = "added:downstream"
        classes[cls] += n
        rows[cls].append(["/".join(map(str, fkey)), line, category, description, n])

    moves = collections.Counter()
    moved = []
    for fkey in sorted(set(before_status) | set(after_status)):
        b, a = before_status.get(fkey), after_status.get(fkey)
        if b != a:
            moves[f"{b} -> {a}"] += 1
            moved.append(["/".join(map(str, fkey)), b, a])
    unit_moves = [["/".join(map(str, k)), before_units.get(k), after_units.get(k)]
                  for k in sorted(set(before_units) | set(after_units)) if before_units.get(k) != after_units.get(k)]

    def category_counts(counter):
        c = collections.Counter()
        for (fkey, line, category, description), n in counter.items():
            c[category] += n
        return c

    result = {
        "before_counts": before_doc["diagnostic_counts"],
        "after_counts": after_doc["diagnostic_counts"],
        "removed_by_category": dict(category_counts(removed)),
        "added_by_category": dict(category_counts(added)),
        "classes": dict(classes),
        "removed_rule_by_shape": {f"{s} | {f}": n for (s, f), n in shapes.most_common() if n},
        "function_moves": dict(moves),
        "functions_moved": moved,
        "unit_moves": unit_moves,
        "rows": {k: v for k, v in rows.items()},
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
    lines += ["", "removed rule leaves by shape:"]
    for k, v in result["removed_rule_by_shape"].items():
        lines.append(f"  {v:6}  {k}")
    lines += ["", f"function moves {dict(moves)}"]
    for row in moved:
        lines.append(f"  {row[1]} -> {row[2]}  {row[0]}")
    lines += ["", f"unit moves: {unit_moves}"]
    for cls in (
        "removed:rule-unforecast",
        "removed:unreached",
        "removed:summary",
        "removed:downstream",
        "added:downstream",
        "added:violation",
    ):
        lines += ["", f"{cls} ({classes[cls]}):"]
        for row in rows.get(cls, [])[:200]:
            lines.append(f"  {row[0]} L{row[1]} {row[2]}: {row[3]} x{row[4]}")
    lines += ["", f"attribution_complete: {result['attribution_complete']}"]
    out_txt.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
