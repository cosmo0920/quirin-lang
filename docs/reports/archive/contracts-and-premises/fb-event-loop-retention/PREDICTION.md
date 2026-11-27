# FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: census and prediction

Frozen before any implementation code (round-19 prediction discipline). This
is the brief's census step and its conditional pause point: two of the
operations need surface the user has not decided (a removal of one
registration, and registration forwarding through an id), so the card stops
here with the proposal in the last section and implements nothing until the
user's choice arrives.

Base: local `main` `cc3e79559` (round-30 figures: corpus 938 / 9 / 3,887;
system 93 / 4 / 351, null 84 / 125 / 245; slice 52 / 1 / 347 / 4 Pending, null
71 / 103 / 232; ledger 790; Lean 329). No BEFORE run was taken at this base:
the leaf attribution below reads FB-QUANT-CONTRACT-CENSUS's per-row artifact
(`../fb-quant-contract-census/artifacts/patterns.json`, sha256 `1992d127…`,
binary `7bf6e937…` built from `d708641bc`), whose system and slice figures
equal round 30's. The BEFORE for the measurement is taken on the
implementation base after the pause.

Sources, read only: the system route's `046accf6` export
(`private/w45-fb/w46-external-callee-contracts/attempt-1`, whole `src/` and
`plugins/` tree) and the tail-pause slice's `90978ac6` export
(`private/w47-tail/attempt-1/prep`). Monkey's `mk_event.c` is byte-identical at
`66910c10` (the corpus export), `046accf6` and `90978ac6`; the epoll backend
differs only by `pthread_detach(tid)` in the pipe-timer arm of
`_mk_event_timeout_create` (`046accf6:335`), which changes no retention. No
Fluent Bit or Monkey checkout was touched.

Scripts: `scripts/loop_census.py` (every site, its enclosing function, its
actuals and how the loop and the registered actual resolve;
`artifacts/loop-census.json`) and `scripts/leaf_routes.py` (each P5a leaf of
the census card attributed to its operation; `artifacts/leaf-routes.json`).
Function boundaries are read over the first arm of each conditional group;
sites are counted in every arm, so the totals equal the census card's.

## 1. Census

### 1.1 Collector registrations: 102 matches in 57 files

99 calls and 3 definitions (`flb_input.c:2273/2294/2313`).
`flb_input_set_collector_server` exists only in a comment (`flb_input.c:2204`);
the calls are `_time` 72, `_event` 17, `_socket` 10, in 63 functions.

| Actual | Spelling | Sites |
| --- | --- | --- |
| instance | a formal (`in`, `ins`) | 94 |
| instance | a formal path `ctx->ins` (`etw.c:1577`, `fw.c:327`, `ne.c:147`, `tcp.c:102`, `udp.c:95`) | 5 |
| callback | a function designator defined in the file | 94 |
| callback | a designator defined in another file (`tail.c:415/436/463/477`) | 4 |
| callback | a member value `coll->cb_update` (`ne.c:147`) | 1 |
| config | the formal `config` / `in->config` / `ins->config` | 97 / 1 / 1 |

**How the loop resolves (every site the same way).** A registration names no
loop. `collector_create` (`flb_input.c:2222-2270`, both revisions) stores the
callback into its fresh collector (`coll->cb_collect = cb`, `:2245`), chooses
the loop by the instance's mode (`coll->evl = thi->evl` if
`flb_input_is_threaded(ins)` at `:2251`, else `coll->evl = config->evl` at
`:2258`), links the collector into `ins->collectors` and returns it; the setter
returns `coll->id`. The event `&coll->event` enters the loop only later, at
`collector_start` (`:2351` `mk_event_timeout_create(coll->evl, …, event)` /
`:2362` `mk_event_add(coll->evl, …, event)`), reached by id through
`flb_input_collector_start(coll_id, in)` (`:2378`, a list search). So the holder
path of every collector registration is `coll.evl` of a collector no parameter
of the setter names: the registration is **forwarded through an id**.

