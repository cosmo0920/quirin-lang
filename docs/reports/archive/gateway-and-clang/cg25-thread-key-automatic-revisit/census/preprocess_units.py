"""Preprocess every evidence unit that names a thread-key call (read-only inputs).

Usage: preprocess_units.py <run-dir> <out-dir>
Writes <out-dir>/<n>.i and <out-dir>/units.tsv (index, source file).
"""
import json
import os
import re
import shlex
import subprocess
import sys

run, out_dir = sys.argv[1], sys.argv[2]
os.makedirs(out_dir, exist_ok=True)
pattern = re.compile(
    r"pthread_(get|set)specific|pthread_key_(create|delete)|FLB_TLS_(GET|SET|INIT)|MK_TLS_(GET|SET|INIT)"
)
dbs = [
    os.path.join(run, "fluent-bit-clang-production/compile_commands.json"),
    os.path.join(run, "monkey/monkey-build/compile_commands.json"),
]
entries = []
seen = set()
for db in dbs:
    for entry in json.load(open(db)):
        path = entry["file"]
        if not os.path.isabs(path):
            path = os.path.join(entry["directory"], path)
        if path in seen or not path.endswith(".c") or "/lib/" in path and "/lib/monkey/" not in path:
            continue
        try:
            text = open(path, errors="replace").read()
        except OSError:
            continue
        if not pattern.search(text):
            continue
        seen.add(path)
        entries.append((path, entry))
index = []
for n, (path, entry) in enumerate(entries):
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
    out[0] = "clang"
    target = os.path.abspath(os.path.join(out_dir, f"{n:03d}.i"))
    out += ["-E", "-P", "-o", target]
    code = subprocess.run(out, cwd=entry["directory"], stderr=subprocess.DEVNULL).returncode
    index.append(f"{n:03d}\t{code}\t{path}")
open(os.path.join(out_dir, "units.tsv"), "w").write("\n".join(index) + "\n")
print(len(index), "units")
