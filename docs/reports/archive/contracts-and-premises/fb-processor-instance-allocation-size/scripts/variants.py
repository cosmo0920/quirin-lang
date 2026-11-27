#!/usr/bin/env python3
"""variants.py LAYOUT_DIR -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: derived
probe readings, labelled as derived (not a recorded Fluent Bit build).

Takes the system-046accf6 static command of src/flb_processor.c and the probe
layout.py generated for it (LAYOUT_DIR/system-046accf6-static.probe.c) and
compiles that probe -fsyntax-only under:
  * the recorded flags with -DFLB_HAVE_REGEX and -DFLB_HAVE_METRICS removed
    and, because the configured header defines them too, a private copy of
    that tree's include/fluent-bit/flb_info.h without the two #define lines
    placed first on the include path (LAYOUT_DIR/shim; the tree is not
    written): the two #ifdef members of struct flb_filter_instance absent;
  * the recorded flags with --target set to other data models Fluent Bit
    ships for (i686, armv7 hard-float, aarch64 Linux), and both #ifdef
    members absent as well.
A target whose system headers are missing fails to compile; that failure is
recorded, never treated as a size. Writes LAYOUT_DIR/variants.json."""
import json
import os
import re
import shlex
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import layout  # noqa: E402

out = os.path.abspath(sys.argv[1])
name, root = layout.CONFIGS[1]
database = os.path.join(root, "fluent-bit-clang-production/compile_commands.json")
entry = next(e for e in json.load(open(database))
             if e["file"].endswith(layout.UNIT) and "fluent-bit-static" in e["command"])
base = layout.base_command(entry)
probe = os.path.join(out, f"{name}-static.probe.c")
drop = {"-DFLB_HAVE_REGEX", "-DFLB_HAVE_METRICS"}
shim = os.path.join(out, "shim")
os.makedirs(os.path.join(shim, "fluent-bit"), exist_ok=True)
info = open(os.path.join(root, "fluent-bit-clang-source/include/fluent-bit/flb_info.h")).read().splitlines()
kept = [l for l in info if l.strip() not in ("#define FLB_HAVE_REGEX", "#define FLB_HAVE_METRICS")]
assert len(info) - len(kept) >= 2, "flb_info.h does not define both"
with open(os.path.join(shim, "fluent-bit/flb_info.h"), "w") as handle:
    handle.write("\n".join(kept) + "\n")
variants = [("recorded-without-regex-metrics", [], True)]
for target in ("i686-linux-gnu", "armv7a-linux-gnueabihf", "aarch64-linux-gnu"):
    variants.append((f"{target}", [f"--target={target}"], False))
    variants.append((f"{target}-without-regex-metrics", [f"--target={target}"], True))
rows = []
for label, extra, strip in variants:
    words = [w for w in base if not (strip and w in drop)]
    command = words[:1] + extra + ([f"-I{shim}"] if strip else []) + words[1:] + ["-fsyntax-only", "-Xclang", "-ast-dump", probe]
    proc = subprocess.run(command, cwd=entry["directory"], capture_output=True, text=True)
    values = {m.group(1): int(m.group(2)) for m in re.finditer(
        r"VarDecl 0x[0-9a-f]+ <[^>]*> (?:col|line):[\d:]+ (\w+) 'char\[(\d+)\]'", proc.stdout)}
    errors = [l for l in proc.stderr.splitlines() if " error: " in l][:5]
    # A reading with a compile error is recorded but its numbers are not taken.
    if proc.returncode != 0:
        values = {}
    rows.append({"variant": label, "extra_flags": extra, "removed_defines": sorted(drop) if strip else [],
                 "exit": proc.returncode, "sizeof_processor": values.get("size_P"),
                 "sizeof_filter": values.get("size_F"), "first_errors": errors})
with open(os.path.join(out, "variants.json"), "w") as handle:
    json.dump(rows, handle, indent=1)
    handle.write("\n")
for r in rows:
    print(f"{r['variant']:42} exit {r['exit']} processor {r['sizeof_processor']} filter {r['sizeof_filter']}"
          + (f"  {r['first_errors'][0][-160:]}" if r["first_errors"] else ""))
