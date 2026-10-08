# REFCOUNT-RESOURCE-KIND: prediction

Frozen before any implementation code or contract. Base: local `main` at
`9258289c9` (the brief's base exactly).

## BEFORE (reproduced)

Binary `private/rrk/bin/before`, sha256 `86110fdc…` (equal to `run.json`'s
`binary_sha256`):

| route | Safe / Unsafe / Unknown | Pending | digest |
| --- | --- | --- | --- |
| corpus `66910c10` | **872 / 9 / 3,953** | 0 | `52d14db1…` (byte for byte SLOT-MOVE-AND-INTERIOR-BORROW's AFTER) |
| system | 87 / 4 / 357 | 0 | `66091702…` (embeds this worktree's paths) |
| slice | 45 / 1 / 354 | 4 | `bf3a615c…` (embeds this worktree's paths) |

Ranking (`scripts/rank.py`): population **545**, **384** decided by one callee
alone (system 26 / 18, slice 38 / 27), as the previous card left it. Class 3
(`scripts/tally.py`, [tally-before.txt](artifacts/tally-before.txt)):
**9 callees / 19 functions alone**, the ledger's figure.

## Membership re-measured on main

`scripts/census.py` ([census-before-corpus.json](artifacts/census-before-corpus.json);
leaves naming the callee / Unknown functions carrying one / functions it
decides alone). System and slice: **no** member leaf (Fluent Bit's core does
not include librdkafka).

| group | member | leaves / functions / alone | decided alone |
| --- | --- | ---: | --- |
| constructor | `rd_kafka_queue_new0` | 6 / 6 / 3 | `rd_kafka_queue_get_consumer`, `_get_main`, `_get_sasl` |
| destructor | `rd_kafka_topic_partition_list_destroy` | 167 / 100 / 2 | `rd_kafka_assignment_destroy`, `rd_kafka_topic_partition_list_destroy_free` |
| destructor | `rd_kafka_AlterConsumerGroupOffsets_destroy` | 2 / 2 / 2 | `_destroy_array`, `_free` |
| destructor | `rd_kafka_DeleteConsumerGroupOffsets_destroy` | 2 / 2 / 2 | `_destroy_array`, `_free` |
| destructor | `rd_kafka_ListConsumerGroupOffsets_destroy` | 2 / 2 / 2 | `_destroy_array`, `_free` |
| destructor | `rd_kafka_topic_partition_destroy0` | 2 / 2 / 2 | `rd_kafka_topic_partition_destroy`, `_destroy_free` |
| destructor | `rd_kafka_topic_partition_result_destroy` | 2 / 2 / 2 | `_destroy_array`, `_free` |
| container | `rd_kafka_txn_clear_partitions_flag` | 3 / 2 / 2 | `rd_kafka_txn_clear_partitions`, `_clear_pending_partitions` |
| primitive | `rd_atomic32_add` | 10 / 10 / 2 | `rd_kafka_cert_dup` (a reference keep), `rd_kafka_idemp_inflight_toppar_add` (a plain counter) |
| primitive | `rd_atomic32_set` | 5 / 4 / 1 | `rd_kafka_bufq_reset` (plain) |
| primitive | `rd_atomic32_sub` | 6 / 6 / 1 | `rd_kafka_broker_persistent_connection_del` (plain) |
| keep | `rd_kafka_toppar_keep0` | 16 / 15 / 1 | `rd_kafka_toppar_list_copy` |
| keep | `rd_kafka_topic_keep` | 9 / 9 / 0 | |
| drop | `rd_kafka_toppar_destroy0` | 72 / 56 / 1 | `rd_kafka_msgbatch_destroy` |
| drop | `rd_kafka_q_destroy_owner` | 46 / 37 / 1 | `ut_destroy_mock_queue` |
| drop | `rd_kafka_q_destroy0` | 18 / 18 / 0 | |
| drop | `rd_kafka_topic_destroy0` | 21 / 15 / 0 | |
| drop | `rd_kafka_enq_once_trigger` | 6 / 6 / 0 | |

24 Unknown functions carry only member leaves (`by_set`); 249 carry a member
leaf beside other leaves (`mixed`). A lexical scan of every librdkafka `.c`
body calling a keep or drop wrapper finds 226 call sites, every one in an
`Unknown` function: no `Safe` row calls one today.

## Reading the members (both revisions, every body byte-identical)

* **Keeps** (`return` the argument after one increment through a member):
  `rd_kafka_toppar_keep0` (`rd_refcnt_add_fl` → `rd_atomic32_add(&rktp->rktp_refcnt, 1)`),
  `rd_kafka_q_keep` (`rkq->rkq_refcnt++` under `rkq_lock`, a plain `int`),
  `rd_kafka_q_keep_nolock`, `rd_kafka_topic_keep` (a lightweight-topic arm
  over the same object).
* **Drops** (one decrement through a member, the final destroy when it reaches
  zero): `rd_kafka_toppar_destroy0`, `rd_kafka_q_destroy0`,
  `rd_kafka_q_destroy_owner` (forwards to `destroy0`), `rd_kafka_topic_destroy0`,
  `rd_kafka_enq_once_trigger` (drops the source's reference and enqueues the
  op the eonce held; `err` an enumeration, rule 12u).
* **The hazard.** `rd_refcnt_add0` is a *macro* for `rd_atomic32_add` in the
  audited build (`HAVE_ATOMICS_32`), and `rd_refcnt_t` is a typedef of
  `rd_atomic32_t`; `rd_refcnt_sub0` is a function. The queue's count is a
  plain `int` (`rkq_refcnt`), so no single counter type covers every family.
* **Constructor.** `rd_kafka_queue_new0` stores `rkq` into the fresh
  `rkqu->rkqu_q` and then keeps it: the reference is handed into the owner it
  returns (`rd_calloc` asserts, so the result is non-null).
* **Destructors.** `rd_kafka_topic_partition_list_destroy` (each element's
  in-place destroy, then the array and the list), the three
  `*ConsumerGroupOffsets_destroy` (the list, then the object),
  `rd_kafka_topic_partition_result_destroy` (the partition, the error, the
  object), `rd_kafka_topic_partition_destroy0` (static; frees the object only
  when `do_free`, every caller that frees passes `1` or `rd_true`, which is
  `1`).
* **Not a destructor.** `rd_kafka_txn_clear_partitions_flag` drops the
  references a caller's list holds and re-initialises the list: a release of a
  set reachable from a parameter, which C04.13 records v1 does not state. Not
  declared.

## Design (frozen)

* **The reference kind** (`ResourceKind::Reference`, contract spec C04.17,
  grammar G05, gateway rule 13b). One reference this function holds on an
  object. Two withheld clauses on a `resource_borrowed` depth-one pointer
  parameter: `resource_reference_kept(p)` (+1, result `resource_return_alias(p,
  nonnull)` or `resource_return_none()`) and `resource_reference_dropped(p)`
  (-1, the last drop may free the object). Both are withheld from the function
  map with the retaining declarations, so every consumer that knows nothing of
  references sees an uncontracted callee: the custody-neutral reading is never
  admitted, and a withdrawn declaration keeps refusing rather than letting an
  inference read the wrapper. `reference` is not spelled through
  `resource_kind`/`resource_return_kind` (the module is rejected).
* **Route G.** A keep of a name bound to no tracked identity (an unrebound
  formal or an untracked local, possibly through a pointer cast) mints one
  *family* for the object, held 1, bound to that name and the result; a keep
  of a name bound to a live family adds 1. A drop of a live family subtracts
  1; at 0 the drop is possibly final: the family is discharged and every name
  bound to it is invalidated (a later read refuses, `reference-dropped`, never
  accused). A drop the body holds no reference for (an unbound formal, a
  member read, an invalidated name) refuses (`reference-drop-unheld`):
  consuming a caller's reference needs a declaration. Hand-offs: a `return`
  of the family, a store into a tracked owner (field link, escaping with its
  owner). At an exit a family still held, or held more than once when handed
  off, is a **leak candidate** on a path with no refusal, and a refusal
  (`reference-unbalanced`) on a path that has one. Joins: equal counts keep,
  different counts make the family undecided (every later operation
  refuses). A heap release of a family is a kind mismatch.
* **Plain counters (the hazard resolved at the wrapper level).** No
  `rd_atomic32_*` primitive is declared: a reference operation is read only
  through a declared wrapper whose body names the object, so a statistics
  counter on the same primitive is never read as a reference, and a reference
  operation spelled through the primitive is never read as anything (it stays
  `call to uncontracted function`). A plain-counter declaration typed by the
  operand's Clang type is a follow-up.
* **Body checks (rule 13c).** In a unit that defines a keep or drop
  declaration's body, it stands only when the body never assigns `p` nor takes
  `&p`, never stores `p` into anything, and returns exactly `p` (alias keep),
  nothing (void keep, drop); otherwise every call of it there refuses
  (`reference-withdrawn`).
* **Plain declarations** (existing vocabulary, justified by the decision's
  hand-off reading): `rd_kafka_queue_new0` (owned non-null result, both
  parameters borrowed: the reference it takes is the result's), the five
  destructors `resource_released(·, nonnull)`, and
  `rd_kafka_topic_partition_destroy0` under the literal premise `do_free = 1`.

## Predicted outcome

| item | predicted |
| --- | --- |
| `rd_kafka_queue_new0` callers | +3 (whole-call returns over member actuals at borrowed positions) |
| destructor `_free` wrappers (Alter, Delete, List, result, list_destroy_free) | +5 |
| `rd_kafka_topic_partition_destroy`, `_destroy_free` | +2 |
| `_destroy_array` loops (Alter, Delete, List, result) | 0 to +4 (element actuals at a non-null release) |
| `rd_kafka_assignment_destroy` | 0 (four member actuals at a non-null release) |
| route: `rd_kafka_toppar_list_copy` (a keep returned) | +1 |
| route: `rd_kafka_msgbatch_destroy`, `ut_destroy_mock_queue` | 0 (drops of an unheld reference refuse) |
| cascade through new summaries | 0 to +3 |
| **corpus total** | **+11 to +18 (central +13)** |
| primitives (`cert_dup`, `idemp_inflight_toppar_add`, `bufq_reset`, `persistent_connection_del`) | 0 |
| Safe → non-Safe | none (no Safe row calls a member) |
| new leak candidates | 0 to 3, each read by hand |
| re-attributed Unknown functions | 150 to 260 (the mixed callers trade `call to uncontracted function` for a route outcome) |
| system / slice | no row moves |
| class 3 after | 9 / 19 → 2 to 4 callees / 3 to 8 functions alone (the primitives, the container clear) |
