#!/usr/bin/env python3
"""Card MEMBER-EFFECT-STORES body half (round 35): re-census the 21 rows K3 left
to this owner on the user's 2026-11-24 `current` report.

usage: recensus.py REPORT_JSON OUT_JSON

Read-only. For each named function row, every diagnostic of the row is kept
(category, description, line), so the hand-off store and any other blocker can
be read beside the body.
"""
import json
import pathlib
import sys

ROWS = [
    ("librdkafka", "rdbuf.c", "rd_buf_init"),
    ("librdkafka", "rdkafka_assignment.c", "rd_kafka_assignment_init"),
    ("librdkafka", "rdkafka_fetcher.c", "rd_kafkap_Fetch_reply_tags_set_topic"),
    ("librdkafka", "rdkafka_fetcher.c", "rd_kafkap_Fetch_reply_tags_set_topic_cnt"),
    ("librdkafka", "rdkafka_partition.c", "rd_kafka_topic_partition_set_metadata_from_rktp_stored"),
    ("librdkafka", "rdkafka_sasl_scram.c", "rd_kafka_sasl_scram_build_client_final_message_wo_proof"),
    ("librdkafka", "rdkafka_sasl_scram.c", "rd_kafka_sasl_scram_generate_nonce"),
    ("librdkafka", "snappy.c", "rd_kafka_snappy_init_env"),
    ("ctraces", "ctr_encode_opentelemetry.c", "otel_span_set_attributes"),
    ("ctraces", "ctr_encode_opentelemetry.c", "otel_span_set_events"),
    ("librdkafka", "rdlist.c", "rd_list_grow"),
    ("librdkafka", "rdkafka_partition.c", "rd_kafka_topic_partition_list_grow"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_counter_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_exp_histogram_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_gauge_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_histogram_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_summary_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "create_untyped_instance"),
    ("cmetrics", "cmt_decode_msgpack.c", "unpack_opts_unit"),
    ("monkey", "mk_iov.c", "mk_buffer_cat"),
    ("monkey", "mk_vhost.c", "mk_vhost_set_property"),
]


def main():
    report = json.load(open(sys.argv[1]))
    out = []
    for library, base, name in ROWS:
        hits = []
        for unit in report["units"]:
            if unit.get("library") != library or not unit["file"].endswith("/" + base):
                continue
            for function in unit.get("functions", []):
                if function["function"] == name:
                    hits.append({
                        "file": unit["file"],
                        "status": function["status"],
                        "diagnostics": [
                            {"category": d["category"], "description": d["description"], "line": d.get("line")}
                            for d in function["diagnostics"]
                        ],
                    })
        out.append({"library": library, "file": base, "function": name, "rows": hits})
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
