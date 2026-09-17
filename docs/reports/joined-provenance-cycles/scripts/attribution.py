#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CYCLES: check the change is confined to cyclic CFGs.

The Lean side proves the fixed-point equation has a unique solution on an
acyclic graph, so no acyclic body's verdict *can* move. This script is the
measured half of that claim over the pinned corpus: every leaf that moved
between BEFORE and AFTER must belong to a function the analyzer's own
instrumentation recorded as having a cyclic CFG. A single acyclic mover is a
defect, not a surprise, and this exits non-zero on one.

Usage: python3 measure/joined-provenance-cycles/attribution.py
"""
import json
import pathlib
import sys

M = pathlib.Path(__file__).resolve().parent
MEASURE = pathlib.Path("/tmp/jpc/measure2.jsonl")


def leaves(stage):
    doc = json.loads((M / "runs" / stage / "resource-diagnostics.json").read_text())
    out = set()
    for u in doc["units"]:
        for f in u.get("functions") or []:
            for d in f.get("diagnostics") or []:
                out.add((u["file"], f["function"], d["line"], d["category"], d["description"]))
    return out


def main():
    cyclic = {}
    for raw in MEASURE.read_text().splitlines():
        if raw.strip():
            r = json.loads(raw)
            cyclic[r["function"]] = cyclic.get(r["function"], False) or r["cyclic"]
    b, a = leaves("before"), leaves("after")
    moved = (a - b) | (b - a)
    print(f"moved leaves: {len(moved)}")
    unseen = sorted({k[1] for k in moved if k[1] not in cyclic})
    acyclic = sorted({k[1] for k in moved if cyclic.get(k[1]) is False})
    print(f"  in functions the instrumentation never saw: {len(unseen)}")
    for name in unseen[:20]:
        print(f"    UNSEEN {name}")
    print(f"  in functions recorded ACYCLIC:              {len(acyclic)}")
    for name in acyclic[:20]:
        print(f"    ACYCLIC MOVER {name}")
    ok = not acyclic and not unseen
    print("\nRESULT: " + ("every moved leaf is in a cyclic function." if ok
                          else "ATTRIBUTION FAILED — see the names above."))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
