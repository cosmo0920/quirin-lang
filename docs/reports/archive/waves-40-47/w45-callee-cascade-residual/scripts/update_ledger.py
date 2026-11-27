#!/usr/bin/env python3
"""W45-CALLEE-CASCADE-RESIDUAL: amend W45-CALLEE-CASCADE-RESIDUAL,
W48-CALLEE-DECISION-TABLE, W43-LIBC-ABI-READER-SHAPES and
W44-CALLEE-DESTRUCTOR-SUMMARY in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length, unchanged:
nothing registered or retired). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (W45-CALLEE-CASCADE-RESIDUAL"

AMEND = {
    "W45-CALLEE-CASCADE-RESIDUAL": (
        " AMENDED (W45-CALLEE-CASCADE-RESIDUAL, wave 48 round 20, base 3905139cd, corpus 66910c10 "
        "ce57d194 756/9/4,069 -> 2734991a 803/9/4,022, scripts/rank.py, artifacts/rank-after-corpus.json): "
        "the 704 reproduce, and so do the cited heads (cJSON_New_Item 10, LZ4_decompress_generic 9, "
        "rd_kafka_confval_set_type 5, rd_kafka_interceptor_method_add 4, rd_kafka_queue_new0 3, "
        "rd_list_add 3, rd_list_clear 3, the ctr_attributes_set_* chain 2 each). The 90 callees deciding "
        "at least 2 alone were read at both revisions and classified "
        "(artifacts/classification-before.json); 21 declared, pinned contracts in qcore-provider-librdkafka "
        "(cJSON_New_Item, three one-shot LZ4 functions, load_descriptor_values, "
        "rd_kafka_aborted_txns_next_offset, rd_kafka_anyconf_desensitize, rd_kafka_dump0, "
        "rd_kafka_metadata_copy_internal), qcore-provider-ctraces (ctr_mpack_consume_int_tag/_uint_tag, "
        "ctr_attributes_set_string/_bool/_int64/_double), quirin-provider-cfl (cfl_kvlist_count, "
        "cfl_kvlist_insert_string/_bool/_int64/_double) and qcore-provider-chunkio (_cio_file_up) freed "
        "47 (+47 Unknown -> Safe, no other status move, no leak or violation added; system and slice no "
        "move). Remaining: 652 Unknown functions blocked by uncontracted calls alone, 473 by one callee "
        "alone. New head: LZ4_decompress_generic 9 (two untagged enum typedef parameters: the declared-ABI "
        "reader refuses enums, W43-LIBC-ABI-READER-SHAPES), cmt_mpack_unpack_array 5 and "
        "ctr_mpack_unpack_array 5 (callbacks), rd_kafka_confval_set_type 5 (retains valuep), "
        "LZ4_compress_HC_extStateHC 4, rd_kafka_admin_result_ret_groups 4 (child pointer), "
        "rd_kafka_error_new_v 4 (enum), rd_kafka_interceptor_method_add 4, sds_cat_safe 4. Every remaining "
        "callee deciding 2 or more is a W48-CALLEE-DECISION-TABLE class (1: 36 callees/94 functions; 2: "
        "7/18; 3: 9/19; 7: 1/2; 8: 6/16), a reader shape (enum 9/28, const T ** 3/7), a destructor whose "
        "callers would move at most 1 each (4/8, W44-CALLEE-DESTRUCTOR-SUMMARY), cJSON's static `print` "
        "(2; a generic name, not declared) or a va_list actual (2). DISCHARGE now: a decision or a "
        "reader widening per class, each re-running scripts/rank.py on its arbiter."
    ),
    "W48-CALLEE-DECISION-TABLE": (
        " AMENDED (W45-CALLEE-CASCADE-RESIDUAL, corpus 803/9/4,022, functions decided alone by one "
        "callee of the class on that run, artifacts/tally-after.txt): class 1 36 callees / 94 "
        "functions, now also holding custody-taking inserts (cJSON's add_item_to_object 3, "
        "add_item_to_array 2, replace_item_in_object 2; ctr_attributes_set_array and _kvlist 2 each, "
        "whose cfl_kvlist_insert_array/_kvlist take custody of value on success only), retaining LZ4 "
        "stream states (LZ4_compress_HC_extStateHC 4, LZ4_compress_HC_continue 3, "
        "LZ4_compress_fast_continue 3, LZ4_compress_fast_extState 2: pointers into src left in a "
        "caller's state), rd_kafka_confval_set_type 5 (its PTR arm stores valuep), the mpack array/map "
        "unpackers (cmt_ 5+3, ctr_ 5+3), rd_list_destroy 3 and rd_list_clear 3 (free callbacks over "
        "retained elements), append_to_ring_buffer (stores ins in the chunk it hands the ring buffer; "
        "corpus 2, system 2), cJSON_ParseWithLengthOpts 2 (stores value in global_error), nanopb "
        "pb_write/pb_encode_varint 2 each (stream callback), the rd_kafka_q_serve* family and "
        "rd_kafka_op_destroy; class 2 7 / 18 (sds_cat_safe 4, cfl_sds_cat_safe 3, format_metric 3, "
        "mk_header_prepare, rd_kafka_parse_Metadata0, two unit-test constructors); class 3 9 / 19 "
        "(rd_kafka_queue_new0 3, rd_atomic32_add 2, the partition-list destructors that drop toppar "
        "references); class 7 1 / 2 (_mk_event_wait_2); class 8 6 / 16 "
        "(rd_kafka_admin_result_ret_groups 4, _resources 3, _topics 3, get_object_item 2, "
        "get_or_create_external_metadata_kvlist 2, rd_kafka_topic_partition_get_private 2)."
    ),
    "W43-LIBC-ABI-READER-SHAPES": (
        " AMENDED (W45-CALLEE-CASCADE-RESIDUAL, corpus 803/9/4,022): the enum-typedef shape is the "
        "largest single blocker of the functions blocked by uncontracted calls alone: 9 callees decide "
        "28 corpus functions alone (LZ4_decompress_generic 9: untagged enum typedefs earlyEnd_directive "
        "and dict_directive; rd_kafka_error_new_v 4, rd_kafka_error_new 3 and six more over "
        "rd_kafka_resp_err_t or other librdkafka enums, 2 each). A further shape, a pointer to pointer "
        "to const (`const char **`, `const void **`; pointer_projection admits only `T **`), keeps 3 "
        "callees / 7 functions undeclarable (rd_flags2str 3, rd_kafka_anyconf_dump 2, rd_slice_reader0 2)."
    ),
    "W44-CALLEE-DESTRUCTOR-SUMMARY": (
        " AMENDED (W45-CALLEE-CASCADE-RESIDUAL, corpus 803/9/4,022): four destructors decide 2 "
        "functions alone each and were not declared: mk_fifo_queue_destroy, rd_kafka_AclBinding_destroy, "
        "rd_kafka_UserScramCredentialAlteration_destroy, sched_thread_cleanup. Their callers release an "
        "element taken from a list or an array, or wrap a void * formal, so a Released declaration would "
        "free at most one caller each."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = {entry["id"]: entry for entry in matrix["named_followups"]}
    for key, text in AMEND.items():
        entry = entries[key]
        if MARK not in entry["why"]:
            entry["why"] += text
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("followups", len(matrix["named_followups"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
