#!/usr/bin/env python3
"""update_ledger.py -- HANDOFF-CANDIDATE-OWN-CALLBACKS: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `HANDOFF-CANDIDATE-OWN-CALLBACKS` (its DISCHARGE holds: a candidate
  whose declaration carries member invocations is decided at the invocation
  as the composition of its own clauses, or recorded as refused with the
  reason named; a member copied exactly into a variable before its call is
  followed by the caller branch check; measured on the serve callers).
* Amended: `MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED` (narrowed: decided
  synchronous, table and member invocations now record their candidates),
  `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` (now read: rd_kafka_poll_cb's own
  invocations), `OP-CALLBACK-CANDIDATES-APP-CALLBACKS` (now the named
  blocker of three rows).
* Registered: the two hand-off readings the composition's next blockers
  need."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "HANDOFF-CANDIDATE-OWN-CALLBACKS"
SOURCE = "docs/reports/handoff-candidate-own-callbacks/README.md"
RUN = ("corpus 66910c10 on base d708641bc: 71387bd3 936/9/3,889 -> 2a1e37c2 938/9/3,887, "
       "Unknown -> Safe 2 (rd_kafka_event_destroy, ut_op_destroy_free), 88 rows move (the "
       "131 rd_kafka_op_destroy leaves of 83 rows decide), binary a2 049bf574 against before "
       "1517b4d5; system 93/4/351 and slice 52/1/347/4 Pending no row moves; null tallies "
       "identical on all three")

RETIRE = ["HANDOFF-CANDIDATE-OWN-CALLBACKS"]

AMEND_APPEND = {
    "MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED": (
        f" Amended by {CARD} ({RUN}): a decided synchronous, table or member invocation "
        "(Parser::callback_operands) now records each candidate, and a composed candidate's "
        "inner candidates, as the row's dependencies (contract_consultation::note_relied, read "
        "by row_dependencies without the body-spelling filter), so their premises are published "
        "with the row; the two Safe rows the card adds list 34 declarations each "
        "(rd_kafka_op_destroy, the 27 rko_op_cb candidates, the six token refreshers); 91 rows' "
        "dependencies grow (89 Unknown), declarations consulted 428 -> 431. Remaining: a "
        "registry invocation decided at the transfer (rule 13w's apply) and a member read "
        "through a decided registration (rule 14e's decide_member) do not record their "
        "candidates. DISCHARGE: those two paths record their candidates too, with a test each, "
        "measured."
    ),
    "UNION-OPAQUE-RECORD-WRITES-BY-TYPE": (
        f" Amended by {CARD} ({RUN}): an invoked member now reads these places: "
        "rd_kafka_poll_cb, composed at its invocations (rule 13u's amendment), refuses at its "
        "own invocation of rd_kafka_op_s.rko_u.offset_commit.cb (and would at "
        "rko_u.terminated.cb), whose run sets are open by these type-keyed writes; it is the "
        "named blocker (callback-own-invocation-refused) of rd_kafka_offsets_for_times and "
        "rd_kafka_query_watermark_offsets (2 rows, their rd_kafka_q_serve leaf)."
    ),
    "OP-CALLBACK-CANDIDATES-APP-CALLBACKS": (
        f" Amended by {CARD} ({RUN}): with rd_kafka_poll_cb and rd_kafka_consume_cb composed, "
        "rd_kafka_toppar_op_serve is the first rko_serve candidate with no contract at "
        "rd_kafka_op_req0, rd_kafka_op_req and rd_kafka_q_serve: the named leaf of "
        "rd_kafka_op_req, rd_kafka_op_req2 and rd_kafka_consume_callback0 (3 rows)."
    ),
}

REGISTER = [
    {
        "id": "HANDOFF-THROUGH-A-HANDING-INVOKER",
        "title": "A hand-off candidate invoked by a composed candidate that hands the same object on is refused at that invoker",
        "why": (
            f"Registered by {CARD} ({RUN}). Composing rd_kafka_poll_cb decides each of its own "
            "invocations with rd_kafka_poll_cb as rule 12m's invoker. Its last, rko_op_cb "
            "(_, _, rko), has three == KEEP hand-off candidates "
            "(rd_kafka_txn_op_send_offsets_to_transaction, rd_kafka_admin_worker, "
            "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb); the caller branch "
            "check from rd_kafka_poll_cb refuses them (examples/handoff_census.rs: \"the body of "
            "rd_kafka_consume_cb is not read\", a direct callee handing rko that records no site), "
            "and, read, it would refuse at the invoker, which returns KEEP to its caller where "
            "the candidate took its parameter, although rd_kafka_poll_cb's own != PASS hand-off "
            "gives the op away exactly there. Size: rd_kafka_poll_cb's composition once its two "
            "union members close (UNION-OPAQUE-RECORD-WRITES-BY-TYPE); 0 rows today. DISCHARGE: a "
            "function with no recorded site read as never naming the object (or its body read), "
            "and the check at an invoker whose own declared hand-off of the same formal admits "
            "the returned class, with Lean parity, measured."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "HANDOFF-MEMBER-OBJECT-BELOW-INVOKER",
        "title": "Rule 12m's path search does not follow an rko_serve invocation whose op is created below the invoker",
        "why": (
            f"Registered by {CARD} ({RUN}). With the exact copy followed "
            "(rd_kafka_op_handle's callback = rko->rko_serve), rd_kafka_poll_cb's != PASS hand-off "
            "is admitted at rd_kafka_q_serve0's rko_serve invocation and refused at the pop "
            "serves (they return the op), but at rd_kafka_q_serve and "
            "rd_kafka_q_serve_maybe_consume the member's invocation is not read (the op is a local "
            "of rd_kafka_q_serve0, below the invoker, and a member invocation of a supplied object "
            "follows only handed variables) and at rd_kafka_op_req0 the body records no site. "
            "Size: no row today (each such call names rd_kafka_toppar_op_serve first, "
            "OP-CALLBACK-CANDIDATES-APP-CALLBACKS). DISCHARGE: the search follows a supplied "
            "object into the direct callee that creates it (a path continuing through calls that "
            "hand no variable, ending where the object is a local), and a function with no site "
            "read as one, with Lean parity, measured on the serve and request declarations."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        extra = AMEND_APPEND.get(entry["id"])
        if extra and extra not in entry["why"]:
            entry["why"] += extra
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] in known:
            for existing in entries:
                if existing["id"] == entry["id"]:
                    existing.update(entry)
        else:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
