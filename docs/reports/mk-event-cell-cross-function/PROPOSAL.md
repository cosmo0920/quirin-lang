# MK-EVENT-CELL-CROSS-FUNCTION: proposal (phase 1)

Card brief: `private/integrator-briefs/mk-event-cell-cross-function.md` and
`r5-common.md` (round 5, 2026-11-02). Base `a9f7bebbe`. This card narrows
(amends, never discharges) `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`.

> **USER DIRECTION 2026-11-02** (verbatim): "We need to launch these cards."

The decisions that bind this card, quoted from the matrix entry, the
[cross-function report](../fb-event-loop-cross-function-lifetime/README.md)
and §C04.6.2 / §C04.21. They are not re-decided here:

- **D1/D5** "S3 declared protocols": "the lifetime model uses declared
  lifetime protocols at record types, each checked against every registration
  and store site of the run, a contradicted declaration withdrawn and an
  uncheckable one a declared assumption, rule 13i".
- **D3** "Yes, derived only": "a guarded loop-path equality only from single
  proven stores and an immutable flag".
- **D6** "Blockers first": "The access and identity blockers on the
  registration and teardown chains get cleared before any lifetime model is
  built."
- **D4**: the `quant [not] #condition in { clauses }: [[local]]` form, with
  `quant registered #coll: T.event |-> T.evl;` record footprints.
