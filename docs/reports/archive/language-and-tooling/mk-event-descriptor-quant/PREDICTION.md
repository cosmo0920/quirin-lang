# MK-EVENT-DESCRIPTOR-QUANT prediction

Committed before any code edit, on base
`eb49bb6ceb52c5a441dc26e68332065119993e99` (proposal commit `262bc151d`).

> **USER DECISION 2026-11-01** (verbatim, relayed by the integrator): "I
> approve the changes of D1-D5."

That is D1 = 1A, D2 = 2A, D3 = 3A, D4 = 4A and D5 = the kind carried on the
value, each as [PROPOSAL.md](PROPOSAL.md) §2 and §6 state it.

## What moves

The rows that rest on one of the four declarations are listed in
PROPOSAL.md §4 (read at `baseline:98b1bb785`). `baseline:eb49bb6ce` was not
published when this was written, and nothing between the two revisions
touches these declarations, rule 12t, rule 30 or the descriptor reading. The
four declarations drop `resource_premise(mk_event_descriptors, …)` and gain
the decided cell clauses. A declaration carrying a cell clause publishes the
rule premise `P-CELL-STATE-NOT-FORWARDED` with every row that rests on it.

**Status.** No row moves on any route.

- **The one `Safe` row.** Of every row consulting the four declarations,
  only `src/flb_scheduler.c::flb_sched_timer_cb_disable` (system, ×2 units)
  is `Safe`. Its call `mk_event_timeout_destroy(timer->sched->evl,
  &timer->event)` names the cell `timer.event.fd` of a caller's object. No
  record is kept for that cell, so `#stale(descriptor)` is the identity on
  custody (the untracked-cell case of D2), and the removal is unchanged. The
  row stays `Safe`, trading `mk_event_descriptors` for
  `P-CELL-STATE-NOT-FORWARDED`.
- **Every other row** that consults the four declarations is `Unknown`
  through leaves that the cell reading does not remove: `retention-undecided`,
  the access gate, uncontracted callees, unresolved identities. The reading
  can add refusals:
  - `descriptor-cell-escapes` at an exit after a creation into a caller's
    object;
  - `descriptor-cell-undecided` at a creation whose `data` actual names no
    cell key, such as a local pointer `event = &timer->event`;
  - `descriptor-cell-guard-undecided` at every Fluent Bit channel destroy,
    whose member-equality guard no body decides.

  It never removes a refusal another route made.
- **No move toward `Safe` is admissible.** Any such row is a soundness
  finding.

**Leaves and digests.** Leaves may trade on the listed `Unknown` rows. The
contract sidecar, the `assumptions` digests of the listed rows, the provider
locks and the pins of the four declarations change. The null property and
the bounds identity are unchanged: no bounds or null clause is touched.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
