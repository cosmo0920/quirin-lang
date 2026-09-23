#!/usr/bin/env python3
"""Run named standing-gate steps, each into its own log, recording exits.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Steps: submodules, clang, test, fmt, clippy, probe, followup, lean, python.
Each step's stdout and stderr go to OUT_DIR/<step>.log and its exit to
OUT_DIR/<step>.exit; OUT_DIR/facts-<step>.json records the command, the
environment the card sets, UTC start/end and the exit. Exits are captured
directly from the process, never through a pipe. A failing step does not stop
later steps. For `test`, the per-binary `test result:` summaries are counted.
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

STEPS = {
    "submodules": ["git", "submodule", "update", "--init", "--recursive"],
    "clang": ["clang", "--version"],
    "test": ["cargo", "test", "--workspace", "--no-fail-fast"],
    "fmt": ["cargo", "fmt", "--all", "--", "--check"],
    "clippy": ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
    "probe": ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"],
    "followup": ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"],
    "lean": ["python3", "scripts/check_lean_audits.py", "--jobs", "4"],
    "python": ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"],
}

CARD_ENVIRONMENT = {
    "CARGO_BUILD_JOBS": "4",
    "RUST_TEST_THREADS": "2",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
}


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ)
    environment.update(CARD_ENVIRONMENT)
    worst = 0
    for step in sys.argv[2:]:
        command = STEPS[step]
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with open(out / f"{step}.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode
        end = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / f"{step}.exit").write_text(f"{code}\n")
        facts = {
            "step": step,
            "command": command,
            "environment": CARD_ENVIRONMENT,
            "start": start,
            "end": end,
            "exit": code,
        }
        if step == "test":
            text = (out / "test.log").read_text(errors="replace")
            totals = {"passed": 0, "failed": 0, "ignored": 0, "summaries": 0}
            for match in re.finditer(
                r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text
            ):
                totals["passed"] += int(match.group(1))
                totals["failed"] += int(match.group(2))
                totals["ignored"] += int(match.group(3))
                totals["summaries"] += 1
            facts["totals"] = totals
            facts["failed_tests"] = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)))
        (out / f"facts-{step}.json").write_text(json.dumps(facts, indent=2) + "\n")
        print(json.dumps(facts))
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
