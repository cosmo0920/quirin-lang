# OP-CALLBACK-MEMBER-PATH-RELEASE: census, syntax proposal and prediction

Card OP-CALLBACK-MEMBER-PATH-RELEASE (wave 48, card-high). Frozen before any
implementation code, at the brief's pause point: the syntax below is a
**proposal** for the user; nothing is implemented until the user chooses.
Base: local `main` `39287f12e`. Authority: the ledger entry
`OP-CALLBACK-CANDIDATE-CONTRACTS` and its DISCHARGE; C04.19.2 (the arms),
C04.19.5 (the hand-off); rules 12m, 12n, 12o, 13l, 13u, 14e, 12t / 13e, 12x.

## 1. Reading

Every body below was read in full at `66910c10`
(`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`) and `046accf6`
(`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source`);
`diff -rq` of the two `lib/librdkafka-2.15.0/src` trees is empty, so every
text is byte-identical at both revisions ([`body.py`](scripts/body.py) prints
them; its one "DIFFERS" for `rd_kafka_admin_common_worker_destroy` is the
script matching a comment at `:445` at one revision, not a difference).

**The protocol the clause must compose with.**

* `rd_kafka_op_destroy` (`rdkafka_op.c:345`) calls `rko->rko_op_cb(rko_rk,
  NULL, rko)` with `rko_err = RD_KAFKA_RESP_ERR__DESTROY` **before** its
  `switch`, asserting the result is neither `YIELD` nor `KEEP`; then the
  type's arm releases its member paths; then the common tail (`rko_rktp`,
  `rko_error`, `rko_replyq`, outside C04.19.2's arms).
* `rd_kafka_op_call` (`:934`) clears `rko_op_cb` after any result but `KEEP`
  (and `YIELD`), so the serve path's later `rd_kafka_op_destroy` (in
  `rd_kafka_op_handle`, `:1096`) does not call the candidate a second time.
* At the serve and request invocations the op is callee-supplied
  (`supplied_owned` or `_`): a member-path effect there is on an object that
  was never the caller's. **The only invocation where a member-path effect
  reaches the caller's state is `rd_kafka_op_destroy`'s
  `rko_op_cb(_, _, rko)`, where `rko` is the caller's own object, released
  right after by the arms.**
* The arms the eight candidates meet. Every arm path a candidate empties is
  null- or count-guarded (`RD_IF_FREE`, `rd_kafka_replyq_destroy` tests
  `q`, `rd_list_destroy` of an embedded list whose count `rd_list_move` set to
  0), and every member a candidate takes **without** an arm releasing it is
  either asserted null by the arm (`rko_u.leaders.eonce`,
  `rko_u.leaders.replyq.q`, `rko_u.admin_request.fanout_parent`,
  `rko_u.admin_result.fanout_parent`) or never read by it
  (`rko_u.admin_request.eonce`, `rko_u.leaders.opaque`, `rko_replyq.q`).

## 2. Census

