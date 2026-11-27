### corpus: 23 rows, 0 unit errors

| Unit | Function | Status | Thread-local uses | Copies | Reading | Diagnostics | Only thread-local aliases |
|---|---|---|---|---|---|---|---|
| `rdkafka.c` | `rd_kafka_consume0` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 7 | no |
| `rdkafka.c` | `rd_kafka_flush` | Unknown | `rd_kafka_yield_thread`: assigned, null-test | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 7 | no |
| `rdkafka.c` | `rd_kafka_last_error` | Safe | `rd_kafka_last_error_code`: return | - | withheld: rd_kafka_last_error_code: not an object pointer (rd_kafka_resp_err_t) | 0 | no |
| `rdkafka.c` | `rd_kafka_log0` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 1 | no |
| `rdkafka.c` | `rd_kafka_poll_cb` | Unknown | `rd_kafka_yield_thread`: other:ParenExpr | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: other:ParenExpr | 65 | no |
| `rdkafka.c` | `rd_kafka_set_thread_name` | Safe | `rd_kafka_thread_name`: array, other:ParenExpr | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array; rd_kafka_thread_name: other:ParenExpr | 0 | no |
| `rdkafka.c` | `rd_kafka_set_thread_sysname` | Unknown | `rd_kafka_thread_sysname`: array, other:ParenExpr, array | - | withheld: rd_kafka_thread_sysname: not an object pointer (char[16]); rd_kafka_thread_sysname: array; rd_kafka_thread_sysname: other:ParenExpr | 1 | no |
| `rdkafka_interceptor.c` | `rd_kafka_interceptors_on_thread_exit` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 3 | no |
| `rdkafka_interceptor.c` | `rd_kafka_interceptors_on_thread_start` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 3 | no |
| `rdkafka_op.c` | `rd_kafka_op_call` | Unknown | `rd_kafka_yield_thread`: other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: other-read:BinaryOperator | 2 | no |
| `rdkafka_queue.c` | `rd_kafka_q_pop_serve0` | Unknown | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 7 | no |
| `rdkafka_queue.c` | `rd_kafka_q_serve0` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 12 | no |
| `rdkafka_queue.c` | `rd_kafka_q_serve_rkmessages` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 51 | no |
| `rdkafka_queue.c` | `rd_kafka_q_serve_share_rkmessages` | Unknown | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 27 | no |
| `rdkafka_queue.c` | `rd_kafka_yield` | Safe | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 0 | no |
| `rdkafka_ssl.c` | `rd_kafka_transport_ssl_cert_verify_cb` | Unknown | `rd_kafka_curr_transport`: init:rktrans | `rktrans`: null-test, null-test, deref-member | withheld: copy rktrans: deref-member | 21 | no |
| `rdkafka_transport.c` | `rd_kafka_transport_close` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 10 | no |
| `rdkafka_transport.c` | `rd_kafka_transport_connect_done` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 2 | no |
| `rdkafka_transport.c` | `rd_kafka_transport_io_serve` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 7 | no |
| `rdkafka_transport.c` | `rd_kafka_transport_recv` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 3 | no |
| `rdkafka_transport.c` | `rd_kafka_transport_send` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 3 | no |
| `rdports.c` | `rd_qsort_r` | Unknown | `rd_qsort_r_arg`: assigned, assigned | - | withheld: rd_qsort_r_arg: assigned; rd_qsort_r_arg: assigned | 3 | no |
| `rdports.c` | `rd_qsort_r_trampoline` | Unknown | `rd_qsort_r_arg`: call-argument | - | withheld: rd_qsort_r_arg: call-argument | 1 | no |

### current: 23 rows, 1 unit errors
- unit error lib/monkey/tls/tls_stub.c: no compile command

