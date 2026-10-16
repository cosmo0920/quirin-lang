#!/usr/bin/env python3
"""layout.py OUT_DIR -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: Clang's layout
of struct flb_processor_instance and struct flb_filter_instance in every
measured configuration, each with the unit's own recorded compile command.

For each configuration (a prepared Fluent Bit tree and its production compile
database) and each recorded command of src/flb_processor.c (the static and the
shared library objects), two Clang readings are taken, both -fsyntax-only (no
object, no dependency file; the read-only trees are never written):

1. `-Xclang -fdump-record-layouts-complete` over the unit itself: the two
   records' layout blocks are kept verbatim (OUT/<config>-<object>.layout.txt).
2. An offsetof/sizeof probe: a generated translation unit, compiled with the
   same command (same flags, same include paths, same directory), that includes
   the two headers and declares, for each top-level member M of
   struct flb_processor_instance, `char off_M[offsetof(T, M) + 1]` and
   `char size_M[sizeof(((T *)0)->M)]`, plus the two records' sizeof and a
   _Static_assert that the filter record is at least the processor record.
   Clang's own AST dump of the probe gives the array extents, so every number
   is Clang's (OUT/<config>-<object>.probe.json).

Writes OUT/layouts.json (all configurations) and prints a table. Exit 0 only
when every reading succeeded."""
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys

CONFIGS = [
    ("corpus-66910c10",
     "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin"),
    ("system-046accf6",
     "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1"),
    ("pins-046accf6",
     "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w48-callee-closure-r3/attempt-1"),
    ("slice-34155955",
     "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep"),
]
UNIT = "src/flb_processor.c"
RECORDS = ("flb_processor_instance", "flb_filter_instance")


def base_command(entry):
    words = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
    out, skip = [], False
    for word in words:
        if skip:
            skip = False
            continue
        if word == "-o":
            skip = True
            continue
        if word == "-c" or word == entry["file"] or word.endswith("/" + UNIT):
            continue
        out.append(word)
    return out


def layouts(text):
    found = {}
    for block in text.split("*** Dumping AST Record Layout"):
        head = block.lstrip().split("\n", 1)[0].strip()
        for record in RECORDS:
            if head.endswith(f"struct {record}") and record not in found:
                found[record] = block.strip("\n")
    return found


def top_fields(block):
    fields = []
    for line in block.splitlines()[1:]:
        m = re.match(r"\s*(\d+) \|   (?! )(.+?) (\w+)$", line)
        if m:
            fields.append({"offset": int(m.group(1)), "type": m.group(2), "name": m.group(3)})
    size = re.search(r"\[sizeof=(\d+), align=(\d+)", block)
    return fields, int(size.group(1)), int(size.group(2))


def probe(command, directory, fields, path):
    lines = [
        "#include <stddef.h>",
        "#include <fluent-bit/flb_processor.h>",
        "#include <fluent-bit/flb_filter.h>",
        "typedef struct flb_processor_instance P;",
        "typedef struct flb_filter_instance F;",
        "char size_P[sizeof(P)];",
        "char size_F[sizeof(F)];",
        "_Static_assert(sizeof(F) >= sizeof(P), \"filter instance smaller than processor instance\");",
    ]
    for f in fields:
        lines.append(f"char off_{f['name']}[offsetof(P, {f['name']}) + 1];")
        lines.append(f"char size_{f['name']}[sizeof(((P *)0)->{f['name']})];")
    with open(path, "w") as handle:
        handle.write("\n".join(lines) + "\n")
    proc = subprocess.run(command + ["-fsyntax-only", "-Xclang", "-ast-dump", path],
                          cwd=directory, capture_output=True, text=True)
    values = {}
    for m in re.finditer(r"VarDecl 0x[0-9a-f]+ <[^>]*> (?:col|line):[\d:]+ (\w+) 'char\[(\d+)\]'", proc.stdout):
        values[m.group(1)] = int(m.group(2))
    return proc.returncode, values, proc.stderr


