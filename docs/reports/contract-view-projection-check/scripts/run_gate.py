#!/usr/bin/env python3
"""Run named gate steps, each into its own log, recording exits.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Core steps: submodules, clang, test, fmt, clippy, probe, followup, lean,
python, docs-index. Provider steps: p-test, p-fmt, p-clippy, p-mutation,
p-contracts, p-python, p-build, p-verify-<package>.

Each step's stdout and stderr go to OUT_DIR/<step>.log and its exit to
OUT_DIR/<step>.exit; OUT_DIR/facts-<step>.json records the command, UTC
start/end and the exit. Exits are captured directly from the process, never
through a pipe. A failing step does not stop later steps. For the two cargo
test steps the per-binary `test result:` summaries are counted. `p-build`
writes packages to OUT_DIR/packages, which the `p-verify-*` steps read.
(Adapted from docs/reports/w43-return-expression/scripts/run_gate.py; clippy
here is `--workspace`, as the card brief states the gate.)
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

PACKAGES = ["qcore.chunkio", "qcore.cmetrics", "qcore.ctraces", "qcore.libevent", "qcore.librdkafka"]
PROVIDERS = ["--manifest-path", "providers/Cargo.toml"]


def steps(out: pathlib.Path) -> dict:
    table = {
        "submodules": ["git", "submodule", "update", "--init", "--recursive"],
        "clang": ["clang", "--version"],
        "test": ["cargo", "test", "--workspace", "--no-fail-fast"],
        "fmt": ["cargo", "fmt", "--all", "--", "--check"],
        "clippy": ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        "probe": ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"],
        "followup": ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"],
        "lean": ["python3", "scripts/check_lean_audits.py", "--jobs", "4"],
        "python": ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"],
        "docs-index": ["python3", "scripts/build_docs_index.py", "--check"],
        "p-test": ["cargo", "test", *PROVIDERS, "--workspace", "--locked", "--offline", "--no-fail-fast"],
        "p-fmt": ["cargo", "fmt", *PROVIDERS, "--all", "--", "--check"],
        "p-clippy": ["cargo", "clippy", *PROVIDERS, "--workspace", "--all-targets", "--locked",
                     "--offline", "--", "-D", "warnings"],
        "p-mutation": ["cargo", "test", *PROVIDERS, "--locked", "--offline", "-p",
                       "qcore-provider-conformance", "--test", "packages",
                       "real_packages_preserve_unknown_and_reject_mutations", "--",
                       "--exact", "--test-threads=1"],
        "p-contracts": ["python3", "providers/contracts.py", "check"],
        "p-python": ["python3", "-m", "unittest", "discover", "-s", "providers/tests"],
        "p-build": ["python3", "providers/package.py", "build", "--output", str(out / "packages")],
    }
    for package in PACKAGES:
        table[f"p-verify-{package}"] = ["python3", "providers/package.py", "verify",
                                         str(out / "packages" / package)]
    return table


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    table = steps(out)
    worst = 0
    for step in sys.argv[2:]:
        command = table[step]
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with open(out / f"{step}.log", "wb") as log:
            code = subprocess.run(command, env=dict(os.environ), stdout=log,
                                  stderr=subprocess.STDOUT).returncode
        end = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / f"{step}.exit").write_text(f"{code}\n")
        facts = {"step": step, "command": command, "start": start, "end": end, "exit": code}
        if step in ("test", "p-test", "p-mutation"):
            text = (out / f"{step}.log").read_text(errors="replace")
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
