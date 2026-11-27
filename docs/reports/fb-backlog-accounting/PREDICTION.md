# FB-BACKLOG-ACCOUNTING prediction

Committed after the probe (`9d2ae0076`) and before the semantic change.
Base: `76c1f9cfc` (main, r77).

**What changes.** Gateway rule 92 adds a ledger checker,
`crates/qcore-c-gateway/src/ledger_transition.rs`, a consumer of the
typed-source route (T) with its own node admission. It reads a function's
paths and certifies each write to a named unsigned ledger field as one of
four saturating transitions (guarded subtraction, its clamp to zero, guarded
addition, its clamp to the maximum), optionally paired one-to-one with the
list link/unlink calls on the same path. Anything else is refused
(`Unknown`). Lean: `QCore.BacklogLedger`. The only shared-file edit is one
`pub mod` line in the crate root.

**Affected rows at the base: none.** The checker is not called by the custody
gateway (`legacy_c`), by `quirin-audit current` or by any profile route. No
provider contract, lock, module list or load path changes. The census (README
§1) shows the custody route already reads every ledger statement as
custody-neutral, so accounting facts cannot move a custody row in either
direction: no `Unknown -> Safe`, no `Safe -> Unknown`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current --monkey-linked` run** (Fluent Bit `046accf6`), base
binary against tip binary, each from its own `git archive` tree: function
rows and unit statuses identical, so no moved row. A difference is a
measurement finding, not an effect of this card.
