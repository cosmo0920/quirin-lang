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

## Amendment, before any measurement (commit after `7dc756613`; superseded in part by the second amendment)

The status prediction is unchanged: **no row moves on any route**. Two
things found during implementation narrow the scope and widen the set of
touched rows.

1. **Cell-bearing declarations only.** Rule 12t classifies a withheld call a
   second time under a minted name (`qcore_retained__<callee>`). The W44
   probe's admitted actuals were matched only under the plain name, so they
   left the gate in the outer classification but not in the inner one. As a
   result, every actual that no older route exempts kept the refusal. That
   includes the Fluent Bit loop actuals `sched->evl`, `config->evl` and
   `thi->evl`, which this body binds, as well as the elements. Rule 38a
   matches the minted name, and admits the element, **only when the withheld
   declaration carries a descriptor-cell state**:
   `mk_event_timeout_create`, `mk_event_timeout_destroy`,
   `mk_event_channel_create` and `mk_event_channel_destroy`. The
   `flb_input_set_collector_event` row (`in_tail_init`) is therefore out of
   scope. So is every other withheld declaration, such as `mk_event_add`.
   The plain-name gap there is the follow-up
   `RETAINING-CALL-MINTED-NAME-ACTUALS`.
2. **Every gate leaf on a cell-bearing call can trade**, not only those with
   element actuals, because the call's other W44-admitted actuals now leave
   the inner gate as well. The rows were relisted at the same baseline:
   every leaf on a call of the four declarations, any actual shape. The table
   above loses `in_tail_init` and gains these rows:

| Route | Row (×units) | Leaves | Cell calls refused at the gate | A leaf that stays |
|---|---|---|---|---|
| system | `src/flb_engine.c::flb_engine_flush_timer_reset` | 4 | timeout_create :325, :337 | :332 uncontracted `flb_utils_error` |
| system | `src/flb_scheduler.c::flb_sched_timer_cb_create` | 7 | timeout_create :672 | :653 uncontracted `flb_sched_timer_create` |
| system | `src/flb_scheduler.c::flb_sched_destroy` (as above) | 10 | timeout_destroy :843 too | :829 unresolved `request` |
| slice | `src/flb_input.c::flb_input_collector_destroy` | 3 | timeout_destroy :3146 | :3150 gate refusal (no cell call) |
| slice | `src/flb_input.c::flb_input_collector_resume` | 5 | :3248 | :3238 gate refusal (no cell call) |

The rows `schedule_request_now`, `collector_start` and
`flb_input_collector_pause` already pass the gate at their cell calls, so they
are unchanged. Each listed row keeps a leaf that this card does not touch, on
a statement before or beside the cell call, so none can become `Safe`. No
base is released before its call, so none can become `Unsafe`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## Second amendment, before any measurement (commit after `6066c25f0`)

The status prediction is unchanged: **no row moves on any route**.

**The integer-array member reader moved.** It was a class of
`typed_field_access.rs`, whose bytes
`docs/reports/libc-reading-site-evidence/bindings.json` pins whole. The first
card gate's `libc_reading_site_evidence` and `reading_site_domain` tests
failed on that pin. The reader now reads the record definition from the
unit's tokens in `legacy_call_argument_access.rs`, behind the declaration
reader's completeness check. The admitted shapes are the same, except that
the member's declaration must have exactly one declarator, which narrows the
rule. Every Fluent Bit slot member is declared alone (`flb_pipefd_t
ch_events[2];`).

**`MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`** (registered by
MK-EVENT-FIRST-MEMBER-CELL). In a function that passes a caller's record `c`
whole to one cell call, a member address `&c->event` at another cell call
kept the access gate's refusal. This is the minted-name gap of the first
amendment. With rule 38a's match switched off in the worktree, the gate
refuses `&c->event`. With the match on, it leaves the gate and rule 38
answers.

**No channel row still refuses because of it.** Fluent Bit's
`mk_event_channel_create(…, &ins->ch_events[0], &ins->ch_events[1], ins)` in
`flb_output_init_all` and `input_instance_channel_events_init` passes the gate
at this tip; only rule 38 refuses there. On this base, rule 38's first-member
reading (MK-EVENT-FIRST-MEMBER-CELL) is not present, so `ins` is "not a
`struct mk_event *`". The fixtures are in `tests/cell_array_slots.rs`
(`a_record_passed_whole_never_keeps_the_gate_at_its_member_address`). Those
two rows keep their other leaves (`flb_input_event_loop_get` uncontracted;
`flb_output_init_all`'s 80-odd others), so the status prediction holds with
or without that card.
