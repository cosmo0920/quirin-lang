# NULL-WITNESS-GUARD-CORRELATION

Round 22, 2026-11-06. Integrator-found defect: `land.py publish` at
`855b34ade` measured the corpus null property's provisional `Unsafe` rows
17 -> 23 against baseline `c8c7edf21`. The user queued this card next (both
offered answers: "Queue NULL-WITNESS-GUARD-CORRELATION as the next card").
Base `13337d0af` (origin/main); `855b34ade..13337d0af` changes no analyzer,
contract or Lean file, so the base's null rows are the `855b34ade` landing
measurement's.

## 1. Diagnosis

**Inputs** (main checkout, read only): base
`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/eight-home/corpus/corpus-null.jsonl`
(SHA-256 `feafb9e8…40ed46`) and tip
`private/measure/landing-855b34adef125604a6a94535ef15b1dcd7791d0e-20261105T134026829943Z/sides/tip/corpus/corpus-null.jsonl`
(`10f005e3…d1bbec`). [`scripts/diagnose.py`](scripts/diagnose.py) lists every
check that is a finding at the tip and was not one at the base, with both
sides' sources: [`diagnosis.json`](diagnosis.json), **7 checks in 6 rows**, all
librdkafka, no other null verdict moved on any route (system and slice have no
lookup source). Every row's custody is `Unknown`; its combined verdict moved
`Unknown -> Unsafe`; its null contract dependencies are identical at both
sides.

**The change: LIBRDKAFKA-HEADER-INLINES (`68c8065e1`), not rule 38h or
MEMBER-ACCESS.** Its module `librdkafka/header_inlines.qv` declares
`rd_kafka_msgq_first`, `rd_kafka_msgq_last` and `rd_tmpabuf_alloc0` with
`ensures normal resource_return_borrowed_from(P, nullable);`. Before, each
call's result was `uncontracted-result:NAME`, a weak source the path search
(rule 14r) never runs on. A nullable borrowed-from result is
`ResourceReturnMode::Derived { nullable: true }`, which the walk reads as
`Source::Lookup` (`null_deref/walk.rs`, the call's result) and
`STRONG_SOURCE_KINDS` includes `lookup`, so each candidate became searchable.
The dependencies did not move because the walk and the search read the
resource contract through `contract_dependencies::unrecorded` (the
`inferred:` entries are the in-unit summaries of the same inline bodies,
present at both sides).

**The validator step.** In the search (`null_deref/witness.rs`), `call`
gives a lookup's result `P::Opaque(Origin::Lookup(NAME, LINE), false)` and
`judge` names "the lookup `NAME` at line L misses (returns NULL)" as the NULL
event whenever that value reaches the target dereference. Nothing reads
whether the miss is feasible on the path: a lookup's result is a function of
the state its arguments reach, and the path may have decided, written or
built that state. That is the step that should have refused each witness.
(An allocation's failure is different: C11 7.22.3 lets any allocation fail,
whatever the state.)

| Row (corpus, librdkafka) | Check | Witness path (decisions, NULL event) | What the route can read | NULL event feasible on the path? |
|---|---|---|---|---|
| `rd_kafka_msgq_allow_wakeup_at` | 15, `rdkafka_msg.c:1830` `rkm->rkm_u` | `RD_KAFKA_MSGQ_EMPTY(dest_rkmq)` false (`:1819`), lookup `rd_kafka_msgq_first(dest_rkmq)` (`:1826`) | `rd_kafka_msgq_first`'s body is `return TAILQ_FIRST(&rkmq->rkmq_msgs);`, the place `(&dest_rkmq->rkmq_msgs)->tqh_first` that the `:1819` test fixed non-null; no call or store between | **No**: excluded by the guard |
| `rd_kafka_ProduceRequest` | 16, `rdkafka_request.c:5099` | `rkbuf` non-null (`:5084`), `required_acks` false (`:5093`), lookup `rd_kafka_msgq_first(&rkbuf->rkbuf_batch.msgq)` | the queue belongs to `rkbuf`, a call's result (`rd_kafka_msgset_create_ProduceRequest`); its length is read at `:5087` (`rd_dassert(cnt > 0)`), and no body ties the count to `tqh_first` (a list invariant) | **Not readable**: state another function built, not an entry value; not validated |
| `rd_kafka_handle_idempotent_Produce_success` | 9, `rdkafka_request.c:4789` | no decision; lookup `rd_kafka_msgq_first(&batch->msgq)` | the body reads `batch->msgq.rkmq_msgs.tqh_first`, an entry memory value; rule 14r never names a memory value's null-ness as a NULL condition | **Not a NULL event** of the lookup's own: the queue's emptiness at entry is the function's precondition domain, like a formal |
| (same) | 11, `rdkafka_request.c:4790` | no decision; lookup `rd_kafka_msgq_last(&batch->msgq)` | `TAILQ_LAST`: a read through `tqh_last`; the same queue was just passed to `rd_kafka_msgq_first` | **Not a NULL event** of its own (as check 9) |
| `rd_kafka_metadata_cache_entry_new` | 27, `rdkafka_metadata_cache.c:308` | `include_racks` false (`:289`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:306`) | `tbuf` is a local arena the function initialised (`rd_tmpabuf_new(&tbuf, 0, rd_true)`) and sized (`rd_tmpabuf_add_alloc`) on the path. `rd_tmpabuf_alloc0` (`rdkafka_buf.h:104-126`) returns NULL only if `tab->failed` (no tmpabuf body sets it non-zero) or `tab->of + size > tab->size`, where `assert_on_fail` asserts first (the corpus compile database does not define `NDEBUG`) | **No in this build** (the size branch aborts; `failed` stays 0); with `NDEBUG`, only on mis-accounting. The size arithmetic (string lengths, loops) is not readable: refuse |
| `rd_kafka_topic_info_new_with_rack` | 20, `rdkafka_topic.c:2201` | `i < partition_cnt` false (`:2175`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:2200`) | as above (`rd_tmpabuf_new(&tbuf, 0, rd_true)`, accounting at `:2173-2191`) | as above |
| `rd_kafka_handle_Produce_metadata_update` | 19, `rdkafka_request.c:3900` | `leader_change_cnt` true (`:3876`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:3897`) | as above (accounting through three `rd_kafkap_leader_discovery_tmpabuf_add_alloc_*` calls) | as above |

So none of the seven is a validated reachable violation (AGENTS.md invariant
2): one is excluded by the guard, three rest on an arena the function itself
established, one on state a callee built, and two name an entry memory value
as a lookup's miss.
