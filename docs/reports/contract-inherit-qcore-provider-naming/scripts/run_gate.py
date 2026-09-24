#!/usr/bin/env python3
"""Run named gate steps for CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, each into its own log.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Each step's stdout and stderr go to OUT_DIR/<step>.log and its exit to
OUT_DIR/<step>.exit; OUT_DIR/facts-<step>.json records the command, the
environment, UTC start/end and the exit. Exits are captured directly from the
process, never through a pipe. A failing step does not stop later steps. For
test steps the per-binary `test result:` summaries are counted. Adapted from
`docs/reports/w43-return-expression/scripts/run_gate.py`, with the provider
steps added and clippy run with `--workspace` as the card brief states.
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

PROVIDERS = ["--manifest-path", "providers/Cargo.toml"]
PACKAGES = ["qcore.chunkio", "qcore.cmetrics", "qcore.ctraces", "qcore.libevent", "qcore.librdkafka"]

STEPS = {
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
    "ptest": ["cargo", "test", *PROVIDERS, "--workspace", "--locked", "--offline", "--no-fail-fast"],
    "pisolated": ["cargo", "test", *PROVIDERS, "-p", "qcore-provider-conformance", "--test", "packages",
                  "--locked", "--offline", "--", "real_packages_preserve_unknown_and_reject_mutations",
                  "--exact", "--test-threads=1"],
    "pfmt": ["cargo", "fmt", *PROVIDERS, "--all", "--", "--check"],
    "pclippy": ["cargo", "clippy", *PROVIDERS, "--workspace", "--all-targets", "--locked", "--offline",
                "--", "-D", "warnings"],
    "pcontracts": ["python3", "providers/contracts.py", "check"],
    "ppython": ["python3", "-m", "unittest", "discover", "-s", "providers/tests"],
}

ENVIRONMENT = {
    "CARGO_BUILD_JOBS": "6",
    "RUST_TEST_THREADS": "2",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
}


def command_for(step, out):
    if step == "pbuild":
        return ["python3", "providers/package.py", "build", "--output", str(out / "packages")]
    if step.startswith("pverify-"):
        return ["python3", "providers/package.py", "verify", str(out / "packages" / step[len("pverify-"):])]
    return STEPS[step]


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ)
    environment.update(ENVIRONMENT)
    worst = 0
    steps = []
    for step in sys.argv[2:]:
        steps.extend(["pbuild"] + [f"pverify-{p}" for p in PACKAGES] if step == "packages" else [step])
    for step in steps:
        command = command_for(step, out)
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with open(out / f"{step}.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode
        end = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / f"{step}.exit").write_text(f"{code}\n")
        facts = {"step": step, "command": command, "environment": ENVIRONMENT,
                 "start": start, "end": end, "exit": code}
        text = (out / f"{step}.log").read_text(errors="replace")
        if step in ("test", "ptest", "pisolated"):
            totals = {"passed": 0, "failed": 0, "ignored": 0, "summaries": 0}
            for match in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
                totals["passed"] += int(match.group(1))
                totals["failed"] += int(match.group(2))
                totals["ignored"] += int(match.group(3))
                totals["summaries"] += 1
            facts["totals"] = totals
            facts["failed_tests"] = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)))
        if step in ("python", "ppython"):
            match = re.search(r"^Ran (\d+) tests?", text, re.M)
            facts["tests"] = int(match.group(1)) if match else None
        if step == "lean":
            facts["tail"] = text.strip().splitlines()[-3:]
        (out / f"facts-{step}.json").write_text(json.dumps(facts, indent=2) + "\n")
        print(json.dumps({k: v for k, v in facts.items() if k != "environment"}))
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
