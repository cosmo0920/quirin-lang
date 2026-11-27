#!/usr/bin/env python3
"""Summarize one user-path run (user-path.sh's OUT) into the committed record:
each fixture function's status, own (host) status, diagnostics and linked
callees with their verdicts, and the binding tally.

    python3 summarize.py OUT > run.json
"""
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
report = json.loads((out / "r.json").read_text())
sidecar = json.loads((out / "r.contracts.json").read_text())
rows = []
for unit in report["units"]:
    for function in unit["functions"]:
        rows.append({
            "unit": unit["file"],
            "function": function["function"],
            "status": function["status"],
            "host_status": function.get("host_status"),
            "diagnostics": [d["description"] for d in function["diagnostics"]],
            "links": [{"callee": link["callee"], "verdict": link["verdict"]}
                      for link in function.get("plugin_links", [])],
            "rests_on": function.get("contract_dependencies", {}).get("declarations", []),
        })
bindings = sidecar.get("plugin_checks", {}).get("bindings", [])
print(json.dumps({
    "rows": rows,
    "bindings": len(bindings),
    "binding_verdicts": sorted({b["verdict"] for b in bindings}),
    "library_versions": sidecar.get("library_versions"),
}, indent=1))
