#!/usr/bin/env python3
"""Replace the inclusive 1-based line range [start, end] of a file with the
contents of another file. Used to swap one compacted block in at a time."""
import sys

path, start, end, newfile = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
lines = open(path, encoding="utf-8").read().split("\n")
new = open(newfile, encoding="utf-8").read().split("\n")
if new and new[-1] == "":
    new = new[:-1]
before = len("\n".join(lines))
lines[start - 1:end] = new
out = "\n".join(lines)
open(path, "w", encoding="utf-8").write(out)
print(f"{path}: {before} -> {len(out)} ({len(out)-before:+d})")
