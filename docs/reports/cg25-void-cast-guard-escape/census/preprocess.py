"""Preprocess one unit with its recorded compile command (read-only inputs).

Usage: preprocess.py <compile_commands.json> <file-substring> <out.i>
"""
import json
import shlex
import subprocess
import sys

db = json.load(open(sys.argv[1]))
entry = next(e for e in db if sys.argv[2] in e["file"])
argv = entry.get("arguments") or shlex.split(entry["command"])
out = []
skip = False
for arg in argv:
    if skip:
        skip = False
        continue
    if arg in ("-o", "-c"):
        skip = arg == "-o"
        continue
    if arg.startswith("-o") and len(arg) > 2:
        continue
    out.append(arg)
out[0] = "/usr/lib/llvm-21/bin/clang"
out += ["-E", "-o", __import__("os").path.abspath(sys.argv[3])]
print(" ".join(shlex.quote(a) for a in out), file=sys.stderr)
sys.exit(subprocess.run(out, cwd=entry["directory"]).returncode)
