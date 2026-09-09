# C03-REACH per-member census table

Every refused member of every recursive strongly connected component in the
pinned Cycle 19 corpus (Fluent Bit `66910c10`, explicit Clang), with the first
V1-V10 obligation that refused it and the blocker families of the instructions
that blocked it. `comp` is the size of the member's component; publication is
all-or-nothing over a component, so a component with any refused member
publishes nothing.

Generated from `artifacts/census.json.gz`. `` `_` `` in a family name is a
collapsed identifier, so that the same refusal for two different callees is one
family. **Every one of the 46 components is refused: the neutral route's reach
on this corpus is 0.**

| library | unit | member | comp | first refusing obligation | blocked | dominant blocker families |
|---|---|---|---|---|---|---|
| chunkio | `cio_os.c` | `cio_os_mkpath` | 1 | `V5-retained-unknown-instruction` | 7 | 3 call to uncontracted function `_`; 2 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_decode_opentelemetry.c` | `clone_array` | 5 | `V5-retained-unknown-instruction` | 2 | 1 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in for-loop condition |
| cmetrics | `cmt_decode_opentelemetry.c` | `clone_array_entry` | 5 | `V5-retained-unknown-instruction` | 3 | 2 call to uncontracted function `_`; 1 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_decode_opentelemetry.c` | `clone_kvlist` | 5 | `V5-retained-unknown-instruction` | 2 | 1 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in for-loop condition |
| cmetrics | `cmt_decode_opentelemetry.c` | `clone_kvlist_entry` | 5 | `V5-retained-unknown-instruction` | 3 | 1 call to uncontracted function `_`; 1 indirect memory access is outside the resource projection |
| cmetrics | `cmt_decode_opentelemetry.c` | `clone_variant` | 5 | `V5-retained-unknown-instruction` | 24 | 8 call to uncontracted function `_`; 8 indirect memory access is outside the resource projection |
| cmetrics | `cmt_encode_opentelemetry.c` | `cfl_variant_array_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 5 | 2 call to uncontracted function `_`; 2 indirect memory access is outside the resource projection |
| cmetrics | `cmt_encode_opentelemetry.c` | `cfl_variant_kvlist_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 9 | 3 call to uncontracted function `_`; 3 indirect memory access is outside the resource projection |
| cmetrics | `cmt_encode_opentelemetry.c` | `cfl_variant_kvpair_to_otlp_kvpair` | 4 | `V5-retained-unknown-instruction` | 8 | 4 indirect memory access is outside the resource projection; 3 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_encode_opentelemetry.c` | `cfl_variant_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 14 | 8 unmodeled call, aggregate or indirect access in branch condition; 6 call to uncontracted function `_` |
| cmetrics | `cmt_encode_opentelemetry.c` | `otlp_any_value_destroy` | 4 | `V5-retained-unknown-instruction` | 12 | 8 unmodeled call, aggregate or indirect access in branch condition; 4 indirect memory access is outside the resource projection |
| cmetrics | `cmt_encode_opentelemetry.c` | `otlp_array_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_encode_opentelemetry.c` | `otlp_kvlist_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_encode_opentelemetry.c` | `otlp_kvpair_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 2 unmodeled call, aggregate or indirect access in branch condition |
| cmetrics | `cmt_encode_text.c` | `append_array_value` | 3 | `V5-retained-unknown-instruction` | 5 | 4 call to uncontracted function `_`; 1 unmodeled call, aggregate or indirect access in for-loop condition |
| cmetrics | `cmt_encode_text.c` | `append_kvlist_value` | 3 | `V5-retained-unknown-instruction` | 10 | 4 call to uncontracted function `_`; 4 indirect memory access is outside the resource projection |
| cmetrics | `cmt_encode_text.c` | `append_variant_value` | 3 | `V5-retained-unknown-instruction` | 23 | 8 indirect memory access is outside the resource projection; 8 unmodeled call, aggregate or indirect access in branch condition |
| ctraces | `ctr_decode_opentelemetry.c` | `convert_any_value` | 3 | `V5-retained-unknown-instruction` | 8 | 7 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in switch expression |
| ctraces | `ctr_decode_opentelemetry.c` | `convert_array_value` | 3 | `V5-retained-unknown-instruction` | 9 | 7 indirect memory access is outside the resource projection; 1 call to uncontracted function `_` |
| ctraces | `ctr_decode_opentelemetry.c` | `convert_kvlist_value` | 3 | `V5-retained-unknown-instruction` | 10 | 8 indirect memory access is outside the resource projection; 1 call to uncontracted function `_` |
| ctraces | `ctr_encode_msgpack.c` | `pack_array` | 3 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 1 call to uncontracted function `_` |
| ctraces | `ctr_encode_msgpack.c` | `pack_kvlist` | 3 | `V5-retained-unknown-instruction` | 9 | 5 indirect memory access is outside the resource projection; 3 call to uncontracted function `_` |
| ctraces | `ctr_encode_msgpack.c` | `pack_variant` | 3 | `V5-retained-unknown-instruction` | 8 | 8 indirect memory access is outside the resource projection |
| ctraces | `ctr_encode_opentelemetry.c` | `ctr_variant_array_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 5 | 2 call to uncontracted function `_`; 2 indirect memory access is outside the resource projection |
| ctraces | `ctr_encode_opentelemetry.c` | `ctr_variant_kvlist_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 9 | 3 call to uncontracted function `_`; 3 indirect memory access is outside the resource projection |
| ctraces | `ctr_encode_opentelemetry.c` | `ctr_variant_kvpair_to_otlp_kvpair` | 4 | `V5-retained-unknown-instruction` | 6 | 5 indirect memory access is outside the resource projection; 1 call to uncontracted function `_` |
| ctraces | `ctr_encode_opentelemetry.c` | `ctr_variant_to_otlp_any_value` | 4 | `V5-retained-unknown-instruction` | 14 | 8 unmodeled call, aggregate or indirect access in branch condition; 6 call to uncontracted function `_` |
| ctraces | `ctr_encode_opentelemetry.c` | `otlp_any_value_destroy` | 4 | `V5-retained-unknown-instruction` | 13 | 8 unmodeled call, aggregate or indirect access in branch condition; 5 indirect memory access is outside the resource projection |
| ctraces | `ctr_encode_opentelemetry.c` | `otlp_array_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| ctraces | `ctr_encode_opentelemetry.c` | `otlp_kvlist_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| ctraces | `ctr_encode_opentelemetry.c` | `otlp_kvpair_destroy` | 4 | `V5-retained-unknown-instruction` | 4 | 2 indirect memory access is outside the resource projection; 2 unmodeled call, aggregate or indirect access in branch condition |
| ctraces | `ctr_encode_text.c` | `format_array` | 1 | `V5-retained-unknown-instruction` | 19 | 6 call to uncontracted function `_`; 6 indirect memory access is outside the resource projection |
| ctraces | `ctr_encode_text.c` | `format_attributes` | 1 | `V5-retained-unknown-instruction` | 22 | 10 indirect memory access is outside the resource projection; 6 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `cJSON_Compare` | 1 | `V5-retained-unknown-instruction` | 19 | 10 indirect memory access is outside the resource projection; 7 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `cJSON_Delete` | 1 | `V5-retained-unknown-instruction` | 10 | 7 indirect memory access is outside the resource projection; 3 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `cJSON_Duplicate_rec` | 1 | `V5-retained-unknown-instruction` | 18 | 11 indirect memory access is outside the resource projection; 5 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `parse_array` | 3 | `V5-retained-unknown-instruction` | 25 | 13 indirect memory access is outside the resource projection; 6 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `parse_object` | 3 | `V5-retained-unknown-instruction` | 33 | 16 indirect memory access is outside the resource projection; 9 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `cJSON.c` | `parse_value` | 3 | `V5-retained-unknown-instruction` | 19 | 8 unmodeled call, aggregate or indirect access in branch condition; 4 indirect memory access is outside the resource projection |
| librdkafka | `cJSON.c` | `print_array` | 3 | `V5-retained-unknown-instruction` | 17 | 6 indirect memory access is outside the resource projection; 4 call to uncontracted function `_` |
| librdkafka | `cJSON.c` | `print_object` | 3 | `V5-retained-unknown-instruction` | 37 | 12 unrecognized assignment destination; 10 indirect memory access is outside the resource projection |
| librdkafka | `cJSON.c` | `print_value` | 3 | `V5-retained-unknown-instruction` | 15 | 7 call to uncontracted function `_`; 4 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `nanopb/pb_decode.c` | `decode_basic_field` | 9 | `V5-retained-unknown-instruction` | 18 | 17 unmodeled call, aggregate or indirect access in return expression; 1 unmodeled call, aggregate or indirect access in switch expression |
| librdkafka | `nanopb/pb_decode.c` | `decode_extension` | 9 | `V5-retained-unknown-instruction` | 5 | 3 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `nanopb/pb_decode.c` | `decode_field` | 9 | `V5-retained-unknown-instruction` | 5 | 4 unmodeled call, aggregate or indirect access in return expression; 1 unmodeled call, aggregate or indirect access in switch expression |
| librdkafka | `nanopb/pb_decode.c` | `decode_static_field` | 9 | `V5-retained-unknown-instruction` | 30 | 11 unmodeled call, aggregate or indirect access in branch condition; 8 indirect memory access is outside the resource projection |
| librdkafka | `nanopb/pb_decode.c` | `default_extension_decoder` | 9 | `V5-retained-unknown-instruction` | 5 | 2 unmodeled call, aggregate or indirect access in branch condition; 2 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `nanopb/pb_decode.c` | `pb_dec_submessage` | 9 | `V5-retained-unknown-instruction` | 11 | 7 unmodeled call, aggregate or indirect access in branch condition; 3 indirect memory access is outside the resource projection |
| librdkafka | `nanopb/pb_decode.c` | `pb_decode_inner` | 9 | `V5-retained-unknown-instruction` | 34 | 14 unmodeled call, aggregate or indirect access in branch condition; 8 indirect memory access is outside the resource projection |
| librdkafka | `nanopb/pb_decode.c` | `pb_field_set_to_default` | 9 | `V5-retained-unknown-instruction` | 14 | 6 unmodeled call, aggregate or indirect access in branch condition; 4 indirect memory access is outside the resource projection |
| librdkafka | `nanopb/pb_decode.c` | `pb_message_set_to_defaults` | 9 | `V5-retained-unknown-instruction` | 11 | 7 unmodeled call, aggregate or indirect access in branch condition; 1 braced initializer, expression, or declaration is not modeled |
| librdkafka | `nanopb/pb_decode.c` | `pb_read` | 1 | `V5-retained-unknown-instruction` | 11 | 5 unmodeled call, aggregate or indirect access in branch condition; 3 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `nanopb/pb_encode.c` | `default_extension_encoder` | 8 | `V5-retained-unknown-instruction` | 3 | 2 unmodeled call, aggregate or indirect access in return expression; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `nanopb/pb_encode.c` | `encode_array` | 8 | `V5-retained-unknown-instruction` | 31 | 14 unmodeled call, aggregate or indirect access in branch condition; 10 indirect memory access is outside the resource projection |
| librdkafka | `nanopb/pb_encode.c` | `encode_basic_field` | 8 | `V5-retained-unknown-instruction` | 11 | 8 unmodeled call, aggregate or indirect access in return expression; 2 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `nanopb/pb_encode.c` | `encode_extension_field` | 8 | `V5-retained-unknown-instruction` | 4 | 3 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `nanopb/pb_encode.c` | `encode_field` | 8 | `V5-retained-unknown-instruction` | 15 | 11 unmodeled call, aggregate or indirect access in branch condition; 4 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `nanopb/pb_encode.c` | `pb_check_proto3_default_value` | 1 | `V5-retained-unknown-instruction` | 21 | 9 unmodeled call, aggregate or indirect access in return expression; 6 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `nanopb/pb_encode.c` | `pb_enc_submessage` | 8 | `V5-retained-unknown-instruction` | 7 | 4 unmodeled call, aggregate or indirect access in branch condition; 2 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `nanopb/pb_encode.c` | `pb_encode` | 8 | `V5-retained-unknown-instruction` | 5 | 4 unmodeled call, aggregate or indirect access in branch condition; 1 unmodeled call, aggregate or indirect access in do-while condition |
| librdkafka | `nanopb/pb_encode.c` | `pb_encode_submessage` | 8 | `V5-retained-unknown-instruction` | 19 | 10 indirect memory access is outside the resource projection; 5 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdavl.c` | `rd_avl_balance_node` | 2 | `V5-retained-unknown-instruction` | 13 | 7 indirect memory access is outside the resource projection; 4 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdavl.c` | `rd_avl_find_node` | 1 | `V5-retained-unknown-instruction` | 3 | 2 unmodeled call, aggregate or indirect access in return expression; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdavl.c` | `rd_avl_insert_node` | 1 | `V5-retained-unknown-instruction` | 6 | 4 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdavl.c` | `rd_avl_move` | 1 | `V5-retained-unknown-instruction` | 2 | 1 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in return expression |
| librdkafka | `rdavl.c` | `rd_avl_remove_elm0` | 1 | `V5-retained-unknown-instruction` | 4 | 2 unmodeled call, aggregate or indirect access in return expression; 1 indirect memory access is outside the resource projection |
| librdkafka | `rdavl.c` | `rd_avl_rotate` | 2 | `V5-retained-unknown-instruction` | 5 | 3 indirect memory access is outside the resource projection; 1 braced initializer, expression, or declaration is not modeled |
| librdkafka | `rdkafka_buf.c` | `rd_kafka_buf_destroy_final` | 1 | `V5-retained-unknown-instruction` | 27 | 13 indirect memory access is outside the resource projection; 11 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_anyconf_clear` | 3 | `V5-retained-unknown-instruction` | 22 | 12 indirect memory access is outside the resource projection; 8 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_anyconf_copy` | 2 | `V5-retained-unknown-instruction` | 26 | 9 indirect memory access is outside the resource projection; 8 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_anyconf_destroy` | 3 | `V5-retained-unknown-instruction` | 3 | 1 call to uncontracted function `_`; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_anyconf_get` | 1 | `V5-retained-unknown-instruction` | 5 | 3 unmodeled call, aggregate or indirect access in branch condition; 1 unmodeled call, aggregate or indirect access in for-loop condition |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_anyconf_set` | 1 | `V5-retained-unknown-instruction` | 9 | 4 unmodeled call, aggregate or indirect access in branch condition; 2 call to uncontracted function `_` |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_topic_conf_destroy` | 3 | `V5-body-not-clean` | 1 | 1 unresolved resource identity |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_topic_conf_dup` | 2 | `V5-retained-unknown-instruction` | 1 | 1 call to uncontracted function `_` |
| librdkafka | `rdkafka_idempotence.c` | `rd_kafka_idemp_set_state` | 1 | `V5-retained-unknown-instruction` | 10 | 5 unmodeled call, aggregate or indirect access in branch condition; 4 indirect memory access is outside the resource projection |
| librdkafka | `rdkafka_op.c` | `rd_kafka_op_destroy` | 1 | `V5-retained-unknown-instruction` | 124 | 59 indirect memory access is outside the resource projection; 52 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_apply` | 1 | `V3-parameters-unreadable` | 0 |  |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_consumer_propagate` | 1 | `V5-retained-unknown-instruction` | 5 | 3 indirect memory access is outside the resource projection; 1 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_dump` | 1 | `V5-retained-unknown-instruction` | 15 | 9 indirect memory access is outside the resource projection; 4 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_move_cnt` | 1 | `V5-retained-unknown-instruction` | 44 | 28 indirect memory access is outside the resource projection; 10 unmodeled call, aggregate or indirect access in branch condition |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_pop_serve0` | 1 | `V5-retained-unknown-instruction` | 23 | 13 indirect memory access is outside the resource projection; 5 call to uncontracted function `_` |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_purge0` | 1 | `V5-retained-unknown-instruction` | 18 | 11 indirect memory access is outside the resource projection; 4 call to uncontracted function `_` |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_purge_toppar_version` | 1 | `V5-retained-unknown-instruction` | 23 | 13 indirect memory access is outside the resource projection; 3 call to uncontracted function `_` |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_serve0` | 1 | `V5-retained-unknown-instruction` | 25 | 13 call to uncontracted function `_`; 6 indirect memory access is outside the resource projection |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_serve_rkmessages` | 1 | `V5-retained-unknown-instruction` | 50 | 22 indirect memory access is outside the resource projection; 13 call to uncontracted function `_` |
| librdkafka | `rdkafka_queue.c` | `rd_kafka_q_serve_share_rkmessages` | 1 | `V5-retained-unknown-instruction` | 31 | 15 indirect memory access is outside the resource projection; 8 call to uncontracted function `_` |
| librdkafka | `rdkafka_transport.c` | `rd_kafka_transport_framed_recv` | 1 | `V5-retained-unknown-instruction` | 31 | 15 indirect memory access is outside the resource projection; 8 unmodeled call, aggregate or indirect access in branch condition |
| monkey-libevent | `mk_core/mk_rconf.c` | `mk_rconf_read` | 2 | `V5-retained-unknown-instruction` | 66 | 35 call to uncontracted function `_`; 18 unmodeled call, aggregate or indirect access in branch condition |
| monkey-libevent | `mk_core/mk_rconf.c` | `mk_rconf_read_glob` | 2 | `V5-retained-unknown-instruction` | 10 | 6 call to uncontracted function `_`; 2 indirect memory access is outside the resource projection |
| monkey-libevent | `mk_core/mk_string.c` | `digits10` | 1 | `V5-retained-unknown-instruction` | 1 | 1 unmodeled call, aggregate or indirect access in return expression |
| monkey-libevent | `mk_server/mk_http.c` | `mk_http_directory_redirect_check` | 5 | `V5-retained-unknown-instruction` | 21 | 10 indirect memory access is outside the resource projection; 5 unmodeled call, aggregate or indirect access in branch condition |
| monkey-libevent | `mk_server/mk_http.c` | `mk_http_error` | 5 | `V5-retained-unknown-instruction` | 46 | 30 indirect memory access is outside the resource projection; 7 call to uncontracted function `_` |
| monkey-libevent | `mk_server/mk_http.c` | `mk_http_init` | 5 | `V5-retained-unknown-instruction` | 126 | 60 indirect memory access is outside the resource projection; 33 unmodeled call, aggregate or indirect access in branch condition |
| monkey-libevent | `mk_server/mk_http.c` | `mk_http_request_end` | 5 | `V5-retained-unknown-instruction` | 21 | 12 indirect memory access is outside the resource projection; 5 call to uncontracted function `_` |
| monkey-libevent | `mk_server/mk_http.c` | `mk_http_request_prepare` | 5 | `V5-retained-unknown-instruction` | 35 | 21 indirect memory access is outside the resource projection; 8 unmodeled call, aggregate or indirect access in branch condition |
