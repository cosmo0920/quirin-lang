# CURRENT-RUN-WORKER-SPIN prediction

Committed with the diagnosis, before any analyzer change, against base
`13337d0af358bd7a02ad2f5509b6a9512327c4aa` (origin/main). The measurement's
base side is `rev:13337d0af` (no published baseline exists for this
revision).

The card changes rule 4c's opt-in cross-unit import route only: a caller
holds, and so replays, only the homes it may reach (the name used in its
unit outside a file-scope prototype, or named as a value by the run's member
facts), and none above a per-unit replay budget. All three live routes run
with the eight-home link map (`link-map.corpus.json`). Eight homes are below
the budget, so no unit is withheld. A caller's analysis consults an import
only for a name its body reads or a member closure delivers, so every
lookup it makes reads what it read before
(`QCore.CrossUnitReplayBudget.consulted_lookup_unchanged`). No verdict and no
diagnostic leaf is predicted to move:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

Evidence fields that are expected to differ and are not verdicts:

- every unit row's `cross_unit_link_resolution` assumption (pinned route) and
  the `current` route's `.contracts.json` assumption gain
  `replay_budget_per_unit`;
- a caller's `contract_dependencies`, trust-context digests and
  reading-site omission counts no longer include what replaying homes it
  never reads recorded into its own analysis (home replay runs inside the
  caller's recording);
- the `current` route prints progress lines on standard error when the link
  map is selected.

The user's `current` run (Fluent Bit 046accf6, no `QCORE_CROSS_UNIT_LINK_MAP`)
is unchanged byte for byte: no import route runs, progress is off, no callee
leaves the uncontracted list, no function leaves the "only uncontracted" set
and no unit status changes.
