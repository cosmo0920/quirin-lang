#!/usr/bin/env python3
"""Name the difference of every differing file of a same-path comparison.

Usage: explain_logs.py LEFT RIGHT COMPARE_JSON OUT_JSON [LEFT_CWD RIGHT_CWD]

For each file `compare_trees.py` listed as differing, cumulative masks are
tried in order, each applied to both sides: (1) CMake's random try-compile
target names `cmTC_xxxxx` and `TryCompile-xxxxxx` scratch directories; (2)
CMake's step timings `(12.3s)`; (3) make's progress percentages `[ 9%]`,
which follow the `-j2` completion order; (4) with LEFT_CWD and RIGHT_CWD, each
relative entry of this process's `PATH` resolved against LEFT_CWD spelled as
resolved against RIGHT_CWD (CMake resolves a relative `PATH` entry against its
working directory: the script's was the repository, the plugin's its output);
then (5) the order of lines (`-j2` interleaving). The first mask
under which the two sides are equal is the file's explanation; a file equal
under none is `unexplained`. Exits 0 whatever it finds.
"""

import json
import os
import pathlib
import re
import sys

TRY_COMPILE = (re.compile(rb"cmTC_[0-9a-f]{5}|TryCompile-[0-9A-Za-z]{6}"), b"cmTC_#####")
TIMING = (re.compile(rb"\(\d+(\.\d+)?s\)"), b"(#s)")
PROGRESS = (re.compile(rb"\[ *\d+%\]"), b"[#%]")


def masks(cwd_pair):
    steps = [
        ("random try-compile names", [TRY_COMPILE]),
        ("random try-compile names and step timings", [TRY_COMPILE, TIMING]),
        ("try-compile names, timings and -j2 progress percentages",
         [TRY_COMPILE, TIMING, PROGRESS]),
    ]
    if cwd_pair:
        relative = [entry for entry in os.environ.get("PATH", "").split(os.pathsep)
                    if entry and not os.path.isabs(entry)]
        rules = [TRY_COMPILE, TIMING, PROGRESS]
        for entry in relative:
            left = os.path.normpath(os.path.join(cwd_pair[0], entry)).encode()
            right = os.path.normpath(os.path.join(cwd_pair[1], entry)).encode()
            rules.append((re.compile(re.escape(left)), right))
        steps.append((
            "try-compile names, timings, progress, and relative PATH entries "
            f"{relative} resolved against the working directory",
            rules,
        ))
    return steps


def masked(data, rules):
    for pattern, replacement in rules:
        data = pattern.sub(replacement, data)
    return data


def main():
    left, right = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    differ = json.loads(pathlib.Path(sys.argv[3]).read_text())["differ"]
    steps = masks(sys.argv[5:7] if len(sys.argv) >= 7 else None)
    out = {}
    for name in differ:
        a, b = (left / name).read_bytes(), (right / name).read_bytes()
        explanation = "unexplained"
        for label, rules in steps:
            if masked(a, rules) == masked(b, rules):
                explanation = label
                break
        else:
            rules = steps[-1][1]
            if sorted(masked(a, rules).splitlines()) == sorted(masked(b, rules).splitlines()):
                explanation = "line order only (parallel build interleaving), after the masks"
        out[name] = explanation
    pathlib.Path(sys.argv[4]).write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
