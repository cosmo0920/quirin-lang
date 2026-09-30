#!/usr/bin/env python3
"""Apply this card's follow-up ledger to the acceptance matrix's hand-maintained
`named_followups` section (card W48-QRLIB-LINKAGE).

Usage: python3 docs/reports/w48-qrlib-linkage/apply_ledger.py

Reads `followups.json` beside this script. **Idempotent**, so it can be rerun
after another card lands on main: a `retired` id is removed when present (and
already-absent is reported, not an error); a `registered` id is appended when
absent and left alone when an entry with the same id, title and why is already
there; an `amended` id gains `Amended by W48-QRLIB-LINKAGE: …` at the end
of its `why` unless that exact sentence is already there. Refuses (writing
nothing) on a conflict: an amended id that is not registered, or a registered
id already present with other content. The derived sections are regenerated
afterwards by docs/reports/remaining-d02/build_matrix.py.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
LEDGER = pathlib.Path(__file__).resolve().parent / "followups.json"
CARD = "W48-QRLIB-LINKAGE"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    ledger = json.loads(LEDGER.read_text())
    entries = matrix["named_followups"]
    by_id = {entry["id"]: entry for entry in entries}
    before = len(entries)
    for registered in ledger["registered"]:
        present = by_id.get(registered["id"])
        if present is not None and present != registered:
            sys.exit(f"registered id {registered['id']} already exists with other content")
    for amended in ledger["amended"]:
        if amended["id"] not in by_id:
            sys.exit(f"amended id {amended['id']} is not registered")
        if not amended["amendment"].startswith(CARD):
            sys.exit(f"amendment of {amended['id']} must name {CARD}")
    notes = []
    retired = {entry["id"] for entry in ledger["retired"]}
    for identifier in sorted(retired - set(by_id)):
        notes.append(f"retired id {identifier} already absent")
    entries = [entry for entry in entries if entry["id"] not in retired]
    amendments = {entry["id"]: entry["amendment"] for entry in ledger["amended"]}
    for entry in entries:
        text = amendments.get(entry["id"])
        if text is None:
            continue
        sentence = f" Amended by {CARD}: {text}"
        if sentence in entry["why"]:
            notes.append(f"amendment of {entry['id']} already applied")
        else:
            entry["why"] = f"{entry['why']}{sentence}"
    for registered in ledger["registered"]:
        if registered["id"] in by_id:
            notes.append(f"registered id {registered['id']} already present")
        else:
            entries.append(registered)
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    for note in notes:
        print(note)
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
