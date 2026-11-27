#!/usr/bin/env python3
"""Run the corpus arbiter and the system slice with an explicit binary.

Usage: measure.py BINARY RUN_DIR COMMIT [--jobs N] [--scope corpus|system|both]

The same commands as `scripts/run_pinned_audit.py` (corpus, pinned inputs
under the read-only `build/cycle19`) and
`w46-contract-gap-expansion/scripts/run_system.py` (system slice at the
prepared, clean `046accf6` checkout), each with `QCORE_TYPED_PATH_PARITY`
inside its own fresh output directory. The binary is named explicitly (a
private copy), so a later build in the worktree cannot change a run in
flight. Every exit goes to its own file; nothing is piped.
"""

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[4]
INPUTS = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
PREPARED = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1")
SLICE = "046accf6aded7906f66a1baf0202da349e521736"


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def corpus(binary, out, commit, jobs):
    source = INPUTS / "c-gateway-wave-pin/fluent-bit-clang-source"
    monkey = INPUTS / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"
    package = INPUTS / "provider-packages-wave20/qcore.chunkio"
    out.mkdir(parents=True, exist_ok=False)
    command = [str(binary), "pinned", "--jobs", str(jobs), str(source), str(monkey),
               str(out / "monkey-source.json"), str(out / "resource-diagnostics.json")]
    environment = dict(os.environ, QCORE_CLANG="/usr/bin/clang",
                       QCORE_CHUNKIO_PACKAGE=str(package),
                       QCORE_TYPED_PATH_PARITY=str(out / "parity.jsonl"))
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment.pop("QCORE_RETURN_CENSUS", None)
    record = {"command": command, "source_commit": commit,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "environment": {"QCORE_CLANG": "/usr/bin/clang",
                              "QCORE_CHUNKIO_PACKAGE": str(package),
                              "QCORE_SOURCE_CONTRACTS": "unset",
                              "QCORE_RETURN_CENSUS": "unset",
                              "QCORE_AUDIT_JOBS": str(jobs)},
              "start": stamp()}
    with (out / "audit.log").open("wb") as log:
        code = subprocess.run(command, env=environment, stdout=log, stderr=log,
                              check=False).returncode
    record.update(exit=code, end=stamp())
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    return code


def system(binary, out, commit, jobs):
    source = PREPARED / "fluent-bit-clang-source"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"],
                                       text=True).strip()
    if revision != SLICE:
        raise SystemExit("prepared source revision does not match the selected slice")
    if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
        raise SystemExit("prepared source is dirty")
    out.mkdir(parents=True, exist_ok=False)
    command = [str(binary), "current", "--checkout", str(source),
               "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
               "--compile-database",
               str(PREPARED / "fluent-bit-clang-production/compile_commands.json"),
               "--selection", str(ROOT / "contract/profiles/fluent-bit/system-selection.json"),
               "--jobs", str(jobs), "--report", str(out / "resource-diagnostics.json")]
    environment = dict(os.environ, QCORE_CLANG="/usr/lib/llvm-21/bin/clang",
                       QCORE_TYPED_PATH_PARITY=str(out / "parity.jsonl"))
    record = {"command": command, "source_revision": revision, "qcore_revision": commit,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "QCORE_CLANG": environment["QCORE_CLANG"], "start": stamp()}
    with (out / "console.log").open("wb") as stdout, (out / "stderr.log").open("wb") as stderr:
        code = subprocess.run(command, cwd=ROOT, env=environment, stdout=stdout,
                              stderr=stderr).returncode
    record.update(exit=code, end=stamp())
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    (out / "exit").write_text(f"{code}\n")
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("commit")
    parser.add_argument("--jobs", type=int, default=5)
    parser.add_argument("--scope", choices=("corpus", "system", "both"), default="both")
    args = parser.parse_args()
    binary = args.binary.resolve()
    run = args.run.resolve()
    run.mkdir(parents=True, exist_ok=True)
    worst = 0
    if args.scope in ("corpus", "both"):
        code = corpus(binary, run / "corpus", args.commit, args.jobs)
        (run / "corpus.exit").write_text(f"{code}\n")
        worst = worst or code
    if args.scope in ("system", "both"):
        code = system(binary, run / "system", args.commit, args.jobs)
        (run / "system.exit").write_text(f"{code}\n")
        worst = worst or code
    return worst


if __name__ == "__main__":
    raise SystemExit(main())
