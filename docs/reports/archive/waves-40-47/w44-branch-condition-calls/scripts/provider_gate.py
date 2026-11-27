#!/usr/bin/env python3
"""Run the provider gate steps, each into its own log, recording exits.

Usage: provider_gate.py OUT_DIR

Steps (constraint 18 and the card brief): provider workspace tests, fmt,
clippy, the isolated package mutation control, `providers/contracts.py check`,
five package builds and verifications, and the package Python tests. Exits are
captured from the process, never through a pipe; a failing step does not stop
later steps. `facts.json` records commands, UTC times, exits and test totals.
"""

import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

PACKAGES = ["qcore.chunkio", "qcore.cmetrics", "qcore.ctraces", "qcore.libevent", "qcore.librdkafka"]


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    packages = out / "packages"
    manifest = ["--manifest-path", "providers/Cargo.toml"]
    steps = [
        ("test", ["cargo", "test", *manifest, "--workspace", "--locked", "--offline", "--no-fail-fast"]),
        ("fmt", ["cargo", "fmt", *manifest, "--all", "--", "--check"]),
        ("clippy", ["cargo", "clippy", *manifest, "--workspace", "--all-targets", "--locked",
                    "--offline", "--", "-D", "warnings"]),
        ("mutation", ["cargo", "test", *manifest, "-p", "qcore-provider-conformance", "--test",
                      "packages", "--locked", "--offline", "--",
                      "real_packages_preserve_unknown_and_reject_mutations", "--exact",
                      "--test-threads=1"]),
        ("contracts", ["python3", "providers/contracts.py", "check"]),
        ("build", ["python3", "providers/package.py", "build", "--output", str(packages)]),
    ]
    steps += [
        (f"verify-{name}", ["python3", "providers/package.py", "verify", str(packages / name)])
        for name in PACKAGES
    ]
    steps.append(("python", ["python3", "-m", "unittest", "discover", "-s", "providers/tests"]))
    environment = dict(os.environ)
    environment["CARGO_INCREMENTAL"] = "0"
    facts = []
    worst = 0
    for name, command in steps:
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with open(out / f"{name}.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode
        (out / f"{name}.exit").write_text(f"{code}\n")
        row = {"step": name, "command": command, "start": start,
               "end": datetime.datetime.now(datetime.timezone.utc).isoformat(), "exit": code}
        text = (out / f"{name}.log").read_text(errors="replace")
        if name in ("test", "mutation"):
            totals = [0, 0, 0, 0]
            for match in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
                totals[0] += int(match.group(1))
                totals[1] += int(match.group(2))
                totals[2] += int(match.group(3))
                totals[3] += 1
            row["totals"] = dict(zip(["passed", "failed", "ignored", "summaries"], totals))
        if name == "python":
            match = re.search(r"Ran (\d+) tests?", text)
            row["ran"] = int(match.group(1)) if match else None
        facts.append(row)
        print(json.dumps(row))
        worst = worst or code
    (out / "facts.json").write_text(json.dumps(facts, indent=2) + "\n")
    return worst


if __name__ == "__main__":
    sys.exit(main())
