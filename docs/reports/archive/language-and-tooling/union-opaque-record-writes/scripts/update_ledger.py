#!/usr/bin/env python3
"""update_ledger.py RUN -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE (adapted from
HANDOFF-DIRECT-CALL-READING): the acceptance matrix's named follow-ups
(indent=1, ensure_ascii=False; `dimensions.named_followups` = list length).
Idempotent. RUN is the measurement summary quoted in every amended or
registered entry.

* Retired: `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` (its DISCHARGE holds: an
  opaque write keyed by its target place, the type-keyed reading kept for a
  write through an untargeted pointer, with Lean parity and a measured
  re-pin).
* Amended: `RKO-U-ARMS-BY-DISCRIMINANT` and
  `OP-CALLBACK-CANDIDATES-APP-CALLBACKS` (the union's bytes 8..16 now close
  over four candidates shared by three arms).
* Registered: `RKO-U-SHARED-BYTES-CANDIDATES`, the four uncontracted
  candidates the two declared members now close over."""
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "UNION-OPAQUE-RECORD-WRITES-BY-TYPE"
SOURCE = "docs/reports/union-opaque-record-writes/README.md"
RUN = sys.argv[1]

RETIRE = ["UNION-OPAQUE-RECORD-WRITES-BY-TYPE"]

AMEND_APPEND = {
    "RKO-U-ARMS-BY-DISCRIMINANT": (
        f" Amended by {CARD} ({RUN}): the declared members rd_kafka_op_s.rko_u.offset_commit.cb "
        "and rko_u.terminated.cb now keep their arm path and close by layout over the stores "
        "on union bytes 8..16, which three arms share (offset_commit.cb, terminated.cb, "
        "broker_monitor.cb): four candidates, rd_kafka_offset_broker_commit_cb, "
        "rd_kafka_decommissioned_broker_thread_join, rd_kafka_coord_rkb_monitor_cb and "
        "rd_kafka_txn_coord_monitor_cb. An arm map by discriminant would narrow each member to "
        "its own arm's stores (offset_commit.cb: rd_kafka_offset_broker_commit_cb and the "
        "application's empty offset_commit_cb set; terminated.cb: "
        "rd_kafka_decommissioned_broker_thread_join)."
    ),
    "OP-CALLBACK-CANDIDATES-APP-CALLBACKS": (
        f" Amended by {CARD} ({RUN}): rko->rko_u.offset_commit.cb, which "
        "rd_kafka_toppar_op_serve and rd_kafka_poll_cb call, is no longer read as the whole "
        "union: its run set closes over the four candidates on union bytes 8..16 "
        "(RKO-U-SHARED-BYTES-CANDIDATES), none contracted."
    ),
}

REGISTER = [
    {
        "id": "RKO-U-SHARED-BYTES-CANDIDATES",
        "title": "rd_kafka_op_s.rko_u.offset_commit.cb and rko_u.terminated.cb close over four uncontracted candidates that three arms share at union bytes 8..16",
        "why": (
            f"Registered by {CARD} ({RUN}). With opaque writes keyed by their target and the "
            "declared spelling keeping its arm path, the two members rd_kafka_poll_cb invokes "
            "close by Clang's layout over the stores on union bytes 8..16: "
            "rd_kafka_offset_broker_commit_cb (rd_kafka_commit0's formal, rdkafka_offset.c), "
            "rd_kafka_decommissioned_broker_thread_join (rdkafka_broker.c, terminated.cb), and "
            "rd_kafka_coord_rkb_monitor_cb and rd_kafka_txn_coord_monitor_cb "
            "(rd_kafka_broker_monitor_s.rkbmon_cb through broker_monitor.cb); "
            "rd_kafka_conf_s.offset_commit_cb closes empty. None has a contract, so the four "
            "rows that named the union now refuse callback-uncontracted at "
            "rd_kafka_coord_rkb_monitor_cb: rd_kafka_consume0 and rd_kafka_consumer_close0 "
            "(callback-uncontracted at the direct call of rd_kafka_poll_cb), "
            "rd_kafka_offsets_for_times and rd_kafka_query_watermark_offsets "
            "(callback-own-invocation-refused at rd_kafka_q_serve). Each row carries 13 to 24 "
            "other leaves (blockers-corpus.json). DISCHARGE: the four candidates declared "
            "against the invocations' actuals (each body read at 66910c10 and 046accf6, "
            "validated or a declared assumption, pinned), or the arms separated by "
            "discriminant (RKO-U-ARMS-BY-DISCRIMINANT), measured."
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
