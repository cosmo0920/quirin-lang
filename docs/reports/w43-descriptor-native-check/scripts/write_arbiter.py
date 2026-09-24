#!/usr/bin/env python3
"""Collect this card's corpus-arbiter evidence into arbiter.json.

Usage: write_arbiter.py AFTER_RUN BASELINE_DIR BASELINE_ROOT [BEFORE_RUN BEFORE_ROOT]

AFTER_RUN is this card's tip audit. BASELINE_DIR is the archived baseline
(its manifest.json names the five artifacts) and BASELINE_ROOT the checkout
root it was captured in. Runs baseline_identity.py with the root alias
this worktree -> BASELINE_ROOT. When a same-source BEFORE audit of main is
given (built in its own checkout BEFORE_ROOT), also runs compare_runs.py.
Writes arbiter.json beside this report; the Phase A record is kept under
`phase_a` when present.
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
RECORD = REPO / "docs/reports/w43-descriptor-native-check/arbiter.json"


def run(command):
    completed = subprocess.run(command, text=True, capture_output=True)
    return completed.returncode, completed.stdout


def main() -> int:
    after, baseline, baseline_root = sys.argv[1:4]
    before = sys.argv[4:6]
    code_identity, identity = run([sys.executable,
                                   str(REPO / "docs/reports/w43-return-expression/scripts/baseline_identity.py"),
                                   after, baseline, "--root-alias", f"{REPO}={baseline_root}"])
    previous = json.loads(RECORD.read_text()) if RECORD.is_file() else {}
    record = {
        "card": "W43-DESCRIPTOR-NATIVE-CHECK",
        "phase": "B",
        "after_run": json.loads((pathlib.Path(after) / "run.json").read_text()),
        "baseline": str(pathlib.Path(baseline).resolve().relative_to(REPO)),
        "baseline_root": baseline_root,
        "baseline_identity": {"exit": code_identity, "result": json.loads(identity)},
        "phase_a": previous.get("phase_a", previous if previous.get("card") and "phase" not in previous else None),
    }
    code_compare = 0
    if before:
        scratch = pathlib.Path(after).resolve().parent
        code_compare, _ = run([sys.executable, str(HERE / "compare_runs.py"), before[0], after, before[1], str(REPO),
                               str(scratch / "arbiter-before-after.json")])
        record["before_run"] = json.loads((pathlib.Path(before[0]) / "run.json").read_text())
        record["same_source_before_after"] = {
            "exit": code_compare, "result": json.loads((scratch / "arbiter-before-after.json").read_text())}
    RECORD.write_text(json.dumps(record, indent=1) + "\n")
    result = record["baseline_identity"]["result"]
    print(json.dumps({"baseline_identity_exit": code_identity, "before_after_exit": code_compare,
                      "verdict_bearing_identical": result["verdict_bearing_identical"],
                      "artifacts": {k: (v["byte_identical"], v.get("identical_after_root_alias"))
                                    for k, v in result["artifacts"].items()}}))
    return code_identity or code_compare


if __name__ == "__main__":
    sys.exit(main())
