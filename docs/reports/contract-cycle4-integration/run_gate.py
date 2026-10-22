#!/usr/bin/env python3
"""Record the cycle's combined standing gate; matrix Python follows publication."""

import argparse
import datetime
import fcntl
import hashlib
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
    parser.add_argument("run", help="fresh private run directory name")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    out = root / "private/contract-cycle4-integration" / args.run
    out.mkdir(parents=True, exist_ok=False)
    records = []
    commands = [
        ("submodules", ["git", "submodule", "update", "--init", "--recursive"]),
        ("clang-version", ["clang", "--version"]),
        ("rust", [sys.executable, "scripts/run_test_binaries.py", "--jobs", "4"]),
        ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
        ("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        ("probe", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]),
        ("followup", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_followup"]),
        ("lean", [sys.executable, "scripts/check_lean_audits.py"]),
        ("audit-examples", ["cargo", "build", "-p", "qcore-audit", "--examples"]),
        ("source-examples", ["cargo", "build", "-p", "quirin-source-analysis", "--examples"]),
        ("release", ["cargo", "build", "--release", "-p", "qcore-audit", "--bin", "quirin-audit"]),
    ]
    packages = out / "packages"
    commands += [
        ("provider-rust", ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--workspace", "--locked", "--offline", "--no-fail-fast"]),
        ("provider-fmt", ["cargo", "fmt", "--manifest-path", "providers/Cargo.toml", "--all", "--", "--check"]),
        ("provider-clippy", ["cargo", "clippy", "--manifest-path", "providers/Cargo.toml", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]),
        ("provider-python", [sys.executable, "-m", "unittest", "discover", "-s", "providers/tests"]),
        ("provider-contracts", [sys.executable, "providers/contracts.py", "check"]),
        ("package-build", [sys.executable, "providers/package.py", "build", "--output", str(packages)])]
    commands += [("package-verify-" + name, [sys.executable, "providers/package.py", "verify", str(packages / ("qcore." + name))]) for name in ["chunkio", "libevent", "librdkafka", "cmetrics", "ctraces"]]
    def git(*parts):
        return subprocess.check_output(["git", *parts], cwd=root, text=True).strip()

    with open("/tmp/qcore-c-card-validation-20261022.lock", "a") as lock, \
            open("/tmp/qcore-c-cycle2-small-compile.lock", "a") as small:
        print("WAITING FOR VALIDATION LOCK", flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        fcntl.flock(small, fcntl.LOCK_EX)
        head = git("rev-parse", "HEAD")
        if git("status", "--porcelain"):
            raise RuntimeError("Combined gate requires a clean committed source")
        provider_delta = git("diff", "--name-only", "31c5ed500d9d4a1448a623b839e1644adb19efd1", "HEAD", "--", "providers")
        if not provider_delta:
            raise RuntimeError("This frozen campaign expects the deliberately changed cJSON provider inputs")
        overrides = {
            "CARGO_PROFILE_DEV_DEBUG": "0",
            "CARGO_PROFILE_TEST_DEBUG": "0",
            "CARGO_INCREMENTAL": "0",
            "QCORE_AUDIT_BUILD_COMMIT": head,
        }
        env = dict(os.environ, **overrides)
        (out / "acquired.json").write_text(json.dumps({"source": head, "utc": now()}, indent=2) + "\n")
        for name, command in commands:
            started, start = now(), time.monotonic()
            print("START", name, started, flush=True)
            with (out / (name + ".log")).open("w") as stream:
                result = subprocess.run(command, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT)
            record = dict(name=name, command=command, cwd=str(root), source=head,
                          started=started, ended=now(), seconds=time.monotonic() - start,
                          exit=result.returncode, environment_overrides=overrides,
                          log=str(out / (name + ".log")), head_after=git("rev-parse", "HEAD"),
                          clean_after=not bool(git("status", "--porcelain")))
            records.append(record)
            (out / (name + ".exit")).write_text(str(result.returncode) + "\n")
            (out / "commands.json").write_text(json.dumps(records, indent=2) + "\n")
            print("END", name, result.returncode, flush=True)
        accepted = all(r["exit"] == 0 and r["head_after"] == head and r["clean_after"] for r in records)
        completion = dict(wrapper_exit=0 if accepted else 1, source=head,
                          head_after=git("rev-parse", "HEAD"), clean_after=not bool(git("status", "--porcelain")),
                          all_commands_zero=all(r["exit"] == 0 for r in records),
                          source_stable=all(r["head_after"] == head and r["clean_after"] for r in records),
                          command_count=len(records),
                          commands_sha256=hashlib.sha256((out / "commands.json").read_bytes()).hexdigest(),
                          matrix_python="pending: run after regenerated matrix is committed",
                          provider_gate="applicable: cJSON contract metadata and official pin/inventory changed; all five packages rebuilt/verified",
                          provider_delta_base="31c5ed500d9d4a1448a623b839e1644adb19efd1",
                          provider_changed_paths=provider_delta.splitlines())
        (out / "completion.json").write_text(json.dumps(completion, indent=2) + "\n")
    (out / "released.json").write_text(json.dumps({"utc": now()}, indent=2) + "\n")
    print(json.dumps(completion), flush=True)
    return completion["wrapper_exit"]


if __name__ == "__main__":
    raise SystemExit(main())
