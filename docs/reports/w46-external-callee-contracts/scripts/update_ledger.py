#!/usr/bin/env python3
"""Apply W46-EXTERNAL-CALLEE-CONTRACTS's matrix edits once, on the merged tree.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: no registered entry is discharged whole by this card;
* amends four entries with this card's measured part, each by an appended
  sentence that starts with MARK (skipped when the marker is already present);
* appends every entry of `followups.json` (refusing a duplicate id).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w46-external-callee-contracts"
MARK = "AMENDED (W46-EXTERNAL-CALLEE-CONTRACTS):"

AMEND = {
    "W44-CALLEE-EXTERNAL-INLINE-DEFINITION": (
        " " + MARK + " the second discharge arm is taken for the two largest callees: "
        "mpack_finish_map and mpack_finish_array are declared borrowed/none in "
        "quirin-provider-mpack (mpack/writer.qcore), pinned at 66910c10 and 046accf6 "
        "after reading all four vendored copies (the amalgamation's finishers add a "
        "builder pop; the custody effect agrees). 50 leaves of the corpus (27 + 23) are "
        "removed and pack_basic_header_finish moves to Safe. Open: the other 14 callees."
    ),
    "NESTED-CALL-IN-ARGUMENT-POSITION": (
        " " + MARK + " the named instance is discharged by declaring the inner callee: "
        "with cfl_sds_len declared (quirin-provider-cfl), `mpack_write_str(writer, str, "
        "cfl_sds_len(str));` is admitted and pack_string moves to Safe, so the note that "
        "declaring the inner callee alone does not help no longer holds at this base "
        "(later cards admit a nested call whose inner callee is contracted). Open: 135 "
        "`nested call` leaves on the AFTER baseline, including pack_metric's."
    ),
    "W43-DESCRIPTOR-REFUSED-SHAPES": (
        " " + MARK + " fcntl is declared at F_SETFD only (libc/unistd.qcore, "
        "`resource_literal_argument(cmd, 2)`, the prctl precedent), which is exact there; "
        "every other command stays undeclared (the slice's F_GETPIPE_SZ site now reads "
        "`outside its declared literal-argument premise`). rd_kafka_open_cb_generic is "
        "still Unknown: its extra `&on` tail actual is refused (W46-FCNTL-SETFD-ADDRESS-TAIL). "
        "The per-command premise set for the other commands stays open."
    ),
    "CTRACES-FORMATTER-CHAIN": (
        " " + MARK + " cfl_sds_cat_safe cannot be declared exactly in v1: it reallocates "
        "the string behind its `cfl_sds_t *` slot (W46-SDS-CAT-SAFE-IN-PLACE-REALLOC). The "
        "chain waits on that vocabulary item."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    missing = sorted(set(AMEND) - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    amended = 0
    for entry in entries:
        if entry["id"] in AMEND and MARK not in entry["why"]:
            entry["why"] = entry["why"] + AMEND[entry["id"]]
            amended += 1
    entries.extend(new)
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(entries)}; amended {amended}; registered {len(new)}; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
