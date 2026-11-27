#!/usr/bin/env python3
"""update_ledger.py -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Amended, not retired: `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`. Its DISCHARGE
  does not hold in full: `queue_destructor.qv`'s `rd_kafka_q_destroy_final`
  reaches the log callback but is a model contract outside the resource view
  whose vocabulary has no premise clause, and the 86 null views cannot carry
  one (both decisions for the user). The 34 client.qv declarations whose
  bodies may run an rk_conf callback now name the premise.
* Registered: `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED` (memory.qv's
  `rd_kafka_crash` and kafka_mem.qv's `rd_kafka_dump0`, outside the card's
  named modules)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED"
SOURCE = "docs/reports/librdkafka-plugin-premise/README.md"
RUN = ("corpus 66910c10 on main d708641bc: 71387bd3 936/9/3,889 -> {after} 936/9/3,889, every row's "
       "status, leaves and contract_dependencies field identical, the sidecar's records of the 34 "
       "gaining the premise; binary a1 {binary} against before 8c8e7e39; system 93/4/351 and slice "
       "52/1/347/4 Pending identical; null rows identical on all three")

AMEND_APPEND = {
    CARD: (
        " Amended by {card} ({run}): a census over the preprocessed librdkafka units at both "
        "revisions (identical) found 34 of the 73 client.qv declarations able to run an rk_conf "
        "callback or interceptor method (32 reach rd_kafka_log_buf's conf->log_cb without "
        "rd_kafka_crash, rd_kafka_topic_destroy0 reaches on_conf_destroy, rd_kafka_toppar_get_avail "
        "only through rd_kafka_crash's dump, premised as the conservative label); each now names "
        "rk_no_plugin_library, re-pinned at both revisions. The other 39 client.qv declarations, "
        "refcount.qv (3), typed_queue.qv (2) and topic_partition.qv (1) reach no rk_conf or "
        "interceptor call (eleven indirect calls of ten declarations resolved by reading). 16 Safe and 317 Unknown corpus "
        "rows now list the premise; no Safe or Pending row resting on the 34 lacks it. Residual "
        "(DISCHARGE narrowed to it): queue_destructor.qv's rd_kafka_q_destroy_final reaches the log "
        "callback (rd_kafka_q_purge0 -> rd_kafka_op_destroy -> ... -> rd_kafka_log_buf) but is a "
        "Quirin model contract outside the provider's resource_view, consulted by no verdict, with "
        "no premise clause in its vocabulary; and the 86 null_deref.qv views (75 bodies reach an "
        "rk_conf or interceptor call) are bounds-contract-v1 views that cannot carry a premise "
        "(bounds_contract.rs rejects any other clause; contract_views.rs check_null_deref_only). "
        "Both need the user's decision; no syntax was invented. New DISCHARGE: the user decides "
        "whether and how a null-deref view (and a model contract outside the resource view) names a "
        "premise, each view whose body may run an rk_conf callback then names it or is ruled "
        "independent of it, measured."
    ),
}

REGISTER = [
    {
        "id": "LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED",
        "title": "rd_kafka_crash and rd_kafka_dump0 may run interceptor or log callbacks through the dump's queue-reference drops but do not list P-RK-NO-PLUGIN-LIBRARY",
        "why": (
            "Registered by {card} ({run}), outside its named modules. The census read the other "
            "librdkafka modules too: memory.qv's rd_kafka_crash (with a non-null rk it calls "
            "rd_kafka_dump0, rdkafka.c:6640-6649) and kafka_mem.qv's rd_kafka_dump0 reach "
            "interceptor methods and rk_conf.log_cb only through rd_kafka_dump0's queue-length reads "
            "(rd_kafka_q_len takes a forwarded queue's reference and drops it with "
            "rd_kafka_q_destroy, final only under a concurrent release), on the census's "
            "context-insensitive graph. The card premised rd_kafka_toppar_get_avail on the same "
            "path as the conservative label but did not change these two in passing: on the BEFORE "
            "corpus rd_kafka_crash is consulted by 67 rows (1 Safe) and rd_kafka_dump0 by 3 (2 "
            "Safe). Size: 2 declarations. DISCHARGE: decide whether an assertion-failure dump counts "
            "as running a callback; then both name the premise (re-pinned, measured) or the census "
            "rules the path out, and rd_kafka_toppar_get_avail follows the same ruling."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    import sys
    after, binary = sys.argv[1], sys.argv[2]
    run = RUN.format(after=after, binary=binary)
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    for entry in entries:
        extra = AMEND_APPEND.get(entry["id"])
        if extra:
            extra = extra.format(card=CARD, run=run)
            if extra not in entry["why"]:
                entry["why"] += extra
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] not in known:
            entry = dict(entry, why=entry["why"].format(card=CARD, run=run))
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


main()
