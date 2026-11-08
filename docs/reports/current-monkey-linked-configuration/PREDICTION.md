# CURRENT-MONKEY-LINKED-CONFIGURATION prediction

Committed before the implementation, against base `a0ac2cae9`
(FLUENT-BIT-LINK-PREPARE's tip). Measurement side:
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`.

The change is in the `current` route's input selection and nowhere else.
`quirin-audit current --monkey-linked` selects Monkey's units from the run's
own `--compile-database`, which is the configuration linked into
`fluent-bit`, and labels them `monkey`. A `--monkey-database` given with it
is only accounted for. `prepare --run` passes both flags. The link step
(`prepare --link`) keys those units as route units. Without `--monkey-linked`
every route reads and writes what it did at base.

## Pinned routes

```measure-prediction
route corpus
none
route system
none
route slice
none
```

- **corpus** runs `quirin-audit pinned` (`pinned_route.rs`), which has its own
  Monkey selection (`monkey-libevent` from the pinned `monkey-build`) and
  does not read `--monkey-linked`. The helpers it shares with the current
  route (`is_monkey_unit`, `monkey_source_root`,
  `verify_monkey_source_copy`) are unchanged.
- **system** and **slice** run `current` with an explicit unit selection and
  no Monkey database or flag. Their path changes only by a refactor of the
  absent-unit refusal: the same text and the same category, through
  `unrecorded_refusal`.
- No profile, `audit.json`, selection, contract, provider or lock file
  changes.

## The user's `current` run (Fluent Bit 046accf6, no link map)

The run is the invocation `prepare --run` makes after this card: the user's
arguments plus `--monkey-linked`. Predicted per unit, from the census
([README](README.md#census)):

- **The 28 files in both databases.** Each `monkey-libevent/lib/monkey/<f>`
  unit is replaced by `monkey/lib/monkey/<f>`, analysed under the linked
  command. The report's `monkey_configuration` record lists the replacement.
  - The 20 units whose every function has equal preprocessed text (with only
    the event backend's declarations differing) and the 5 whole-unit
    token-equal units: every function keeps its verdict and leaves. The
    only change is the key. These are `mk_core/{mk_file,mk_iov,mk_memory,
    mk_rconf,mk_string,mk_utils}.c` and
    `mk_server/{mk_cache,mk_clock,mk_config,mk_fifo,mk_header,mk_http,
    mk_http_parser,mk_http_thread,mk_kernel,mk_mimetype,mk_net,mk_plugin,
    mk_scheduler,mk_socket,mk_stream,mk_user,mk_utils,mk_vhost,monkey}.c`.
    Reason: their bodies and their callees' declarations are the same text,
    their cross-unit callees go through the same contracts (pins are
    configuration-independent source identities), and the only differing
    declarations are `struct mk_event_ctx` and the epoll or Libevent headers,
    which these bodies do not access.
  - `mk_core/mk_event.c`: the 15 public `mk_event_*` wrappers have equal
    text, but they call the backend statics. The 11 backend statics
    (`_mk_event_{add,backend,channel_create,channel_destroy,del,inject,
    loop_create,loop_destroy,timeout_create,timeout_destroy,wait_2}`) are now
    the epoll bodies. Any of these 26 rows may change verdict, and each change
    comes from the linked configuration compiling the epoll backend, a
    different branch of `mk_event.c`'s `#if`. A row that becomes `Safe` does
    so only because of that branch, and the report will say so per function.
    The 6 Libevent-only functions (`_mk_event_update`,
    `_mk_event_wait_with_flags`, `cb_event`, `cb_timeout`,
    `cb_wait_2_timeout`, `mk_event_libevent_socketpair`) have no row: the
    linked configuration does not compile them.
  - `mk_server/mk_lib.c` (`mk_lib_worker`, `mk_start`) and
    `mk_server/mk_server.c` (`mk_server_loop_balancer`,
    `mk_server_worker_loop`): `mk_event_foreach` expands over the epoll
    context (`events[i].data.ptr`). These four rows may change for that reason
    alone. Their other functions keep their verdicts.
- **`tls/tls_stub.c`** (standalone only): it stays as
  `monkey-libevent/lib/monkey/tls/tls_stub.c`, now an `Unknown` unit with
  reason `not linked: …` (`environment-unavailable`). Its 5 functions
  (`mk_tls_{enabled,exit,init,thread_init,transport}`) move to `not_projected`.
  Any of them that was `Safe` at base becomes `Unknown`, which is the
  fail-closed direction.
- **`tls/tls_openssl.c`** (linked only): a new unit,
  `monkey/lib/monkey/tls/tls_openssl.c`, with 27 functions. Most call OpenSSL,
  for which no contract exists (external libraries are out of scope), so they
  are predicted not `Safe`: `Unknown` with `call to uncontracted function`.
- **Every other unit** keeps every row. The run's member pre-pass and rule
  31's reach now read the compile database alone, without the 29 standalone
  units added at base. 25 of those 29 have token-equal or function-equal text
  already in the database. What leaves the set is the Libevent backend text in
  `mk_event.c` and `tls_stub.c`, which name no member that a Fluent Bit,
  CMetrics, CTraces, librdkafka or ChunkIO unit invokes.
- **Unit totals.** Monkey's contribution changes from 29 units to 30
  (28 replaced, `tls_openssl.c` added, `tls_stub.c` kept as not-linked). With
  no link map, cross-unit imports are off, as at base. No callee leaves the
  uncontracted list because of a link, and no non-Monkey function leaves the
  "only uncontracted" set. The callees that change are Monkey's own: the
  re-keyed units call the same names.

The integrator measures this and reruns `current`. The base-binary probes
(`scripts/run_probes.sh`: the standalone Monkey run as the user reads it, and
the linked units through an explicit selection) are queued at commit time.
The report records their per-function comparison as measured evidence.
