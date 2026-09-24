#!/usr/bin/env python3
"""Collect this card's corpus-arbiter evidence into arbiter.json.

Usage: write_arbiter.py AFTER_RUN BEFORE_RUN BEFORE_ROOT BASELINE_ROOT

AFTER_RUN is this card's tip audit; BEFORE_RUN a same-source audit of main
built in its own checkout BEFORE_ROOT. Runs baseline_identity.py against the
main baseline (docs/reports/w43-indirect-access-general/artifacts/, root
alias this worktree -> BASELINE_ROOT), compare_runs.py BEFORE vs AFTER and
attribute_arbiter.py against the baseline, and writes all three results.
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
BASELINE = REPO / "docs/reports/w43-indirect-access-general/artifacts"


def run(command):
    completed = subprocess.run(command, text=True, capture_output=True)
    return completed.returncode, completed.stdout


def main() -> int:
    after, before, before_root, baseline_root = sys.argv[1:5]
    scratch = pathlib.Path(after).resolve().parent
    code_identity, identity = run([sys.executable, str(REPO / "docs/reports/w43-return-expression/scripts/baseline_identity.py"),
                                   after, str(BASELINE), "--root-alias", f"{REPO}={baseline_root}"])
    code_compare, _ = run([sys.executable, str(HERE / "compare_runs.py"), before, after, before_root, str(REPO),
                           str(scratch / "arbiter-before-after.json")])
    code_attr, _ = run([sys.executable, str(HERE / "attribute_arbiter.py"), after, str(BASELINE), str(REPO),
                        baseline_root, str(scratch / "arbiter-attribution.json")])
    record = {
        "card": "W43-DESCRIPTOR-NATIVE-CHECK",
        "after_run": json.loads((pathlib.Path(after) / "run.json").read_text()),
        "before_run": json.loads((pathlib.Path(before) / "run.json").read_text()),
        "baseline": str(BASELINE.relative_to(REPO)),
        "baseline_identity": {"exit": code_identity, "result": json.loads(identity)},
        "same_source_before_after": {"exit": code_compare,
                                     "result": json.loads((scratch / "arbiter-before-after.json").read_text())},
        "baseline_attribution": {"exit": code_attr,
                                 "result": json.loads((scratch / "arbiter-attribution.json").read_text())},
    }
    (REPO / "docs/reports/w43-descriptor-native-check/arbiter.json").write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({"baseline_identity_exit": code_identity, "before_after_exit": code_compare,
                      "verdict_bearing_identical": record["baseline_identity"]["result"]["verdict_bearing_identical"]}))
    return code_identity or code_compare


if __name__ == "__main__":
    sys.exit(main())
