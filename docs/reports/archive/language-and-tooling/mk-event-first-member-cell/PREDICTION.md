# MK-EVENT-FIRST-MEMBER-CELL prediction

Committed before any code edit, on base
`e589ee5a7248dfd4afbcefd5b542e0de339c31dd`. Card authority: ledger entry
`MK-EVENT-FIRST-MEMBER-CELL` (DISCHARGE: "read the first member (C11
6.7.2.1p15: a pointer to a structure, suitably converted, points to its
initial member) as the event's cell, with a test that a record whose first
member is not the event still refuses, measured"). The vocabulary is the one
the user approved for MK-EVENT-DESCRIPTOR-QUANT (USER DECISION 2026-11-01,
verbatim: "I approve the changes of D1-D5."). This card adds no clause
spelling.

## The reading

Rule 38 (1) reads a member cell `#data: mk_event.fd` only where every actual
of the callee in the calling function has Clang type `struct mk_event *`.
This card adds one case. When every actual has one type `struct R *`, and
the unit holds exactly one complete definition of `struct R` whose first
member is declared `struct mk_event m`, the actual `X` is read as `&X->m`,
which is the same key the direct reading gives. Everything else keeps the
refusal (`descriptor-cell-undecided`): a first member of another type, a
pointer or array of the event, a later member, a union, an incomplete or
ambiguous record, a nested first-member chain, a mix of types, and an
unknown type.

## Rows whose cell-bearing call has a first-member actual

These are listed at `baseline:e589ee5a7` (`private/baselines/e589ee5a7…/`;
each row's diagnostics at the call line). The bodies were read at
66910c10 (corpus, `build/cycle19/c-gateway-wave-pin`, Monkey
`monkey/monkey-source`) and at 046accf6 (system,
`private/w45-fb/w46-external-callee-contracts/attempt-1`).

| Route | Row | Call | Actual (first member) | At the baseline | Under this card |
|---|---|---|---|---|---|
| corpus | `mk_server/mk_server.c::mk_server_worker_loop` (Unknown, 32 leaves) | `mk_event_timeout_destroy(evl, server_timeout)` :586 | `struct mk_server_timeout *` (`event`) | `descriptor-cell-undecided` ("not a `struct mk_event *`") | the cell is read; the create at :502 never reached rule 38 (an access-gate refusal at :502), so the table is empty and the release is the identity on custody. The leaf goes; 31 others remain, **Unknown** |
| corpus | `mk_server/mk_server.c::mk_server_worker_loop` | `mk_event_timeout_create(evl, server->timeout, 0, server_timeout)` :502 | as above | access gate (`indirect memory access`), rule 38 not asked | unchanged |
| corpus | `mk_server/mk_scheduler.c::mk_sched_launch_worker_loop` (Unknown, 27) | `mk_event_channel_create(sched->loop, &sched->signal_channel_r, …, notif)` :365 | `struct mk_sched_notif *` (`event`) | access gate at :365 | unchanged |
| system ×2 | `src/flb_output.c::flb_output_init_all` (Unknown, 86) | `mk_event_channel_create(config->evl, &ins->ch_events[0], &ins->ch_events[1], ins)` :1545 | `struct flb_output_instance *` (`event`) | access gate at :1545 (array-element slots) | unchanged: `MK-EVENT-CELL-ARRAY-SLOTS` |
| slice ×2 | `src/flb_input.c::input_instance_channel_events_init` (Unknown, 5) | `mk_event_channel_create(evl, &ins->ch_events[0], &ins->ch_events[1], ins)` :1380 | `struct flb_input_instance *` (`event`) | access gate at :1380 (array-element slots) | unchanged: `MK-EVENT-CELL-ARRAY-SLOTS` |

Every other call of the four cell-bearing declarations in the three routes
passes `&X->member`, a `struct mk_event *` local or member
(`event`, `ctx->event_channel`), or is refused before rule 38. None of them
changes. No audited unit holds `flb_output_thread.c`'s `th_ins` calls.

## What moves

**Status.** No row moves on any route.

- `mk_server_worker_loop` loses one `descriptor-cell-undecided` leaf and stays
  `Unknown`, held by uncontracted callees, unresolved identities and access
  refusals. Rule 12t's removal at :586 is applied as before. With rule 38 off,
  the call still refuses.
- The three channel creators keep the access gate's refusal. Once
  `MK-EVENT-CELL-ARRAY-SLOTS` admits the slots, their `ins` / `notif` actual
  reads a cell here instead of refusing.
- **No move toward `Safe` is admissible.** Any such move is a soundness
  finding.

**Leaves and digests.** Leaves may trade on the one listed corpus row. The
contract sidecar, the provider locks and the pins are unchanged, because no
declaration is touched. The null property and the bounds identity are
unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
