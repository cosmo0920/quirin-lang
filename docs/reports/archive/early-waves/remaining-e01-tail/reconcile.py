#!/usr/bin/env python3
"""E01-TAIL: reconcile a fresh corpus measurement against the archived baseline.

Read-only. Card **E01-TAIL** (`docs/reports/remaining-e01-tail/README.md` §4).

The archived baseline keys each unit by a path relative to its library root,
while the current route keys it by the checkout-relative path, so units are
matched by suffix. Every archived unit must match at least one measured
configuration and every measured configuration must match an archived unit;
anything else is reported rather than absorbed.

    python3 -B docs/reports/remaining-e01-tail/reconcile.py <reports directory>

Exits nonzero if any unit verdict, enumerated denominator or function status
moved, so it can be used as a check and not only as a report.
"""
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
BASELINE = ROOT / "docs/reports/wave14-corpus/artifacts/diagnostics.json.gz"


def load(path):
    if path.suffix == ".gz":
        return json.load(gzip.open(path))
    return json.load(open(path))


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    measured_dir = pathlib.Path(sys.argv[1])
    archived = load(BASELINE)["units"]
    measured = load(measured_dir / "resource-diagnostics.json")["units"]

    by_key = {(u["library"], u["file"]): u for u in archived}

    def match(unit):
        for library, name in by_key:
            if unit["library"] == library and (
                unit["file"] == name or unit["file"].endswith("/" + name)
            ):
                return (library, name)
        return None

    groups, unmatched = {}, []
    for unit in measured:
        key = match(unit)
        if key is None:
            unmatched.append((unit["library"], unit["file"]))
        else:
            groups.setdefault(key, []).append(unit)

    missing = [key for key in by_key if key not in groups]
    duplicated = {key: len(rows) for key, rows in groups.items() if len(rows) != 1}

    verdict_moves, denominator_moves, function_moves, set_differences = [], [], [], []
    for key, rows in groups.items():
        base = by_key[key]
        for unit in rows:
            if unit["status"] != base["status"]:
                verdict_moves.append((key, base["status"], unit["status"]))
            if unit["source_functions"] != base["source_functions"]:
                denominator_moves.append(
                    (key, len(base["source_functions"]), len(unit["source_functions"]))
                )
            before = {f["function"]: f["status"] for f in base["functions"]}
            after = {f["function"]: f["status"] for f in unit["functions"]}
            for name in before.keys() & after.keys():
                if before[name] != after[name]:
                    function_moves.append((key, name, before[name], after[name]))
            if set(before) != set(after):
                set_differences.append((key, sorted(set(before) ^ set(after))))

    report = {
        "archived_units": len(by_key),
        "measured_configurations": len(measured),
        "measured_configurations_matching_no_archived_unit": unmatched,
        "archived_units_with_no_measured_configuration": missing,
        "archived_units_with_more_than_one_configuration": {
            f"{library}/{name}": count for (library, name), count in duplicated.items()
        },
        "unit_verdict_moves": verdict_moves,
        "enumerated_denominator_moves": denominator_moves,
        "function_status_moves": function_moves,
        "function_set_differences": set_differences,
    }
    print(json.dumps(report, indent=1, sort_keys=True))
    bad = (
        unmatched
        or missing
        or verdict_moves
        or denominator_moves
        or function_moves
        or set_differences
    )
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
