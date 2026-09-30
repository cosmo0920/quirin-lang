#!/usr/bin/env python3
"""B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (every DISCHARGE clause holds)
and registers FIELD-LINK-ESCAPE-PATH-CORRELATION (the repair's residual
imprecision, 0 measured functions); keeps `dimensions.named_followups` equal
to the list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's measurement runs (private/b08fl/{before,census,after}*,
corpus arbiter --jobs 8), read with scripts/escape.py and CG04's withheld.py.
"""

import json
import sys

REPORT = "docs/reports/b08-field-link-join-silent-unresolved/README.md"
BY = "B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED"

RETIRE = ["B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED"]

REGISTER = [
    {
        "id": "FIELD-LINK-ESCAPE-PATH-CORRELATION",
        "title": "An escape of an owner whose child is linked on one arm and discharged on the other is refused although every path is correct",
        "why": (
            "Residual of B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (gateway rule 11c). The flow joins a child's status bits "
            "across paths, so after `if (n) b->item = it; else k_release(it); return b;` it knows the child may be live "
            "(the linking arm) and is linked on some path only, but not that the live bit comes from exactly the linking "
            "arm. The escape therefore refuses (`field-link escape: ...`, Unknown) a program that is correct on every "
            "path; the refusal is the conservative reading, never an accusation "
            "(crates/qcore-c-gateway/tests/field_link_join.rs `released-arm-escape`). Measured on the card's AFTER: 0 "
            "functions in the corpus, the system slice and the tail-pause slice reach a partial-link escape at all "
            "(escape census, parity `position: field-link-escape`, `partial: true`: 0 rows). DISCHARGE: a path "
            "correlation between a field link and its child's status (for instance a per-link status carried with the "
            "partial triple) that escapes such a child without refusing, with Lean parity and a measured re-pin; or a "
            "recorded decision to keep the refusal."
        ),
    },
]


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in RETIRE:
        assert name in ids, f"{name} is not registered"
    for entry in REGISTER:
        assert entry["id"] not in ids, f"{entry['id']} already registered"
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in REGISTER:
        entries.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
