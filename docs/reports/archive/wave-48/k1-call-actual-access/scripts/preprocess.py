#!/usr/bin/env python3
"""preprocess.py COMPILE_DB SITES_JSONL OUTDIR -- K1 (AG1's preprocess.py): for every
non-Monkey unit named in SITES_JSONL (sites.py output), reproduce the gateway's plain
preprocessing (`clang -x c -std=... -E -P` with the unit's -I/-D options) into
OUTDIR/<unit path>.i, so the report's `line` (the -E -P line) can be read.
Read-only on the inputs."""
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
    database, sites, outdir = sys.argv[1:4]
    wanted = sorted({json.loads(line)["file"] for line in open(sites)})
    entries = {}
    for entry in json.load(open(database)):
        entries.setdefault(entry["file"], entry)
    failures = 0
    done = 0
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
        done += 1
    print(len(wanted), "units,", done, "preprocessed,", failures, "failures")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