| Unit | Function | Status | Thread-local uses | Copies | Reading | Diagnostics | Only thread-local aliases |
|---|---|---|---|---|---|---|---|
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_consume0` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 7 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_flush` | Unknown | `rd_kafka_yield_thread`: assigned, null-test | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 7 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_last_error` | Safe | `rd_kafka_last_error_code`: return | - | withheld: rd_kafka_last_error_code: not an object pointer (rd_kafka_resp_err_t) | 0 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_log0` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 1 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_poll_cb` | Unknown | `rd_kafka_yield_thread`: other:ParenExpr | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: other:ParenExpr | 62 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_set_thread_name` | Safe | `rd_kafka_thread_name`: array, other:ParenExpr | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array; rd_kafka_thread_name: other:ParenExpr | 0 | no |
| `lib/librdkafka-2.15.0/src/rdkafka.c` | `rd_kafka_set_thread_sysname` | Unknown | `rd_kafka_thread_sysname`: array, other:ParenExpr, array | - | withheld: rd_kafka_thread_sysname: not an object pointer (char[16]); rd_kafka_thread_sysname: array; rd_kafka_thread_sysname: other:ParenExpr | 1 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_interceptor.c` | `rd_kafka_interceptors_on_thread_exit` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 3 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_interceptor.c` | `rd_kafka_interceptors_on_thread_start` | Unknown | `rd_kafka_thread_name`: array | - | withheld: rd_kafka_thread_name: not an object pointer (char[64]); rd_kafka_thread_name: array | 3 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_op.c` | `rd_kafka_op_call` | Unknown | `rd_kafka_yield_thread`: other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: other-read:BinaryOperator | 2 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_queue.c` | `rd_kafka_q_pop_serve0` | Unknown | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 7 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_queue.c` | `rd_kafka_q_serve0` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 12 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_queue.c` | `rd_kafka_q_serve_rkmessages` | Unknown | `rd_kafka_yield_thread`: assigned, other-read:BinaryOperator | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned; rd_kafka_yield_thread: other-read:BinaryOperator | 48 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_queue.c` | `rd_kafka_q_serve_share_rkmessages` | Unknown | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 26 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_queue.c` | `rd_kafka_yield` | Safe | `rd_kafka_yield_thread`: assigned | - | withheld: rd_kafka_yield_thread: not an object pointer (int); rd_kafka_yield_thread: assigned | 0 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_ssl.c` | `rd_kafka_transport_ssl_cert_verify_cb` | Unknown | `rd_kafka_curr_transport`: init:rktrans | `rktrans`: null-test, null-test, deref-member | withheld: copy rktrans: deref-member | 21 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_transport.c` | `rd_kafka_transport_close` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 10 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_transport.c` | `rd_kafka_transport_connect_done` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 2 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_transport.c` | `rd_kafka_transport_io_serve` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 7 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_transport.c` | `rd_kafka_transport_recv` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 3 | no |
| `lib/librdkafka-2.15.0/src/rdkafka_transport.c` | `rd_kafka_transport_send` | Unknown | `rd_kafka_curr_transport`: assigned | - | withheld: rd_kafka_curr_transport: assigned | 3 | no |
| `lib/librdkafka-2.15.0/src/rdports.c` | `rd_qsort_r` | Unknown | `rd_qsort_r_arg`: assigned, assigned | - | withheld: rd_qsort_r_arg: assigned; rd_qsort_r_arg: assigned | 3 | no |
| `lib/librdkafka-2.15.0/src/rdports.c` | `rd_qsort_r_trampoline` | Unknown | `rd_qsort_r_arg`: call-argument | - | withheld: rd_qsort_r_arg: call-argument | 1 | no |

### slice: 8 rows, 0 unit errors

| Unit | Function | Status | Thread-local uses | Copies | Reading | Diagnostics | Only thread-local aliases |
|---|---|---|---|---|---|---|---|
| `src/flb_lib.c` | `flb_cf_context_get` | Unknown | `flb_lib_active_cf_context`: copy:cf | `cf`: assigned, return | admitted | 2 | yes |
| `src/flb_lib.c` | `flb_cf_context_set` | Unknown | `flb_lib_active_cf_context`: assigned | - | withheld: flb_lib_active_cf_context: assigned | 1 | no |
| `src/flb_lib.c` | `flb_context_get` | Unknown | `flb_lib_active_context`: copy:ctx | `ctx`: assigned, return | admitted | 2 | yes |
| `src/flb_lib.c` | `flb_context_set` | Unknown | `flb_lib_active_context`: assigned | - | withheld: flb_lib_active_context: assigned | 1 | no |
| `src/flb_lib.c` | `flb_cf_context_get` | Unknown | `flb_lib_active_cf_context`: copy:cf | `cf`: assigned, return | admitted | 2 | yes |
| `src/flb_lib.c` | `flb_cf_context_set` | Unknown | `flb_lib_active_cf_context`: assigned | - | withheld: flb_lib_active_cf_context: assigned | 1 | no |
| `src/flb_lib.c` | `flb_context_get` | Unknown | `flb_lib_active_context`: copy:ctx | `ctx`: assigned, return | admitted | 2 | yes |
| `src/flb_lib.c` | `flb_context_set` | Unknown | `flb_lib_active_context`: assigned | - | withheld: flb_lib_active_context: assigned | 1 | no |

### system: 12 rows, 0 unit errors

| Unit | Function | Status | Thread-local uses | Copies | Reading | Diagnostics | Only thread-local aliases |
|---|---|---|---|---|---|---|---|
| `src/flb_engine.c` | `flb_engine_evl_get` | Unknown | `flb_engine_evl`: copy:evl | `evl`: assigned, return | admitted | 2 | yes |
| `src/flb_engine.c` | `flb_engine_evl_set` | Unknown | `flb_engine_evl`: assigned | - | withheld: flb_engine_evl: assigned | 1 | no |
| `src/flb_engine.c` | `flb_engine_shutdown` | Unknown | `sched_timer_coro_cb_params`: copy:sched_params, assigned | `sched_params`: assigned, other-read:BinaryOperator, deref-member, call-argument | withheld: sched_timer_coro_cb_params: assigned; copy sched_params: other-read:BinaryOperator; copy sched_params: deref-member | 17 | no |
| `src/flb_engine.c` | `flb_engine_evl_get` | Unknown | `flb_engine_evl`: copy:evl | `evl`: assigned, return | admitted | 2 | yes |
| `src/flb_engine.c` | `flb_engine_evl_set` | Unknown | `flb_engine_evl`: assigned | - | withheld: flb_engine_evl: assigned | 1 | no |
| `src/flb_engine.c` | `flb_engine_shutdown` | Unknown | `sched_timer_coro_cb_params`: copy:sched_params, assigned | `sched_params`: assigned, other-read:BinaryOperator, deref-member, call-argument | withheld: sched_timer_coro_cb_params: assigned; copy sched_params: other-read:BinaryOperator; copy sched_params: deref-member | 17 | no |
| `src/flb_scheduler.c` | `flb_sched_ctx_get` | Unknown | `flb_sched_ctx`: copy:sched | `sched`: assigned, return | admitted | 2 | yes |
| `src/flb_scheduler.c` | `flb_sched_ctx_set` | Unknown | `flb_sched_ctx`: assigned | - | withheld: flb_sched_ctx: assigned | 1 | no |
| `src/flb_scheduler.c` | `flb_sched_ctx_get` | Unknown | `flb_sched_ctx`: copy:sched | `sched`: assigned, return | admitted | 2 | yes |
| `src/flb_scheduler.c` | `flb_sched_ctx_set` | Unknown | `flb_sched_ctx`: assigned | - | withheld: flb_sched_ctx: assigned | 1 | no |
| `src/flb_output.c` | `flb_output_exit` | Unknown | `out_flush_params`: copy:params, assigned | `params`: assigned, null-test, call-argument | withheld: out_flush_params: assigned; copy params: call-argument | 10 | no |
| `src/flb_output.c` | `flb_output_exit` | Unknown | `out_flush_params`: copy:params, assigned | `params`: assigned, null-test, call-argument | withheld: out_flush_params: assigned; copy params: call-argument | 10 | no |

