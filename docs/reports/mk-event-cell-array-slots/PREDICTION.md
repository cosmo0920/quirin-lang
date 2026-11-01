# MK-EVENT-CELL-ARRAY-SLOTS prediction

Committed before any code edit, on base
`e589ee5a7248dfd4afbcefd5b542e0de339c31dd`. The rows were relisted at
`baseline:e589ee5a7248dfd4afbcefd5b542e0de339c31dd` (the main checkout's
`private/baselines/`) by reading every gate-refusal leaf
(`indirect memory access is outside the resource projection`) whose origin
statement holds an actual `X->f[K]`, `X.f[K]` or `&X->f[K]` with `K` a decimal
literal.

> **USER DECISION 2026-11-01** (verbatim, relayed by the integrator): "I
> approve the changes of D1-D5." This card adds no clause spelling.

## The rule this predicts

A constant-index element of an integer array member, read (`X->f[K]`,
`S.f[K]`) or addressed (`&X->f[K]`, `&S.f[K]`), is admitted by the access gate
only at a `Borrowed` position of a withheld declaration (a retaining
declaration that is not in the function map). `K` must be a canonical decimal
literal below the member's declared length, and the element type must be an
integer type. The base keeps W44-INDIRECT-CALL-ARGUMENT-ACCESS's typed-read
obligation and its second obligation. Rule 38 then decides the cell.

## The rows that can trade leaves

Each of these rows is `Unknown` at the baseline with other leaves that this
card does not touch: uncontracted callees, unresolved identities, nested
calls, other gate refusals and braced initialisers.

| Route | Row (×units) | Leaves | Element calls refused at the gate |
|---|---|---|---|
| system | `src/flb_scheduler.c::flb_sched_create` (×2) | 25 | channel_create :794 |
| system | `src/flb_scheduler.c::flb_sched_destroy` (×2) | 10 | channel_destroy :856 |
| system | `src/flb_engine.c::flb_engine_start` (×2) | 87 | channel_create :1168, :1237, :1246 |
| system | `src/flb_engine.c::flb_engine_shutdown` (×2) | 15 | channel_destroy :1711, :1718 |
| system | `src/flb_output.c::flb_output_init_all` (×2) | 86 | channel_create :1545 |
| slice | `src/flb_input.c::input_instance_channel_events_init` (×2) | 5 | channel_create :1380 |
| slice | `src/flb_input_thread.c::input_thread_instance_create` (×2) | 30 | channel_create :262, :275, :287 |
| slice | `src/flb_input_thread.c::input_thread_instance_destroy` (×2) | 13 | channel_destroy :197 |
| slice | `src/flb_lib.c::flb_create` (×2) | 11 | channel_create :209 |
| slice | `plugins/in_tail/tail.c::in_tail_init` | 36 | flb_input_set_collector_event :410, :450, if withheld |

The corpus route has no such leaf. The other element actuals at the baseline,
`mk_event_closesocket(X->f[K])` (`close`, a function-map release), are outside
the rule.

## Status

**No row moves on any route.**

- **No `Safe` move.** An admitted element can only exchange its gate refusal
  for rule 38's answer, which never discharges a row here: a creation into a
  caller's or tracked object then escapes or stays undecided
  (`descriptor-cell-escapes`, `descriptor-cell-undecided`), and every Fluent
  Bit channel destroy's member-equality guard is undecided within its body
  (`descriptor-cell-guard-undecided`; the creation is in another function,
  `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`). Every row also keeps unrelated
  leaves. `input_instance_channel_events_init`'s `ins` data actual is a record
  whose first member is the event (rule 38 (1),
  `MK-EVENT-FIRST-MEMBER-CELL`), and its `flb_input_event_loop_get` stays
  uncontracted.
- **No `Unsafe` move.** No base is released before its element is read, and
  rule 38 never accuses.
- **No move away from `Safe` or `Pending`.** Only a statement that the gate
  already refused is affected, so every touched row is already `Unknown`.

Leaves may trade on the listed rows: a gate leaf becomes a rule-38 refusal or
an unresolved base identity. A path the analysis used to stop at the refused
statement can reach later refusals. The contract sidecar, provider locks,
digests and bounds identity are unchanged, because no contract is touched.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
