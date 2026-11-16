# THREAD-LOCAL-CUSTODY-FREE-READ

User decision, 2026-11-16, quoted exactly: "Queue both (Recommended)". This
card is option D1 of CURRENT-CONTEXT-NEW-TRIAGE's proposal
(`docs/reports/current-context-new-triage/PROPOSAL.md` sections 3-4 on that
card's branch, tip `3d735b88c`): read a `__thread` / `_Thread_local` pointer
variable's value as a custody-free value, the reading `pthread_getspecific`'s
result already gets. Base `0f6e17118`.

## 1. Census (before any semantic change)

Every reported function that names a thread-local object, on the three live
routes at `baseline:497bdeb90` and on the user's `current --monkey-linked`
report at binary `89fa31d21` (the integrator's evidence tree, read only).
Script: [`scripts/tls_census.py`](scripts/tls_census.py) (preprocesses each
unit with its own compile command, finds the thread-local declarations and
the main-file bodies naming one, reads each such function's Clang AST and
classifies every reference with the card's admission); table:
[`scripts/tls_census_table.py`](scripts/tls_census_table.py). Output:
[`census/tls_census.json`](census/tls_census.json), rendered in
[`census/table.md`](census/table.md). "Only thread-local aliases" says every
diagnostic of the row is an unresolved identity of a thread-local object or
of one of its copies, the only diagnostics this reading can remove.

| Route | Rows naming a thread-local object | Reading admitted | Rows whose every blocker is the read |
|---|---|---|---|
| corpus (`baseline:497bdeb90`) | 23 (librdkafka only) | 0 | 0 |
| system | 12 (6 functions x 2 configurations) | 4 (`flb_engine_evl_get`, `flb_sched_ctx_get`) | 4 |
| slice | 8 (4 x 2) | 4 (`flb_context_get`, `flb_cf_context_get`) | 4 |
| current (`--monkey-linked`, `89fa31d21`) | 23 (the corpus's librdkafka rows) | 0 | 0 |

What blocks each today, and what the reading does:

* **The four getters** (`flb_engine_evl_get`, `flb_sched_ctx_get` on system;
  `flb_context_get`, `flb_cf_context_get` on slice; two configurations each)
  are `Unknown` for exactly two diagnostics, the unresolved identity of the
  thread-local object at the copy (`evl = flb_engine_evl;`) and of the local
  at the `return`. Every reference is a copy into an automatic local and a
  return. The reading is admitted and predicted `Unknown -> Safe`: 8 rows.
* **The setters** (`flb_engine_evl_set`, `flb_sched_ctx_set`,
  `flb_context_set`, `flb_cf_context_set`) write the object: the reading is
  withheld. Their one diagnostic (the stored formal's unresolved identity,
  rule 10's publication) is untouched.
* **`flb_engine_shutdown`, `flb_output_exit`** (system) write the object and
  hand the copy to calls and dereferences: withheld; 17 and 10 independent
  diagnostics.
* **librdkafka** (corpus, current): `rd_kafka_yield_thread` is an `int`,
  `rd_kafka_last_error_code` an enumeration, `rd_kafka_thread_name` /
  `rd_kafka_thread_sysname` arrays, all outside the object-pointer
  condition; `rd_kafka_curr_transport` is written in five transport functions
  and dereferenced through its copy in `rd_kafka_transport_ssl_cert_verify_cb`;
  `rd_qsort_r_arg` is written in `rd_qsort_r` and passed to a call in
  `rd_qsort_r_trampoline`. All withheld. The three Safe rows
  (`rd_kafka_last_error`, `rd_kafka_set_thread_name`, `rd_kafka_yield`) are
  already Safe and stay so.
* **Monkey** reads no `__thread` object on any route: neither the corpus's
  Monkey build nor Fluent Bit's `lib/monkey` defines `MK_HAVE_C_TLS`, so
  `MK_TLS_GET` is `pthread_getspecific` there (0 of 1,419 current compile
  commands, 0 of 142 corpus Monkey commands). Fluent Bit's own units define
  `FLB_HAVE_C_TLS` (841 of 1,419), so `FLB_TLS_GET(x)` is a plain read.
  `lib/monkey/tls/tls_stub.c` has no compile command in the current report,
  so the script could not preprocess it; its source names no `__thread`,
  `_Thread_local` or `*_TLS_*` macro.

No other function on any route reads a thread-local object.
