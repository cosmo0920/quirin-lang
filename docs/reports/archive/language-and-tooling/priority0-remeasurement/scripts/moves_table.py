#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: the attribution table of every moved row of
the priority0 chain (read-only).

    python3 moves_table.py CHAIN.json LANDING_RECORDS.json OUT.json OUT.md

A row's cause is, in order: the chain tool's own attribution (a `--declared`
prediction of a card landed in that step, or a landed card's pinned record
that measured and predicted the move); else the entry of CAUSES below, each
checked by this card against the record it names. Nothing is accepted
without one of the two; a row with neither is HELD.
"""

import json
import sys
from pathlib import Path

# function name -> (card, class, evidence). Class: `card-measured` (the card's
# own measurement shows the move; its prediction block missed it),
# `card-predicted-prose` (predicted in the card's report, not its block),
# `verified` (a cause checked after integration from the records, the rule
# text and the row's evidence), `pinned-tree` (the card's mechanism, on the
# pinned tree's body, which differs from the tree the card measured).
CAUSES = {
    # --- step 1 (prev -> common) -----------------------------------------
    **{fn: ("AG2-UNRESOLVED-ALIAS", "card-measured",
            "docs/reports/ag2-unresolved-alias/integration/native-comparison.json.gz (pinned, unpredicted); "
            "each checked against base diagnostics and source in its README, rule 62")
       for fn in ("input_chunk_has_magic_bytes", "LZ4F_optimalBSID", "rd_kafka_message_leader_epoch",
                  "rd_kafka_topic_partition_cmp_opaque", "sb_is_rejected_stream", "flb_task_retry_count")},
    **{fn: ("RULE50-CONSUMER-COMPOSITION", "verified",
            "rule 50 composes mk_http_thread_destroy's helper mk_thread_destroy at its consumers over the run's "
            "set of mk_thread.cb_destroy; on these pinned inputs 34 units are unread (lj_vm.S absent from "
            "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production), so the set is not closed "
            "(callback-member-open) and the consumers are Unknown; the card measured a 4,704-row corpus whose run "
            "read every unit")
       for fn in ("sched_thread_cleanup", "mk_sched_threads_destroy_conn")},
    # --- step 2 (common -> tip) ------------------------------------------
    **{fn: ("CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT", "card-measured",
            "landing r50 gate note: ten cmetrics/ctraces unpack_* rows Unknown -> Safe on its opt-in run, resting "
            "on five declarations not validated (CG10-OOM-DECLARATIONS-UNVALIDATED); its block named other rows; "
            "rule 72")
       for fn in ("unpack_opts_desc", "unpack_opts_name", "unpack_opts_ns", "unpack_opts_ss",
                  "unpack_static_label_component", "unpack_instrumentation_scope_name",
                  "unpack_instrumentation_scope_version", "unpack_link_trace_state", "unpack_span_schema_url",
                  "unpack_span_status_message")},
    **{fn: ("CG10-MEMBER-STORE-RETAINED-BY", "card-measured",
            "landing r43 gate note and README: ten Unknown -> Safe on its opt-in run (block named four); rule 70")
       for fn in ("otel_span_set_name", "otel_span_set_trace_state", "pb_field_iter_begin", "rd_kafka_confval_disable",
                  "rd_kafka_confval_init_int", "init_snappy_decompressor", "mk_ptr_set")},
    "ctr_link_set_attributes": (
        "CG10-MEMBER-STORE-RETAINED-BY", "pinned-tree",
        "rule 70 (`link->attr = attr`, a formal stored into a formal's member) on the pinned 66910c10 body; its "
        "only common leaf was rule 38i's member-store-through-formal; 046accf6's body releases first and stays "
        "Unknown (card evidence/changed-rows-base-tip2.txt)"),
    **{fn: ("CG10-MEMBER-STORE-RETAINED-BY", "verified",
            "rule 70 (`ins->context = context` / `in->context = context`, an object-pointer formal stored into a "
            "formal's member); the single-statement body's only common leaf was the access refusal of that store; "
            "the card predicted no system or slice move without measuring those routes")
       for fn in ("flb_output_set_context", "flb_input_set_context")},
    "ctr_link_set_trace_state": (
        "MEMBER-EFFECT-STORES", "card-measured",
        "round-37 landing note: rule 38k on the pinned tree's body (store of cfl_sds_create's result); "
        "docs/reports/member-effect-stores/integration/native-comparison.json.gz"),
    **{fn: ("CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4", "card-predicted-prose",
            "README (integrator: expect XXH32_digest, XXH64_digest, LZ4_prepareTable, LZ4HC_clearTables) and r48 "
            "gate note (seven opt-in moves on neutral declarations, CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED); rule 66")
       for fn in ("LZ4_prepareTable", "LZ4HC_clearTables", "XXH32_digest", "XXH64_digest")},
    **{fn: ("CG10-READER-GAPS", "card-measured",
            "r51 gate note: both Unknown -> Safe on its opt-in run (rule 71, declared assumptions)")
       for fn in ("rd_kafka_assignment_pause", "rd_kafka_assignment_resume")},
    **{fn: ("K1-CALL-ACTUAL-ACCESS", "card-measured",
            "R34-K-REPAIR's corrected prediction (its report section 4) on the round-34 record "
            "docs/reports/r34-k-repair/integration/native-comparison.json.gz")
       for fn in ("rd_kafka_broker_destroy_error", "rd_kafka_toppar_is_on_cgrp")},
    "mk_string_html_escape": (
        "K5-LOCAL-ORIGIN-ALIASES", "card-measured",
        "R34-K-REPAIR's corrected prediction on the round-34 record"),
    "rd_kafka_sw_str_sanitize_inplace": (
        "LATER-DECLARATOR-INITIALISER-UNREAD", "card-measured",
        "r67 gate note: the card's one opt-in move, missed by its block and reported before the landing decision; "
        "rule 74k"),
    "rd_kafka_msgq_allow_wakeup_at": (
        "K6-DIRECT-POSITION-ACCESS", "card-measured",
        "round-37 landing note: a same-unit cascade of rule 81 through rd_kafka_msgq_first's new summary; "
        "docs/reports/k6-direct-position-access/integration/native-comparison.json.gz"),
    "get_member_id": (
        "AMPERSAND-RUN-PARITY", "card-measured",
        "README and evidence/moved-rows-base-tip.jsonl: its only leaf was rule 5j reading the second `&` of `&&` "
        "as an address"),
    "rd_kafka_sasl_safe_string": (
        "LATER-DECLARATOR-INITIALISER-UNREAD", "card-predicted-prose",
        "README fixture table: the scram shape `char *safe = 0, *d = 0;` read one per statement is Unsafe on a "
        "provisional loop re-acquisition leak, not a C leak; follow-up PASS-CORRELATED-LOOP-ALLOCATION; "
        "bracketed by SP9's tip 2aaf3a585 and round 34's base 4be28c3cd (corpus Unsafe 12 -> 13)"),
    "ut_testSameSubscriptions": (
        "RK-TOPIC-PARTITION-LIST-CONSTRUCTORS", "card-measured",
        "README and r57 gate note: provisional Unsafe on a unit test's assertion-failure returns; bracketed by "
        "95f36b628 and 9193b196a (corpus Unsafe 11 -> 12)"),
    "unittest_http_get_params_append": (
        "NULLABLE-GUARDED-RELEASE", "verified",
        "rule 74c (A) withdraws a settled leak beside an Unknown leaf (Unsafe -> Unknown only; "
        "SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN); bracketed by SP5's tip 38cf960fa and SP6's base 1f2007667 "
        "(corpus Unknown +1, Unsafe -1), whose only code commits are this card's"),
    "sb_remove_chunk_from_segregated_backlog": (
        "NULLABLE-GUARDED-RELEASE", "verified",
        "rule 74c reads `&chunk->_head` as an address inside *chunk, so `chunk` stays resolved through "
        "mk_list_del and sb_destroy_chunk; bracketed by 38cf960fa and 1f2007667 (system Unknown 330 -> 329, "
        "Unsafe 4 -> 5), whose only code commits are this card's; its block predicted none"),
}
UNIT_CAUSE = {  # unit rows follow their functions
    "rdhttp.c": "unittest_http_get_params_append",
    "rdkafka_sasl_scram.c": "rd_kafka_sasl_safe_string",
    "rdkafka_sticky_assignor.c": "ut_testSameSubscriptions",
    "plugins/in_storage_backlog/sb.c": "sb_remove_chunk_from_segregated_backlog",
    "rdunittest_acknowledge.c": "ut_case_acknowledge_accept",
}

STEP = {1: "prev->common", 2: "common->tip"}


def card_of(label):
    if "/chain-predictions/" in label:
        return label.split("/chain-predictions/")[1].split(".block")[0]
    if "/docs/reports/" in label:
        return label.split("/docs/reports/")[1].split("/")[0]
    return label


def main():
    chain = json.loads(Path(sys.argv[1]).read_text())
    table = []
    held = []
    by_key = {}
    ordered = ["corpus", "system", "slice", "twelve-home"]
    routes = chain["segments"][0]["routes"]
    for route in ordered:
        r = routes[route]
        units = []
        for x in r["rows"]:
            if x["pattern"] == "evidence-only":
                continue
            parts = x["row"].split("::")
            fn = parts[-1].split("#")[0] if x["kind"] == "function" else None
            name = fn or parts[1]
            verdicts = x["verdicts"]
            sources = sorted({(card_of(a["declared"]), "declared" if a["declared"].endswith(".md") else "card record")
                              for a in x.get("attributed", []) if "declared" in a})
            derived = any("derived_from_function_rows" in a for a in x.get("attributed", []))
            entry = {"route": route, "kind": x["kind"], "row": x["row"], "name": name,
                     "file": parts[1], "verdicts": verdicts, "moved_in": [STEP[s] for s in x["moved_in"]],
                     "net": x["net"]}
            if x["kind"] == "unit":
                units.append((entry, derived or bool(sources)))
                continue
            if sources and not x.get("unattributed_in"):
                cards = sorted({s.upper() for s, _ in sources})
                entry.update(card=", ".join(cards), cls="tool",
                             evidence="chain tool: " + "; ".join(sorted({k for _, k in sources})))
            elif fn in CAUSES:
                card, cls, ev = CAUSES[fn]
                entry.update(card=card, cls=cls, evidence=ev)
            elif route == "twelve-home":
                twin = by_key.get(("corpus", x["row"]))
                if twin and twin["verdicts"] == verdicts and twin.get("card"):
                    entry.update(card=twin["card"], cls=twin["cls"] + "; twelve-home as corpus",
                                 evidence="the same row of the same preparation moves identically in the same step "
                                          "on corpus (" + twin["evidence"] + "); the card's block had no twelve-home "
                                          "route")
            by_key[(route, x["row"])] = entry
            table.append(entry)
        for entry, tool in units:
            fn_for = UNIT_CAUSE.get(entry["file"])
            fn_entry = next((e for e in table if e["route"] == entry["route"] and e["file"] == entry["file"]
                             and e["name"] == fn_for), None)
            if fn_entry and fn_entry.get("card"):
                entry.update(card=fn_entry["card"], cls=("tool" if tool else fn_entry["cls"]),
                             evidence=f"unit verdict follows {fn_for} (the unit's only moved function row)"
                             if not tool else "chain tool: every moved function row of the unit is attributed")
            table.append(entry)
    for entry in table:
        if not entry.get("card"):
            entry.update(card=None, cls="HELD", evidence="no prediction and no checked cause")
            held.append(entry)
    Path(sys.argv[3]).write_text(json.dumps({"rows": table, "held": held}, indent=1, sort_keys=True) + "\n")
    # Markdown: one line per (name, file, verdict sequence), routes merged.
    merged = {}
    for e in table:
        key = (e["file"], e["name"], e["kind"], tuple(e["verdicts"]), e["card"], e["cls"])
        merged.setdefault(key, []).append(e["route"] + (e["row"].split("::unit#")[1][0] if "::unit#" in e["row"] else ""))
    lines = ["| Row | Routes | prev / common / tip | Card | Class |", "|---|---|---|---|---|"]
    for (file, name, kind, verdicts, card, cls), routes in sorted(merged.items(), key=lambda kv: (kv[0][5], kv[0][4] or "", kv[0][0], kv[0][1])):
        label = f"`{file}::{name}`" if kind == "function" else f"unit `{file}`"
        lines.append(f"| {label} | {', '.join(sorted(set(routes)))} | {' / '.join(verdicts)} | {card} | {cls} |")
    Path(sys.argv[4]).write_text("\n".join(lines) + "\n")
    from collections import Counter
    print("rows", len(table), "held", len(held))
    print(Counter((e["route"], e["cls"]) for e in table))
    print(Counter(e["card"] for e in table if e["route"] == "corpus"))


if __name__ == "__main__":
    main()
