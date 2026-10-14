# OP-CONSTRUCTOR-DECLARATIONS: census, design and prediction

Frozen before any implementation code, at the syntax pause point: the surface
spellings in section 3 are a **proposal awaiting the user's choice** (the
brief: "If a new surface word is needed, propose it in the report; do not
invent an alias"). Base: local `main` at `f3e1d724c`. Rule placement: amend
13w and 12o in place; a new id, if one is truly needed, is `14e`.

## 1. Census on the base

### 1.1 Bodies (read by hand at `66910c10` and `046accf6`)

`scripts/body.py`: every body below is byte-identical at both revisions.

* **`rd_kafka_op_new0(source, type)`** (`rdkafka_op.c:224-342`). Looks up the
  payload size `op2size[type & ~RD_KAFKA_OP_FLAGMASK]`, asserts it is
  non-zero (`rd_assert`, live: no `NDEBUG` in the audited compile commands),
  allocates with `rd_calloc` (asserts its result) and stores
  `rko->rko_type = type` — the **full** actual, flags included — on the fresh
  zeroed object (so `rko_op_cb`, `rko_serve` and every payload member are
  NULL); `source` is stored only under `ENABLE_DEVEL` (off in the audited
  build). Returns the object: owned, non-null. `rd_kafka_op_new(type)` is the
  macro `rd_kafka_op_new0(NULL, type)` (`rdkafka_op.h:929-932`).
* **`rd_kafka_op_new_cb(rk, type, cb)`** (`rdkafka_op.c:785-793`).
  `rd_kafka_op_new(type | RD_KAFKA_OP_CB)`, then `rko->rko_op_cb = cb` and
  `rko->rko_rk = rk` on the fresh object. `rk` is kept as a plain pointer
  (no reference taken). Returns it: owned, non-null; its `rko_type` masked by
  `RD_KAFKA_OP_FLAGMASK` is `type`'s masked value.
* **`rd_kafka_op_req0(destq, recvq, rko, timeout_ms)`** (`:850-866`). Sets
  `rko`'s reply queue to `recvq` (`rd_kafka_op_set_replyq`: a reference
  `rd_kafka_q_keep` takes for the op, the caller's unchanged), enqueues `rko`
  on `destq` (`rd_kafka_q_enq`: on a disabled queue `rd_kafka_op_reply`
  re-queues it to `recvq` or destroys it; either way the caller no longer
  holds it) and returns `NULL` when that fails, otherwise
  `rd_kafka_q_pop(recvq, …, 0)`: an op or `NULL`. The pop serves `recvq`
  through `rd_kafka_q_pop_serve0` with `RD_KAFKA_Q_CB_RETURN` and a null
  callback, so each dequeued op's `rko_serve` (set at enqueue from the
  original queue's `rkq_serve`) and `rko_op_cb` may run, exactly as the
  declared `rd_kafka_q_pop_serve*` state. The returned op's type is the
  replier's (usually the request op re-used with `| RD_KAFKA_OP_REPLY`, or
  `rd_kafka_op_new_reply`'s copy, or any op queued on `recvq`): **never
  decided**.
* **`rd_kafka_op_req(destq, rko, timeout_ms)`** (`:874-886`). A temporary
  `recvq = rd_kafka_q_new(destq->rkq_rk)`, `rd_kafka_op_req0`, then
  `rd_kafka_q_destroy_owner(recvq)` (which purges what is left). Same result.
* **`rd_kafka_op_req2(destq, type)`** (`:892-897`). `rd_kafka_op_new(type)`
  then `rd_kafka_op_req(destq, rko, RD_POLL_INFINITE)`. Same result; the
  request op never reaches the caller.

### 1.2 Call sites by how the type actual is decided

