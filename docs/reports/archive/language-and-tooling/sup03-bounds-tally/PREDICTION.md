# SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: frozen prediction

Written and committed on the card's branch at base `e0b92ece6` before the
measurement ran. The report ([README](README.md)) checks the result against
it and does not edit it.

## What is measured

The pinned corpus arbiter (`scripts/run_pinned_audit.py … 66910c10`, `--jobs
4`) with `QCORE_BOUNDS_PROPERTY` set, using a release `quirin-audit` built from
`e0b92ece6` ([scripts/build.sh](scripts/build.sh),
[scripts/measure-bounds.sh](scripts/measure-bounds.sh)). The tally and the
per-function reason codes are read from the run's own `.summary.json`; the
obligation counts from its rows.

## Recorded figures it is compared with

* STATUS bounds row, first measured on `b6a3dc42`: **162 Safe / 0 Pending / 0
  Unsafe / 4,759 Unknown** over 4,921 definitions (4,834 projected + 87
  `bounds-import-failure`).
* Combined custody + bounds (QUANT-INDEX-CHECKS on `0ebe77f86`, held by
  CHUNKIO-META-LEN-BOUNDS-CONTRACTS and by CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS
  on `84ec54d1d`): **111 Safe / 4,801 Unknown / 9 Unsafe**.
* The last bounds measurement (CG16-BOUNDS-TRANSPORT, on `84ec54d1d` plus the
  card, landed as `43732604c`) had custody 881 / 9 / 3,944 on the arbiter;
  main now records custody **935 / 9 / 3,890** (QR-PRINT-ROUTE-MATCHES-DECLARATION),
  +54 Unknown → Safe with no Safe → non-Safe move.

## Prediction

**Bounds row: 162 Safe / 0 Pending / 0 Unsafe / 4,759 Unknown over 4,921,
unchanged.** Reasons:

* Every bounds-relevant change since `b6a3dc42` up to `43732604c` was already
  measured on the bounds route with no verdict move (CG24 frames, QUANT-INDEX-CHECKS,
  CHUNKIO-META-LEN-BOUNDS-CONTRACTS, CG16 transport).
* Since `43732604c`, the code changes touching bounds contract v1
  (`bounds_contract.rs`, `bounds_selection.rs`, `index_extent/body_evidence.rs`)
  and the new `bounds-contracts.lock` entries in librdkafka, cmetrics,
  fluent-bit and cfl come from the null cards (NULL-SAFETY-PROPERTY,
  NULL-DEREF-QTT-GROUNDING, NULL-CANDIDATE-TRIAGE, NULL-MEMBER-INVARIANT-SCOPE,
  NULL-TRIAGE-UNRESOLVED-PATHS). Their clauses (`quant null-deref`,
  `nonnull_*`, `quant live`) are removed by `BoundsContracts::for_bounds()`
  before the bounds consumer reads the environment, and no corpus contract
  uses `quant deref`, the only form with an extent part that reaches it.
  So the bounds consumer sees the same five chunkio declarations.
* The custody-side cards (CG22 callbacks, W45 cascade round 3,
  QR-PRINT-ROUTE, GAPO return correctness, route selection order) change
  custody or the `.qr` front end, not the bounds consumer's inputs.

**Per-function reason codes unchanged:** `bounds-missing-extent` 4,650,
`bounds-unsupported-form` 817, `bounds-unsupported-range` 214,
`bounds-import-failure` 87, `bounds-range-not-contained` 8. Obligation counts
by kind and outcome equal QUANT-INDEX-CHECKS' `bounds-summary-after.json`
except for the reason text CG16 transport already moved (same outcomes).
No bounds Unsafe; empty refutation table.

**Combined row: 111 Safe / 4,801 Unknown / 9 Unsafe, unchanged**, with the
custody column of the run at **935 Safe / 9 Unsafe / 3,890 Unknown + 87
absent**. Reason: combined is Safe only where custody and bounds are both
Safe. With bounds unchanged, the combined Safe count can move only if some of
the 51 functions that were custody-Unknown × bounds-Safe at QUANT-INDEX-CHECKS
became custody Safe among the +54 (and the +125 before them, which CG16
transport showed did not). The +54 come from callee declarations, null
premises, callback contracts and the print route, which are functions that pass
pointers to callees and so are mostly `bounds-missing-extent`; I predict none
of the 51 is among them. Low confidence on this half: if it is wrong, it
moves only upward (combined Safe > 111, Unknown < 4,801), since custody had
no Safe → non-Safe move. Combined Unsafe stays 9, custody's 9 provisional
Unsafe, each bounds Unknown.

**Row bytes:** not predicted as identical to CG16 transport's AFTER rows
(`79a90176…`), which are not available in this worktree to compare; the digest
is recorded, not compared.
