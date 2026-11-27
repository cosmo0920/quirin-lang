#!/usr/bin/env python3
"""REFCOUNT-RESOURCE-KIND: record the user's decision of 2026-10-05 (verbatim)
in W48-CALLEE-DECISION-TABLE with class 3 re-sized; amend
W45-CALLEE-CASCADE-RESIDUAL and CG22-CALLBACK-REGISTRY-CONTRACT; register the
residuals. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (REFCOUNT-RESOURCE-KIND"
SOURCE = "docs/reports/refcount-resource-kind/README.md"
CARD = "REFCOUNT-RESOURCE-KIND"

DECISION = (
    "USER DECISION 2026-10-05. **Class 3, reference counts -> a new reference-count resource "
    "kind.** Taking a reference (\"keep\") is +1 owned reference. Dropping one (\"destroy\" or "
    "\"drop\") is -1. Each function must balance the references it takes, or hand them off "
    "(return, store into a tracked owner, a declared transfer), exactly as with other owned "
    "resources. A plain counter that is not a reference (e.g. a statistics counter incremented "
    "with `rd_atomic32_add`) needs its own declaration, and never inherits the reference reading. "
    "**Rejected by the user:** treating keep/drop as custody-neutral. It is not fail-closed: a "
    "missing drop would read Safe. Never fall back to it."
)

RUN = ("corpus 66910c10 52d14db1 872/9/3,953 -> 7a4595d6 884/9/3,941, base 9258289c9, binary "
       "bfd36e2a; system 87/4/357 and slice 45/1/354/4 Pending, no row moves, parity multisets "
       "identical")

AMEND = {
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}): " + DECISION + " Carried out by card " + CARD + " (gateway rules 13b, "
        "13c; contract spec C04.17, C04.6; grammar G05; QCore.ReferenceCount, 25 audited; "
        "ResourceKind::Reference). The hazard is resolved at the wrapper level: no rd_atomic32_* "
        "primitive is declared (rd.h:298,351 make rd_refcnt_t an rd_atomic32_t and rd_refcnt_add0 "
        "rd_atomic32_add(R, 1)); keeps and drops are declared on named wrappers whose bodies name "
        "the object and are checked structurally in every defining unit (rule 13c). Sixteen pinned "
        "declarations in qcore-provider-librdkafka librdkafka/client.qv: keeps rd_kafka_toppar_keep0, "
        "rd_kafka_q_keep, rd_kafka_q_keep_nolock, rd_kafka_topic_keep; drops "
        "rd_kafka_toppar_destroy0, rd_kafka_q_destroy0, rd_kafka_q_destroy_owner, "
        "rd_kafka_topic_destroy0, rd_kafka_enq_once_trigger; and, in existing vocabulary under the "
        "decision's hand-off reading, the queue handle constructor rd_kafka_queue_new0 and six "
        "destructors (rd_kafka_topic_partition_list_destroy, rd_kafka_topic_partition_destroy0 under "
        "the literal premise do_free = 1, the three *ConsumerGroupOffsets_destroy, "
        "rd_kafka_topic_partition_result_destroy). +12 Unknown -> Safe (five destructor _free "
        "wrappers, rd_kafka_topic_partition_destroy and _destroy_free, three "
        "rd_kafka_queue_get_* and rd_kafka_share_queue_get_sasl, rd_kafka_toppar_list_copy), no "
        "Safe -> non-Safe move, no leak or violation added. Class 3 re-sized (functions decided "
        "alone, scripts/tally.py over artifacts/rank-a3-corpus.json): 9 callees / 19 -> 2 / 4: "
        "rd_atomic32_add 2 (rd_kafka_cert_dup, a reference keep spelled through the primitive, and "
        "rd_kafka_idemp_inflight_toppar_add, a statistics counter; REFCOUNT-PLAIN-COUNTER-"
        "DECLARATION) and rd_kafka_txn_clear_partitions_flag 2 (a release of the references a "
        "caller's list holds, not declared: REFCOUNT-CONTAINER-REFERENCE-SET). Not freed although "
        "declared: the four *_destroy_array loops and rd_kafka_assignment_destroy hand an element or "
        "member read to a non-null release (the access gate, W44-CALL-ARGUMENT-CUSTODY-POSITION); "
        "rd_kafka_msgbatch_destroy drops a member-held reference (REFCOUNT-MEMBER-REFERENCE-DROP); "
        "ut_destroy_mock_queue consumes its caller's reference "
        "(REFCOUNT-REFERENCE-RETURNING-DECLARATIONS)."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a3-corpus.json): the 545 reproduce; "
        "sixteen reference and owner declarations freed 12. Remaining: 522 Unknown functions blocked "
        "by uncontracted calls alone, 365 by one callee alone (system 26 / 18, slice 38 / 27 "
        "unchanged). Head: cmt_mpack_unpack_array 5 and ctr_mpack_unpack_array 5 (callbacks), "
        "rd_list_destroy 3, add_item_to_object 3, cJSON_DetachItemViaPointer 3, format_metric 3, "
        "rd_kafka_assignor_add 3, rd_list_add 3, rd_list_clear 3. Declaring "
        "rd_kafka_topic_partition_list_destroy exposes the next blocker in 57 Unknown functions: the "
        "list they release came from an undeclared constructor or copy "
        "(rd_kafka_topic_partition_list_new, _copy), now read `unresolved resource identity` (90 "
        "leaves at the destructor's own lines, scripts/reattribution.py)."
    ),
    "CG22-CALLBACK-REGISTRY-CONTRACT": (
        f" {MARK}, {RUN}): the reference drops of rd_kafka_q_serve*, rd_kafka_q_pop_serve0 and "
        "rd_kafka_op_destroy now have vocabulary (resource_reference_dropped, rule 13b); their "
        "callback parts do not, so none is declared and their sizes are unchanged; the refcount "
        "part of this entry is not discharged by a declaration."
    ),
}

RETIRE = set()

NEW = [
    {
        "id": "REFCOUNT-REFERENCE-RETURNING-DECLARATIONS",
        "title": "Reference-returning calls and reference transfers are spelled but undeclared: 131 drops through a plain name refuse",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (gateway rule 13b, contract spec C04.17; " + RUN
               + "). A returned reference is spelled (resource_return_owned with "
               "resource_return_kind(reference): an acquired family, over-drop a violation "
               "candidate), but no corpus declaration uses it: rd_kafka_q_new0 (`tmpq = "
               "rd_kafka_q_new(rk)` ... `rd_kafka_q_destroy_owner(tmpq)`), rd_kafka_toppar_get, _get2 "
               "and the other lookups (\"refcnt from .._get()\") hand their caller a reference the "
               "route cannot see, so its drop reads `reference-drop-unheld`. A function that "
               "consumes its caller's reference (ut_destroy_mock_queue) needs a declared transfer, "
               "which is not spelled. Size (artifacts/refusals-a3-corpus.json): 131 "
               "reference-drop-unheld leaves through a plain name, 91 functions, 1 alone. "
               "rd_kafka_q_new0 also keeps `rk` and `func` in the queue it returns "
               "(RETAINED-BY-RESULT-HOLDER). DISCHARGE: the lookups and constructors declared with "
               "the reference result after reading each body at both revisions, a transfer clause "
               "if a member needs one, every new leak or over-drop candidate read, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-PLAIN-COUNTER-DECLARATION",
        "title": "No plain-counter declaration for the rd_atomic32_* primitives: statistics counters stay uncontracted",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (" + RUN + "). The decision requires a plain "
               "counter to have its own declaration that never inherits the reference reading; the "
               "card resolved the hazard at the wrapper level and declared no primitive, so every "
               "counter update and every reference operation spelled through the primitive stays "
               "`call to uncontracted function`. A plain-counter declaration needs the operand's "
               "Clang type (an rd_refcnt_t member is a reference count; rd_atomic32_t is not) read "
               "at the call. Size (artifacts/census-a3-corpus.json): rd_atomic32_add 10 leaves / 10 "
               "functions / 2 alone (rd_kafka_cert_dup is a keep, rd_kafka_idemp_inflight_toppar_add "
               "a counter), rd_atomic32_set 5 / 4 / 1 (rd_kafka_bufq_reset), rd_atomic32_sub 6 / 6 / "
               "1 (rd_kafka_broker_persistent_connection_del). DISCHARGE: a counter clause applied "
               "only where Clang states a non-reference operand type, a reference reading where it "
               "states rd_refcnt_t, Lean, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-MEMBER-REFERENCE-DROP",
        "title": "A drop of a reference a member holds, in its owner's destructor, is refused",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (rule 13b). A destructor body that drops the "
               "reference a member of the object it tears down holds "
               "(`rd_kafka_topic_partition_private_destroy`: `rd_kafka_toppar_destroy(parpriv->rktp)`; "
               "`rd_kafka_msgbatch_destroy`: the same, then `rkmb->rktp = NULL`) drops a reference "
               "the function never took, so it is refused; the member-release rule (W45) reads only "
               "a nullable heap release of `B->F` with nothing after it, and the toppar macro copies "
               "the member into `_RKTP` first. Size (artifacts/refusals-a3-corpus.json): 3 "
               "member-operand and 3 macro-copy leaves, 1 function alone "
               "(rd_kafka_msgbatch_destroy); it is why rd_kafka_topic_partition_destroy0 had to be "
               "declared rather than inferred. DISCHARGE: the member-release rule extended to a "
               "reference drop of a member (through the macro's block-scope copy), with its "
               "single-use conditions and Lean, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-KEEP-INTO-PLACE",
        "title": "A keep stored straight into a member, or of an operand the route does not name, is refused",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (rule 13b). `rko->rko_rktp = "
               "rd_kafka_toppar_keep(rktp)` and `rd_list_add(list, rd_kafka_toppar_keep(...))` hand "
               "the new reference to a place or call the route does not track, and a keep of a "
               "binding the route cannot resolve (poisoned by an earlier refusal) refuses. Size "
               "(artifacts/refusals-a3-corpus.json): 7 `reference-undecided` leaves, 7 functions, "
               "none alone; most keeps into members sit in statements the access gate refuses "
               "first. DISCHARGE: a keep into a member of a tracked owner linked as a field link "
               "(rule 13b's hand-off), measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-CONTAINER-REFERENCE-SET",
        "title": "A call that drops every reference a caller's container holds is not stated",
        "why": "Registered by REFCOUNT-RESOURCE-KIND. rd_kafka_txn_clear_partitions_flag "
               "(rdkafka_txnmgr.c:1212-1225) walks a caller's TAILQ of toppars, clears a flag and "
               "drops each element's reference, then re-initialises the list: a release of a set "
               "reachable from a parameter, which C04.13 records v1 does not state; declaring it "
               "borrowed would hide the drops. Size: 2 functions decided alone "
               "(rd_kafka_txn_clear_partitions, _clear_pending_partitions). DISCHARGE: a container "
               "reference-set clause, or a recorded decision that a container's references are "
               "outside the custody projection, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-INFERRED-REFERENCE-RESULT",
        "title": "No derived summary publishes a reference result",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (C04.17, C04.6 rule 7). A body that returns a "
               "kept reference (rd_kafka_toppar_list_copy) is Safe on its own row, but inference "
               "never publishes a non-heap result, so its callers see an uncontracted callee. Size: "
               "0 measured corpus functions (list_copy is a callback). DISCHARGE: the derived "
               "summary admits an owned reference result validated against the body's family "
               "balance, with Lean, when a member needs it.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "RETAINED-BY-RESULT-HOLDER",
        "title": "No retention clause names the call's result as the holder",
        "why": "Registered by REFCOUNT-RESOURCE-KIND (C04.14). rd_kafka_queue_new0 stores `rk` in the "
               "handle it returns (`rkqu->rkqu_rk`) and rd_kafka_q_new0 stores `rk` and `func`; "
               "resource_retained_by names a parameter or `global` as the holder, never the result, "
               "so `rk` is declared borrowed with the gap recorded in the contract (librdkafka "
               "requires every queue to be destroyed before its client, and no audited caller tracks "
               "a client handle). Size: 0 measured functions. DISCHARGE: a holder spelled as the "
               "result, rule 12t's outlives decision for it, Lean, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    by_id = {entry["id"]: entry for entry in followups}
    for key, text in AMEND.items():
        entry = by_id[key]
        if MARK not in entry["why"]:
            entry["why"] += text
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    ids = {entry["id"] for entry in followups}
    for entry in NEW:
        if entry["id"] not in ids:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
