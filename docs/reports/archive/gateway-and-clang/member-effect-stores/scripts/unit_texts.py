#!/usr/bin/env python3
"""Card MEMBER-EFFECT-STORES (rule 38k): write the analysed `-E -P` text of
each unit holding one of the 21 rows, from the user's run's own compile
database, and a manifest line per unit for the probe (`unit_probe.rs.txt`).

usage: unit_texts.py COMPILE_COMMANDS OUT_DIR

The compile record whose file ends with the unit's path is rerun with `-c`,
`-o X` and the dependency-file flags dropped and `-E -P` inserted, so nothing
is written beside the build (K3's `leaf_lines.py` reading). Read-only on the
user's tree.
"""
import json
import pathlib
import shlex
import subprocess
import sys

SKIP_WITH_VALUE = {"-o", "-MF", "-MT", "-MQ"}
SKIP = {"-c", "-MD", "-MMD", "-MP"}

UNITS = {
    "lib/cmetrics/src/cmt_decode_msgpack.c": [
        "create_counter_instance", "create_gauge_instance", "create_untyped_instance",
        "create_summary_instance", "create_histogram_instance", "create_exp_histogram_instance",
        "unpack_opts_unit",
    ],
    "lib/ctraces/src/ctr_encode_opentelemetry.c": ["otel_span_set_attributes", "otel_span_set_events"],
    "lib/librdkafka-2.15.0/src/rdbuf.c": ["rd_buf_init"],
    "lib/librdkafka-2.15.0/src/rdlist.c": ["rd_list_grow"],
    "lib/librdkafka-2.15.0/src/snappy.c": ["rd_kafka_snappy_init_env"],
    "lib/librdkafka-2.15.0/src/rdkafka_assignment.c": ["rd_kafka_assignment_init"],
    "lib/librdkafka-2.15.0/src/rdkafka_fetcher.c": [
        "rd_kafkap_Fetch_reply_tags_set_topic_cnt", "rd_kafkap_Fetch_reply_tags_set_topic",
    ],
    "lib/librdkafka-2.15.0/src/rdkafka_sasl_scram.c": [
        "rd_kafka_sasl_scram_build_client_final_message_wo_proof",
        "rd_kafka_sasl_scram_generate_nonce",
    ],
    "lib/monkey/mk_server/mk_utils.c": ["mk_buffer_cat"],
    "lib/monkey/mk_server/mk_lib.c": ["mk_vhost_set_property"],
}


def preprocess(database, suffix, out):
    for entry in database:
        if not entry["file"].endswith(suffix):
            continue
        words = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
        argv, skip = [], False
        for word in words:
            if skip:
                skip = False
                continue
            if word in SKIP_WITH_VALUE:
                skip = True
                continue
            if word in SKIP:
                continue
            argv.append(word)
        argv[1:1] = ["-E", "-P"]
        result = subprocess.run(argv, cwd=entry["directory"], capture_output=True, text=True)
        out.write_text(result.stdout)
        return result.returncode
    return None


def main():
    database = json.load(open(sys.argv[1]))
    out_dir = pathlib.Path(sys.argv[2]).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for unit, functions in UNITS.items():
        path = out_dir / (unit.replace("/", "__") + ".i")
        code = preprocess(database, unit, path)
        lines = len(path.read_text().splitlines()) if code == 0 else 0
        print(f"{unit} exit={code} lines={lines}")
        if code == 0:
            manifest.append(f"{path} {','.join(functions)}")
    (out_dir / "manifest.txt").write_text("\n".join(manifest) + "\n")


if __name__ == "__main__":
    main()
