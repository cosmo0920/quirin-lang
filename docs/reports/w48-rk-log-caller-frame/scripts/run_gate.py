#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: the standing gate plus the provider gate on the
committed card tip, under the shared card validation lock (adapted from
docs/reports/contract-cycle4-integration/run_gate.py). Every step's exit is
recorded; nothing is piped; a failing step does not stop later steps.

Usage: run_gate.py RUN   (writes private/rkcf/gate-RUN/, refuses an existing one)
"""

import argparse
import datetime
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    out = root / "private/rkcf" / ("gate-" + args.run)
    out.mkdir(parents=True, exist_ok=False)
    packages = out / "packages"
    commands = [
        ("submodules", ["git", "submodule", "update", "--init", "--recursive"]),
        ("clang-version", ["clang", "--version"]),
        ("rust", [sys.executable, "scripts/run_test_binaries.py", "--jobs", "4"]),
        ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
        ("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        ("probe", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]),
        ("followup", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"]),
        ("lean", [sys.executable, "scripts/check_lean_audits.py"]),
        ("python", [sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests"]),
        ("provider-rust", ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--workspace", "--locked", "--offline", "--no-fail-fast"]),
        ("provider-fmt", ["cargo", "fmt", "--manifest-path", "providers/Cargo.toml", "--all", "--", "--check"]),
        ("provider-clippy", ["cargo", "clippy", "--manifest-path", "providers/Cargo.toml", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]),
        ("provider-python", [sys.executable, "-m", "unittest", "discover", "-s", "providers/tests"]),
        ("provider-contracts", [sys.executable, "providers/contracts.py", "check"]),
        ("package-build", [sys.executable, "providers/package.py", "build", "--output", str(packages)]),
    ]
    commands += [("package-verify-" + name, [sys.executable, "providers/package.py", "verify", str(packages / ("qcore." + name))])
                 for name in ["chunkio", "libevent", "librdkafka", "cmetrics", "ctraces"]]

    def git(*parts):
        return subprocess.check_output(["git", *parts], cwd=root, text=True).strip()

    records = []
    with open("/tmp/qcore-c-card-validation-20261022.lock", "a") as lock:
        print("WAITING FOR VALIDATION LOCK", flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        head = git("rev-parse", "HEAD")
        if git("status", "--porcelain", "--untracked-files=no"):
            raise RuntimeError("the gate requires a clean committed source")
        env = dict(os.environ, CARGO_BUILD_JOBS="6")
        (out / "acquired.json").write_text(json.dumps({"source": head, "utc": now()}, indent=2) + "\n")
        for name, command in commands:
            started, start = now(), time.monotonic()
            print("START", name, started, flush=True)
            with (out / (name + ".log")).open("w") as stream:
                result = subprocess.run(command, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT)
            records.append(dict(name=name, command=command, source=head, started=started, ended=now(),
                                seconds=round(time.monotonic() - start, 1), exit=result.returncode,
                                log=str(out / (name + ".log")), head_after=git("rev-parse", "HEAD")))
            (out / "gate.json").write_text(json.dumps(records, indent=2) + "\n")
            print("END", name, result.returncode, flush=True)
    return 0 if all(r["exit"] == 0 for r in records) else 1


if __name__ == "__main__":
    sys.exit(main())
