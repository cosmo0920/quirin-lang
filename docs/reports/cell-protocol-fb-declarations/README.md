# CELL-PROTOCOL-FB-DECLARATIONS

Card brief: `private/integrator-briefs/cell-protocol-fb-declarations.md` and
`r6-common.md` (round 6, 2026-11-02).

- **Base.** `aa55f2235` (rule 38c), merged with the landed main `86b26e5b2`
  (records only).
- **What it discharges.** `CELL-PROTOCOL-FB-DECLARATIONS`, registered by
  MK-EVENT-CELL-CROSS-FUNCTION.
- **Concurrent card.** CELL-PROTOCOL-CLOSE-ACTUAL runs alongside this one,
  with disjoint files.

> **USER DIRECTION 2026-11-02** (verbatim): "Then, we'll have to launch both
> of them."
>
> **USER DECISION 2026-11-02** (verbatim, the decision this card applies):
> "B as recommended".

## Result in one paragraph

- **What it declares.** One protocol, on `struct flb_input_thread_instance`:

  ```qcore
  quant cell #t: flb_input_thread_instance.notification_channels[0] |-> #fresh(descriptor) when #t.notification_channels_initialized == 1;
  quant cell #t: flb_input_thread_instance.notification_channels[1] |-> #fresh(descriptor) when #t.notification_channels_initialized == 1;
  quant cell #t: flb_input_thread_instance.notification_event.fd |-> #same(notification_channels[0]) when #t.notification_channels_initialized == 1;
  ```

- **The host.** The clauses are hosted on `flb_config_set_program_name` in
  `fluent_bit/teardown_destroy.qv`.
- **The census.** It covers every store of every candidate member at the three
  trees. Its verdicts by record:
  - `flb_sched` and `flb_config`'s flag protocol: every store keeps them.
  - The `> 0` channels (`flb_input_instance`, `flb_output_instance`, and the
    thread instance's and config's other channels) are broken by their
    destroyers, so they are not declared.
- **Why `flb_sched` and `flb_config` are withheld.** Rule 38c's host reads
  them wrongly at their own destroyers (§3). Declaring them would publish
  `descriptor-cell-protocol-broken` refusals for records that keep their
  protocols.
- **The yield.** Of the three ends the brief expected to trade, only
  `input_thread_instance_destroy` :197 does. The other two wait for the
  follow-up `CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD`.
- **No status move is predicted.**

This is a narrower delivery than the brief listed. The reasons are in §2 and
§3. The withheld declarations' exact text is in §3, ready for the follow-up.

## 1. The store census

`census/census.py` is a lexical reader over every `.c`/`.h` under `src/`,
`include/` and `plugins/` of each tree.

- **Inputs.** The 66910c10 corpus source, the 046accf6 system clone and the
  34155955 slice export, read only.
- **What it lists.** Every access to a candidate member:
  - `store`, `address` (the address handed to a call), `array` and `read`;
  - `alloc` (`sizeof(struct R)`, `memset`/`memcpy` of a record);
  - a record whose first member is its `struct mk_event`, passed as an event.
- **How it resolves.** It resolves the base through the enclosing function's
  declarations and the header field table. It reports what it cannot
  resolve; it never drops it.
- **The annotations.** `census/annotations.json` gives every non-read site
  its reading: `keeps`, or `outside` (a member of no declared protocol). Each
  key is revision-independent text, so one reading covers a site whose line
  moved.
- **The checked record.** `census/census.json` holds the result. It records
  each tree's SHA-256 tree digest (over every file read) and the digest of
  every file holding a site. These are the counts:

| Tree | Files read | Sites |
|---|---|---|
| 66910c10 | 1,175 | 126 |
| 046accf6 | 1,182 | 128 |
| 34155955 | 1,184 | 128 |

The census has 58 distinct non-read sites. Each run recomputes it and checks
it as follows:

```sh
python3 -I docs/reports/cell-protocol-fb-declarations/census/census.py \
  --root 66910c10=<corpus fluent-bit-clang-source> \
  --root 046accf6=<system fluent-bit-clang-source> \
  --root 34155955=<slice fluent-bit-clang-source> \
  --check docs/reports/cell-protocol-fb-declarations/census/census.json
```

That command exits 0 with 0 problems. It refuses a changed tree digest, an
unannotated site and an unused annotation.

**Readings, per record** (lines at 046accf6, which equal 34155955; 66910c10
differs only in line numbers, given in the census):

