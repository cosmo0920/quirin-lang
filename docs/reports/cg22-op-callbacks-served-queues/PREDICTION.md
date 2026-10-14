# CG22-OP-CALLBACKS-AND-SERVED-QUEUES: census, design and prediction

Frozen before any implementation code, at the brief's pause point: the
surface syntax below is a **proposal awaiting the user's choice**. Base: local
`main` at `b565f79ab`. Rule ids reserved: `12m`–`12p`.

## 1. Census on the base

### 1.1 Member sizes (corpus, leaves / functions / decided alone)

BEFORE binary `bin/before` (built from `b565f79ab`, `scripts/build.sh`),
corpus arbiter `66910c10` (`scripts/measure-binary.sh`),
`scripts/size_members.py` ([sizes](artifacts/size-before-corpus.json)):

SIZES-TABLE

Every body was read by hand at `66910c10` and `046accf6`
(`scripts/body.py`): byte-identical at both for every member and every
candidate named below.

### 1.2 What each body does (read by hand)

* **`rd_kafka_q_serve0`** (`rdkafka_queue.c:558-649`). With a forward queue
  it keeps it (`rd_kafka_q_fwd_get`), serves it recursively and drops it
  (`rd_kafka_q_destroy`): rule 13b decides that part. Otherwise it moves up
  to `max_cnt` ops into the frame queue `localq`, and for each op it dequeues
  calls `rd_kafka_op_handle(rk, &localq, rko, cb_type, opaque, callback)`,
  asserts the result is not `PASS` (`rd_kafka_assert` aborts), prepends what
  is left on a yield and finally `rd_kafka_q_destroy_owner(&localq)`.