def main():
    out = os.path.abspath(sys.argv[1])
    os.makedirs(out, exist_ok=True)
    result, ok = [], True
    for name, root in CONFIGS:
        database = os.path.join(root, "fluent-bit-clang-production/compile_commands.json")
        source = os.path.join(root, "fluent-bit-clang-source")
        rev = subprocess.run(["git", "-C", source, "rev-parse", "HEAD"],
                             capture_output=True, text=True).stdout.strip()
        db_sha = hashlib.sha256(open(database, "rb").read()).hexdigest()
        unit_sha = hashlib.sha256(open(os.path.join(source, UNIT), "rb").read()).hexdigest()
        entries = [e for e in json.load(open(database)) if e["file"].endswith(UNIT)]
        for entry in entries:
            words = shlex.split(entry["command"]) if "command" in entry else entry["arguments"]
            obj = words[words.index("-o") + 1]
            tag = "static" if "fluent-bit-static" in obj else ("shared" if "fluent-bit-shared" in obj else obj)
            command = base_command(entry)
            dump = subprocess.run(command + ["-fsyntax-only", "-Xclang", "-fdump-record-layouts-complete", entry["file"]],
                                  cwd=entry["directory"], capture_output=True, text=True)
            found = layouts(dump.stdout)
            stem = os.path.join(out, f"{name}-{tag}")
            with open(stem + ".layout.txt", "w") as handle:
                handle.write(f"exit {dump.returncode}\n")
                for record in RECORDS:
                    handle.write(found.get(record, f"(no layout of struct {record})") + "\n\n")
            row = {"config": name, "revision": rev, "compile_database": database,
                   "compile_database_sha256": db_sha, "unit_sha256": unit_sha,
                   "object": obj, "variant": tag, "directory": entry["directory"],
                   "compiler": command[0], "layout_exit": dump.returncode}
            if dump.returncode != 0 or set(found) != set(RECORDS):
                ok = False
                result.append(row)
                continue
            pf, ps, pa = top_fields(found["flb_processor_instance"])
            ff, fs, fa = top_fields(found["flb_filter_instance"])
            code, values, err = probe(command, entry["directory"], pf, stem + ".probe.c")
            row.update({
                "processor_sizeof": ps, "processor_align": pa,
                "filter_sizeof": fs, "filter_align": fa,
                "filter_minus_processor": fs - ps,
                "filter_has_match_regex": any(f["name"] == "match_regex" for f in ff),
                "filter_has_metrics": any(f["name"] == "metrics" for f in ff),
                "processor_members": [],
                "probe_exit": code,
                "probe_sizeof_processor": values.get("size_P"),
                "probe_sizeof_filter": values.get("size_F"),
            })
            for f in pf:
                off = values.get("off_" + f["name"])
                size = values.get("size_" + f["name"])
                end = None if off is None or size is None else off - 1 + size
                row["processor_members"].append({
                    "name": f["name"], "type": f["type"], "layout_offset": f["offset"],
                    "probe_offset": None if off is None else off - 1, "size": size,
                    "end": end, "within_filter_sizeof": None if end is None else end <= fs})
            agree = (code == 0 and values.get("size_P") == ps and values.get("size_F") == fs
                     and all(m["probe_offset"] == m["layout_offset"] for m in row["processor_members"]))
            row["layout_and_probe_agree"] = agree
            ok = ok and agree and all(m["within_filter_sizeof"] for m in row["processor_members"])
            with open(stem + ".probe.json", "w") as handle:
                json.dump({"exit": code, "values": values, "stderr_tail": err[-2000:]}, handle, indent=1)
            result.append(row)
    with open(os.path.join(out, "layouts.json"), "w") as handle:
        json.dump(result, handle, indent=1)
        handle.write("\n")
    for row in result:
        print(f"{row['config']:18} {row.get('variant','?'):7} rev {row['revision'][:8]} "
              f"processor {row.get('processor_sizeof')} filter {row.get('filter_sizeof')} "
              f"regex {row.get('filter_has_match_regex')} metrics {row.get('filter_has_metrics')} "
              f"agree {row.get('layout_and_probe_agree')}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
