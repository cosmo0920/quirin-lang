#!/usr/bin/env python3
"""summarise.py DIR -- one screen of the compare.py results in DIR/<scope>.json."""
import json
import pathlib
import sys

KEYS = [
    "resource_diagnostics_identical",
    "contracts_sidecar_identical",
    "report_differing_json_paths",
    "sidecar_differing_json_paths",
    "unit_status_moves",
    "function_status_moves",
    "function_leaf_moves",
    "rows_differing_otherwise",
]

for scope in ("corpus", "system", "slice"):
    path = pathlib.Path(sys.argv[1]) / f"{scope}.json"
    data = json.loads(path.read_text())
    print(f"== {scope}")
    for key in KEYS:
        print(f"  {key}: {json.dumps(data[key])[:600]}")
    print(f"  rows_differing_only_by_review_holds: {len(data['rows_differing_only_by_review_holds'])}")
    for row in data["rows_differing_only_by_review_holds"]:
        holds = [(h["declaration"], h["scope"], h["category"]) for h in row["review_holds"]]
        print(f"    {row['row']} {row['status']} {holds}")
    console = data["console"]
    print(f"  console {console['files']}: -{len(console['only_in_base'])} +{len(console['only_in_run'])}")
    for line in console["only_in_base"][:6]:
        print(f"    - {line[:300]}")
    for line in console["only_in_run"][:6]:
        print(f"    + {line[:300]}")
    print(f"  base {json.dumps(data['base'])}")
    print(f"  run  {json.dumps(data['run'])}")