* **`rd_kafka_op_handle`** (`rdkafka_op.c:1096-1124`). If the op carries
  `rko_serve`, that **replaces the caller's `callback`** (and
  `rko_serve_opaque` replaces `opaque`). Then `rd_kafka_op_handle_std`: a
  callback op (`rko_type & RD_KAFKA_OP_CB`, not `RD_KAFKA_Q_CB_EVENT`) runs
  `rd_kafka_op_call` → `rko->rko_op_cb(rk, rkq, rko)`; a `RECV_BUF` op runs
  `rd_kafka_buf_handle_op` (the buffer's own `rkbuf_cb`). `HANDLED` → the op
  is destroyed here; `KEEP` → the callback kept it; `YIELD` → returned;
  `PASS` → `callback(rk, rkq, rko, cb_type, opaque)` if non-null.
* **`rd_kafka_q_pop_serve0`** (`:398-505`): the same through
  `rd_kafka_op_handle` on the caller's own `rkq` (not a frame queue); a
  `PASS` returns the op to the caller (the owned result), `HANDLED`/`KEEP`
  loop on.
* **`rd_kafka_q_serve`, `_serve_maybe_consume`, `_pop_serve`,
  `_pop_serve_maybe_consume`**: one-line forwarders, the callback a formal
  handed on unchanged. `rd_kafka_q_pop` is `rd_kafka_q_pop_serve(…,
  RD_KAFKA_Q_CB_RETURN, NULL, NULL)`.
* **`rd_kafka_op_destroy`** (`rdkafka_op.c:345-628`). If the op is a
  callback op with `rko_op_cb`, sets `rko_err = RD_KAFKA_RESP_ERR__DESTROY`
  and calls `rko_op_cb(rko->rko_rk, NULL, rko)` (asserting neither `YIELD`
  nor `KEEP`). Then `switch (rko->rko_type & ~RD_KAFKA_OP_FLAGMASK)`
  (`FLAGMASK = (1<<29)|(1<<30)`, `rdkafka_op.h:227-229`) over 59 case labels
  in 33 groups (`ADMIN_FANOUT` falls through into the admin-request group)
  plus `default`, each releasing members of one union arm (`rd_free`,
  `rd_kafka_topic_partition_list_destroy`, `rd_list_destroy` of lists whose
  `free_cb` the op's creator registered, `rd_kafka_buf_handle_op`,
  `rd_kafka_msgq_purge`, reference drops on topics, brokers and queues, a
  recursive `rd_kafka_op_destroy` over `share_fetch_response.message_rkos`),
  `default: break;`; then `rko_rktp`, `rko_error`, `rko_replyq` and
  `rd_free(rko)`.
* **`rd_kafka_q_wait_result`** pops through `rd_kafka_q_pop` (so through
  `rd_kafka_op_handle` with a null callback: `rko_serve` and `rko_op_cb`
  run) and destroys the op. **`rd_kafka_q_purge0`** only destroys the ops
  it moved (`rko_op_cb` with `ERR__DESTROY` and `rkq = NULL`, then the arms).

### 1.3 The callback actuals at every served-queue call site

`scripts/op_callback_census.py` (textual reading aid,
[census](artifacts/op-callback-census.json)); 13 call sites outside the
forwarders:

| callback actual | sites | what the candidate does |
| --- | ---: | --- |
| `rd_kafka_poll_cb` | 7 | calls the **application's** callbacks from `rk->rk_conf` (`consume_cb`, `dr_cb`, `dr_msg_cb`, `error_cb`, `log_cb`, `rebalance_cb`, `stats_cb`, `throttle_cb`); returns `PASS` without taking the op on several arms |
| `rd_kafka_consume_cb` | 1 | calls the application's `consume_cb` through `opaque` |
| `rd_kafka_background_queue_serve` | 1 | calls the application's `background_event_cb`, then `rd_kafka_poll_cb` |
| `NULL` | 4 | none itself; but `rd_kafka_op_handle` substitutes the op's `rko_serve` |

### 1.4 The op-carried callback sets, closed over the two trees

Every store to `rd_kafka_op_s.rko_op_cb`, `.rko_serve`, `rd_kafka_q_s.rkq_serve`
and `rd_kafka_op_s.rko_u.leaders.cb` in librdkafka's sources, and every call
of the registrar `rd_kafka_op_new_cb` (including the `rd_kafka_txn_op_req`
macro's ten expansions):

* **`rko_serve`** is stored only from `orig_destq->rkq_serve`
  (`rdkafka_queue.h:452`, `rd_kafka_q_enq1`) or NULL; `rkq_serve` only from
  `rd_kafka_poll_cb` (`rdkafka.c:2617`, `:2622`: `rk_ops`, `rk_logq`),
  `rd_kafka_cgrp_op_serve`, `rd_kafka_mock_cluster_op_serve`,
  `rd_kafka_toppar_op_serve`, NULL, or a copy of another queue's `rkq_serve`
  (`rdkafka_cgrp.c:503`, `rdkafka_msgset_reader.c:253`). **Closed: 4
  candidates, one of them `rd_kafka_poll_cb`.** So every served queue may run
  `rd_kafka_poll_cb`, and with it the application's `rk_conf` callbacks,
  **whatever callback its caller passes, NULL included.**
* **`rko_op_cb`** is stored from designators (`rd_kafka_share_fetch_reply_op_cb`
  twice, `rd_kafka_offset_reset_op_cb`, `rd_kafka_offset_validate_op_cb`),
  NULL (`rd_kafka_op_call`), or `rd_kafka_op_new_cb`'s formal `cb`, whose
  actuals are 24 designators plus one member read
  (`rko->rko_u.leaders.cb`, `rdkafka_partition.c:4135`), itself stored only
  from `rd_kafka_topic_partition_list_query_leaders_async`'s formal, whose
  two callers pass designators. **Closed: 29 candidates.** 17 open with an
  `rko_err == RD_KAFKA_RESP_ERR__DESTROY` test (the ten transaction ops, the
  two admin workers, the three group-metadata ops, the broker monitor, the
  OAuth refresh); 12 do not, and some do real work when
  `rd_kafka_op_destroy` calls them: `rd_kafka_offset_reset_op_cb` runs a
  whole `rd_kafka_offset_reset` under the toppar lock;
  `rd_kafka_oauthbearer_refresh_op` calls the application's
  `token_refresh_cb` (only when not destroying); `rd_kafka_broker_monitor_op_cb`
  calls a further op-carried member `rko_u.broker_monitor.cb`.
* No candidate has a custody contract in force today (none is declared;
  none has a validated in-unit summary on the measured route).
* Whole-op writes: none found (no `*rko = *other`, no `memcpy` into an op).
* The discriminant `rko_type` is written at construction (`rdkafka_op.c:335`),
  flag-or'ed by the two reply functions (`:811`, `:836`: masked bits only)
  and rewritten once on an existing op (`rdkafka_broker.c:3631`,
  `PARTITION_LEAVE` → `PARTITION_JOIN`, both in the `default` arm).

### 1.5 Where `rd_kafka_op_destroy`'s argument comes from

`scripts/destroy_actuals.py` ([rows](artifacts/destroy-actuals.json); 96
textual call sites in 52 functions, a reading aid): 29 an op popped by
`rd_kafka_q_pop` (undeclared, `REFCOUNT-Q-POP-OWNED-OP`), 16 a formal, 25
unit-test constructors, 4 `rd_kafka_op_req2`, 2 `rd_kafka_op_new_cb`, 1
`rd_kafka_op_new`, the rest locals assigned otherwise. The op constructors
(`rd_kafka_op_new0`, `_new_cb`, `_req*`) are undeclared, so a caller tracks
almost no op it destroys.

## 2. Proposed syntax (for the user's choice)

All three extend C04.19's vocabulary, keep its placement rules (a declaration
carrying any of them is withheld from the function map; no callback is ever
read as benign) and are total-or-reject. Alternatives are listed where the
spelling is a real choice.

### 2.1 Clause 1: an object the callee hands the callback

A new word in a callback actual list, **`handed`**: at each invocation the
callee supplies a live object it owns and hands its custody to the callback.

```qcore
// rd_kafka_q_serve0 (rdkafka_queue.c:558-649)
extern fn rd_kafka_q_serve0(rkq: ptr<mut, u8>, timeout_ms: i32, max_cnt: i32,
                            cb_type: i32, callback: fnptr, opaque: ptr<mut, u8>,
                            is_consume_call: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(timeout_ms);
    ensures normal resource_borrowed(max_cnt);
    ensures normal resource_borrowed(cb_type);
    ensures normal resource_borrowed(callback);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_borrowed(is_consume_call);
    // each op moved to the frame queue goes to the callback, which takes it
    ensures normal resource_callback_invoked(callback, _, _, handed, cb_type, opaque);
    // the op's own callbacks run first (clause 3)
    ensures normal resource_callback_member_invoked(handed, rd_kafka_op_s, rko_op_cb, _, _, handed);
    ensures normal resource_callback_member_invoked(handed, rd_kafka_op_s, rko_serve, _, _, handed, cb_type, _);
    ensures normal resource_return_none();
};
```

* Static: at most one `handed` per actual list; not in a table or a registry
  invocation; a declaration with a parameter named `handed` rejects it.
* Meaning for the caller: the handed object was never the caller's; the call
  is still the identity on the caller's state. A candidate must have a
  contract in force that is the identity at every other position and
  **takes** the handed position: `resource_consumed(p, nonnull)` or
  `resource_released(p, nonnull)`, unconditionally. A candidate that only
  borrows it, or takes it on some return codes only (`rd_kafka_poll_cb`'s
  `PASS` arms), refuses (`callback-handed-kept`): the callee would be left
  holding an op its declaration says it gave away.
* Alternatives: `given`, `supplied_owned`, `_owned`.

### 2.2 Clause 2: a release chosen by a discriminant

A discriminant declared once, then one release clause per arm, guarded with
W43's `when` (C04.14.1) generalised from an integer parameter to the declared
discriminant; `otherwise` names the default arm explicitly.

```qcore
// rd_kafka_op_destroy (rdkafka_op.c:345-628)
extern fn rd_kafka_op_destroy(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    // switch (rko->rko_type & ~RD_KAFKA_OP_FLAGMASK); FLAGMASK = 1610612736, rdkafka_op.h:227-229
    requires resource_discriminant(rko, rko_type, 1610612736);
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s, rko_op_cb, _, _, rko);
    // RD_KAFKA_OP_OFFSET_COMMIT = 6, rdkafka_op.h:93
    ensures normal resource_arm_released(rko, rko_u.offset_commit.partitions) when rko_type == 6;
    ensures normal resource_arm_released(rko, rko_u.offset_commit.reason) when rko_type == 6;
    // RD_KAFKA_OP_LOG = 30, rdkafka_op.h:125; … one clause per released member per arm
    ensures normal resource_arm_released(rko, rko_u.log.str) when rko_type == 30;
    // every other value: the body's `default: break;`
    ensures normal resource_arm_empty(rko) otherwise;
    ensures normal resource_return_none();
};
```

* Static: one discriminant per declaration, on a depth-one pointer parameter;
  `MASK` a canonical decimal (the bits the switch clears; `0` for none); arm
  guards `==` only, on that discriminant, decimal literals with a source
  citation in a comment (never enum names, C04.14.1's rule); member paths
  dotted; at most one `otherwise`; an arm value repeated with disagreeing
  members, or a value both listed and claimed by `otherwise`, rejects.
* Meaning: the caller decides the discriminant only from a tracked op whose
  constructor states it (`resource_discriminant_set(result, rko_type,
  type)` on `rd_kafka_op_new0`, with the actual decided as C04.14.1 decides
  one) **and** whose discriminant the run proves stable (every store to
  `rd_kafka_op_s.rko_type` on an existing object changes only masked bits or
  stays within one arm, checked run-wide like `nonnull_member_stable`). A
  decided value takes its arm; an undecided one takes the **cautious join**:
  a tracked object linked at a member some arms release and others do not is
  refused (`arm-undecided`); a value no arm and no `otherwise` covers is
  **Unknown** (`arm-undeclared`), never neutral.
* Body check (rule `12o`): in `rdkafka_op.c` each arm's members are exactly
  the ones the `case` group passes to a release, and `default:` releases
  nothing; otherwise the declaration is withdrawn in that unit.
* Alternatives: a block form `resource_released_by(rko, rko_type, MASK) {
  6 => rko_u.offset_commit.partitions, rko_u.offset_commit.reason; …; _ => ; }`
  (one clause, a new block grammar); or omit member arms entirely and state
  only `resource_released(rko)` plus the closed member invocation (no
  per-member precision; a payload link stays refused at the release as today).

### 2.3 Clause 3: a registration carried by the object

The invocation names the member; the registrar names where it stores.

```qcore
// rd_kafka_op_new_cb (rdkafka_op.c:785-792): rko->rko_op_cb = cb
extern fn rd_kafka_op_new_cb(rk: ptr<mut, u8>, type: i32, cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(type);
    ensures normal resource_borrowed(cb);
    ensures normal resource_retained_by(rk, result);
    ensures normal resource_callback_registered(cb, result, rd_kafka_op_s, rko_op_cb);
    ensures normal resource_return_owned(nonnull);
};

// rd_kafka_op_call (rdkafka_op.c:934-944): res = rko->rko_op_cb(rk, rkq, rko)
ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s, rko_op_cb, rk, rkq, rko);
```

* `resource_callback_member_invoked(OBJ, TAG, FIELD, A…)`: during the call
  the function pointer stored in `OBJ->FIELD` (`OBJ` a parameter or
  `handed`; `TAG` the struct tag, `FIELD` a dotted member path, as
  `nonnull_member(TAG, FIELD)` names one) is called zero or more times with
  `A…`. `resource_callback_registered(F, H, TAG, FIELD)` adds where the
  registrar stores `F` (rule 13y's store check then requires `H->FIELD = F`).
* **Closure by evidence, run-wide** (the fold `nonnull_member` uses): every
  store the analysed units make to `TAG.FIELD` must be a null constant, a
  function designator, the `F` of a declared registrar naming that member
  (each call of the registrar in any unit then contributes its actual, under
  the same test), or a read of another closed member (`rko_serve =
  orig_destq->rkq_serve`; the fold iterates to the least closed set). Any
  other store (a formal of an undeclared function, a call's result, a cast,
  an untyped whole-object write, an address-taken member) **opens** the
  member for the run: every clause naming it refuses
  (`callback-member-open`, naming the store). The candidate set is then
  decided as rule 13u's (each candidate a contract in force, the identity on
  the receiving positions, a `handed` position taken per clause 1).
* The per-path registration of rule 13w still wins where it exists: an op
  this body created with `rd_kafka_op_new_cb(…, f)` (or `rd_kafka_op_new`,
  no callback) has that one registration in force, not the run-wide set.
* Premise stated, not proved: `P-CG22-MEMBER-CLOSED-WORLD`, every unit that
  can store into the member is analysed by the run (librdkafka's
  `rd_kafka_op_s` is defined in the internal `rdkafka_op.h`).
* Alternatives: `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)` with
  a dotted tag; or a declaration-level `requires
  resource_callback_member(TAG, FIELD);` hosted on the registrar, with the
  invoking clause naming only `FIELD`.

## 3. Mapping onto existing kinds and rules

| clause | builds on | new |
| --- | --- | --- |
| `handed` | rule 13u (candidates, loop-safe composition), C04.2 `consumed`/`released` | a taken position admitted only at `handed` (13u admits a release only at a registry's `element`) |
| member invocation | rule 11p (closed sets), rule 13w (per-path registration; a result holder), `nonnull_member`'s run-wide store fold (14x–14z) | a run-wide closed set for a function-pointer member; the registrar's member argument |
| discriminant arms | W43 guards (C04.14.1, rule 12w: decided actual, cautious union, `when`), C04's containment (field links released with a parent), `nonnull_member_stable`'s store check | a guard on an object's discriminant; per-arm member releases; `otherwise` |

Body standing (rules 13i–13k): the registrar's member store and the arm
check are validated (`12o`); `handed`, the member invocation's actuals and
the closure's premise are declared assumptions or premises.

## 4. Fail-closed refusals

`callback-handed-kept` (a candidate that does not take the handed object on
every return), `callback-member-open` (an unclosed member, naming the store),
`arm-undecided` (a tracked payload link under a join that disagrees),
`arm-undeclared` (a decided discriminant no arm covers), `discriminant-unstable`
(the run-wide store check fails: every value reads undecided), plus rule 13u's
`callback-uncontracted` / `callback-effectful` / `callback-undecided` over
the closed sets. Out of scope and never decided here: callbacks the
application registered in `rk` (`rd_kafka_destroy_app`'s pending decision);
no candidate that calls an `rk->rk_conf` callback is declared, so it stays
`callback-uncontracted`.

## 5. Predicted yield per callee

PREDICTED-YIELD

## 6. The served-queue premise (C04.18)

PREMISE-PARAGRAPH
