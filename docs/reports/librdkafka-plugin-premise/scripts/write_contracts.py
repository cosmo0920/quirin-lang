#!/usr/bin/env python3
"""write_contracts.py [--check] -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: state the
premise P-RK-NO-PLUGIN-LIBRARY (user decision U2, 2026-10-14: "List it in
every dependent verdict") on every librdkafka `client.qv` declaration whose
body the census (`reach.py`, `artifacts/census.json`) found able to reach an
rk_conf callback or interceptor call, the log callback that `rd_kafka_log_buf`
calls (`rdkafka.c:280-305`) included.

Each declaration gains, right after its `requires resource_contract_v1;`
selector, the clause OP-APP-RK-CALLBACKS-VIA-POLL-CB wrote on its own
declarations, byte for byte (`requires resource_premise(rk_no_plugin_library,
"...")`, contract specification C04.18), and one comment line above its
`extern fn` naming the census path to the log callback. Nothing else in a
declaration changes. The selection is the census's, cross-checked against the
frozen list below: a mismatch, a declaration not found exactly once, or one
that already names the premise fails the run. `--check` verifies every listed
declaration names the premise exactly once, as written."""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CLIENT = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
CENSUS = HERE.parent / "artifacts/census.json"

PLUGIN = ("requires resource_premise(rk_no_plugin_library, \"P-RK-NO-PLUGIN-LIBRARY (user "
          "decision 2026-10-14, U2): no plugin.library.paths is configured, so no library "
          "outside the run (rd_kafka_plugin_new's dlsym conf_init) registers a callback or an "
          "interceptor into a conf, and every rk_conf callback a call may run, the log callback "
          "rd_kafka_log_buf calls included, is one the run registers or a stated default, each "
          "declared (OP-APP-RK-CALLBACKS-VIA-POLL-CB)\");")

# The 34 declarations (PREDICTION.md, section 2), in module order.
FROZEN = [
    "rd_kafka_err_action", "rd_kafka_oauthbearer_set_token_failure", "rd_kafka_toppars_pause_resume",
    "rd_kafka_toppar_destroy0", "rd_kafka_q_destroy0", "rd_kafka_q_destroy_owner",
    "rd_kafka_topic_destroy0", "rd_kafka_enq_once_trigger", "rd_kafka_topic_partition_list_destroy",
    "rd_kafka_topic_partition_destroy0", "rd_kafka_AlterConsumerGroupOffsets_destroy",
    "rd_kafka_DeleteConsumerGroupOffsets_destroy", "rd_kafka_ListConsumerGroupOffsets_destroy",
    "rd_kafka_topic_partition_result_destroy", "rd_kafka_toppar_new0", "rd_kafka_toppar_desired_add",
    "rd_kafka_toppar_get2", "rd_kafka_toppar_get_avail", "ut_create_mock_toppar", "ut_destroy_mock_queue",
    "rd_kafka_q_wait_result", "rd_kafka_q_purge0", "rd_kafka_toppar_fetch_backoff", "unit_test_telemetry",
    "rd_kafka_q_serve0", "rd_kafka_q_serve", "rd_kafka_q_serve_maybe_consume", "rd_kafka_q_pop_serve0",
    "rd_kafka_q_pop_serve", "rd_kafka_q_pop_serve_maybe_consume", "rd_kafka_op_destroy",
    "rd_kafka_op_req0", "rd_kafka_op_req", "rd_kafka_op_req2",
]


def selection():
    rows = json.loads(CENSUS.read_text())["rows"]
    chosen = [r for r in rows if r["module"] == "client.qv" and r["class"] == "rk-conf"]
    return {r["function"]: r for r in chosen}


def shown_path(row):
    """(sink, path): the log callback's shortest path avoiding rd_kafka_crash,
    else the shortest such path to any rk_conf or interceptor call, else the
    shortest path through rd_kafka_crash."""
    a = row["66910c10"]
    avoiding, sinks = a["rk_conf_sinks_avoiding_rd_kafka_crash"], a["rk_conf_sinks"]
    for table in (avoiding, sinks):
        if "rk-conf:log_cb" in table:
            return "rk-conf:log_cb", table["rk-conf:log_cb"]
        if table:
            return next(iter(table.items()))
    raise SystemExit(f"{row['function']}: no sink")


def note(name, row):
    a = row["66910c10"]
    sink, path = shown_path(row)
    members = sorted(k.split(":", 1)[1] for k in a["rk_conf_sinks"])
    call = "conf->log_cb" if sink == "rk-conf:log_cb" else sink.split(":", 1)[1]
    crash = "" if a["rk_conf_sinks_avoiding_rd_kafka_crash"] else \
        " (only through rd_kafka_crash's dump of a non-null rk)"
    text = (f"// LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: `{name}` may run an rk_conf callback{crash} "
            f"({' -> '.join(path)} -> {call}; reached: {', '.join(members)}), so it names "
            f"`rk_no_plugin_library`.")
    import textwrap
    return textwrap.fill(text, width=79, subsequent_indent="// ", break_on_hyphens=False,
                         break_long_words=False) + "\n"


def span(text, name):
    start = text.find(f"extern fn {name}(")
    if start < 0 or text.find(f"extern fn {name}(", start + 1) >= 0:
        raise SystemExit(f"declaration {name}: not found exactly once")
    return start, text.index("\n};", start) + 3


def main():
    check = "--check" in sys.argv
    chosen = selection()
    text = CLIENT.read_text(encoding="utf-8")
    if not check:
        # The census runs on the tree before the edit: its premise-less rows
        # are exactly the frozen list.
        unstated = [n for n, r in chosen.items() if not r["has_premise"]]
        if sorted(unstated) != sorted(FROZEN):
            raise SystemExit(f"census selection differs from the frozen list: "
                             f"{sorted(set(unstated) ^ set(FROZEN))}")
    for name in FROZEN:
        start, end = span(text, name)
        body = text[start:end]
        present = body.count("resource_premise(rk_no_plugin_library,")
        if check:
            if present != 1 or "    " + PLUGIN + "\n" not in body:
                raise SystemExit(f"{name}: premise not present exactly once as written")
            continue
        if present:
            raise SystemExit(f"{name}: already names rk_no_plugin_library")
        marker = "    requires resource_contract_v1;\n"
        if body.count(marker) != 1:
            raise SystemExit(f"{name}: selector not found exactly once")
        body = body.replace(marker, marker + "    " + PLUGIN + "\n", 1)
        text = text[:start] + note(name, chosen[name]) + body + text[end:]
    if check:
        print(f"{len(FROZEN)} declarations name rk_no_plugin_library")
        return
    CLIENT.write_bytes(text.encode("utf-8"))
    print(f"wrote {CLIENT.relative_to(REPO)}: {len(FROZEN)} declarations")


main()
