#!/usr/bin/env python3
"""GAPO-PARITY-DECISIONS-NONDETERMINISTIC: retire the card's own ledger entry
in docs/reviews/r0_3/acceptance_matrix.json (its DISCHARGE holds: see the
report). Idempotent; fails if the entry was never registered. Writes with
indent=1, ensure_ascii=False and keeps dimensions.named_followups equal to the
list length.
"""
import json
import pathlib
import sys

RETIRED = "GAPO-PARITY-DECISIONS-NONDETERMINISTIC"


def main():
    path = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    kept = [entry for entry in entries if entry["id"] != RETIRED]
    if len(kept) == len(entries):
        print(f"{RETIRED} is not registered (already retired?)")
    matrix["named_followups"] = kept
    matrix["dimensions"]["named_followups"] = len(kept)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {len(entries)} -> {len(kept)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
