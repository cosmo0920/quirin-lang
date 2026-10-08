# REFCOUNT-FAMILY-BLOCKING-CALLEES: the callees that break reference families

Card REFCOUNT-FAMILY-BLOCKING-CALLEES (wave 48, card-high), on the user's
request of 2026-10-09 ("we need to fill the lack of contracts"). Base: local
`main` at `652dc085f`, which it stayed at. [PREDICTION.md](PREDICTION.md) was
committed at `e76ff4fc5`, before any contract.

**Published property and scope.** Custody (`c.custody`) on the pinned corpus,
the system route and the tail-pause slice, as before. The card adds four
contracts and no semantics: no gateway rule (13f–13j unused), no Lean.

## 1. BEFORE and the census

Reproduced (binary `private/rfbc/bin/before`, `24b13b40…`, equal to
`run.json`): corpus **880 / 9 / 3,945** (`79dc429d…`, the brief's), system
87 / 4 / 357, slice 45 / 1 / 354 with 4 Pending. `reference-drop-unheld`
through a plain name **127 leaves / 89 functions**, `reference-unbalanced` 11 /
10 ([refusals-before-corpus.json](artifacts/refusals-before-corpus.json)).
System and slice carry no `reference-` leaf. Uncontracted-call population
523 / 367 (the ledger's).

[`scripts/withdrawers.py`](scripts/withdrawers.py) names, for each plain-name
unheld drop, the family's origin and every refused statement between the
acquisition (inclusive) and the drop that hands the name on as rule 13b's
refused hand-on rule reads it; [`scripts/family_closure.py`](scripts/family_closure.py)
reads them as sets. Lexical and approximate; the arbiter decides. Of the 127,
65 have an origin no declaration covers (35 undeclared calls, 26 member reads
or list walks, 4 formals: the previous card's classes), so no family formed.
The other **62** families were formed by a declared call and then withdrawn.
Their first withdrawer ([withdrawers-before.txt](artifacts/withdrawers-before.txt)):

| first withdrawer | families |
| --- | ---: |
| an uncontracted callee (12 callees, §2) | **17** |
| access gate at the acquiring `rd_kafka_q_new(X->f)`: member actual at a withheld declaration's retained `rk` | 13 |
| unresolved identity at the acquiring `ut_create_mock_toppar(rkshare->…)` | 9 |
| unresolved identity in the null test of a name two acquisitions join (`rktp = get(…); if (!rktp) rktp = desired_get(…); if (unlikely(!rktp))`) | 5 |
| access gate, `mtx_lock(&(rktp)->rktp_lock)` (the `rd_kafka_toppar_lock` / `rd_kafka_topic_*lock` macros) | 4 |
| `retention-undecided` at the acquiring `rd_kafka_q_new0` | 3 |
| loop re-acquisition | 3 |
| unresolved identity at a `rd_kafka_toppar_get2` statement | 2 |
| branch condition `if ((err = rd_kafka_toppar_op_seek(…)))` | 2 |
| braced initializer, access at `rd_atomic64_add`, two unresolved identities | 4 |

The lock macros' access refusal is the most frequent withdrawer overall (91
appearances): a family whose callee is contracted is withdrawn again at its
next lock line. The "lock wrappers" the brief names are macros over the
already-declared `mtx_lock` and `pthread_rwlock_*`; the refusal is the access
gate's residual address class (`W47-ACCESS-PATH-POSITION-RESIDUALS`), not a
missing contract.

## 2. The ranking, each body read

(i) families broken first; (ii) functions the reference route would decide if
the callee alone were contracted; (iii) of those, functions whose every other
leaf names the same callee (the CG10-residual check). Bodies are
byte-identical at `66910c10` and `046accf6` ([`scripts/body.py`](scripts/body.py)).

| callee | (i) | (ii) | (iii) | why not declared |
| --- | ---: | ---: | ---: | --- |
| `rd_kafka_toppar_op_seek` | 3 | 0 | 0 | `rd_kafka_fetch_pos_t` and `rd_kafka_replyq_t` by value; keeps `rktp` into its op |
| `rd_kafka_ListOffsetsRequest` | 2 | 0 | 0 | `replyq` by value |
| `rd_kafka_consumer_close_q` | 2 | 0 | 0 | keeps `rkq` into two forwarding queues and the terminate op |
| `rd_kafka_share_fetch_reply_handle_partition_error` | 2 | 0 | 0 | keeps `rktp` into an error op (`rd_kafka_consumer_err`) |
| `rd_kafka_replyq_make` | 1 | 0 | 0 | returns `rd_kafka_replyq_t` by value, holding a kept queue reference |
| `rd_kafka_op_set_replyq` | 1 | 0 | 0 | keeps `rkq` into `rko->rko_replyq` |
| `rd_kafka_broker_purge_queues` | 1 | 0 | 0 | `replyq` by value |
| `rd_kafka_consume_stop0` | 1 | 1 | 0 | drops the desired list's reference, keeps `rktp` into an op |
| `rd_kafka_offset_store0` | 1 | 1 | 0 | `pos` by value |
| `rd_kafka_toppar_broker` | 1 | 1 | 0 | a conditional keep of a member broker, dropped through the `rd_refcnt` primitive (rule 13d's shape) |
| `rd_kafka_toppar_op_pause_resume` | 1 | 0 | 0 | `replyq` by value; keeps `rktp` |
| `rd_kafka_toppar_pid_change` | 1 | 0 | 0 | `rd_kafka_pid_t` by value |

**None of the twelve is declarable with the vocabulary on main.** Seven keep a
reference on the object they are handed into an owner they build or reach
(an op, a reply queue, a forwarding queue, the desired list): the brief forbids
a plain borrowed reading of such a member, and C04.17 has no clause for a
reference kept into an owner (a reference clause cannot stand beside a
retention of the same parameter). Six take or return a C structure by value,
which the declared-ABI reader does not project, and `rd_kafka_replyq_t` also
carries a kept reference the callee takes over. One is rule 13d's
conditional-keep shape. The heads of the ranking are therefore decisions and
reader work, recorded in §6, and the stopping rule (yield per contract about
one function) holds from the head.

## 3. Contracts

Declared: the members of the 62 families' withdrawer sets that the vocabulary
states exactly (bodies read at both revisions, pinned with `quirin-audit pin`
at both, logs `private/rfbc/pin-a1/`, `pin-a2/`), appended to
`qcore-provider-librdkafka` `librdkafka/client.qv`, in the function map:

| declaration | body | in families |
| --- | --- | ---: |
| `rd_kafka_q_wait_result(rkq, timeout_ms) -> i32` | pops one op and destroys it; returns its `rko_err` or `__TIMED_OUT` | 5 |
| `rd_kafka_q_purge0(rkq, do_lock) -> i32` | destroys the queue's ops, or purges the forward queue (kept and dropped: another object) | 1 |
| `rd_kafka_toppar_needs_query(rk, rktp) -> ptr<const, u8>` | reads; returns a string literal or NULL (`return_none`, as `gai_strerror`) | 1 |
| `rd_kafka_toppar_fetch_backoff(rkb, rktp, err) -> i64` | writes one scalar member, logs, returns a time | 2 |

Every parameter is `resource_borrowed`; the enumeration `rd_kafka_resp_err_t`
is `i32` (rule 12u). None keeps, drops, retains or moves a reference on the
object it is handed. The module states the **served-queue premise**
(constraint 4): `rd_kafka_q_wait_result` (through `rd_kafka_q_pop`) and
`rd_kafka_q_purge0` may run a popped op's own callback, which receives the
served queue and is assumed to take over no reference the caller holds on it;
the callbacks are not modelled (`CG22-CALLBACK-REGISTRY-CONTRACT`), and
`rd_kafka_q_serve*` / `rd_kafka_q_pop_serve*`, which take a callback, stay
undeclared.

**Withdrawn after the first AFTER: `rd_kafka_q_pop`.** Its body states
`resource_borrowed(rkq)` and an owned nullable op. Declared (run `a1`, binary
`d59b325d…`, corpus 881 / **19** / 3,934, [compare-a1-corpus.json](artifacts/compare-a1-corpus.json)),
it moved ten Unknown rows to Unsafe and repaired no family. Read by hand:
`rd_kafka_metadata`'s leak (`rdkafka_metadata.c:155` → `:111`) is on an
**infeasible path**: the route takes `if (err)`'s `err == 0` edge, skipping
`rd_kafka_op_destroy(rko)`, and then the do-while's `err ==
RD_KAFKA_RESP_ERR__DESTROY_BROKER` edge back to the loop head; the two tests
of `err` are not correlated. The nine `rdunittest_msgset_errors.c` tests leak
the popped op on the arm where an `RD_UT_ASSERT` on its members fails
(`return 1`): reachable only on a failing test, provisional, never an
upstream defect. Publishing a known-infeasible Unsafe for no repaired family
is not a trade the card makes; the declaration was removed before `a2`
(`REFCOUNT-Q-POP-OWNED-OP`).

Pins: `pin --check` of the crate 0 at both revisions; the whole-tree check 1
at both, as on the base (its one row the libevent crate's tip-mode
`mk_vhost_set`); `--verify-contracts` 0; `providers/contracts.py sync` and
`check` 0. Deliberate pin updates: `compiler/tests/contract_loader.rs` v1
declarations 359 → 363 (retaining unchanged at 28);
`compiler/tests/qcore_contract_corpus.rs` four clause rows.

## 4. AFTER, attributed

Binary `private/rfbc/bin/a2` (`f0646e94…`, equal to `run.json`), built from
`e6a8cce2b`.

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus | 880 / 9 / 3,945 | **881 / 9 / 3,944** | `79dc429d…` → `36fd1649…` |
| system | 87 / 4 / 357 | unchanged, no row moves | `2ad95080…` → `013bf83d…` (both embed run paths) |
| slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `ca70a3e4…` → `7f3e8305…` |

**+1 Unknown → Safe:** `rd_kafka_toppar_purge_and_disable_queues`
(`rdkafka_partition.c:1158`), whose two `rd_kafka_q_purge` calls were its only
leaves ([compare-a2-corpus.json](artifacts/compare-a2-corpus.json)). **No Safe
→ non-Safe move; Unsafe 9 → 9; no leak or violation added.** 20 more Unknown
rows changed leaves only, each attributed to a declared callee:
`rd_kafka_q_purge0` 10 (six of them now report the access gate's unresolved
identity on the member actual, `rk->rk_rep` and kin, behind the admitted
call), `rd_kafka_q_wait_result` 5, `rd_kafka_toppar_fetch_backoff` 4,
`rd_kafka_toppar_needs_query` 1. Removed: 24 `call to uncontracted function`
leaves, 3 unresolved identities, and 2 `reference-dropped` leaves in
`rd_kafka_seek` (`rdkafka.c:4766`, reported by the refused `wait_result`
statement's line reading names invalidated earlier). Added: 6 unresolved
identities.

**Reference leaves remaining** ([refusals-a2-corpus.json](artifacts/refusals-a2-corpus.json)):
plain-name `reference-drop-unheld` **127 / 89, unchanged**, as predicted: no
family is repaired, because every family that needed a declared callee also
needs an undeclarable one or a non-callee breaker
([closure-a2.txt](artifacts/closure-a2.txt): still 12 families whose every
withdrawer is an uncontracted callee, now without the four declared ones).
`reference-unbalanced` 11 / 10 unchanged (the same functions, or keep bodies
behind access-gate refusals), `reference-dropped` 20 → 18, the rest unchanged.
Why each remains: §1's table and §2.

**Parity, compared in full** ([corpus](artifacts/parity-a2-corpus.json),
[system](artifacts/parity-a2-system.json), [slice](artifacts/parity-a2-slice.json)):
system (139,962 rows) and slice (50,407) multisets identical; corpus 662,742 →
662,667 rows, 24 functions differ: the 21 moving rows, the declared bodies'
own census rows (`rd_kafka_toppar_fetch_backoff` 4, `rd_kafka_q_wait_result`
1, bound-untracked-root rows), and one unit's derivation call graph (a
declared callee leaves it).

Uncontracted-call population ([rank-a2-corpus.json](artifacts/rank-a2-corpus.json),
[tally](artifacts/tally-a2.txt)): **522 / 366**; system 26 / 18, slice 38 / 27
unchanged.

**`mk_vhost_set` at the tip** ([rows](artifacts/tip-rows-after.txt)):
byte-identical to the previous card's record.

## 5. Prediction against outcome

| item | predicted | measured |
| --- | --- | --- |
| Unknown → Safe | 1 (0 to 3) | **1**, the one named |
| Safe → non-Safe | 0 | **0** |
| corpus | 881 / 9 / 3,944 | **881 / 9 / 3,944** |
| system / slice | no row moves | **no row moves** |
| plain-name unheld drops | 127 (125 to 127) | **127** |
| `reference-unbalanced` | 11 | **11** |
| re-attributed Unknown functions | 30 to 60 | **20** (`rd_kafka_q_pop` withdrawn; it accounted for most of the predicted range) |
| new leak candidates | 0 to 2 over the popped op | **10 with `q_pop` declared** (a1), one on an infeasible path; **0** after its withdrawal |

The deviation is `rd_kafka_q_pop`: predicted at most two leak candidates, it
produced ten, which is why it is not declared.

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py) (follow-ups 728 →
731). The card had no entry of its own; nothing is retired. Amended:
`W48-CALLEE-DECISION-TABLE` (class 3 gains the kept-into-an-owner shape, 13
callees, 6 families broken first, 1 function decidable, 0 Safe),
`W45-CALLEE-CASCADE-RESIDUAL` (523 / 367 → 522 / 366),
`REFCOUNT-REFERENCE-RESULT-BODY-SHAPES` (`rd_kafka_toppar_broker`),
`RETAINING-CALL-MEMBER-ACTUAL` (13 families broken at their acquiring
`rd_kafka_q_new(X->f)`, plus 3 at its retention), and
`CG22-CALLBACK-REGISTRY-CONTRACT` (the served-queue premise). Registered:
`REFCOUNT-BY-VALUE-AGGREGATE-ABI` (10 callees, 10 families broken first, 1
function decidable), `REFCOUNT-Q-POP-OWNED-OP` (10 Unknown → Unsafe when
declared, one infeasible; 15 families, never first), and
`REFCOUNT-FAMILY-NON-CALLEE-BREAKERS` (§1's table).

## 7. Lean

None: four contracts in existing vocabulary, no semantics added. They are
checked by the existing loader, the pins and the ABI reader.

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/).

## Files

* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`.
* Pins: `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`.
* Records: `docs/STATUS.md`, `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, plus
  `withdrawers.py` and `family_closure.py`.
