#!/usr/bin/env python3
"""Apply W47-CALLEE-CLOSURE-R2's matrix edits once, on the merged tree.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: no registered entry is discharged whole by this card
  (W46-EXTERNAL-RESIDUAL-RANKING's 30 functions are measured again, unchanged);
* amends four entries with this card's measured part, each by an appended
  sentence that starts with MARK (skipped when the marker is already present);
* appends every entry of `followups.json` (refusing a duplicate id).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-callee-closure-r2"
MARK = "AMENDED (W47-CALLEE-CLOSURE-R2):"

AMEND = {
    "W46-EXTERNAL-RESIDUAL-RANKING": (
        " " + MARK + " re-measured on the post-P3 corpus (main 3ba72320, 507 / 9 / 4,318): the same "
        "30 functions, in the same classes, are blocked alone by one external callee; P3 exposed "
        "three more (cfl_arena_calloc 2, rd_file_read 1), declared by this card (+3 Safe). The "
        "round-2 ranking, with the one-other-blocker tier, is W47-EXTERNAL-RESIDUAL-RANKING-R2."
    ),
    "W46-HEADER-INLINE-DECLARED-UNCHECKED": (
        " " + MARK + " a third such declaration: rd_file_read (lib/librdkafka-2.15.0/src/rd.h:472-521, "
        "`static RD_INLINE`), declared owned/nullable in qcore-provider-librdkafka and pinned at "
        "66910c10 and 046accf6. 2 call sites; rd_kafka_oidc_assertion_read_from_file moved to Safe on it."
    ),
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        " " + MARK + " the closure re-measured over the measured post-P3 diagnostics "
        "(docs/reports/w47-callee-closure-r2/artifacts/ranking/): 374 Unknown functions are blocked "
        "only by in-corpus callees; the existing same-unit summary rule frees 0 of them; a cross-unit "
        "route would free 3 in one round (cio_file_real_size, cmt_initialize, ctxTypeID_to_size) and "
        "nothing in a second, even with every freed function taken Safe. The 374 wait on Unknown "
        "callees (cJSON_New_Item 10, LZ4_decompress_generic 9, cmt_mpack_unpack_array and "
        "ctr_mpack_unpack_array 5 each)."
    ),
    "NESTED-CALL-IN-ARGUMENT-POSITION": (
        " " + MARK + " a nested call whose inner callee is a Safe same-unit function is not served "
        "by its summary either: snappy.c::hash waits only on get_unaligned32 in that position. This "
        "card's declarations exchanged 4 uncontracted-call leaves of append_sample_to_metric for "
        "`nested call get_context_arena` leaves at the same calls."
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
