#!/usr/bin/env python3
"""rows.py REPORT OUT.json -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (adapted from OP-APP-RK-CALLBACKS-VIA-POLL-CB): the census rows
on one route report (a run directory or its resource-diagnostics.json): each
function below with its unit, status and leaf descriptions. Writes OUT.json
and prints the first four leaves of each."""
import json
import pathlib
import sys

FUNCTIONS = [
    # the invocation sites and their serve callers
    "rd_kafka_poll_cb", "rd_kafka_consume_cb", "rd_kafka_background_queue_serve",
    "rd_kafka_poll", "rd_kafka_flush", "rd_kafka_queue_poll_callback",
    "rd_kafka_consumer_close", "rd_kafka_q_wait_result",
    # lifecycle and setters
    "rd_kafka_conf_new", "rd_kafka_new", "rd_kafka_destroy", "rd_kafka_destroy_flags",
    "rd_kafka_conf_set_dr_msg_cb", "rd_kafka_conf_set_log_cb",
    "rd_kafka_conf_set_oauthbearer_token_refresh_cb",
    # Fluent Bit's registrars and candidates, and the default candidate
    "flb_out_kafka_create", "flb_kafka_conf_create", "flb_aws_msk_iam_register_oauth_cb",
    "cb_kafka_msg", "cb_kafka_logger", "oauthbearer_token_refresh_cb", "rd_kafka_log_print",
]


def main():
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    out = []
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["function"] in FUNCTIONS:
                leaves = [d.get("description", "") for d in function.get("diagnostics") or []]
                out.append({"unit": unit["file"], "function": function["function"],
                            "status": function["status"], "leaves": leaves})
    out.sort(key=lambda row: (FUNCTIONS.index(row["function"]), row["unit"]))
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    for row in out:
        print("%s %s %s %d" % (row["unit"].split("/")[-1], row["function"], row["status"],
                               len(row["leaves"])))
        for leaf in row["leaves"][:4]:
            print("    " + leaf[:160])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
