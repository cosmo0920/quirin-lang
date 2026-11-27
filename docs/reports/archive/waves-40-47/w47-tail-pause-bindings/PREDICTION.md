# W47-TAIL-PAUSE-BINDINGS: prediction

This card changes no analyzer, contract, provider or Lean code, so the only
figures it can move are the new slice's own. The prediction is the brief's
stated expectation, recorded before the slice ran; this file itself was written
after the run and is therefore not an independent freeze. It is kept so the
outcome can be read against the expectation.

1. **185-unit corpus and system slice: byte-identical by construction.** No
   file under `crates/`, `compiler/`, `providers/`, `proof/` or the
   `fluent-bit` profile's `audit.json`/`profile.json` changes. Base `3aed6150`
   is the round-3 baseline. The brief waives a corpus re-run, and none was made.
2. **Tail-pause slice at `90978ac6`:** no function on a concurrency path is
   Safe on any concurrency ground, because the resource analysis claims no
   concurrency property. Functions with no resource operations, such as small
   getters, may still be resource-Safe. Every such Safe is a custody statement
   only.
3. **Test file:** `tests/runtime/in_tail.c` is expected to be outside the
   recorded route, because FLB_DEV does not enable FLB_TESTS_RUNTIME.

Outcome (README §4):

1. Held by construction, not re-measured.
2. Held. Every bound function in a selected file is Unknown. The 24 Safe
   functions are getters and pure helpers (listed in §4). The one Safe function
   that reads a cross-thread field, `flb_loop` (it polls `ctx->status`), is Safe
   on custody grounds only. One provisional Unsafe, `flb_tail_file_budget_destroy`,
   was not in the expectation. It is classified as an artifact (§4.2).
3. Held. A control selection that includes the file gives one Unknown
   `environment-unavailable` unit.
