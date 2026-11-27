#!/usr/bin/env python3
"""Card W44-BRANCH-CONDITION-CALLS: attribute every BEFORE -> AFTER delta, leaf by leaf.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS CENSUS_FINAL_GZ OUT_JSON OUT_TXT

CENSUS_FINAL_GZ is the census of record's final records
(`census/condition-call-census.final.jsonl.gz`, one per controlling expression,
the rule's own answer while it was not consulted). A leaf is the diagnostic
`(library, file, function, line, category, description)`; diagnostics are a set
per function.

Classes:

* `removed_admitted` - a removed condition-family leaf whose every census record
  at that (function, line, position) is `admitted`: the rule lowered it.
* `exchange_at_admitted_line` - an added leaf at the very line of a
  `removed_admitted` leaf: the refusal became a modeled branch or call whose
  own check now reports there.
* `desugared_function` - any other added or removed leaf in a function carrying
  at least one admitted census record: the function's CFG changed, and the
  delta is the flow over the new nodes (a leak exposed on a path no longer
  flagged unmodeled, a binding no longer poisoned by the refusal, an actual the
  admitted call now checks). Each is listed with its line and text for review.
* `summary_neighbour` - a delta in a function with no admitted record, of the
  one kind a desugared callee can cause: the function calls (per its BEFORE or
  AFTER leaves naming it) a function whose own status moved. Listed.
* `unattributed` - anything else; the script then exits 1.

Unit rows are compared field by field, excluding `functions`; a unit status
move is listed with the function moves it rolls up.

The census join is W43-INDIRECT-ACCESS-GENERAL's `match_units` over the
BEFORE document.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.append(str(HERE))
sys.path.append(str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

FAMILY = re.compile(
    r"unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)$"
)
POSITION = {
    "branch condition": "if",
    "loop condition": "while",
    "for-loop condition": "for",
    "do-while condition": "do-while",
    "switch expression": "switch",
}
NAMED = re.compile(r"`([^`]*)`")


def load_census(path):
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    for line in gzip.open(path, "rt"):
        record = json.loads(line)
        stream[record["unit_seq"]][(record["function"], record["line"], record["position"])].append(record)
    return stream


def leaves_of(document):
    functions = {}
    units = {}
    for unit in document["units"]:
        ukey = (unit.get("library"), unit["file"])
        units[ukey] = {k: v for k, v in unit.items() if k != "functions"}
        for entry in unit["functions"]:
            functions[ukey + (entry["function"],)] = {
                "status": entry["status"],
                "leaves": {
                    (d["line"], d["category"], d["description"]) for d in entry["diagnostics"]
                },
            }
    return functions, units


def main() -> int:
    before_path, after_path, census_path, out_json, out_txt = sys.argv[1:6]
    before = json.load(open(before_path))
    after = json.load(open(after_path))
    before_functions, before_units = leaves_of(before)
    after_functions, after_units = leaves_of(after)
    _, _, unit_keys = census.load_document(before_path)
    stream = load_census(census_path)
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l, _) in keys} for seq, keys in stream.items()}],
        unit_keys,
    )

    def records(fkey):
        return stream.get(assignment.get(fkey[:2]), {})

    def admitted_somewhere(fkey):
        return any(
            record["outcome"] == "admitted"
            for (function, _, _), rows in records(fkey).items()
            if function == fkey[2]
            for record in rows
        )

    moved = {
        fkey: (before_functions[fkey]["status"], after_functions[fkey]["status"])
        for fkey in before_functions
        if fkey in after_functions
        and before_functions[fkey]["status"] != after_functions[fkey]["status"]
    }
    moved_names = {fkey[2] for fkey in moved}

    classes = collections.defaultdict(list)
    for fkey in sorted(set(before_functions) | set(after_functions), key=str):
        if fkey not in before_functions or fkey not in after_functions:
            classes["unattributed"].append({"function": list(fkey), "why": "function set changed"})
            continue
        old = before_functions[fkey]["leaves"]
        new = after_functions[fkey]["leaves"]
        for sign, delta in (("removed", old - new), ("added", new - old)):
            for line, category, description in sorted(delta, key=str):
                row = {
                    "unit": list(fkey[:2]),
                    "function": fkey[2],
                    "line": line,
                    "category": category,
                    "description": description,
                    "delta": sign,
                    "status": [before_functions[fkey]["status"], after_functions[fkey]["status"]],
                }
                family = FAMILY.match(description)
                if sign == "removed" and family:
                    found = records(fkey).get((fkey[2], line, POSITION[family.group(1)]), [])
                    if found and all(record["outcome"] == "admitted" for record in found):
                        row["texts"] = [" ".join(record["texts"]) for record in found]
                        classes["removed_admitted"].append(row)
                        continue
                if admitted_somewhere(fkey):
                    classes["desugared_function"].append(row)
                    continue
                named = set(NAMED.findall(description))
                if named & moved_names:
                    classes["summary_neighbour"].append(row)
                    continue
                classes["unattributed"].append(row)

    # An added leaf at the very line of a removed admitted condition is that
    # condition's exchange: the refusal became a modeled branch or call whose
    # own check reports (an unresolved identity tested for null, or an actual
    # the admitted call now checks).
    admitted_lines = {
        (tuple(row["unit"]), row["function"], row["line"]) for row in classes["removed_admitted"]
    }
    kept = []
    for row in classes["desugared_function"]:
        key = (tuple(row["unit"]), row["function"], row["line"])
        if row["delta"] == "added" and key in admitted_lines:
            classes["exchange_at_admitted_line"].append(row)
        else:
            kept.append(row)
    classes["desugared_function"] = kept

    unit_moves = []
    for ukey, row in before_units.items():
        other = after_units.get(ukey)
        if other is None:
            unit_moves.append({"unit": list(ukey), "change": "unit missing"})
            continue
        changed = {
            key: [row.get(key), other.get(key)]
            for key in set(row) | set(other)
            if row.get(key) != other.get(key)
        }
        if changed:
            unit_moves.append({"unit": list(ukey), "changed": changed})

    counts = {
        "before": before["diagnostic_counts"],
        "after": after["diagnostic_counts"],
    }
    by_category = collections.Counter()
    for name, rows in classes.items():
        for row in rows:
            by_category[(name, row.get("delta"), row.get("category"))] += 1
    result = {
        "diagnostic_counts": counts,
        "function_moves": [
            {"function": list(fkey), "before": a, "after": b} for fkey, (a, b) in sorted(moved.items(), key=str)
        ],
        "unit_changes": unit_moves,
        "class_counts": {f"{k[0]}:{k[1]}:{k[2]}": v for k, v in sorted(by_category.items(), key=str)},
        "classes": classes,
        "attribution_complete": not classes["unattributed"],
    }
    pathlib.Path(out_json).write_text(json.dumps(result, indent=1) + "\n")
    with open(out_txt, "w") as text:
        text.write("W44-BRANCH-CONDITION-CALLS attribution\n\n")
        text.write(f"diagnostic counts before {counts['before']}\n")
        text.write(f"diagnostic counts after  {counts['after']}\n\n")
        for key, value in sorted(by_category.items(), key=str):
            text.write(f"  {value:6}  {key[0]:20} {key[1]:8} {key[2]}\n")
        text.write(f"\nfunction moves: {len(moved)}\n")
        for fkey, (a, b) in sorted(moved.items(), key=str):
            text.write(f"  {'/'.join(str(x) for x in fkey)}: {a} -> {b}\n")
        text.write(f"\nunit changes: {len(unit_moves)}\n")
        for row in unit_moves:
            text.write(f"  {json.dumps(row)}\n")
        for name in ("exchange_at_admitted_line", "desugared_function", "summary_neighbour", "unattributed"):
            text.write(f"\n{name}: {len(classes[name])}\n")
            for row in classes[name]:
                text.write(
                    f"  {row.get('delta')} {row.get('unit')} {row.get('function')} L{row.get('line')} "
                    f"{row.get('category')}: {row.get('description')} {row.get('status')}\n"
                )
        text.write(f"\nattribution_complete: {result['attribution_complete']}\n")
    print(pathlib.Path(out_txt).read_text()[:6000])
    return 0 if result["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