- **Descriptor cells** (USER DECISION 2026-11-01, "I approve the changes of
  D1-D5."): `#fresh(descriptor)`, `#stale(descriptor)`, `{#stale, #kept}`,
  `#same(S)`, `#pointee`, the member-equality guard, and the rule premise
  `P-CELL-STATE-NOT-FORWARDED`.

**Result of this phase: stop for a user decision.** The decided forms do not
cover forwarding a cell effect (§2). Two new things are needed, and there is a
real choice between derived and declared designs (§3). This card adds no
clause spelling and no code before that decision.

## 1. Census

### 1.1 Method

Every call of the four cell-bearing declarations (`mk_event_timeout_create`,
`mk_event_timeout_destroy`, `mk_event_channel_create`,
`mk_event_channel_destroy`) was read in the analysed trees, together with
every other end of the same cells: `mk_event_closesocket` of a member,
copies of the number into another member, and the sentinel and flag stores
that guard the ends.

- 66910c10: the corpus, `build/cycle19/c-gateway-wave-pin` (Fluent Bit and
  Monkey `monkey-source`).
- 046accf6: the system route,
  `private/w45-fb/w46-external-callee-contracts/attempt-1`.
- 34155955: the slice route, `private/w47-tail/attempt-1/prep`.

At 046accf6 and 34155955 every file below is byte-identical (sha256 prefix
in §1.4). At 66910c10 the scheduler, input-thread, lib, downstream-worker,
metrics and Monkey files are identical too. The engine, input, output, config,
output-thread and stream-processor files differ there only by line numbers
at these calls; the shapes are the same. Lines are given as `046accf6`
(`66910c10` in brackets where it differs). Baseline leaves were read from
`baseline:a9f7bebbe` (`private/baselines/a9f7bebbe…/`). Owners were found
with the scratch reader in the card's scratchpad (`census.py`), and every row
was checked by hand.

### 1.2 Channels: create and end in different functions

"Holder" is the record whose members hold the cells after the create. It
might be fresh in the creator and returned (R), or a caller's formal (F). "End" is
how each cell is closed, and the test that guards it.

| # | Create (unit:line) | Holder · cells | End (function:line) and its guard | Functions between | Row at the baseline (first refusal at the cell call) |
|---|---|---|---|---|---|
| C1 | `flb_sched_create` scheduler:794 | R `sched`: `ch_events[0]`, `ch_events[1]`, `event.fd` (#same [0]) | `flb_sched_destroy`:856, `channel_destroy` under `sched->ch_events[0] != -1`, then `= -1` stores (:860–861) | `flb_engine_start`:1288 [1252] creates it and stores `config->sched = sched` (:1293 [1257]) → `flb_config_exit` (flb_config.c:687 [686]) → `flb_sched_destroy`; also `input_thread` (input_thread.c:340 → :517) and `output_thread` (:213 → :404), each in its own thread; also the failure arm :799 | creator: the access gate at :794; destroyer: `descriptor-cell-guard-undecided` :856, beside `unresolved resource identity` `sched` (:855–864) and `request`/`timer` |
| C2 | `flb_engine_start` engine:1168 [1152] | F `config`: `ch_self_events[0..1]`, `event_thread_init.fd` | `flb_engine_shutdown`:1711 [1675], `channel_destroy` under `if (config->evl)`: no cell sentinel | the same body (:1440, :1561 call `flb_engine_shutdown`), the pipeline thread `flb_lib_worker` (flb_lib.c:926 → :929, after a failed start) and `flb_destroy` (:247) | creator: the gate at :1168; destroyer: `guard-undecided` :1711, beside `unresolved resource identity config` |
| C3 | `flb_engine_start` engine:1237 [1201] | F `config`: `ch_manager[0..1]`, `ch_event.fd` | `flb_config_exit` (flb_config.c:617–621): `mk_event_closesocket` of both ends under `> 0` and `[0] != [1]`; never `channel_destroy` | `flb_engine_start` → (caller) → `flb_config_exit` | creator: the gate at :1237 |
| C4 | `flb_engine_start` engine:1246 [1210] | F `config`: `notification_channels[0..1]`, `notification_event.fd`; the flag `notification_channels_initialized = FLB_TRUE` (:1256 [1220]); `[1]` copied into `ins->notification_channel` (flb_input.c:2016, flb_output.c:1898, flb_filter.c:720) | `flb_engine_shutdown`:1718 [1682], `channel_destroy` under the flag `== FLB_TRUE`, then the flag `= FLB_FALSE` | as C2 | creator: the gate at :1246; destroyer: `guard-undecided` :1718 |
| C5 | `input_thread_instance_create` input_thread.c:262 | R `thi`: `ch_parent_events[0..1]`, `event.fd` | `input_thread_instance_destroy` :210–214, `closesocket` of each end under `> 0` | `flb_input_thread_instance_init` (:622, stores `ins->thi`) → thread `input_thread` (another thread) → `input_thread_instance_destroy` (:518); failure arms :269–310 and :632 | creator: the gate at :262 |
| C6 | same :275 | R `thi`: `ch_thread_events[0..1]`, `event_local.fd` | same body :218–222, `closesocket` under `> 0` | as C5 | creator: the gate at :275 |
| C7 | same :287 | R `thi`: `notification_channels[0..1]`, `notification_event.fd`; flag `= FLB_TRUE` :300; `[1]` copied into `ins->notification_channel` :303 | `input_thread_instance_destroy`:197, `channel_destroy` under the flag `== FLB_TRUE`, then `= FLB_FALSE` | as C5; both functions are `static` (internal linkage) | creator: the gate at :287; destroyer: `guard-undecided` :197 |
| C8 | `input_instance_channel_events_init` flb_input.c:1380 [1372] (`static`) | F `ins`: `ch_events[0..1]`, `event.fd` (first member, rule 38 (1)) | `flb_input_instance_destroy` :1311–1316 [1304–1308], `closesocket` under `> 0` | `flb_input_instance_init` :1993 (threaded arm) / :2010 (else arm) → … → `flb_input_instance_destroy` | **reaches rule 38**: `descriptor-cell-escapes` at exit :1401 for all three cells, plus `retention-undecided` :1380 |
| C9 | `flb_output_init_all` flb_output.c:1545 [1463] | F loop element `ins`: `ch_events[0..1]`, `event.fd` | `flb_output_instance_destroy` :615–620 [538–542], `closesocket` under `> 0` | the same body's failure arm (:1552), and `flb_output_exit` (:671) | creator: the gate at :1545 |
| C10 | `flb_create` flb_lib.c:209 | F `config` (from `flb_config_init`): `ch_notif[0..1]`; data `ctx->event_channel`, a pointer read | `flb_engine_failed` (engine:1063–1065 [1048]) `closesocket([1])` under `[1] != [0]`, then `= -1`; `flb_config_exit` :625–629 `closesocket` under `> 0` | `flb_create` → `flb_destroy` → `flb_config_exit`; `flb_engine_failed` is called from `flb_lib_worker` (flb_lib.c:928) and `flb_engine_start` (:1439) | creator: the gate at :209 |
| C11 | `flb_output_thread_pool_create` output_thread.c:537 [530], :553 [546] | F `th_ins`: `ch_parent_events`, `notification_channels` | `output_thread` :418 [411], :424 [417], another function on another thread; the failure arm :560 [553] in the same body | thread spawn | in no measured unit |
| C12 | `mk_server_create` monkey.c:115, :136; `mk_sched_launch_worker_loop` mk_scheduler.c:365 (corpus Monkey) | R/F `server`, `sched`: `lib_ch_manager`, `lib_ch_start`, `signal_channel_r/w` | **never closed** (process lifetime; the ends are only read and written) | none | the corpus rows refuse before rule 38 (the access gate) |

Same-function pairs, which this card does not touch: `output_thread` :238 →
:386 (`event_local` is a local) and `downstream_worker_thread` :248 → :334.

### 1.3 Timeouts: create and end in different functions

A timeout cell is the `fd` member of one `struct mk_event` inside a record.
Rule 38 (4) reads `timeout_destroy` of a cell this function does not record
as the identity on custody, under `P-CELL-STATE-NOT-FORWARDED`. So a
cross-function destroyer passes, and only the creator refuses.

| # | Create | Holder | End and guard | Row at the baseline |
|---|---|---|---|---|
| T1 | `schedule_request_now` scheduler:133 | F `timer->event` (`event` is a local copy of `&timer->event`) | through `sched->timers`: `flb_sched_destroy`:843 (a list element) or `flb_sched_timer_cb_disable`:723 | `descriptor-cell-undecided` :133 (the key `event` is a local pointer, not `&X->f`) |
| T2 | `flb_sched_timer_cb_create` :672 | R `timer` from the uncontracted `flb_sched_timer_create`; the number copied to `timer->timer_fd` | `flb_sched_timer_cb_disable`:723 under `timer->timer_fd != -1` (the copy), then `= -1`; `flb_sched_destroy`:843 over the list | `cb_disable` is **Safe** at the baseline (an untracked destroy); `cb_create` refuses earlier |
| T3 | `flb_sched_create` :783 | `timer->event` of the frame timer, linked into `sched->timers` by an uncontracted call; the number copied to `sched->frame_fd` | `flb_sched_destroy`:843, list traversal | `descriptor-cell-undecided` :783 |
| T4 | `collector_start` flb_input.c:2351 [2333] | F `coll->event` | `flb_input_collector_pause`:3182 [3162] (`coll->evl`), `flb_input_collector_destroy`:3146 [3126] (`config->evl`); re-created by `_resume`:3248 [3228] | `descriptor-cell-undecided` :2351 |
| T5 | `flb_engine_flush_timer_reset` engine:325, :337; `flb_engine_start` sets `event = &config->event_flush` (:1327) | F `config->event_flush`; the number copied to `config->flush_fd` | the same body's `timeout_destroy` :320 (before re-creating); `flb_config_exit` :681 under `config->event_flush.status != MK_EVENT_NONE` | refuses earlier (`flb_utils_error`, round 4) |
| T6 | `flb_me_create` metrics:234 | R `me->event` | `flb_me_destroy` :261 (`me->config->evl`) | not in a measured unit |
| T7 | `flb_sp_task_create` flb_sp.c:519, :535 | `task->window.event` | `flb_sp_window_destroy` :671; `flb_sp_fd_event` :2191 destroys and :2194 re-creates | not in a measured unit |

Same-function pairs: `flb_engine_start` :1498 → :1564 (`shutdown_fd`
sentinels) and `mk_server_worker_loop` :502 → :586 (corpus).

### 1.4 Digests (sha256 prefix, 046accf6 = 34155955 / 66910c10)

`flb_scheduler.c` 5e2c36c2d741 / same; `flb_input_thread.c` ac3517f13136 /
same; `flb_lib.c` 93113329f2a3 / same; `flb_downstream_worker.c`
eef1192e8246 / same; `flb_metrics_exporter.c` 65e0de81f513 / same;
`flb_engine.c` 2f70e703eb8f / 8a941d363c5c; `flb_input.c` 92a0917b89dd /
2056b5856e4c; `flb_output.c` 2d31e29a14f4 / 2f6cde94c613; `flb_config.c`
289b881eae45 / 1b7cc310ce3e; `flb_output_thread.c` b6d1e7ad71b9 /
932ed5789941; Monkey `mk_server.c` 07a15b43f175, `mk_scheduler.c`
9a515fa6d0cc, `monkey.c` 821fe95aa0d8 (all three).

### 1.5 What the census shows

1. **Creators mostly refuse before rule 38.** At `baseline:a9f7bebbe`, every
   production channel creator except C8 is refused by the access gate at the
   creation (`indirect memory access is outside the resource projection`).
   Each also has unresolved identities of its holder (`sched`, `thi`,
   `config`, `ins`) at earlier statements.
   - So the brief's first case ("stays live in a caller-visible record at
     exit, `descriptor-cell-escapes`") holds today only for C8
     (`input_instance_channel_events_init`).
   - It would hold for C1–C7 and C9 once their gate and identity refusals
     clear. That is D6's ordering ("Blockers first"), and part of it is the
     concurrent card RETAINING-CALL-MINTED-NAME-ACTUALS's area.
2. **Every production end is guarded by a member sentinel or flag, never by
   the creation's witness.** The guards are:
   - `!= -1` on the slot (C1);
   - `== FLB_TRUE` on a separate flag member (C4, C7);
   - `> 0` (C3, C5, C6, C8, C9, C10);
   - a copy of the number in another member (`timer_fd != -1`, T2);
   - the event's registration status (`status != MK_EVENT_NONE`, T5);
   - or nothing related to the cell (`if (config->evl)`, C2).

   Rule 38 (3)'s witness is "the call's result held by a plain name, until
   the name is written". It never reaches a destroyer in another function.
3. **Half of the ends are not a cell-bearing call.** C3, C5, C6, C8, C9 and C10
   end through `mk_event_closesocket(X->f[K])`, which is a function-map `close`
   of an integer member read. Rule 38 reads a release only at the four declarations.
4. **Further names.** The write end or the number is copied into other
   members or other records: `sched->frame_fd`, `timer->timer_fd`,
   `config->flush_fd`, and `ins->notification_channel` (three units).
5. **The holder reaches the destroyer through records, lists and threads.**
   - Records: `config->sched`, `ins->thi`.
   - Lists: `sched->timers`, the output and input instance lists.
   - Threads: `input_thread`, `output_thread` and the engine thread.

   Only C7's pair, and C5/C6's failure arms, are same-unit calls with internal
   linkage. `flb_sched_destroy`, `flb_engine_shutdown` and `flb_config_exit`
   are external and sit in other units from most of their callers.

## 2. Do the decided forms cover forwarding a cell effect?

No. Each decided form, and what it stops short of:

- **Descriptor cells (§C04.6.2, rule 38)** are read in one body.
  - At an exit, a cell that may be live refuses (`descriptor-cell-escapes`,
    "never `Safe`: the cross-function reading is
    `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`'s").
  - An untracked release is the identity on custody under
    `P-CELL-STATE-NOT-FORWARDED` ("a state a call puts on a descriptor cell of
    an object this function does not track is not forwarded into its
    summary").
  - So no cell state crosses a call in either direction.
- **S3 forwarding (rule 12t's S3 amendment, §C04.21)** forwards an outgoing
  *live* record only under a declared footprint: "an unconditional declared
  record footprint may forward an exact caller-owned record". In code,
  `legacy_retained_by.rs` installs the forward only when some declaration
  carries a `quant registered` protocol. It then attributes it to
  `ForwardedLifetime::premise`.
  - The footprint names a registration (interior → holder), not a descriptor
    identity.
  - No decided clause states a record type's cells.
- **Exact ordered transport (EVENT-ORDERED-LIFETIME)** carries removal-only
  traces only. The trace must be one acyclic path of removal calls with exact
  formal indices, and "branches, aliases, writes, releases and mixed
  registration/removal classes remain unsupported". Every production
  destroyer branches on a sentinel and writes a member.
- **Holder frames (EVENT-HOLDER-IDENTITY)** prove that a holder member is
  preserved. They carry no cell.
- **D3** derives loop-path equalities only. It says nothing about a sentinel
  or flag relating a member's value to a cell's liveness.
- **§C04.21 caller prerequisites** export an obligation from a body only for
  a *declared* clause inside a block: "an inner `requires` remains an
  obligation, never a fact minted by the annotation". The listed kinds are
  named, loop-owner, thread-sync, lifecycle and registered-lifetime
  prerequisites. None is a cell state.

So forwarding needs two things that no decided form provides:

1. **An outgoing cell effect** from a creator: "the record at this exact
   path holds a live identity in these cells, with this second name". By the
   S3 pattern, an outgoing live record is forwarded only under a declared
   footprint.
2. **A destroyer's entry state.** A destroyer's guard is decided only from a
   fact about its formal's cells at entry. That fact is either imported from
   each caller's exact state (the guard is decided at the caller, and the
   destroyer forwards its guarded trace), or declared as a record-type
   protocol that relates the sentinel or flag to liveness and is checked
   against every store of the run.

Both are choices between real alternatives (§3), and the declared one needs
a new clause spelling. Per the brief, this card stops here.

## 3. Design and alternatives

All three share the brief's invariants:

- a forwarded cell equals the in-body reading;
- an unresolved forward never decides;
- a forwarded cell ends only by its own close (`#stale`) or its holder's
  release.

They also share the fail-closed ends: an unknown record identity, a
conditional path, a threaded join order and an alias each keep refusing.

### A. Derived exact cell transport (no clause spelling)

- **Channel.** Extend the effect-bearing lifetime record
  (`InferredLifetimeSummary`, `loop_lifetime_summary.rs`) with:
  - per exit, the cells whose keys are rooted at a formal or at the
    result, with the identity classes (`Live`, `Stale`, `#same` groups);
  - the ordered cell trace of a destroyer: guarded releases whose guard
    is stated over formal-rooted cell keys.
- **Publication.** The record is published only where the existing S3
  admission holds:
  - the body is otherwise `Safe` and diagnostic-free;
  - internal linkage, or an independently replayed home;
  - exact substitution, with stable snapshots;
  - the same trace on every exit, at most 4096 steps.
- **At the caller.** The caller substitutes actuals and replays: outgoing
  cells enter its table, and a destroyer's guarded release is decided
  against its exact incoming state (rule 38 (5)).
- **The premise.** `P-CELL-STATE-NOT-FORWARDED` stays for every call
  without such a record.
- **Lean.** `QCore.DescriptorCellTransport`:
  - `a_forwarded_cell_equals_the_in_body_reading`;
  - `an_unresolved_forward_never_decides`;
  - `a_forwarded_cell_ends_only_by_its_close_or_its_holder`.
- **What it decides.** Fixtures where a static helper creates into, or
  destroys through, its caller's record, and the caller holds the other end.
  It leaves every sentinel and flag test undecided.
- **Yield at these revisions: none.**
  - Every production creator except C8 refuses at the gate.
  - C8 has the uncontracted `flb_input_event_loop_get` and a retention
    refusal, so it publishes no summary.
  - Every production destroyer branches on a sentinel and has unresolved
    identities.
- **The departure.** It forwards an outgoing *live* state without a
  declaration. The S3 pattern requires one for registrations
  (`quant registered`). Removal traces are forwarded without one, but they
  carry endings only.

### B. Declared record-type cell protocols (S3 for cells; needs a spelling)

- **Model.** A provider module declares, for a record type, which members
  are cells of one descriptor identity, and how a member sentinel or flag
  relates to their liveness. Rule 15w's discipline checks the declaration
  against every creation, release and store of those members in the run:
  - a contradicted declaration is withdrawn;
  - an uncheckable one is a named declared assumption (rule 13i).
- **Use in a body.** A validated protocol decides a destroyer's sentinel or
  flag test in its own body. On the edge where the protocol says "live", the
  member-equality guard reads two names of one live identity, so it is
  `Equal`, and the guarded release ends it. The same reading lets a
  creator's exit establish the protocol instead of escaping.
- **A's transport.** A's transport is the composition that B's validated
  declarations feed, as S1's summaries feed S3 in the cross-function
  report.
- **What a declaration must say**, for C1 as the example:
  1. the binder is a record type, local to the clause (like `quant
     registered #coll`);
  2. the cells: an array-element member (`flb_sched.ch_events[0]`,
     `[1]`) and the event member (`flb_sched.event.fd`) as a second name
     of `[0]`;
  3. the liveness relation, both ways: `ch_events[0] != -1` exactly when
     all three name one live identity.

  C4 and C7 use a flag member (`notification_channels_initialized ==
  FLB_TRUE`). The `> 0` ends (C3, C5, C6, C8–C10) also need decision Q3.
- **A candidate for the decision (not implemented).** It is built only from
  decided pieces: `quant live`, `#P: T.F`, `#fresh(descriptor)`,
  `#same(S)`, `when`, and the `quant registered` record binder:

  ```qcore
  quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
  quant cell #s: flb_sched.ch_events[1] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
  quant cell #s: flb_sched.event.fd |-> #same(ch_events[0]) when #s.ch_events[0] != -1;
  ```

  New in it:
  - the clause-kind word `cell`;
  - a member-valued guard `#s.member` (the cross-function report noted
    that member guards extend decision 2, which allows only integer
    parameters);
  - an array-element member path;
  - the biconditional reading of the guard (the complement edge means no
    live cell).

  The user may prefer another spelling.
- **Lean.** `QCore.DeclaredCellProtocol`:
  - `a_validated_protocol_equals_the_in_body_reading`;
  - `a_withdrawn_protocol_never_decides`;
  - `the_sentinel_edge_decides_only_under_a_validated_protocol`;
  - A's three theorems.
- **Yield at these revisions.**
  - It can decide the guard leaves of `flb_sched_destroy` (:856) and
    `input_thread_instance_destroy` (:197).
  - It can decide C4's `flb_engine_shutdown` :1718, if every store of the
    flag is in the run.
  - C2's :1711 (`if (config->evl)`) stays undecided.
  - Every row keeps other leaves: unresolved identities, uncontracted
    calls, list traversals, loop destroys at a path.
  - So **no status move** is expected. A move to `Safe` would be a
    soundness finding.
  - Creators gain nothing until their gate refusals clear (D6).

### C. Records only: defer until the creators' blockers clear

Register this census as an amendment of
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` and build nothing yet, by the
reasoning of D6. Eight of the nine production creators are refused by the
access gate before any cell is recorded. Neither A nor B moves a row here.

### Recommendation

**B, staged with A as its composition, after Q2 fixes the spelling.**

- This follows D1/D5. The user chose declared protocols for the
  cross-function lifetime model, and the decided S3 forwarding of a live
  record requires a declared footprint. A forwards live cell state without
  one, which is a different policy from S3's, and only the user can choose
  it.
- B is also the only alternative that can decide the production
  destroyers' sentinel and flag tests. A's exact transport alone decides
  only fixtures.
- If the user wants measurable movement first, choose **C**. Then the
  creators' gate and identity refusals (C1–C7, C9, C10) become the prior
  card, as D6 ordered for registrations.

## 4. Rows a phase 2 could touch (the prediction to come)

Under A or B, the rows that can trade leaves are:

- system: `flb_sched_destroy`, `flb_engine_shutdown`, and possibly
  `flb_sched_create`;
- slice: `input_thread_instance_destroy` and
  `input_instance_channel_events_init`.

Each has two units. No row is predicted to change status: each keeps
unrelated leaves (§1.2's last column). The Safe row `flb_sched_timer_cb_disable`
(system) must stay `Safe`. Its untracked timeout destroy keeps
`P-CELL-STATE-NOT-FORWARDED`, because neither alternative removes the premise
where no forwarded record exists. PREDICTION.md will relist these rows at
`baseline:a9f7bebbe` before any code.

## 5. Observations (outside the decision)

- **C2's guard is not a cell sentinel.** `flb_engine_shutdown` destroys
  `ch_self_events` under `if (config->evl)` alone. It is reached from
  `flb_lib_worker` (flb_lib.c:929) after `flb_engine_start` failed, which may
  be before the create at :1168.
  - The two members then hold whatever `flb_config_init` left. If both are
    0, the backend's `event->fd == r_fd` holds and it closes descriptor 0.
  - This reading is unconfirmed (`flb_config_init`'s initial values were
    not read here), so it is no finding, and certainly not an upstream
    defect claim.
  - Under every alternative it stays `descriptor-cell-guard-undecided`. A
    follow-up would read the initial values.
- **Ends through `close`.** C3, C5, C6 and C8–C10 close channel ends with
  `mk_event_closesocket` and never remove the read end's registration
  (`event`). The loop's destroy ends that registration (rule 12t). Reading
  a function-map `close` of a cell member as the cell's `#stale` is Q3.
- **Monkey's process-lifetime channels** (C12) are never closed. A
  forwarded cell there ends by neither its close nor its holder's release,
  so it stays refused under every alternative, which is correct.

## 6. Decisions requested

1. **Q1, the approach:**
   - **B** (declared record-type cell protocols, with A's exact transport
     as their composition), the recommendation;
   - **A** (derived exact transport only, with no declaration, forwarding
     live cell state without a footprint);
   - **C** (records only, until the creators' gate and identity blockers
     clear).
2. **Q2, the spelling (only if B):** accept the §3 B candidate, or give
   another. Q2 has four parts:
   - the word `cell`;
   - the member-valued guard `#s.member OP V`, whose operators are `!=
     -1`, `== FLB_TRUE` and `> 0`;
   - array-element members;
   - the biconditional reading.
3. **Q3, `close` as a cell end:** whether a function-map release of an
   integer read from a declared cell member (`mk_event_closesocket(ins->ch_events[0])`)
   counts as that cell's `#stale`. Recommendation: yes, but only at a
   validated protocol's member and only for that one name.
   - The other names of the identity (`event.fd`) are staled with it.
   - The read end's registration is left to the loop's destroy, as now.
   - Without it, C3, C5, C6 and C8–C10 stay refused under A and B alike.
4. **Q4, numeric copies:** whether `frame_fd`, `timer_fd`, `flush_fd` and
   `ins->notification_channel` may be declared as further names
   (`#same`). Recommendation: not in this card. They stay refused, and
   rows T2 and T5 keep their current readings.

## 7. Decision (2026-11-02)

> **USER DECISION 2026-11-02** (verbatim, relayed by the coordinator): "B as
> recommended".

The coordinator's reading accepts §6 as written:

- **Q1** is B: declared record-type cell protocols, with exact transport.
- **Q2** is the §3 B candidate.
- **Q3** is yes: a close of a declared cell member is its `#stale`.
- **Q4** is no: copied numbers are not declared.

### The grammar, finalized (G05i, total or reject)

The grammar document numbers it G05i, because G05e to G05h were already taken.

```qcore
quant cell #B: TAG.CELL |-> #fresh(descriptor) when #B.GUARD OP V;
quant cell #B: TAG.CELL |-> #same(CELL2) when #B.GUARD OP V;
```

The parts:

- `#B` is a binder local to the clause, as in `quant registered`. It is not
  a formal of the host and not a reserved `#` word. One binder is used per
  record in a declaration.
- `TAG` is a record tag.
- `CELL`, `CELL2` and `GUARD` are member paths: identifiers joined by `.`,
  where a segment may carry one `[K]`, with `K` a canonical decimal.
  Examples are `ch_events[0]`, `event.fd` and
  `notification_channels_initialized`.
- `OP V` is exactly one of three pairs:
  - `!= -1`, a sentinel cell;
  - `> 0`, a sentinel cell;
  - `== 1`, a flag member. `FLB_TRUE` is `true`, which is `1` after
    preprocessing.

The rules for one record's clauses, all in one declaration:

- Every clause has the same guard.
- At least one clause is `#fresh`.
- The cells are distinct.
- A `#same(CELL2)` names a cell stated `#fresh` in the same record.
- Under `!= -1` and `> 0`, the guard is one of the `#fresh` cells.
- Under `== 1`, the guard is no cell.

Two declarations stating different protocols for one record leave that
record without a protocol, so rule 38c reads none (fail-closed). Every
other spelling rejects the whole module (`quant-cell-*`). The clause adds
nothing to the host's own contract. Like `quant registered`, it makes the
host a withheld lifetime declaration.

### Meaning (rule 38c)

A record's protocol is a declared assumption (rule 13i), and every row that
reads it names its host as a dependency.

- **Its claim.** Whenever a `struct TAG` object's guard satisfies `OP V`,
  each `#fresh` cell names its own live descriptor, and each `#same` cell
  names the same descriptor as its target. Otherwise, no cell of it names a
  live descriptor.
- **The guard test.** Route G reads a test of exactly `r->GUARD OP V`, or
  its complement, where `r` is a caller's record-pointer formal of type
  `struct TAG *` that this body has not touched. On the live edge the test
  introduces the cells as the caller's live identities. On the other edge
  it records that none is live.
- **What a test, a store, a creation, a release or a close of such a cell
  must leave.** At an exit, and at a call that passes the record on, the
  record must again satisfy the claim:
  - every cell is live under its group's identity and the guard says live;
    or
  - no cell is live and the guard says dead.

  A record that satisfies it is forwarded under the protocol, in place of
  `descriptor-cell-escapes`. Any other state refuses
  (`descriptor-cell-protocol-broken`).
- **Guard values and stores.**
  - A creation into the guard cell of a `!= -1` protocol makes the guard
    say live. A live descriptor is never `-1`.
  - Nothing a creation does establishes `> 0`, because a descriptor may be
    0.
  - Under `== 1`, only a store of `1` makes the guard say live, and only
    when every cell is live. A store of the dead value is admitted only
    when no cell is live.
- **Q3.** A `close` of `r->CELL`, after the protocol has been read for `r`,
  is that cell's `#stale`.
- **Nothing is accused.** Rule 38c never yields `Unsafe`, and with `38` in
  `QCORE_GATEWAY_RULES_OFF` it reads nothing.
