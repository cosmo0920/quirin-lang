# LINK-RECORD-KILL-POLICY-SCALAR-CALLS: prediction

Frozen before any tip measurement. Base `fb4adb697` (r83).

**Disclosure.** The gateway change (rule 46's kill policy in
`legacy_link_member.rs`, the classification hook recording the scalar
actuals, the pre-call nested kill in `legacy_transfer.rs`) was drafted in
the worktree before this file was committed and run only on the card's
fixtures (`link_member_premises.rs`, module `carried_link_obligation`;
base verdicts pinned by the probe commit `c03e9b792`): the five scalar-only
fixtures move Unknown -> Safe there and the nine near misses stay Unknown.
The base sides of the fluent-bit-system route (`private/m/sys-base`:
116 / 0 / 328 / 4) and of r83's census units (`private/m/census-base`:
34 / 1 / 165 / 0) were measured; the base opt-in current run was started.
Nothing was measured at tip.

## Rows at base

Every row of either base report carrying a rule 46 or rule 91 link leaf
(`unlink by D of E, which this function has not published` / `… holds, but
…`) was read with its leaves. Only one has a contracted call whose actuals
are literals or a scalar errno dereference between its carried record and
the destroyer:

| Function | Base leaves | Expected at tip |
|---|---|---|
| `flb_output_task_queue_enqueue` (`flb_output.c`, both system configurations) | `unresolved resource identity retry` (twice) and the held refusal at `flb_task_retry_destroy` | **Unknown -> Safe**: `flb_errno()` is `flb_errno_print((*__errno_location ()), "…", LINE)`, every actual a scalar (an admitted N4-D/N10 dereference of the neutral `__errno_location`, a string literal, an integer literal), so the carried `Ready(_head)` record reaches the destroyer and r83's exit-class summary validates |
| `flb_output_task_singleplex_enqueue` (caller) | Unsafe on its own `resource_user_exit(task, -1, restored)` refutation, `call to uncontracted function flb_output_task_queue_enqueue` | leaf trade only (the call now meets the enqueue's summary, whose precondition its own formal `retry` does not discharge); stays Unsafe |
| every other link-leaf row (scheduler, task, engine, input chunk, output instance, `flb_input.c`, `in_tail`) | an uncontracted call, an indirect access, a plain `mk_list_del` of an uncarried formal, or an unresolved identity before or at the unlink | unchanged |

## Predicted moves

* system: `flb_output_task_queue_enqueue` Unknown -> Safe in both
  configurations (Safe +2, Unknown -2); a leaf trade in
  `flb_output_task_singleplex_enqueue`, status unchanged.
* census units: none.
* opt-in current: none (no unit it reads has a link record alive across a
  scalar-only call before a destroyer, to this reading; the refinement is
  general, so any row it moves is reported and justified).
* No Safe -> Unknown; no leak, double release or Unsafe added or removed.

```measure-prediction
route system
move flb_output_task_queue_enqueue Unknown -> Safe
delta Safe +2
delta Unknown -2
route slice
none
route corpus
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have; the card records the refusal.
