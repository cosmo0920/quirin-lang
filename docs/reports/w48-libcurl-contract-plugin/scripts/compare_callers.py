#!/usr/bin/env python3
"""compare_callers.py RUN OUT: compare the function rows of callers.sh's
`without` and `with` runs (same units, same shipped modules; `with` adds the
libcurl plugin's user crate and checker bindings). Writes OUT (JSON) and
prints the moves. Exit 1 if any row moved to Safe or any row is unmatched."""
import json
import pathlib
import sys

run = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])


def rows(side):
    report = json.loads((run / side / "resource-diagnostics.json").read_text())
    table = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            table[(unit["file"], function["function"])] = function
    units = {unit["file"]: unit["status"] for unit in report["units"]}
    return table, units


before, before_units = rows("without")
after, after_units = rows("with")
moves, tallies, unmatched = [], {"without": {}, "with": {}}, sorted(
    {f"{k[0]}::{k[1]}" for k in set(before) ^ set(after)}
)
for side, table in (("without", before), ("with", after)):
    for function in table.values():
        tallies[side][function["status"]] = tallies[side].get(function["status"], 0) + 1
for key in sorted(set(before) & set(after)):
    a, b = before[key]["status"], after[key]["status"]
    links = after[key].get("plugin_links", [])
    curl_uncontracted = sorted(
        {
            d["description"].split("`")[1]
            for d in after[key]["diagnostics"]
            if "uncontracted function `curl_" in d["description"]
            or "uncontracted function `Wcurl_" in d["description"]
        }
    )
    if a != b or links:
        moves.append(
            {
                "file": key[0],
                "function": key[1],
                "without": a,
                "with": b,
                "plugin_links": [
                    {"callee": l["callee"], "verdict": l["verdict"]} for l in links
                ],
                "curl_still_uncontracted": curl_uncontracted,
            }
        )
record = {
    "schema": "w48-libcurl-callers-comparison/1",
    "units": {"without": before_units, "with": after_units},
    "function_tallies": tallies,
    "rows_moved_or_linked": moves,
    "unmatched": unmatched,
    "rows_to_safe": [m for m in moves if m["with"] == "Safe" and m["without"] != "Safe"],
    # A move into or out of Unsafe would be a host-verdict change the plugin
    # cannot cause; Unknown -> Pending is the contract applying.
    "rows_into_or_out_of_unsafe": [
        m for m in moves if (m["with"] == "Unsafe") != (m["without"] == "Unsafe")
    ],
}
out.write_text(json.dumps(record, indent=1) + "\n")
for move in moves:
    print(
        f"{move['file']}::{move['function']}: {move['without']} -> {move['with']}"
        f" links={[l['callee'] + ':' + l['verdict'] for l in move['plugin_links']]}"
        f" still={move['curl_still_uncontracted']}"
    )
print("tallies", tallies)
print("unmatched", unmatched)
sys.exit(1 if record["rows_to_safe"] or record["rows_into_or_out_of_unsafe"] or unmatched else 0)
