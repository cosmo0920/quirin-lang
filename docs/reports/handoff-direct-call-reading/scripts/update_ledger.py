#!/usr/bin/env python3
"""update_ledger.py RUN -- HANDOFF-DIRECT-CALL-READING: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent. RUN is the
measurement summary quoted in every amended or registered entry.

* Retired: `HANDOFF-DIRECT-CALL-READING` (its DISCHARGE holds: a direct call
  of a hand-off declaration splits the caller's state on the guard at the
  caller's own branch on the result, the caller branch check applied in the
  calling body, with Lean parity, measured on the four rows).
* Amended: `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` (now also the next blocker of
  the two direct callers the reading admits).
* Registered: the stored result tested later than the next statement (or
  initialised in its declaration), and the invoker check's converted store."""
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "HANDOFF-DIRECT-CALL-READING"
SOURCE = "docs/reports/handoff-direct-call-reading/README.md"
RUN = sys.argv[1]

RETIRE = ["HANDOFF-DIRECT-CALL-READING"]

AMEND_APPEND = {
    "UNION-OPAQUE-RECORD-WRITES-BY-TYPE": (
        f" Amended by {CARD} ({RUN}): the direct calls of rd_kafka_poll_cb in "
        "rd_kafka_consume0 and rd_kafka_consumer_close0, now read (rule 12m's direct split), "
        "refuse at the same invocation of rd_kafka_op_s.rko_u.offset_commit.cb "
        "(callback-member-open), the named leaf of those 2 rows; the blocker now covers 4 rows."
    ),
}

REGISTER = [
    {
        "id": "HANDOFF-DIRECT-LATER-BRANCH",
        "title": "A direct call of a hand-off declaration whose stored result is tested later than the next statement, or initialised in a declaration, is refused",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12m's direct split joins the taken and kept "
            "states only at the caller's own test of the result as rule 11r reads it: the call "
            "tested where it stands, or `NAME = f(...);` tested by the next statement. A result "
            "the calling body's reading follows further (`T r = f(...); if (r == C)`, a statement "
            "between the store and the test, a switch, a later re-test) is refused "
            "callback-handoff-direct even where the walk admits the body "
            "(tests/handoff_direct_call.rs: statement_between, initialised, split_edge). Size: 0 "
            "corpus rows (the four direct calls of rd_kafka_poll_cb use the next-statement form "
            "or are refused by their bodies' disagreement). DISCHARGE: a split carried from the "
            "call to the body's test of the stored result along the walk's own classes (the "
            "result variable unwritten in between), with Lean parity, measured."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "HANDOFF-INVOKER-CONVERTED-RESULT",
        "title": "Rule 12m's invoker check reads a callback result stored through a conversion as the result itself",
        "why": (
            f"Registered by {CARD} ({RUN}). handoff_branch's result_variable follows `r = cb(...)` "
            "and `T r = cb(...)` through Clang's implicit casts, so a store into a narrower or "
            "differently signed variable is compared as if it held the result: the classes the "
            "walk splits could then disagree with the classes the guard names. The card's direct "
            "reading refuses such a store (Site::result_converted, callback-handoff-direct); the "
            "candidate route's check_path does not yet consult it. Size: 0 corpus rows (every "
            "invoker on the read paths stores rd_kafka_op_res_t into rd_kafka_op_res_t, "
            "examples/handoff_census.rs). DISCHARGE: check_path refuses a site whose result is "
            "stored through a conversion, with a test and Lean parity, measured (expected "
            "identical)."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main() -> None:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        extra = AMEND_APPEND.get(entry["id"])
        if extra and extra not in entry["why"]:
            entry["why"] += extra
    present = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] not in present:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups {before} -> {len(entries)}")


if __name__ == "__main__":
    main()
