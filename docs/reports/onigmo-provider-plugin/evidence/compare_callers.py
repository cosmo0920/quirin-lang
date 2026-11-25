#!/usr/bin/env python3
"""compare_callers.py RUN OUT: compare callers.sh's `without` run (shipped
load path) with each `with-*` run (the user load path with
quirin-provider-onigmo, under one probe configuration). Card
ONIGMO-PROVIDER-PLUGIN.

Rows are keyed by (file, configuration, function): the selection keeps every
configuration, so a file is two units. Writes OUT (JSON): each side's
tallies, the library-version record, the plugin checks, every row that moved
or links a bound Onigmo contract with the diagnostics that differ, and, for
`without`, every row whose diagnostics name an Onigmo callee (how each call is
read today). Exit 1 if any row moved to Safe, into or out of Unsafe, or is
unmatched."""
import json
import pathlib
import re
import sys

run = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
SIDES = [side for side in ("with-selected", "with-full")
         if (run / side / "resource-diagnostics.json").is_file()]
ONIG = re.compile(r"`onig_[a-z_]+`")


def configuration(unit):
    return json.dumps(unit.get("compile_configuration"), sort_keys=True)


def load(side):
    path = run / side / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    sidecar_path = path.with_name("resource-diagnostics.contracts.json")
    sidecar = json.loads(sidecar_path.read_text()) if sidecar_path.is_file() else {}
    configurations = sorted({configuration(unit) for unit in report["units"]})
    table, units = {}, {}
    for unit in report["units"]:
        index = configurations.index(configuration(unit))
        units[f"{unit['file']}#{index}"] = unit["status"]
        for function in unit["functions"]:
            table[(unit["file"], index, function["function"])] = function
    return table, units, sidecar


def descriptions(row):
    return sorted({d["description"] for d in row["diagnostics"]})


def tally(table):
    counts = {}
    for row in table.values():
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    return dict(sorted(counts.items()))


def name(key):
    return f"{key[0]}#{key[1]}::{key[2]}"


before, before_units, _ = load("without")
today = []
for key in sorted(before):
    named = [d for d in descriptions(before[key]) if ONIG.search(d)]
    if named:
        today.append({"row": name(key), "status": before[key]["status"],
                      "onigmo_diagnostics": named,
                      "other_diagnostics": len(descriptions(before[key])) - len(named)})
record = {
    "schema": "onigmo-provider-plugin-callers-comparison/1",
    "without": {"units": before_units, "function_tallies": tally(before),
                "rows_naming_onigmo": today},
    "sides": {},
}
print(f"== without: tallies {tally(before)}; {len(today)} rows name an Onigmo callee")
for row in today:
    print(f"   {row['row']}: {row['status']} {row['onigmo_diagnostics']} "
          f"(+{row['other_diagnostics']} other)")
failed = False
for side in SIDES:
    after, after_units, sidecar = load(side)
    unmatched = sorted(name(k) for k in set(before) ^ set(after))
    rows = []
    for key in sorted(set(before) & set(after)):
        a, b = before[key], after[key]
        links = [link for link in b.get("plugin_links", [])
                 if link.get("crate") == "quirin-provider-onigmo"]
        removed = sorted(set(descriptions(a)) - set(descriptions(b)))
        added = sorted(set(descriptions(b)) - set(descriptions(a)))
        if a["status"] == b["status"] and not links and not removed and not added:
            continue
        rows.append({
            "row": name(key),
            "without": a["status"],
            "with": b["status"],
            "plugin_links": [{"callee": link.get("callee"), "verdict": link.get("verdict"),
                              "reasons": [reason.get("code") for reason in link.get("reasons", [])]}
                             for link in links],
            "diagnostics_removed": removed,
            "diagnostics_added": added,
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
                             for declaration in versions.get("declarations", [])
                             if str(declaration.get("function", "")).startswith("onig_")],
        },
        "plugin_checks": checks.get("counts"),
        "rows_moved_or_changed": rows,
        "rows_to_safe": [row for row in rows if row["with"] == "Safe" != row["without"]],
        "rows_into_or_out_of_unsafe": [row for row in rows
                                       if (row["with"] == "Unsafe") != (row["without"] == "Unsafe")],
    }
    record["sides"][side] = entry
    failed |= bool(entry["rows_to_safe"] or entry["rows_into_or_out_of_unsafe"] or unmatched)
    print(f"== {side}: tallies {entry['function_tallies']}; plugin_checks "
          f"{entry['plugin_checks']}; unmatched {unmatched}")
    for declaration in entry["library_versions"]["declarations"]:
        print(f"   {declaration['function']}: in_force={declaration['in_force']} "
              f"reason={declaration['reason']}")
    for row in rows:
        print(f"   {row['row']}: {row['without']} -> {row['with']} "
              f"links={[str(link['callee']) + ':' + str(link['verdict']) for link in row['plugin_links']]}")
        for text in row["diagnostics_removed"]:
            print(f"      - {text}")
        for text in row["diagnostics_added"]:
            print(f"      + {text}")
out.write_text(json.dumps(record, indent=1) + "\n")
sys.exit(1 if failed else 0)
