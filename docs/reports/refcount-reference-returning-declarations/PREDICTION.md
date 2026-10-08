# REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: prediction

Frozen before any implementation code, contract or Lean. Base: local `main` at
`e27f98d5b` (REFCOUNT-RESOURCE-KIND's gate records).

## BEFORE, reproduced

Binary `private/rrrd/bin/before` (`5c324125…`, equal to the corpus
`run.json`'s `binary_sha256`), built from `e27f98d5b`:

| route | totals | digest |
| --- | --- | --- |
| corpus | **884 / 9 / 3,941** | `7a4595d6…` (the brief's) |
| system | 87 / 4 / 357 | `b5118a80…` (embeds this worktree's paths) |
| slice | 45 / 1 / 354, 4 Pending | `32c1b2cc…` (embeds this worktree's paths) |

`scripts/refusals.py` reproduces REFCOUNT-RESOURCE-KIND's census exactly
([refusals-before-corpus.json](artifacts/refusals-before-corpus.json)):
`reference-drop-unheld` through a plain name **131 leaves / 91 functions / 1
alone**; member 3 / 3 / 1; address 1; unread 3; `reference-unbalanced` 5;
`reference-undecided` 7. `scripts/rank.py`: 522 Unknown functions blocked by
uncontracted calls alone, 365 by one callee alone (the ledger's figures).

## Where the 131 references come from

`scripts/origins.py` ([origins-before-corpus.json](artifacts/origins-before-corpus.json))
reads, lexically, the last assignment of each dropped name before the drop:

| origin | leaves | functions | alone |
| --- | ---: | ---: | ---: |
| `rd_kafka_q_new` (`rd_kafka_q_new0`) | 29 | 24 | 0 |
| `rd_kafka_toppar_get` (`rd_kafka_toppar_get0`) | 15 | 11 | 0 |
| `ut_create_mock_toppar` | 9 | 9 | 0 |
| `rd_kafka_topic_partition_get_toppar` | 8 | 7 | 0 |
| `rd_kafka_q_fwd_get` | 7 | 7 | 0 |
| `rd_kafka_toppar_get2` | 5 | 5 | 0 |
| `rd_kafka_topic_proper` | 4 | 2 | 0 |
| `rd_kafka_toppar_desired_get`, `_desired_add`, `_get_avail` | 3, 3, 2 | 3, 1, 1 | 0 |
| topic lookups (`rd_kafka_topic_find*`, `_find_by_topic_id`, `_new0`, `_new_with_id`) | 10 | 9 | 0 |
| list removals (`rd_list_remove`, `_pop`, `_elem`, `FOREACH`) | 9 | 8 | 0 |
| a formal (the caller's reference) | 4 | 4 | 1 (`ut_destroy_mock_queue`) |
| other (an op's or a topic's member, a callback's opaque, a cast) | 26 | 18 | 0 |

**Ranking (task 1).** No function is cleared by one declaration: the declared
origins below touch **66 leaves in 43 functions**, and every one of the 43
carries another leaf family (42 an uncontracted call: `rd_kafka_op_destroy`
16, `rd_kafka_q_pop` 13, `rd_kafka_share_msgset_parse` 10, the unit-test
helpers 9 each, `rd_kafka_buf_destroy_final` 9, …; 38 the access gate, 33 an
unresolved identity, 28 a branch condition). In the uncontracted-call
population (522 / 365) none of the candidates decides a function alone
(`rd_kafka_toppar_new0` and `rd_kafka_topic_proper` 1 each with exactly one
other callee). So the declarations **re-attribute**; they are not predicted to
free a function.

## What will be declared

Every body read at `66910c10` and `046accf6` (byte-identity checked by
`scripts/body.py` before pinning). Placement: `qcore-provider-librdkafka`
`librdkafka/client.qv`.

* **Returned references** (`resource_return_owned(n)` +
  `resource_return_kind(reference)`): `rd_kafka_toppar_get0` (nullable: `NULL`
  or `rd_kafka_toppar_keep_fl`), `rd_kafka_toppar_get2` and
  `rd_kafka_toppar_get_by_id2` (nullable: `NULL` or `_desired_add`'s),
  `rd_kafka_toppar_desired_get` (nullable: a keep in the loop or `NULL`),
  `rd_kafka_toppar_desired_add` (declared nullable: its sources are the
  nullable lookups and `rd_kafka_toppar_new0`; the structural check cannot see
  that the last is non-null), `rd_kafka_toppar_get_avail` (nullable),
  `rd_kafka_toppar_new0` (non-null: `return rd_kafka_toppar_keep(rktp)` over a
  count initialised to 0), `ut_create_mock_toppar` (nullable), and the
  constructors `rd_kafka_q_new0` (non-null: `rd_malloc` asserts, the count is
  initialised to 1 by `rd_kafka_q_init0`) and `ut_create_mock_queue`
  (non-null, `rd_calloc` then `rd_kafka_q_init`).
* **A drop:** `ut_destroy_mock_queue` (`resource_reference_dropped(rkq)`: a
  null test, then `rd_kafka_q_destroy_owner(rkq)`; rule 13c validates it as a
  forward to a declared drop at the same position). This is the "transfer"
  the ledger asks about, seen from the caller: the caller's reference leaves
  its family, and a later drop of an acquired family is an over-drop. No new
  clause is needed by any caller; the callee's own row reading its parameter
  as the reference it took over is **not** built (1 function, registered).
* **Result holder:** `resource_retained_by(P, result)`. `rd_kafka_q_new0`
  retains `rk` (`rkq->rkq_rk`) and `func` (`rkq->rkq_name = func` in the
  audited build: `ENABLE_DEVEL:BOOL=OFF`); `rd_kafka_queue_new0` is
  re-declared retaining `rk` (`rkqu->rkqu_rk = rk`) by its result. Rule 12t's
  outlives decision: the result is a tracked holder; a caller's object
  (`rk`, an unrebound formal) is recorded as held by it, discharged when the
  handle is released or the function drops its last reference on the queue;
  a holder still holding at an exit (returned) refuses (`retention-undecided`);
  a retained actual the route cannot name (`rkcg->rkcg_rk`) refuses at the
  call. `func` is `__FUNCTION__`, a predefined identifier of static storage
  (C11 6.4.2.2p1), read as static like a string literal.
* **Body check (new rule):** in a unit that defines a returned-reference
  declaration's body, it stands only when every `return` is a null constant
  (declared nullable), a call of a declared keep or returned reference, or a
  local whose every assignment is one of those or a declared heap allocation,
  never address-taken, never at a declared release, drop or consumption, and
  the body has no `goto`; otherwise every call there refuses
  (`reference-withdrawn`) and no inference reads the body in its place.

Not declared (bodies the check cannot validate, or not a reference on every
path), registered with sizes: `rd_kafka_q_fwd_get` and
`rd_kafka_topic_partition_get_toppar` (a conditional keep of a member or an
undeclared lookup), the topic lookups (a list walk kept at `break`),
`rd_kafka_topic_proper` (returns its argument on one path, a new topic on the
other), `rd_kafka_topic_new0`/`_new_with_id`.

## Predicted outcome

| item | predicted |
| --- | --- |
| Unknown → Safe | **0** (range 0 to 2, a cascade only) |
| Safe → non-Safe | **4**, each a named correction of REFCOUNT-RESOURCE-KIND: `rd_kafka_queue_get_main`, `_get_consumer`, `rd_kafka_queue_get_sasl` (each returns a handle that retains its caller's `rk`, now `retention-undecided`), and `rd_kafka_share_queue_get_sasl` (its summary over `_get_sasl` goes) |
| corpus | **880 / 9 / 3,945** (range 880–882 Safe; Unsafe 9 to 12) |
| new leak or over-drop candidates | 0 to 3, each read by hand; provisional |
| `reference-drop-unheld` through a plain name | 131 → about 65 (the 66 declared-origin leaves leave; some return as `retention-undecided`, `reference-unbalanced` or another family) |
| `call to uncontracted function` on the declared callees | 25 functions (`rd_kafka_q_new0`), 10 (`_get0`), 9 (`ut_create_mock_toppar`), 4 (`_get2`), 3 (`_desired_get`) lose that leaf |
| re-attributed Unknown functions | 40 to 80 |
| system / slice | no row moves (no member is called there) |
| uncontracted-call population | 522 / 365 → about 520 / 365 |
