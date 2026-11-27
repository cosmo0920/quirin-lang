#!/usr/bin/env python3
"""ARM-OBLIGATIONS: which corpus units carry rule 37's premise, against what
each unit's body evidence (rule 13g) says of the two specifications naming
`asserted_allocation` (`rd_calloc`, `rd_strdup`).

A unit's rows carry the premise when one of those specifications stands in its
analysis; standing needs the body in the derivation scope, so such a unit
examined the body (`validated` or `unvalidated` or `failed` in
`contract_body_evidence`). Prints the cross table of (carries, examined a
body) and exits 1 when a unit carries the premise without having examined
either body, or when the carrying set differs between the two row files.

Usage: carrying_units.py TIP_CORPUS_DIR
"""
import json
import sys
from collections import Counter
from pathlib import Path

NAMES = ("rd_calloc", "rd_strdup")


def main():
    report = json.loads((Path(sys.argv[1]) / "resource-diagnostics.json").read_text())
    table = Counter()
    bad = []
    for unit in report["units"]:
        carries = any(f.get("rule_obligations") for f in unit.get("functions", []))
        evidence = unit.get("contract_body_evidence") or {}
        examined = sorted(
            name
            for name in NAMES
            for kind in ("validated", "failed")
            if name in (evidence.get(kind) or {})
        ) + sorted(name for name in NAMES if name in (evidence.get("unvalidated") or []))
        all_rows = all(f.get("rule_obligations") for f in unit.get("functions", []))
        table[(carries, tuple(sorted(set(examined))), all_rows if carries else None)] += 1
        if carries and not examined:
            bad.append(f"{unit['library']}/{unit['file']}")
    for (carries, examined, all_rows), count in sorted(table.items(), key=str):
        print(f"carries={carries} examined={list(examined)} every_row={all_rows}: {count}")
    print(f"carrying without an examined body: {bad}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
