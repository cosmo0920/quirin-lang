#!/usr/bin/env python3
"""ARM-OBLIGATIONS-UNIT-GRAIN: respell one route's report artifacts of a side
so that two sides measured in different runs can be compared token for token.

The two sides were measured by two `quirin-audit measure` runs (BASE as
`rev:3e53c34fe` in run 2, TIP as `rev:051aef7f2` in run 1, reused by run 2 as
`run:`), so their reports spell different side directories and different
build commits (`evidence_version`, `QCORE_AUDIT_BUILD_COMMIT`). This copies
every `*.json`/`*.jsonl` file of ROUTE_DIR into OUT with the side directory
PREFIX written `<SIDE>` and the full revision REV written `<REV>`, and
nothing else changed; RULE35-PILOT's `substitution_ids.py` then checks the
copies (build identities, which digest the side directory, are one-to-one
token substitutions there).

Usage: respell_sides.py ROUTE_DIR PREFIX REV OUT
"""
import sys
from pathlib import Path


def main():
    route, prefix, revision, out = Path(sys.argv[1]), sys.argv[2], sys.argv[3], Path(sys.argv[4])
    if out.exists():
        print(f"{out} exists; use a fresh path")
        return 2
    count = 0
    for path in sorted(route.rglob("*")):
        if not path.is_file() or path.suffix not in (".json", ".jsonl"):
            continue
        text = path.read_text().replace(prefix, "<SIDE>").replace(revision, "<REV>")
        target = out / path.relative_to(route)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        count += 1
    print(f"{count} file(s) respelled into {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