"Taken": released, or handed to another holder (not the caller). "Cleared":
the member reads `NULL` (an embedded list: count 0). "Stale": taken but the
member still designates the object. "Kept": untouched (still the op's). Every
candidate is invoked at `rd_kafka_op_destroy` under `__DESTROY` and returns
`HANDLED` there.

### 2.1 `rd_kafka_admin_worker` (`rdkafka_admin.c:1035`; registered `:632` for the admin request types)

Results: `KEEP` (six waits: coordinator, broker, controller, broker list,
request sent; the trailing return) and `HANDLED` (the `destroy:` label only,
reached from `__DESTROY`, a terminating handle, an error, a timeout, a send
failure, `WAIT_FANOUTS` and `WAIT_RESPONSE`). On `KEEP` the op is the
callback's (the eonce holds it): the C04.19.5 hand-off `== KEEP`. On
`HANDLED`, through `rd_kafka_admin_result_fail` → `rd_kafka_admin_result_new`
(`:341`) and `rd_kafka_admin_result_enq` (`:433`), the 23 parse callbacks of
the request tables (each builds its result with `rd_kafka_admin_result_new`),
`rd_kafka_admin_fanout_op_distribute` (`:1337`) and
`rd_kafka_admin_common_worker_destroy` (`:708`):

| member path (under `rd_kafka_op_s`) | on HANDLED | goes to | after | arm (types 32–58) |
| --- | --- | --- | --- | --- |
| `rko_u.admin_request.eonce` | released (the op's reference on the counted eonce, `rd_kafka_enq_once_destroy`), every path | — | cleared | not read |
| `rko_u.admin_request.args` | moved (`rd_list_move`) where a result is built; kept at `WAIT_FANOUTS` (copied to the children) and where `replyq.q` is null | the result op, enqueued on the reply queue | cleared (count 0) or kept | `rd_list_destroy(&args)`: what remains |
| `rko_u.admin_request.fanout_parent` | moved where a result is built or the fan-out distributed; kept where `replyq.q` is null | the result op's `admin_result.fanout_parent`, or the children (counted by `fanout.outstanding`) | cleared or kept | asserted null |
| `rko_u.admin_request.replyq.q` | released (`rd_kafka_replyq_enq` clears it and drops the reference) where a result is enqueued; kept otherwise | — | cleared or kept | `rd_kafka_replyq_destroy`: q if set |

Not owned and not touched: `reply_buf` ("temporary reference not owned by
this rko", `rdkafka_op.h:497`), `options`, `cbs`, `coordkey` (read).
`rko_u.admin_result.result_cb` (a union member read through the request) is
set only to `rd_kafka_ListOffsets_handle_result` (`:4581`).

### 2.2 `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` (`rdkafka_partition.c:4156` → worker `:4030`; registered `:4192`, type `LEADERS` 59)

`KEEP` while waiting for metadata (the eonce re-enabled with the op): the
hand-off `== KEEP`. `HANDLED` (the `reply:` label; `__DESTROY` goes there
directly):

| member path | on HANDLED | goes to | after | arm (59) |
| --- | --- | --- | --- | --- |
| `rko_u.leaders.eonce` | released (`rd_kafka_enq_once_disable`) if set, every path | — | cleared | asserted null |
| `rko_u.leaders.replyq.q` | released (`rd_kafka_replyq_enq`) if set, every path | — | cleared | asserted null |
| `rko_u.leaders.partitions` | moved where `replyq.q` is set, else kept | the reply op's `rko_u.leaders.partitions` | cleared or kept | `RD_IF_FREE` |
| `rko_u.leaders.opaque` | moved where `replyq.q` is set, else kept | the reply op's `rko_u.leaders.opaque` (the DeleteRecords / ListOffsets fan-out op) | **stale** or kept | not read |

`rko_u.leaders.leaders` is not touched (the reply gets a fresh list).

### 2.3 `rd_kafka_admin_fanout_worker` (`rdkafka_admin.c:1393`; registered `:350`, type `ADMIN_RESULT` 51)

`HANDLED` only. `rko_u.admin_result.fanout_parent`: cleared on every path,
the fan-out op's `outstanding` decremented, and the op destroyed by this
call when it reaches 0: the op's **share of a counted object is dropped**
(released). The arm asserts it null. `partial_response` receives the op as
`const rd_kafka_op_t *` (typedef `:263`); the five implementations read and
copy. Nothing else of the op is touched (the arm releases `args`,
`results`, `errstr` as today).

### 2.4 `rd_kafka_DeleteRecords_leaders_queried_cb` (`:4398`) and `rd_kafka_ListOffsets_leaders_queried_cb` (`:4294`); registered `:4559`, `:4649` as the reply's `rko_u.leaders.cb`

`HANDLED` only. `rko_u.leaders.opaque` (the fan-out op): on an error
(`__DESTROY` included) released (`rd_kafka_admin_common_worker_destroy(…,
rd_true)` → `rd_kafka_op_destroy`), otherwise handed to the children
(`fanout_parent`, counted by `outstanding`); **never cleared: stale** on
every path. No arm reads it. `rko_u.leaders.partitions` and
`rko_u.leaders.leaders` are read only (kept; the arm releases them).

### 2.5 `rd_kafka_share_commit_async_fanout_op` (`rdkafka.c:4032`; registered `:4377`, type 71)

`HANDLED` only. `rko_u.share_commit_async_fanout.ack_batches`: when set and
non-empty, handed to `rd_kafka_share_segregate_and_dispatch_acks` (`:3689`:
the elements to the brokers, the container `rd_list_destroy`ed) and cleared:
released; otherwise kept. Arm `RD_IF_FREE`.

### 2.6 `rd_kafka_share_fetch_fanout_op` (`rdkafka.c:3745`; registered `:3408`, type 70)

`HANDLED` only. `rko_u.share_fetch_fanout.ack_batches`: kept when no broker
is selected and there are no acks; otherwise cleared, and released only when
non-empty: **a non-null empty list with a selected broker is cleared without
its release** (`has_fanout_acks ? ack_batches : NULL`). Its producer
`rd_kafka_share_build_ack_details` (`rdkafka_share_acknowledgement.c:686`)
creates the list only at its first add (`:822-826`), so it is null or
non-empty; a declaration stating "released" rests on that, which it must
name as a premise (`share_ack_batches_nonempty`). Arm `RD_IF_FREE`.

### 2.7 `rd_kafka_share_commit_sync_fanout_op` (`rdkafka.c:4251`; registered `:4466`, type 72)

`HANDLED` only; every path:

| member path | effect | goes to | after | arm (72) |
| --- | --- | --- | --- | --- |
| `rko_u.share_commit_sync_fanout.ack_batches` | released (`RD_IF_FREE` here, or `rd_kafka_share_segregate_sync_acks_by_leader`, `rdkafka_share_acknowledgement.c:527`, pops and destroys) | — | cleared | `RD_IF_FREE` |
| `rko_u.share_commit_sync_fanout.results` | moved | `rkcg->rkcg_commit_sync_request.results` (reached from `rk`) | cleared | `RD_IF_FREE` |
| `rko_replyq.q` | moved | `rkcg->rkcg_commit_sync_request.replyq` | cleared | (common tail, null-tolerant) |

**Summary.** No candidate releases a path an arm releases while leaving it
designated: there is no double release in librdkafka's own composition.
Taken and cleared on every path: 7 paths; taken and cleared or kept: 6;
stale: 3 (`leaders.opaque` in three candidates, one of them "stale or kept").
A cleared path is idempotent under repeated invocation (a second call finds
it cleared); a stale one is not.

## 3. Syntax proposal (for the user)

All three build on the decisions in force: `#` binders and value keywords,
the bare clause kind after `quant`, `|->` with a dotted `TAG.FIELD` on its
left, `when #result OP C` with `@` names or aliases. In each, a member-state
clause's guard is either absent (every return) or the complement of the
declaration's hand-off guard (`!= #KEEP` beside `== #KEEP`), so every return
value has exactly one reading and rule 12m's caller branch check already
splits them. The binder is the borrowed (or, beside a hand-off,
`resource_owned`) op formal. A path the declaration does not name is kept.

**Composition with the arms (all three).** At a member invocation the
consumer composes, per candidate, the candidate's state of a path with the
arm the release then applies there, and joins over the candidates (and the
decided arms) exactly as C04.19.2 joins arms; only a tracked object the
caller linked at that path is affected:

| candidate leaves the path | an arm releases the path | no arm releases it |
| --- | --- | --- |
| kept (unnamed) | released by the arm (as today) | stays linked (P-B08-SHALLOW, as today) |
| taken, cleared | the arm reads it cleared and releases nothing; the object is gone (released or moved) | gone |
| taken, stale | **refused**: the arm would release it a second time | **refused**: an invocation is zero or more times (rule 13u), and a second one would take it again |
| taken-cleared or kept | gone on both readings | **refused** (undecided) |
| taken-stale or kept | refused | refused |
| unstated (no declaration in force, or withdrawn) | refused | refused |

Two candidates whose outcomes differ at a linked path refuse, as two arms do
(`arm-undecided`); a decided registration (C04.19.4) narrows the candidates
as it narrows the arms.

### Alternative A: the object's fate, on every path (`#released`, `#moved`)

The right-hand side says what became of the object; both values mean the
member is cleared afterwards, on every return the guard admits.

```qcore
quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.ack_batches |-> #released;
quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.results |-> #moved;
quant live #rko: rd_kafka_op_s.rko_replyq.q |-> #moved;
```

Least surface (two value keywords). It cannot state a path taken on some
paths only or left stale, so `rd_kafka_admin_worker`,
`…query_leaders_async_worker_op_cb`, both `…_leaders_queried_cb` and the two
share fan-outs stay refused: **2 of 8** declared
(`rd_kafka_admin_fanout_worker`, `rd_kafka_share_commit_sync_fanout_op`).

### Alternative B (recommended): the member's state after the call, with a may-set

The right-hand side is the member's state after the call, one of
`#released`, `#moved` (taken, cleared), `#stale` (taken, still designated)
and `#kept` (untouched, the default), or a two-element set `{V, #kept}`: one
of the two on each path.

```qcore
alias #KEEP = @RD_KAFKA_OP_RES_KEEP;

extern fn rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == #KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.eonce |-> #released when #result != #KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.replyq.q |-> #released when #result != #KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when #result != #KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.opaque |-> {#stale, #kept} when #result != #KEEP;
};

extern fn rd_kafka_share_commit_sync_fanout_op(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
    quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.ack_batches |-> #released;
    quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.results |-> #moved;
    quant live #rko: rd_kafka_op_s.rko_replyq.q |-> #moved;
};
```

At `rd_kafka_op_destroy` (`__DESTROY`, so `!= #KEEP`): a caller that linked
its tracked list at `rko_u.leaders.partitions` sees it gone on both readings
(moved, or kept and then released by the `LEADERS` arm); one that linked an
object at `rko_u.leaders.opaque` is refused (no arm releases it, and it may
have been kept); one that linked its eonce at `rko_u.leaders.eonce` sees it
released, and the arm asserts the member null. Covers **8 of 8**:

| candidate | clauses under B (paths under `rd_kafka_op_s.rko_u.`) |
| --- | --- |
| `rd_kafka_admin_worker` | hand-off `== #KEEP`; `!= #KEEP`: `admin_request.eonce \|-> #released`, `admin_request.args \|-> {#moved, #kept}`, `admin_request.fanout_parent \|-> {#moved, #kept}`, `admin_request.replyq.q \|-> {#released, #kept}` |
| `…query_leaders_async_worker_op_cb` | as above |
| `rd_kafka_admin_fanout_worker` | `admin_result.fanout_parent \|-> #released` (its share of the counted fan-out op) |
| `rd_kafka_DeleteRecords_leaders_queried_cb`, `rd_kafka_ListOffsets_leaders_queried_cb` | `leaders.opaque \|-> #stale` |
| `rd_kafka_share_commit_async_fanout_op` | `share_commit_async_fanout.ack_batches \|-> {#released, #kept}` |
| `rd_kafka_share_fetch_fanout_op` | `share_fetch_fanout.ack_batches \|-> {#released, #kept}`, premise `share_ack_batches_nonempty` |
| `rd_kafka_share_commit_sync_fanout_op` | as above (`rd_kafka_op_s.rko_replyq.q`, outside `rko_u`) | Four new
value keywords (`#released`, `#moved`, `#stale`, `#kept`; reserved like
`#absent`) and the set braces.

### Alternative C: arm-style canonical clauses (no maps-to)

The C04.19.2 spelling, paths relative to the formal like an arm's, the same
meanings as B:

```qcore
ensures normal resource_member_released(rko, rko_u.leaders.eonce) when #result != #KEEP;
ensures normal resource_member_moved(rko, rko_u.leaders.partitions, maybe) when #result != #KEEP;
ensures normal resource_member_stale(rko, rko_u.leaders.opaque, maybe) when #result != #KEEP;
```

Composition is literally per arm path (the same path language as the arms),
covers 8 of 8, and needs no new keyword (three clause words and `maybe`);
it is the form B would elaborate into if a lower level were wanted, as
`|-> #absent` elaborates into its parts (C04.19.4).

### The destination of a move

Not stated in any alternative. For the caller a moved object leaves its
custody exactly as `resource_consumed` does: no destination here is an
object the caller tracks (the result or reply op is created inside the
callback and enqueued; the children ops; `rk`'s cgroup, reached from a
borrowed formal), and stating one would add a declared assumption no
consumer reads. If the user wants it for review, a comment beside the clause
(as the arms cite `rdkafka_op.h` lines) carries it without new surface.

### Recommendation

**B.** It is the only maps-to form that states all eight candidates as they
are (A refuses six for a lack of words, not of facts), it keeps one clause
per path in the user's `|->` notation with the decided `TAG.FIELD` left-hand
side, and its values are a closed set whose composition with the arms is the
table above (to be proved in Lean: no double release, no missed release, an
unstated member never decides). The may-set is the one new piece of surface;
`{V, #kept}` reads as "maps to one of" and admits only `#kept` as its second
element.

## 4. Body checks and standing (planned, for whichever form is chosen)

* **Checked where the body shows it** (rule 13y-style, per unit): a `taken,
  cleared` path is validated when, on every return of the guard's class, the
  candidate's own body (or a static callee in its unit that it calls with the
  formal) stores a null pointer constant into that path (or hands `&P->path`
  to `rd_list_move` as the source) after reading it; a `stale` path when the
  body never stores into it. A body that stores into, or hands `&P->m` for, a
  path the declaration does not name withdraws the declaration in that unit.
* **Declared assumption otherwise** (rules 13i–13k): that the taken object is
  released or moved (not leaked), which needs the callees (`rd_kafka_admin_result_new`,
  the 23 parse callbacks, `rd_kafka_share_segregate_*`), and the premise of
  2.6.

## 5. Prediction (yield)

**BEFORE** (binary `private/mpr/bin/before`, `2260bdae…`, built from the
base; `run.json`'s `binary_sha256` agrees; [`before.sh`](scripts/before.sh),
[`leaves.py`](scripts/leaves.py)): corpus **936 / 9 / 3,889**, report
`99d6362f…` (the brief's figure, reproduced), 670.7 s; system 93 / 4 / 351
(81.0 s); slice 52 / 1 / 347 / 4 Pending (38.7 s); null property corpus
1,113 / 1,435 / 2,355 / 18 Unsafe, system 84 / 125 / 245, slice 71 / 103 /
232: every brief figure reproduces.
`rd_kafka_op_destroy`: **131 leaves in 83 functions**, every one `call to
rd_kafka_op_destroy: callback rd_kafka_DeleteRecords_leaders_queried_cb
(from rd_kafka_op_s.rko_op_cb) has no contract in force
(callback-uncontracted)`; **no function is decided by them alone** (every
one of the 83 carries other leaves: 840 `unresolved resource identity`, 673
indirect accesses, 439 uncontracted calls, 419 branch conditions, …; the
fewest are `rd_kafka_event_destroy` and `ut_op_destroy_free`, one other leaf
each, the op reached through an untracked pointer, `unresolved resource
identity` on the same line). No other invoker's leaf names one of the eight
(the serve and request families refuse first on `rd_kafka_poll_cb`,
`rd_kafka_consume_cb`, a null callback or a withdrawn body).

**No caller links a tracked object at a stated path.** The one corpus
function that stores into these members and destroys the op on a path,
`rd_kafka_topic_partition_list_query_leaders_async`, refuses each store
(`indirect memory access is outside the resource projection`), so no field
link exists there; the composition table acts on nothing in the corpus, and
admission changes no tracked state.

**Predicted AFTER, alternative B (or C).** All eight declared; at
`rd_kafka_op_destroy` the run's 27 `rko_op_cb` candidates are then all
contracted but `rd_kafka_oauthbearer_refresh_op`, whose own member
invocation still makes it read uncontracted (C04.19.5's last paragraph;
`HANDOFF-CANDIDATE-OWN-CALLBACKS`, `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`).
So:

* corpus **936 / 9 / 3,889**, no status move; the **131 leaves trade text
  only**, now naming `rd_kafka_oauthbearer_refresh_op`
  (`callback-uncontracted`); no Safe gain, no Safe → non-Safe;
* system 93 / 4 / 351 and slice 52 / 1 / 347 / 4 Pending: no row moves
  and no leaf changes (no leaf on either names `rd_kafka_op_destroy`, an
  invoker or a candidate, BEFORE);
* null property: every row identical (the clause is custody-only);
* declarations consulted +8 (declared assumptions where section 4's check
  cannot validate a path).

**Rows that could become Safe and what else they need.** None on this
corpus. A caller of `rd_kafka_op_destroy` would need, besides this card: the
oauthbearer candidate decided as the composition of its own clauses
(`HANDOFF-CANDIDATE-OWN-CALLBACKS`); its op argument resolved to a tracked
identity (the leading `unresolved resource identity`); its other leaves; and
rule 12n's run fold to keep it (and its dependency listing,
`MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED`). **Alternative A** declares two of the eight; the 131 leaves keep
naming `rd_kafka_DeleteRecords_leaders_queried_cb` and nothing moves at all.
