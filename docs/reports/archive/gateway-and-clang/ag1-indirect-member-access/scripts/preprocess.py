#!/usr/bin/env python3
"""preprocess.py COMPILE_DB ROWS.json OUTDIR [--all] -- AG1 census, step 2.

For every unit named in ROWS.json (default: units with a sole-blocker row;
--all: every unit with an access row), reproduce the gateway's plain
preprocessing (`clang -x c -std=... -E -P` with the unit's -I/-D options,
legacy_preparation.rs::preprocess_plain) into OUTDIR/<unit path>.i, so the
report's `line` (the -E -P line) can be read. Read-only on the inputs."""
import json
import os
import shlex
import subprocess
import sys

CLANG = os.environ.get("QCORE_CLANG", "/usr/lib/llvm-21/bin/clang")


def options(command: str):
    words = shlex.split(command)
    out = []
    dialect = "-std=gnu17"
    skip = False
    for index, word in enumerate(words[1:], start=1):
        if skip:
            skip = False
            continue
        if word in ("-I", "-D", "-include", "-isystem"):
            out += [word, words[index + 1]]
            skip = True
        elif word.startswith(("-I", "-D", "-U", "-isystem")):
            out.append(word)
        elif word.startswith("-std="):
            dialect = word
    return dialect, out


def main() -> int:
    database, rows_path, outdir = sys.argv[1:4]
    every = "--all" in sys.argv[4:]
    rows = json.load(open(rows_path))["rows"]
    wanted = sorted({row["unit"] for row in rows if every or row["sole"]})
    entries = {}
    for entry in json.load(open(database)):
        entries.setdefault(entry["file"], entry)
    failures = 0
    for unit in wanted:
        match = [entry for path, entry in entries.items() if path.endswith("/" + unit)]
        if not match:
            print("no compile entry", unit)
            failures += 1
            continue
        entry = match[0]
        dialect, extra = options(entry.get("command") or shlex.join(entry["arguments"]))
        target = os.path.join(outdir, unit + ".i")
        os.makedirs(os.path.dirname(target), exist_ok=True)
        with open(target, "w") as handle:
            result = subprocess.run(
                [CLANG, "-x", "c", dialect, "-E", "-P", *extra, entry["file"]],
                cwd=entry["directory"],
                stdout=handle,
                stderr=subprocess.DEVNULL,
            )
        if result.returncode != 0:
            print("preprocess failed", unit, result.returncode)
            failures += 1
    print(len(wanted), "units,", failures, "failures")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
