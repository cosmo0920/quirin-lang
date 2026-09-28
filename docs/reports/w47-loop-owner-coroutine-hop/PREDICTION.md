# W47-LOOP-OWNER-COROUTINE-HOP: frozen prediction

Frozen before any census, kernel or Lean change. Base: local `main` at
`51fb793c` (W47-LOOP-OWNER-CONTRACTS landed). BEFORE census reproduced
byte for byte from `docs/reports/w47-loop-owner-contracts/scripts/census.sh`
(`fixed.json` SHA-256 `76f75dc9…`, `previous.json` `1f02be39…`, both equal
to the archived `artifacts/census/*.json.gz`).

Every statement below is "discharged / not discharged / unresolved at this
path under these derived premises"; nothing is a verified race or fix.

## What the two blocking reasons are (read before predicting)

1. **"`include/fluent-bit/flb_input.h`: a conditional group this reader cannot
   structure."** Diagnosed by reading the census scan, not by running a
   changed census: an include guard's closing `#endif` stands after the last
   token of the file, at `token_index == tokens.len()`, which the scan's token
   loop never reaches, so the guard's frame is left open and the file is read
   as unstructured. 919 of the 1,685 read files end in such an `#endif`; every
   one that defines a function carries the reason (221 files at `90978ac6`).
   `flb_input.h`'s own groups (`FLB_HAVE_METRICS`, `FLB_HAVE_CHUNK_TRACE`,
   `FLB_HAVE_TLS`, `FLB_HAVE_VALGRIND`) all end at their entry depth. After the
   repair about 11 files stay genuinely unstructured (arms that end at
   different depths, e.g. `plugins/in_tail/tail_file.c`, `src/flb_config.c`).
   The configured reading (the pinned build's `-D` set from its
   `compile_commands.json`; a group is narrowed only where every arm's
   condition is decided) is then applied on top; it is not needed for
   `flb_input.h`, whose `FLB_HAVE_VALGRIND` arm the build takes.
2. **"address of `input_pre_cb_collect` taken at `flb_input.h:721`."** The
   coroutine constructor `flb_input_coro_collect` passes the entry to
   `co_create`. The hop to derive: the entry's pre-yield body runs on the
   thread that constructs the coroutine and resumes the handle it returns
   (`flb_input_collector_fd` on the engine loop, `input_collector_fd` on the
   worker loop, each `v = flb_input_coro_collect(...); flb_input_coro_resume(v);`),
   and its post-yield continuation on a resumer the census cannot name
   (`flb_coro_resume(connection->coroutine)` and the other generic resume sites
   take a handle from `flb_coro_get()`). So a dispatched callback that may reach
   a non-terminal yield stays unresolved; one that cannot runs only on the
   constructing dispatcher. Which inputs run in a coroutine at all is read from
   the `.flags` of the `struct flb_input_plugin` descriptors: 8 plugin
   directories carry `FLB_INPUT_CORO` (in_health, in_prometheus_textfile,
   in_event_type, in_nginx_exporter_metrics, in_prometheus_scrape,
   in_event_test, in_calyptia_fleet, in_kubernetes_events); in_tail's is `0`.
   A collector callback registered outside a plugin directory (the fixed test's
   `cb_tail_pause_test`, registered in `tests/runtime/in_tail.c` on the
   instance of a runtime copy of the tail plugin) has no derived flag.

## Census (`90978ac6~1` / `90978ac6`)

| Sites | BEFORE | Predicted AFTER |
|---|---|---|
| fixed test `cb_tail_pause_test` `:2483`, `:2492`, `:2494` (absent at `~1`) | unresolved (the two reasons) | **unresolved**, both old reasons gone; the one remaining reason is that the callback may reach a non-terminal yield through field-keyed `cb_pause`/`cb_resume` dispatch (`flb_input_pause` > `flb_input_plugin_pause` > …) while its instance's coroutine flag is not derived, so its post-yield resumer is not derived. Discharged instead only if no yield is reachable from it (predicted not; ~70%). Every derived context stays an owner context. |
| in_docker_events (4), in_fluentbit_metrics (1), in_stdin (1) | unresolved | **discharged** at both revisions: no `FLB_INPUT_CORO` in their descriptors, so the coroutine dispatch no longer reaches them, and the conditional-group reason is gone; their remaining contexts are the two loop dispatchers' owners |
| in_calyptia_fleet (3), in_event_test (4) (`FLB_INPUT_CORO`) | unresolved | **unresolved** at both revisions: they run in a coroutine and may yield (calyptia's HTTP client; event_test through field-keyed dispatch), so the post-yield resumer is not derived |
| in_podman_metrics (2) | unresolved (no derived caller) | unchanged |
| not-discharged sites (132 at `~1`, 126 at `90978ac6`) | not discharged | **not discharged**, no status move expected; witnesses may change. Contexts can only shrink through the repair (an unresolved spawner no longer roots its entry in every mode); any site that moves out of not-discharged is attributed to that rule |

Old-vs-fixed after the card: the previous test's NC1 path stays not
discharged; the fixed test's path stays unresolved at this path, now only on
the post-yield resumer of a coroutine its instance is not derived to run.

## Corpus, system, tail-pause slice

Unchanged, byte for byte: the per-unit routes read no census. The one shared
edit is a side channel on the tokenizer's conditional marks (the directive's
byte span), which leaves the token stream and every existing consumer
unchanged. Corpus 526 / 10 / 4,298 (units 6 / 8 / 167 / 4, leak 36,
unknown_alias 7,936, unsupported 35,737); system unique 31 / 2 / 201;
slice units 0 / 1 / 15, functions 24 / 1 / 245 / 1. No leak or violation added
or removed.

## Lean

`QCore.LoopOwner` gains the coroutine hop: a coroutine body's contexts are its
resumers' contexts (the thread is kept), a coroutine resumed only on `owner L`
runs on `owner L`, an undetermined resumer never discharges, one non-owner
resumer refuses; an input whose derived flag says it runs no coroutine
receives nothing through the hop; and a configured reading keeps every arm a
build consistent with the known macros can take, dropping only an arm it
cannot. Registered in `AuditLoopOwner.lean` (no new audit module).
