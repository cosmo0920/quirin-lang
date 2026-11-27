# CG27-LOOPS-RECURSION-BUDGETS — frozen prediction

Frozen before any AFTER measurement, on base `b6a3dc42`. The implementation
was drafted in the working tree before this file was written (an ordering
departure from the brief, stated here rather than hidden); no AFTER corpus,
system or slice run had been taken, and none of the BEFORE artifacts had been
read beyond their totals.

## What changes

1. **Counted-user loop summary** (gateway rule 11x(1), route G's counted-user
   pass, `legacy_counted_loops.rs`). A `while`, a `for` or a `do` (not
   `do … while (0)`) none of whose tokens performs a counted-user operation
   (no counter write, no call declaring an operation or exit clause, every
   other call neutral, no call through an expression, no `return` or `goto`)
   is summarized by the identity on the counts, forgetting the values of the
   names it spells. Any other loop keeps the pre-existing refusal and wording.
2. **Signed threshold rank** (rule 11x(2), `legacy_signed_rank.rs`). A
   recursive component left unpublished by every existing route (ordinary,
   zero/nonzero ranked, neutral frame) is read once more with one signed
   `int`/`long`/`long long` control, sign guards `n <= c` / `n > c` at one
   literal threshold, and actuals `n - k` (`1 <= k <= 1024`) or `0`, under the
   existing base/step validation.
3. **Budgets** (rule 11x(3)). No analyzer change: tests and Lean that every
   budget's exhaustion is reported and never `Safe`.

## Predictions

| slice | BEFORE (brief's baseline) | predicted AFTER |
|---|---|---|
| corpus `66910c10` | 629 / 10 / 4,195, `3d53a5d0…` | **byte-identical** (0 moved functions) |
| system | 44 / 2 / 188, `79404fed…` | totals identical; digest identical unless a counted-user record moves |
| tail-pause slice | 29 / 1 / 239 / 1 Pending, `93cc660c…` | totals identical; digest identical unless a counted-user record moves |

Reasons:

- **Recursion, corpus.** C03-REACH measured 46 recursive components (96
  members); 94 members refuse at the containment gate (a retained `Unknown`
  instruction), which the contextual CFG refuses before any rank is read
  (`contextual_cfg_under` returns nothing for a CFG with an `Unknown` node).
  The signed route adds no admission there. Predicted reach: **0** functions.
- **Counted users, corpus.** The corpus profile's counted-user clauses are the
  Fluent Bit retry/task ones; a function whose loop is summarized still needs
  every other custody obligation to discharge. Predicted: no function moves.
- **Counted users, system and slice.** The retry-route functions named by
  W45-COUNTED-USERS-LOOPS (`handle_output_event`, `flb_engine_dispatch_retry`)
  have loops whose bodies call task and retry functions with counted-user
  clauses, or opaque calls, so they keep the refusal. A loop that is
  summarized may turn an `undecided` clause record into `held` or `refuted`
  without moving its function, whose custody blockers remain. Predicted: no
  function moves; if a record moves, every moved record is attributed by
  reading.
- **No Safe → non-Safe move** is predicted. Any that occurs is a named
  correction.

## Absorbed follow-ups (predicted disposition)

- `W45-COUNTED-USERS-LOOPS`: the bounded loop rule lands; re-measured on the
  retry route (system and slice). Closed if its DISCHARGE (rule with Lean
  parity, measured) holds.
- `W43-COLLECTION-LOOP-SHAPES`: untouched by this card (decrementing sweeps,
  `while` headers, member bounds and the vacuity-aware zero fact stay open);
  amended with this card's reading, not closed.
- `JOINED-PROVENANCE-CYCLE-SITES`: untouched; amended with the observation that
  CG04's generations supply a per-iteration identity the restriction could
  build on, not closed.
- `C03-NFR-REACH`: re-read by re-running the C03-REACH census at BEFORE and
  AFTER; predicted 46 components, the signed route admitting 0 of them.
