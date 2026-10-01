#!/usr/bin/env python3
"""CG05-ELABORATOR-CALL-FOOTPRINTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG05-ELABORATOR-CALL-FOOTPRINTS (every DISCHARGE clause holds: the
elaborator's call events carry their contract footprint, `elaborate` asks
`check_with_footprints`, the lowering instantiates each call through
`instantiate_call`, measured, Lean `QCore.ElaboratorCallFootprints`) and
CG05-FOOTPRINT-ADDRESS-CONSERVATISM (its DISCHARGE's option (c): accepted as a
measured boundary, 25 / 14 re-measured, with the Lean statement that the
refusal is necessary without a premise), registers the card's one residual,
and keeps `dimensions.named_followups` equal to the list length. Writes the
matrix back in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: the AFTER corpus arbiter at the card's tip on main `0a22a998`
(private/cg05ef/after2), identical on `cfb5d8e1` (after3; the entry's text was
then pointed at `cfb5d8e1`), `scripts/census.py` and `scripts/predict.py`.
"""

import json
import sys

REPORT = "docs/reports/cg05-elaborator-call-footprints/README.md"
BY = "CG05-ELABORATOR-CALL-FOOTPRINTS"

RETIRE = ["CG05-ELABORATOR-CALL-FOOTPRINTS", "CG05-FOOTPRINT-ADDRESS-CONSERVATISM"]

REGISTER = [
    {
        "id": "CG05-ELABORATOR-FOOTPRINTLESS-CALLS",
        "title": "Elaborated statements whose call has no contract footprint (603 rows over 400 functions; 515 with the callee's own uncontracted leaf)",
        "why": (
            "Registered by CG05-ELABORATOR-CALL-FOOTPRINTS (gateway rule 11n). The elaborator now reads each call "
            "event's contract footprint and lowers the call through instantiate_call; a call with no footprint (no "
            "contract in force, an indirect callee, a container effect, a reallocated result, an arity mismatch) keeps "
            "the refusal `call ... unsequenced with another event (CG05)`. On the corpus arbiter at the card's tip "
            "(main 0a22a998): 603 de-duplicated `position: elaboration-*` rows over 400 functions (601 statements, 2 "
            "returns; 621 / 411 at the base), 515 of them with the leaf `call to uncontracted function X` for their own "
            "callee at their own line (unit-test helpers, cfl_kvlist_fetch, add, pthread_*, the *_destroy free "
            "callbacks), the rest with another leaf there (an indirect access the older routes refuse first: "
            "XXH_readLE32_align, rd_clock, rd_kafka_buf_write_*) or none. Further, 9 rows are refused by the "
            "application point for an owned result in a consumed value (rd_malloc 4, rd_calloc 3, calloc 2: use "
            "`Tested` owns nothing), by design. Returns and conditions holding calls are not lowered by this route "
            "(rd_kafka_broker_ApiVersion_at_least, ..._no_lock). DISCHARGE: per callee, a declared contract or a "
            "validated summary (the elaborator needs no change); a return/condition lowering only with its own "
            "measurement."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")


if __name__ == "__main__":
    main()
