#!/usr/bin/env python3
"""OUTER-CONTRACT: register stage 2 (librdkafka's envelope, after
NONNULL-MEMBER-UNIFIED-QUALIFIER), the library-mode host (USER DECISION D4
"Later") and the checkpoint route's projection. Idempotent. indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "OUTER-CONTRACT"
SOURCE = "docs/reports/outer-contract/README.md"

NEW = [
    {
        "id": "RK-PLUGIN-LIBRARY-OUTER-ENVELOPE",
        "title": "Stage 2 of the outer contract: librdkafka's plugin libraries as one rk_plugin_library envelope in place of the 163 rk_no_plugin_library premise clauses",
        "why": "OUTER-CONTRACT (rule 31, C04.20; USER DECISIONS 2026-10-17 D1 \"B: loader envelope\") implemented "
        "stage 1, Fluent Bit's flb_external_plugin. librdkafka's plugin.library.paths (Fluent Bit passes "
        "rdkafka.plugin.library.paths through, src/flb_kafka.c:82-87; WITH_PLUGINS 1) still rests on the free-text "
        "premise rk_no_plugin_library (163 clauses: 92 consulted custody declarations, the 66 premised scope "
        "functions and the rd_kafka_broker_s.rkb_rk stable fact), which the runtime projection reads as an "
        "uncovered closed-world fact: with flb_external_plugin loaded it moves corpus custody 31 and null 162 "
        "Safe rows to Unknown (see the report). PREREQUISITE: NONNULL-MEMBER-UNIFIED-QUALIFIER, which is "
        "migrating the bounds clauses that carry the premise; do not edit them before it lands. DISCHARGE: "
        "an outer module in qcore-provider-librdkafka defining rk_plugin_library on rd_kafka_plugin_new "
        "(binder #conf, guard #result == @RD_KAFKA_RESP_ERR_NO_ERROR), pinned with pin --outer at 66910c10 and "
        "046accf6, with an envelope covering the rk_conf callback members and interceptor lists "
        "(|-> #registered), rd_kafka_broker_s.rkb_rk and rule 14t's five librdkafka members (|-> #kept), and the "
        "internal closed-world premises (#premise assignor_callback, broker_monitor_callback, conf_property_writer, "
        "share_ack_batches_nonempty, served_queue, cjson_list_invariant, and cjson_allocator_hooks or a reading "
        "that excludes it); the rk_no_plugin_library clauses retired; the Fluent Bit envelope decided for the "
        "librdkafka facts a Fluent Bit plugin reaches (or those rows left Unknown in the all scenario, named); "
        "every runtime-moved row read; corpus, system and slice measured; Lean beside QCore.OuterContract if "
        "the semantics move.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "FLB-LIB-HOST-OUTER-CALLER",
        "title": "The library-mode host (flb_lib, out_lib, in_lib) as an outer caller under the outer contract vocabulary",
        "why": "OUTER-CONTRACT-DESIGN's census M8 (USER DECISION 2026-10-17 D4 \"Later\"): an application embedding "
        "libfluent-bit is code the audit never reads that calls the exported API (flb_ctx_t, whose config is "
        "public) and registers callbacks (out_lib's cb, flb_output_set_test). No row rests on a closed-world fact "
        "it alone could falsify today. DISCHARGE: decide whether the outer vocabulary states an outer caller "
        "(hosted on flb_create / flb_output_set_callback, binder the context) or the library-mode build is a "
        "separate profile; if declared, pinned and measured like flb_external_plugin, every moved row read.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "OUTER-PROJECTION-CHECKPOINT-ROUTE",
        "title": "The runtime projection on a checkpointed run and its composition",
        "why": "OUTER-CONTRACT selects the projection (QCORE_OUTER_PROJECTION) on the pinned, configured and "
        "no-build routes only; a checkpointed run refuses it (checkpoint_run.rs, as it refuses the null and "
        "bounds selections), so a composed report has no runtime scope. DISCHARGE: the checkpoint carries the "
        "outer environment's identity and each loader's reach, the composer projects the composed rows exactly "
        "as a whole run does (or refuses when the checkpoints disagree), tested on a split run equal to the "
        "whole one.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {e["id"] for e in entries}
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


main()
