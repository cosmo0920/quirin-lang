#!/usr/bin/env python3
"""Write a copy of the pinned Fluent Bit Clang database in which every ChunkIO
unit's command gains `-DQCORE_CHECKPOINT_PROBE=1`: one group's compile
configurations change, every other group's stay byte-identical.

Usage: probe_database.py OUT_PATH

OUT_PATH must not exist. The define is unused by the sources, so the change is
to the compile-task identity (command, definitions), which a resumed run must
notice for exactly the ChunkIO group.
"""

import json
import pathlib
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    if out.exists():
        raise SystemExit(f"{out} exists")
    source = PIN / "fluent-bit-clang-source/build/compile_commands.json"
    entries = json.loads(source.read_text())
    changed = 0
    for entry in entries:
        if "/lib/chunkio/src/" in entry["file"]:
            driver, rest = entry["command"].split(" ", 1)
            entry["command"] = f"{driver} -DQCORE_CHECKPOINT_PROBE=1 {rest}"
            changed += 1
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(entries, indent=2) + "\n")
    print(json.dumps({"entries": len(entries), "changed": changed}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
