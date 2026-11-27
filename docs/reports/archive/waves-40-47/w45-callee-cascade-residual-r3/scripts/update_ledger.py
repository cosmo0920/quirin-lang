#!/usr/bin/env python3
"""W45-CALLEE-CASCADE-RESIDUAL round 3: amend W45-CALLEE-CASCADE-RESIDUAL,
W44-CALLEE-DESTRUCTOR-SUMMARY, W48-CALLEE-DECISION-TABLE and
W43-LIBC-ABI-READER-SHAPES, and register three residuals, in the acceptance
matrix (indent=1, ensure_ascii=False; `dimensions.named_followups` = list
length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/w45-callee-cascade-residual-r3/README.md"
CARD = "W45-CALLEE-CASCADE-RESIDUAL round 3"
MARK = "AMENDED (" + CARD
RUN = (
    "corpus 66910c10, base ef6b32192, binary 551fa31b, ded14632 915/9/3,910 -> cdb0267b "
    "933/9/3,892; system 91/4/353 -> 93/4/351; slice 50/1/349/4 Pending -> 52/1/347/4 Pending"
)

AMEND = {
    "W45-CALLEE-CASCADE-RESIDUAL": (
        " " + MARK + ", " + RUN + "; null property unchanged at 1,113 / 1,435 / 2,355 / 18 "
        "with 29 findings; scripts/rank.py, artifacts/rank-a3-corpus.json): the BEFORE population "
        "is 506 Unknown functions blocked by uncontracted calls alone, 352 by one callee alone "
        "(CG22's 501 / 348 plus the null cards' moves, among them cfl_list_size's two callers). "
        "Fourteen pinned declarations, each body read at 66910c10 and 046accf6: the five Admin-API "
        "destructors (rd_kafka_AclBinding_destroy, _ConfigResource_destroy, _NewPartitions_destroy, "
        "_NewTopic_destroy, _UserScramCredentialAlteration_destroy), mk_fifo_queue_destroy, "
        "unit_test_telemetry (a synchronous callback invocation), cfl_list_size, flb_hash_table_del "
        "(slice), and five 1-tier callees whose own body is Safe in its home unit and validates "
        "the declaration (W4 and entry): rd_bytes_hash, rd_kafka_mem_calloc, "
        "cio_file_native_get_size, cio_stream_size_chunks_up, cmt_atomic_initialize. Corpus +18 "
        "Unknown -> Safe (5 `_free` wrappers, 3 telemetry tests, 3 cmetrics counters, 7 through "
        "the validated 1-tier), system +2 (flb_input_chunk_total_size, two configurations), slice "
        "+2 (flb_tail_scan_unregister_*); no Safe -> non-Safe move, no leak or violation added; "
        "corpus Safe rows resting only on validated declarations 101 -> 108. Two declarations were "
        "measured and withdrawn: cio_file_hash (its own row Safe -> Unknown on run a1, "
        "BORROWED-RESULT-SUMMARY-CONFLICT) and cJSON's static print (+2 on run a1, but the "
        "contract corpus's compile route reads `print (` as a .qr print call, "
        "QR-PRINT-ROUTE-MATCHES-DECLARATION). Remaining: 481 Unknown functions blocked by "
        "uncontracted calls alone, 329 by one callee alone (system 26 / 18, slice 40 / 26). The "
        "26 callees still deciding 2 or more corpus functions alone (57 functions) are each a "
        "W48-CALLEE-DECISION-TABLE class, a reader extension (mockv_internal's va_list, "
        "W43-LIBC-ABI-READER-SHAPES; the partition-list set operations' map-stored callbacks, "
        "CALLBACK-STORED-IN-FRAME-MAP), an element-releasing destructor (sched_thread_cleanup, "
        "W44-CALLEE-DESTRUCTOR-SUMMARY) or print; slice file_pending_flush 2 has two static "
        "definitions of one name. The 1-tier was entered only for callees whose home body is "
        "Safe, and is exhausted at a yield of 1 per contract. DISCHARGE unchanged: a decision or "
        "a reader extension per class, each re-running scripts/rank.py on its arbiter."
    ),
    "W44-CALLEE-DESTRUCTOR-SUMMARY": (
        " " + MARK + ", " + RUN + "): the five Admin-API destructors (each releases its "
        "object and what it owns; none drops a reference or releases a child it does not own) "
        "and mk_fifo_queue_destroy (unlinks the queue from its list, frees it) are declared "
        "`resource_released` and pinned. They free the five `_free` wrappers and nothing else: "
        "the five `_destroy_array` loops now read `indirect memory access is outside the "
        "resource projection` at the element actual (W44-CALL-ARGUMENT-CUSTODY-POSITION); "
        "mk_fifo_queue_destroy's callers read `unresolved resource identity q` (an mk_list_entry "
        "element and mk_fifo_queue_get's result); three ConfigResource response parsers read "
        "`unresolved resource identity config`. The bodies are Unknown in rdkafka_admin.c and "
        "mk_fifo.c, so each declaration is a declared assumption (no body evidence, none "
        "refuted). sched_thread_cleanup (2 alone) releases every element of a caller's intrusive "
        "list; no clause states it (resource_elements_dropped is registry-only): not declared."
    ),
    "W48-CALLEE-DECISION-TABLE": (
        " " + MARK + ", " + RUN + "; functions decided alone, artifacts/rank-a3-corpus.json): "
        "two members surfaced that need a decision. cJSON_DetachItemViaPointer 3 is a detach: "
        "custody of `item` moves from the parent's child list to the result, which is `item` "
        "itself; resource_return_alias says the result owns nothing and resource_return_owned "
        "that it is fresh, and neither is the body. rd_kafka_destroy_app 2 releases rk only "
        "without RD_KAFKA_DESTROY_F_IMMEDIATE and invokes application-registered callbacks held "
        "in rk (log_cb, the consumer close's rebalance and commit callbacks, interceptors' "
        "on_destroy): an open callback set registered elsewhere, the shape of "
        "CG22-OP-CALLBACKS-AND-SERVED-QUEUES. Still undecided (unchanged): class 1 rd_kafka_log0 "
        "3, add_item_to_object 3, rd_kafka_assignor_add 3, cJSON_ParseWithLengthOpts 2, call_once "
        "2, rd_kafka_op_destroy, rd_kafka_q_serve0, rd_kafka_q_pop_serve0 and "
        "rd_kafka_q_serve_maybe_consume 2 each; class 2 format_metric 3, mk_header_prepare 2, "
        "rd_kafka_parse_Metadata0 2, setupRackAwareAssignment0 2, ut_init_member_internal 2; "
        "class 3 rd_atomic32_add 2, rd_kafka_txn_clear_partitions_flag 2; class 7 "
        "_mk_event_wait_2 2; class 8 get_or_create_external_metadata_kvlist 2."
    ),
    "W43-LIBC-ABI-READER-SHAPES": (
        " " + MARK + ", " + RUN + "): the `va_list` shape also keeps "
        "rd_kafka_metadata_new_topic_mockv_internal undeclared (2 corpus functions alone, "
        "rd_kafka_metadata_new_topic_mockv and _with_partition_replicas_mockv). Measured on "
        "this base: the one declaration with a `va_list` parameter, rd_kafka_error_new_v, is "
        "withdrawn as `abi-unreadable-declaration` in rdkafka_error.c (4 leaves), so a "
        "declaration of mockv_internal would be withdrawn at both callers."
    ),
}

NEW = [
    {
        "id": "QR-PRINT-ROUTE-MATCHES-DECLARATION",
        "title": "The compiler routes any module whose tokens read `print (` to the .qr print-call lowering, so a contract cannot declare a C function named `print`",
        "why": (
            "Measured by " + CARD + " (" + RUN + "): `qcore_frontend::source_scalar_calls::"
            "has_print_calls` (crates/qcore-frontend/src/source_scalar_calls.rs:94-98) is a "
            "lexical test for the word `print` followed by `(` anywhere in a program, and "
            "qcore-verification's driver (driver.rs:2056) then lowers the whole module through "
            "the scalar-calls route, which refuses every `extern fn` declaration (Q020 \"only "
            "source function declarations are supported\"). A declaration of cJSON's static "
            "`print` (cJSON.c:1234-1299, read at both revisions; its pinned identity binds the "
            "name to that one definition) in librdkafka/cjson.qv therefore failed "
            "qcore_contract_corpus's compile regression at the module's first declaration, "
            "although the resource loader and pin accepted it and run a1 measured +2 (cJSON_Print, "
            "cJSON_PrintUnformatted Unknown -> Safe). The card withdrew the declaration. "
            "DISCHARGE: the print-call route selection reads a call expression (not a declaration "
            "name) or a contract module is never compiled on it; then declare `print` as recorded "
            "in the card's report and measure."
        ),
        "source": REPORT,
        "recorded_by": "W45-CALLEE-CASCADE-RESIDUAL",
    },
    {
        "id": "CALLBACK-STORED-IN-FRAME-MAP",
        "title": "A callback a body stores into a frame-local map initializer and invokes through the map has no checkable clause (3 callees, 6 corpus functions alone)",
        "why": (
            "Measured by " + CARD + " (" + RUN + "): rd_kafka_topic_partition_list_difference0, "
            "_intersection0 and _union0 (rdkafka_partition.c:4961-5034, static, byte-identical at "
            "66910c10 and 046accf6) each decide 2 corpus functions alone (the _by_id and _by_name "
            "wrappers). Each passes its `cmp` and `hash` formals to `RD_MAP_INITIALIZER` (a "
            "frame-local map), which RD_MAP_SET and RD_MAP_GET then call during the call; nothing "
            "outlives it, and the result is a fresh partition list of copies. "
            "`resource_callback_invoked` is the closest clause, but rule 13y's body check admits "
            "it only where the body names the callback solely as the callee of direct calls, so "
            "it would be withdrawn in the one unit that holds every caller, and a borrowed "
            "reading of a callback is forbidden (C04.19). DISCHARGE: a body check (or a clause) "
            "for a callback stored into a frame object that does not escape and is invoked only "
            "through it, with its Lean model; then declare the three and measure."
        ),
        "source": REPORT,
        "recorded_by": "W45-CALLEE-CASCADE-RESIDUAL",
    },
    {
        "id": "BORROWED-RESULT-SUMMARY-CONFLICT",
        "title": "A borrowed-result declaration whose body's W4 summary is neutral turns the body's own Safe row Unknown",
        "why": (
            "Measured by " + CARD + " (run a1, " + RUN.split(';')[0] + " -> 5354f216 934/9/3,891): "
            "`cio_file_hash(cf)` (lib/chunkio/src/cio_file.c:1304-1307, `return (cf->map + 2);`) "
            "declared `resource_return_borrowed_from(cf, nullable)` in chunkio/chunk_residency.qv "
            "moved its own row Safe -> Unknown: the derived summary is "
            "`{parameters: [Borrowed], result: None}`, which `contradicts the declared contract` "
            "(SummaryConflict), and its one caller cio_chunk_hash stayed Unknown (its base is "
            "the member read `ch->backend`). Rule 13a's structural check reads such a body as "
            "reached from the base, but W4's inference does not derive a borrowed result through "
            "a member read plus an offset, so the declaration and the summary disagree. The card "
            "withdrew the declaration (no Safe -> non-Safe move on its AFTER). DISCHARGE: the "
            "derived-result inference (C04.3.2) publishes the borrowed result rule 13a admits, or "
            "a 13a-validated declaration is not compared with a less precise inferred summary; "
            "then declare cio_file_hash and measure."
        ),
        "source": REPORT,
        "recorded_by": "W45-CALLEE-CASCADE-RESIDUAL",
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    by_id = {e["id"]: e for e in entries}
    for key, text in AMEND.items():
        entry = by_id[key]
        if MARK not in entry["why"]:
            entry["why"] += text
    for new in NEW:
        if new["id"] not in by_id:
            entries.append(new)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
