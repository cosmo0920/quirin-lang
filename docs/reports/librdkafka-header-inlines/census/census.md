| # | Callee | Pairs | Sites | Definition (046accf6) | Same at all three | Body calls uncontracted in the run | Outcome |
|---|---|---:|---:|---|---|---|---|
| 1 | `rd_kafka_q_enq` | 83 | 87 | `rdkafka_queue.h:484` (header static inline) | yes | none | declared: `rko` consumed (linked or destroyed on every exit), premise |
| 2 | `rd_kafka_buf_write_i32` | 63 | 198 | `rdkafka_buf.h:1142` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 3 | `rd_kafka_buf_write_i8` | 47 | 115 | `rdkafka_buf.h:1105` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 4 | `rd_kafka_buf_write_str` | 47 | 92 | `rdkafka_buf.h:1338` (header static inline) | yes | `rd_kafka_buf_write`, `rd_kafka_buf_write_i16`, `rd_kafka_buf_write_uvarint` | declared: borrowed, scalar result |
| 5 | `rd_kafka_buf_write_i16` | 39 | 55 | `rdkafka_buf.h:1122` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 6 | `rd_kafka_buf_write_arraycnt` | 35 | 73 | `rdkafka_buf.h:1203` (header static inline) | yes | `rd_kafka_buf_write_i32`, `rd_kafka_buf_write_uvarint` | declared: borrowed, scalar result |
| 7 | `rd_kafka_buf_write_kstr` | 22 | 43 | `rdkafka_buf.h:1294` (header static inline) | yes | `rd_kafka_buf_write`, `rd_kafka_buf_write_i16`, `rd_kafka_buf_write_uvarint` | declared: borrowed, scalar result |
| 8 | `rd_kafka_replyq_destroy` | 21 | 42 | `rdkafka_queue.h:787` (header static inline) | yes | none | undeclared: drops the reference the member `replyq->q` holds; no v1 form drops a reference at a member path (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 9 | `rd_kafka_buf_write_i64` | 17 | 32 | `rdkafka_buf.h:1271` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 10 | `rd_kafka_buf_finalize_arraycnt` | 12 | 30 | `rdkafka_buf.h:1237` (header static inline) | yes | `rd_kafka_buf_update_i32` | declared: borrowed, premise (segment `seg_free`) |
| 11 | `rd_kafka_set_last_error` | 12 | 30 | `rdkafka_int.h:1251` (header static inline) | yes | none | declared: scalars |
| 12 | `rd_kafkap_str_new` | 12 | 13 | `rdkafka_proto.h:308` (header static inline) | yes | none | declared: owned, nullable |
| 13 | `rd_atomic32_add` | 10 | 10 | `rdatomic.h:56` (header static inline) | yes | none | owned elsewhere: the primitive under rd_refcnt_t is never declared (C04.17; REFCOUNT-PLAIN-COUNTER-DECLARATION) |
| 14 | `rd_kafka_buf_update_i32` | 8 | 13 | `rdkafka_buf.h:1153` (header static inline) | yes | none | declared: borrowed |
| 15 | `rd_kafka_buf_write_bool` | 8 | 10 | `rdkafka_buf.h:1447` (header static inline) | yes | `rd_kafka_buf_write_i8` | declared: borrowed, scalar result |
| 16 | `rd_kafka_buf_write_uuid` | 8 | 9 | `rdkafka_buf.h:1519` (header static) | yes | `rd_kafka_buf_write_i64` | declared: borrowed |
| 17 | `rd_kafka_msgq_enq` | 8 | 12 | `rdkafka_msg.h:530` (header static inline) | yes | none | declared: `rkm` consumed |
| 18 | `rd_kafka_idemp_get_pid0` | 7 | 7 | `rdkafka_idempotence.h:55` (header static inline) | yes | none | undeclared: struct result by value (`rd_kafka_pid_t`) (LIBRDKAFKA-HEADER-INLINES-RESIDUAL; MPACK-STRUCT-RESULT-BY-VALUE once landed) |
| 19 | `rd_kafka_op_set_replyq` | 7 | 7 | `rdkafka_queue.h:745` (header static inline) | yes | none | undeclared: keeps a queue reference into the op's member; a keep into a non-plain place is refused (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 20 | `rd_tmpabuf_alloc0` | 7 | 7 | `rdkafka_buf.h:104` (header static) | yes | none | declared: borrowed from `tab`, nullable |
| 21 | `rd_atomic32_sub` | 6 | 6 | `rdatomic.h:74` (header static inline) | yes | none | owned elsewhere: REFCOUNT-PLAIN-COUNTER-DECLARATION |
| 22 | `rd_kafka_msgq_init` | 6 | 11 | `rdkafka_msg.h:283` (header static inline) | yes | none | declared: borrowed |
| 23 | `rd_atomic32_set` | 5 | 7 | `rdatomic.h:118` (header static inline) | yes | none | owned elsewhere: REFCOUNT-PLAIN-COUNTER-DECLARATION |
| 24 | `rd_kafka_app_poll_start` | 5 | 5 | `rdkafka_int.h:1366` (header static inline) | yes | `rd_atomic64_set` | declared: borrowed |
| 25 | `rd_kafka_app_polled` | 5 | 11 | `rdkafka_int.h:1402` (header static inline) | yes | `rd_atomic64_set`, `rd_kafka_cgrp_consumer_expedite_next_heartbeat` | declared: borrowed, premise |
| 26 | `rd_kafka_buf_write_kbytes` | 5 | 5 | `rdkafka_buf.h:1390` (header static inline) | yes | `rd_kafka_buf_write`, `rd_kafka_buf_write_i32`, `rd_kafka_buf_write_uvarint` | declared: borrowed, scalar result |
| 27 | `rd_kafka_msgq_deq` | 5 | 5 | `rdkafka_msg.h:372` (header static inline) | yes | none | undeclared: hands the caller a message unlinked from a borrowed queue; `resource_return_removed` needs an owned parent and an alias owns nothing (W45-SUB-OBJECT-STORE-FORMAL-VALUE) |
| 28 | `rd_kafka_topic_partition_get_toppar` | 5 | 5 | `rdkafka_partition.h:929` (header static inline) | yes | `rd_kafka_topic_partition_ensure_toppar` | undeclared: returns a reference kept by a statement on a local assigned from another call; rule 13d would withdraw it in every including unit (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 29 | `rd_kafka_buf_write_bytes` | 4 | 5 | `rdkafka_buf.h:1431` (header static inline) | yes | `rd_kafka_buf_write`, `rd_kafka_buf_write_i32` | declared: borrowed, scalar result |
| 30 | `rd_kafka_buf_write_uvarint` | 4 | 7 | `rdkafka_buf.h:1185` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 31 | `rd_kafka_msgq_first` | 4 | 5 | `rdkafka_msg.h:404` (header static inline) | yes | none | declared: borrowed from `rkmq`, nullable |
| 32 | `rd_usleep` | 4 | 4 | `rdposix.h:144` (header static inline) | yes | none | declared: borrowed |
| 33 | `rd_atomic64_set` | 3 | 3 | `rdatomic.h:208` (header static inline) | yes | none | declared: borrowed (64-bit, never a reference count) |
| 34 | `rd_avl_find` | 3 | 3 | `rdavl.h:234` (header static inline) | yes | `rd_avl_find_node` | undeclared: returns an element the tree indexes but does not own; no v1 result states it (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 35 | `rd_kafka_broker_get_state` | 3 | 4 | `rdkafka_broker.h:513` (header static inline) | yes | none | declared: borrowed, scalar result |
| 36 | `rd_kafka_buf_write` | 3 | 9 | `rdkafka_buf.h:1071` (header static inline) | yes | `rd_buf_write`, `rd_crc32_update` | declared: borrowed, scalar result |
| 37 | `rd_kafka_enq_once_del_source` | 3 | 4 | `rdkafka_queue.h:1045` (header static inline) | yes | none | undeclared: drops an eonce reference; the eonce family has no declared constructor or keep (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 38 | `rd_kafka_msgq_last` | 3 | 3 | `rdkafka_msg.h:414` (header static inline) | yes | none | declared: borrowed from `rkmq`, nullable |
| 39 | `rd_kafka_offset_store0` | 3 | 3 | `rdkafka_offset.h:88` (header static inline) | yes | none | undeclared: struct parameter by value (`rd_kafka_fetch_pos_t`), no ABI projection; also replaces a member allocation (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 40 | `rd_kafka_q_concat0` | 3 | 3 | `rdkafka_queue.h:550` (header static inline) | yes | `cnd_signal`, `rd_kafka_q_io_event`, `rd_kafka_q_reset` | declared: borrowed, premise (queue `event_cb`) |
| 41 | `rd_kafka_q_yield` | 3 | 3 | `rdkafka_queue.h:366` (header static inline) | yes | `cnd_broadcast`, `rd_kafka_q_io_event` | declared: borrowed, premise |
| 42 | `rd_kafka_replyq_enq` | 3 | 3 | `rdkafka_queue.h:809` (header static inline) | yes | `rd_kafka_q_enq` | undeclared: takes over the member's queue reference, enqueues, drops it (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 43 | `rd_kafka_replyq_make` | 3 | 3 | `rdkafka_queue.h:674` (header static inline) | yes | none | undeclared: struct result by value, with a queue keep inside it (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 44 | `rd_kafkap_bytes_new` | 3 | 3 | `rdkafka_proto.h:440` (header static inline) | yes | none | declared: owned, nullable |
| 45 | `rd_segment_write_remains` | 3 | 3 | `rdbuf.h:163` (header static inline) | yes | none | undeclared: an out slot null-tested before its write; rule 13a would withdraw `resource_out_derived` in every including unit (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 46 | `rd_buf_write_pos` | 2 | 8 | `rdbuf.h:132` (header static inline) | yes | none | declared: borrowed, scalar result |
| 47 | `rd_crc32_init` | 2 | 3 | `rdcrc32.h:98` (header static inline) | yes | none | declared: scalar result |
| 48 | `rd_crc32_update` | 2 | 5 | `rdcrc32.h:123` (header static inline) | yes | none | declared: borrowed, scalar result |
| 49 | `rd_file_mkstemp` | 2 | 2 | `rd.h:524` (header static inline) | yes | none | undeclared: stream result whose body leaks its descriptor when `fdopen` fails; needs a reviewed reading (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 50 | `rd_getenv` | 2 | 2 | `rdposix.h:170`, `rdwin32.h:220` (header static inline) | yes | none | undeclared: returns `getenv`'s storage or the argument `def`; no v1 result states either-or (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 51 | `rd_kafka_buf_update_i16` | 2 | 2 | `rdkafka_buf.h:1133` (header static inline) | yes | none | declared: borrowed |
| 52 | `rd_kafka_enq_once_del_source_return` | 2 | 2 | `rdkafka_queue.h:1079` (header static inline) | yes | none | undeclared: eonce drop that hands back the op the eonce holds (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 53 | `rd_kafka_enq_once_disable` | 2 | 2 | `rdkafka_queue.h:1191` (header static inline) | yes | `rd_kafka_replyq_destroy` | undeclared: eonce drop, member replyq drop, hands back the held op (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 54 | `rd_kafka_msgq_concat` | 2 | 2 | `rdkafka_msg.h:312` (header static inline) | yes | `rd_kafka_msgq_init` | declared: borrowed |
| 55 | `rd_kafka_msgq_move` | 2 | 2 | `rdkafka_msg.h:325` (header static inline) | yes | `rd_kafka_msgq_init` | declared: borrowed |
| 56 | `rd_kafka_q_len` | 2 | 2 | `rdkafka_queue.h:638` (header static inline) | yes | none | declared: borrowed, premise |
| 57 | `rd_kafka_q_reset` | 2 | 2 | `rdkafka_queue.h:263` (header static inline) | yes | none | declared: borrowed |
| 58 | `rd_kafka_topic_partition_toppar` | 2 | 2 | `rdkafka_partition.h:909` (header static inline) | yes | none | declared: borrowed from `rktpar`, nullable |
| 59 | `rd_kafka_txn_add_partition` | 2 | 3 | `rdkafka_txnmgr.h:96` (header static inline) | yes | `rd_kafka_txn_schedule_register_partitions` | declared: borrowed (the list takes its own reference), premise |
| 60 | `rd_tmpabuf_write_str0` | 2 | 2 | `rdkafka_buf.h:157` (header static) | yes | `rd_tmpabuf_write0` | undeclared: forwards rd_tmpabuf_write0 (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 61 | `rd_avg_destroy` | 1 | 1 | `rdavg.h:252` (header static) | yes | `mtx_destroy` | undeclared: releases the member `ra->ra_hdr` and destroys a mutex in place (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 62 | `rd_avg_rollover` | 1 | 1 | `rdavg.h:139` (header static) | yes | `mtx_init`, `rd_hdr_histogram_mean`, `rd_hdr_histogram_new`, `rd_hdr_histogram_quantile`, `rd_hdr_histogram_stddev` | undeclared: replaces the member `src->ra_hdr` and initialises `dst` in place (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 63 | `rd_avl_remove_elm` | 1 | 2 | `rdavl.h:220` (header static inline) | yes | `rd_avl_remove_elm0` | declared: borrowed, premise (tree `ravl_cmp`) |
| 64 | `rd_kafka_broker_active_toppar_next` | 1 | 1 | `rdkafka_broker.h:802` (header static inline) | yes | none | undeclared: a retention by the caller's broker (C04.14), retention-undecided at every call (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 65 | `rd_kafka_buf_crc_init` | 1 | 1 | `rdkafka_buf.h:1474` (header static inline) | yes | `rd_crc32_init` | declared: borrowed |
| 66 | `rd_kafka_buf_update_i64` | 1 | 2 | `rdkafka_buf.h:1282` (header static inline) | yes | none | declared: borrowed |
| 67 | `rd_kafka_buf_update_i8` | 1 | 1 | `rdkafka_buf.h:1114` (header static inline) | yes | none | declared: borrowed |
| 68 | `rd_kafka_buf_update_u32` | 1 | 1 | `rdkafka_buf.h:1163` (header static inline) | yes | none | declared: borrowed |
| 69 | `rd_kafka_buf_write_varint` | 1 | 1 | `rdkafka_buf.h:1172` (header static inline) | yes | `rd_kafka_buf_write` | declared: borrowed, scalar result |
| 70 | `rd_kafka_curr_msgs_add` | 1 | 1 | `rdkafka_int.h:975` (header static inline) | yes | `rwlock_rdlock`, `rwlock_rdunlock` | declared: borrowed, scalar result |
| 71 | `rd_kafka_curr_msgs_wait_zero` | 1 | 1 | `rdkafka_int.h:1079` (header static inline) | yes | none | declared: borrowed, scalar result |
| 72 | `rd_kafka_enq_once_destroy` | 1 | 1 | `rdkafka_queue.h:1158` (header static inline) | yes | `rd_kafka_replyq_destroy` | undeclared: eonce drop and member replyq drop (REFCOUNT-MEMBER-REFERENCE-DROP) |
| 73 | `rd_kafka_msgq_purge` | 1 | 1 | `rdkafka_msg.h:352` (header static inline) | yes | `rd_kafka_msg_destroy`, `rd_kafka_msgq_init` | declared: borrowed, premise |
| 74 | `rd_kafka_op_version_outdated` | 1 | 1 | `rdkafka_partition.h:1095` (header static inline) | yes | none | declared: borrowed, scalar result |
| 75 | `rd_kafka_partition_leader_destroy` | 1 | 1 | `rdkafka_partition.h:1126` (header static) | yes | none | declared: `leader` released, premise |
| 76 | `rd_kafka_partition_leader_new` | 1 | 1 | `rdkafka_partition.h:1135` (header static) | yes | `rd_kafka_topic_partition_list_new` | declared: owned, nullable |
| 77 | `rd_kafka_q_io_event` | 1 | 1 | `rdkafka_queue.h:322` (header static inline) | yes | none | undeclared: the queue event-callback invocation itself; its candidates need a registry declaration (C04.19) (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 78 | `rd_kafka_q_last` | 1 | 1 | `rdkafka_queue.h:915` (header static inline) | yes | none | undeclared: a borrowed result found by a list walk; rule 13a reads only member paths (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 79 | `rd_kafka_q_prepend0` | 1 | 1 | `rdkafka_queue.h:609` (header static inline) | yes | `rd_kafka_q_io_event`, `rd_kafka_q_reset` | declared: borrowed, premise (queue `event_cb`) |
| 80 | `rd_kafka_q_reenq` | 1 | 1 | `rdkafka_queue.h:501` (header static inline) | yes | none | declared: `rko` consumed, premise |
| 81 | `rd_kafka_rkt_get_lw` | 1 | 1 | `rdkafka_topic.h:70` (header static inline) | yes | none | declared: the argument's alias or NULL |
| 82 | `rd_kafka_topic_get_error` | 1 | 1 | `rdkafka_topic.h:296` (header static inline) | yes | none | declared: borrowed, scalar result |
| 83 | `rd_kafka_topic_partition_get_fetch_pos` | 1 | 1 | `rdkafka_partition.h:1004` (header static) | yes | `rd_kafka_topic_partition_get_leader_epoch` | undeclared: struct result by value (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 84 | `rd_slice_pos` | 1 | 1 | `rdbuf.h:261` (header static inline) | yes | none | undeclared: struct result by value (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 85 | `rd_slice_read_varint` | 1 | 2 | `rdbuf.h:323` (header static inline) | yes | none | declared: borrowed, scalar result |
| 86 | `rd_sockaddr_list_next` | 1 | 1 | `rdaddr.h:133` (header static inline) | yes | none | undeclared: a borrowed result through an array index; rule 13a reads only member paths (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 87 | `rd_tmpabuf_destroy` | 1 | 1 | `rdkafka_buf.h:85` (header static) | yes | none | undeclared: releases the member `tab->buf`, not `tab` (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 88 | `rd_tmpabuf_write0` | 1 | 1 | `rdkafka_buf.h:138` (header static) | yes | `rd_tmpabuf_alloc0` | undeclared: forwards rd_tmpabuf_alloc0's borrowed result, which W4 may now infer (C04.3.4 rule 6) (LIBRDKAFKA-HEADER-INLINES-RESIDUAL) |
| 89 | `rd_varint_dec_i64` | 1 | 1 | `rdvarint.h:135` (header static inline) | yes | none | declared: borrowed, scalar result |

Owned elsewhere (external, no body in the run, called from librdkafka):

| Owner | Callees | Pairs | Largest |
|---|---:|---:|---|
| external library libcurl (own provider/plugin crate; user decision) | 56 | 169 | `Wcurl_easy_setopt_err_CURLSH` 4, `Wcurl_easy_setopt_err_FILE` 4, `Wcurl_easy_setopt_err_cb_data` 4, `Wcurl_easy_setopt_err_chunk_bgn_cb` 4 |
| external library OpenSSL (own provider/plugin crate; user decision) | 68 | 99 | `X509_free` 6, `BIO_free` 5, `BIO_new_mem_buf` 4, `EVP_PKEY_free` 4 |
| LIBC-GAP-DECLARATIONS (libc or compiler builtin) | 22 | 83 | `__builtin_alloca` 41, `__builtin_memcpy` 8, `freeaddrinfo` 4, `getaddrinfo` 4 |
| not a function: a call through a function-pointer object (indirect-call rules) | 8 | 12 | `callback` 5, `cb` 1, `copy_cb` 1, `rd_list_cmp_curr` 1 |
| external library zstd (own provider/plugin crate; user decision) | 7 | 7 | `ZSTD_compressStream` 1, `ZSTD_createCStream` 1, `ZSTD_decompress` 1, `ZSTD_endStream` 1 |
| declared already; owned by DECLARED-NAMES-RESOURCE-V1-PROJECTIONS | 1 | 6 | `rd_kafka_q_deq0` 6 |
| external library zlib (own provider/plugin crate; user decision) | 5 | 5 | `compress` 1, `deflateEnd` 1, `deflateInit2_` 1, `inflate` 1 |
| external library Cyrus SASL (own provider/plugin crate; user decision) | 4 | 4 | `sasl_client_init` 1, `sasl_client_start` 1, `sasl_client_step` 1, `sasl_listmech` 1 |
