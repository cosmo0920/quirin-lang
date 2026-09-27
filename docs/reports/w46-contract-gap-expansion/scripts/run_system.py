#!/usr/bin/env python3
"""Run the pinned current Fluent Bit system slice into a fresh directory.

Usage: run_system.py OUT [--jobs N]
Reuses the prepared, clean 046accf6 checkout and its recorded build commands.
It does not modify the source or create a substitute build configuration.
"""

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[4]
    prepared = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1")
    binary = root / "target/release/quirin-audit"
    source = prepared / "fluent-bit-clang-source"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != "046accf6aded7906f66a1baf0202da349e521736":
        raise SystemExit("prepared source revision does not match the selected slice")
    if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
        raise SystemExit("prepared source is dirty")
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    command = [str(binary), "current", "--checkout", str(source),
               "--profile-directory", str(root / "contract/profiles/fluent-bit"),
               "--compile-database", str(prepared / "fluent-bit-clang-production/compile_commands.json"),
               "--selection", str(root / "contract/profiles/fluent-bit/system-selection.json"),
               "--jobs", str(args.jobs), "--report", str(out / "resource-diagnostics.json")]
    environment = dict(os.environ, QCORE_CLANG="/usr/lib/llvm-21/bin/clang")
    record = {"command": command, "source_revision": revision,
              "qcore_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "QCORE_CLANG": environment["QCORE_CLANG"],
              "start": datetime.datetime.now(datetime.timezone.utc).isoformat()}
    with (out / "console.log").open("wb") as stdout, (out / "stderr.log").open("wb") as stderr:
        result = subprocess.run(command, cwd=root, env=environment, stdout=stdout, stderr=stderr)
    record.update(exit=result.returncode, end=datetime.datetime.now(datetime.timezone.utc).isoformat())
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    (out / "exit").write_text(str(result.returncode) + "\n")
    print(json.dumps(record, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
