#!/usr/bin/env python3
"""CG22-CALLBACK-REGISTRY-CONTRACT: amend the card's own entry (its DISCHARGE's
"checked against bodies" holds for the invocation and the registration in a
parameter holder only, rule 13y, so the entry is amended, not retired),
W48-CALLEE-DECISION-TABLE (class 1, shape (c)), W45-CALLEE-CASCADE-RESIDUAL and
W44-CALLEE-CROSS-UNIT-SUMMARY; register the residuals with sizes.
RDKAFKA-SORT-COMPARATOR-EFFECTS, named in a module comment, was never
registered: nothing to retire. Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "CG22-CALLBACK-REGISTRY-CONTRACT"
MARK = "AMENDED (" + CARD
SOURCE = "docs/reports/cg22-callback-registry-contract/README.md"

RUN = ("corpus 66910c10 ab8f5f14 881/9/3,944 -> c0320aca 879/9/3,946, base 84ec54d1d, binary "
       "3b204784; system 87/4/357 and slice 45/1/354/4 Pending, no status move")

AMEND = {
    CARD: (
        f" {MARK}, {RUN}): the vocabulary is built (contract spec C04.19, grammar G05, gateway "
        "rules 13u-13z, QCore.CallbackContracts): resource_callback_invoked (synchronous "
        "invocation, loop-safe composition: every candidate's contract the identity, "
        "resource_owned a requirement put on the call), resource_callback_table (closed over "
        "an automatic array's initializer), resource_callback_registered / "
        "_registry_invoked / _unregistered (a registration in force on every path in the "
        "calling body, decided as callback_trace requires), resource_element_added / "
        "resource_elements_dropped (custody into a releasing registry held by a tracked "
        "identity) and resource_released_if_constructed (RD_LIST_F_ALLOCATED). Route G "
        "consumes it; the declarations are pinned at both revisions (bsearch, qsort, "
        "pthread_once; the cmetrics and ctraces array and map unpackers; rd_list_init, _new, "
        "_add, _destroy, _clear; pb_write, pb_encode_varint; rd_kafka_topic_partition_list_sort "
        "and msgpack_packer_init re-declared). The msgpack data capture is re-examined: "
        "msgpack_packer_init retains data in pk and registers callback in pk (rule 13y "
        "validates the registration); every site is in a function Unknown before. Two named "
        "corrections (rd_kafka_topic_partition_list_sort_by_topic, _by_topic_id: Safe only by "
        "reading the comparator as benign). No Unknown -> Safe. Rule 13y checks against bodies "
        "the invocation (cmt_ and ctr_mpack_unpack_array validated) and a registration in a "
        "parameter holder (rd_list_init, msgpack_packer_init validated; "
        "rd_kafka_topic_partition_list_sort withdrawn in rdkafka_partition.c). REMAINING "
        "DISCHARGE (narrowed): the callback clauses no body check reads are declared "
        "assumptions: the two map unpackers' tables, rd_list_new's constructor registration, "
        "rd_list_add's element, rd_list_destroy's and rd_list_clear's registry invocation, "
        "element drop, unregistration and self-release, pb_write's and pb_encode_varint's "
        "registry invocation (8 declarations); a structural check of each against its body "
        "(rdlist.c, cmt_/ctr_mpack_utils.c, nanopb/pb_encode.c), measured. The other residuals "
        "are CG22-REGISTRY-OUTSIDE-THE-FRAME, CG22-CALLBACK-CANDIDATE-CONTRACTS, "
        "CG22-CALLBACK-OPEN-ACTUALS, CG22-OP-CALLBACKS-AND-SERVED-QUEUES and "
        "CG22-MSGPACK-PACK-INVOCATIONS."
    ),
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}, scripts/size_members.py over the a3 corpus; leaves naming the callee "
        "/ functions carrying one / functions it decides alone): class 1 shape (c) has its "
        "vocabulary (C04.19, rules 13u-13z). Declared: bsearch 6/5/1, qsort 4/3/0, "
        "pthread_once 2/2/1, cmt_mpack_unpack_array 12/12/5, _map 10/10/3, "
        "ctr_mpack_unpack_array 5/5/5, _map 9/9/3 (each now `callback-uncontracted`: their "
        "callbacks have no contract in force, CG22-CALLBACK-CANDIDATE-CONTRACTS), rd_list_init "
        "50/33/0, rd_list_new 2/2/0, rd_list_add 48/36/0, rd_list_destroy 131/82/1, "
        "rd_list_clear 0 (refused first by the access gate at its member actual), pb_write "
        "8/8/2, pb_encode_varint 5/4/2 (each `callback-registry-undecided`: the list or stream "
        "is reached through a formal or a member, CG22-REGISTRY-OUTSIDE-THE-FRAME). Not "
        "declared, unchanged: rd_kafka_q_serve0 3/3/2, rd_kafka_q_pop_serve0 3/3/2, "
        "rd_kafka_q_serve_maybe_consume 2/2/2, rd_kafka_op_destroy 132/83/2 "
        "(CG22-OP-CALLBACKS-AND-SERVED-QUEUES); pthread_getspecific 16/15/1, "
        "pthread_setspecific 15/10/1, mk_sched_get_thread_conf 9/9/0 (a result returning what "
        "a holder retains: still no clause). No function moved to Safe."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a3-corpus.json): Unknown functions "
        "blocked by uncontracted calls alone 523 -> 487, by one callee alone 366 -> 335: the 36 "
        "carry named callback refusals now (rules 13u-13x); none freed."
    ),
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        f" {MARK}, {RUN}): callback candidates too: three comparators Safe in their own unit "
        "(rd_kafka_metadata_broker_cmp, rd_kafka_metadata_broker_internal_cmp, rd_strcmp3) "
        "have no contract in force where bsearch is called (rule 13u's `callback-uncontracted`, "
        "4 leaves)."
    ),
}

NEW = [
    {
        "id": "CG22-REGISTRY-OUTSIDE-THE-FRAME",
        "title": "A registry used through a formal or a member has no registration in force in the calling body",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/reasons.py, "
               "artifacts/reasons-a3-corpus.json). Rule 13w decides a registry use only under a "
               "registration made in the calling body on every path; every measured rd_list and "
               "nanopb caller reaches the list or stream through a formal or a member "
               "(rd_list_destroy(&rk->rk_conf.partition_assignors), rd_list_clear("
               "&mcluster->request_list), pb_write(stream, ...)): 239 "
               "`callback-registry-undecided` leaves / 112 functions / 5 decided alone "
               "(rd_list_destroy 131, rd_list_init 49, rd_list_add 44, pb_write 8, "
               "pb_encode_varint 5, rd_list_new 2), plus 114 access-gate refusals at member "
               "actuals of the now-classified calls; 3 `callback-element-custody-undecided` "
               "(rd_list_add of a tracked element into a frame list with a free callback: a frame "
               "object has no identity to own it). nanopb's registration, pb_ostream_from_buffer, "
               "returns the stream by value (not read by the declared-ABI reader). DISCHARGE: a "
               "registration carried by the caller's object (a precondition a summary or a "
               "type-level invariant states and the caller's caller establishes), or a frame "
               "registry identity, decided against callback_trace, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "CG22-CALLBACK-CANDIDATE-CONTRACTS",
        "title": "Synchronous invocations refuse because their callbacks have no contract in force",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/reasons.py and "
               "scripts/candidate_status.py over the a3 corpus). 46 `callback-uncontracted` "
               "leaves / 44 functions / 17 decided alone (the cmetrics and ctraces unpackers 36, "
               "bsearch 6, qsort 3). Of the 44 candidates named: 40 are Unknown in their own unit "
               "(the cmetrics and ctraces decoder trees, every table holding at least one "
               "Unknown handler, and rd_kafka_assignor_topic_cmp, rd_kafka_group_member_cmp, "
               "rd_qsort_r_trampoline, toppar_sort_by_list_cnt), 3 are Safe in another unit "
               "(W44-CALLEE-CROSS-UNIT-SUMMARY), 1 is a header static W4 does not summarise "
               "(rd_kafka_ApiVersion_key_cmp, rdkafka_proto.h:208; rd_kafka_ApiVersion_check "
               "would be Safe with it). DISCHARGE: the candidates' own verdicts (bottom-up, the "
               "decoders' leaves first), the cross-unit route, a summary for header statics, "
               "each measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "CG22-CALLBACK-OPEN-ACTUALS",
        "title": "Callback actuals rule 11p's resolution leaves open: a null default, an unprototyped designator, a formal",
        "why": "Registered by " + CARD + " (" + RUN + "; artifacts/reasons-a3-corpus.json). 10 "
               "`callback-undecided` leaves / 9 functions / 1 decided alone: "
               "rd_kafka_topic_partition_list_sort(list, NULL, NULL) 6 (the body substitutes "
               "rd_kafka_topic_partition_cmp_opaque for a NULL comparator: a clause reading a null "
               "actual as a declared default is missing), pthread_once over a K&R-defined routine "
               "2 (mk_http_thread_initialize_tls_once(), mk_set_up_tls_keys(): Clang's implicit "
               "BitCast from `void (*)()` to `void (*)(void)` is not read by rule 11p's "
               "resolution), a formal 2 (chunkio's qsort_stream compar, rd_list_new's free_cb: "
               "correct refusals). DISCHARGE: a declared default for a null callback, and rule "
               "11p reading a compatible-function-type BitCast, each measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "CG22-OP-CALLBACKS-AND-SERVED-QUEUES",
        "title": "Op callbacks and served queues stay undeclared; the served-queue premise stays",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/size_members.py, "
               "artifacts/bodies.txt, each body read at both revisions). rd_kafka_q_serve0 "
               "(rdkafka_queue.c:558-649) keeps and drops its forward queue in one body (rule "
               "13b decides it) but hands each op it moved to a local queue to the callback "
               "through rd_kafka_op_handle, which must take it over: a callback with custody of "
               "a callee-supplied object, which rule 13u refuses, so every call would refuse. "
               "rd_kafka_op_destroy calls rko_op_cb (a member registered by the op's creator, an "
               "open set under rule 11p) and releases a payload chosen by the op type from a "
               "union: not tractable. Sizes unchanged (leaves / functions / alone): "
               "rd_kafka_op_destroy 132/83/2, rd_kafka_q_serve0 3/3/2, rd_kafka_q_pop_serve0 "
               "3/3/2, rd_kafka_q_serve_maybe_consume 2/2/2, rd_kafka_q_serve 12/10/0, "
               "rd_kafka_q_pop_serve 2/2/1, rd_kafka_poll_cb 4/4/0. The served-queue premise "
               "(C04.18) on rd_kafka_q_wait_result and rd_kafka_q_purge0 stays: the op callbacks "
               "they run are read from op members no registration in the calling body states. "
               "DISCHARGE: a clause for a callback that takes over an object the callee supplies "
               "(custody handed to the callback per invocation), a type-switched release, and a "
               "registration carried by an op, each measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "CG22-MSGPACK-PACK-INVOCATIONS",
        "title": "msgpack's packer registry has no declared invoking side",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/prefix_size.py). "
               "msgpack_packer_init now registers callback in pk and retains data there; the "
               "msgpack_pack_* family (header inline bodies calling pk->callback(pk->data, buf, "
               "len)) is undeclared: system 122 leaves / 6 functions / 0 alone, slice 2 / 1 / 1, "
               "corpus 0. DISCHARGE: resource_callback_registry_invoked declarations for the "
               "family, checked against the inline bodies, with the packer a frame registry "
               "holder, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    missing = [name for name in AMEND if name not in known]
    if missing:
        raise SystemExit("not registered: %s" % missing)
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text and MARK not in entry["why"]:
            entry["why"] += text
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
