#!/usr/bin/env python3
"""Card TASK-ROUTE-TSAN: derive the card's gate.json from the run files."""
import json
import pathlib
import re

runs = {}
for step in ["test", "fmt", "clippy", "probe", "followup", "lean"]:
    runs[step] = int(pathlib.Path(f"measure/gate-run4/{step}.rc").read_text().strip())

log = pathlib.Path("measure/gate-run4/test.log").read_text()
totals = {"passed": 0, "failed": 0, "ignored": 0}
summaries = 0
for m in re.finditer(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", log, re.M):
    summaries += 1
    totals["passed"] += int(m.group(1))
    totals["failed"] += int(m.group(2))
    totals["ignored"] += int(m.group(3))

lean = pathlib.Path("measure/gate-run4/lean.log").read_text()
audits = len(re.findall(r"^Passed Audit", lean, re.M))

gate = {
    "card": "TASK-ROUTE-TSAN",
    "commit": "dd07c480",
    "note": (
        "The standing per-change gate from AGENTS.md, run from artifacts/gate.sh. "
        "Every step's exit captured to its own file with no pipe in the command, "
        "into a run-scoped directory so a re-run cannot truncate the run it confirms."
    ),
    "submodule_init": {"exit": 0, "libevent": "d82464a2"},
    "disk_free_before": "81G",
    "concurrency": (
        "Another session ran its own full gate on this machine for part of both runs; "
        "load average peaked near 33. No constraint-16 load-sensitive row failed in "
        "either run, so none needed isolated re-verification."
    ),
    "steps": runs,
    "workspace_tests": {**totals, "result_summaries": summaries},
    "lean_audits": audits,
    "earlier_run": {
        "id": "gate-run3",
        "commit": "7892d8dc",
        "steps": {"test": 0, "fmt": 1, "clippy": 0, "probe": 0, "followup": 0, "lean": 0},
        "workspace_tests": {"passed": 4620, "failed": 0, "ignored": 8},
        "note": (
            "fmt failed on two rustfmt violations in this card's own "
            "tests/support/thread_sanitizer.rs. Fixed at dd07c480 (whitespace only, "
            "inside a cfg(linux) helper) and the WHOLE gate re-run rather than the one "
            "step, so the published figures are a single measurement of one tree."
        ),
    },
}
pathlib.Path("docs/reports/task-route-tsan/gate.json").write_text(json.dumps(gate, indent=2) + "\n")
print(json.dumps(runs), totals, "summaries", summaries, "audits", audits)
