#!/usr/bin/env python3
"""layout.py DATABASE UNIT RECORD OUT.txt -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION:
Clang's record layout (`-Xclang -fdump-record-layouts`) of RECORD as the
unit's own recorded command compiles it, written to OUT.txt. A reading aid
for the layout route (brief option 2)."""
import json
import shlex
import subprocess
import sys

database, unit, record, out = sys.argv[1:]
entry = next(e for e in json.load(open(database)) if e["file"].endswith(unit))
words = shlex.split(entry["command"])
command, skip = [], False
for word in words:
    if skip:
        skip = False
        continue
    if word == "-o":
        skip = True
        continue
    if word == "-c":
        continue
    command.append(word)
command += ["-fsyntax-only", "-Xclang", "-fdump-record-layouts-complete"]
proc = subprocess.run(command, cwd=entry["directory"], capture_output=True, text=True)
text = proc.stdout
blocks = text.split("*** Dumping AST Record Layout")
wanted = [b for b in blocks if b.lstrip().split("\n", 1)[0].strip().endswith(f"struct {record}")]
with open(out, "w") as handle:
    handle.write(f"exit {proc.returncode}\n")
    for block in wanted[:1]:
        handle.write(block)
print(len(wanted), "layout(s) of", record)
