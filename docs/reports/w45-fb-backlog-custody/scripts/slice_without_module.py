#!/usr/bin/env python3
"""Run the system slice once with `fluent_bit/storage_backlog.qcore` removed
from `cycle19-resource-contracts.json` (the profile manifest), to separate
what the checked specifications move in the slice from what the gateway
mechanisms move.

Usage: slice_without_module.py ATTEMPT RUN BINARY

Calls `scripts/run-current.sh ATTEMPT RUN BINARY --selection
contract/profiles/fluent-bit/system-selection.json --jobs 8` with the
manifest rewritten, then restores its bytes and asserts the SHA-256.
"""

import hashlib
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MANIFEST = ROOT / "cycle19-resource-contracts.json"
LINE = '    "providers/quirin-provider-fluent-bit/contracts/fluent_bit/storage_backlog.qcore",\n'


def main():
    attempt, run, binary = sys.argv[1:4]
    original = MANIFEST.read_bytes()
    digest = hashlib.sha256(original).hexdigest()
    text = original.decode()
    if text.count(LINE) != 1:
        raise SystemExit("module line not found exactly once")
    try:
        MANIFEST.write_text(text.replace(LINE, ""))
        code = subprocess.run(
            [
                str(ROOT / "docs/reports/w45-fb-backlog-custody/scripts/run-current.sh"),
                attempt,
                run,
                binary,
                "--selection",
                "contract/profiles/fluent-bit/system-selection.json",
                "--jobs",
                "8",
            ],
            cwd=ROOT,
        ).returncode
    finally:
        MANIFEST.write_bytes(original)
        os.utime(MANIFEST, None)
    assert hashlib.sha256(MANIFEST.read_bytes()).hexdigest() == digest, "manifest not restored"
    print(f"exit {code}; manifest restored {digest}")
    return code


if __name__ == "__main__":
    sys.exit(main())
