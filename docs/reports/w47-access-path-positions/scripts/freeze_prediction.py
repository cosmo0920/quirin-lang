#!/usr/bin/env python3
"""Freeze the W47 P3 per-function prediction (docs/reports/w47-access-path-positions).

Usage: freeze_prediction.py P1_PREDICTION BASELINE_DIAGNOSTICS OUT_JSON

Reads the frozen P1 forecast's `freed.P3` list (228 entries: 220 same-unit and
8 cross-unit) and the combined baseline diagnostics, and writes this card's
prediction: for every entry, the baseline verdict, this card's predicted
verdict, the step expected to move it (`positions` = step A, the three
positions; `enum` = step B, the owner-aware enum arm) and the reason. The
per-function judgements below were made by reading each function's blocking
leaves and source before any implementation; they are data, not derived.
"""

import json
import sys

# Step A: the typed layer in conditions, returns and call arguments.
POSITIONS = {
    "check_buckets": "condition `buckets->upper_bounds[i - 1] > buckets->upper_bounds[i]`: scalar mixed path, untracked formal root",
    "ctr_id_get_buf": "return of pointer member `cid->buf` from an untracked formal root",
    "rd_kafka_messages_get": "return of pointer element `messages->elems[index]` from an untracked formal root",
    "rd_kafka_opaque": "return of pointer `rk->rk_conf.opaque` (mixed path, untracked root)",
    "rd_kafka_ConfigEntry_name": "return of pointer chain `entry->kv->name`, untracked root",
    "rd_kafka_ConfigEntry_value": "return of pointer chain `entry->kv->value`, untracked root",
    "rd_kafka_Node_host": "return of pointer member `node->host`, untracked root",
    "rd_kafka_Node_rack": "return of pointer member `node->rack`, untracked root",
    "rd_kafka_conf_get_default_topic_conf": "return of pointer member `conf->topic_conf`, untracked root",
    "rd_kafka_event_config_string": "switch on an int typedef member and return of a pointer chain, untracked root",
    "rd_kafka_event_message_count": "switch on an int typedef member `rkev->rko_evtype`",
    "rd_kafka_handle_mock_cluster": "return of `rk->rk_mock.cluster` behind a carrier-preserving pointer cast the cast-return route strips",
    "rd_kafka_mock_cluster_bootstraps": "return of pointer member `mcluster->bootstraps`, untracked root",
    "rd_kafka_mock_cluster_handle": "return of `mcluster->rk` behind a carrier-preserving pointer cast",
    "rd_kafka_topic_partition_get_current_leader_epoch": "condition `!(parpriv = rktpar->_private)`: typed borrowed assignment then the existing null branch",
    "rd_kafka_topic_partition_get_leader_epoch": "condition `!(parpriv = rktpar->_private)`: typed borrowed assignment then the existing null branch",
    "rd_kafka_topic_partition_list_count_abs_offsets": "for condition `i < rktparlist->cnt` and branch `!((rktparlist->elems[i].offset) < 0)`",
    "rd_kafka_topic_partition_list_reset_offsets": "for condition `i < rktparlist->cnt`",
    "rd_kafka_share_partition_offsets_list_get": "return of pointer element `list->elems[index]`, untracked root",
    "rd_kafka_share_partition_offsets_offsets": "return of pointer member `partition_offsets->offsets`, untracked root",
    "rd_list_first": "condition `rl->rl_cnt == 0` and return of `rl->rl_elems[0]`, untracked root",
    "rd_list_last": "condition `rl->rl_cnt == 0` and return of `rl->rl_elems[rl->rl_cnt - 1]`, untracked root",
    "mk_ptr_print": "for condition `i < p.len && p.data != NULL` (short-circuit plan) and a scalar `p.data[i]` actual at a variadic data tail; uncertain: needs `printf`'s declared tail",
    "_next_buffer": "condition `pointer->data == buffers[0]` (pointer comparison) and returns of `buffers[k]` from a file-scope array",
}

# Step B: the owner-aware enum arm (positive EnumType evidence).
ENUM = {
    "otel_span_set_kind": "enum member stores `otel_span->kind = …` (P2 miss)",
    "LZ4F_resetDecompressionContext": "enum member store `dctx->dStage = …` (P2 miss)",
    "rd_kafka_consumer_group_state_name": "return of a file-scope `const char *` array indexed by an enum value",
    "rd_kafka_consumer_group_type_name": "return of a file-scope `const char *` array indexed by an enum value",
    "rd_kafka_cgrp_clear_last_err": "enum member store `rkcg->rkcg_last_err` (P2 miss)",
    "rd_kafka_cgrp_rebalance_protocol": "conditions compare the enum member `rkcg_group_protocol`",
    "rd_kafka_mock_cgrp_classic_check_state": "switch on the enum member `mcgrp->state` (its branch condition is step A)",
    "rd_kafka_op_get_offset": "conditions compare the enum member `rko->rko_type`",
    "rd_kafka_topic_partition_list_get_err": "condition reads the enum member `rktparlist->elems[i].err`",
    "rd_kafka_topic_partition_list_set_err": "store of the enum member `rktparlist->elems[i].err`",
    "ut_set_types": "enum element copy `entry->types[i] = types[i]` (P2 miss)",
}

