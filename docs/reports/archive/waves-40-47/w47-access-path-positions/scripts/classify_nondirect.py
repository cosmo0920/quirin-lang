#!/usr/bin/env python3
"""Group every non-direct changed leaf of a comparison by mechanism.

Usage: classify_nondirect.py COMPARE_DIR OUT_JSON

Mechanisms (checked in order; every leaf lands in exactly one group, and a
leaf no rule explains is `unexplained`, which the report must read):

* `callee-now-safe`: a removed `call to uncontracted function` whose callee
  moved Unknown -> Safe in this comparison (the existing summary rule);
* `callee-summary-now-derived`: a removed uncontracted-call leaf whose callee
  did not itself move in this comparison's function rows (a header-inline or
  static helper whose W4 summary now validates);
* `later-obligation`: an added `unresolved resource identity` in a function
  with at least one admitted W47 or new statement row: a borrowed or
  unresolved root the admitted read exposed, checked later in the function;
* `cascade-condition`: a removed branch/loop refusal in a function with no
  admitted row (its call-bearing condition now plans because a callee's
  summary validated);
* `exchanged-refusal`: an added `unsupported` leaf on the line of a removed
  refusal of the same function (a refusal exchanged for a later one);
* `later-exposed-refusal`: an added non-alias leaf in a function with an
  admitted row: an obligation the newly admitted read lets the flow reach;
* `reviewed:<mechanism>`: a leaf read by hand, keyed by its origin in
  `REVIEWED` below (a call whose callee's summary now validates, so the
  statement or condition carrying it is modelled by an existing route);
* otherwise `unexplained`.
"""

# Every leaf the rules above do not explain, read in the pinned source.
CALLEE = "callee-now-modelled"
REVIEWED = {
    # corpus
    "cmt_histogram.c:259": CALLEE + " (check_buckets now Safe; W44 call-argument route admits h->buckets)",
    "ctr_encode_opentelemetry.c:824": CALLEE + " (otel_span_set_kind now Safe; its span->kind actual admitted)",
    "rdkafka_cgrp.c:5487": CALLEE + " (rd_kafka_cgrp_rebalance_protocol now Safe; W44 condition plan)",
    "rdkafka_cgrp.c:2544": CALLEE + " (rd_kafka_cgrp_rebalance_protocol now Safe; W44 condition plan)",
    "rdkafka_cgrp.c:1984": CALLEE + " (rd_kafka_cgrp_rebalance_protocol now Safe; W44 condition plan)",
    "rdkafka_cgrp.c:7691": CALLEE + " (rd_kafka_cgrp_rebalance_protocol now Safe; W44 condition plan)",
    "rdkafka_mock_handlers.c:2967": CALLEE + " (rd_kafka_Uuid_cmp summary now validates; W44 condition plan)",
    "rdkafka_mock_handlers.c:4964": CALLEE + " (i == 0 || rd_kafka_Uuid_cmp(...) != 0; summary now validates)",
    "rdkafka_mock_handlers.c:4988": CALLEE + " (i == 0 || rd_kafka_Uuid_cmp(...) != 0; summary now validates)",
    "rdkafka_mock_handlers.c:4463": CALLEE + " (i == 0 || rd_kafka_Uuid_cmp(...) != 0; summary now validates)",
    "rdkafka_mock_handlers.c:4487": CALLEE + " (i == 0 || rd_kafka_Uuid_cmp(...) != 0; summary now validates)",
    "mk_server/mk_fifo.c:443": CALLEE + " (mk_fifo_queue_get now Safe; fw->fifo, fm->queue_id actuals admitted)",
    "mk_server/mk_http.c:1625": CALLEE + " (mk_http_status_completed's summary now validates; W44 condition plan)",
    "mk_server/mk_scheduler.c:729": CALLEE + " (mk_channel_is_empty's summary now validates)",
    "mk_server/mk_stream.c:291": CALLEE + " (mk_channel_is_empty's summary now validates)",
    "mk_server/mk_vhost.c:440": CALLEE + " (mk_list_is_empty's summary now validates)",
    "rdkafka_admin.c:5044": "later-obligation (reply read by rd_kafka_buf_read_* expansions the now-modelled condition reaches)",
    "rdkafka_admin.c:5052": "later-obligation (reply read by rd_kafka_buf_read_* expansions the now-modelled condition reaches)",
    "rdkafka_cgrp.c:743": "alias-no-longer-poisoned (the refused condition before it no longer invalidates ErrorCode, 10a)",
    "rdkafka_msg.c:1518": "alias-no-longer-poisoned (a now-modelled condition no longer invalidates err, 10a)",
    "chunkio.c:258": "alias-no-longer-poisoned (a now-modelled condition no longer invalidates arr, 10a)",
    "mk_core/mk_memory.c:69": "alias-no-longer-poisoned (`!p.data || p.len == 0` is now planned; buf keeps its null-tested identity)",
    "mk_server/mk_utils.c:484": "alias-no-longer-poisoned (a now-modelled condition no longer invalidates buf, 10a)",
    # system
    "src/flb_engine.c:1751": "alias-no-longer-needed (flb_engine_exit now Safe, so W4 no longer binds config)",
    "src/flb_engine_dispatch.c:58": "alias-no-longer-poisoned (the call-bearing condition at :54 now plans instead of refusing)",
    "src/flb_engine_dispatch.c:59": "alias-no-longer-poisoned (the call-bearing condition at :54 now plans instead of refusing)",
    "src/flb_output.c:318": CALLEE + " (mk_list_is_empty's summary now validates; &queue->pending admitted)",
    "src/flb_output.c:398": CALLEE + " (mk_list_is_empty's summary now validates)",
    "src/flb_output.c:407": CALLEE + " (mk_list_is_empty's summary now validates)",
    "src/flb_output.c:439": CALLEE + " (flb_task_set_route_data's header-inline summary now validates)",
}


