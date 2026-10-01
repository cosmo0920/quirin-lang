# CG05-ELABORATOR-CALL-FOOTPRINTS: frozen prediction

Frozen before the implementation commit and before any AFTER run, on the
BEFORE of base `d4df096f` (local `main`, round 7 complete). No Rust test, no
fixture probe and no corpus run of the new route had been executed when this
file was committed. CG00's `predictions/CG05.md` was frozen for the
CG05-NESTED-CALL-EFFECTS card, which has landed; its function list does not
apply to these two follow-ups, so this prediction is derived afresh, from this
base, with [scripts/predict.py](scripts/predict.py) (amendment: reasons below).

## BEFORE (reproduces the baseline)

| Route | Totals | `resource-diagnostics.json` |
|---|---|---|
| corpus `66910c10` (`--jobs 4`) | **628 / 10 / 4,196** | `3b09309a…` |
| system | **44 / 2 / 188** | `3e88dd81…` |
| tail-pause slice | **29 / 1 / 239 / 1 Pending** | `8fbf8ffe…` |

The corpus digest is not CG12's integration digest `394b190b…`: CG18 landed
after it and changed the leaves of two functions without moving a status (its
integration record says so). The status totals are the baseline's. Binary
`8bed2ee9…`, built at `d4df096f`.

## The populations (de-duplicated parity rows, [scripts/census.py](scripts/census.py))

**CG05-ELABORATOR-CALL-FOOTPRINTS.** The ledger's 98 consultations over 33
functions do **not** reproduce at this base. The refusal `call … unsequenced
with another event (CG05)` is now **621 rows over 411 functions** (619
statements, 2 returns), by shape ([scripts/families.py](scripts/families.py)):

| Shape | Rows | Functions (all rows of the shape) |
|---|---:|---:|
| compound assignment (the ledger's family, `fails += ut_x()`) | 118 | 39 |
| store `NAME = CALL(…)` / path store | 276 | 211 |
| call statement | 170 | — |
| declaration | 52 | 51 |
| other | 5 | 4 |

The compound family grew from 98/33; the other 503 rows carry the same refusal
text because later cards (CG08's rule 11b consults every statement holding a
cast) route more statements to the elaborator, whose lowering refuses every
call outside a sequenced piece with that same refusal. This card's lowering
answers every call event of a refused piece, so the whole 621 is its
population.

Of the 646 population rows (621 + the 25 below), **514** have the leaf `call
to uncontracted function X` at their own line for their own callee: no
contract is in force, so no footprint can be read and they **stay refused by
design**. At most **132** rows can change.

**CG05-FOOTPRINT-ADDRESS-CONSERVATISM.** Reproduced exactly: **25** nested-call
statement rows refused as `footprints do not commute`, over **14** functions:
24 `fprintf(stderr, …, f(&local))` (`rd_kafka_msgq_len` 12, `rd_list_cnt` 7,
`rd_buf_len` 3, `rd_kafka_msgq_size` 2) in 13 librdkafka unit-test functions,
and `strftime(time_string, …, localtime_r(&utime, &result))` in
`mk_clock_log_set_time`.

## Decision for the address part (stated before implementing)

Option **(c), a measured boundary**. (b) is not sound from the flow: the
contract spec says `resource_borrowed` "is not the full QCore lifetime-checked
borrow type", so a callee that receives `&local` at a `Borrowed` formal may
retain it, and no route has an escape analysis for an automatic's address into
a file-scope object; the identity a file-scope pointer denotes is unknown to
the flow, so it may be the local. (a) is a new program premise that every
using verdict would have to name; **no function would move** with it (each of
the 14 has other blocking leaves, by `predict.py`). The Lean evidence states
why the refusal is necessary (an address-taken object an identity may reach,
and two orders that differ).

## Predicted outcome

1. **Corpus Safe moves: 0** (upper bound **3**). `predict.py` finds 53
   functions freed alone if every population row on their lines were admitted
   (46 with no alias leaf). By reading each one's population leaves:
   - 50 have only `call to uncontracted function X` there (their callees: unit-test
     helpers, `cfl_kvlist_fetch`, `add`, `pthread_*`, `dlclose`, the
     `*_destroy` free callbacks …): they stay Unknown.
   - `rd_kafka_broker_ApiVersion_at_least` and `…_no_lock` are `return` rows:
     this card lowers calls in statements only, so they stay.
   - `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic`: `(tbuf)->size +=
     (strlen(topic_name) + …)` is a destination read through a pointer
     unordered with a call reading an identity; the footprints never prove two
     identities distinct, so it stays refused.
   The upper bound counts these three in case the reading is wrong.
2. **No Safe → non-Safe move and no Unknown → Unsafe move.** A call is applied
   with use `Tested`, which accepts no owned result, so no new leak can be
   reported from these statements; a released actual is applied exactly once.
   A violation could appear only where a body releases one identity twice or
   uses it after release, which the flow already checks; each such leaf would
   be classified by reading.
3. **Leaves.** At most 132 rows change: each admitted row removes its older
   leaf; a row may exchange its leaf for a later refusal of the same statement
   (a footprint conflict, the application point's decline, a path the typed
   reader refuses). Functions that change diagnostics: at most the 423 touched.
4. **The address population stays 25 / 14**, byte for byte in its parity rows.
5. **The `aliasing` correction** (an array decay or a member/subscript address
   now takes its root object's address): predicted to move no function; any
   CG05 nested admission it withdraws is a named correction.
6. **System and slice:** no status move; leaf changes possible only where a
   system function has a contracted call in a refused compound or store.
   Sidecars byte-identical.
