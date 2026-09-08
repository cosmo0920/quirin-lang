#!/usr/bin/env python3
"""Census of the enumerated main-source definition denominator (unit E01-F1).

Read-only. Aggregates the per-unit `enumeration` block a post-E01-F1
`resource-diagnostics.json` carries, and — given the pre-change artifact —
reports the denominator delta per library and every unit whose verdict moved.

    python3 -B docs/reports/remaining-e01-f1/census.py \
        --before <before>/resource-diagnostics.json \
        --after  <after>/resource-diagnostics.json \
        --output docs/reports/remaining-e01-f1/census.json
"""
import argparse
import collections
import gzip
import json
import pathlib


def load(path):
    data = path.read_bytes()
    if path.suffix == ".gz":
        data = gzip.decompress(data)
    return json.loads(data)


def library(unit):
    return unit.get("library", "?")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=pathlib.Path, required=True)
    parser.add_argument("--after", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path)
    args = parser.parse_args()

    before = {(library(u), u["file"]): u for u in load(args.before)["units"]}
    after = {(library(u), u["file"]): u for u in load(args.after)["units"]}

    per_library = collections.defaultdict(lambda: {
        "units": 0, "source_functions_before": 0, "source_functions_after": 0,
        "absent_from_raw_scan": 0, "unanalyzable": 0, "enumeration_refused_units": 0,
    })
    added_rows = []
    removed_rows = []
    verdict_moves = []
    refusals = collections.Counter()
    forms = collections.Counter()

    for key in sorted(set(before) | set(after)):
        left = before.get(key)
        right = after.get(key)
        lib = key[0]
        entry = per_library[lib]
        entry["units"] += 1
        if left:
            entry["source_functions_before"] += len(left.get("source_functions", []))
        if right:
            entry["source_functions_after"] += len(right.get("source_functions", []))
            block = right.get("enumeration") or {}
            entry["absent_from_raw_scan"] += len(block.get("absent_from_raw_scan", []))
            entry["unanalyzable"] += len(block.get("unanalyzable", []))
            if block.get("completeness") == "refused":
                entry["enumeration_refused_units"] += 1
            for row in block.get("refusals", []):
                refusals[row["ground"]] += 1
            for row in block.get("unanalyzable", []):
                forms[row["form"]] += 1
        if left and right:
            gained = sorted(set(right.get("source_functions", [])) -
                            set(left.get("source_functions", [])))
            lost = sorted(set(left.get("source_functions", [])) -
                          set(right.get("source_functions", [])))
            if gained:
                added_rows.append({"library": lib, "file": key[1], "functions": gained})
            if lost:
                removed_rows.append({"library": lib, "file": key[1], "functions": lost})
            if left.get("status") != right.get("status"):
                verdict_moves.append({
                    "library": lib, "file": key[1],
                    "before": left.get("status"), "after": right.get("status"),
                    "after_reason": right.get("reason"),
                })

    def totals(field):
        return sum(entry[field] for entry in per_library.values())

    document = {
        "card": "E01-F1",
        "per_library": {name: value for name, value in sorted(per_library.items())},
        "totals": {
            "units": totals("units"),
            "source_functions_before": totals("source_functions_before"),
            "source_functions_after": totals("source_functions_after"),
            "absent_from_raw_scan": totals("absent_from_raw_scan"),
            "unanalyzable": totals("unanalyzable"),
            "enumeration_refused_units": totals("enumeration_refused_units"),
        },
        "denominator_rows_added": added_rows,
        "denominator_rows_removed": removed_rows,
        "unit_verdict_moves": verdict_moves,
        "refusal_grounds": dict(refusals),
        "unanalyzable_forms": dict(forms),
        "status_before": dict(collections.Counter(
            u.get("status") for u in before.values())),
        "status_after": dict(collections.Counter(
            u.get("status") for u in after.values())),
    }
    text = json.dumps(document, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(text)
    print(text)


if __name__ == "__main__":
    main()