# Predicted-Safe P1 rows this card does not serve, with the retained cause.
MISS = {
    "cio_file_real_size": "cross-unit callee `cio_file_native_get_size`; no cross-unit summary rule (W44-CALLEE-CROSS-UNIT-SUMMARY)",
    "cmt_initialize": "cross-unit callee `cmt_atomic_initialize`",
    "ctxTypeID_to_size": "cross-unit callees `LZ4_sizeofState`/`LZ4_sizeofStateHC`",
    "compute_flat_scope_index": "compound assignment `offset += …` in a statement (P2 reader handles `=` only)",
    "flb_input_chunk_get_name": "pointer cast in a statement path (P2 refusal kept)",
    "flb_input_chunk_is_storage_overlimit": "pointer cast in a statement path (P2 refusal kept)",
    "cJSON_GetErrorPtr": "pointer arithmetic in the return expression",
    "cJSON_SetNumberHelper": "assignment inside the return expression",
    "parse_hex4": "compound assignments `h += …` in statements",
    "update_offset": "pointer arithmetic `buffer->buffer + buffer->offset` in a statement",
    "extra_alloc": "pointer arithmetic `rbuf->rbuf_extra + of` in a statement",
    "rd_kafka_name": "returns the array member `rk->rk_name`: an interior address (a derived result, rule 5j), refused",
    "rd_kafka_mk_brokername": "a `?:` actual in the second `rd_snprintf` call",
    "rd_kafka_sw_str_sanitize_inplace": "multi-pointer declarator and pointer increments (P2 residual)",
    "rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic": "the `rd_tmpabuf_add_alloc` macro expands to a compound assignment through `->`",
    "rd_kafkap_leader_discovery_tmpabuf_add_alloc_topics": "the `rd_tmpabuf_add_alloc` macro expands to a compound assignment through `->`",
    "rd_kafka_sasl_cyrus_ready": "call `rd_atomic32_get(&handle->ready)` inside the return expression",
    "ut_get_consumer_rack": "`sizeof` of an expression (RD_ARRAYSIZE) in a statement",
    "ut_testConflictingPreviousAssignments": "indexed aggregate pointer stores (P5) and call arguments",
    "clear_unit_test_data": "stores into fields of a file-scope record (P2 residual)",
    "get_client_rack": "`&&` inside a return expression (returns are single reads here)",
    "get_group_id": "`?:` with a pointer cast inside a return expression",
    "get_group_instance_id": "`?:` with a pointer cast inside a return expression",
    "get_transactional_id": "`?:` with a pointer cast inside a return expression",
    "XXH32_reset": "local aggregate address handed to `memcpy` (P2 residual)",
    "XXH64_reset": "local aggregate address handed to `memcpy` (P2 residual)",
}


def main():
    p1_text = open(sys.argv[1]).read()
    p1 = json.loads(p1_text[p1_text.index("\n") + 1:])
    diagnostics = json.load(open(sys.argv[2]))
    status = {}
    for unit in diagnostics["units"]:
        for function in unit.get("functions", []):
            status.setdefault((unit["file"], function["function"]), []).append(function["status"])
    rows = []
    for entry in p1["freed"]["P3"]:
        found = [s for (file, name), ss in status.items() if name == entry["function"]
                 and file.endswith(entry["file"]) for s in ss]
        baseline = found[0] if found and len(set(found)) == 1 else "ambiguous"
        name = entry["function"]
        if baseline == "Safe":
            verdict, step, reason = "Safe", "none", "already Safe on the combined baseline (P2); no Safe to non-Safe move expected"
        elif name in POSITIONS:
            verdict, step, reason = "Safe", "positions", POSITIONS[name]
        elif name in ENUM:
            verdict, step, reason = "Safe", "enum", ENUM[name]
        elif name in MISS:
            verdict, step, reason = "Unknown", "none", MISS[name]
        elif entry["verdict"] == "Unknown":
            verdict, step = "Unknown", "none"
            reason = "P1: " + entry["reason"][:300]
        else:
            raise SystemExit(f"unclassified P1 Safe row {name}")
        rows.append({
            "library": entry["library"],
            "file": entry["file"],
            "function": name,
            "cross_unit": entry["needs_cross_unit_summary"],
            "p1_verdict": entry["verdict"],
            "baseline": baseline,
            "predicted": verdict,
            "step": step,
            "reason": reason,
        })
    used = {r["function"] for r in rows}
    for table in (POSITIONS, ENUM, MISS):
        missing = set(table) - used
        if missing:
            raise SystemExit(f"names not in the P1 P3 list: {sorted(missing)}")
    summary = {}
    for row in rows:
        key = f'{row["baseline"]}->{row["predicted"]}:{row["step"]}'
        summary[key] = summary.get(key, 0) + 1
    out = {"schema": "qcore.w47-access-path-positions-prediction.v1", "summary": summary, "rows": rows}
    with open(sys.argv[3], "w") as handle:
        json.dump(out, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps(summary, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
