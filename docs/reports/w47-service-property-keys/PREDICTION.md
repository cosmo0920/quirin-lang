# W47-SERVICE-PROPERTY-KEYS: frozen prediction

Frozen before any census, typed-reading, contract or Lean change. Base:
local `main` at `ec4c9d4a` (the worktree's HEAD was already there).

BEFORE is measured: the base binaries (`loop_owner_census` `6966df12…`,
`typed_dispatch_facts` `7088ace3…`) regenerate the typed facts
(`library.json` `165363ca…`, `fixed-test.json` `9349fbb1…`, identical to the
archive; `previous-test.json` differs only by its scratch path) and reproduce
the four census readings of
`docs/reports/w47-loop-owner-instance-flow/artifacts/census/` byte for byte
(`6ea187e4…`, `d6475e38…`, `d05ebc0e…`, `a5edf1ed…`).

## Planned mechanism

Step B's list-head region reads a variadic property setter's call
(`flb_service_set(ctx, K1, V1, …, NULL)`) as reaching only the effects of its
keys when: the typed reading records one argument list for the call, every
key is an ASCII string literal, the list is NULL-terminated with an even
number of key/value arguments; the setter's body reads its variadic list in
key-first pairs and passes each key to its property function; the key table
(`service_configs[]`) and the property function's keyed branch chain
(`flb_config_set_property`) are read from the source (keys and types from
Clang, fields from the configured lexical reading, cross-checked entry by
entry); and every key matches at least one table entry under the comparison
`prop_key_check` makes (read from its body). Each key's effect is its
branch's calls (the scalar-store arm, the `Log_Level`, `Parsers_File` or
`Plugins_File` handler), plus everything outside the branch chain. Anything
else keeps every branch (the whole call, as today).

## Predictions

| Measure | Prediction |
|---|---|
| Key table at the export | 53 keys + the NULL terminator (the build defines `FLB_HAVE_HTTP_SERVER`, `FLB_HAVE_STREAM_PROCESSOR`, `FLB_HAVE_CHUNK_TRACE`, not `FLB_SYSTEM_WINDOWS`); compared ignoring ASCII case over the whole key; three handler keys (`Log_Level`, `Parsers_File`, `Plugins_File`), every other key a scalar store into a `struct flb_config` field; only `Plugins_File` reaches the registration hazard (90%) |
| `90978ac6` configured, fixed test's 3 sites | the `flb_service_set` creation blocker is gone (the test's keys `Flush`, `Grace`, `Log_Level`, `Parsers_File` reach no hazard) (85%). Final status: **unresolved** with a narrower blocker (70%): the `Parsers_File` handler's closure still holds `flb_cf_yaml.c::dirname` (Windows-only, no typed reading), so the unmodelled-call blocker stays; **discharged** (20%); unresolved for another newly surfaced reason (10%) |
| `90978ac6` raw, fixed test's 3 sites | **unresolved**, first reason unchanged: `flb_config.c` is unstructured unconfigured, so the key reading refuses and keeps every branch (90%) |
| `90978ac6~1` (old test, negative control), both readings | **not discharged**; census JSON byte-identical (no plugin copy, Step B does not apply) (90%) |
| Every other site (in_tail's 16, NC3's 2, other plugins) | verdict and first reason unchanged at both revisions; no other site's derivation passes a property setter (95%) |
| Edges / seeds / functions | unchanged at all four readings (95%) |
| `flb_input_set` (:2549), `flb_output_set`, `flb_filter_set` | do not reach the registration hazard in the census graph at the export, so no key reading is needed for them (80%) |
| Negative controls (fixtures) | literal `"Plugins_File"` (and a case variant) keeps the hazard; a key from a variable, an odd list, a missing NULL, an unknown key all keep every branch |
| Corpus (185 units, `66910c10`) | `resource-diagnostics.json` byte-identical; only the contract sidecar and console tally lines name the new/changed declaration (90%) |
| System (`046accf6`) | 31 / 2 / 201 unchanged (90%) |
| Tail-pause slice (`90978ac6`) | 24 / 1 / 244 / 1 Pending unchanged (85%); `flb_service_set`'s `:691` statement may gain or reword one leaf (a held `flb_config_set_property`), never Safe (30%) |
| Follow-up `W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS` | retired only if the 3 sites discharge; otherwise amended with the narrower blocker (70% amended) |
