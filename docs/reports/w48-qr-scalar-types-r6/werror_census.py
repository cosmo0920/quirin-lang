#!/usr/bin/env python3
"""Compile every tracked source fixture's emitted C under `-Werror`.

Card W48-QR-SCALAR-TYPES-R6, item 5. Usage:

    python3 docs/reports/w48-qr-scalar-types-r6/werror_census.py QUIRIN OUT.json [--recorded-target FILE]

For every tracked `.qr`, `.qv` and `.qrlib` file (read from `git ls-files`,
never a tree walk: constraint 29) whose `QUIRIN emit FILE` exits 0, the
emitted C is compiled — compile only, `-c`, so a program linking a native
library needs none — with `clang -std=c11 -Wall -Wextra -Werror -pedantic`
(and `-I compiler/tests/fixtures/apple-threads` on macOS). The manifest
records, per file, the emit exit and the compile exit, and the first
diagnostic line of a failing compile, so two manifests (base and tip) compared
key by key name every file whose emitted C moved between building and not.
"""

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

SUFFIXES = (".qr", ".qv", ".qrlib")


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    quirin = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2])
    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True
        ).stdout.decode().strip()
    )
    listed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout.decode()
    files = sorted(p for p in listed.split("\0") if p.endswith(SUFFIXES))
    manifest = {
        "quirin_sha256": hashlib.sha256(quirin.read_bytes()).hexdigest(),
        "flags": "-std=c11 -Wall -Wextra -Werror -pedantic -c",
        "files": {},
    }
    counts = {"emitted": 0, "built": 0, "failed": 0}
    with tempfile.TemporaryDirectory() as scratch:
        for index, path in enumerate(files):
            emit = subprocess.run([str(quirin), "emit", path], cwd=root, capture_output=True)
            entry = {"emit": emit.returncode}
            if emit.returncode == 0:
                counts["emitted"] += 1
                c = Path(scratch) / f"{index}.c"
                c.write_bytes(emit.stdout)
                build = subprocess.run(
                    [
                        "clang",
                        "-std=c11",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic",
                        "-c",
                        str(c),
                        "-o",
                        str(Path(scratch) / f"{index}.o"),
                    ],
                    capture_output=True,
                )
                entry["compile"] = build.returncode
                if build.returncode == 0:
                    counts["built"] += 1
                else:
                    counts["failed"] += 1
                    first = next(
                        (
                            line
                            for line in build.stderr.decode(errors="replace").splitlines()
                            if "error" in line
                        ),
                        "",
                    )
                    entry["first_error"] = first.replace(str(c), "<emitted>.c")
            manifest["files"][path] = entry
    manifest["counts"] = counts
    out.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps(counts))
    return 0


if __name__ == "__main__":
    sys.exit(main())
