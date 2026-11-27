#!/usr/bin/env python3
"""Summarize one user-path run (user-path.sh's OUT) into the committed
record: each fixture function's status, diagnostics and plugin links, every
plugin_checks binding (claim, code, verdict, proposed verdict, the plugin's
own diagnostics) and the library_versions record.

    python3 summarize.py OUT > run.json
"""
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
report = json.loads((out / "r.json").read_text())
sidecar = json.loads((out / "r.contracts.json").read_text())
fixture = {"clear_error", "init_library", "create_server_context", "ctx_term", "ctx_cycle"}
rows = []
for unit in report["units"]:
    for function in unit["functions"]:
        if function["function"] in fixture:
            rows.append({
                "unit": unit["file"],
                "function": function["function"],
                "status": function["status"],
                "diagnostics": [d["description"] for d in function["diagnostics"]],
                "plugin_links": function.get("plugin_links", []),
            })
bindings = [{key: binding.get(key) for key in
             ("crate", "module", "functions", "task_digest", "plugin_id", "package_digest",
              "claim", "code", "verdict", "reasons", "proposed", "plugin_diagnostics")}
            for binding in sidecar["plugin_checks"]["bindings"]]
print(json.dumps({
    "rows": sorted(rows, key=lambda row: row["function"]),
    "plugin_checks": {"counts": sidecar["plugin_checks"]["counts"], "bindings": bindings},
    "library_versions": sidecar["contract_revision"].get("library_versions"),
}, indent=1))
