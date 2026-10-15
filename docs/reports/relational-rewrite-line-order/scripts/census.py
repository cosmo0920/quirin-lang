#!/usr/bin/env python3
"""census.py RUN [ROUTE-REPORT] OUT.json -- RELATIONAL-REWRITE-LINE-ORDER:
the stream line-order census from a run's parity stream (private/rrlo/RUN-parity.jsonl,
RUN-system-parity.jsonl or RUN-slice-parity.jsonl; QCORE_TYPED_PATH_PARITY rows with
`position` `stream-rewrite` or `stream-stage`, written by
crates/qcore-c-gateway/src/legacy_stream_line_order.rs).

* `rewrites`: per rewrite (`relational-11r`, `handoff-direct`), the units (source
  digests) where it fired, its sites (unit, line) and how many firings left the
  stream unsorted.
* `stages`: per rewrite stage of build_function_cfg_with (and `input`), the
  functions whose build it touched, and those whose stream it left unsorted
  (a stage that found the stream sorted and left it unsorted is the culprit).
* `units`: per source digest where any stage left a stream unsorted, the unit
  file(s) of the route report holding its functions, and those functions.

The analyser builds a function several times (derivation rounds, the final
analysis); rows are de-duplicated by key."""
import collections
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/rrlo"


def main():
    parity, report_path, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
    if not parity.is_absolute() and not parity.exists():
        parity = PRIVATE / parity
    rewrites = collections.defaultdict(lambda: {"units": set(), "sites": set(), "unsorted_units": set()})
    stages = collections.defaultdict(lambda: {"functions": set(), "unsorted_after": set(),
                                              "broke": set()})
    unsorted_functions = collections.defaultdict(set)
    with parity.open() as handle:
        for line in handle:
            if '"stream-' not in line:
                continue
            row = json.loads(line)
            position = row.get("position")
            source = row.get("source_sha256")
            if position == "stream-rewrite":
                entry = rewrites[row["rewrite"]]
                entry["units"].add(source)
                for site in row["sites"]:
                    entry["sites"].add((source, site))
                if not row["sorted_after"]:
                    entry["unsorted_units"].add(source)
            elif position == "stream-stage":
                entry = stages[row["stage"]]
                key = (source, row["function"])
                entry["functions"].add(key)
                if not row["sorted_after"]:
                    entry["unsorted_after"].add(key)
                    unsorted_functions[source].add(row["function"])
                    if row["sorted_before"]:
                        entry["broke"].add(key)
    files = collections.defaultdict(set)
    if report_path != "-":
        path = pathlib.Path(report_path)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        report = json.loads(path.read_text())
        for unit in report["units"]:
            for function in unit["functions"]:
                files[function["function"]].add(unit["file"])
    units = {}
    for source, functions in sorted(unsorted_functions.items()):
        candidates = None
        for function in functions:
            found = files.get(function, set())
            if found:
                candidates = set(found) if candidates is None else candidates & found
        units[source] = {"files": sorted(candidates or []),
                         "functions": sorted(functions)}
    summary = {
        "rewrites": {name: {"units": len(e["units"]), "sites": len(e["sites"]),
                            "unsorted_units": len(e["unsorted_units"]),
                            "site_list": sorted([s, l] for s, l in e["sites"])}
                     for name, e in sorted(rewrites.items())},
        "stages": {name: {"functions": len(e["functions"]),
                          "unsorted_after": len(e["unsorted_after"]),
                          "broke": len(e["broke"])}
                   for name, e in sorted(stages.items())},
        "unsorted_units": len(units),
        "unsorted_unit_functions": sum(len(u["functions"]) for u in units.values()),
        "units": units,
    }
    out.write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: summary[k] for k in ("stages", "unsorted_units",
                                              "unsorted_unit_functions")}, indent=1))
    for name, entry in summary["rewrites"].items():
        print(name, {k: v for k, v in entry.items() if k != "site_list"})
    for source, unit in units.items():
        print(source[:12], unit["files"], len(unit["functions"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
