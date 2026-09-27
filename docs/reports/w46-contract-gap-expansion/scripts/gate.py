#!/usr/bin/env python3
"""Run the standing and provider gates, preserving every exit and private log.

Usage: gate.py LOG_DIR PACKAGE_DIR
LOG_DIR must be new. No check is piped; a failed check does not hide later ones.
"""

import argparse
import datetime
import json
import os
import pathlib
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", type=pathlib.Path)
    parser.add_argument("packages", type=pathlib.Path)
    parser.add_argument("--root", type=pathlib.Path,
                        help="isolated worktree to validate (default: this checkout)")
    args = parser.parse_args()
    root = args.root.resolve() if args.root else pathlib.Path(__file__).resolve().parents[4]
    logs = args.logs.resolve()
    logs.mkdir(parents=True, exist_ok=False)
    packages = args.packages.resolve()
    environment = dict(os.environ, CARGO_INCREMENTAL="0")
    steps = [
        ("submodule", ["git", "submodule", "update", "--init", "--recursive"]),
        ("tests", ["python3", "scripts/run_test_binaries.py"]),
        ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
        ("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        ("reevaluation-probe", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]),
        ("reevaluation-followup", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"]),
        ("lean", ["python3", "scripts/check_lean_audits.py"]),
        ("python", ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]),
        ("docs-index", ["python3", "scripts/build_docs_index.py", "--check"]),
        ("provider-tests", ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--workspace", "--locked", "--offline"]),
        ("provider-fmt", ["cargo", "fmt", "--manifest-path", "providers/Cargo.toml", "--all", "--", "--check"]),
        ("provider-clippy", ["cargo", "clippy", "--manifest-path", "providers/Cargo.toml", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]),
        ("provider-contracts-check", ["python3", "providers/contracts.py", "check"]),
        ("package-build", ["python3", "providers/package.py", "build", "--output", str(packages)]),
    ]
    for package in ["qcore.chunkio", "qcore.cmetrics", "qcore.ctraces", "qcore.libevent", "qcore.librdkafka"]:
        steps.append(("package-verify-" + package, ["python3", "providers/package.py", "verify", str(packages / package)]))
    steps.extend([
        ("provider-python", ["python3", "-m", "unittest", "discover", "-s", "providers/tests"]),
        ("clang-version", ["clang", "--version"]),
    ])
    record = {"revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "logs": str(logs), "steps": []}
    for name, command in steps:
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with (logs / (name + ".log")).open("wb") as output:
            result = subprocess.run(command, cwd=root, env=environment, stdout=output, stderr=subprocess.STDOUT)
        (logs / (name + ".exit")).write_text(str(result.returncode) + "\n")
        record["steps"].append({"name": name, "command": command, "exit": result.returncode,
                                "start": start, "end": datetime.datetime.now(datetime.timezone.utc).isoformat()})
        (logs / "steps.json").write_text(json.dumps(record, indent=2) + "\n")
        print(name, result.returncode, flush=True)
    (logs / "ALL_DONE").write_text("complete\n")
    return 1 if any(step["exit"] != 0 for step in record["steps"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
