#!/usr/bin/env python3
"""ast_dump.py UNIT FUNCTION OUT -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: dump
Clang's JSON AST of FUNCTION (an `-ast-dump-filter`) from the corpus unit
UNIT (a path suffix, e.g. `librdkafka-2.15.0/src/rdkafka_op.c`) with the
unit's own production command, into OUT. A reading aid for the census and
for the shapes the caller branch check reads."""
import json
import shlex
import subprocess
import sys

DATABASE = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-production/compile_commands.json"
CLANG = "/usr/lib/llvm-21/bin/clang"


def main():
    unit, function, out = sys.argv[1:4]
    entries = json.load(open(DATABASE))
    entry = next(e for e in entries if e["file"].endswith(unit))
    arguments = entry.get("arguments") or shlex.split(entry["command"])
    keep = []
    skip = False
    for argument in arguments[1:]:
        if skip:
            skip = False
            continue
        if argument in ("-o", "-c"):
            skip = argument == "-o"
            continue
        if argument == entry["file"] or argument.endswith(unit):
            continue
        keep.append(argument)
    command = [CLANG, *keep, "-fsyntax-only", "-Xclang", "-ast-dump=json",
               "-Xclang", f"-ast-dump-filter={function}", entry["file"]]
    with open(out, "w") as handle:
        result = subprocess.run(command, cwd=entry["directory"], stdout=handle,
                                stderr=subprocess.PIPE, text=True, check=False)
    print(result.returncode, result.stderr[-2000:])


if __name__ == "__main__":
    main()