`scripts/ctor_census.py` ([census](artifacts/ctor-census.json); a textual
reading aid over librdkafka's `src/*.c` at both revisions, identical; the
gateway reads Clang's AST): 140 calls.

| callee | constant | formal | computed | no type actual |
| --- | ---: | ---: | ---: | ---: |
| `rd_kafka_op_new` (→ `new0`) | 81 | 3 | 2 | |
| `rd_kafka_op_new_cb` | 12 | 1 | | |
| `rd_kafka_txn_op_req` macro (→ `new_cb(rk, RD_KAFKA_OP_TXN, op_cb)`) | 9 | | | |
| `rd_kafka_op_req2` | 7 | | | |
| `rd_kafka_op_req` | | | | 24 |
| `rd_kafka_op_req0` | | | | 1 |

* **Formal** (the arm stays undecided: the cautious join):
  `rd_kafka_cgrp_op(type)`, `rd_kafka_toppar_op(type)`,
  `rd_kafka_admin_request_op_new(optype)` (through `new_cb`) and
  `rd_kafka_op_req2`'s own `type`.
* **Computed** (undecided): `rd_kafka_op_new_reply`
  (`rko_orig->rko_type | RD_KAFKA_OP_REPLY`) and `rd_kafka_op_new_cb`'s own
  `type | RD_KAFKA_OP_CB` (decided by the declaration's clause, not at the
  call).
* **Callback actuals of `new_cb`**: 12 designators, 1 computed
  (`rko->rko_u.leaders.cb` in the leaders worker); the txn macro's 9 are
  designators.
* By file, the constant constructor sites are spread over the client:
  `rdkafka.c` 13, `rdkafka_mock.c` 21, `rdkafka_partition.c` 12,
  `rdkafka_cgrp.c` 10, `rdkafka_txnmgr.c` 1 + 9, `rdkafka_broker.c` 5,
  `rdkafka_op.c` 5, unit tests 5, others 1–3 each.

**Where a decided arm or a per-path registration could be read**: only at an
`rd_kafka_op_destroy` of the op in the body that created it. Six textual
sites destroy a constructed op in its creating body: `rd_kafka_committed`
(`OFFSET_FETCH`), `rd_kafka_cgrp_metadata_refresh` (`new_cb`, `METADATA`,
`rd_kafka_cgrp_handle_Metadata_op`), `rd_kafka_share_build_response_rko`,
`rd_kafka_metadata`, `rd_kafka_mock_topic_set_error`,
`rd_kafka_topic_partition_list_query_leaders_async` (`new_cb`, `LEADERS`),
several of them destroying a reply op the variable was reassigned to. CG22's
destroy-actual tally agrees: of 96 textual destroys, 2 destroy a `new_cb`
local and 1 a `new`/`req` local; 4 destroy an `rd_kafka_op_req2` reply
(undecided); 25 a unit-test helper's op (`ut_make_*`,
`ut_create_share_fetch_response_rko`, not declared constructors).

### 1.3 The discriminant's run-wide stability (stores to `rko_type`)

Textual census over librdkafka's sources (the gateway re-reads every
analysed unit's Clang AST):

| store | where | classification |
| --- | --- | --- |
| `rko->rko_type = type` | `rd_kafka_op_new0` (`rdkafka_op.c:335`) | construction-time: on the fresh object the body allocated |
| `rko->rko_type \|= (rko->rko_op_cb ? RD_KAFKA_OP_CB : RD_KAFKA_OP_REPLY)` | `rd_kafka_op_reply` (`:811`), `rd_kafka_op_error_reply` (`:836`) | masked flag-or: both values lie inside `RD_KAFKA_OP_FLAGMASK` |
| `rko->rko_type = RD_KAFKA_OP_PARTITION_JOIN` | `rd_kafka_broker_op_serve` (`rdkafka_broker.c:3631`) | a rewrite on an existing object, under `case RD_KAFKA_OP_PARTITION_LEAVE:` of `switch (rko->rko_type)` (`:3356`, `:3532`), `rko` a formal not assigned between |

No whole-object write of an op (`memcpy`/`memset`/struct assignment) appears.

