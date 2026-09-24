#!/usr/bin/env python3
"""Run named standing- and provider-gate steps, each into its own log.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Adapted from docs/reports/w43-return-expression/scripts/run_gate.py: every
step's stdout and stderr go to OUT_DIR/<step>.log, its exit to
OUT_DIR/<step>.exit, and OUT_DIR/facts-<step>.json records the command, the
card environment, UTC start/end and the exit. Exits are captured from the
process, never through a pipe; a failing step does not stop later steps.
`test`-shaped steps count their `test result:` summaries. The card sets
CARGO_INCREMENTAL=0 (coordinator disk instruction) and bounded parallelism.
Package steps write into OUT_DIR/packages.
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

PACKAGES = ("qcore.chunkio", "qcore.cmetrics", "qcore.ctraces", "qcore.libevent", "qcore.librdkafka")


def steps(out):
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
        "docs_index": ["python3", "scripts/build_docs_index.py", "--check"],
        "provider_test": ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--workspace", "--locked",
                          "--offline", "--no-fail-fast"],
        "provider_fmt": ["cargo", "fmt", "--manifest-path", "providers/Cargo.toml", "--all", "--", "--check"],
        "provider_clippy": ["cargo", "clippy", "--manifest-path", "providers/Cargo.toml", "--workspace",
                            "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
        "provider_mutation": ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--locked", "--offline",
                              "-p", "qcore-provider-conformance", "--test", "packages",
                              "real_packages_preserve_unknown_and_reject_mutations", "--", "--exact",
                              "--test-threads=1"],
        "contracts_check": ["python3", "providers/contracts.py", "check"],
        "package_build": ["python3", "providers/package.py", "build", "--output", str(out / "packages")],
        "provider_python": ["python3", "-m", "unittest", "discover", "-s", "providers/tests"],
    }
    for package in PACKAGES:
        table["verify_" + package] = ["python3", "providers/package.py", "verify", str(out / "packages" / package)]
    return table


CARD_ENVIRONMENT = {
    "CARGO_INCREMENTAL": "0",
    "CARGO_BUILD_JOBS": "8",
    "RUST_TEST_THREADS": "4",
}


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    table = steps(out)
    environment = dict(os.environ)
    environment.update(CARD_ENVIRONMENT)
    worst = 0
    for step in sys.argv[2:]:
        command = table[step]
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with open(out / f"{step}.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode
        end = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / f"{step}.exit").write_text(f"{code}\n")
        facts = {"step": step, "command": command, "environment": CARD_ENVIRONMENT, "start": start, "end": end,
                 "exit": code}
        text = (out / f"{step}.log").read_text(errors="replace")
        if step in ("test", "provider_test", "provider_mutation"):
            totals = {"passed": 0, "failed": 0, "ignored": 0, "summaries": 0}
            for match in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
                totals["passed"] += int(match.group(1))
                totals["failed"] += int(match.group(2))
                totals["ignored"] += int(match.group(3))
                totals["summaries"] += 1
            facts["totals"] = totals
            facts["failed_tests"] = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)))
        if step in ("python", "provider_python"):
            match = re.search(r"^Ran (\d+) tests?", text, re.M)
            facts["tests_run"] = int(match.group(1)) if match else None
        if step == "lean":
            match = re.search(r"Passed (\d+) Lean audits", text)
            facts["audits_passed"] = int(match.group(1)) if match else None
        (out / f"facts-{step}.json").write_text(json.dumps(facts, indent=2) + "\n")
        print(json.dumps({k: facts.get(k) for k in ("step", "exit", "totals", "tests_run", "audits_passed")}),
              flush=True)
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
