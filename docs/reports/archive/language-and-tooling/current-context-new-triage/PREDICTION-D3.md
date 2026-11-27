# CURRENT-CONTEXT-NEW-TRIAGE, D3: prediction

Frozen before any D3 code. Base for the D3 measurement: `3d735b88c`, whose
code equals phase 1's `1e691eecc`. Measurement side: `baseline:497bdeb90`. As
with phase 1, a baseline-vs-tip run also shows the r8-r12 moves and phase 1's
`context_new`; attribute only the moves listed here.

User decision, 2026-11-16, relayed by the integrator and quoted exactly: D1/D3
→ "Queue both (Recommended)". No measurement was run for this file.

## Mechanism

D3 extends CG25's in-place mode (rule 11v) with a new in-place kind,
`thread_key`.

* **Contracts** (`quirin-provider-libc`, `threads.qv`).
  * `pthread_key_create(key, destructor)` initialises `*key` in place. It
    applies only where `destructor` is the null pointer constant
    (`resource_literal_argument(destructor, 0)`); a non-NULL destructor is a
    callback registration, so the call stays undeclared there.
  * `pthread_key_delete(key)` finalises the key, which it receives **by
    value**.
* **Gateway** (`legacy_in_place_custody.rs`).
  * A creation into `&L`, with `L` one automatic object, is CG25's rule
    unchanged: the status witnesses the creation, and a by-value
    `pthread_key_delete(L)` discharges it.
  * A creation into `&G`, with `G` an unshadowed object of **static** storage
    duration (file scope or block `static`; not thread-local, volatile or
    atomic), creates **no frame obligation**. The key is held by static
    storage for the process, under the stated premise P-THREAD-KEY-STATIC:
    a key created into static storage with no destructor is created once
    and is not owed a delete. Monkey guards each such creation by
    `pthread_once` or a flag. Its status need not be witnessed, because no
    frame obligation exists on either edge.
  * Refused:
    * `pthread_key_delete(G)` of a static key: its holder's lifetime is
      outside the frame;
    * a delete of anything untracked;
    * a creation into any other place (a member, `node->thread_key`, a
      pointer read, a thread-local object).

## Rows that call the pair (all routes at `baseline:497bdeb90`, and the evidence run)

| Function | Route(s) | Call(s) | Today | Predicted |
|---|---|---|---|---|
| `mk_http_thread_initialize_tls_once` | corpus, current | `pthread_key_create(&mk_http_thread_libco_params, NULL)`, `(&mk_thread, NULL)`: `MK_TLS_DEFINE` file-scope `pthread_key_t`s | Unknown (only those two refusals) | **Safe** |
| `mk_plugin_preworker_calls` | corpus, current | `pthread_key_create(node->thread_key, NULL)` (a pointer read, not `&G`) | Unknown | Unknown (the uncontracted refusal becomes rule 11v's actual refusal). PROPOSAL.md's D3 row predicted Safe here; it was wrong, and this replaces it |
| `mk_set_up_tls_keys` | corpus, current | two static creations | Unknown | Unknown (`mk_http_thread_initialize_tls` stays uncontracted: its `pthread_once` callback is refused as `callback-undecided`) |
| `mk_fifo_create` | corpus, current | `pthread_key_create(ctx->key, NULL)` | Unknown | Unknown (actual refusal plus 7 others) |
| `mk_tls_init` | current | `pthread_key_create(&local_context, NULL)` (static) | Unknown | Unknown (refusal removed, 14 others remain) |
| `mk_tls_exit` | current | `pthread_key_delete(local_context)` (static, by value) | Unknown | Unknown (static-delete refusal replaces the uncontracted one) |

System and slice have no row that calls either function. No `Safe` row
moves. No leak, double release or `Unsafe` is added: a refusal replaces a
refusal, and the only removal is the static creation, which creates no
obligation.

## Predicted moves

* corpus: `mk_http_thread_initialize_tls_once` `Unknown -> Safe`.
* system, slice: none.
* the user's `current --monkey-linked`: `mk_http_thread_initialize_tls_once`
  `Unknown -> Safe`; no other status move.

```measure-prediction
route corpus
move mk_http_thread_initialize_tls_once Unknown -> Safe
delta Safe +1
delta Unknown -1
route system
none
route slice
none
```

## Amendment, before the null-pointer-constant code (2026-11-16)

The text above was frozen before D3's code. A unit-scoped reproduction then
showed that Monkey's `NULL` (`((void *) 0)` on glibc) is outside the
literal-argument premise as §C04.5.1 stated it, so D3 alone moved no row (see
the README's D3 finding).

User decision, 2026-11-16, relayed by the integrator and quoted exactly:
"Admit (void *) 0 (Recommended)". §C04.5.1 is amended to admit the C11
6.3.2.3p3 null pointer constant `(void *) 0` (matched parentheses only, the
cast exactly `void *`, the operand the canonical `0`), and only at a
pointer-typed premise parameter whose declared literal is `0`. Scalar
premises are unchanged.

Reach of the amendment: pointer premises only. These are `pthread_join`'s
`thread_return` and `pthread_key_create`'s `destructor`. Every
literal-premise refusal on corpus, system and slice today is a scalar premise
(`flb_hash_table_add`, `fcntl`, `flb_task_users_dec`,
`rd_kafka_topic_partition_destroy0`). The `pthread_join` callers
(`mk_stop`, `mk_sched_workers_join`, `mk_clock_exit`) keep the
thread-synchronization gate. So the predicted moves are exactly those stated
above:

* corpus: `mk_http_thread_initialize_tls_once` `Unknown -> Safe`;
* system, slice: none;
* the user's current: `mk_http_thread_initialize_tls_once`
  `Unknown -> Safe`, nothing else.

The `measure-prediction` block above stands unchanged.
