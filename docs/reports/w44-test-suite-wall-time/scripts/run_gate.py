#!/usr/bin/env python3
"""Run named standing-gate and provider-gate steps, each into its own log.

Usage: run_gate.py OUT_DIR STEP [STEP ...]

Core steps: submodules, clang, runner, test, fmt, clippy, census, probe,
followup, lean, python, docsindex. Provider steps: ptest, pisolated, pfmt,
pclippy, pcontracts. Each step's combined output goes to OUT_DIR/<step>.log and
its exit to OUT_DIR/<step>.exit; OUT_DIR/facts-<step>.json records the
command(s), the environment this card sets, UTC start/end, wall seconds and the
exit. Exits are captured directly from the process, never through a pipe; a
failing step does not stop later ones. `test`/`ptest` count the `test result:`
summaries; `runner` reads its own summary.json.

Copied from docs/reports/w44-assignment-destination-suffix-place/scripts/run_gate.py
by card W44-TEST-SUITE-WALL-TIME, which (1) adds the `runner` step, the gate's
new form of the Rust suite (scripts/run_test_binaries.py), keeping `test`
(plain `cargo test --workspace --no-fail-fast`) as the reference; (2) drops the
earlier cards' CARGO_BUILD_JOBS, RUST_TEST_THREADS and CARGO_PROFILE_*_DEBUG
overrides, so the committed profile and libtest's own thread count are what is
measured; and (3) records each step's wall time.
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys
import time

PROVIDERS = ["--manifest-path", "providers/Cargo.toml"]


def steps(out):
    return {
        "submodules": [["git", "submodule", "update", "--init", "--recursive"]],
        "clang": [["clang", "--version"]],
        "runner": [["python3", "scripts/run_test_binaries.py", "--log-dir", str(out / "runner")]],
        "test": [["cargo", "test", "--workspace", "--no-fail-fast"]],
        "fmt": [["cargo", "fmt", "--all", "--", "--check"]],
        "clippy": [["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]],
        "census": [
            ["cargo", "clippy", "-p", "qcore-c-gateway", "--features", "indirect-census", "--all-targets", "--", "-D", "warnings"],
            ["cargo", "test", "-p", "qcore-c-gateway", "--features", "indirect-census", "--lib"],
        ],
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
    }


CARD_ENVIRONMENT = {"CARGO_INCREMENTAL": "0"}


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
        t0 = time.monotonic()
        load = os.getloadavg()
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
                 "start": start, "end": end, "wall_seconds": round(time.monotonic() - t0, 1),
                 "load_before": [round(x, 2) for x in load], "exit": code}
        if step in ("test", "ptest", "python", "pisolated", "census"):
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
        if step == "runner":
            summary = out / "runner" / "summary.json"
            if summary.exists():
                document = json.loads(summary.read_text())
                facts["totals"] = document["totals"]
                facts["phases_seconds"] = document["phases_seconds"]
                facts["runner_ok"] = document["ok"]
                facts["units_failed"] = document["units_failed"]
                facts["exclusive_units"] = {
                    u["uid"]: u["ok"] for u in document["units"] if u["exclusive"]
                }
        (out / f"facts-{step}.json").write_text(json.dumps(facts, indent=2) + "\n")
        print(json.dumps({k: facts[k] for k in ("step", "exit", "wall_seconds")} | (
            {"totals": facts["totals"]} if "totals" in facts else {})), flush=True)
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
