# W46-EXTERNAL-CALLEE-CONTRACTS: prediction (frozen before implementation)

Frozen at base `85a35431` before any provider file was edited. The ranking is
[`scripts/rank_external.py`](scripts/rank_external.py) over the baseline
`docs/reports/w45-container-root-reach/integration/artifacts/`, with owners
from [`scripts/classify_owners.py`](scripts/classify_owners.py). The BEFORE run
reproduced that baseline (diagnostics byte-identical; the contracts sidecar
differs only by the expected `refuted_contracts` section and source-effects
only by the digests that bind it).

## 1. What the ranking says

- 385 external callees (no definition in any corpus unit), 2,277 external
  `UNC` call sites. The plan's 390 / 2,284 counted callees by a different
  "defined" test; this card counts a name as defined when any unit lists it
  among its analysed functions or `source_functions`.
- **Only 33 Unknown functions are blocked alone by one external callee, and no
  callee blocks more than 2.** The brief's "at least 3 alone" threshold is met
  by nothing, so the scope is the libc/POSIX functions in the top 30 plus any
  exact contract that closes a function outright.
- **Every libc/POSIX function in the top 30 is already a recorded decline**,
  each for a named vocabulary or reader gap, not for lack of reading:
  `pthread_setspecific`/`pthread_getspecific` (retained pointer and its alias),
  `pthread_key_create`, `pthread_rwlock_init`/`_destroy` (in-place object
  lifecycles), `pthread_once` (caller-supplied function), `dlopen`/`dlclose`
  (no library-handle kind, spec C04.6), `bind` (transparent-union
  `__CONST_SOCKADDR_ARG`, withdrawn by the reader). The one exception is
  `fcntl`: its earlier decline was that no *single unrestricted* declaration is
  exact, and the `prctl` precedent (a literal premise) makes a declaration
  restricted to `F_SETFD` exact.

## 2. The contracts this card will write

| Callee | Owner / crate | Mode | Effect read from source |
|---|---|---|---|
| `fcntl` | libc, `quirin-provider-libc` `libc/unistd.qcore` | flexible | premise `cmd == 2` (`F_SETFD`): fd borrowed, tail data, `int` none |
| `mk_utils_libc_error` | Monkey, `qcore-provider-libevent` `monkey/logging.qcore` | pinned | all borrowed, `void` |
| `rd_atomic32_get` | librdkafka, `qcore-provider-librdkafka` `librdkafka/memory.qcore` | pinned | counter borrowed, `int32_t` none |
| `cfl_sds_len` | cfl, `quirin-provider-cfl` `cfl/sds_lifetime.qcore` | pinned | string borrowed, `size_t` none |
| `mpack_finish_map`, `mpack_finish_array`, `mpack_write_true`, `mpack_write_false`, `mpack_write_bin` | mpack, `quirin-provider-mpack` `mpack/writer.qcore` | pinned | writer (and data, count) borrowed, `void` |

All are in existing effect kinds (borrowed, return none, the literal premise,
the variadic data tail), so **no Lean theorem is added**.

## 3. Predicted function moves (Unknown → Safe), with the contract that moves each

| Function | Moved by | Confidence and risk |
|---|---|---|
| `cmt_encode_cloudwatch_emf.c::pack_basic_header_finish` | `mpack_finish_map`, `mpack_finish_array` | high |
| `ctr_encode_msgpack.c::pack_bool` | `mpack_write_true`, `mpack_write_false` | high |
| `ctr_encode_msgpack.c::pack_bytes` | `cfl_sds_len`, `mpack_write_bin` | high |
| `mk_server/mk_scheduler.c::mk_sched_send_signal` | `mk_utils_libc_error` | high |
| `mk_server/mk_socket.c::mk_socket_create` | `mk_utils_libc_error` | high (the descriptor from `socket` is returned on the valid edge) |
| `rdkafka.c::rd_kafka_thread_cnt` | `rd_atomic32_get` | medium: the actual is `&rd_kafka_thread_cnt_curr`, the address of a global |
| `rdkafka_offset.c::rd_kafka_open_cb_generic` | `fcntl` | medium: the premise must read `F_SETFD` as `2`, the declaration may meet a `fcntl64` redirect, and the tail carries `&on` |

**Prediction: +5 to +7 Safe, the same number fewer Unknown; units unchanged;
no Safe → non-Safe move; no new leak or violation.** Every other site of these
callees sits in a function with other blockers and only loses leaves
(`UNC` leaves: about 60 removed, mostly `mpack_finish_*` in cmetrics and
ctraces encoders). `mk_socket_set_nonblocking`'s `fcntl` sites use
`F_GETFL`/`F_SETFL` and stay refused, as uncontracted or outside the premise.

`getrlimit`/`setrlimit` (`mk_server_capacity`) are **not** attempted: their
first parameter is the enumeration typedef `__rlimit_resource_t`, which the
reader does not project (measured withdrawn by card W43-LIBC-CONTRACTS).

## 4. Fluent Bit system slice

None of the nine callees is called by a function of the eight slice files at
`66910c10` (checked by a textual search). If that holds at `046accf6`, the
slice is off route and is not re-run.
