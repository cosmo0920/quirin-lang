#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: the ledger edits, applied once (idempotent).

usage: update_ledger.py MATRIX_JSON

Amends CG12-VOID-VIEW-READBACK and W47-FIELDS-BY-MEMBER-VOID-ESCAPES with what
landed and what remains (see the report's §8 for why neither is retired),
registers the residual classes with their sizes, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ledger_text  # noqa: E402  (AMEND, REGISTER)

BY = "CG12-VOID-VIEW-READBACK"
REPORT = "docs/reports/cg12-void-view-readback/README.md"


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    for entry in entries:
        amendment = ledger_text.AMEND.get(entry["id"])
        if amendment and amendment not in entry["why"]:
            entry["why"] += amendment
    for new in ledger_text.REGISTER:
        if new["id"] in ids:
            continue
        entries.append({**new, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
