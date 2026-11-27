#!/usr/bin/env python3
"""Run named standing-gate and provider-gate steps, each into its own log.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Core steps: submodules, clang, test, fmt, clippy, probe, followup, lean, python.
Provider steps (constraint 18): ptest, pisolated, pfmt, pclippy, pcontracts,
pbuild, pverify, ppython. Each step's combined output goes to OUT_DIR/<step>.log
and its exit to OUT_DIR/<step>.exit; OUT_DIR/facts-<step>.json records the
command(s), the environment this card sets, UTC start/end and the exit. Exits
are captured directly from the process, never through a pipe; a failing step
does not stop later ones. `test`/`ptest` count the `test result:` summaries.
`pverify` verifies each of the five packages `pbuild` wrote (worst exit kept).
Copied from docs/reports/w43-libc-result-alias/scripts/run_gate.py; the
changes are `CARGO_INCREMENTAL=0` (the card brief's build hygiene) and a
`docsindex` step (`scripts/build_docs_index.py --check`).
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


def steps(out):
    packages = str(out / "packages")
    return {
        "submodules": [["git", "submodule", "update", "--init", "--recursive"]],
        "clang": [["clang", "--version"]],
        "test": [["cargo", "test", "--workspace", "--no-fail-fast"]],
        "fmt": [["cargo", "fmt", "--all", "--", "--check"]],
        "clippy": [["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]],
        "probe": [["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]],
        "followup": [["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"]],
        "lean": [["python3", "scripts/check_lean_audits.py", "--jobs", "4"]],
        "python": [["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]],
        "docsindex": [["python3", "scripts/build_docs_index.py", "--check"]],
        "ptest": [["cargo", "test", *PROVIDERS, "--workspace", "--locked", "--offline", "--no-fail-fast"]],
        "pisolated": [[
            "cargo", "test", *PROVIDERS, "-p", "qcore-provider-conformance", "--test", "packages",
            "real_packages_preserve_unknown_and_reject_mutations", "--locked", "--offline",
            "--", "--exact", "--test-threads=1",
        ]],
        "pfmt": [["cargo", "fmt", *PROVIDERS, "--all", "--", "--check"]],
        "pclippy": [["cargo", "clippy", *PROVIDERS, "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]],
        "pcontracts": [["python3", "providers/contracts.py", "check"]],
        "pbuild": [["python3", "providers/package.py", "build", "--output", packages]],
        "pverify": [["python3", "providers/package.py", "verify", f"{packages}/{p}"] for p in PACKAGES],
        "ppython": [["python3", "-m", "unittest", "discover", "-s", "providers/tests"]],
    }

CARD_ENVIRONMENT = {
    "CARGO_INCREMENTAL": "0",
    "CARGO_BUILD_JOBS": "6",
    "RUST_TEST_THREADS": "2",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
}


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ)
    environment.update(CARD_ENVIRONMENT)
    table = steps(out)
    worst = 0
    for step in sys.argv[2:]:
        commands = table[step]
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        codes = []
        with open(out / f"{step}.log", "wb") as log:
            for command in commands:
                log.write(("$ " + " ".join(command) + "\n").encode())
                log.flush()
                codes.append(subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode)
        code = next((c for c in codes if c != 0), 0)
        end = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / f"{step}.exit").write_text(f"{code}\n")
        facts = {"step": step, "commands": commands, "exits": codes, "environment": CARD_ENVIRONMENT,
                 "start": start, "end": end, "exit": code}
        if step in ("test", "ptest", "python", "ppython", "pisolated"):
            text = (out / f"{step}.log").read_text(errors="replace")
            totals = {"passed": 0, "failed": 0, "ignored": 0, "summaries": 0}
            for match in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
                totals["passed"] += int(match.group(1))
                totals["failed"] += int(match.group(2))
                totals["ignored"] += int(match.group(3))
                totals["summaries"] += 1
            ran = re.findall(r"^Ran (\d+) tests?", text, re.M)
            if ran:
                totals["python_ran"] = sum(int(n) for n in ran)
            facts["totals"] = totals
            facts["failed_tests"] = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)))
        (out / f"facts-{step}.json").write_text(json.dumps(facts, indent=2) + "\n")
        print(json.dumps(facts))
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
