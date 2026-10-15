# LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: the plugin premise on every dependent declaration

Card LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED (wave 48, round 28, card-high).
Contracts and records only: no rule, analyser or Lean change. Base: local
`main` `d708641bc`. The census and the frozen [prediction](PREDICTION.md)
were committed (`9197085f1`) before any contract changed.

## 1. Authority

* **User decision U2 (2026-10-14).** "List it in every dependent verdict":
  the premise `P-RK-NO-PLUGIN-LIBRARY` (no `plugin.library.paths` configured),
  written `rk_no_plugin_library` by OP-APP-RK-CALLBACKS-VIA-POLL-CB
  ([report](../op-app-rk-callbacks/README.md)).
* **Hard invariant 2.** A verdict carries its assumptions. A row lists a
  declaration's premises through the contract sidecar's per-declaration
  record (rules 13f, 13h).
* **The defect** (ledger entry of the same name). Any librdkafka body that
  logs may call `rk_conf.log_cb` (`rdkafka.c:280-305`). That callback's run
  set is closed only under the premise. 45 declarations named it; 73 other
  `client.qv` declarations, 7 in four other modules and the 86 null views did
  not.

## 2. Census

[PREDICTION.md](PREDICTION.md) sections 1–2 give the method and the
decisions. [`scripts/census.sh`](scripts/census.sh) preprocesses librdkafka's
94 C units with their own recorded commands at both revisions. It then walks
an over-approximating call graph ([`reach.py`](scripts/reach.py),
[census](artifacts/census.json)). Every macro is expanded, and an unresolved
indirect call counts as reaching. All 253 librdkafka declarations have
identical definitions and results at `66910c10` and `046accf6`.

| Module | Without the premise | Given it | Not given |
| --- | ---: | ---: | ---: |
| `client.qv` | 73 | **34** | 39 |
| `refcount.qv` | 3 | 0 | 3 |
| `typed_queue.qv` | 2 | 0 | 2 |
| `queue_destructor.qv` | 1 | 0 | 1, reaches `log_cb` (section 4) |
| `topic_partition.qv` | 1 | 0 | 1 |

### Census

Every declaration without the premise before the card, with its ground
([`scripts/table.py`](scripts/table.py); `Class` is the census's, `Premise`
the decision):