### 1.2 `mk_event_add` / `mk_event_timeout_create`: 37 sites in 21 files

| Loop (holder) actual | Resolves as | Sites |
| --- | --- | --- |
| formal path (`config->evl`, `connection->evl`, `coll->evl`, `sched->evl`, `sp->config->evl`, `session->event_loop`, `u_conn->evl`, `conn->evl`, `ctx->evl`) | a holder path `FORMAL.FIELD…`, resolved at the call | 19 |
| a formal that is the loop (`event_loop`, `evl`) | the formal itself | 5 |
| a local whose one write is a formal path (`flb_tls.c:207`, `flb_network.c:1014`) | the formal path it copies | 2 |
| `flb_engine_evl_get()` (the thread's loop from TLS; `http_conn.c:274`, `mqtt_conn.c:123`, `syslog_conn.c:199`, `tcp_conn.c:631`, `unix_socket_conn.c:392`) | unresolved: a call's result | 5 |
| a member of a fresh local (`log->evl`, `flb_log.c:1108`; `evl` from `mk_event_loop_create`, `flb_engine.c:1498`) | a path below a tracked identity of this body | 2 |
| a member of a collector looked up by id (`flb_input_collector_resume`, `:3248`/`:3261`) | unresolved: no path to a formal | 2 |
| a local with several writes (`flb_downstream.c:136`, `flb_sp.c:2194`) | unresolved | 2 |

Registered (retained) actual: an interior of a formal (`&connection->event`,
`&ctx->listener_event`, …) 18; a local `event = &X->f` copied from a formal
path 6 (+1 interior); a local with several writes (`event` set to two members)
7; an interior of a fresh local 2; of an id-looked-up collector 1; other 3.

### 1.3 Timer-callback registrations: 22 matches in 15 files

20 calls and 2 definitions (`flb_scheduler.c:638/699`). Scheduler actual:
`sched = flb_sched_ctx_get()` (a TLS lookup) 14, a formal path
(`config->sched`, `ins->config->sched`, `ctx->sched`) 4, a formal 1
(`flb_sched_timer_coro_cb_create`), other 1. Callback: a designator 19, a
formal 1. **How the loop resolves:** inside `flb_sched_timer_cb_create`
(`:638-693`) the timer is created by `flb_sched_timer_create` (published into
`sched->timers`, `:889`), `data` and `cb` are stored into it (`timer->data`,
`timer->cb`), and `&timer->event` is registered at `mk_event_timeout_create(
sched->evl, …, event)` (`:672`): the holder path `sched.evl` (stored once, at
`flb_sched_create`, `sched->evl = evl`, `:753`). The call returns 0 / -1 and
optionally the timer through `out_timer`. So a timer registration retains the
caller's `data` in a timer that `sched` reaches, and registers `cb` in
`flb_sched_timer.cb`.

### 1.4 The lifetime side: removals, loop stores and loop destroys

* **Removals, 37 sites** (`mk_event_del` 22, `mk_event_timeout_destroy` 9,
  `mk_event_channel_destroy` 6): holder a formal path 26, a local copy of one
  7, id-looked-up 2, other 2. `mk_event_del` and `mk_event_timeout_destroy`
  remove the registration unconditionally (`mk_event.c:132-153`,
  `mk_event_epoll.c:156-186`, `:341-353`: an unregistered event is left as it
  is; a NULL `data` is a no-op). `mk_event_channel_destroy` removes it **only
  when `event->fd == r_fd`** (`mk_event_epoll.c:397`), a relation between a
  member and a scalar parameter no clause can state.
* **Stores into loop members, 25 sites** (`evl`, `ch_evl`, `event_loop`):
  formal copies (`config->evl = evl`, `sched->evl = evl`,
  `connection->evl = event_loop`), the instance-mode choice of
  `collector_create`, fresh loops (`thi->evl = mk_event_loop_create(256)`,
  `ctx->event_loop = …`, `worker->event_loop = …`), and resets to NULL
  (`rb->event_loop = NULL`, `worker->event_loop = NULL`).
* **Loop destroys, 11 sites** (`mk_event_loop_destroy`): `config->evl` at
  `flb_config_exit` (`flb_config.c:742`), `thi->evl` at
  `input_thread_instance_destroy` (`flb_input_thread.c:206`), `ctx->event_loop`
  at `flb_destroy` (`flb_lib.c:242`), the log and output-thread loops.

**The ordering the brief asks about, read by hand (slice, `90978ac6`).** A
threaded input's collectors are registered in `thi->evl`
(`collector_create:2251`, `collector_start`); at shutdown
`flb_input_exit_all` (`:2145`) calls `flb_input_instance_exit` (which, through
`flb_input_thread_instance_exit`, joins the input thread, whose exit path
`input_thread:518` runs `input_thread_instance_destroy` and so
`mk_event_loop_destroy(thi->evl)`) **before** `flb_input_instance_destroy`,
whose collector loop (`:1328-1332`) calls `flb_input_collector_destroy`. That
function removes the event from `coll->instance->config->evl`
(`:3146`/`:3150`), not from `coll->evl`: for a threaded instance the removal
names another loop, and the registration was already dropped by the thread
loop's destroy. So the order is safe, but only through four functions and a
thread join; and a path-tracked removal does not match the registration it is
meant to end (`config.evl` against `coll.evl`). Recorded as a reading, not a
finding.

### 1.5 The P5a leaves on the routes, by what would decide them

From `artifacts/leaf-routes.json` (sites of the census card's P5a rows, one
per distinct function):

| Class | What decides it | System sites | Slice sites |
| --- | --- | --- | --- |
| A | a registration (`mk_event_add`, `_timeout_create`, `_channel_create`): `resource_retained_by(data, loop)`, holder actual a path | 11 | 9 |
| B | a removal (`mk_event_del`, `_timeout_destroy`, `_channel_destroy`): **needs a removal word** | 7 | 6 |
| C | `mk_event_wait` (plain: borrows the loop) and the `mk_event_foreach` macro | 1 | 3 |
| D | `flb_input_set_collector_*`: **registration forwarded through an id** | 1 | 11 |
| E | `flb_sched_timer_cb_create`: retains `data` in a timer `sched` reaches | 4 | 2 |
| F | `flb_input_collector_start` / `_pause` by id | 0 | 3 |
| G | `flb_input_collector_fd` (the dispatcher; P1, rule 10p) | 1 | 0 |
| H | `flb_input_pause` / `_resume` / `_pause_all` (dispatch a plugin callback; rule 10j, P1) | 5 | 5 |
| I | an indirect access at a member actual | 1 | 0 |

## 2. Holder-path design (decision 1)

**Spelling.** `resource_retained_by(P, H)` with `H` a parameter, `global`,
`result` or, new, `FORMAL.FIELD[.FIELD]`: a parameter of the declaration (a
depth-one pointer, `resource_borrowed`, no kind) followed by one or more
member names. **It cannot be confused with `TAG.FIELD`**: the dotted record
form stands only in member operands (`resource_callback_member_invoked` /
`_registered`'s second operand, `resource_discriminant_set`, the left of
`|->` in `quant live`), and a holder path stands only in a retention's holder
operand. The loader reads by position and requires the first segment to name a
parameter of the same declaration, so a record tag there rejects the module
("unknown parameter"); a parameter named like a record tag is still a
parameter. No disambiguation is needed.

**Where it is needed.** None of the eight Monkey operations needs it: their
holder is the loop formal itself (`resource_retained_by(data, loop)`). A
holder path is needed by a Fluent Bit declaration whose loop is a member of
its formal: `collector_start(coll, config)` would be
`resource_retained_by(coll, coll.evl)` (the collector's interior event in the
loop its `evl` member holds), and `flb_sched_timer_cb_create` would retain
`data` in `sched` (a timer `sched->timers` links). At a **call** of
`mk_event_add` the holder actual is the path as spelled (`coll->evl`): the
route resolves it to the loop the member holds (a formal path, a local's one
copy of one, or a member of a tracked identity of the body); every other
spelling (a call's result, an id lookup, a local with several writes) is
`retention-undecided`.

**What decides "outlives".** Rule 12t decides a retention from what the body
sees and never forwards it into a summary. Read with paths, a record is
(held object, holder path) and ends at a removal of that object from that same
path (§3, the new word), at the destroy of the loop at that path (the path's
value unchanged since the registration: a store into the member in between
opens it), or at the frame's end for a frame holder. A record whose holder is
a caller's loop (a formal path) still holding at an exit refuses, as a tracked
holder does today. **Consequence, measured by the census:** every registration
site in §1.2 and §1.3 keeps its registration live past its function's exit
(registering is the function's purpose: `collector_start`,
`flb_sched_timer_cb_create`, `flb_engine_start`, `net_io_*_async`), and every
removal is in another function (§1.4). So intra-body tracking decides **no
registration** in the tree; it only names the reason precisely
(`retention-undecided` on a resolved path instead of an uncontracted call).
Deciding the FB shape needs the lifetime of the loop across functions (§1.4's
ordering): either retentions forwarded into summaries along call edges (a
change to rule 12t's "do not forward"), or a run-wide closure per loop member
like rule 12n's (stores into `evl`, registrations, removals, releases of the
held record type, loop destroys). Neither is in this card's decided scope;
§4 proposes how to proceed.

## 3. Proposal: the two undecided surfaces

### 3.1 Removing one registration (`mk_event_del`, `mk_event_timeout_destroy`)

v1 has `resource_holder_reset(H)` ("H keeps nothing it kept"), which is
**false** for a loop: it holds every other event. Declaring a removal with it
would drop the records of other events and admit a release of one of them
while the loop still holds it. Declaring the removal as a plain borrow is
cautious (records are never dropped, so later releases refuse) but states an
effect v1 cannot state exactly, which rule 10j forbids. Alternatives:

* **R1 (recommended): `ensures normal resource_unretained_by(P, H);`** — after
  the call `H` keeps no pointer into `P`'s object; its other retentions are
  unchanged. Same placement as `resource_retained_by` (a holder parameter or a
  holder path), the pair mirroring `resource_published` /
  `resource_unpublished`.

  ```qcore
  extern fn mk_event_del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32
  contract {
      requires resource_contract_v1;
      ensures normal resource_borrowed(loop);
      ensures normal resource_borrowed(event);
      ensures normal resource_unretained_by(event, loop);
      ensures normal resource_return_none();
  };
  ```

* **R2: `resource_holder_reset(H, P)`**, the existing word with an optional
  second operand ("H keeps nothing of P"). No new word, but one word with two
  meanings by arity.
* **R3: a `quant` state on the holder,** `quant live #loop: #holds(event)
  |-> #absent;` — in the user's sigil family, but it invents `#holds(…)`, a
  parameterised value keyword, which the decided forms do not have.

**Composition (any of the three).** At a call, the removal drops exactly the
record whose held object is the actual's identity (the same tracked site, or
the same caller formal) and whose holder is the actual's resolved holder (the
same parameter, or the same path with no store into its member since the
registration). Anything it cannot match drops nothing: the record stays and a
later release of the object refuses (`retention-released-while-held`), so an
unmatched removal is never `Safe`, never `Unsafe`. A removal with no record in
the body is the identity on custody. `mk_event_channel_destroy` removes only
when `event->fd == r_fd` (§1.4), a condition no clause states, so it would be
declared **without** the removal (its records kept, the cautious reading) and
its two closes left to the fd reading; `mk_event_timeout_destroy(loop, NULL)`
is a no-op, which the removal covers.

