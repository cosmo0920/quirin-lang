"""Collect the R7 held-slot test's records into one JSON file.

Usage: evidence.py SCRATCH_ROOT OUT_JSON, where SCRATCH_ROOT holds the test's
<pid>-slot-wait-counted and <pid>-slot-wait-excluded directories."""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
document = {"schema": "measure-quota-r7-evidence/1",
            "test": "crates/quirin-audit-plugin-measure/tests/slot_wait.rs::"
                    "a_measurement_waiting_for_a_held_slot_is_not_killed_for_waiting",
            "quota_ms": 4000, "holder": "gate_slot.py rust-suite -- sleep 6", "runs": {}}
for case in ("counted", "excluded"):
    [directory] = sorted(root.glob(f"*-slot-wait-{case}"))
    out = directory / "out"
    report = json.loads((out / "quirin-audit-measure.report.json").read_text())
    invocation = json.loads((out / "quirin-audit-measure.report.invocation.json").read_text())
    run = {key: report["run"].get(key) for key in (
        "outcome", "code", "reason", "verdict", "wall_time_ms", "slot_wait_ms",
        "analysis_time_ms")}
    entry = {"slot_wait_limit_ms": invocation["slot_wait_limit_ms"],
             "quotas": invocation["quotas"], "host_run": run}
    record = out / "native-comparison.json"
    if record.is_file():
        entry["timing"] = json.loads(record.read_text())["timing"]
    document["runs"][case] = entry
Path(sys.argv[2]).write_text(json.dumps(document, indent=1, sort_keys=True) + "\n")
