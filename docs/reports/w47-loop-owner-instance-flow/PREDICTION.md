# W47-LOOP-OWNER-INSTANCE-FLOW: frozen prediction

Frozen before any census, gateway, kernel, contract or Lean change. Base:
local `main` at `fefb9199` (this worktree's HEAD was already there). BEFORE
census (`../w47-loop-owner-typed-dispatch/scripts/census.py` with the base
example binary `a99c62a9…`, the archived typed facts) reproduced
W47-LOOP-OWNER-TYPED-DISPATCH's archive byte for byte (all four readings:
`fixed` `07f3f74e…`, `previous` `d05ebc0e…`, `fixed-configured` `e23cc7e8…`,
`previous-configured` `a5edf1ed…`).

Every statement reads *discharged / not discharged / unresolved at this path
under these derived premises*; none is a verified race, bug or fix.

## What the source says before any change (read at the `90978ac6` export)

* `flb_input(ctx, name, data)` (`src/flb_lib.c:268`) calls
  `flb_input_new(ctx->config, name, data, FLB_TRUE)` once and returns the
  instance's id. `flb_input_new` (`src/flb_input.c:326`) creates an instance
  for **every** registered plugin whose name matches `name` (the loop over
  `config->in_plugins` has no `break` after a creation; `check_protocol`,
  `:244`, is an exact case-insensitive comparison), appends each with
  `mk_list_add(&instance->_head, &config->inputs)` (`:677`), sets
  `instance->p = plugin` (`:490`), `instance->data = data` (`:498`) and
  `runs_in_coroutine` from the plugin's `FLB_INPUT_CORO` (`:501`, `:625-626`).
* The fixed test (`flb_test_inotify_pause_collectors`, `tests/runtime/in_tail.c`)
  reads `head = ctx->flb->config->inputs.next` and
  `ins = mk_list_entry(head, …)` (`:2551-2552`) **before** `flb_start`.
  Between the context's creation (`flb_create` in `test_tail_ctx_create`) and
  that read the test calls `flb_service_set`, `flb_input(…, "tail", NULL)`,
  `flb_output(…, "lib", …)`, `flb_input_set` and libc/acutest functions; the
  failure paths call `flb_destroy`. A lexical census closure (exploration
  only) found `flb_input_new` reachable from `flb_input` alone among these.
* `cb_tail_pause_test_pre_run` is dispatched through
  `struct flb_input_plugin::cb_pre_run` as `ins->p->cb_pre_run(ins, …)`
  (`src/flb_input.c:2046`), `p->cb_pre_run(ins, …)` after `p = ins->p`
  (`src/flb_input_thread.c:335`, `:409`), and by itself through
  `test->original_plugin->cb_pre_run(ins, …)` on its own first parameter.
  `collector_create` stores `coll->instance = ins` (`:2246`) and the route
  switch reads `collector->instance->runs_in_coroutine` (`:3313`).
* `&test` escapes into `ins->data` (`:2557`); `struct tail_pause_test` is
  declared in the test file only.

## Mechanism to be built

A new module (`crates/qcore-c-gateway/src/loop_owner_instance.rs`) with thin
hooks in Step B: (1) the list-head instance (single creation on every path from
the test's entry to the read, the read list being the one of the context the
path created; creators reached through the census graph plus a typed-refined
escape set; list mutations, registrations and context re-pointing read from new
typed member-access facts); (2) the holder escape (the holder's type named only
in the test file, no write of the copy member outside the modelled function,
instance `p` and `runs_in_coroutine` writers program-wide); (3)
P-CALLBACK-INSTANCE derived at every dispatch of the copy's callback field and
for `coll->instance`. Any unmodelled list mutation, second creator, dynamic
plugin name, underived context, or escaping writer leaves the flag underived
and the coroutine route kept.

## Predicted site status (90978ac6~1 / 90978ac6; raw and configured alike)

| Sites | BEFORE | AFTER |
|---|---|---|
| fixed test `cb_tail_pause_test` ×3 (`:2483`, `:2492`, `:2494`) | unresolved (coroutine route not dropped: instance flag underived) | **discharged** on `(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)` (~55%); otherwise unresolved with a narrower named blocker (a hazard in the `flb_create`/`flb_destroy` closure, an unmatched typed access of a listed field, or a dispatch shape) (~45%) |
| previous test's paths (`~1` only), NC1 | not discharged (T) | **not discharged** (99%): its collector calls are on the test thread; it has no plugin copy |
| in_tail ×16, NC3 ×2, other-plugin refusals (130 configured at `90978ac6`, 136 at `~1`) | not discharged | unchanged (99%) |
| Go-proxy-held ×8 | discharged | unchanged (99%) |
| in_calyptia_fleet ×3, in_event_test `:188`, `:218` | unresolved (own yields) | unchanged (99%) |
| in_podman_metrics ×2 | unresolved (no derived caller) | unchanged (99%) |
| `flb_proxy_input_cb_collect`, `flb_input_ingress_collector` instance flags | underived | underived, same reasons (95%) |

Census totals: edges and unresolved seeds unchanged (the new facts are read
only by Step B) (90%). The descriptor-name match becomes case-insensitive
(`check_protocol`); no descriptor at the export differs from `"tail"` only by
case, so nothing moves by it (95%).

## Corpus, system and slice

The per-unit routes read no census. The integrator's added step (record the
facts as provider contracts, checked against their bodies, pinned; uncheckable
clauses under W45-CONTRACT-DEFERRED-REVIEW) can move them: a pending
declaration holds every call site of its callee. Predicted: a declaration
whose callee an analysed unit calls turns that call from uncontracted to held
(`awaiting-human-review`); a function whose only unresolved construct was that
call moves `Unknown → Pending` (never to `Safe`, never to `Unsafe`); every
other row keeps its verdict and gains the held count in its summary text.
Declarations chosen to hold only callees with no or few analysed callers.
Without the contract step: corpus, system and slice byte-identical to BEFORE
(99%). No Safe → non-Safe correction; no leak or violation added or removed
(95%).

## Negative controls (fixtures first)

| Control | Predicted outcome |
|---|---|
| two inputs created before the read (head ambiguous) | unresolved (route kept) |
| an input created by a dynamic (non-literal) name | unresolved |
| a holder that writes `runs_in_coroutine` | unresolved (a second writer) |
| a holder that re-points `ins->p` | unresolved (a second `p` writer) |
| a coroutine-flagged descriptor at the head | route kept (flag derived coroutine) |
| the old test's shape | not discharged (T) |
| the fixed shape | discharged only when every derivation holds |
| a creation inside a loop; a creator reached through an escaped function; a list mutation in the closure | unresolved |
