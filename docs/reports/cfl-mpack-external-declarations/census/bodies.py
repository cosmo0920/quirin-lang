#!/usr/bin/env python3
"""CFL-MPACK-EXTERNAL-DECLARATIONS: the facts the declarations are read under.

    python3 bodies.py ROOT_66910c10 ROOT_046accf6 ROOT_34155955 [COMPILE_DB ...]

Each ROOT is a Fluent Bit checkout at that revision. Prints, for each pair of
roots, the library files that differ (`lib/cfl/src`, `lib/cfl/include` and
the three library mpack copies); the files outside the mpack copies that name
an mpack callback installer or a stream/file initialiser (the callbacks the
declared reader and writer bodies can reach); and, per compile database, the
`-D` flags that would turn mpack's tracking on (DEBUG, _DEBUG, MPACK_DEBUG,
MPACK_READ_TRACKING, MPACK_WRITE_TRACKING).
"""
import json
import os
import re
import sys

LIBRARY = ["lib/cfl/src", "lib/cfl/include", "lib/cmetrics/lib/mpack",
           "lib/ctraces/lib/mpack", "lib/cprofiles/lib/mpack"]
GENERATED = {"lib/cfl/include/cfl/cfl_info.h"}  # written by CMake configure
USERS = ["src", "include", "plugins", "lib/cmetrics/src", "lib/cmetrics/include",
         "lib/ctraces/src", "lib/ctraces/include", "lib/cprofiles/src"]
INSTALLERS = re.compile(r"\bmpack_(writer_set_error_handler|writer_set_flush|writer_set_teardown|"
                        r"reader_set_fill|reader_set_skip|reader_set_error_handler|reader_set_teardown|"
                        r"writer_init_stream|reader_init_stream|writer_init_file|reader_init_file|"
                        r"writer_init_filename|reader_init_filename)\b")
TRACKING = re.compile(r"-D(DEBUG|_DEBUG|MPACK_DEBUG|MPACK_READ_TRACKING|MPACK_WRITE_TRACKING)\b")


def files(root, sub):
    out = {}
    for dirpath, _, names in os.walk(os.path.join(root, sub)):
        for name in names:
            path = os.path.join(dirpath, name)
            out[os.path.relpath(path, root)] = open(path, "rb").read()
    return out


def main():
    roots = sys.argv[1:4]
    databases = sys.argv[4:]
    tags = ["66910c10", "046accf6", "34155955"]
    trees = [{k: v for sub in LIBRARY for k, v in files(root, sub).items()} for root in roots]
    for i in range(1, 3):
        differ = sorted(k for k in set(trees[0]) | set(trees[i])
                        if trees[0].get(k) != trees[i].get(k))
        print(f"library files {tags[0]} vs {tags[i]}: {len(trees[0])} files; differ: {differ}"
              f" (generated, ignored: {sorted(set(differ) & GENERATED)})")
        assert set(differ) <= GENERATED, differ
    for tag, root in zip(tags, roots):
        hits = []
        for sub in USERS:
            for rel, data in files(root, sub).items():
                if rel.endswith((".c", ".h")) and INSTALLERS.search(data.decode("utf-8", "replace")):
                    hits.append(rel)
        print(f"{tag}: callback installers outside the mpack copies: {sorted(hits)}")
    for database in databases:
        units = []
        for entry in json.load(open(database)):
            command = entry.get("command") or " ".join(entry.get("arguments", []))
            flags = sorted(set(TRACKING.findall(command)))
            if flags:
                units.append((entry["file"].split("fluent-bit-clang-source/")[-1], flags))
        print(f"{database}: units defining a tracking flag: {units}")


if __name__ == "__main__":
    main()
