| sub-shape | sole functions mentioning | sole functions blocked by it alone | owner / reason | sample |
|---|---:|---:|---|---|
| `several` | 0 | 45 |  | `cat_histogram_values`, `cat_summary_values`, `unpack_context_external_metadata` |
| `uncontracted-callee-unread-actual` | 55 | 40 | contracts (rule 12k cannot name: an actual it does not read) | `unpack_opts_desc`, `unpack_opts_name`, `unpack_opts_ns` |
| `embedded-list-address` | 23 | 23 | container custody (rd_list_*): decision class 1 / rule 46 | `pack_static_labels`, `rd_kafka_ConfigResource_add_ConfigEntry`, `rd_kafka_assignors_term` |
| `pointer-member-read-actual` | 19 | 16 | callee's mode or another premise of the statement | `cio_file_native_open`, `print_string`, `LZ4F_localSaveDict` |
| `pointer-name-into-member` | 19 | 13 | decision class 1 (retained_by) / rule 10v | `cio_set_log_callback`, `create_counter_instance`, `create_exp_histogram_instance` |
| `sub-object-address-actual` | 17 | 12 | callee's mode or another premise | `LZ4_prepareTable`, `LZ4HC_clearTables`, `rd_kafka_Uuid_copy` |
| `formal-array-element-released` | 12 | 12 | rule 5k / W45-MEMBER-RELEASE-PREMISES (by design) | `rd_kafka_AclBinding_destroy_array`, `rd_kafka_AlterConsumerGroupOffsets_destroy_array`, `rd_kafka_ConfigResource_destroy_array` |
| `owned-result-into-formal-member` | 14 | 11 | MEMBER-EFFECT-STORES (rule 38i names it) | `otel_span_set_attributes`, `otel_span_set_events`, `rd_buf_init` |
| `nested-return-op-req` | 18 | 8 | contract: rd_kafka_op_err_destroy (cross-unit, uncontracted) | `rd_kafka_mock_broker_add`, `rd_kafka_mock_broker_decommission`, `rd_kafka_mock_broker_remove_from_metadata` |
| `indirect-call` | 9 | 7 | rule 11p | `compare_label_keys`, `cJSON_New_Item`, `cJSON_free` |
| `custody-free-read-other` | 8 | 7 | ?: / cast / descriptor | `cio_file_native_close`, `rd_kafka_broker_weight_usable`, `rd_kafka_mock_msgset_est_record_size` |
| `pointer-arithmetic-store` | 8 | 7 | rule 10v (identity-carrying value) | `LZ4_renormDictT`, `buf_read`, `buf_write` |
| `union-member-access` | 10 | 5 | rule 10w residual | `LZ4_resetStream_fast`, `LZ4_compressHC2_continue`, `LZ4_compressHC2_limitedOutput_continue` |
| `owned-result-into-local-member` | 7 | 5 | rule 61 (1) where the base is alias-spelled; else B08 premises | `rd_kafka_TopicCollection_of_topic_names`, `rd_kafka_UserScramCredentialDeletion_new`, `rd_kafka_topic_partition_new_from_rktp` |
| `form:aggregate-value` | 6 | 5 | see premise table | `LZ4F_freeCDict`, `LZ4F_freeCompressionContext`, `LZ4F_freeDecompressionContext` |
| `union-member-pointer-store` | 18 | 4 | rule 10w (a pointer store through a union refuses) | `kvlist_fetch_string`, `LZ4_attach_HC_dictionary`, `rd_kafka_q_op_err` |
| `pointer-other-store` | 10 | 4 | rule 10v (identity-carrying value) | `create_reference`, `ut_populate_internal_broker_metadata`, `rd_kafka_timer_unschedule` |
| `no-evaluated-path` | 4 | 3 | see premise table | `rd_kafka_sw_str_sanitize_inplace`, `rd_kafka_share_partition_offsets_list_new`, `rd_kafka_topic_cmp_rkt` |
| `member-released-parent-kept` | 10 | 2 | rule 38j / MEMBER-EFFECT-SUMMARY | `rd_kafka_assignment_destroy`, `rd_kafkap_Produce_reply_tags_destroy` |
| `root:file-scope` | 3 | 2 | see premise table | `clear_unit_test_data`, `unit_test_telemetry_decoded_int64` |
| `log-macro-actuals` | 3 | 2 | contract/decision: rd_kafka_log0 (callback, class 1) | `rd_kafka_idemp_drain_toppar`, `rd_kafka_toppar_reset_base_msgid` |
| `root:call-result` | 2 | 2 | see premise table | `cmt_labels_create`, `rd_hexdump` |
| `lock-or-atomic-address` | 8 | 1 | nested borrowed calls (rule 14a: one call) / uncontracted | `append_metric_value` |
| `refcount-keep-stored` | 4 | 1 | decision class 3 (reference counts) | `rd_kafka_op_new_reply` |
| `queue-macro-iteration` | 3 | 1 | rule 5l list iteration residual | `rd_kafka_mock_error_stack_find` |
| `store:custody-free` | 1 | 1 | see premise table | `rd_hdr_iter_next` |
| `root:static-local` | 1 | 1 | see premise table | `rd_kafka_fetch_pos2str` |
| `form:cast` | 2 | 0 | see premise table |  |

total sole functions 240
