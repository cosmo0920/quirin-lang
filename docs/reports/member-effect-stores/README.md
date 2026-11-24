# MEMBER-EFFECT-STORES, body half: re-census and stop

Round 35, 2026-11-24. **USER DECISION** (verbatim): "Launch MEMBER-EFFECT-STORES
body half", answering K3-POINTER-VALUED-STORES' option 1
([K3 §6](../k3-pointer-valued-stores/README.md#6-options-for-the-rest-of-the-bundle-hand-back)).
Base `c5cd10688` (integrate/r34-k-batch). **The card stops at its re-census**:
under the brief's premises at most 4 of the 21 rows can be read soundly, below
the brief's stop threshold of about 5. No semantic change, no rule id, no Lean
module; options in section 3.

## 1. Re-census on the 2026-11-24 `current` report

[scripts/recensus.py](scripts/recensus.py) reads every diagnostic of the 21 rows
from the user's report (`build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`,
1,173 Safe / 3,750 Unknown / 11 Unsafe):
[evidence/rows-1124.json](evidence/rows-1124.json). All 21 are `Unknown` and
every diagnostic of every row is `indirect memory access is outside the resource
projection`, the gate's text: rule 38i's name does not apply because each
store's value reads memory (`rd_malloc(rbuf->rbuf_extra_size)`), is a local
acquisition, a cast, or an embedded path. `rd_kafka_assignment_init` carries
four leaves (its four stores), each other row one. The Monkey rows are
`mk_server/mk_utils.c::mk_buffer_cat` and `mk_server/mk_lib.c::mk_vhost_set_property`
(K3 called them unmapped). None changed blocker since K3's census of the same
report; K1, K5 and K7 on this base touch none of them (each row's one leaf is
the store).

Bodies read at Fluent Bit `046accf6` (the run's prepared tree). Brief premises:
**H1** the place is used only at the store; **H2** a local value is never used,
released, rebound or stored again after it; **H3** no failure path stores a
dangling or stale value; **H4** the old value of `P->F` is established
custody-free in the body, or is the realloc input under a non-null clause; plus
the near-misses (a local alias of the formal, a loop, a rebound formal).

| Row | Store | Fails | Reason |
|---|---|---|---|
| `rd_buf_init` (`rdbuf.c:351`) | `rbuf->rbuf_extra = rd_malloc(rbuf->rbuf_extra_size)` after `memset(rbuf, 0, sizeof(*rbuf))` | none, given a new reading | H4 holds only if a whole-object zeroing `memset` is read as establishing the member null; no such reading exists in the gateway |
| `rd_kafka_snappy_init_env` (`snappy.c:1837`) | `env->hash_table = vmalloc(...)` (`rd_malloc`) after `clear_env(env)` | H1 (a null test of the place follows) | H4 needs the in-unit inline `clear_env` (`memset(env, 0, sizeof(*env))`) read through the call; H1 holds only if a null test of the place after the store is allowed |
| `rd_list_grow` (`rdlist.c:43`), `rd_kafka_topic_partition_list_grow` (`rdkafka_partition.c:2816`) | `P->F = rd_realloc(P->F, n)` | H4 | `rd_realloc` states `resource_return_reallocated(ptr, nullable)` (`librdkafka/memory.qv`): no non-null clause, though `rd.h:151-155` asserts the result (rule 37's shape, not declared for `rd_realloc`) |
| `create_{counter,gauge,untyped,summary,histogram,exp_histogram}_instance` (6, `cmt_decode_msgpack.c:38-161`) | `map->parent = (void *) counter` (local `calloc`, null-tested before) | H4 | the old `map->parent` is the caller's; nothing in the body establishes it |
| `rd_kafka_assignment_init` (`rdkafka_assignment.c:1274`) | four `rk->rk_consumer.assignment.X = rd_kafka_topic_partition_list_new(100)` | H4 | caller's old values; embedded paths below the member |
| `rd_kafkap_Fetch_reply_tags_set_topic_cnt` (`rdkafka_fetcher.c:346`) | `reply_tags->Topics = rd_calloc(...)` | H4 | `rd_dassert(!reply_tags->Topics)` compiles to nothing (`ENABLE_DEVEL:BOOL=OFF` in the run's CMake cache) |
| `rd_kafkap_Fetch_reply_tags_set_topic` (`:355`) | `reply_tags->Topics[TopicIdx].Partitions = rd_calloc(...)` | H4, shape | the same disabled `rd_dassert`; the place lies past a pointer hop below the formal (`Topics[i]`), not in `*reply_tags` |
| `otel_span_set_attributes`, `otel_span_set_events` (`ctr_encode_opentelemetry.c:695`, `:767`) | `span->attributes = set_attributes_from_ctr(attr)` | H4 (`_events` also H1) | caller's old value; the value is an in-unit static whose summary would have to return `Owned` (not checked); `_events` null-tests the place after |
| `rd_kafka_topic_partition_set_metadata_from_rktp_stored` (`rdkafka_partition.c:3076`) | `rktpar->metadata = rd_malloc(...)` | H1, H4 | `memcpy(rktpar->metadata, …)` reads the place after; caller's old value |
| SCRAM `…_client_final_message_wo_proof`, `…_generate_nonce` (`rdkafka_sasl_scram.c:258`, `:91`) | `out->ptr = rd_malloc(out->size + 1)` | H1, H4 | `rd_snprintf(out->ptr, …)` / `dst->ptr[i] = …` use the place after; caller's old value |
| `mk_buffer_cat` (`mk_utils.c:397`) | `p->data = (char *) mk_mem_alloc(...)` | H1, H4 | three uses of `p->data` after; caller's old value |
| `mk_vhost_set_property` (`mk_lib.c:488`) | `vh->documentroot.data = mk_string_dup(v)` | H4 | caller's old value; embedded path |
| `unpack_opts_unit` (`cmt_decode_msgpack.c:227`) | `map->unit = value` / `map->unit = NULL` | alias, H4 | `map = (struct cmt_map *) context`: a store through a local alias of the formal |

**Count.** Readable soundly under the brief as written: **0 today**; **2** with
a new zeroing reading (`rd_buf_init`; `rd_kafka_snappy_init_env` also needs the
inline call read and a post-store null test allowed); **4** with, in addition,
a checked non-null `rd_realloc` specification. The other 17 rows fail H4 on
the caller's old value (16 through the formal, 1 through a local alias of it).

Per reason (a row may carry several): caller's old value not established 17;
place used after the store 6 (a null test 2; memcpy, snprintf or index writes
4); embedded path 2; pointer hop below the formal 1; realloc without a non-null
clause 2; whole-object zeroing not read 2; local alias of the formal 1.

## 2. Why H4 decides the yield

Rule 10v already admits identity-free stores into an untracked owner under the
stated premise **P-CG11-CALLER-SLOT** ("the old value of a slot in an untracked
object is not this body's obligation"), and MEMBER-EFFECT-STORES' recorded
design places the old-value check at the call site ("an untracked member's old
value must be established custody-free (fail closed otherwise)"). The r35
brief instead requires H4 inside the body. Fail-closed settles H4 (refuse), but
the brief's stop rule then applies. Whether this card's bodies may carry
P-CG11-CALLER-SLOT, with every caller refused at the call by construction, is a
semantic choice for the user, not one to guess.

## 3. Options (hand-back)

1. **(recommended) Carry P-CG11-CALLER-SLOT for the hand-off store, H1-H3
   strict.** The body's `Safe` states the premise in its assumptions, as rule
   10v's does; the summary is withheld, so every caller stays refused at the
   call and the old value becomes the caller half's obligation (its recorded
   design). H1 allowing a null test of the place after the store, embedded
   member paths `P->F.G` allowed, a pointer hop below the formal refused.
   Expected rows: `rd_buf_init`, `rd_kafka_assignment_init`,
   `rd_kafkap_Fetch_reply_tags_set_topic_cnt`, `rd_kafka_snappy_init_env`, the
   six `create_*_instance`, `mk_vhost_set_property`, and the two otel setters
   if their in-unit callees summarise `Owned`: **11-13 rows**. Still refused:
   the realloc pair, the four place-used-after rows, `set_topic`,
   `unpack_opts_unit`.
2. **The brief as written (H4 in the body), with a zeroing reading and a rule
   37 `rd_realloc` specification.** A whole-object `memset(P, 0, sizeof(*P))`
   (or an in-unit inline wrapper of one) before the store, with no write to
   `P->F` between, establishes the old value null (stated premise: the null
   pointer is all-bits-zero on the supported targets, C11 6.2.6.1 leaves it
   to the implementation); and `rd_realloc` gains rule 37's checked
   `resource_return_reallocated(ptr, nonnull)` beside `rd_calloc` and
   `rd_strdup` (a provider change read at every `rd_realloc` call, with
   LIBC-REALLOC-SIZE-ZERO's size-zero case to settle). **2-4 rows**, at the
   cost of a corpus-wide contract change for two of them.
3. Option 1 plus borrowed uses of the place after the store (reads and writes
   through it under borrowed contracts, no release, rebind or escape before
   return): adds the SCRAM pair, `set_metadata_from_rktp_stored` and
   `mk_buffer_cat` (**up to 17 rows**). It needs an argument that no call
   between the store and the return can release the member, which the
   withheld summary does not give by itself; better as its own follow-up.

## 4. Gate

No gate and no semantic change (the card stops before its prediction). The
re-census script read only the user's report; nothing was written under
`build/`. `python3 scripts/apply_amendments.py
docs/reports/member-effect-stores/integration-amendments.json --check`: see
the hand-back.
