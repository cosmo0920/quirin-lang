#!/usr/bin/env python3
"""SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: the ledger edit, applied once.

usage: update_ledger.py MATRIX_JSON

Retires SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN. Its DISCHARGE holds: the corpus
arbiter ran with QCORE_BOUNDS_PROPERTY on main e0b92ece6, and its custody
report is byte-identical to main's recorded arbiter (bdf5f1c8...). The bounds
tally (162 / 0 / 0 / 4,759, unchanged since b6a3dc42) and the combined tally
(111 -> 113 Safe, cmt_initialize and unittest_telemetry_decode through
W45-CALLEE-CASCADE-RESIDUAL round 3's custody moves) are attributed against a
regenerated run at 43732604c whose rows equal CG16 transport's 79a90176... .
The STATUS bounds row is updated, and PREDICTION.md was committed (f60cb6a2d)
before the run. Keeps `dimensions.named_followups` equal to the list length.
Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline)."""

import json
import sys

RETIRE = {"SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN"}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    missing = RETIRE - {entry["id"] for entry in followups}
    if missing:
        raise SystemExit(f"not registered: {sorted(missing)}")
    matrix["named_followups"] = [entry for entry in followups if entry["id"] not in RETIRE]
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups), "->", len(matrix["named_followups"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