**The `PARTITION_LEAVE` → `PARTITION_JOIN` rewrite.** Proposed treatment
(no surface word; the card's own judgment, open to the user): a store of a
constant into the discriminant of an existing object is classified only when
it sits in a case group of a `switch` on the same object's discriminant
(masked or not), the object's pointer is never assigned in the function, and
the group's labels are constants. It then rewrites the values its labels name
(here exactly `17`) to the stored value (`16`). A decided value `v` is read
as the least set containing `v` and closed under every classified rewrite, and
the consumer joins the arms over that set. `PARTITION_LEAVE` (17) and
`PARTITION_JOIN` (16) both fall to `rd_kafka_op_destroy`'s `otherwise`
(empty) arm, so a decided 17 reads `{17, 16}` and still selects exactly
`otherwise`; no other decided value moves. Any other store into
`rd_kafka_op_s.rko_type` (an unguarded constant, a non-constant value on an
existing object, a flag-or with bits outside a reading mask, a whole-object
write) is **unclassified and withdraws every decided arm for the run**
(rule 12o's fold demotes a row resting on one, as rule 12n's does).

### 1.4 Stores to `rko_op_cb` (the per-path registration's stability)

CG22's census ([op-callback-census](../cg22-op-callbacks-served-queues/artifacts/op-callback-census.json)):
`rd_kafka_op_new_cb`'s `rko->rko_op_cb = cb` (`:790`, construction);
`rd_kafka_share_enqueue_fetch_op` / `_sync_ack_op` (`rdkafka.c:3648`,
`:4188`) and the two offset-reset paths (`rdkafka_offset.c:811`, `:1136`):
each a designator stored into an op the same body just made with
`rd_kafka_op_new`; `rd_kafka_op_call` (`:941`) stores `NULL` into an
existing op (only removes a candidate). So: a registration read on the path
is kept at a later destroy only when every store into
`rd_kafka_op_s.rko_op_cb` in the run is on the storing body's own freshly
constructed object (which that body's route reads on its own path: a
designator replaces the registration, a null constant empties it, anything
else makes it undecided) or a null constant; otherwise the registration is
withdrawn run-wide and the destroy reads rule 12n's closure as today.

## 2. Measured sizes on the base

BEFORE binary `bin/before` (`7a64858c…`, built from `f3e1d724c` by
`scripts/build.sh`; each `run.json` names it), every route with
`QCORE_NULL_PROPERTY` (`scripts/measure-binary.sh`, logs `private/opctor/`):

| route | BEFORE | report sha256 | wall (s) |
| --- | --- | --- | ---: |
| corpus `66910c10` | 935 / 9 / 3,890 | `527a4cad…` (the brief's) | 549.2 |
| system | 93 / 4 / 351 | `b2929d30…` (paths embedded; totals agree) | 78.8 |
| slice | 52 / 1 / 347, 4 Pending | `5203a285…` (the same) | 39.5 |
| null, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | rows `a0a1e6d7…` (CG22's) | |
| null, system / slice | 84 / 125 / 245; 71 / 103 / 232 | | |

Member sizes (`scripts/size_members.py`; leaves / functions / decided
alone), corpus ([sizes](artifacts/size-before-corpus.json)); none has a leaf
on the system route or the slice (no librdkafka unit):

| member | leaves | functions | alone |
| --- | ---: | ---: | ---: |
| `rd_kafka_op_new0` | 87 | 86 | 0 |
| `rd_kafka_op_new_cb` | 14 | 14 | 0 |
| `rd_kafka_op_req` | 2 | 2 | 0 |
| `rd_kafka_op_req0` | 1 | 1 | 0 |
| `rd_kafka_op_req2` | 6 | 6 | 0 |
| `rd_kafka_op_destroy` | 132 | 83 | 2 |
| `rd_kafka_q_enq` | 87 | 83 | 1 |
| `rd_kafka_q_pop` | 38 | 15 | 0 |
| `rd_kafka_op_set_replyq` | 7 | 7 | 0 |
| `rd_kafka_op_reply` | 13 | 7 | 0 |

The ledger's sizes reproduce (`new0` 87 / 86 / 0, `new_cb` 14 / 14 / 0). The
132 `rd_kafka_op_destroy` leaves are 119 `callback-member-open` and 13
`callback-uncontracted`, as CG22 measured. Of the 32 textual
`rd_kafka_op_req*` sites only 9 carry a leaf (2 + 1 + 6): the rest sit in
functions already refused earlier on the path.

## 3. Design, and the surface the user must choose

Declarations go into `qcore-provider-librdkafka/contracts/librdkafka/client.qv`
through `scripts/write_contracts.py`, each body as read in 1.1, pinned at both
revisions.

| function | clauses (existing words) | new surface |
| --- | --- | --- |
| `rd_kafka_op_new0` | `resource_premise(asserted_allocation, …)` (as `rd_kafka_q_new0`), `source`/`type` borrowed, `resource_return_owned(nonnull)` | the discriminant set from `type` (A); optionally the result's `rko_op_cb` empty (C) |
| `rd_kafka_op_new_cb` | the same premise, `rk`/`type`/`cb` borrowed, `resource_return_owned(nonnull)` | the discriminant from `type` plus the `RD_KAFKA_OP_CB` flag (A); `cb` registered in the result's `rko_op_cb` (B) |
| `rd_kafka_op_req0` | `destq`, `recvq`, `timeout_ms` borrowed; `resource_consumed(rko, nonnull)`; the two member invocations over `supplied_owned` ops `rd_kafka_q_pop_serve0` carries; `resource_return_owned(nullable)` | none: the result's discriminant is not stated (undecided) |
| `rd_kafka_op_req` | as `req0` without `recvq` | none |
| `rd_kafka_op_req2` | `destq`, `type` borrowed; the member invocations; `resource_return_owned(nullable)` | none |

**A. The discriminant set at construction (a new word: approval needed).**
No existing clause states a member's value; `resource_discriminant` is a
`requires` on the releasing function and names no struct tag. Proposed,
following the user's dotted `TAG.FIELD` (the stability check of 1.3 needs the
tag, as `nonnull_member_stable(TAG, FIELD)` does):

* **A1 (recommended).**
  `ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0);`
  (`new0`) and `…(result, rd_kafka_op_s.rko_type, type, 536870912);`
  (`new_cb`, `RD_KAFKA_OP_CB = 1 << 29`): the result's member holds the
  actual of the integer parameter `type` OR-ed with the canonical decimal
  flags. Exact: a call's decided value is `actual | FLAGS` when the actual
  folds to a constant, and a consumer selects the arm of that value under its
  own `resource_discriminant` mask. A non-constant actual decides nothing.
* **A2.** The brief's three-operand form
  `resource_discriminant_set(result, rd_kafka_op_s.rko_type, type)`, read as
  "equal to `type` under the reading mask" (the `RD_KAFKA_OP_CB` that
  `new_cb` adds is cleared by `rd_kafka_op_destroy`'s mask). Shorter, but the
  value is stated only up to a mask the constructor does not name, so the
  loader cannot check it against a consumer.

**B. A registration into the result's member (a new form: approval needed).**
Rule 13w's `resource_callback_registered(F, H)` registers into a holder's one
registry; an op carries two function-pointer members (`rko_op_cb`,
`rko_serve`), and `rd_kafka_op_destroy` invokes a member
(`resource_callback_member_invoked(rko, rd_kafka_op_s.rko_op_cb, …)`), so the
registration must name the member:

* **B1 (recommended).**
  `ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, cb);`
  mirroring `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)`.
* **B2.** A third operand on the existing word:
  `ensures normal resource_callback_registered(cb, result, rd_kafka_op_s.rko_op_cb);`

Meaning (either spelling): on the path, the tracked result's member holds
exactly `cb`'s candidates (rule 11p's resolution of the actual; a null
constant: none); `rd_kafka_op_destroy`'s member invocation on that tracked
object reads these candidates instead of rule 12n's closure, while the
registration is in force on every arriving path (rule 13w's agreement join)
and the run-wide stability of 1.4 holds.

**C. The fresh object's member is empty (optional; a new form if wanted).**
`rd_kafka_op_new0`'s result has `rko_op_cb == NULL` (calloc). Stated, a
destroy of a `new0` op in its creating body (with no store into the member
between) reads no candidate rather than the run's closure. Options:
**C1** `ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, null);`
(B1's form with the null word), **C2** a separate
`resource_callback_member_empty(result, rd_kafka_op_s.rko_op_cb)`, or **C3**
(recommended for scope) not stated in this card: a `new0` op's destroy keeps
reading the closure, registered as a residual with its size.

Everything else needs no new word: the owned results, the consumed request
op, the member invocations of the request family (existing C04.19.3 words),
the run-wide stability (a check, not a clause) and the rewrite treatment of
1.3.

## 4. Prediction

* **Corpus, system, slice: unchanged in status** (935 / 9 / 3,890;
  93 / 4 / 351; 52 / 1 / 347 / 4 Pending). No function has a constructor as
  its only leaf (ledger: `new0` alone 0, `new_cb` alone 0); every created op
  is next handed to an undeclared call (`rd_kafka_q_enq`, 87 leaves,
  `rd_kafka_op_set_replyq`, `rd_kafka_MetadataRequest`, …) or to
  `rd_kafka_op_destroy`, whose candidates are uncontracted
  (`OP-CALLBACK-CANDIDATE-CONTRACTS`).
* **No Safe → non-Safe; no Unknown → Safe expected.** An `Unknown → Unsafe`
  could appear only where a tracked op reaches an exit through contracted
  calls alone; none is expected, and any would be read by hand and reported as
  provisional, never an upstream defect.
* **Leaves.** `call to uncontracted function rd_kafka_op_new0` (87) and
  `rd_kafka_op_new_cb` (14) and the request family's leaves disappear; the
  rows keep their next refusal. At the few in-body destroys of 1.2: a
  `new_cb` op's destroy refuses `callback-uncontracted` naming the
  registered candidate (`rd_kafka_cgrp_handle_Metadata_op`,
  `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb`) instead
  of the closure; a decided arm replaces the join (no `arm-undecided` there).
  The 29 `rd_kafka_q_pop` and 16 formal destroys do not move.
* **Null property**: unchanged rows on all three routes (no null clause is
  added; owned non-null results are custody facts).
* **Lean**: `QCore.OpCallbacks` gains the decided-arm selection, the
  unstable/undecided join and the registration-then-destroy theorem; audits
  318 stays 318 (an existing module extended) unless a new audit file is
  needed.
