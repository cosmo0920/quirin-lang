# LIBRDKAFKA-HEADER-INLINES

Card brief: `private/integrator-briefs/r16-librdkafka-inlines.md` (round 16,
2026-11-05; common brief `r13-common.md`). Base `945c0db1b`. The card is new;
it is registered and retired through its [amendments](integration-amendments.json).

> **USER REQUEST 2026-11-05** (verbatim): "Now, we're still needed to fill the
> gap of the missing contracts." The integrator refilled the free slot from the
> contract-gap queue under the user's standing three-card rule.

**Question.** The user's `quirin-audit current` run at Fluent Bit `046accf6`
reports librdkafka callees as uncontracted although the run has no body for
them: `rd_kafka_q_enq` (83 functions), the `rd_kafka_buf_write_*` family,
`rd_kafka_replyq_destroy`, `rd_kafka_set_last_error`, `rd_kafkap_str_new`, ...
Where are their bodies, why does the run have none, and which can be declared
truthfully?

**Answer.**

- **Where.** Of the 260 callees with no body in the run that a librdkafka unit
  calls (1,014 pairs), 89 (629 pairs) are librdkafka's own `static` header
  definitions. All 89 are byte-identical at `046accf6`, `66910c10` and
  `34155955`. The other 171 are owned elsewhere: libc and builtins 22,
  OpenSSL 68, libcurl 56, zstd 7, zlib 5, Cyrus SASL 4, calls through
  function-pointer objects 8, and one name already declared (`rd_kafka_q_deq0`).
- **Why no body.** Every one of the 89 has internal linkage, so gateway
  constraint 4a already offers its body to the W4 derivation in every
  including unit. None is an external-linkage `inline`, so no linkage reading
  (`W44-CALLEE-EXTERNAL-INLINE-DEFINITION`) applies and step 1 had no stop.
  The run published no summary because the bodies do not validate. For
  example, the writers reach the cross-unit `rd_buf_write`, the queues reach
  `rd_kafka_op_destroy`, and some bodies use struct results.
- **Declared.** 59 callees, 524 pairs, in a new resource-v1 module
  [`librdkafka/header_inlines.qv`](../../../providers/qcore-provider-librdkafka/contracts/librdkafka/header_inlines.qv).
  Each is stated from its body and pinned at `66910c10` and `046accf6`.
  Twelve name `rk_plugin_callback_closure`.
- **Not declared, with the missing form named.** 27 callees, 84 pairs.
- **Owned elsewhere.** The three `rd_atomic32_*` primitives, 21 pairs, belong
  to `REFCOUNT-PLAIN-COUNTER-DECLARATION`. Contract spec C04.17 forbids
  declaring the primitive under `rd_refcnt_t`.

No analyzer code, rule or Lean model changes. Every declaration is a trusted,
pinned assumption read from the body, with the same strength as the crate's
earlier header declarations (`W46-HEADER-INLINE-DECLARED-UNCHECKED`).

## 1. Inputs and method

- **Run report.** The main checkout's
  `build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json`
  (SHA-256 `e965950e…d017f`, read only). It is the same report
  CONTRACT-ADOPTION-NOT-IN-FORCE read: 2,790 uncontracted callees.
