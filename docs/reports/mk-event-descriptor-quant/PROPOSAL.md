# MK-EVENT-DESCRIPTOR-QUANT: proposal (phase 1)

Card authority: ledger entry `MK-EVENT-DESCRIPTOR-CUSTODY`
(`docs/reviews/r0_3/acceptance_matrix.json` `named_followups`; DISCHARGE: "a
descriptor out slot or a member-held descriptor identity in v1, the four
declarations restated with it (bodies read at both revisions, pinned),
measured"). Base `eb49bb6ce`. This file is the phase-1 proposal only: nothing
is implemented, and no grammar, loader, gateway, Lean or contract file
changes until the user decides.

> **USER DIRECTION 2026-11-01** (verbatim): "We must decompose the premise
> into quant like composed clauses."

The premise is `requires resource_premise(mk_event_descriptors, ...)` on
`mk_event_timeout_create`, `mk_event_timeout_destroy`,
`mk_event_channel_create` and `mk_event_channel_destroy`
(`providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`).

## 1. The bodies

Read at three audited inputs. `lib/monkey/mk_core/mk_event.c` is byte-identical
at all three (`8ef958d2…`): Fluent Bit 66910c10, 046accf6 (system) and
34155955 (tail-pause slice), and the corpus's own Monkey source. The epoll
backend `mk_event_epoll.c` is `0bbc8f00…` at 66910c10 and `a2a16119…` at
046accf6 and 34155955. The only difference is `pthread_detach(tid);` at
046accf6:335, in the pipe arm. That arm is not compiled:
`MK_HAVE_TIMERFD_CREATE` is defined in the production build's
`lib/monkey/mk_core/CMakeFiles/mk_core.dir/flags.make`. `struct mk_event`'s
first member is `int fd` (`mk_event.h:104`). `MK_EVENT_NEW` writes only
`mask` and `status` (`mk_event.h:140`), so no removal resets `fd`. Each
public operation forwards to its backend through `loop->data`. The lines
below are in the epoll backend unless stated.

| # | Operation | Descriptor effect, read from the body |
|---|---|---|
| 1 | `mk_event_timeout_create` (`mk_event.c:156` → `:190`) | A `clock_gettime`, `timerfd_create` or `timerfd_settime` failure returns -1 with `event->fd` untouched (`timer_fd` is closed on the third). Otherwise `event->fd = timer_fd` (`:233`). If `_mk_event_add` then fails, the body closes `timer_fd` and returns -1, **leaving the closed number in `event->fd`** (`:240-243`). On success it returns `timer_fd` (≥ 0), which `event->fd` also holds. |
| 2 | `mk_event_timeout_destroy` (`mk_event.c:172` → `:341`) | A NULL `data` returns 0 and does nothing. Otherwise `_mk_event_del` runs (it does nothing when the event is unregistered), then `close(event->fd)` runs **unconditionally**, and `event->fd` is never written. Afterwards the member still holds the closed number. A second destroy closes it again. |
| 3 | `mk_event_channel_create` (`mk_event.c:181` → `:355`) | When `pipe` fails, it returns -1 and writes nothing. Otherwise `event->fd = fd[0]` (`:371`). If `_mk_event_add` fails, both ends are closed and -1 is returned: `event->fd` keeps the closed `fd[0]`, and `*r_fd` / `*w_fd` are not written. On success `*r_fd = fd[0]`, `*w_fd = fd[1]` and the result is 0. `event->fd` and `*r_fd` then name one descriptor. |
| 4 | `mk_event_channel_destroy` (`mk_event.c:193` → `:389`) | When `event->fd != r_fd` it returns -1 and touches nothing (`:397-399`). Otherwise `_mk_event_del` runs (the registration removed; the status cleared even through a loop that does not hold it), then `close(r_fd)` and `close(w_fd)`, and the result is `_mk_event_del`'s, 0 or -1. So -1 does not tell the two arms apart, and C04.13's return-code split cannot state this effect. |

Two facts are not in the brief's list.

- **The member ends stale in the failure class.** Effects 1 and 3 leave a
  *closed* number in `event->fd` on their `_mk_event_add` failure arm.
- **The destroy does not clear the member.** Effect 2 leaves the member stale
  after a destroy. In C04.19.6's words that is `#stale`, not `#absent` and not
  `#released`.

## 2. The design: descriptor cells in the `quant live` family

Every effect is about a place that holds a descriptor number: the event's
member `fd`, or the `int` that `r_fd` / `w_fd` point to. The proposal names
such a place a **cell**. A cell gets a state with the forms the user already
chose:

- C04.19.6's `quant live #P: TAG.PATH |-> V [when …]` states.
- The set `{V, #kept}`.
- C04.14.1's guards.
- C04.19.4's two levels: a shorthand plus its canonical parts, read as one
  canonical set, with the equivalence proven in Lean.

The cells are:

- **Member cell** `#P: TAG.FIELD`: the member of the object the binder `P`
  designates, here `#data: mk_event.fd`. It is the existing member-path left
  side, read as a cell when the member's C type is an integer. Clang checks
  that per unit; a non-integer member at a descriptor-kinded state refuses.
- **Slot cell** `#S: #pointee`: the `int` that the slot formal `S`
  (`ptr<mut, u8>`, C type `int *`, checked by Clang) points to. `#pointee` is
  a new left-side keyword.

**Custody model.** The custody rules come from C04.6, unchanged.

- A descriptor is an **identity**, and a cell is a **name** for it, like a
  plain name today (C04.6 rule 4: "a copy into a plain name aliases the same
  identity").
- A release through any name of an identity discharges that identity once.
  Every other name of it is then stale.
- A cell's kind is written on its value (`#fresh(descriptor)`,
  `#stale(descriptor)`). A bare `#stale` keeps meaning a heap link (rule 30,
  unchanged), so heap stays spelled by absence (C04.6).
- A call's integer result is a **witness**, as an in-place status is
  (C04.6.1 rule 2). A plain, not address-taken name that holds the result
  witnesses the cell until the name is written. A validity test of it
  (`fd == -1`, `ret != 0`, …) decides the class on each edge. A copy or a
  store of the witness is data: it owns nothing and names nothing. This keeps
  every Fluent Bit caller's `timer->timer_fd = fd` / `coll->fd_timer = fd`
  store out of rule 4's refusal.

**Which cell owns** (the brief asks this to be stated). No single cell owns;
the identity carries the custody. For the timer, the identity's only name
is the **member** `event->fd`, and the result is a witness. For the channel,
the read end's names are the slot `*r_fd` and the member `event->fd`, and the
write end's name is the slot `*w_fd`.

### Effect 1: `mk_event_timeout_create` creates a descriptor into the member

**Recommended (1A).**

```qcore
quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;
quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1;
ensures normal resource_return_none();
```

- **Meaning.**
  - In the class `#result != -1`, the call created one descriptor identity,
    and the event's `fd` member is its name.
  - In the class `#result == -1`, the cell names no live identity. The member
    is untouched, or holds a number the call already closed.
  - The result is the witness, so a caller's `fd == -1` test decides the
    class.
- **Two levels.**
  - The two clauses are the shorthand.
  - The canonical parts are:
    - the binder's mode `resource_borrowed(data)` (its liveness part,
      restated, as C04.19.4's shorthand restates `resource_return_owned`);
    - the cell's kind `resource_member_kind(data, mk_event.fd, descriptor)`;
    - the per-class states `#fresh` / `{#stale, #kept}`, written alone as
      `resource_on_return(ne, -1, resource_member_out(data, mk_event.fd))` and
      `resource_on_return(eq, -1, resource_member_stale_or_kept(data, mk_event.fd))`.
  - This lower level is C04.13's disposition with a member operand. The loader
    elaborates every spelling into the canonical parts, so each level, and any
    mix of the two, loads to one set.
- **Composition.**
  - The cell is independent of `resource_retained_by(data, loop)`: the
    retention record is keyed on (event, loop), the cell on (event, `fd`).
  - The new descriptor follows every C04.6 rule as a descriptor-returning call
    does: validity narrowing (rule 2), kind matching (rule 1) and use after
    release (rule 6).
  - At the caller's exit, a live identity whose only names are cells of
    objects that outlive the frame (a caller's object, an escaping tracked
    object) **refuses** (`descriptor-cell-escapes`), and is never `Safe`.
    This follows rule 12t's path-held records, whose cross-function reading
    is follow-up `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (T1/T2).
- **Alternatives.**
  - **1B, the brief's sketch:** `quant live #data: mk_event.fd |-> #result;`
    beside `resource_return_owned(sentinel)` and
    `resource_return_kind(descriptor)`. The result is owned and the member
    keeps a copy of it. Every Fluent Bit caller stores the result into a
    member (`timer->timer_fd = fd`, `request->fd = fd`,
    `sched->frame_fd = fd`, `coll->fd_timer = fd`,
    `config->flush_fd = mk_event_timeout_create(…)`), and rule 4 refuses
    each store. The destroy releases through the member, which never resolves
    back to the result's identity across functions. The form also says nothing
    of the failure-class stale member.
  - **1B′:** read `|-> #result` as a store that moves custody into the
    member. The caller then sees 1A, but through two custody facts for one
    identity, and the failure class is still unstated. Not recommended.
  - **1C:** write only the lower level. It is the same canonical set.

### Effect 2: `mk_event_timeout_destroy` closes the descriptor the member names

**Recommended (2A).**

```qcore
quant live #data: mk_event.fd |-> #stale(descriptor);
quant live #loop: #holds(data) |-> #absent;          // unchanged (R3)
```

- **Meaning.**
  - The identity the event's `fd` member names is released (closed), and the
    member still holds the number.
  - A NULL `data` designates no object, so the state is the identity there,
    as the body's `data == NULL` arm is.
- **Two levels.** The shorthand decomposes into:
  - `resource_borrowed(data)`;
  - `resource_member_kind(data, mk_event.fd, descriptor)`;
  - the state `#stale` (lower level
    `resource_member_released(data, mk_event.fd, stale)`).
- **Composition.**
  - **With the removal.** The removal and the release touch different
    records, so they commute (the body removes first, then closes). The
    removal keeps its R3/R1 semantics and its poisoning (C04.14.3). Poisoning
    reaches records and never cells.
  - **At a direct call (rule 30).** A cell that names a tracked identity is
    released once, and a later release or hand-off of that identity through
    any of its names refuses (`member-path-double-release`). That is the
    second-destroy hazard, refused and never accused.
  - **With no tracked identity.** A cell that names no tracked identity (a
    caller's object, as in `flb_sched_timer_cb_disable(timer)` on
    `&timer->event`) leaves the call's custody reading unchanged. This is rule
    30's "no link" case. The row then publishes the rule premise
    `P-CELL-STATE-NOT-FORWARDED` ("a state a call puts on a cell of an object
    this function does not track is not forwarded into its summary"). It is
    a tracking limit, like `mk_event_descriptors`, and joins the C04.20
    exception list.
  - **With the kind rules.** The release is of descriptor kind (rule 1).
    The cell is an `int`, so it can carry no identity of another kind.
- **Alternatives.**
  - **2B, the brief's sketch:** `|-> #absent`. Rejected as false.
    `#absent` (C04.19.4) means that the member holds nothing, but the member
    keeps the closed number. It would hide the double close a second destroy
    performs.
  - **2C:** `|-> #released(descriptor)`. Rejected as false: `#released`
    means that the member reads cleared, and the body never writes it.
  - **2D:** refuse a state on an untracked cell instead of publishing
    `P-CELL-STATE-NOT-FORWARDED`. This is stricter, and it costs the two
    `Safe` instances of `flb_sched_timer_cb_disable` (§4).

### Effect 3: `mk_event_channel_create` writes two descriptors through slots

**Recommended (3A).**

```qcore
quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
quant live #w_fd: #pointee |-> #fresh(descriptor) when #result == 0;
quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;
quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result != 0;
ensures normal resource_return_none();
```

- **Meaning.**
  - In the class `0`, the slots receive two new descriptor identities, and
    the event's `fd` member is a second name of the one `r_fd` received
    (`#same(r_fd)`: one identity, two names, custody once).
  - In the other class, the slots are not written (`#kept`, the default
    complement of a guarded state). The member is untouched or holds the
    closed read end.
  - The result is the witness: `ret != 0` / `ret == -1` decides the class.
- **Two levels.** The slot shorthand decomposes into **existing words**:
  - `resource_on_return(eq, 0, resource_out(r_fd, nonnull))`;
  - `resource_on_return(ne, 0, resource_borrowed(r_fd))` (C04.13's "Out
    slots" paragraph: the slot left unwritten);
  - `resource_kind(r_fd, descriptor)`.

  Only the **descriptor out slot** relaxation is new: C04.6's table drops "a
  descriptor has no out slot", and its "Not in the vocabulary" line for
  `pipe` / `socketpair` narrows. The member clauses decompose as in effect 1,
  with `#same(r_fd)` at the lower level written
  `resource_member_alias(data, mk_event.fd, r_fd)`.
- **Composition.**
  - **The slot actual.** It is `&L` (a local `int`, tracked as a plain name),
    or `&X->f` / `&X->f[K]` for a constant `K` (a cell of `X`'s object, the
    Fluent Bit shape `&sched->ch_events[0]`). Anything else refuses, as
    C04.13's out slots do.
  - **The member actual.** The member cell resolves only when the `data`
    actual's pointee is `struct mk_event` or `&X->m` of that type. A record
    with a `struct mk_event` first member (`ins` at
    `input_instance_channel_events_init` and `flb_output_init_all`, Monkey's
    `server_timeout`) refuses. Reading the first member is follow-up
    candidate MK-EVENT-FIRST-MEMBER-CELL.
  - **The exit.** It reads as in effect 1. The retention stays as declared.
- **Alternatives.**
  - **3B:** `|-> #view` in place of `#same(r_fd)`, a copy that names no
    identity. It needs one keyword less, but effect 4's guard can then never
    be decided through the member.
  - **3C:** `quant out #r_fd: #fresh(descriptor) when #result == 0;`, a new
    clause-kind word instead of `#pointee`.
  - **3D:** keep only the lower level (C04.13 + C04.6) for the slots. That
    needs no new left side, but the slots are then not in the quant family.

### Effect 4: `mk_event_channel_destroy` closes and removes only when `event->fd == r_fd`

**Recommended (4A): state the condition as a member-equality guard.**

```qcore
ensures normal resource_kind(r_fd, descriptor);
ensures normal resource_kind(w_fd, descriptor);
ensures normal resource_released(r_fd, valid) when #data: mk_event.fd == #r_fd;
ensures normal resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd;
quant live #loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd;
```

- **Meaning.**
  - In the class where the event's `fd` member equals `r_fd`, both
    descriptors are closed and the registration is removed.
  - Otherwise nothing changes.
  - The member is not written. Under the guard it names `r_fd`'s identity,
    which is released through `r_fd`, so it needs no state of its own.
- **Two levels.** The guarded predicates are themselves the canonical form,
  as C04.14.1's guarded retention is. The **undecided reading** of each is the
  cautious join:
  - each release is `{released, kept}`;
  - the removal leaves its record and poisons what it may name, the union of
    the removal and nothing.

  The guard **partitions** every call (equal or not), as C04.14.1 requires.
- **How the guard is decided.** A consumer decides it from identities only.
  - It is equal when the member cell and the `r_fd` actual name the same
    tracked identity (`#same(r_fd)` from effect 3 in the same body).
  - It is unequal when they name two distinct *live* tracked descriptor
    identities. Two open descriptors never share a number (POSIX), which the
    Lean model states as a hypothesis of the theorem, never as an axiom.
  - Every other call is undecided and reads the join. A set refuses at a
    direct call (`member-path-undecided`), and a release whose actual is no
    tracked identity refuses as C04.6 decides.
- **Composition.**
  - **With the removal (R3).** It is the guarded removal above. This also
    states what the current no-removal declaration leaves out: on the
    equal arm `_mk_event_del` clears the event's status even through a loop
    that does not hold it, which is exactly the poisoning case of
    RETENTION-UNMATCHED-REMOVAL-POISONS. Today a channel destroy poisons
    nothing (§5, observation 1).
  - **With the kind rules.** The two releases follow rules 1 and 6, as
    `close` does.
- **Alternatives.**
  - **4B: keep it unstated under a narrowed premise.** Retire
    `mk_event_descriptors` from the other three, and keep only
    `resource_premise(mk_event_channel_teardown, "…closes r_fd and w_fd and
    removes the registration only when event->fd == r_fd")` here. Its
    justification: the condition is a member-against-parameter equality that
    no v1 guard reads, and no measured call site could decide it (§4). It
    misses the user's direction for this one declaration.
  - **4C: an unguarded cautious set.** It needs a parameter set mode
    ("maybe released"), which v1 lacks. Declaring `r_fd` / `w_fd`
    `resource_borrowed` without the premise would be **unsound**: a caller
    that tracks the read end would believe it still open.

## 3. Impact

### Grammar (G05 / G05d, total-or-reject; implemented grammars and C04.6, C04.13, C04.14.1, C04.14.3, C04.19.6, C04.18, C04.20 text)

- **Values on a member-path state.** The new values are `#fresh(KIND)`,
  `#stale(KIND)`, `{#stale(KIND), #kept}` and `#same(IDENT)`. `KIND` admits
  only `descriptor`. `IDENT` names a slot formal of the same declaration that
  carries a `#fresh` state. Every other kind or operand rejects.
- **Slot left side.** `#S: #pointee` names a depth-one `ptr<mut, u8>`
  formal stated by the slot form only. Its C `int *` type is checked per unit
  by Clang, and a mismatch makes the declaration unreadable there.
- **Result guard with a decimal.** A descriptor-cell state may take
  `when #result (==|!=) INT` (a canonical decimal that fits an `int`), with no
  hand-off. Today a result guard is admitted only as the `@`/alias complement
  of a hand-off.
- **Per-class pairs.** A cell may carry two states whose result guards
  complement each other and together cover every result. This amends "one
  state per path".
- **Member-equality guard.** `when #P: TAG.FIELD == #X`, with `P` a
  borrowed depth-one pointer and `X` a descriptor-kinded `i32` parameter, is
  admitted on a descriptor `resource_released` and on the `#holds` removal.
  Today guards stand only on `resource_retained_by` and on member-path
  states.
- **Placement.** Descriptor-cell states stand beside a retention and a
  removal. Today any retention beside a member-path state rejects the module.
- **Reserved keywords.** `fresh`, `same` and `pointee` join the reserved
  keywords of a `#` form.
- **Lower-level words.** `resource_member_kind`, `resource_member_out`,
  `resource_member_released(…, stale)`, `resource_member_alias` and
  `resource_member_stale_or_kept` are each admitted inside
  `resource_on_return` where C04.13 admits a predicate.
- **Retirement.** `mk_event_descriptors` leaves C04.18's text, C04.20's
  tracking-limit list and `crates/qcore-audit/src/outer_selection.rs:77`.
  `P-CELL-STATE-NOT-FORWARDED` joins that list.

### Gateway rules that read them

- **Rule 30, amended.** Descriptor-cell states at a direct call, with result
  guards decided by the witness.
- **Rule 11r (C04.13), amended.** Slot dispositions beside a retention;
  `&X->f[K]` slot actuals.
- **Rule 12t, amended.** The guarded removal and its undecided reading
  (keep and poison).
- **A new rule package for the cell reading.** It covers the cell bindings
  (keyed like rule 12t's path records, with a path-stability premise), the
  witness, `#same` aliasing, the exit refusal, and the untracked-cell case
  with its published premise. Its id is assigned at implementation after
  checking every numeric and lettered id: 38 appears "reserved, unused" in
  `w48-bu-external-callee-contracts/integration-amendments.json`.
- **Body checks, rule 30 / MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS.** These
  run on the Monkey unit that defines the four functions. The `void *` cast
  `event = data` in each backend helper ends the bounded reading, so the
  states stay declared assumptions (rules 13i–13k), never withdrawn. Phase 2
  pins all four at 66910c10, 046accf6 and 34155955.

### Lean (new `QCore.DescriptorCells`, audited; reuses `MemberPathDirectCall`, `RetentionPath`, `KindedCustody`, `RelationalPost`)

| Theorem | Statement |
|---|---|
| `each_shorthand_and_its_parts_mean_the_same` / `both_levels_are_one_canonical_set` | Each shorthand and its canonical parts read the same. For the slot form the parts are C04.13 + C04.6's own words. |
| `a_fresh_identity_is_owed_once` | A release through any of an identity's names discharges it exactly once. `the_same_alias_releases_once` is the two-name case. |
| `the_witness_decides_the_cell` | The valid edge names a live identity and the invalid edge names none. `an_untested_witness_never_discharges` holds: the join is never `Safe`. |
| `a_stale_cell_never_releases_again` | Built from `a_release_after_taken_is_a_double_release`. |
| `an_untracked_cell_decides_nothing` | A state on an untracked cell is the identity on custody. |
| `an_escaping_cell_refuses` | At the exit. |
| `equal_identities_decide_eq`, `distinct_live_descriptors_decide_ne`, `an_undecided_guard_gives_the_join`, `the_join_never_releases` | The member-equality guard. |
| `an_undecided_guarded_removal_keeps_and_poisons` | Composed with `an_unmatched_removal_poisons_what_it_may_name`. |
| `removal_and_cell_release_commute` | The two effects touch different records. |

The module is placeholder-free with no project axioms. The POSIX uniqueness
of live descriptor numbers is an explicit hypothesis of the one theorem that
uses it.

## 4. Rows that carry the premise, and predicted movement

From the latest published baseline, `baseline:98b1bb785`
(`private/baselines/98b1bb7851a71ed2d007abc734ed0b94ec1b00f6`).
`baseline:eb49bb6ce` was not yet published when this was written, and phase
2's PREDICTION.md relists against it. A row carries the premise when its
`contract_dependencies.declarations` names one of the four operations. The
system and slice profiles hold each listed function in two units (two
compile configurations), and ×2 below counts both.

| Profile | Row (function) | Status | Rests on |
|---|---|---|---|
| corpus | `mk_core/mk_event.c`: `mk_event_timeout_create`, `mk_event_timeout_destroy`, `mk_event_channel_create`, `mk_event_channel_destroy` | Unknown (each: uncontracted backend `_mk_event_*`) | its own declaration |
| corpus | `mk_server/mk_plugin.c::mk_plugin_api_init` | Unknown (79 leaves) | timeout_create, channel_create (function designators stored) |
| corpus | `mk_server/mk_scheduler.c::mk_sched_launch_worker_loop` | Unknown (27) | channel_create |
| corpus | `mk_server/mk_server.c::mk_server_worker_loop` | Unknown (31) | timeout_create, timeout_destroy |
| corpus | `mk_server/monkey.c::mk_server_create` | Unknown (37) | channel_create |
| system ×2 | `src/flb_engine.c::flb_engine_flush_timer_reset` | Unknown (4) | timeout_create, timeout_destroy |
| system ×2 | `src/flb_engine.c::flb_engine_shutdown` | Unknown (15) | channel_destroy |
| system ×2 | `src/flb_engine.c::flb_engine_start` | Unknown (87) | timeout_create, timeout_destroy, channel_create |
| system ×2 | `src/flb_scheduler.c::flb_sched_create` | Unknown (24; `retention-undecided` at :783) | timeout_create, channel_create |
| system ×2 | `src/flb_scheduler.c::flb_sched_destroy` | Unknown (10) | timeout_destroy, channel_destroy |
| system ×2 | `src/flb_scheduler.c::flb_sched_timer_cb_create` | Unknown (7) | timeout_create |
| system ×2 | `src/flb_scheduler.c::flb_sched_timer_cb_disable` | **Safe** (0) | timeout_destroy |
| system ×2 | `src/flb_scheduler.c::schedule_request_now` | Unknown (2; `retention-undecided` at :133) | timeout_create |
| system ×2 | `src/flb_output.c::flb_output_init_all` | Unknown (86) | channel_create |
| slice ×2 | `src/flb_input.c::collector_start` | Unknown (3; `retention-undecided` at :2351) | timeout_create |
| slice ×2 | `src/flb_input.c::flb_input_collector_destroy` | Unknown (3) | timeout_destroy |
| slice ×2 | `src/flb_input.c::flb_input_collector_pause` | Unknown (4) | timeout_destroy |
| slice ×2 | `src/flb_input.c::flb_input_collector_resume` | Unknown (5) | timeout_create |
| slice ×2 | `src/flb_input.c::input_instance_channel_events_init` | Unknown (5) | channel_create |
| slice ×2 | `src/flb_input_thread.c::input_thread_instance_create` | Unknown (30) | channel_create |
| slice ×2 | `src/flb_input_thread.c::input_thread_instance_destroy` | Unknown (11) | channel_destroy |
| slice ×2 | `src/flb_lib.c::flb_create` | Unknown (11) | channel_create |

Totals: corpus 8 rows; system 18 row instances (9 functions); slice 16 (8
functions).

**Predicted movement under the recommended forms.** No status moves on any
route, and nothing flips Unknown → Safe.

- **Unknown rows.** No listed Unknown row has a descriptor leaf today: each
  is held by other leaves (`retention-undecided`, gate refusals,
  uncontracted callees).
- **Callers that create.** These either reach the exit with a cell of a
  caller's or escaping object (`descriptor-cell-escapes`), or read the
  witness from a member (`config->flush_fd`, not a plain name: undecided),
  so they may gain refusal leaves. The rows involved are `collector_start`,
  `flb_input_collector_resume`, `schedule_request_now`,
  `flb_sched_timer_cb_create`, `flb_sched_create`,
  `flb_engine_flush_timer_reset`, `flb_engine_start`, the channel creators,
  and Monkey's `mk_server_worker_loop`.
- **Channel destroyers.** `flb_engine_shutdown`, `flb_sched_destroy` and
  `input_thread_instance_destroy` pass member reads to a guard that is
  undecided, so they gain a refusal leaf.
- **`flb_sched_timer_cb_disable` (×2) stays `Safe`.** Its actual
  `&timer->event` is an untracked cell. The row trades `mk_event_descriptors`
  for `P-CELL-STATE-NOT-FORWARDED`. Under 2D it would read
  `Safe → Unknown` (×2), the only alternative that moves a status.
- **Report digests and pins.** These change wherever the row premises and
  contract digests change: the provider lock's clause counts and the four
  pins.
- **Other properties.** The null property and the bounds identity are
  unchanged: no bounds clause is touched.

## 5. Observations (outside the decision; candidate follow-ups)

1. **Removal poisoning is missing at channel destroy.** Today's
   `mk_event_channel_destroy` declaration states no removal, so it poisons
   nothing. Yet its equal arm runs `_mk_event_del`, which clears the
   event's status through whatever loop it is given. 4A's guarded removal
   closes this gap in the refusing direction. Under 4B it stays open, and is
   then candidate CHANNEL-DESTROY-REMOVAL-POISONING. The kernel side is
   milder than the model: the `close(r_fd)` that follows ends the epoll
   registration when no duplicate of the descriptor exists.
2. **First-member actuals.** `data` actuals whose pointee is a record with
   `struct mk_event` as its first member (`ins`, `server_timeout`) refuse
   under the recommended reading. Candidate: MK-EVENT-FIRST-MEMBER-CELL.
3. **The pipe arm of `_mk_event_timeout_create`.** It is not compiled in the
   analysed configuration. It ignores `_mk_event_add`'s result and hands
   `fd[1]` to a detached worker, so it needs its own statement if a
   configuration without timerfd is ever audited.
4. **Cross-function reading.** Exit refusal of escaping cells joins
   `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (T1/T2) rather than opening a new
   track.

## 6. Decisions requested

| # | Question | Recommended | Alternatives |
|---|---|---|---|
| D1 | Which place carries the timer's descriptor, and what is the creation form | 1A: the member names it, the result is a witness, `#fresh(descriptor)` with the per-class pair | 1B / 1B′ (the brief's `\|-> #result`), 1C (lower level only) |
| D2 | The destroy's state | 2A `#stale(descriptor)` beside the R3 removal, with `P-CELL-STATE-NOT-FORWARDED` for an untracked cell | 2D (refuse an untracked cell; loses 2 Safe instances); `#absent` / `#released` rejected as false |
| D3 | The channel's slots and member | 3A `#S: #pointee \|-> #fresh(descriptor)` + `#same(r_fd)` | 3B `#view`; 3C `quant out`; 3D lower level only |
| D4 | The channel destroy's condition | 4A the member-equality guard on both releases and the removal | 4B a narrowed premise on this one declaration |
| D5 | The kind spelling on a cell | The value carries it (`#fresh(descriptor)`, `#stale(descriptor)`; heap stays bare) | A separate `resource_member_kind` clause at the shorthand level too |
