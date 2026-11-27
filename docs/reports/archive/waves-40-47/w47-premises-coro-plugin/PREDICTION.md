# W47-PREMISES-CORO-PLUGIN: frozen prediction

Frozen after BEFORE was measured and before any Rust, Lean, census AFTER,
arbiter or contract change. Base: local `main` at `fe603d49` (= the
worktree's HEAD at start). Only the Fluent Bit export was read (grep and
`sed` over the prepared tree `private/w47-tail/attempt-1/prep/`), to find the
writers and resume sites named below; no derivation had run.

## BEFORE (measured)

- Base binaries (release): `loop_owner_census` `3c2e0d28…`,
  `typed_dispatch_facts` `4b7655a4…`, `predefined_macro_facts` `9923a0cc…`.
- Typed facts regenerated: `library.json` `75f252d7…`, `fixed-test.json`
  `35dabc49…` (byte-identical to W47-SERVICE-PROPERTY-KEYS's manifest);
  predefined facts `9dd8bb73…` (both, byte-identical to the
  W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS manifest).
- All six census readings byte-identical to the archive of
  W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: fixed configured `719a7d0d…`
  (11 / 130 / 7), previous configured `418a9251…` (8 / 136 / 7), fixed raw
  `983c74b9…` (8 / 126 / 10), previous raw `d05ebc0e…` (8 / 132 / 7), and the
  two controls `4955a12d…`, `a5edf1ed…`.
- The 11 discharged sites at the fixed configured reading: the fixed test's 3
  (`tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) and 8 plugin sites
  (in_docker_events `:87`, `:125`, `:159`, `:197`, in_event_test `:124`,
  `:163`, in_fluentbit_metrics `:82`, in_stdin `:272`), the 8 discharged only
  because their directories' descriptors derive no coroutine (P-PLUGIN-DIR).

## What each premise holds up (read, not measured)

- P-CORO-HANDLE: the hop-C resumers of `input_pre_cb_collect`, and the
  soundness of dropping the coroutine route (the entry reads its collector
  from `params->coll`, written by `input_params_set` just before its first
  switch). All 11 discharges rest on it through the route drop.
- P-PLUGIN-DIR: (a) descriptor flags fixed at run time: the fixed test's 3
  through `descriptors_flag("tail")` and the 8 through their directories;
  (b) registration on the plugin's own instances: the 8 only.
- The immutable threaded flag: every discharge (hop L / hop F fix the mode of
  the loop a collector was attached to at its creation).

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| P-CORO-HANDLE | derived at both revisions except the handle a running coroutine takes of itself (`flb_coro_get()`, `co_active()`), narrowed to a named residual: the input coroutine's `struct flb_coro` is read only in the hop's constructor, wrapper, destructor and entry, `params->coll`/`coro` only in `input_params_set` and the entry, `input_coro_list` only initialised and added to | 70% |
| P-PLUGIN-DIR (a) | derived: the only runtime write of a `struct flb_input_plugin`'s `flags` is the Go proxy's, to its own fresh allocation before it is published (`src/flb_plugin_proxy.c`); whole-object writes stay a named residual | 65% |
| P-PLUGIN-DIR (b) | derived for the registrants holding the 8 plugin sites (registration on a parameter chained to the directory's `cb_init` or to its own collector callbacks) | 50%; if not, those 8 move discharged → unresolved and are named as corrections |
| immutable threaded flag | writers enumerated as `flb_input_new` (constructor), `flb_input_set_property`'s `threaded` arm and the chunk-trace emitter's fresh instance; narrowed to a residual over the property-setter calls whose key is not a literal (`src/flb_lib.c:328`, `src/flb_config.c:1384`, `:1388`, `plugins/custom_calyptia/calyptia.c:264`) | 60% |
| fixed test ×3, configured `90978ac6` | stay **discharged at this path** under the derived premises plus the narrowed residuals | 60% |
| old test ×8, `90978ac6~1` | stay **not discharged** (`T Either`) | 97% |
| other sites | no status move except as named above | 65% |
| raw readings | move only as the configured ones do (they carry the typed facts) | 60% |
| negative controls (a foreign resume, a list resumed from, a flags write at run time, a threaded flag toggled after start, a registration on another plugin's instance) | each makes the dependent fixture sites lose their discharge | 90% |
| corpus / system / slice | byte-identical sidecars (the census is on no audit route); if a held contract is added for a residual, still unchanged | 90% |
| condvar/lifecycle census | untouched | 95% |
