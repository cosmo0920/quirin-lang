# REFCOUNT-FAMILY-BLOCKING-CALLEES: prediction

Frozen before any contract is written. Base: local `main` at `652dc085f`
(REFCOUNT-REFERENCE-RETURNING-DECLARATIONS' gate records).

## BEFORE, reproduced

Binary `private/rfbc/bin/before` (`24b13b40…`, equal to the corpus
`run.json`'s `binary_sha256`), built from `652dc085f`:

| route | totals | digest |
| --- | --- | --- |
| corpus | **880 / 9 / 3,945** | `79dc429d…` (the brief's) |
| system | 87 / 4 / 357 | `2ad95080…` (embeds this worktree's paths) |
| slice | 45 / 1 / 354, 4 Pending | `ca70a3e4…` (embeds this worktree's paths) |

`scripts/refusals.py` ([refusals-before-corpus.json](artifacts/refusals-before-corpus.json)):
`reference-drop-unheld` through a plain name **127 leaves / 89 functions / 1
alone** (the previous card's 127), member 3, address 1, unread 10;
`reference-unbalanced` **11 leaves / 10 functions**; `reference-dropped` 20 / 5;
`reference-undecided` 8 / 8. System and slice carry no `reference-` leaf.
`scripts/rank.py`: 523 Unknown functions blocked by uncontracted calls alone,
367 by one callee alone (the ledger's figures).

## Which refused statement withdrew each family

`scripts/withdrawers.py` ([withdrawers-before-corpus.json](artifacts/withdrawers-before-corpus.json),
[table](artifacts/withdrawers-before.txt)) reads, per plain-name unheld drop,
its family's origin (origins.py), and every refused leaf of the same function
from the origin's assignment line (inclusive) to the drop whose statement
hands the name on as rule 13b's refused hand-on rule reads it (a value, a
call actual, an operand; not a member value read, not an assignment's
destination). Lexical and approximate; AFTER is the arbiter.

Of the 127 plain-name drops, **65 have an origin no declaration covers**
(an undeclared call 35, a member read or list walk 26, a formal 4: the
previous card's classes, `REFCOUNT-REFERENCE-RESULT-BODY-SHAPES` and
`REFCOUNT-DROP-OWN-BODY-TRANSFER`), so no family formed. The other **62**
have a declared origin and were withdrawn; every one has at least one
withdrawer. Their **first** withdrawer (the family's breaker):

| breaker | families |
| --- | ---: |
| **an uncontracted callee** (12 callees, below) | **17** |
| access gate at the acquiring `rd_kafka_q_new(X->f)`: a member actual at a withheld declaration's retained `rk` (`RETAINING-CALL-MEMBER-ACTUAL`; even read, rule 12t refuses a member `P`) | 13 |
| unresolved identity at the acquiring `ut_create_mock_toppar(rkshare->…)` (a member of a tracked identity as actual) | 9 |
| unresolved identity in a null test of a name two acquisitions join (`rktp = get(…); if (!rktp) rktp = desired_get(…); if (unlikely(!rktp))`) | 5 |
| access gate, `&(rktp)->rktp_lock` at `mtx_lock` (the lock "wrappers" are macros: `rd_kafka_toppar_lock`, `rd_kafka_topic_*lock`; residual address class of `W47-ACCESS-PATH-POSITION-RESIDUALS`) | 4 |
| `retention-undecided` at the acquiring `rd_kafka_q_new0` (rule 13e) | 3 |
| loop re-acquisition (a second generation while one may be live) | 3 |
| unresolved identity at a `rd_kafka_toppar_get2` statement (`rktpar` member) | 2 |
| branch condition `if ((err = rd_kafka_toppar_op_seek(…)))` | 2 |
| braced initializer; access at `rd_atomic64_add`; two unresolved identities | 4 |

Across **all** withdrawers (not only the first) the access gate's
`&(rktp)->rktp_lock` / unlock refusal appears 91 times: even where a callee
is contracted, a later lock line withdraws the family again. Twelve of the 62
families have **only** uncontracted callees as withdrawers
([closure-before.txt](artifacts/closure-before.txt)).

## The uncontracted breakers, each body read

Ranked by families broken first (i); (ii) functions the reference route
would decide if the callee alone were contracted; (iii) of those, functions
whose every other leaf names the same callee (the CG10-residual check).

| callee | (i) | (ii) | (iii) | body (both revisions identical, `scripts/body.py`) | class |
| --- | ---: | ---: | ---: | --- | --- |
| `rd_kafka_toppar_op_seek` | 3 | 0 | 0 | `pos` (`rd_kafka_fetch_pos_t`) and `replyq` (`rd_kafka_replyq_t`, carrying a kept queue reference the op takes over) by value; keeps `rktp` into the op | by-value struct ABI + kept into an owner |
| `rd_kafka_ListOffsetsRequest` | 2 | 0 | 0 | `replyq` by value | by-value struct ABI |
| `rd_kafka_consumer_close_q` | 2 | 0 | 0 | `rd_kafka_q_fwd_set(…, rkq)` keeps `rkq` into two forwarding queues; `RD_KAFKA_REPLYQ(rkq, 0)` keeps it into the terminate op | kept into an owner |
| `rd_kafka_share_fetch_reply_handle_partition_error` | 2 | 0 | 0 | `rd_kafka_consumer_err(…, rktp, …)` keeps `rktp` into an error op (`rdkafka_op.c:690`) | kept into an owner |
| `rd_kafka_replyq_make` | 1 | 0 | 0 | returns `rd_kafka_replyq_t` by value holding `rd_kafka_q_keep(rkq)` | by-value struct result + kept |
| `rd_kafka_op_set_replyq` | 1 | 0 | 0 | keeps `rkq` into `rko->rko_replyq` | kept into an owner |
| `rd_kafka_broker_purge_queues` | 1 | 0 | 0 | `replyq` by value, stored into the op | by-value struct ABI |
| `rd_kafka_consume_stop0` | 1 | 1 | 0 | `rd_kafka_toppar_desired_del` drops the desired list's reference; `rd_kafka_toppar_op_fetch_stop` keeps `rktp` into an op | kept into / dropped from an owner |
| `rd_kafka_offset_store0` | 1 | 1 | 0 | `pos` by value; touches no count | by-value struct ABI |
| `rd_kafka_toppar_broker` | 1 | 1 | 0 | returns `rd_kafka_broker_keep(rktp->rktp_broker)` under a condition: a returned broker reference of a member, dropped by callers through the `rd_refcnt_*` primitive macro | rule 13d shape (`REFCOUNT-REFERENCE-RESULT-BODY-SHAPES`) |
| `rd_kafka_toppar_op_pause_resume` | 1 | 0 | 0 | `replyq` by value; keeps `rktp` into the op | by-value struct ABI + kept |
| `rd_kafka_toppar_pid_change` | 1 | 0 | 0 | `pid` (`rd_kafka_pid_t`) by value | by-value struct ABI |

**None of the twelve is declarable with the vocabulary on main.** Seven keep
(or drop) a reference on the object they are handed and store it into an
owner they build or reach (an op, a reply queue, a forwarding queue, the
desired list): the brief forbids a plain borrowed reading of such a member,
and C04.17 has no clause for "kept into an owner" (a reference clause cannot
stand beside a retention of the same parameter, and a retention is not a
reference). Six take or return a C structure by value (`rd_kafka_replyq_t`,
`rd_kafka_fetch_pos_t`, `rd_kafka_pid_t`), which the declared-ABI reader does
not project; `rd_kafka_replyq_t` also carries a kept queue reference the
callee takes over (a transfer through a by-value aggregate). One is rule 13d's
conditional-keep shape. These are recorded, not declared:
`W48-CALLEE-DECISION-TABLE` class 3 (kept into an owner: 7 callees, 12
first-broken families) and a registered reader residual (by-value
aggregates).

## What will be declared

The declarable members of the 62 families' withdrawer sets (each read at both
revisions), in `qcore-provider-librdkafka` `librdkafka/client.qv`, existing
vocabulary only, no reference clause (each body touches no count of the
object it is handed; the callees named below keep and drop a *different*
object or one an op holds):

* `rd_kafka_q_wait_result(rkq, timeout_ms) -> i32` (5 families): pops one op and
  destroys it; `rkq` borrowed, result an `rd_kafka_resp_err_t` (`i32`, rule
  12u), `resource_return_none()`.
* `rd_kafka_q_pop(rkq, timeout_us, version) -> ptr` (15 families): the popped
  op is the caller's (`resource_return_owned(nullable)`: NULL on timeout or
  yield); `rkq` borrowed. Inside, `rd_kafka_q_fwd_get` keeps the forward
  queue and `rd_kafka_q_destroy` drops it (another object, balanced); handled
  ops are destroyed.
* `rd_kafka_q_purge0(rkq, do_lock) -> i32` (1 family): moves the queue's ops
  out and destroys them, or purges the forward queue; `rkq` borrowed.
* `rd_kafka_toppar_needs_query(rk, rktp) -> ptr<const, u8>` (1 family): reads
  only, returns a string literal or NULL (`resource_return_none()`, as
  `gai_strerror`).
* `rd_kafka_toppar_fetch_backoff(rkb, rktp, err) -> i64` (2 families): writes
  one scalar member, reads configuration, logs.

The served-queue premise (stated in the module, constraint 4): `q_pop`,
`q_wait_result` and `q_purge0` may run a popped op's own callback
(`rd_kafka_op_handle` → `rd_kafka_op_call`, `rd_kafka_op_destroy`'s
`ERR__DESTROY` call) with `rkq` as the served queue; that callback takes no
reference its caller's caller holds. The callbacks themselves are not
modelled (`CG22-CALLBACK-REGISTRY-CONTRACT`); `rd_kafka_q_serve*` and
`rd_kafka_q_pop_serve*`, which take a callback argument, stay undeclared there.

**Yield.** With all five declared, **0 families are repaired** (each family
that needs one of them also needs an undeclarable callee or a non-callee
breaker), so **0 functions** become decided by the reference route and 0 go
`Safe` through it. In the uncontracted-call population, `rd_kafka_q_purge0`
decides `rd_kafka_toppar_purge_and_disable_queues` alone (two member actuals
`rktp->rktp_fetchq`, `rktp->rktp_ops` at a function-map borrowed position,
which the gate reads), the only function any of the five frees. The stopping
rule (yield per contract about 1) is reached at the head: the ranking's heads
are not declarable, and the declarable members free at most one function
each.

## Predicted AFTER

| item | prediction |
| --- | --- |
| Unknown → Safe | **1** (`rd_kafka_toppar_purge_and_disable_queues`), range 0 to 3 |
| Safe → non-Safe | 0 (no `Safe` row's text calls any of the five, `scripts/safe_callers.py`) |
| corpus | 881 / 9 / 3,944 |
| system / slice | no row moves (none of the five is called there) |
| plain-name unheld drops | 127, unchanged (no family repaired), range 125 to 127 |
| `reference-unbalanced` | 11, unchanged |
| re-attributed Unknown functions | 30 to 60 (`call to uncontracted function` leaves on the five lost; later refusals, mostly `rd_kafka_op_destroy` on the popped op and the access gate, remain) |
| new leak candidates | 0 to 2 over the popped op (an owned result now tracked); each read by hand |
| Lean | none: contracts only, no semantics added |
