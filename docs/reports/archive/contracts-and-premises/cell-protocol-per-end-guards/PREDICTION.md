# CELL-PROTOCOL-PER-END-GUARDS prediction

Committed before any implementation, on base `1b859bdc9` (origin/main). The
rows are relisted at `baseline:1b859bdc9` (`private/baselines/1b859bdc9ad16759428d4a99628aa08d75f13101/`
in the main checkout).

> **USER DECISION 2026-11-03** (verbatim, on the choice between "a sentinel on
> each channel end, or several protocols per record"): "Per-end sentinels".
>
> **USER DIRECTION 2026-11-03** (standing, verbatim): "The Fluent Bit's caller
> tests sometimes check the incorrect assertions so we need to pay attention
> for the assertions and actual returning values of Fluent Bit functions."

## What the card will change

1. **Grammar G05i, amended.** A record's `quant cell` clauses are grouped by
   guard. Besides the one record guard G05i has today, each `#fresh` cell may
   carry its own sentinel (`when #B.CELL > 0` or `!= -1` on the clause's own
   cell), and a `#same` cell takes its target's guard. A record keeps at most
   one guard over several `#fresh` cells; every other guard is a per-end
   sentinel over exactly one.
2. **Rule 38c's reading, amended (new rule 38f).** Each guard has its own fact
   on a path. A `> 0` sentinel's dead edge (`<= 0`) holds descriptor 0, a
   success value of every creator and calloc's absent value at once, so it is
   never read as "the end is absent": its cells stay undecided (`Maybe`), and
   an end left so at its holder's release is named a candidate defect
   (`descriptor-cell-zero-undecided`, `Unknown`). Only a `!= -1` dead edge
   decides absent. A join keeps every name that may be live. The holder's
   release ends its record's protocols once no cell may be live. A per-end
   guard gains a fact at a cell-bearing call only through one of its own
   cells. Lean parity in a new module.
3. **Declarations.** On the existing host `flb_config_set_program_name`
   (`fluent_bit/teardown_destroy.qv`), re-pinned at the three trees:
   per-end `> 0` sentinels for `flb_output_instance.ch_events[0..1]`
   (`event.fd` `#same(ch_events[0])`), `flb_input_instance.ch_events[0..1]`
   (`event.fd` `#same(ch_events[0])`) and `ingress_queue_channels[0..1]`, and
   `flb_input_thread_instance.ch_parent_events[0..1]` (`event.fd`
   `#same(ch_parent_events[0])`) and `ch_thread_events[0..1]`
   (`event_local.fd` `#same(ch_thread_events[0])`), beside the record's
   existing flag group. `flb_config` (`ch_manager`, `ch_notif`) is withheld:
   every `config->evl` cell-bearing call would give its record-level groups
   the reached fact CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD describes.
   Its declaration text and fixtures are in the report.

## Rows the change can reach

The census (`candidates`, a lexical scan of `src/flb_input.c`, `flb_output.c`,
`flb_input_thread.c`, `flb_input_ingest.c`, `flb_output_thread.c` and
`flb_engine.c` at 046accf6, joined with the baseline's rows) finds these rows
whose body names a member the declarations state:

| Route | Row (units) | Baseline | Predicted |
|---|---|---|---|
| system | `flb_output_instance_destroy` (×2) | Unknown, 27 | **Unknown.** The two `> 0` tests (:615, :619) parse as per-end guard tests, so the row gains the host `flb_config_set_program_name` among its contract dependencies. `ins` is already an unresolved identity at :615 (earlier uncontracted calls), so no test is read; the closes (:616, :620) are admitted by rule 38d and, with no fact, keep the gate's refusal. The admitted actual's read of the unresolved `ins` may add `unresolved resource identity ins` at :616 and :620. |
| system | `flb_output_init_all` (×2) | Unknown, 86 | **Unknown.** The creation at :1545 is refused at the access gate first. The row may gain the host as a dependency (the parser names the protocol of the creation's actuals); no leaf changes. |
| system | `flb_output_task_flush_error` (×2) | Unknown, 2 | **Unchanged.** `flb_pipe_w(out_ins->ch_events[1], …)` is no test, store, close or cell-bearing call. |
| slice | `flb_input_instance_destroy` (×2) | Unknown, 63 | **Unknown.** As the output destroyer: four `> 0` tests (:1311, :1315, :1319, :1323) add the host dependency; `ins` is unresolved at each, the four admitted closes keep the gate's refusal, and may gain `unresolved resource identity ins` at :1312, :1316, :1320, :1324. |
| slice | `flb_input_new` (×2) | Unknown, 178 | **Unknown.** The `ingress_queue_channels[0..1] = -1` stores (:549, :550) parse as protocol stores and add the host dependency. A `-1` store is a `> 0` sentinel's dead value, so where `instance` resolves the store keeps the protocol and no leaf is added. |
| slice | `input_instance_channel_events_init` (×2) | Unknown, 8 | **Unknown.** The creation (:1380) adds the host dependency. If `ins` still resolves there, the per-end groups gain a fact through the creation's own cells and the creation into them refuses (`descriptor-cell-undecided`: a creation never establishes `> 0`, since the end may be descriptor 0), with `descriptor-cell-protocol-broken` and `descriptor-cell-escapes` at the exit. If it does not resolve, no leaf changes. |
| slice | `input_thread_instance_destroy` (×2) | Unknown, 11 | **Unknown.** Already lists the host. The four `> 0` tests (:210, :213, :218, :221) are now per-end guard tests, but `thi` is unresolved after the gate refusal at :206, so none is read; the four admitted closes keep the gate's refusal and may gain `unresolved resource identity thi` at :211, :214, :219, :222. The flag group's leaves are unchanged. |
| slice | `input_thread_instance_create` (×2) | Unknown, 30 | **Unknown, leaves unchanged.** Already lists the host; the creations are refused at the access gate first. |
| slice | `flb_input_thread_collectors_signal_start`, `flb_input_thread_wait_until_is_ready` (×2 each) | Safe, 0 | **Safe, unchanged.** `flb_pipe_w` / `read` of `thi->ch_parent_events[…]` is no step and reaches no fact. |
| slice | `flb_input_thread_collectors_signal_wait`, `flb_input_thread_instance_exit`, `_pause`, `_resume` (×2 each) | Unknown | **Unchanged**, for the same reason. |

The host's contract text changes (it gains the per-end clauses), so a row
already depending on it may show a changed hypothesis or assumptions digest
without a leaf change.

No other row reads a declared member:

- **corpus.** The only Fluent Bit unit, `src/flb_input_chunk.c`, names none of
  these records, and Monkey has none of them.
- **system and slice.** Every other row names none of the declared members in
  a test, a store, a close or a cell-bearing call.

The `> 0` reading change, the join change and the holder-release change act
only on a record with a fact; at the base the only declared protocol is the
thread instance's flag group, whose reading (`input_thread_instance_destroy`
:196–:202) has no `> 0` guard, joins two holding paths with no pending name
left, and releases no holder.

- **No `Safe` move.** Every reached row keeps leaves of its own (uncontracted
  calls, the access gate, unresolved identities). A `> 0` end can never be
  decided on its dead edge, so no destroyer of these records can become
  `Safe`.
- **No `Unsafe` move.** Rule 38c/38f never accuse.
- **No move away from `Safe` or `Pending`.** The two `Safe` rows above take no
  step.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Where this could be wrong.** If `ins` resolves at a destroyer's tests (it
does not at the baseline), the per-end tests are read: each live edge's close
is decided, each dead edge leaves its end undecided, and the row gains
`descriptor-cell-protocol-broken` at the first call reaching the record after
a close and `descriptor-cell-zero-undecided` at `flb_free`. The row stays
`Unknown`. A creator's refusal set depends on whether the formal still
resolves at the creation, which the baseline does not show directly.
