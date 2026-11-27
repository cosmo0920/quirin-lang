"""Wall split of a recorded measurement: per side and route, the slot wait
(the gate_slot receipt's waited_seconds) and the route's run time."""
import datetime
import json
import sys
from pathlib import Path


def parse(text):
    return datetime.datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ")


output = Path(sys.argv[1])
host = json.loads((output / "quirin-audit-measure.report.json").read_text())
print("host run:", {k: host["run"].get(k) for k in ("outcome", "wall_time_ms")})
for side in sorted((output / "sides").iterdir()):
    run = side / "run.json"
    if not run.is_file():
        continue
    record = json.loads(run.read_text())
    for name, route in sorted(record.get("routes", {}).items()):
        if "started" not in route:
            continue
        wall = (parse(route["ended"]) - parse(route["started"])).total_seconds()
        receipt = route.get("slot_receipt")
        waited = None
        if receipt and Path(receipt).is_file():
            waited = json.loads(Path(receipt).read_text())["slot_record"].get("waited_seconds")
        print(f"{side.name:6} {name:7} {route['profile']:22} wall {wall/60:7.1f} min  "
              f"slot wait {waited/60 if waited is not None else float('nan'):7.1f} min  "
              f"run {(wall - (waited or 0))/60:7.1f} min  started {route['started']}")
