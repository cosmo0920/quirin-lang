# BODY-SUMMARY-REUSE prediction

Committed before any analyzer change, against base
`13337d0af358bd7a02ad2f5509b6a9512327c4aa` (origin/main). The measurement's
base side is `rev:13337d0af`.

The card's first step is a census (scripts, report and an experiment-only
link map under this directory). Its second step, the reading, is gated on a
semantic choice the census exposes (see the report's "Stop" section): every
reachable site needs either a linked-program resolution premise on the
`current` route or a declared contract, and neither is settled by
fail-closed. Until the user decides, the card changes no analyzer, contract,
provider or Lean file, so no row moves on any live route.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The user's `current` run (Fluent Bit 046accf6, no `QCORE_CROSS_UNIT_LINK_MAP`)
is likewise unchanged by this branch: no callee leaves the uncontracted
list, no function leaves the "only uncontracted" set, no unit status
changes. The report states what each option would move, measured on that
run's inputs with the base binary.
