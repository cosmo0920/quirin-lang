#!/usr/bin/env python3
"""preprocess.py FILE_SUFFIX OUT.c -- K7: preprocess one production unit with
its own compile command (read from the user's compile database, read only),
for building faithful probes. Output under the caller's path."""
import json
import shlex
import subprocess
import sys

DB = ("/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/"
      "fluent-bit-clang-source/fluent-bit-clang-production/compile_commands.json")
suffix, out = sys.argv[1:3]
for entry in json.load(open(DB)):
    if entry["file"].endswith(suffix):
        argv = shlex.split(entry["command"])
        clean = []
        skip = False
        for arg in argv:
            if skip:
                skip = False
                continue
            if arg in ("-o", "-c"):
                skip = arg == "-o"
                continue
            if arg == entry["file"] or arg.endswith(suffix):
                continue
            clean.append(arg)
        clean += ["-E", entry["file"], "-o", out]
        sys.exit(subprocess.run(clean, cwd=entry["directory"]).returncode)
sys.exit("no such unit")