| Module | Declaration | Definition | Class | Premise | Ground |
| --- | --- | --- | --- | --- | --- |
| `client.qv` | `rd_kafka_error_new_v` | `rdkafka_error.c:51` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_error_new` | `rdkafka_error.c:112` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_err_action` | `rdkafka_request.c:84` | rk-conf | yes | `rd_kafka_err_action → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_oauthbearer_set_token_failure` | `rdkafka.c:371` | rk-conf | yes | `rd_kafka_oauthbearer_set_token_failure → rd_kafka_oauthbearer_set_token_failure0 → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_toppars_pause_resume` | `rdkafka_partition.c:2591` | rk-conf | yes | `rd_kafka_toppars_pause_resume → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_broker_state_change`, `on_conf_destroy`, `on_conf_set`, `on_response_received` |
| `client.qv` | `rd_kafka_AclBinding_new0` | `rdkafka_admin.c:5210` | none | no | 6 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_ConsumerGroupDescription_new` | `rdkafka_admin.c:7997` | none | no | 34 function(s) reached, no rk_conf or interceptor call; `copy_cb(` resolved to `rd_kafka_MemberDescription_list_copy`: rd_list_copy_to's copier is the body's only one, rd_kafka_MemberDescription_list_copy (rdkafka_admin.c:8007-8008) |
| `client.qv` | `rd_kafka_admin_add_config0` | `rdkafka_admin.c:2025` | none | no | 8 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_anyconf_dump` | `rdkafka_conf.c:3446` | none | no | 8 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_confval_set_type` | `rdkafka_conf.c:3767` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_interceptor_method_add` | `rdkafka_interceptor.c:140` | none | no | 10 function(s) reached, no rk_conf or interceptor call; `cmp(` resolved to `rd_kafka_interceptor_method_cmp`: rd_list_find's comparator is the body's only one, rd_kafka_interceptor_method_cmp (rdkafka_interceptor.c:151); `external:bsearch` resolved to `rd_kafka_interceptor_method_cmp`: the same comparator, through bsearch |
| `client.qv` | `rd_flags2str` | `rdstring.c:225` | none | no | 1 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_admin_result_ret_topics` | `rdkafka_admin.c:518` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_admin_result_ret_resources` | `rdkafka_admin.c:534` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_admin_result_ret_groups` | `rdkafka_admin.c:578` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_topic_partition_get_private` | `rdkafka_partition.h:869` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_slice_reader0` | `rdbuf.c:842` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_toppar_keep0` | `rdkafka_partition.h:533` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_q_keep` | `rdkafka_queue.h:157` | none | no | 1 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_q_keep_nolock` | `rdkafka_queue.h:165` | none | no | 1 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_topic_keep` | `rdkafka_topic.h:229` | none | no | 5 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_toppar_destroy0` | `rdkafka_partition.h:548` | rk-conf | yes | `rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_destroy0` | `rdkafka_queue.h:220` | rk-conf | yes | `rd_kafka_q_destroy0 → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_destroy_owner` | `rdkafka_queue.h:253` | rk-conf | yes | `rd_kafka_q_destroy_owner → rd_kafka_q_destroy0 → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_topic_destroy0` | `rdkafka_topic.h:247` | rk-conf | yes | `rd_kafka_topic_destroy0 → rd_kafka_topic_destroy_final → rd_kafka_anyconf_destroy → rd_kafka_interceptors_on_conf_destroy` → `on_conf_destroy`; also `log_cb`, `on_acknowledgement`, `on_response_received` |
| `client.qv` | `rd_kafka_enq_once_trigger` | `rdkafka_queue.h:1113` | rk-conf | yes | `rd_kafka_enq_once_trigger → rd_kafka_replyq_destroy → rd_kafka_q_destroy0 → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_queue_new0` | `rdkafka_queue.c:988` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_topic_partition_list_destroy` | `rdkafka_partition.c:3118` | rk-conf | yes | `rd_kafka_topic_partition_list_destroy → rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_topic_partition_destroy0` | `rdkafka_partition.c:2971` | rk-conf | yes | `rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_AlterConsumerGroupOffsets_destroy` | `rdkafka_admin.c:6810` | rk-conf | yes | `rd_kafka_AlterConsumerGroupOffsets_destroy → rd_kafka_topic_partition_list_destroy → rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_DeleteConsumerGroupOffsets_destroy` | `rdkafka_admin.c:5000` | rk-conf | yes | `rd_kafka_DeleteConsumerGroupOffsets_destroy → rd_kafka_topic_partition_list_destroy → rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_ListConsumerGroupOffsets_destroy` | `rdkafka_admin.c:7048` | rk-conf | yes | `rd_kafka_ListConsumerGroupOffsets_destroy → rd_kafka_topic_partition_list_destroy → rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_topic_partition_result_destroy` | `rdkafka_aux.c:371` | rk-conf | yes | `rd_kafka_topic_partition_result_destroy → rd_kafka_topic_partition_destroy → rd_kafka_topic_partition_destroy0 → rd_kafka_topic_partition_private_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_toppar_get0` | `rdkafka_partition.c:422` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_toppar_desired_get` | `rdkafka_partition.c:629` | none | no | 4 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_toppar_new0` | `rdkafka_partition.c:222` | rk-conf | yes | `rd_kafka_toppar_new0 → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_toppar_desired_add` | `rdkafka_partition.c:712` | rk-conf | yes | `rd_kafka_toppar_desired_add → rd_kafka_toppar_desired_add0 → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_toppar_get2` | `rdkafka_partition.c:450` | rk-conf | yes | `rd_kafka_toppar_get2 → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_broker_state_change`, `on_conf_destroy`, `on_conf_set`, `on_response_received` |
| `client.qv` | `rd_kafka_toppar_get_avail` | `rdkafka_partition.c:552` | rk-conf | yes | `rd_kafka_toppar_get_avail → rd_kafka_crash → rd_kafka_dump0 → rd_kafka_q_len → rd_kafka_q_destroy0 → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb` (only through `rd_kafka_crash`'s dump); also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_new0` | `rdkafka_queue.c:118` | none | no | 4 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `ut_create_mock_toppar` | `rdunittest_msgset_errors.c:122` | rk-conf | yes | `ut_create_mock_toppar → rd_kafka_toppar_new0 → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_broker_state_change`, `on_conf_destroy`, `on_conf_set`, `on_response_received` |
| `client.qv` | `ut_create_mock_queue` | `rdunittest_fetcher.c:95` | none | no | 4 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `ut_destroy_mock_queue` | `rdunittest_fetcher.c:102` | rk-conf | yes | `ut_destroy_mock_queue → rd_kafka_q_destroy_owner → rd_kafka_q_destroy0 → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_wait_result` | `rdkafka_queue.c:1199` | rk-conf | yes | `rd_kafka_q_wait_result → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_purge0` | `rdkafka_queue.c:205` | rk-conf | yes | `rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_toppar_needs_query` | `rdkafka_topic.c:1859` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_toppar_fetch_backoff` | `rdkafka_fetcher.c:62` | rk-conf | yes | `rd_kafka_toppar_fetch_backoff → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_list_init` | `rdlist.c:53` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_list_new` | `rdlist.c:84` | none | no | 6 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_list_add` | `rdlist.c:143` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_list_destroy` | `rdlist.c:316` | none | no | 3 function(s) reached, no rk_conf or interceptor call; `rl->rl_free_cb(` resolved to none here: the declaration states `resource_callback_registry_invoked`: the call is decided over the registration in force at each call site, whose candidates' own declarations are consulted and publish their premises (rules 13u, 13w) |
| `client.qv` | `rd_list_clear` | `rdlist.c:311` | none | no | 3 function(s) reached, no rk_conf or interceptor call; `rl->rl_free_cb(` resolved to none here: the declaration states `resource_callback_registry_invoked`: the call is decided over the registration in force at each call site, whose candidates' own declarations are consulted and publish their premises (rules 13u, 13w) |
| `client.qv` | `pb_write` | `nanopb/pb_encode.c:84` | none | no | 1 function(s) reached, no rk_conf or interceptor call; `(!stream->callback(` resolved to none here: the declaration states `resource_callback_registry_invoked`: the call is decided over the registration in force at each call site, whose candidates' own declarations are consulted and publish their premises (rules 13u, 13w) |
| `client.qv` | `pb_encode_varint` | `nanopb/pb_encode.c:607` | none | no | 3 function(s) reached, no rk_conf or interceptor call; `(!stream->callback(` resolved to none here: the declaration states `resource_callback_registry_invoked`: the call is decided over the registration in force at each call site, whose candidates' own declarations are consulted and publish their premises (rules 13u, 13w) |
| `client.qv` | `rd_kafka_AclBinding_destroy` | `rdkafka_admin.c:5393` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_ConfigResource_destroy` | `rdkafka_admin.c:2976` | none | no | 7 function(s) reached, no rk_conf or interceptor call; `rl->rl_free_cb(` resolved to `rd_kafka_ConfigEntry_free`: config->config is initialised only by rd_kafka_ConfigResource_new (rdkafka_admin.c:2971) and each entry's synonyms only by rd_kafka_ConfigEntry_new / _copy (:2782, :2810, rd_list_init_copy keeps the source's), each with rd_kafka_ConfigEntry_free |
| `client.qv` | `rd_kafka_NewPartitions_destroy` | `rdkafka_admin.c:2516` | none | no | 5 function(s) reached, no rk_conf or interceptor call; `rl->rl_free_cb(` resolved to `rd_list_destroy_free`: newps->replicas is initialised by rd_kafka_NewPartitions_new (rdkafka_admin.c:2483) or copied from one (:2509), with rd_list_destroy_free; its elements are rd_list_new(0, NULL) int32 lists (:2563) or rd_list_copy_preallocated copies (rdlist.c:539-541), with no free callback |
| `client.qv` | `rd_kafka_NewTopic_destroy` | `rdkafka_admin.c:1950` | none | no | 8 function(s) reached, no rk_conf or interceptor call; `rl->rl_free_cb(` resolved to `rd_list_destroy_free`, `rd_kafka_ConfigEntry_free`: new_topic->replicas is initialised with rd_list_destroy_free (rdkafka_admin.c:1905, copied at :1939) over rd_list_new(0, NULL) int32 lists (:2010) and new_topic->config with rd_kafka_ConfigEntry_free (:1911, copied at :1943), whose synonyms use the same (:2782, :2810) |
| `client.qv` | `rd_kafka_UserScramCredentialAlteration_destroy` | `rdkafka_admin.c:6071` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `unit_test_telemetry` | `rdkafka_telemetry_decode.c:535` | rk-conf | yes | `unit_test_telemetry → rd_kafka_telemetry_encode_metrics → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_conf_set`, `on_response_received` |
| `client.qv` | `rd_kafka_q_serve0` | `rdkafka_queue.c:564` | rk-conf | yes | `rd_kafka_q_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_serve` | `rdkafka_queue.c:657` | rk-conf | yes | `rd_kafka_q_serve → rd_kafka_q_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_serve_maybe_consume` | `rdkafka_queue.c:672` | rk-conf | yes | `rd_kafka_q_serve_maybe_consume → rd_kafka_q_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_pop_serve0` | `rdkafka_queue.c:404` | rk-conf | yes | `rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_pop_serve` | `rdkafka_queue.c:512` | rk-conf | yes | `rd_kafka_q_pop_serve → rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_q_pop_serve_maybe_consume` | `rdkafka_queue.c:528` | rk-conf | yes | `rd_kafka_q_pop_serve_maybe_consume → rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_op_destroy` | `rdkafka_op.c:345` | rk-conf | yes | `rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_op_new0` | `rdkafka_op.c:224` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_op_new_cb` | `rdkafka_op.c:787` | none | no | 3 function(s) reached, no rk_conf or interceptor call |
| `client.qv` | `rd_kafka_op_req0` | `rdkafka_op.c:853` | rk-conf | yes | `rd_kafka_op_req0 → rd_kafka_q_pop → rd_kafka_q_pop_serve → rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_op_req` | `rdkafka_op.c:875` | rk-conf | yes | `rd_kafka_op_req → rd_kafka_op_req0 → rd_kafka_q_pop → rd_kafka_q_pop_serve → rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_op_req2` | `rdkafka_op.c:892` | rk-conf | yes | `rd_kafka_op_req2 → rd_kafka_op_req → rd_kafka_op_req0 → rd_kafka_q_pop → rd_kafka_q_pop_serve → rd_kafka_q_pop_serve0 → rd_kafka_app_polled → rd_kafka_cgrp_consumer_expedite_next_heartbeat → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `client.qv` | `rd_kafka_log_print` | `rdkafka.c:382` | none | no | 1 function(s) reached, no rk_conf or interceptor call |
| `refcount.qv` | `rd_refcnt_add0` | `—` | undefined | no | no C definition: a macro (`rd_refcnt_add0` is `rd_atomic32_add`, rd.h:351) or the model's abstract destructor |
| `refcount.qv` | `rd_refcnt_sub0` | `rd.h:354` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `refcount.qv` | `rd_refcnt_destroy_final` | `—` | undefined | no | no C definition: a macro (`rd_refcnt_add0` is `rd_atomic32_add`, rd.h:351) or the model's abstract destructor |
| `typed_queue.qv` | `rd_kafka_q_enq0` | `rdkafka_queue.h:399` | none | no | 2 function(s) reached, no rk_conf or interceptor call |
| `typed_queue.qv` | `rd_kafka_q_deq0` | `rdkafka_queue.h:514` | none | no | 1 function(s) reached, no rk_conf or interceptor call |
| `queue_destructor.qv` | `rd_kafka_q_destroy_final` | `rdkafka_queue.c:57` | rk-conf | no (section 4) | `rd_kafka_q_destroy_final → rd_kafka_q_purge0 → rd_kafka_op_destroy → rd_kafka_toppar_destroy0 → rd_kafka_toppar_destroy_final → rd_kafka_log0 → rd_kafka_log_buf` → `conf->log_cb`; also `on_acknowledgement`, `on_conf_destroy`, `on_response_received` |
| `topic_partition.qv` | `rd_kafka_topic_partition_list_sort` | `rdkafka_partition.c:3667` | none | no | 2 function(s) reached, no rk_conf or interceptor call; `external:qsort` resolved to none here: the declaration states `resource_callback_invoked(cmp, _, _, opaque)`: the comparator is decided at each call site over its own candidates' declarations (rule 13u) |

## 3. Contracts

[`scripts/write_contracts.py`](scripts/write_contracts.py) adds the clause
OP-APP-RK wrote on its own declarations, byte for byte, after the selector
of each of the 34:
`requires resource_premise(rk_no_plugin_library, "P-RK-NO-PLUGIN-LIBRARY …")`.
It also adds one comment line naming the census path. `--check` passes: 34
declarations name it exactly once.

* **Pins** ([`scripts/pin.sh`](scripts/pin.sh), logs `private/lpp/pin-a1/`).
  The 34 were re-pinned at both revisions. The crate's checks are 0 at both
  and `--verify-contracts` is 0. The whole-tree check is 1 at both, only for
  libevent's tip-mode `mk_vhost_set`, by design.
* **Sync.** `providers/contracts.py sync` and `check` are 0 (`contracts.json`'s
  `client.qv` digest).
* **Corpus pins.** In `compiler/tests/qcore_contract_corpus.rs`
  ([`scripts/edit_pins.py`](scripts/edit_pins.py)), the 34 `EXPECTED_CONTRACTS`
  rows each gain one clause. The resource-v1 and retaining declaration counts
  (455, 78) do not move. `qcore_contract_corpus` passes (4 tests, log
  `private/lpp/corpus-test.log`).

## 4. Not done: decisions for the user

1. **The 86 null views cannot carry a premise.** They are bounds-contract-v1
   views. The bounds clause parser rejects every clause outside its vocabulary
   (`unsupported bounds clause`, `crates/qcore-contract/src/bounds_contract.rs:960`).
   A null-deref view beside a resource view may state only the selector and
   the null clauses (`check_null_deref_only`,
   `crates/qcore-contract/src/contract_views.rs:874-905`). A null row's
   dependency fields name only `null_premises` (rule 14n). Per the brief, no
   syntax was invented. The census over the views' bodies is in the
   [census](artifacts/census.json): 75 reach an rk_conf or interceptor call,
   2 only an indirect call (the SASL provider's method table), and 9 neither.
   The options are:
   * a premise clause in the bounds vocabulary (new surface syntax);
   * a premise attached outside the view;
   * a ruling that a `quant null-deref` fact does not depend on which callback
     runs. The member declarations (`nonnull_member*`) are the ones a callback
     outside the run could falsify by a store.
2. **`queue_destructor.qv`'s `rd_kafka_q_destroy_final`** reaches `log_cb`.
   The module is a Quirin model contract (`resource type KafkaQueueOwner`),
   outside the provider's `resource_view` (`providers/contract-load-path.json`).
   No verdict consults it, and its vocabulary has no premise clause.

The card carried on with the 34 custody declarations, which do not depend on
either decision.

## 5. Measurement

BEFORE: binary `bin/before` (`8c8e7e39…`) from `d708641bc`, on the base
contracts. AFTER: binary `a1` (`f79d9846…`) from `7cf9f2d5b` (the contracts
commit), on the card's contracts. Both read the contracts at run time.
Scripts: [`before.sh`](scripts/before.sh), [`after.sh`](scripts/after.sh),
[`measure.sh`](scripts/measure.sh). The null property ran on every route.
Summaries: [before](artifacts/summary-before.json),
[both](artifacts/summary.json).

| Route | Custody before → after | Report | Null rows (`sha256`) | Null property | Wall before → after |
| --- | --- | --- | --- | --- | ---: |
| corpus | 936 / 9 / 3,889 → same | `71387bd3…` → `82214412…` | `5e854d2a…`, identical | 1,113 / 1,435 / 2,355 / 18 Unsafe | 783 → 757 s |
| system | 93 / 4 / 351 → same | `2aac1fad…` → `50d2bcaa…` | `23316f8b…`, identical | 84 / 125 / 245 | 95 → 82 s |
| slice | 52 / 1 / 347 / 4 Pending → same | `e4b97054…` → `47e5f874…` | `ed8de3d2…`, identical | 71 / 103 / 232 | 49 → 40 s |

`run.json`'s `binary_sha256` is the measured binary on both corpus runs. Wall
times were measured inside the shared lock, with three cards running; they are
not a performance claim.

**Verdicts.** No verdict changed and none was corrected. Each report
was compared with every `contract_dependencies` and `contract_body_evidence`
field removed, and every row's status and leaves are identical on all three
routes. The row `contract_dependencies` fields are identical too: they name
declarations, and the premise belongs to a declaration. The report digests move
for one reason each, by kind ([`report_diff.py`](scripts/report_diff.py)):

* **corpus:** 328 `contract_body_evidence.clauses.F.resource_premise(rk_no_plugin_library)`
  entries: the new clause, uncovered, in the clause evidence of the units
  that examined a premised declaration's body (64 units for each of the five
  header-inline `rd_kafka_enq_once_trigger`, `rd_kafka_q_destroy0`,
  `rd_kafka_q_destroy_owner`, `rd_kafka_topic_destroy0` and
  `rd_kafka_toppar_destroy0`, one unit for each of eight more)
  (`artifacts/report-diff-corpus.txt`);
* **system and slice:** only the contract-source identities
  (`resource_contract_sources`, `inventory_sha256`) of each unit's compile
  configuration, since `client.qv`'s text changed.

The sidecar's dependency counts and Safe roll-up are identical on every route
(corpus: 428 declarations consulted, 63 validated, 45 partly validated, 319
declared assumptions, 1 held; Safe rows 397 on no declaration, 108 only on
validated declarations, 25 on a partly-validated one, 382 on a declared
assumption, 24 only on a null premise).

**Rows that now list the premise**
([`scripts/premise_rows.py`](scripts/premise_rows.py);
[before](artifacts/rows-before.json), [after](artifacts/rows-after.json)).

On the corpus, 368 rows rest on at least one of the 34.
**Every one of them now lists `rk_no_plugin_library`** (17 Safe, 351 Unknown;
no Pending). Before the card, 35 did, through another declaration. So **16 Safe
rows and 317 Unknown rows gain the premise**, and no Safe or Pending row
resting on the 34 lacks it. All 34 declarations' sidecar records carry the
premise. No row on the system route or the slice rests on the 34: neither
analyses a librdkafka unit. The Safe rows, with the declarations that publish
the premise to them:

| Route | Unit | Function | Status | Rests on | Premise through |
| --- | --- | --- | --- | --- | --- |
| corpus | `rdkafka_admin.c` | `rd_kafka_AlterConsumerGroupOffsets_free` | Safe | `rd_kafka_AlterConsumerGroupOffsets_destroy` | `rd_kafka_AlterConsumerGroupOffsets_destroy` |
| corpus | `rdkafka_admin.c` | `rd_kafka_DeleteConsumerGroupOffsets_free` | Safe | `rd_kafka_DeleteConsumerGroupOffsets_destroy` | `rd_kafka_DeleteConsumerGroupOffsets_destroy` |
| corpus | `rdkafka_admin.c` | `rd_kafka_ListConsumerGroupOffsets_free` | Safe | `rd_kafka_ListConsumerGroupOffsets_destroy` | `rd_kafka_ListConsumerGroupOffsets_destroy` |
| corpus | `rdkafka_aux.c` | `rd_kafka_topic_partition_result_free` | Safe | `rd_kafka_topic_partition_result_destroy` | `rd_kafka_topic_partition_result_destroy` |
| corpus | `rdkafka_partition.c` | `rd_kafka_topic_partition_destroy` | Safe | `rd_kafka_topic_partition_destroy0` | `rd_kafka_topic_partition_destroy0` |
| corpus | `rdkafka_partition.c` | `rd_kafka_topic_partition_destroy_free` | Safe | `rd_kafka_topic_partition_destroy0` | `rd_kafka_topic_partition_destroy0` |
| corpus | `rdkafka_partition.c` | `rd_kafka_topic_partition_list_destroy_free` | Safe | `rd_kafka_topic_partition_list_destroy` | `rd_kafka_topic_partition_list_destroy` |
| corpus | `rdkafka_partition.c` | `rd_kafka_toppar_purge_and_disable_queues` | Safe | `rd_kafka_q_purge0` | `rd_kafka_q_purge0` |
| corpus | `rdkafka_request.c` | `rd_kafka_handle_OffsetCommit_error` | Safe | `rd_kafka_err_action` | `rd_kafka_err_action` |
| corpus | `rdkafka_request.c` | `rd_kafka_handle_OffsetFetch_err_action` | Safe | `rd_kafka_err_action` | `rd_kafka_err_action` |
| corpus | `rdkafka_sasl.c` | `rd_kafka_share_oauthbearer_set_token_failure` | Safe | `rd_kafka_oauthbearer_set_token_failure` | `rd_kafka_oauthbearer_set_token_failure` |
| corpus | `rdkafka_sasl_oauthbearer_oidc.c` | `rd_kafka_oidc_token_metadata_aws_iam_refresh_cb` | Safe | `rd_kafka_oauthbearer_set_token_failure` | `rd_kafka_oauthbearer_set_token_failure`, `rd_kafka_oidc_token_metadata_aws_iam_refresh_cb` |
| corpus | `rdkafka_subscription.c` | `rd_kafka_pause_partitions` | Safe | `rd_kafka_toppars_pause_resume` | `rd_kafka_toppars_pause_resume` |
| corpus | `rdkafka_subscription.c` | `rd_kafka_resume_partitions` | Safe | `rd_kafka_toppars_pause_resume` | `rd_kafka_toppars_pause_resume` |
| corpus | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_gauge` | Safe | `unit_test_telemetry` | `unit_test_telemetry` |
| corpus | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_sum` | Safe | `unit_test_telemetry` | `unit_test_telemetry` |
| corpus | `rdkafka_telemetry_decode.c` | `unittest_telemetry_decode` | Safe | `unit_test_telemetry` | `unit_test_telemetry` |

**Prediction against outcome.** Exactly as frozen. Verdicts and null rows are
identical; the 34 records gain the premise; no roll-up bucket moves (13
partly-validated declarations stay partly validated, the premise uncovered);
16 Safe and 317 Unknown corpus rows gain the premise; the pins move by one
clause on 34 rows.

## 6. Lean

None. A named premise changes no mode (C04.18). The verdict-dependency
theorems (`QCore.ContractDependencies`) already cover a declaration's
premises.

## 7. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py): 780 → **781**.

* **Amended, not retired: `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`.** Its
  DISCHARGE does not hold in full, because of the two decisions in section 4.
  The 34 are done, and the DISCHARGE is narrowed to the null views and the
  model contract.
* **Registered: `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED`.** This is outside
  the card's named modules. `memory.qv`'s `rd_kafka_crash` and `kafka_mem.qv`'s
  `rd_kafka_dump0` reach interceptor and log calls only through the
  assertion-failure dump's queue-reference drops. On the BEFORE corpus they
  are consulted by 67 rows (1 Safe) and 3 rows (2 Safe). The card premised
  `rd_kafka_toppar_get_avail` on the same path as the conservative label, and
  the entry asks for one ruling on all three.

## 8. Gate

GATE_SECTION

## Files

* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`; pins `compiler/tests/qcore_contract_corpus.rs`.
* Records: `docs/STATUS.md`, `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
