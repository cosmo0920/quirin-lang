#!/usr/bin/env python3
"""compare_callers.py RUN OUT: compare callers.sh's `without` run (shipped
load path) with each `with-*` run (the user load path with this crate, under
one probe configuration). Writes OUT (JSON) and prints every row that moved
or links a bound libcurl contract, with the diagnostics that differ, so each
move is attributed. Exit 1 if any row moved to Safe, into or out of Unsafe,
or is unmatched."""
import json
import pathlib
import sys

run = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
SIDES = ("with-selected", "with-default", "with-full")


def load(side):
    path = run / side / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    sidecar = json.loads(path.with_name("resource-diagnostics.contracts.json").read_text())
    table = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            table[(unit["file"], function["function"])] = function
    units = {unit["file"]: unit["status"] for unit in report["units"]}
    return table, units, sidecar


def descriptions(row):
    return sorted({d["description"] for d in row["diagnostics"]})


def tally(table):
    counts = {}
    for row in table.values():
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    return dict(sorted(counts.items()))


before, before_units, _ = load("without")
record = {
    "schema": "libcurl-provider-plugin-callers-comparison/1",
    "without": {"units": before_units, "function_tallies": tally(before)},
    "sides": {},
}
failed = False
for side in SIDES:
    after, after_units, sidecar = load(side)
    unmatched = sorted(f"{k[0]}::{k[1]}" for k in set(before) ^ set(after))
    rows = []
    for key in sorted(set(before) & set(after)):
        a, b = before[key], after[key]
        links = [link for link in b.get("plugin_links", [])
                 if link.get("crate") == "quirin-provider-libcurl"]
        if a["status"] == b["status"] and not links:
            continue
        rows.append({
            "file": key[0],
            "function": key[1],
            "without": a["status"],
            "with": b["status"],
            "plugin_links": [{"callee": link["callee"], "verdict": link["verdict"],
                              "reasons": [reason["code"] for reason in link.get("reasons", [])]}
                             for link in links],
            "diagnostics_removed": sorted(set(descriptions(a)) - set(descriptions(b))),
            "diagnostics_added": sorted(set(descriptions(b)) - set(descriptions(a))),
        })
    versions = sidecar.get("contract_revision", {}).get("library_versions") or {}
    checks = sidecar.get("plugin_checks") or {}
    entry = {
        "units": after_units,
        "function_tallies": tally(after),
        "unmatched": unmatched,
        "library_versions": {
            "configurations": versions.get("configurations"),
            "libraries": versions.get("libraries"),
            "declarations": [{key: declaration.get(key) for key in
                              ("function", "range", "audited_version", "pinned_version",
                               "in_force", "reason", "detail")}
                             for declaration in versions.get("declarations", [])],
        },
        "plugin_checks": checks.get("counts"),
        "rows_moved_or_linked": rows,
        "rows_to_safe": [row for row in rows if row["with"] == "Safe" != row["without"]],
        "rows_into_or_out_of_unsafe": [row for row in rows
                                       if (row["with"] == "Unsafe") != (row["without"] == "Unsafe")],
    }
    record["sides"][side] = entry
    failed |= bool(entry["rows_to_safe"] or entry["rows_into_or_out_of_unsafe"] or unmatched)
    print(f"== {side}: tallies {entry['function_tallies']} (without {tally(before)}); "
          f"plugin_checks {entry['plugin_checks']}; unmatched {unmatched}")
    for declaration in entry["library_versions"]["declarations"]:
        print(f"   {declaration['function']}: in_force={declaration['in_force']} "
              f"reason={declaration['reason']}")
    for row in rows:
        print(f"   {row['file']}::{row['function']}: {row['without']} -> {row['with']} "
              f"links={[link['callee'] + ':' + link['verdict'] for link in row['plugin_links']]} "
              f"-{len(row['diagnostics_removed'])} +{len(row['diagnostics_added'])}")
out.write_text(json.dumps(record, indent=1) + "\n")
sys.exit(1 if failed else 0)
