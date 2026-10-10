#!/usr/bin/env python3
"""NULL-CANDIDATE-TRIAGE: retire NULL-CANDIDATE-TRIAGE, amend
NULL-CLAUSE-COVERAGE and NULL-CUSTODY-PREMISE-RESIDUALS with this card's
measurements, and register the residuals with sizes. Acceptance matrix,
indent=1, ensure_ascii=False; `dimensions.named_followups` = list length.
Idempotent. Sizes are the a4 runs' (report sections 3-6)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "NULL-CANDIDATE-TRIAGE"
SOURCE = "docs/reports/null-candidate-triage/README.md"

MEASURED = (
    "Measured by NULL-CANDIDATE-TRIAGE (gateway rules 14r-14w; corpus 66910c10, run a4, binary "
    "81c61ca8; custody byte-identical 915 / 9 / 3,910 ded14632, null 1,115 / 1,421 Safe, 2,363 "
    "Unknown, 22 provisional Unsafe with 33 findings; candidates 12,809 = 10,907 on the functions' "
    "own checks + 1,902 on travelled caller checks)"
)

NEW = [
    {
        "id": "NULL-TRIAGE-UNRESOLVED-PATHS",
        "title": "104 of the 139 strong-source NULL candidates have no validated path and no infeasibility proof",
        "why": MEASURED + ": the path search (rule 14r) validated 33 findings (30 null-on-allocation-failure, "
        "3 null-on-normal-path) and proved 2 infeasible; 104 stay candidates, unresolved: a condition over a "
        "value the search does not follow 47 (pointer differences such as rd_kafka_buf_read's `__len0 > "
        "&rkbuf->rkbuf_reader->end - rd_slice_abs_offset(...)`, arithmetic of a formal, a floating value, a "
        "masked value with several bits), a correlated condition 32 (a memory value re-read after a call "
        "whose spelling an earlier decision read, e.g. `map->type == 2`, `evl->n_events > 0`), a loop's "
        "iteration bound 14 (64), a switch over an unfollowed value 4, a case label inside a nested statement "
        "3, an ordered pointer comparison 3, a correlated pointer test 1 (artifacts/triage-corpus.json). One "
        "finding (copy_label_values) rests on an uncontracted callee's integer result the callee never "
        "returns (cfl_list_size() < 0): the search reads a call's result as unconstrained. DISCHARGE: each "
        "class read by a sound extension (relational facts between values, a call's documented result range "
        "as a checked contract, memory values a call provably does not write), each re-measured; no finding "
        "made by widening what a free value may be.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-MEMBER-INVARIANT-SCOPE",
        "title": "The largest memory-field candidate sources hold only on linked or received objects; no declared member covers them",
        "why": MEASURED + ": rule 14t's member declarations are checked at every analysed store and withdrawn "
        "by one not proved non-null, with zero-initialised construction a premise. The five declared "
        "(rd_kafka_toppar_s.rktp_rkt, rd_kafka_topic_s.rkt_rk and .rkt_topic, rd_kafka_cgrp_s.rkcg_rk, "
        "rd_kafka_transport_s.rktrans_rkb) stand partly-validated and discharge 1,002 memory-field candidates "
        "(9,607 -> 8,605). Not declarable on measured grounds: rd_kafka_buf_s.rkbuf_rkb (3,474 candidates on "
        "the base; rdkafka_broker.c resets it to NULL after a response), rd_kafka_broker_s.rkb_rk (682; the "
        "store check withdrew it in run a3: rd_kafka_broker_find_by_nodeid0_fl's lookup key "
        "`{.rkb_nodeid = ...}` and three unit tests), struct mk_list.next (595; mk_list_del stores NULL). "
        "Circular list heads, preallocated elements and counters correlated with a pointer have no sound "
        "checked reading. DISCHARGE: a vocabulary for an invariant scoped to the objects it holds on (a "
        "received buffer, a linked list entry, a registered broker), checked against the stores and the "
        "scope's entry points, each measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-TRAVEL-CALLER-REINFERENCE",
        "title": "212 corpus rows left Safe for Unknown when cross-unit travel checked their calls; 650 of the travelled candidates are the caller's own untested formal",
        "why": MEASURED + ": rule 14v adds, after every unit, a call-precondition check at each actual passed "
        "to an external function another unit defines and dereferences on every path (6,125 checks: 3,340 "
        "proved, 883 moved, 1,902 candidates in 1,364 rows). Every Safe -> Unknown move (212 corpus rows, 1 "
        "slice row) is a named correction: the call was unchecked before. Travel never re-analyses the "
        "caller, so a caller whose own formal reaches such a call on every path does not infer its own "
        "clause: 650 travelled candidates in 364 rows have only the caller's formal as their source. "
        "DISCHARGE: a second null pass with the exported clauses in the callers' environments (or an "
        "inference that counts an external actual as a dereference once travel confirms it), measured "
        "against these 212 rows; never a clause inferred from an unconfirmed callee.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-TRAVELLED-CHECK-TRIAGE",
        "title": "40 strong-source candidates on travelled caller checks are not path-searched",
        "why": MEASURED + ": rule 14r searches the candidates the walk records; the checks cross-unit travel "
        "adds after every unit (rule 14v) carry 38 null-literal and 2 allocation candidates that no search "
        "validated or refuted. DISCHARGE: the search run for a travelled check (the caller's body is not "
        "kept after its unit), or the search's evidence recorded at the call for the fold, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-RESULT-BODY-UNPROVED",
        "title": "rd_kafka_topic_keep's declared non-null result is a declared assumption: the walk proves none of its 64 bodies' returns",
        "why": MEASURED + ": rule 14u checks a declared result against every analysed body; "
        "rd_kafka_topic_keep (rdkafka_topic.h, static inline) is examined in 64 units, 0 validated, 0 "
        "refuted: its return of `rkt` follows `rd_kafka_rkt_get_lw(rkt)` and `rd_refcnt_add(&rkt->rkt_refcnt)`, "
        "whose reads of rkt the walk does not carry to the return. The declaration makes the store "
        "`rktp->rktp_rkt = rd_kafka_topic_keep(rkt)` proved. DISCHARGE: the walk reads the dereference inside "
        "the called inline helpers (or the atomic refcount) as a dominating dereference of rkt, and the "
        "standing re-measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-SYSTEM-CONFIGURATION-AMBIGUITY",
        "title": "On the system route every definition is analysed in two configurations, so 129 names export nothing for cross-unit travel",
        "why": MEASURED + ": rule 14v exports a definition only when exactly one analysed unit defines it; the "
        "system route analyses its files in two configurations, so 129 names count as defined twice (3 "
        "exported, 2 caller checks added). DISCHARGE: identify a definition across configurations by its "
        "source identity (same file and body digest), and travel only when every configuration's summary "
        "agrees, measured on the system route.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]

AMEND = {
    "NULL-CLAUSE-COVERAGE": (
        " Amended by NULL-CANDIDATE-TRIAGE: inferred clauses now travel to callers in other units through "
        "exported summaries (rule 14v: an external definition analysed in exactly one unit; corpus 2,590 "
        "exported, 3 ambiguous, 6,125 caller checks added) and seven more declarations are pinned at both "
        "revisions (rd_kafka_flush, rd_kafka_message_errstr, rd_kafka_queue_poll, rd_kafka_event_error, "
        "rd_kafka_outq_len, rd_kafka_op_destroy, rd_kafka_topic_name; every defining body the corpus "
        "analyses checks necessary, 16 rows). Left: a bounds v1 admission of scalar floating and slot parameters (the cmetrics "
        "setters), and a check of the function pointer at an indirect call (re-measured: 235 calls in 185 "
        "corpus functions)."
    ),
    "NULL-CUSTODY-PREMISE-RESIDUALS": (
        " Re-measured by NULL-CANDIDATE-TRIAGE (corpus a4): the custody report is byte-identical; the null "
        "property proves 782 premise checks (676 before; candidates 1,000 -> 892, 2 now findings); custody "
        "Safe rows with an own premise 12, every premise proved 1 (unchanged): none of the 31 premise-carrying "
        "rows is discharged by the new member declarations (their premises read s->iov, i->storage and list "
        "elements)."
    ),
}

RETIRE = "NULL-CANDIDATE-TRIAGE"


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [e for e in entries if e["id"] != RETIRE]
    for entry in entries:
        if entry["id"] in AMEND and AMEND[entry["id"]] not in entry["why"]:
            entry["why"] += AMEND[entry["id"]]
    known = {e["id"] for e in entries}
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


main()