- **`flb_input_thread_instance`, flag `notification_channels_initialized`**
  (all in `src/flb_input_thread.c`, byte-identical at the three trees).
  **Declared.**
  - `flb_calloc` (:235): the flag is 0.
  - The creation (:287-290) runs with the flag still 0, because every earlier
    failure arm destroys and returns. A failed creation writes no end: a pipe
    failure returns first, and a failed registration closes both ends and
    returns before `*r_fd`/`*w_fd`.
  - `= FLB_TRUE` (:300) comes right after a successful creation.
  - `input_thread_instance_destroy` calls `mk_event_channel_destroy` under the
    flag (:196-200), then stores `= FLB_FALSE` (:202).
  - `ins->notification_channel = thi->notification_channels[1]` (:303) is a
    copy (Q4).
- **`flb_sched`, `ch_events[0] != -1`** (`src/flb_scheduler.c`, identical at
  the three trees). **Keeps.**
  - `flb_calloc` (:746): the zero bytes make the guard say live with no cell,
    but the pointer is still local. The `-1` stores at :754-755 and
    `MK_EVENT_ZERO` (:756, `event.fd = -1`) restore the dead reading before
    any call or read.
  - The creation (:794-797): on failure no end is written, so the guard stays
    `-1`.
  - The destroy under the guard, then `-1` (:855-862).
- **`flb_config`, flag** (`src/flb_engine.c`). **Keeps.**
  - `flb_calloc` in `flb_config_init` (flb_config.c:288): the flag is 0.
  - The creation (:1246-1249).
  - `= FLB_TRUE` (:1256), right after the creation succeeds. On failure it
    returns -1 with the flag 0.
  - In `flb_engine_shutdown`, the destroy under the flag, then `= FLB_FALSE`
    (:1717-1724). A second shutdown reads the flag dead.
- **`flb_input_instance` / `flb_output_instance`, `ch_events[0] > 0`.**
  **Broken.**
  - `flb_input_instance_destroy` (flb_input.c:1311-1316) and
    `flb_output_instance_destroy` (flb_output.c:615-620) close both ends
    under `> 0`. They never restore the sentinel, and then keep using the
    record (more teardown, `flb_output_free_properties(ins)`) before
    releasing it.
  - So the guard says live over closed descriptors in a reachable state.
  - Also, no creation establishes `> 0` (descriptor 0 exists).
- **The `> 0` channels of `flb_input_thread_instance`** (`ch_parent_events`,
  `ch_thread_events`) **and of `flb_config`** (`ch_manager`/`ch_event`,
  `ch_notif`). **Broken.**
  - They have the same destroyer shape: input_thread.c:210-222, then
    `flb_tp_destroy(thi->tp)`; flb_config.c:607-630.
  - G05i also states one guard per record, so neither record could carry a
    second protocol.
- **C2, `ch_self_events` / `event_thread_init`.** Its destroy is guarded by
  `if (config->evl)` alone, so no protocol states it. It is listed only for
  §4.

## 2. The declaration

**The host.** `flb_config_set_program_name` hosts the declaration. It is a
vehicle chosen so that no call reading changes: no audited unit calls or
defines it, and its only caller is `src/fluent-bit.c`.

- **Body.** src/flb_config.c:956-966 at 046accf6 and 34155955, :955-965 at
  66910c10, the same body.
- **Its own clauses** are what that body does: `config` borrowed, `name`
  borrowed (copied by `flb_sds_create`), and the status result.
- **Every creator and destroyer of the declared record is defined in an
  audited unit**, so none of them could host it without changing a call
  reading.

**The pins.** `quirin-audit pin --crate quirin-provider-fluent-bit --function
flb_config_set_program_name` was run at the three trees. The lock now holds
86 pinned entries. `providers/contracts.py sync` then regenerated
`contract-inventory.lock`. The reviewed diff is one declaration with seven
clauses: `teardown_destroy.qv` 14 → 15 declarations, 140 → 147 clauses.
`providers/contracts.py check` exits 0.

**Effects on the loader.** The host leaves the function map for the lifetime
table. `compiler/tests/contract_loader.rs` counts it:

- v1 declarations 498 → 499;
- retaining 108 → 109;
- selected modules and rejected declarations are unchanged.

## 3. Why `flb_sched` and `flb_config` are withheld

Rule 38c's host gives a record a "reached" fact (touched, guard unknown) at
every cell-bearing call that has an actual rooted at the record. That
includes actuals that name no protocol member: `sched->evl` in
`mk_event_timeout_destroy(sched->evl, &timer->event)`, and `config->evl`
with C2's `ch_self_events`. In `legacy_descriptor_cells.rs::apply`,
`operand_protocols` is the root of every actual.

