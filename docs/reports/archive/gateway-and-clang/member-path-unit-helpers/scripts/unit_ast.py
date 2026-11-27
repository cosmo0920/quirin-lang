#!/usr/bin/env python3
"""unit_ast.py SOURCE OUT -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: write
the Clang JSON AST of one corpus unit (SOURCE relative to the pinned Fluent
Bit tree, e.g. lib/librdkafka-2.15.0/src/rdkafka_partition.c) to OUT, with
the unit's own command from the pinned compile database (its -I/-D/-std
flags; output and dependency flags dropped). Read-only on the corpus; a
reading aid for the probe (`examples/member_path_probe.rs`), not the arbiter.
"""
import json
import pathlib
import shlex
import subprocess
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")
DATABASE = ROOT / "fluent-bit-clang-production/compile_commands.json"
CLANG = "/usr/lib/llvm-21/bin/clang"


def main():
    source, out = sys.argv[1], pathlib.Path(sys.argv[2])
    entries = json.loads(DATABASE.read_text())
    entry = next(e for e in entries if e["file"].endswith("/" + source))
    words = entry.get("arguments") or shlex.split(entry["command"])
    keep = []
    skip = False
    for word in words[1:]:
        if skip:
            skip = False
            continue
        if word in ("-o", "-MF", "-MT", "-MQ", "-c"):
            skip = word != "-c"
            continue
        if word.startswith(("-M", "-o")) or word == entry["file"] or word.endswith(".c"):
            continue
        keep.append(word)
    command = [CLANG, *keep, "-fsyntax-only", "-Xclang", "-ast-dump=json", entry["file"]]
    with out.open("w") as handle:
        result = subprocess.run(command, cwd=entry["directory"], stdout=handle,
                                stderr=subprocess.PIPE, text=True, check=False)
    sys.stderr.write(result.stderr[-2000:])
    sys.exit(result.returncode)


main()
