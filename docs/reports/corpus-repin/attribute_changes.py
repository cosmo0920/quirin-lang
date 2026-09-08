#!/usr/bin/env python3
"""Attribute the re-pin's diagnostic delta to named causes.

Card CORPUS-REPIN, deliverable 3. Reads the archived `diagnostics.json.gz` and
a re-measured one and produces a machine-readable attribution:

* per-function status transitions (the C02 precision effect),
* the line-independent diagnostic delta by (category, description), which
  separates *content* changes from preprocessed *line* movement,
* the units whose preprocessed line numbers moved by a constant offset, which
  is the compile-database / Monkey-preparation provenance effect,
* the unit-set difference.

    python3 -B docs/reports/corpus-repin/attribute_changes.py \
        --archived docs/reports/w5-cmetrics-unit-provider/diagnostics.json.gz \
        --measured docs/reports/corpus-repin/artifacts/diagnostics.json.gz \
        --output   docs/reports/corpus-repin/attribution.json

Read-only with respect to every artifact.
"""
import argparse
import collections
import gzip
import json
import pathlib

# The three diagnostic descriptions card C02's typed direct scalar-field access
# whitelist is expected to remove. Naming them here makes the attribution
# falsifiable: any other description that disappears is reported separately.
C02_DESCRIPTIONS = {
    "unsupported assignment expression",
    "indirect memory access is outside the resource projection",
    "unrecognized assignment destination",
}


def load(path):
    raw = path.read_bytes()
    if path.suffix == ".gz":
        raw = gzip.decompress(raw)
    return json.loads(raw)


def unit_key(unit):
    return (unit["library"], unit["file"])


def diagnostics_by_unit(document, with_line):
    table = {}
    for unit in document["units"]:
        bag = collections.Counter()
        for function in unit.get("functions", []):
            for diagnostic in function.get("diagnostics", []):
                key = (function["function"], diagnostic["category"], diagnostic["description"])
                if with_line:
                    key = key + (diagnostic["line"],)
                bag[key] += 1
        table[unit_key(unit)] = bag
    return table


def statuses_by_unit(document):
    return {unit_key(unit): {f["function"]: f["status"] for f in unit.get("functions", [])}
            for unit in document["units"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archived", type=pathlib.Path, required=True)
    parser.add_argument("--measured", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path)
    arguments = parser.parse_args()
    before, after = load(arguments.archived), load(arguments.measured)

    report = {"schema_version": 1, "card": "CORPUS-REPIN",
              "archived": str(arguments.archived), "measured": str(arguments.measured)}

    report["unit_status"] = {
        "before": dict(collections.Counter(u["status"] for u in before["units"])),
        "after": dict(collections.Counter(u["status"] for u in after["units"])),
    }
    before_units, after_units = {unit_key(u) for u in before["units"]}, {unit_key(u) for u in after["units"]}
    report["unit_set"] = {
        "added": sorted(list(key) for key in after_units - before_units),
        "removed": sorted(list(key) for key in before_units - after_units),
        "common": len(before_units & after_units),
    }
    status_before = {unit_key(u): u["status"] for u in before["units"]}
    status_after = {unit_key(u): u["status"] for u in after["units"]}
    report["unit_status_transitions"] = [
        {"unit": list(key), "before": status_before[key], "after": status_after[key]}
        for key in sorted(before_units & after_units) if status_before[key] != status_after[key]
    ]

    sb, sa = statuses_by_unit(before), statuses_by_unit(after)
    transitions = []
    for key in sorted(before_units & after_units):
        for function in sorted(set(sb[key]) | set(sa[key])):
            old, new = sb[key].get(function), sa[key].get(function)
            if old != new:
                transitions.append({"unit": list(key), "function": function,
                                    "before": old, "after": new})
    report["function_status_transitions"] = transitions
    report["function_status_transition_counts"] = dict(
        collections.Counter(f'{t["before"]} -> {t["after"]}' for t in transitions))

    plain_before, plain_after = diagnostics_by_unit(before, False), diagnostics_by_unit(after, False)
    removed, added = collections.Counter(), collections.Counter()
    for key in before_units & after_units:
        for diagnostic, count in (plain_before[key] - plain_after[key]).items():
            removed[(diagnostic[1], diagnostic[2])] += count
        for diagnostic, count in (plain_after[key] - plain_before[key]).items():
            added[(diagnostic[1], diagnostic[2])] += count
    report["line_independent_delta"] = {
        "removed_total": sum(removed.values()),
        "added_total": sum(added.values()),
        "removed": [{"category": c, "description": d, "count": n}
                    for (c, d), n in removed.most_common()],
        "added": [{"category": c, "description": d, "count": n}
                  for (c, d), n in added.most_common()],
    }
    c02_removed = sum(n for (_, d), n in removed.items() if d in C02_DESCRIPTIONS)
    c02_added = sum(n for (_, d), n in added.items() if d in C02_DESCRIPTIONS)
    report["c02_attribution"] = {
        "descriptions": sorted(C02_DESCRIPTIONS),
        "removed_in_those_descriptions": c02_removed,
        "added_in_those_descriptions": c02_added,
        "net": c02_removed - c02_added,
        "share_of_all_removals": round(c02_removed / max(sum(removed.values()), 1), 4),
        "other_removals": sum(removed.values()) - c02_removed,
    }

    line_before, line_after = diagnostics_by_unit(before, True), diagnostics_by_unit(after, True)
    offsets = []
    for key in sorted(before_units & after_units):
        grouped_before, grouped_after = collections.defaultdict(list), collections.defaultdict(list)
        for diagnostic, count in line_before[key].items():
            grouped_before[diagnostic[:3]] += [diagnostic[3]] * count
        for diagnostic, count in line_after[key].items():
            grouped_after[diagnostic[:3]] += [diagnostic[3]] * count
        histogram = collections.Counter()
        for name in set(grouped_before) & set(grouped_after):
            if len(grouped_before[name]) == len(grouped_after[name]):
                for old, new in zip(sorted(grouped_before[name]), sorted(grouped_after[name])):
                    histogram[new - old] += 1
        if histogram:
            offset, matched = histogram.most_common(1)[0]
            if offset:
                offsets.append({"unit": list(key), "offset": offset,
                                "matched": matched, "total": sum(histogram.values())})
    report["preprocessed_line_offsets"] = offsets

    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(text)
        print(f"wrote {arguments.output}")
    else:
        print(text, end="")
    print(f'unit statuses: {report["unit_status"]["before"]} -> {report["unit_status"]["after"]}')
    print(f'unit-level transitions: {len(report["unit_status_transitions"])}')
    print(f'function-level transitions: {report["function_status_transition_counts"]}')
    print(f'line-independent delta: -{report["line_independent_delta"]["removed_total"]} '
          f'+{report["line_independent_delta"]["added_total"]}')
    print(f'C02-attributed removals: {c02_removed} '
          f'({report["c02_attribution"]["share_of_all_removals"]:.1%} of removals)')
    print(f'units with a constant preprocessed-line offset: {len(offsets)}')


if __name__ == "__main__":
    main()