Worked example 1 (system, `flb_scheduler.c:720-728` at `046accf6`, and the
same text at `66910c10`):

```c
int flb_sched_timer_cb_disable(struct flb_sched_timer *timer)
{
    if (timer->timer_fd != -1) {
        mk_event_timeout_destroy(timer->sched->evl, &timer->event);  // removal at timer.sched.evl
        timer->timer_fd = -1;
    }
    return 0;
}
```

Its one leaf is the uncontracted `mk_event_timeout_destroy` (class B). With R1
the call removes `&timer->event` (an interior of the formal `timer`) from the
path `timer.sched.evl`; the body holds no record, so the call is the identity
and the function's custody is decided (it borrows `timer`). Without a removal
word it stays undeclared and the row stays `Unknown`.

Worked example 2 (slice, `flb_input.c:3140-3156` at `90978ac6`):
`flb_input_collector_destroy` removes `&coll->event` from
`coll->instance->config->evl` and frees `coll`. In this body no record exists,
so with R1 both removals are the identity and the free decides as today (the
row's third leaf is outside P5a). Under a run-wide reading (§4, T1) this
removal would **not** match `collector_start`'s registration at `coll.evl` for
a threaded instance; the hand reading in §1.4 shows the thread loop's destroy
ends it first.

### 3.2 Registration forwarded through an id (`flb_input_set_collector_*`)

C04.19.3's setter form `resource_callback_member_registered(P, TAG.FIELD, F)`
states that **P's object** holds `F` in its member. A collector setter stores
`cb_collect` into a collector it creates and links into `ins->collectors`, and
returns that collector's id: no parameter's object holds the member, and the
result is an integer, so neither `P` nor `result` names the holder. (The run's
closure of `flb_input_collector.cb_collect` reads every call as a store of the
`F` actual and never reads `P`, and rule 12n already follows
`collector_create`'s formal `cb` to its callers' actuals; the clause is needed
so a caller such as `in_tail_init` sees a declared callee instead of an
uncontracted one, and is not read as a benign borrow.) Alternatives:

* **F1: no new word, holder meaning widened.** Declare
  `resource_callback_member_registered(ins, flb_input_collector.cb_collect,
  cb_collect)` and amend C04.19.3: a setter's holder is the parameter
  **through which** the record holding `TAG.FIELD` is reached (its own object,
  or a record the call creates and links into it). Consumers are unchanged.
* **F2 (recommended): the decided holder path, at the link.**
  `resource_callback_member_registered(ins.collectors,
  flb_input_collector.cb_collect, cb_collect)`: a record of type
  `flb_input_collector` linked at `ins`'s member `collectors` holds `F` in
  `cb_collect`. It reuses decision 1's `FORMAL.FIELD` spelling in a second
  position (a holder), and states where the record is, which a later
  start/pause by id can reuse. Not confusable with the `TAG.FIELD` operand
  beside it for the reason in §2 (position, root a parameter).
* **F3: a value keyword for the object an id names,**
  `resource_callback_member_registered(#id, flb_input_collector.cb_collect,
  cb_collect)` beside an integer result: the record the returned id names. A
  new keyword; it would also serve the by-id operations (class F) later.

Worked example 3 (slice, `flb_input.c:2273-2292` with `:2222-2270`), under F2:

```qcore
extern fn flb_input_set_collector_time(ins: ptr<mut, u8>, cb_collect: fnptr,
                                       seconds: i64, nanoseconds: i64,
                                       config: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ins);
    ensures normal resource_borrowed(cb_collect);
    ensures normal resource_borrowed(config);
    ensures normal resource_callback_member_registered(ins.collectors, flb_input_collector.cb_collect, cb_collect);
    ensures normal resource_return_none();
};
```

**Forwarding check (any option).** In the unit that defines the setter, the
clause stands only when the body names `cb_collect` only as the whole actual
at the callback position of a direct call to an in-unit function whose body
stores that formal, unchanged, into `X->cb_collect` of a fresh object (`coll =
flb_calloc(…)`) that it links into `ins->collectors` (`mk_list_add(&coll->_head,
&ins->collectors)`) and returns, the setter returning only `coll->id` or `-1`;
otherwise the declaration is withdrawn there (rule 15w's discipline for
invocations, applied to a registration). Elsewhere it is a declared assumption
(rule 13i). Lean: a forwarded registration contributes exactly the store the
direct one does to the member's closure, and a deviating body never stands.
At a call the clause is the identity on the caller's custody (the setter
borrows `ins` and `config`), so `in_tail_init`'s 7 class-D leaves would decide
and it keeps its 30 other leaves.

