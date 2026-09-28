# W47-LOOP-OWNER-TYPED-DISPATCH: frozen prediction

Frozen before any census, gateway, kernel or Lean change. Base: local `main`
at `27854776` (this worktree's HEAD was already there). BEFORE census
(`scripts/census.py` with the base example binary, SHA-256 `2ad0cf46…`)
reproduced W47-LOOP-OWNER-COROUTINE-HOP's archive byte for byte (all four
readings: `fixed` `e5d833fb…`, `previous` `f501c164…`, `fixed-configured`
`cecc2167…`, `previous-configured` `517ceca5…`).

Every statement reads *discharged / not discharged / unresolved at this path
under these derived premises*; none is a verified race or a verified fix.

## What the source says before any change

* The route switch between a direct collector call and the coroutine is
  `collector->instance->runs_in_coroutine` (`src/flb_input.c:3313`,
  `src/flb_input_thread.c:162`), written only in `flb_input_new`
  (`:501` false, `:625-626` true iff the *creating* plugin's `flags` has
  `FLB_INPUT_CORO`). The fixed test copies the instance's plugin after the
  instance exists (`test.plugin = *ins->p; … ins->p = &test.plugin;`,
  `tests/runtime/in_tail.c`), so the copy's `.flags` does not set the route:
  the instance's creating plugin does. The brief's Step B ("derive the
  coroutine flag of a runtime plugin copy from the data flow of the copy") is
  therefore necessary but not sufficient; this card derives the copy's flag as
  asked and additionally requires the instance's creation flag (conjunction,
  fail-closed).
* The test's instance is `mk_list_entry(ctx->flb->config->inputs.next, …)`,
  created by `flb_input(ctx->flb, "tail", NULL)` in `test_tail_ctx_create`,
  which returns a descriptor number, not the instance; and `&test` escapes
  into `ins->data`.
* `src/proxy/go/go.c:248` dispatches `plugin->cb_collect` of
  `struct flbgo_input_plugin`, whose only stores are
  `flb_plugin_proxy_symbol(…)` results (no function of the read files).

## Predicted site status (90978ac6~1 / 90978ac6; raw and configured alike unless noted)

| Sites | BEFORE | after Step A (type-keyed) | after Step B (instance flag) |
|---|---|---|---|
| fixed test `cb_tail_pause_test` ×3 (`:2483`, `:2492`, `:2494`; absent at `~1`) | unresolved (own post-yield + Go proxy) | **unresolved**, the Go proxy's reason gone; the own post-yield reason stays through `flb_input_plugin.cb_pause` (every input plugin's pause is still a registrant of the same type, in_emitter's included) (~60%; ~40% that no yield stays reachable and the sites discharge on Step A alone) | **unresolved** (~90%): the instance flag is underived (the list-head instance is not tied to its creation; `&test` escapes) |
| in_tail plugin collector ops ×16 | not discharged (threaded test's T) | not discharged (same type, same path) (95%) | unchanged |
| NC3 `wait_tail_collectors_state` ×2 | not discharged (T) | not discharged (99%) | unchanged |
| previous test's own `running` reads ×6 (`~1` only) | not discharged (T) | not discharged (99%) | unchanged |
| Go-proxy-held ×8 (in_docker_events ×4, in_event_test `:124`, `:163`, in_fluentbit_metrics, in_stdin) | unresolved (only the proxy's reason) | **discharged** (~65%); otherwise unresolved on a reason other than the proxy's `go.c:248` dispatch | unchanged |
| in_calyptia_fleet ×3, in_event_test `:188`, `:218` | unresolved (own + proxy) | unresolved on their own yields only (85%) | unchanged |
| in_podman_metrics ×2 | unresolved (no derived caller) | unchanged | unchanged |
| other-plugin sites (105 raw / 109 configured) | not discharged (T via field-keyed V) | at least 95 of 105 stay not discharged (the threaded test's `flb_input_pause > flb_input_plugin_pause > ins->p->cb_pause` is typed `struct flb_input_plugin`, whose registrants are every input plugin's callbacks); at most ~10 move, each through a cross-type field | unchanged |
| `src/` ×3 | not discharged | unchanged | unchanged |

Census totals: edges fall (cross-type edges removed); unresolved seeds may
rise (dispatches through a cast or a union, stores with a conversion, now
refused as unmatched) — size not predicted. Owners unchanged
(`src/flb_lib.c::flb_lib_worker`, `src/flb_input_thread.c::input_thread`).

## Corpus, system and slice

The per-unit route reads no census: the 185-unit corpus, the system slice
at `046accf6` and the tail-pause slice at the `90978ac6` export are
**byte-identical to BEFORE** (and compose with the W47-LOOP-OWNER-CONTRACTS
archive with 0 mismatches). No Safe → non-Safe correction, no leak or
violation added or removed (99%).

## Negative controls (fixtures first)

| Control | Predicted outcome |
|---|---|
| two struct types with the same field name | a call through one never reaches the other's registrant |
| a dispatch through a cast base / a union field; a store with a conversion | unresolved |
| a file with no typed reading | the pre-card field-keyed reading (never narrower) |
| a plugin copy whose `.flags` is written with `FLB_INPUT_CORO` | coroutine route kept |
| a copy passed to an unmodelled function | flag underived, route kept |
| fixed-test shape (copy of `*ins->p` on the list-head instance) | unresolved while a yield is reachable |
| the same shape on an instance created from a derived non-coroutine descriptor | discharged only when both the copy's and the instance's flags derive no coroutine |