- **Sources.** The librdkafka `src/` trees at `046accf6` (the run's checkout),
  `66910c10` (`build/cycle19/clang/66910c10…`) and `34155955` (the slice
  checkout `private/w47-tail/attempt-1/prep/…`). They are read as files only.
- **Census.** [census/census.py](census/census.py) writes
  [census/census.json](census/census.json) and [census/census.md](census/census.md).
  For each callee it records:
  - every file-scope definition, declaration and macro at the three
    revisions;
  - each definition's storage class and body digest;
  - the body's direct calls that the run itself reports uncontracted (a
    reading aid, never the derivation's record);
  - the run's callers, and its "only uncontracted" rows.

  `census.json` is the step-1 output at the base, committed in `c7529d30a`
  before anything changed. A rerun after this card's module exists also lists
  the new declarations under `declared_in`. The table's last column comes from
  [census/decisions.json](census/decisions.json). To rerun it:

```sh
python3 docs/reports/librdkafka-header-inlines/census/census.py \
  --report <run report> \
  --adoption docs/reports/contract-adoption-not-in-force/census/census.json \
  --tree 046accf6=<046accf6 checkout> --tree 66910c10=<66910c10 checkout> \
  --tree 34155955=<34155955 checkout> \
  --out docs/reports/librdkafka-header-inlines/census \
  --decisions docs/reports/librdkafka-header-inlines/census/decisions.json
```

- **Premise census.** LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's call-graph reading
  aid is copied to [scripts/](scripts/), with the tree roots spelled
  canonically. It walks only this card's module. One cast that the lexical
  scan reads as a call is resolved by hand
  (`rd_kafka_curr_msgs_add`, `rdkafka_int.h:1186`).
  `scripts/preprocess.py` (94 units at each revision, every exit 0) and then
  `scripts/reach.py` write
  [census/premise-reach.json](census/premise-reach.json). The results are
  identical at both revisions for all 59 declarations.

## 2. Census

The full table is [census/census.md](census/census.md). Summary of the 89
in-scope callees:

| Outcome | Callees | Pairs | Members (pairs) |
|---|---:|---:|---|
| Declared, borrowed (scalar or no result) | 47 | 398 | 14 writers and 5 updaters, `rd_kafka_buf_finalize_arraycnt` and `_crc_init`; `rd_kafka_set_last_error`; `rd_kafka_q_len`/`_yield`/`_reset`/`_concat0`/`_prepend0`; `rd_kafka_msgq_init`/`_concat`/`_move`/`_purge`; accessors; `rd_atomic64_set`, `rd_usleep`, `rd_crc32_*`, `rd_buf_write_pos`, varint readers, `rd_avl_remove_elm` |
| Declared, the argument consumed | 3 | 92 | `rd_kafka_q_enq` 83, `rd_kafka_q_reenq` 1, `rd_kafka_msgq_enq` 8 |
| Declared, owned result | 3 | 16 | `rd_kafkap_str_new` 12, `rd_kafkap_bytes_new` 3, `rd_kafka_partition_leader_new` 1 |
| Declared, released argument | 1 | 1 | `rd_kafka_partition_leader_destroy` |
| Declared, derived result | 5 | 17 | `rd_kafka_msgq_first`/`_last` and `rd_kafka_topic_partition_toppar` (borrowed from the parent), `rd_tmpabuf_alloc0` (borrowed from `tab`), `rd_kafka_rkt_get_lw` (the argument or NULL) |
| Undeclared: a reference held at a member path | 7 | 39 | `rd_kafka_replyq_destroy` 21, `rd_kafka_op_set_replyq` 7, `rd_kafka_replyq_enq` 3, `rd_kafka_enq_once_*` (4) 8 |
| Undeclared: a struct passed or returned by value | 5 | 15 | `rd_kafka_idemp_get_pid0` 7, `rd_kafka_replyq_make` 3, `rd_kafka_offset_store0` 3, `rd_kafka_topic_partition_get_fetch_pos` 1, `rd_slice_pos` 1 |
| Undeclared: a removal from a borrowed container | 1 | 5 | `rd_kafka_msgq_deq` |
| Undeclared: other missing forms or body checks | 14 | 25 | §4 |
| Owned elsewhere (plain counters) | 3 | 21 | `rd_atomic32_add` 10, `_sub` 6, `_set` 5 |

(The pairs in the first five rows add to 524.)

## 3. The declarations

The bodies and the reading of each group are in the module's comments. The
clauses that carry more than a borrow:

- **The request-buffer writers** borrow the buffer and the data. They return
  the write offset, a scalar. `rd_buf_write` (`rdbuf.c:448`) copies the bytes
  into segments the buffer owns and allocates new segments as it grows. The
  buffer is never replaced, and no pointer to the data is kept. A pointer the
  caller took into the buffer earlier is ended at the call by C04.3.4 rule 2:
  every call naming the parent ends its borrows. That is the truthful reading
  of the growth.
- **`rd_kafka_q_enq` / `_reenq`.** `rd_kafka_q_enq1` (`rdkafka_queue.h:425-470`)
  has three exits:
  - The queue is disabled. The op is replied to (`rd_kafka_op_reply`,
    `rdkafka_op.c:804-816`). It is destroyed when it has no reply queue
    (return 0), and otherwise enqueued on the reply queue, which may itself
    destroy it.
  - The op is linked into the queue (return 1).
  - The op is handed to the forward queue, whose own result is ignored
    (return 1, possibly after a destruction).

  On every exit the caller holds nothing of the op, so it is
  `resource_consumed(rko, nonnull)`. That claims neither destruction nor
  linking (C04.2). A return code does not decide the custody, so no
  `resource_on_return` split is stated: a 1 can follow a destruction. The
  body dereferences `rko` on every path, so the op is non-null.
- **`rd_kafka_msgq_enq`** links the message into the queue, whose purge or
  delivery frees it, so the message is consumed.
- **`rd_kafkap_str_new` / `_bytes_new` and `rd_kafka_partition_leader_new`**
  return one fresh `rd_malloc` block, read as owned and nullable. This is the
  crate's reading of `rd_malloc` through libc `malloc`.
  `rd_kafka_partition_leader_destroy` releases the leader with its children.
- **Derived results.**
  - `rd_kafka_msgq_first`/`_last` return a message the queue holds, and
    `rd_kafka_topic_partition_toppar` returns the toppar the partition's
    private part holds without taking a reference. Each is borrowed from its
    parent, and nullable.
  - `rd_tmpabuf_alloc0` returns a pointer into the arena block `tab` owns, or
    NULL.
  - `rd_kafka_rkt_get_lw` returns its argument cast, or NULL.

  Rule 13a checks the borrowed results structurally in every including unit;
  a body that does not validate withdraws its declaration there (the cautious
  direction).
- **`rd_kafka_txn_add_partition`** links the toppar into the client's
  pending list with a reference of its own. The caller's reference is
  untouched, so the toppar is borrowed.

**The premise.** Under user decision U2 (2026-10-14, "List it in every
dependent verdict"), the twelve declarations whose bodies may run a callback
name `rk_plugin_callback_closure`, with the exact text of the other 89
declarations:

- **Eight** reach an rk_conf or interceptor callback through direct calls
  (`rk-conf`): `rd_kafka_q_enq`, `_reenq`, `_len`, `_yield`,
  `rd_kafka_msgq_purge`, `rd_kafka_app_polled`, `rd_kafka_txn_add_partition`
  and `rd_kafka_partition_leader_destroy`.
- **Four** reach only an unresolved indirect call (`indirect`) and take the
  premise as that census's conservative label:
  - `rd_kafka_q_concat0`, `_prepend0`: the queue's `event_cb`;
  - `rd_kafka_buf_finalize_arraycnt`: a segment's `seg_free`;
  - `rd_avl_remove_elm`: the tree's `ravl_cmp`.

The paths are in [census/premise-reach.json](census/premise-reach.json). The
premise is stated plainly: its symbolic decomposition and binding metadata
(premise-symbolic-adoption) are not written for these twelve; see the
follow-up.

**Placement.** The module is `qcore-provider-librdkafka`'s (namespace
`qcore.librdkafka`), in the crate's resource view and the route manifest
(`cycle19-resource-contracts.json`). It is a new file because the existing
selected modules `client.qv`, `memory.qv`, `kafka_mem.qv` and `logging.qv` are
byte-pinned by `production_symbolic_adoption`'s migration record.

**Pins.** `quirin-audit pin --source-root <tree> --crate qcore-provider-librdkafka --function …`
ran at `66910c10` (the read-only corpus tree) and at `046accf6` (the prepared
system tree), each exit 0. The 59 entries were added to `contracts.lock`, each
with one identity reviewed at both revisions, and no existing entry changed.
`pin --check` reports librdkafka 231 recorded / 0 not in force at `66910c10`,
and 229 recorded / 2 in force through another revision at `046accf6`. Its exit
of 1 at both comes from entries of other crates that were already not in
force there (`mk_vhost_set`'s tip; `flb_config_set_program_name`,
`flb_tail_*`). `pin --verify-contracts` exits 0.
`providers/contracts.py sync` regenerated the inventory lock and
`contracts.json`, and `check` exits 0. Logs are under the scratchpad's
`pin/` and `logs/`.

**Counts.**

| Count | Before | After |
|---|---:|---:|
| Selected resource modules | 66 | 67 |
| v1 declarations | 499 | 558 |
| Corpus modules | 147 | 148 |
| `qcore-provider-*` modules | 89 | 90 |

The 59 declarations all go to the function map; none is retaining. The
updated tests are `compiler/tests/contract_loader.rs` (three counts and the
expected mode table) and `scripts/tests/test_contract_corpus.py`.

**Controls.** [`crates/qcore-c-gateway/tests/librdkafka_header_inlines.rs`](../../../crates/qcore-c-gateway/tests/librdkafka_header_inlines.rs)
loads the production module byte for byte, with libc's `malloc` and `free`:

- *Accepting:* each family used as the corpus uses it is `Safe`. This
  includes the predicted corpus shapes: a formal op handed to the queue, a
  member-read queue, and a written formal buffer.
- *Withdrawal:* without the module each accepting body has an uncontracted
  call.
- *Refusing:* none of the following is ever `Safe`:
  - an owned buffer that is only written still leaks;
  - an unreleased protocol string leaks;
  - an op freed or enqueued again after it was enqueued, a possibly-NULL op at
    the non-null consumption, and a message freed after it was enqueued;
  - a borrowed message handed to `free`.
- *The header body in the unit:* a unit that defines `rd_kafka_q_enq`'s
  non-validating forwarding body keeps the declaration in force at the call.

## 4. Not declared

Each callee keeps `call to uncontracted function`, with its missing form
recorded:

- **A reference held at a member path, 7 callees, 39 pairs.** These go to
  `REFCOUNT-MEMBER-REFERENCE-DROP`, amended.
  - `rd_kafka_replyq_destroy` drops the reference `replyq->q` holds. C04.17
    reads a drop only on the object a parameter designates. C04.19.6's
    `#released` is a release of the member's object, not one reference of it.
  - `rd_kafka_op_set_replyq` keeps a queue reference into `rko->rko_replyq`.
    A keep into a place that is not a plain name is refused.
  - `rd_kafka_replyq_enq` takes the member's reference over, enqueues and
    drops it.
  - The four `rd_kafka_enq_once_*` drop an eonce reference, whose family has
    no declared constructor or keep, and the member reply queue's reference.
    Two of them hand back the op the eonce held.
- **A struct passed or returned by value, 5 callees, 15 pairs:**
  `rd_kafka_idemp_get_pid0`, `rd_kafka_replyq_make`,
  `rd_kafka_topic_partition_get_fetch_pos`, `rd_slice_pos` and the parameter
  of `rd_kafka_offset_store0`. This is the vocabulary gap
  CFL-MPACK-EXTERNAL-DECLARATIONS registers as `MPACK-STRUCT-RESULT-BY-VALUE`.
  That card has not landed at this base, so the gap is recorded under
  `LIBRDKAFKA-HEADER-INLINES-RESIDUAL`. §6 gives the integrator the amendment
  text.
- **`rd_kafka_msgq_deq`, 5 pairs.** It hands the caller the message it
  unlinks from a borrowed queue. `resource_return_removed` needs an owned
  parent, and an alias owns nothing. This is the removal side of a derived
  container, so `W45-SUB-OBJECT-STORE-FORMAL-VALUE` is amended.
- **Other missing forms or body checks, 14 callees, 25 pairs.** These are
  recorded under `LIBRDKAFKA-HEADER-INLINES-RESIDUAL`:
  - `rd_kafka_topic_partition_get_toppar` returns a reference kept by a
    statement on a local assigned from another call. Rule 13d would withdraw
    it in every including unit.
  - `rd_avl_find` returns an element the tree indexes but does not own.
  - `rd_segment_write_remains` null-tests its out slot. Rule 13a admits no
    such mention.
  - `rd_kafka_q_last` and `rd_sockaddr_list_next` derive their results by a
    list walk or an array index. Rule 13a reads member paths only.
  - `rd_getenv` returns `getenv`'s storage or its argument.
  - `rd_file_mkstemp` returns a stream; its body leaks the descriptor when
    `fdopen` fails.
  - `rd_kafka_q_io_event` is the queue event-callback invocation itself. Its
    candidates need a C04.19 registry declaration.
  - `rd_tmpabuf_destroy`, `rd_avg_destroy` and `rd_avg_rollover` release or
    replace a member.
  - `rd_tmpabuf_write0` and `_write_str0` forward `rd_tmpabuf_alloc0`'s
    borrowed result. W4 may now infer that result (C04.3.4 rule 6).
  - `rd_kafka_broker_active_toppar_next` is a retention by the caller's
    broker, so it would be retention-undecided at every call.

## 5. Prediction and measurement

The [prediction](PREDICTION.md) was committed before implementation
(`2c44bffbb`):

- **Corpus:** six rows move Unknown -> Safe. They are the rows whose only
  diagnostics are calls to the 59 names.
- **System and slice:** no move. Neither selects a librdkafka unit.
- **The user's `current` run:**
  - 59 callees and 524 pairs leave the uncontracted list
    (2,790 -> 2,731 callees);
  - the "only uncontracted" set goes from 508 to 502 rows;
  - no unit changes status.

The integrator runs the measurement. Its command is the hand-back's.

## 6. For the integrator

- **Expected conflict.** `compiler/tests/contract_loader.rs` counts:
  - CFL-MPACK-EXTERNAL-DECLARATIONS also moves the v1 count.
  - This card adds one selected module, 59 v1 declarations and one corpus
    module.
- **After CFL-MPACK-EXTERNAL-DECLARATIONS lands**, amend
  `MPACK-STRUCT-RESULT-BY-VALUE` with this text: "AMENDED
  (LIBRDKAFKA-HEADER-INLINES): librdkafka's header definitions add five
  callees, 15 pairs on the user's current run at 046accf6:
  rd_kafka_idemp_get_pid0 7 (rd_kafka_pid_t), rd_kafka_replyq_make 3,
  rd_kafka_topic_partition_get_fetch_pos 1, rd_slice_pos 1 (struct
  results), and rd_kafka_offset_store0 3 (a struct parameter by value)."
  Then narrow `LIBRDKAFKA-HEADER-INLINES-RESIDUAL` accordingly.
- **Pre-existing failure.** `providers/tests/test_contract_load_path.py`
  (`test_the_real_workspace_manifest_is_current`) fails at this base and is
  unchanged by this card:
  - Its expected bounds views omit `qcore-provider-libevent`'s
    `monkey/result_range.qv`, which CELL-GUARD-RESULT-RANGE-CHECKED added.
  - No gate step runs `providers/tests`.

  The ledger registers it as `PROVIDER-TESTS-BOUNDS-VIEW-PIN-STALE`.

**Measurement (integrator, 2026-11-05, combined).** This card was merged by
the user together with CELL-PROTOCOL-RELEASE-WINDOW-FRAMES,
CFL-MPACK-EXTERNAL-DECLARATIONS, LIBC-GAP-DECLARATIONS,
LOG-PRINT-BODY-CONTRACTS, LIBRDKAFKA-HEADER-INLINES,
MEMBER-ACCESS-PROJECTION and CURRENT-RUN-DEFECTS (main `25508ccaf`, records
`8531b005b`, full gate passed), so one undeclared measurement covers them:
tip `rev:8531b005b` against `baseline:c8c7edf21`, output
`private/measure/integrate-r4-20261105T111129Z`, exit 0, 145 min. System
and slice: 0 moves. Corpus: 38 moves (Safe +37, Unsafe +1), 2 unit moves
(`ctr_attributes.c` -> Safe, `cmt_decode_statsd.c` -> provisional Unsafe), 0
unmatched. Every move declared by a merged card's PREDICTION.md happened. 29
moves are declared there; `_rd_strcasestr` is LIBC-GAP's recorded extra; 8
are cascades no single-card prediction counted, each with no leaf left at
the tip:
- `crc32c_init_hw` (crc32c.c): its only base leaves were the two calls to
  `crc32c_zeros`, which MEMBER-ACCESS-PROJECTION makes Safe (in-unit summary).
- `rd_kafka_mock_Produce_reply_tags_partition_write`,
  `rd_kafka_msgset_writer_write_MessageSet_v2_header`,
  `rd_kafka_msgset_writer_write_Produce_header`: base leaves were calls to
  the `rd_kafka_buf_write_*` writers this card declares, plus `unresolved
  resource identity` and member-store `indirect memory access` leaves on
  their results (`msetw->msetw_of_start = rd_kafka_buf_write_i64(rkbuf, 0)`,
  a scalar offset) that were artefacts of the undeclared call.
- `rd_kafka_msg_partitioner_consistent` (`rd_crc32_init`/`rd_crc32_update` in
  its return expression), `rd_kafka_outq_len` and `rd_kafka_queue_length`
  (`rd_kafka_q_len(rkqu->rkqu_q)` in a return): the return-expression and
  argument-member leaves were on calls this card declares borrowing.
- `rd_kafka_partition_leader_destroy_free(void *ptr)`: passes its formal (as
  `leader = ptr`) to `rd_kafka_partition_leader_destroy`, declared here
  `resource_released(leader, nonnull)`: a release of its own formal, as the
  base's `unresolved resource identity ptr` leaf could not read.
[integrate-r4-native-comparison.json](../librdkafka-header-inlines/integrate-r4-native-comparison.json)
(sha256 `01ab43b871fb84afdc0710b3326a00c5dc67ea785daad61d2a22f0d7519eb9f5`).