## 4. Scope and prediction

**Proposed implementation after the decision.** The holder path (loader,
rule 12t's path holder with the composition of §3.1 and the store and
loop-destroy endings), the removal word, the forwarding word and check, and the
declarations: in `qcore-provider-libevent` (Monkey owner) `mk_event_add`,
`mk_event_timeout_create`, `mk_event_channel_create` (retentions),
`mk_event_del`, `mk_event_timeout_destroy` (removals),
`mk_event_channel_destroy` (no removal: §3.1), `mk_event_wait` (a borrow), each
read at `66910c10` and `046accf6` (identical texts); in
`quirin-provider-fluent-bit` the three collector setters and
`flb_sched_timer_cb_create`, in their own module. Tests: a path holder, an
unresolved path, a loop destroyed before deregistration (the later removal
borrows a released loop: refused), forwarding. Lean: a resolved path's record
ends only by its own removal, its loop's destroy or the frame; an unresolved
path never decides; a store into the path's member opens it. **Not proposed
here** (a rule-12t semantic change, for the user): forwarding retentions into
summaries (T2) or a run-wide loop-member closure (T1), which are what would
decide the FB registrations (§2, §1.4).

**Predicted movement (with R1 and any F option), custody.**

* **System** 93 / 4 / 351: **+1 Safe** possible, `flb_sched_timer_cb_disable`
  (its only leaf is class B), provided the declaration's member actuals pass
  the access gate as at other contracted calls; its callers
  `flb_sched_timer_destroy`, `flb_sched_timer_invalidate`,
  `flb_sched_event_handler` keep their other leaves (not predicted to move).
  Every class-A and class-E row trades `call to uncontracted function` for
  `retention-undecided` (a caller's loop path holding at the exit, a TLS
  scheduler, an id lookup) and stays `Unknown`. No Safe → non-Safe.
