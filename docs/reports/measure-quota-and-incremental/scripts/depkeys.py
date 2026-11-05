"""Print which dependency keys a report's rows publish, and the sidecar's sections."""
import json
import sys
from pathlib import Path

route = Path(sys.argv[1])
report = json.loads((route / "resource-diagnostics.json").read_text())
keys = set()
rows = 0
for unit in report["units"]:
    for function in unit.get("functions", []):
        rows += 1
        dependencies = function.get("contract_dependencies")
        if isinstance(dependencies, dict):
            keys |= set(dependencies)
print("rows", rows, "units", len(report["units"]))
print("row dependency keys", sorted(keys))
print("report keys", sorted(report))
sidecar = json.loads((route / "resource-diagnostics.contracts.json").read_text())
print("contracts sidecar keys", sorted(sidecar))
