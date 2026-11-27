# CELL-PROTOCOL-CLOSE-ACTUAL prediction

Committed before any implementation commit, on base `aa55f2235` (the tip of
MK-EVENT-CELL-CROSS-FUNCTION, rule 38c). The rows were relisted at
`baseline:0ac169180f549406ec2f4aef9effb7ad2eb63df0` (the main checkout's
`private/baselines/`). Rule 38c, between that baseline and this base, is
predicted to move no row; the integrator confirms that.

> **USER DECISION 2026-11-02** (verbatim, relayed by the coordinator): "B as
> recommended". Its Q3: `close` / `mk_event_closesocket` of a declared cell
> member is that cell's `#stale`.

## The rule this predicts

The access gate (W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 1) refuses an
integer member actual at `close`'s custody position. The card admits exactly
one more actual there: an integer member read `r->CELL` (or `r->CELL[K]`) of
a declared protocol record's one-segment cell, at a descriptor-kind release
position of a function-map callee, in a statement that is exactly rule 38c's
close. Its custody effect is rule 38c's close (`#stale`). An admitted close
that rule 38c does not decide refuses.

**What can move.** Only a statement the gate already refused that is
`close(r->CELL);` with `r` declared a pointer to a record whose `quant cell`
protocol names `CELL`. Every hook is behind rule 38c's parser reading, which
emits nothing in an environment that declares no `quant cell` clause. No
production declaration carries one at this base (rule 38c's own prediction);
production protocols arrive only with CELL-PROTOCOL-FB-DECLARATIONS. So on
its own this card moves no row on any route.

## The rows whose leaves include the gate refusal this card narrows

Every `indirect memory access is outside the resource projection` leaf of
every row was read at its origin line in the analysed sources (corpus
`build/cycle19/c-gateway-wave-pin`, system
`private/w45-fb/w46-external-callee-contracts/attempt-1`, slice
`private/w47-tail/attempt-1/prep`), keeping those whose statement closes a
member read (`close`, `mk_event_closesocket`, `closesocket`,
`evutil_closesocket` of `X->…`). The reader is a scratch script; 98 corpus
origins did not resolve to a unique file and 28 resolved to several, so the
corpus list is a lower bound. That does not affect the prediction: no
profile declares a protocol.

| Route | Row (×units) | Leaf (origin: statement) | Rule 38d shape |
|---|---|---|---|
| system | `flb_engine_failed` (×2) | flb_engine.c:1064 `mk_event_closesocket(config->ch_notif[1]);` | admitted shape once `flb_config` declares `ch_notif`; the guard `[1] != [0]` is no protocol test, so the gate's refusal stays |
| system | `flb_output_instance_destroy` (×2) | flb_output.c:616, :620 `mk_event_closesocket(ins->ch_events[0|1]);` | admitted shape once `flb_output_instance` declares a `> 0` protocol |
| slice | `flb_input_instance_destroy` (×2) | flb_input.c:1312, :1316 `ch_events[0|1]`; :1320, :1324 `ingress_queue_channels[0|1]` | admitted shape for each declared record cell |
| slice | `input_thread_instance_destroy` (×2) | flb_input_thread.c:211, :214 `ch_parent_events[0|1]`; :219, :222 `ch_thread_events[0|1]` | admitted shape once `flb_input_thread_instance` declares them |
| slice | `collector_start` (×2) | flb_input.c:2368 `mk_event_closesocket(coll->fd_event);` | a cell only if `flb_input_collector.fd_event` is declared one |
| slice | `flb_tail_file_remove` | tail_file.c:2049 `close(file->fd);` | a cell only if declared |
| slice | `flb_tail_fs_inotify_exit` | tail_fs_inotify.c:502 `return close(ctx->fd_notify);` | never: a used result is no rule-38c close |
| corpus | `cio_file_native_close` | cio_file_unix.c:428 `result = close(cf->fd);` | never: a used result |
| corpus | `mk_fifo_worker_destroy_all` | mk_fifo.c:264, :265 `close(fw->channel[0|1]);` | a cell only if declared (no corpus declaration) |
| corpus | `mk_server_listen_exit` | mk_server.c:161 `mk_event_closesocket(listen->server_fd);` | a cell only if declared |
| corpus | `mk_vhost_fdt_close` | mk_vhost.c:255, :265, :279, :285 `return close(sr->in_file.fd);` | never: multi-segment, used result |
| corpus | `mk_sched_add_connection`, `mk_sched_remove_client`, `mk_server_listen_handler` | `X->network->close(…)` / `conn->net->close(…)` | never: an indirect call |

- **No `Safe` move.** No row reads a protocol at this card's tip.
- **No `Unsafe` move.** Neither rule 38c nor 38d accuses.
- **No move away from `Safe` or `Pending`.** No `Safe` row carries a gate
  refusal, and the identity environment changes no lowering.

Provider locks, contract digests and the bounds identity are unchanged: no
provider file changes. The one new serialized field (`admitted` on rule 38c's
close step) is omitted when false and appears only where a protocol is
declared, so serialized instructions are byte-identical on every route.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