A record with a fact is never tested again, and a reached fact holds neither
reading. Under the census protocols, the ignored fixture
`withheld_protocols_reading` (run with `--ignored`) shows the result:

- **`flb_sched_destroy`'s teardown alone** is `Safe`.
- **The same teardown after the timer loop** keeps
  `descriptor-cell-guard-undecided`, and gains these:
  - `descriptor-cell-overwritten` at both `-1` stores;
  - `descriptor-cell-protocol-broken`;
  - three `descriptor-cell-escapes`.
- **`flb_engine_shutdown`'s two teardowns** keep both guard leaves, and gain
  these:
  - `protocol-broken` at the flag store and at the exit;
  - three `escapes`.

The same reached fact would touch every row with a `config->evl` or
`sched->evl` timeout or channel call:

- `flb_engine_flush_timer_reset`, `schedule_request_now`;
- `flb_sched_timer_cb_create`, `flb_output_init_all`;
- `flb_engine_start`, `flb_create`, `flb_input_collector_destroy`.

Each would gain a wrong protocol-broken refusal. None of those rows is
`Safe`, so no status would move, but the diagnostics would be false.

The fix belongs in the host: a cell-bearing call should give a protocol
record a fact only through an actual that names one of its protocol members.
That host is CELL-PROTOCOL-CLOSE-ACTUAL's file, and this card changes no
gateway code. So it is the follow-up `CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD`,
whose DISCHARGE includes these declarations:

```qcore
quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
quant cell #s: flb_sched.ch_events[1] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
quant cell #s: flb_sched.event.fd |-> #same(ch_events[0]) when #s.ch_events[0] != -1;
quant cell #c: flb_config.notification_channels[0] |-> #fresh(descriptor) when #c.notification_channels_initialized == 1;
quant cell #c: flb_config.notification_channels[1] |-> #fresh(descriptor) when #c.notification_channels_initialized == 1;
quant cell #c: flb_config.notification_event.fd |-> #same(notification_channels[0]) when #c.notification_channels_initialized == 1;
```

**A second limit, recorded for the creators.** The Monkey declaration states
the creation's cells under `#result == 0`. Every production creator tests
`ret == -1` (flb_input_thread.c:291, flb_engine.c:1252, flb_scheduler.c:798).
The live edge of that test does not establish the cells, so even a creator
over a resolved caller record refuses
(`the_production_result_test_does_not_establish_the_cells`). Today each
production creator is refused earlier: by the access gate, or by an
unresolved identity at the store. So this changes no row.

## 4. Observations from the census (not findings)

**The failed-start path of C2** (the integrator's reading, which the brief
asked this card to confirm or refute):

- **The path is textually present at all three trees.**
  - `flb_engine_start` sets `config->evl = evl` (:1150; :1134 at 66910c10).
  - It then returns -1 on a failed `flb_bucket_queue_create` (:1154; :1138),
    before the `ch_self_events` channel is created (:1168).
  - A failed `pipe` in that creation leaves the same state.
  - `flb_lib_worker` then calls `flb_engine_failed` and `flb_engine_shutdown`
    (flb_lib.c:928-929).
- **What the shutdown does then.** Its `if (config->evl)` arm calls
  `mk_event_channel_destroy(evl, 0, 0, &config->event_thread_init)` (:1710).
  The two ends are calloc's zeros, and `flb_config_init` does not zero that
  event with `MK_EVENT_ZERO`: it zeroes only `ch_event`, `event_flush` and
  `event_shutdown` (flb_config.c:297-299). So `event_thread_init.fd` is 0 as
  well.
- **The epoll backend then closes descriptor 0 twice.** Its equality test
  passes, and `_mk_event_del` returns 0 for an unregistered event
  (mk_event_epoll.c:163, :389-404).
- **For `flb_config`'s declared candidate (the notification flag), the path
  is refuted.** The flag is still 0, so the flag-guarded destroy is not
  taken. C2's destroy is no protocol's, and this card does not change its
  `guard-undecided` reading.
- **This is a source reading, not a reproduction.** Nothing is filed
  upstream.

**`flb_config_exit` closes the manager channel's read end twice**
(flb_config.c:607-621). It closes `config->ch_event.fd` when it is nonzero,
then `config->ch_manager[0]` under `> 0`. After the creation (engine:1237)
both hold the same descriptor (the backend's `event->fd = fd[0]`,
`*r_fd = fd[0]`). The second close then returns `EBADF`, unless another
thread has been handed the number in between. This too is a reading, not a
reproduction.

