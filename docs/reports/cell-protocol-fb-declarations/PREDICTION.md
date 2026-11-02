# CELL-PROTOCOL-FB-DECLARATIONS prediction

Committed before any declaration or provider edit, on base `aa55f2235`
(MK-EVENT-CELL-CROSS-FUNCTION's tip, rule 38c). The rows are relisted at
`baseline:0ac169180` (`private/baselines/0ac169180f549406…/` in the main
checkout). Rule 38c lies between that baseline and the base and is predicted
to move no row (its own prediction, `none` on every route); the integrator
confirms that.

The user's direction (2026-11-02, verbatim): "Then, we'll have to launch both
of them." The decision this card applies (2026-11-02, verbatim): "B as
recommended".

## What the census decides before any declaration

The store census ([census/](census/), `census.py --check` over the three
trees) reads every store, address-taking, allocation and first-member event
pass of every candidate member at 66910c10, 046accf6 and 34155955. Its
reading, record by record (details in the report):

| Record | Candidate | Stores keep it? | Declared here? |
|---|---|---|---|
| `flb_input_thread_instance` | `notification_channels_initialized == 1` over `notification_channels[0..1]`, `notification_event.fd` | yes | **yes** |
| `flb_sched` | `ch_events[0] != -1` over `ch_events[0..1]`, `event.fd` | yes | no: withheld (below) |
| `flb_config` | `notification_channels_initialized == 1` over `notification_channels[0..1]`, `notification_event.fd` | yes | no: withheld (below) |
| `flb_input_instance`, `flb_output_instance` | `ch_events[0] > 0` | no: the destroyer closes both ends and does not restore the sentinel, then keeps using the record | no |
| `flb_input_thread_instance`, `flb_config` | the `> 0` channels (`ch_parent_events`, `ch_thread_events`, `ch_manager`, `ch_notif`) | no, same shape; and G05i states one guard per record | no |

**Why `flb_sched` and `flb_config` are withheld.** Their stores keep the
protocol, but rule 38c as landed reads the declaration badly at their own
destroyers. A replica of each body under the candidate declaration
(`crates/qcore-c-gateway/tests/cell_protocol_fb_declarations.rs`, the ignored
`withheld_protocols_reading`) shows this:

- **`flb_sched_destroy`.** The timer loop's
  `mk_event_timeout_destroy(sched->evl, &timer->event)` is a cell-bearing call
  whose actual is rooted at `sched`. Rule 38c then gives `sched` a "reached"
  fact before the guard test at :855, so the test is not read.
  - `descriptor-cell-guard-undecided` :856 stays.
  - The row gains `descriptor-cell-overwritten` at the two `-1` stores, and
    `descriptor-cell-protocol-broken` at `flb_free(sched)` and at the exit.
  - It also gains three `descriptor-cell-escapes` at the exit.
- **`flb_engine_shutdown`.** C2's
  `mk_event_channel_destroy(config->evl, config->ch_self_events…)` does the
  same to `config` before the flag test at :1717. :1718 stays undecided, and
  the row gains `protocol-broken` and three `escapes`.
- **Other rows.** The same reading would also give a fact to every
  `config->evl` timeout call in other rows (`flb_engine_flush_timer_reset`,
  `schedule_request_now`, `flb_output_init_all`, `flb_create`, …).

These refusals say that records which keep their protocol do not keep it, so
declaring the protocols would publish wrong diagnostics. The fix is in rule
38c's host (a cell-bearing call should touch a protocol record only through
one of its protocol members). That host is outside this card
(`legacy_descriptor_cells.rs` belongs to CELL-PROTOCOL-CLOSE-ACTUAL), so it is
a follow-up, and the two records are not declared.

## Rows the declaration can reach

The declared record, `struct flb_input_thread_instance`, is read only in
`src/flb_input_thread.c` (the census lists every site; no other file names
its protocol members). The rows whose body reads a declared member, or names
the record in a cell-bearing call:

| Route | Unit | Row | Baseline | Predicted |
|---|---|---|---|---|
| slice | `src/flb_input_thread.c` #2, #3 | `input_thread_instance_destroy` | Unknown, 13 leaves, `descriptor-cell-guard-undecided` at the destroy (pp 17231, origin :197) | **Unknown**. Leaf trade: the guard leaf goes, and so does `unresolved resource identity thi` at the `if (thi->evl)` test (pp 17235). The rest stays: the access gate's refusals at the four `> 0` closes, `flb_tp_destroy`, and the later `thi` identities. |
| slice | `src/flb_input_thread.c` #2, #3 | `input_thread_instance_create` | Unknown, 30 leaves | **Unknown, unchanged**. `thi` is the body's own allocation and is an unresolved identity at its one protocol store (the flag at :300), so rule 38c finds no root there and reads nothing. The creations are refused by the access gate first. |

No other row on any route reads a member of the declared protocol:

- **corpus.** The only Fluent Bit unit, `src/flb_input_chunk.c`, names no
  thread instance. Monkey has no such record.
- **system.** No unit includes `flb_input_thread.c`. The system rows that
  read `flb_sched` or `flb_config` members read records that stay without a
  protocol.

The host declaration adds one pinned declaration
(`flb_config_set_program_name`, `fluent_bit/teardown_destroy.qv`). No unit of
any route calls or defines it: its only caller is `src/fluent-bit.c`. So no
call reading changes. It goes to the retaining table as every lifetime host
does.

- **No `Safe` move.** The thread instance's destroyer keeps the access gate's
  refusals.
- **No `Unsafe` move.** Rule 38c never accuses.
- **No move away from `Safe` or `Pending`.**

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## With CELL-PROTOCOL-CLOSE-ACTUAL merged

The close admission matters only at a declared protocol cell. The only one
here is the thread instance's notification channel, which ends through
`mk_event_channel_destroy`, never through `close`. So on the merged tree:

- **`input_thread_instance_destroy`** reads as above. Its `> 0` closes
  (`ch_parent_events`, `ch_thread_events`) are not protocol cells, so they
  keep the access gate's refusal unless CLOSE-ACTUAL also admits integer
  member reads outside a protocol.
- **`flb_sched_destroy` :856 and `flb_engine_shutdown` :1718** are unchanged,
  because their records carry no protocol.
- **The `> 0` ends** (`flb_input_instance_destroy`,
  `flb_output_instance_destroy`, `flb_config_exit`, the thread instance's
  `> 0` closes) are unchanged, because no protocol declares their cells.

So the merged tree is predicted to move the same rows as this card alone,
and nothing more.
