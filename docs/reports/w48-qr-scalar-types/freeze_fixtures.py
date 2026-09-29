#!/usr/bin/env python3
"""Freeze `quirin check` and `quirin emit` over every tracked source fixture.

Card W48-QR-SCALAR-TYPES. Usage:

    python3 docs/reports/w48-qr-scalar-types/freeze_fixtures.py QUIRIN OUT.json [RAW_DIR]

For every tracked `.qr`, `.qv` and `.qrlib` file (read from `git ls-files`,
never a tree walk: constraint 29) the script runs `QUIRIN check FILE` and
`QUIRIN emit FILE` from the repository root, one file per invocation and no
options, and records each command's exit status and the SHA-256 of its stdout
and stderr. The emitted C is the `emit` stdout, and the `check` stdout is the
published verdict line with its declared assumptions, so a BEFORE and an AFTER
manifest compared key by key show every changed certificate or C byte.

`RAW_DIR`, when given, receives the raw streams (`<index>.check.out`, ...)
for diffing a changed entry. The manifest itself holds only digests.
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

SUFFIXES = (".qr", ".qv", ".qrlib")


def tracked(root: Path) -> list[str]:
    listed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout.decode()
    files = sorted(p for p in listed.split("\0") if p.endswith(SUFFIXES))
    if not files:
        raise SystemExit("no tracked source fixtures: refusing to freeze an empty set")
    return files


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    if len(sys.argv) not in (3, 4):
        print(__doc__, file=sys.stderr)
        return 2
    quirin = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2])
    raw = Path(sys.argv[3]) if len(sys.argv) == 4 else None
    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True
        ).stdout.decode().strip()
    )
    if raw:
        raw.mkdir(parents=True, exist_ok=True)
    manifest = {"quirin_sha256": digest(quirin.read_bytes()), "files": {}}
    for index, path in enumerate(tracked(root)):
        entry = {}
        for command in ("check", "emit"):
            run = subprocess.run(
                [str(quirin), command, path], cwd=root, capture_output=True, timeout=120
            )
            entry[command] = {
                "exit": run.returncode,
                "stdout_sha256": digest(run.stdout),
                "stderr_sha256": digest(run.stderr),
            }
            if raw:
                (raw / f"{index}.{command}.out").write_bytes(run.stdout)
                (raw / f"{index}.{command}.err").write_bytes(run.stderr)
        manifest["files"][path] = entry
    out.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    exits = {}
    for entry in manifest["files"].values():
        for command, result in entry.items():
            key = f"{command}:{result['exit']}"
            exits[key] = exits.get(key, 0) + 1
    print(f"froze {len(manifest['files'])} files; exits {json.dumps(exits, sort_keys=True)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
