# CONNECT-LIBC-EFFECT-CLAIMS: prediction (Option A)

Base `9903b027a`. USER DECISION (2026-11-27, verbatim): "Option A". These are
predictions for the integrator to measure; none of them has been measured.

| Route | Verdicts | Coverage | Reason |
|---|---|---|---|
| corpus | no move | no move | Neither changed libevent module (`monkey/callback_task_handoff.qv`, `monkey/timeout_teardown.qv`) is in libevent's resource, bounds or outer view, and the new `libc/callee_effects.qv` is in no view. No resource-v1 declaration, pin or link changed. |
| twelve-home | no move | no move | same |
| system | no move | no move | same. The pinned route reports each Monkey callback's Cycle 18 composition as `Unknown` with a reason, without running the callback task-handoff projection. |
| slice | no move | no move | same |

**Expected identity changes, none of them verdict moves.**

- The load-path manifest (`providers/contract-load-path.json`) gains one libc
  module.
- Both crates' `contract-inventory.lock` files change, and libevent's
  `contracts.json` records two new module digests.
- A route whose input identity binds the whole manifest or those files records
  a new input identity.
- No `contracts.lock`, `linkage-reliances.json` or `contract_dependencies`
  entry changed, so `measure chain`'s dependency graph is unchanged.

**The `quirin-audit callback-task-handoff` audit**, which is not a pinned route:

- Its result is expected to be unchanged ("bounded obligations checked") for a
  configuration that names the reviewed libc module and lock.
- Its `policy_digest` changes because the libevent policy text changed.
- The report gains `libc_effects_module` and `libc_effects_digest`.
- A schema-1 configuration now refuses (Unknown) until it names the two new
  inputs.
- The taskful fixture audit (`callback-task-handoff-fixture`) reads the same
  policy. Its frozen run manifest
  (`docs/reports/cycle18-task-handoff/taskful-run-manifest.json`, policy
  `be5964f3…`) already differed from the base's policy (`f988c81e…`), so
  re-running it needs a fresh native run, as it did before this card.
