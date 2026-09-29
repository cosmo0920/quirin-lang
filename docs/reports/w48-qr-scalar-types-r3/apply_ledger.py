#!/usr/bin/env python3
"""Apply this card's follow-up ledger to the acceptance matrix's hand-maintained
`named_followups` section (card W48-QR-SCALAR-TYPES-R3).

Usage: python3 docs/reports/w48-qr-scalar-types-r3/apply_ledger.py

Reads `followups.json` beside this script: every `retired` id must be present
and is removed; every `registered` id must be absent and is appended; every
`amended` id must be present and gains `Amended by W48-QR-SCALAR-TYPES-R3: …`
at the end of its `why`, the wording earlier cards used. Refuses (writing
nothing) on any mismatch. The derived sections are regenerated afterwards by
docs/reports/remaining-d02/build_matrix.py.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
LEDGER = pathlib.Path(__file__).resolve().parent / "followups.json"
CARD = "W48-QR-SCALAR-TYPES-R3"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    ledger = json.loads(LEDGER.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    for retired in ledger["retired"]:
        if retired["id"] not in ids:
            sys.exit(f"retired id {retired['id']} is not registered")
    for registered in ledger["registered"]:
        if registered["id"] in ids:
            sys.exit(f"registered id {registered['id']} already exists")
    for amended in ledger["amended"]:
        if amended["id"] not in ids:
            sys.exit(f"amended id {amended['id']} is not registered")
    retired = {entry["id"] for entry in ledger["retired"]}
    entries = [entry for entry in entries if entry["id"] not in retired]
    amendments = {entry["id"]: entry["amendment"] for entry in ledger["amended"]}
    for entry in entries:
        if entry["id"] in amendments:
            text = amendments[entry["id"]]
            if not text.startswith(CARD):
                sys.exit(f"amendment of {entry['id']} must name {CARD}")
            entry["why"] = f"{entry['why']} Amended by {CARD}: {text}"
    entries.extend(ledger["registered"])
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