import collections
import gzip
import json
import pathlib
import re
import sys


def main():
    compare, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    comparison = json.loads((compare / "comparison.json").read_text())
    safe_now = {m["function"] for m in comparison["moves"] if m["after"] == "Safe"}
    rows = [json.loads(line) for line in gzip.open(compare / "changed-leaves.jsonl.gz", "rt")]
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[(row["unit"], row["function"])].append(row)
    groups = collections.defaultdict(list)
    for row in rows:
        if row["attribution"].startswith("direct"):
            continue
        leaf = row["leaf"]
        description = leaf["description"]
        siblings = by_function[(row["unit"], row["function"])]
        admitted = bool(row["admitted_lines_in_function"]) or any(
            r["attribution"].startswith("direct") for r in siblings)
        callee = re.match(r"call to uncontracted function `([^`]+)`", description)
        if row["change"] == "removed" and callee:
            kind = "callee-now-safe" if callee.group(1) in safe_now else "callee-summary-now-derived"
        elif row["change"] == "added" and row["family"] == "unknown_alias" and admitted:
            kind = "later-obligation"
        elif row["change"] == "removed" and row["family"] == "unsupported:COND" and not admitted:
            kind = "cascade-condition"
        elif row["change"] == "added" and any(
                r["change"] == "removed" and r["leaf"]["line"] == leaf["line"] for r in siblings):
            kind = "exchanged-refusal"
        elif row["change"] == "added" and admitted:
            kind = "later-exposed-refusal"
        elif leaf["origin"] in REVIEWED:
            kind = "reviewed:" + REVIEWED[leaf["origin"]]
        else:
            kind = "unexplained"
        groups[kind].append({
            "change": row["change"], "family": row["family"], "library": row["library"],
            "file": row["file"], "function": row["function"], "before": row["before"],
            "after": row["after"], "description": description, "origin": leaf["origin"],
            "line": leaf["line"],
        })
    summary = {kind: len(items) for kind, items in sorted(groups.items())}
    out.write_text(json.dumps({"summary": summary, "groups": groups}, indent=1, sort_keys=True) + "\n")
    print(json.dumps(summary, indent=1, sort_keys=True))
    for kind in ("unexplained", "removed-alias-review", "exchanged-refusal", "cascade-condition"):
        for item in groups.get(kind, []):
            print(kind, item["change"], item["family"], item["file"], item["function"],
                  item["description"][:80], item["origin"])


if __name__ == "__main__":
    main()
