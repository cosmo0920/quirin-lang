#!/usr/bin/env python3
"""The three measurements this card must show unchanged, into a fresh directory.

Usage: arbiters.py OUT COMMIT RUN_NAME

The 185-unit corpus (quirin-audit pinned at 66910c10), the Fluent Bit system
slice at 046accf6 and the tail-pause slice at the 90978ac6 export (into
private/w47-tail/attempt-1/RUN_NAME), each with --jobs 8, every exit captured
to a file (W47-LOOP-OWNER-COROUTINE-HOP's arbiters.sh, as Python). Uses
target/release/quirin-audit as built by the caller; read-only on every tree.
"""

import hashlib
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]


def run(out, name, command, environment=None):
    with open(out / f"{name}.log", "wb") as log:
        code = subprocess.run(command, cwd=REPO, stdout=log, stderr=subprocess.STDOUT,
                              env=environment, check=False).returncode
    (out / f"{name}.exit").write_text(f"{code}\n")


def main():
    out, commit, name = pathlib.Path(sys.argv[1]).resolve(), sys.argv[2], sys.argv[3]
    out.mkdir(parents=True, exist_ok=False)
    binary = REPO / "target/release/quirin-audit"
    (out / "binary.sha256").write_text(
        f"{hashlib.sha256(binary.read_bytes()).hexdigest()}  {binary}\n")
    run(out, "corpus", ["python3", "scripts/run_pinned_audit.py", str(out / "corpus"), commit,
                        "--inputs-root", "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19",
                        "--jobs", "8"],
        dict(os.environ, QCORE_TYPED_PATH_PARITY=str(out / "corpus/parity.jsonl")))
    run(out, "system", ["python3", "docs/reports/w46-contract-gap-expansion/scripts/run_system.py",
                        str(out / "system"), "--jobs", "8"])
    run(out, "slice", ["bash", "docs/reports/w47-tail-pause-bindings/scripts/run-current.sh",
                       "attempt-1", name, str(binary), "--selection",
                       "contract/profiles/fluent-bit/tail-pause-selection.json", "--jobs", "8"])
    (out / "slice.path").write_text(
        f"/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/{name}\n")


if __name__ == "__main__":
    main()