## 5. Prediction and measurement

[PREDICTION.md](PREDICTION.md) was committed at `2b644efde`, before the
declaration (`b07b574f9`). It predicts `none` on corpus, system and slice:

- **The slice's `input_thread_instance_destroy`** (×2 units) loses
  `descriptor-cell-guard-undecided` at its destroy (origin :197) and the
  `unresolved resource identity thi` at the `if (thi->evl)` test. It keeps
  every other leaf and stays `Unknown`.
- **`input_thread_instance_create`** is unchanged.
- **No other row** of any route reads a declared member.

The forecast for `input_thread_instance_destroy` comes from a replica. It
was run once during the card, on the full production body with its `> 0`
closes; the harness is not shipped. Under the declaration the replica keeps
the four close refusals, the evl release, `flb_tp_destroy` and the release
of `thi`. The guard leaf and the evl test's identity leaf are gone. The
committed fixture is the same body without the closes.

The worktree's isolation guard refuses the live measurement. The integrator
runs it from the main checkout (§ reply). `<tip>` is the last code commit,
`e598aff65`.

**With CELL-PROTOCOL-CLOSE-ACTUAL merged:**

- **Its close admission matters only at a declared protocol cell.** The
  thread instance's notification channel never ends in `close`, so the
  merged tree moves the same rows as this card alone.
- **`flb_sched_destroy` :856 and `flb_engine_shutdown` :1718 stay
  guard-undecided**, because their records carry no protocol.
- **The `> 0` ends stay refused**: `flb_input_instance_destroy`,
  `flb_output_instance_destroy`, `flb_config_exit`, and the thread instance's
  `ch_parent_events`/`ch_thread_events`. No protocol declares their cells.

## 6. Tests

`crates/qcore-c-gateway/tests/cell_protocol_fb_declarations.rs` holds 9 tests
and 1 ignored test. Each asserts the complete diagnostic vector under the
production `teardown_destroy.qv` and `monkey/event_loop.qv`.

- **The production inventory.** One protocol, on `flb_input_thread_instance`,
  hosted by `flb_config_set_program_name`, which sits in the retaining table.
- **Forwarded.**
  - The production teardown (without its `> 0` closes, which rest on the
    access gate) keeps no descriptor-cell leaf.
  - The complement test is `Safe`.
  - A creator over a dead flag, with `#result == 0`'s test, keeps only
    `retention-undecided`.
- **Broken.**
  - A teardown that keeps the flag.
  - A flag set over dead cells.
  - The production `ret == -1` creator shape.
- **Unprotocolled.**
  - The same teardown over `struct flb_out_thread_instance`.
  - `flb_sched`'s and `flb_engine_shutdown`'s teardowns under the production
    modules, which keep rule 38's `guard-undecided`.
- **Ignored.** `withheld_protocols_reading` (§3). It pins rule 38c's current
  host reading, which the follow-up changes.

The fixtures that rest on a refusal name it. None rests on the access gate's
close refusal, which is CELL-PROTOCOL-CLOSE-ACTUAL's area.

There is no Lean change: the card changes no semantics, only a declaration
under rule 38c's existing model (`QCore.DescriptorCellProtocol`).

## 7. Card gate

See [gate.json](gate.json) and the reply. The gate ran as
`python3 scripts/gate_scope.py --base 86b26e5b2 --out private/gate` on
`e598aff65`. The console log is outside the worktree, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/cpfd-gate-console.log`.

## 8. Follow-ups

- **`CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD`** (registered). Narrow rule
  38c's reached fact to actuals that name a protocol member. Then declare
  `flb_sched` and `flb_config` (§3) and measure. The expected yield is leaf
  trades at `flb_sched_destroy` :856 and `flb_engine_shutdown` :1718.
- **`CELL-PROTOCOL-POSITIVE-SENTINEL-DESTROYERS`** (registered). The `> 0`
  channels are not declarable as G05i stands.
  - Their destroyers close without restoring the sentinel and use the record
    before releasing it.
  - A record carries one guard.
  - Declaring them needs a reading in which the holder's release ends the
    protocol, plus several protocols per record, or an upstream sentinel
    reset.
- **Amended into `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`.**
  - The creators' `ret == -1` tests do not establish the cells
    (`#result == 0`).
  - The §4 readings.
