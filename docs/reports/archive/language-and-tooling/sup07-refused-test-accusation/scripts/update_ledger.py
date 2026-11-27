#!/usr/bin/env python3
"""SUP07-REFUSED-TEST-ACCUSATION: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires MONKEY-REFUSED-NULL-TEST-JOINED-LEAK (its DISCHARGE holds: the
correlation of a nullable site with its binding across the join, measured on
the arbiter with every withdrawn leak attributed, with Lean parity). Amends
LEAK-UNDECIDED-NULL-TEST: its mk_sched_add_connection half is discharged by
rule 12e with the tie names; its rd_kafka_snappy_java_uncompress half is not,
and the leaf it names is published on main (a correction to the entry's
"withdrawn"). Keeps `dimensions.named_followups` equal to the list length.
Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline).

Sizes: the corpus arbiter (66910c10) at BEFORE (cffe232d4), BEFORE2 (main
f50fa2009) and AFTER2 (the card's tip), private/sup07/*, scripts/changed.py
and scripts/classify_census.py (artifacts/).
"""

import json
import sys

RETIRE = {"MONKEY-REFUSED-NULL-TEST-JOINED-LEAK"}

AMEND = {
    "LEAK-UNDECIDED-NULL-TEST": (
        " AMENDED by SUP07-REFUSED-TEST-ACCUSATION (gateway rule 12e): the mk_sched_add_connection half is "
        "discharged. Its two leaves (mk_scheduler.c:210 from :201 and :205) rested only on the refused "
        "`if (!conn)` (leak-witness census: every path takes the test's null edge), and rule 12e settles both "
        "sites there: joined provenance over the tie names ties them to `conn` once `&conn->event` and "
        "`&conn->channel.streams` are read as member addresses (C11 6.5.2.3, 6.5.3.2), so the function is "
        "Unknown, not a provisional Unsafe (corpus 66910c10: leak 42 -> 39 with mk_http_error_page). The "
        "rd_kafka_snappy_java_uncompress half stays open and is corrected: its leaf at snappy.c:1712 (acquired "
        ":1705) is still published at main cffe232d4 and f50fa2009 (with six sibling exits of the same "
        "allocation in that Unknown function); the test it sits on is inside a refused __builtin_expect call, "
        "not a refused null test, so rule 12e does not reach it. DISCHARGE for the remaining half unchanged."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    present = {entry["id"] for entry in followups}
    missing = (RETIRE | set(AMEND)) - present
    if missing:
        raise SystemExit(f"not registered: {sorted(missing)}")
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    matrix["named_followups"] = [entry for entry in followups if entry["id"] not in RETIRE]
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups), "->", len(matrix["named_followups"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