* **Slice** 52 / 1 / 347 / 4 Pending: **no status moves predicted.**
  `in_tail_init`, `flb_tail_fs_inotify_init` and `flb_tail_fs_stat_init`
  lose their class-D leaves (7, 2, 2) and keep others; class-B rows
  (`flb_input_collector_destroy`, `flb_input_collector_pause`, `flb_destroy`,
  `input_thread_instance_destroy`) keep non-P5a leaves; the four slice rows the
  census found alone in P5a are classes E, F, H and stay `Unknown`.
* **Corpus** 938 / 9 / 3,887: the only Fluent Bit unit's two P5a rows are
  class H: **no move.** Monkey units in the corpus that call these operations
  may trade leaves; any Safe gain is read by hand.
* **Null property:** no move predicted on any route (the null leaves at these
  sites are member dereferences such as `timer->sched`, outside this card).

**Obligations.** C1 (collector access follows the loop owner) and C2 are rule
10p's and P1's; OB-10, OR-9, C4, C5 rest on cross-function ordering (shutdown,
event-loop rounds, join before release) that §2 shows intra-body tracking
does not reach. **None is predicted to discharge**; each would be re-read
after measurement.

**Ledger.** `W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS` would be narrowed (the
eight operations declared, its DISCHARGE's census check met for the Monkey
operations); `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION` narrowed to the
cross-function lifetime (T1/T2) and classes E/F, with a follow-up for each.
Neither retires on this prediction.
