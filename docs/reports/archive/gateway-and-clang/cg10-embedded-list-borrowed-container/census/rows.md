# The 23 embedded-list sole rows (round 3's tip)

`scripts/rows.py` over round 3's tip premises (`census-tip2/premises.jsonl.gz`) and preprocessed units, `refine.py`'s class `embedded-list-address` alone; `scripts/rows_table.py`. Contract: the declaration in force (`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, `quirin-provider-cfl/contracts/cfl/sds_lifetime.qv`).

| # | file | function | leaves | helper | embedded list | helper contract |
|---:|---|---|---:|---|---|---|
| 1 | `cmt_encode_msgpack.c` | `pack_static_labels` | 1 | `cfl_list_size` | `&cmt->static_labels->list` | borrowed(head), return none; the call is nested in mpack_start_array's actual |
| 2 | `rdkafka_admin.c` | `rd_kafka_ConfigResource_add_ConfigEntry` | 1 | `rd_list_add` | `&config->config` | borrowed(rl), element_added(elem, rl), return borrowed_from(rl) |
| 3 | `rdkafka_assignor.c` | `rd_kafka_assignors_term` | 1 | `rd_list_destroy` | `&rk->rk_conf.partition_assignors` | borrowed(rl), registry invoked over the elements, elements dropped, unregistered, released_if_constructed(rl) |
| 4 | `rdkafka_broker.c` | `rd_kafka_brokers_wait_state_change_async` | 1 | `rd_list_add` | `&rk->rk_broker_state_change_waiters` | borrowed(rl), element_added(elem, rl), return borrowed_from(rl) |
| 5 | `rdkafka_interceptor.c` | `rd_kafka_conf_interceptor_add_on_conf_destroy` | 1 | `rd_kafka_interceptor_method_add` | `&conf->interceptors.on_conf_destroy` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 6 | `rdkafka_interceptor.c` | `rd_kafka_conf_interceptor_add_on_conf_dup` | 1 | `rd_kafka_interceptor_method_add` | `&conf->interceptors.on_conf_dup` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 7 | `rdkafka_interceptor.c` | `rd_kafka_conf_interceptor_add_on_conf_set` | 1 | `rd_kafka_interceptor_method_add` | `&conf->interceptors.on_conf_set` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 8 | `rdkafka_interceptor.c` | `rd_kafka_conf_interceptor_add_on_new` | 1 | `rd_kafka_interceptor_method_add` | `&conf->interceptors.on_new` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 9 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_acknowledgement` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_acknowledgement` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 10 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_broker_state_change` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_broker_state_change` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 11 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_commit` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_commit` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 12 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_consume` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_consume` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 13 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_destroy` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_destroy` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 14 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_request_sent` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_request_sent` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 15 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_response_received` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_response_received` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 16 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_send` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_send` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 17 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_thread_exit` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_thread_exit` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 18 | `rdkafka_interceptor.c` | `rd_kafka_interceptor_add_on_thread_start` | 1 | `rd_kafka_interceptor_method_add` | `&rk->rk_conf.interceptors.on_thread_start` | all borrowed, retained_by(func, list), retained_by(ic_opaque, list), return none |
| 19 | `rdkafka_interceptor.c` | `rd_kafka_interceptors_destroy` | 15 | `rd_list_destroy` | `&conf->interceptors.config`, `&conf->interceptors.on_acknowledgement`, `... (15 lists)` | borrowed(rl), registry invoked over the elements, elements dropped, unregistered, released_if_constructed(rl) |
| 20 | `rdkafka_metadata_cache.c` | `rd_kafka_metadata_cache_wait_state_change_async` | 1 | `rd_list_add` | `&rk->rk_metadata_cache.rkmc_observers` | borrowed(rl), element_added(elem, rl), return borrowed_from(rl) |
| 21 | `rdkafka_mock.c` | `rd_kafka_mock_clear_requests` | 1 | `rd_list_clear` | `&mcluster->request_list` | borrowed(rl), registry invoked over the elements, elements dropped |
| 22 | `rdkafka_mock.c` | `rd_kafka_mock_start_request_tracking` | 1 | `rd_list_clear` | `&mcluster->request_list` | borrowed(rl), registry invoked over the elements, elements dropped |
| 23 | `rdkafka_mock.c` | `rd_kafka_mock_stop_request_tracking` | 1 | `rd_list_clear` | `&mcluster->request_list` | borrowed(rl), registry invoked over the elements, elements dropped |
