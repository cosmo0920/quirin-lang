## rule 67 (two or more direct calls in the statement)
| class | leaf groups | functions mentioning | sole functions alone | samples |
|---|---:|---:|---:|---|
| some uncontracted / statement:statement | 1144 | 447 | 9 | cmt_labels_create, rd_hexdump, rd_kafka_cgrp_heartbeat, rd_kafka_fetch_pos2str |
| all contracted / statement:statement | 1084 | 423 | 3 | append_metric_value, get_stream_path, rd_kafka_queue_poll |
| some uncontracted / return:return | 24 | 24 | 9 | rd_kafka_group_result_copy, rd_kafka_mock_broker_add, rd_kafka_mock_broker_decommission, rd_kafka_mock_broker_remove_from_metadata |
| some uncontracted / statement:for-init,statement:for-step,statement:statement | 6 | 6 | 0 |  |
| some uncontracted / statement:for-init,statement:for-step | 2 | 2 | 0 |  |
| some uncontracted / condition:if,return:return | 1 | 1 | 0 |  |

sole functions all of whose leaves are in some class: 21: append_metric_value, cmt_labels_create, get_stream_path, rd_hexdump, rd_kafka_cgrp_heartbeat, rd_kafka_fetch_pos2str, rd_kafka_group_result_copy, rd_kafka_interceptor_failed, rd_kafka_metadata_log, rd_kafka_mock_broker_add, rd_kafka_mock_broker_decommission, rd_kafka_mock_broker_remove_from_metadata, rd_kafka_mock_broker_set_down, rd_kafka_mock_broker_set_rtt, rd_kafka_mock_broker_set_up, rd_kafka_mock_set_apiversion, rd_kafka_mock_telemetry_set_push_interval, rd_kafka_offset_file_sync, rd_kafka_queue_poll, rd_kafka_timer_schedule, rd_kafka_topic_partition_list_log

## rule 68 (a pointer member spelled through a typedef)
| class | leaf groups | functions mentioning | sole functions alone | samples |
|---|---:|---:|---:|---|
| read via cfl_sds_t / root local / read:pointer-member | 54 | 36 | 0 |  |
| read via cfl_sds_t / root formal / read:pointer-member | 35 | 25 | 0 |  |
| store via cfl_sds_t / root local / store:pointer-call-result | 19 | 14 | 0 |  |
| address via cfl_sds_t / root local / call:uncontracted-callee | 17 | 16 | 10 | unpack_instrumentation_scope_name, unpack_instrumentation_scope_version, unpack_link_trace_state, unpack_opts_desc |
| store via cfl_sds_t / root formal / store:pointer-name | 14 | 8 | 0 |  |
| read via cfl_sds_t / root local / store:pointer-other | 5 | 5 | 0 |  |
| store via cfl_sds_t / root local / store:pointer-name | 5 | 4 | 0 |  |
| null-store via cfl_sds_t / root local / store:pointer-null | 4 | 4 | 0 |  |
| read via cfl_sds_t / root formal / store:pointer-call-result | 4 | 2 | 0 |  |
| read via cfl_sds_t / root formal / store:pointer-other | 4 | 3 | 0 |  |
| read via flb_sds_t / root local / read:pointer-member | 4 | 1 | 0 |  |
| address via cfl_sds_t / root local / read:pointer-member | 3 | 1 | 0 |  |
| address via cfl_sds_t / root local / address:sub-object | 2 | 1 | 0 |  |
| read via cfl_sds_t / root formal / call:uncontracted-callee | 2 | 2 | 0 |  |
| read via cfl_sds_t / root local / call:uncontracted-callee | 2 | 2 | 0 |  |
| store via cfl_sds_t / root local / store:pointer-identity-free | 2 | 1 | 0 |  |
| store via flb_sds_t / root local / store:pointer-call-result | 2 | 1 | 0 |  |
| null-store via flb_sds_t / root local / store:pointer-null | 2 | 1 | 0 |  |
| read via flb_sds_t / root local / call:uncontracted-callee | 2 | 1 | 0 |  |
| store via cothread_t / root local / store:pointer-call-result | 2 | 1 | 0 |  |
| read via cfl_sds_t / root formal / call:indirect-callee | 1 | 1 | 1 | compare_fqname |
| read via cfl_sds_t / root local / call:indirect-callee | 1 | 1 | 1 | compare_label_keys |

sole functions all of whose leaves are in some class: 18: compare_fqname, compare_label_keys, ctr_instrumentation_scope_create, unpack_event_name, unpack_instrumentation_scope_name, unpack_instrumentation_scope_version, unpack_link_trace_state, unpack_opts_desc, unpack_opts_name, unpack_opts_ns, unpack_opts_ss, unpack_resource_span_schema_url, unpack_scope_span_schema_url, unpack_span_name, unpack_span_schema_url, unpack_span_status_message, unpack_span_trace_state, unpack_static_label_component

