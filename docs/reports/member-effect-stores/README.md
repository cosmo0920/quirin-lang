# MEMBER-EFFECT-STORES, body half (gateway rule 38k)

Round 35. **USER DECISIONS** (verbatim): 2026-11-24, "Launch MEMBER-EFFECT-STORES
body half"; 2026-11-25, answering this card's stop (section 3), "Option 1:
caller-slot premise"; and 2026-11-25, "Plus, banned for just written down as
English description. We must write down with logical symbols for them." Base
`c5cd10688` (integrate/r34-k-batch). [PREDICTION.md](PREDICTION.md) was
committed (`7759651cc`) before the semantic change (`98b21cffe`).

- **Rule.** New lettered [rule 38k](../../agents/rules/38k.md) (38k and 79 were
  free on the base and on origin/main; 38i is a re-attribution that never
  admits, so the admission does not fit there; 38i gains one sentence naming
  38k's precedence).
- **Rust.** [`member_handoff.rs`](../../../crates/qcore-c-gateway/src/member_handoff.rs)
  (the formulas, the decision, the premise carriage, the lexical readings),
  [`legacy_member_handoff.rs`](../../../crates/qcore-c-gateway/src/legacy_member_handoff.rs)
  (the parser half and the flow); the MES block of the typed rendering in
  [`premise_formula.rs`](../../../crates/qcore-frontend/src/premise_formula.rs).
- **Lean.** `QCore.MemberHandoff`
  ([`proof/QCore/MemberHandoff.lean`](../../../proof/QCore/MemberHandoff.lean),
  audited in `proof/AuditMemberHandoff.lean`).

Sections 1-3 are the census and the stop that led to the decision, kept as
written; sections 4-10 are the implementation.

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

## 3. Options (the stop's hand-back; the user chose option 1)

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

## 4. The rule: formulas first

Every condition, the premise and every refusal is a formula of the C04.18.9
typed rendering (G01p: `forall`, `exists`, `!`, `&&`, `||`, `=>`, `==`) over
the new **MES block** of registered terms (29 terms, prefix `mes_`, two value
sorts `mes_site` and `mes_cell`, appended after SP10 through the existing
rendered-term path: `RENDERED_TERMS`/`RENDERED_VALUE_SORTS` in Rust, the
`mesTerms` list in Lean, bound by `tests/unit/member_handoff.rs`). Each formula
is checked by the premise-formula validator `guarded_proposition` (free names
must be registered terms in their rendered sorts, the text must be the
canonical printing); prose is refused by it (`prose_is_refused_by_the_validator`).
At run time the rule checks every formula once (`member_handoff::validated`)
and carries the premise only when all validate.

**Admission** (all hold):

| id | formula | decided |
|---|---|---|
| `mes_place_shape` | `(#mes_formal_root() && #mes_in(#mes_place,#mes_parent))` | Clang AST (`typed_place::read_place`: a `ParmVarDecl` root, `->F` then `.G…`, every step `Ordinary`) |
| `mes_formal_kept` | `forall (#s: mes_site) { !#mes_formal_written(#s) }` | `CallerFormals` |
| `mes_value_shape` | `((#mes_owned_result() \|\| #mes_local_acquisition()) && #mes_pointer_casts())` | contract in force (no hold) returns `Owned`; or a `VarDecl` local of object pointer type; casts `BitCast`/`NoOp` (brief item 4) |
| `mes_h1_place_only_at_store` | `forall (#s: mes_site) { (#mes_occurs(#s,#mes_place) => (#s == #mes_store \|\| #mes_null_test(#s))) }` | fused body tokens (`place_and_formal`); `sizeof` operands are no occurrence |
| `mes_h2_local_kept` | `forall (#s: mes_site) { (!#mes_local_copy(#s) && (#mes_local_mention(#s) => !#mes_after(#s,#mes_store))) }` | `local_kept` |
| `mes_h3_no_stale_value` | `forall (#o: obj) { (#mes_stored(#o) => (#mes_owed(#o) && #mes_live(#o))) }` | flow: exactly one live, unreleased, unescaped, uncontained, resolved identity |
| `mes_not_in_loop` | `!#mes_in_loop(#mes_store)` | `loop_ranges` (any `goto`/`setjmp` refuses) |
| `mes_nothing_released_after` | `forall (#s: mes_site) { (#mes_after(#s,#mes_store) => ((!#mes_formal_passed(#s) && !#mes_indirect(#s)) && (#mes_call(#s) => #mes_borrowed_only(#s)))) }` | `place_and_formal`, `indirect_after`, `calls_after` against the contracts in force |
| `mes_owner_untracked` | `(#mes_untracked_owner() && !#mes_bound_old())` | flow: rule 10v's owner class and old-value check |
| `mes_summary_withheld` | `!#mes_published()` | rule 10v's summary gate (`has_pointer_store`), rule 70's statement |
| `mes_premise_stated` | `#mes_premise_carried()` | `carry_premise`, `enforce_premise` |

**The stated premise** (P-CG11-CALLER-SLOT for this store):
`mes_caller_slot: forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }`.
Every row whose CFG holds a hand-off lists exactly that text in its verdict
assumptions (`rule_obligations.premises`, rule `38k`; the audit folds it into
the verdict contract's `assumptions`); a row with a hand-off that does not is
refused (`mes_refuse_premise_missing`).

**Named refusals** (each the negation of one condition; `QCore.MemberHandoff.a_refusal_negates_its_condition`):

| id | formula |
|---|---|
| `mes_refuse_realloc_self` | `(#mes_reallocates() && !#mes_nonnull_clause())` |
| `mes_refuse_value_shape` | `!((#mes_owned_result() \|\| #mes_local_acquisition()) && #mes_pointer_casts())` |
| `mes_refuse_local_alias_of_formal` | `!#mes_formal_root()` |
| `mes_refuse_place_shape` | `!#mes_in(#mes_place,#mes_parent)` |
| `mes_refuse_rebound_formal` | `exists (#s: mes_site) { #mes_formal_written(#s) }` |
| `mes_refuse_loop` | `#mes_in_loop(#mes_store)` |
| `mes_refuse_place_used` | `exists (#s: mes_site) { (#mes_occurs(#s,#mes_place) && !(#s == #mes_store \|\| #mes_null_test(#s))) }` |
| `mes_refuse_local_used` | `exists (#s: mes_site) { (#mes_local_copy(#s) \|\| (#mes_local_mention(#s) && #mes_after(#s,#mes_store))) }` |
| `mes_refuse_borrowed_use_after` | `exists (#s: mes_site) { (#mes_after(#s,#mes_store) && ((#mes_formal_passed(#s) \|\| #mes_indirect(#s)) \|\| (#mes_call(#s) && !#mes_borrowed_only(#s)))) }` |
| `mes_refuse_owner` | `!#mes_untracked_owner()` |
| `mes_refuse_owned_old_value` | `#mes_bound_old()` |
| `mes_refuse_stale_value` | `exists (#o: obj) { (#mes_stored(#o) && !(#mes_owed(#o) && #mes_live(#o))) }` |
| `mes_refuse_premise_missing` | `!#mes_premise_carried()` |

Parity runs record each consultation (`position: "member-handoff"`) with the
refusal's id and formula.

**Why it is sound.** On the admitted path the value is one identity the body
owes (an `Owned` result just produced, or a local acquisition that is live,
never copied and never mentioned again), the place is storage of the
caller's object reached through a formal the body never rebinds, nothing
after the store can reach the place (no hop, no subscript, no dereference, no
whole formal, no call that releases or consumes anything), and no loop or
jump repeats the store. Consuming the identity there (`resource_consumed(v,
nullable)`) moves its custody into the caller's object: the body's
obligation is discharged by the hand-off and by nothing else
(`discharged_only_by_the_handoff`). What the store overwrites is the caller's,
by the stated premise, which the row lists
(`the_old_value_is_custody_free_or_premised`). No summary is published, so no
caller reads the body as borrowing its formal: every call of it stays refused
(`no_summary_with_a_handoff`, `a_callers_decision_is_unchanged`). A store
that fails any formula is the gate's refusal (rule 38i's name of it), or,
where W45 lowered the statement, W45's own store, which then decides a value
with no identity exactly as before.

## 5. Shapes on the 21 rows

| Rows | Reading |
|---|---|
| `rd_buf_init`, `rd_kafka_assignment_init` (4 stores), `rd_kafkap_Fetch_reply_tags_set_topic_cnt`, `rd_kafka_snappy_init_env`, the six `create_*_instance` (cast of a local acquisition), `mk_vhost_set_property` (embedded `vh->documentroot.data`) | admitted (11 rows) |
| `otel_span_set_attributes`, `otel_span_set_events` | admitted only where the in-unit `set_attributes_from_ctr`/`set_events_from_ctr` summarise `Owned`; otherwise `mes_refuse_value_shape` |
| `rd_list_grow`, `rd_kafka_topic_partition_list_grow` | `mes_refuse_realloc_self` (`rd_realloc` states `resource_return_reallocated(ptr, nullable)`; no provider change here) |
| SCRAM `…_wo_proof`, `…_generate_nonce`, `rd_kafka_topic_partition_set_metadata_from_rktp_stored`, `mk_buffer_cat` | `mes_refuse_place_used` (the place used after the store: option 3) |
| `rd_kafkap_Fetch_reply_tags_set_topic` | `mes_refuse_place_shape` (`Topics[i].Partitions`: a hop below the formal) |
| `unpack_opts_unit` | `mes_refuse_local_alias_of_formal` (`map = (struct cmt_map *) context`) |

**Unit reproduction** ([scripts/unit_texts.py](scripts/unit_texts.py) writes
each unit's analysed `-E -P` text from the run's own compile database into
`private/mes/units/`; [scripts/unit_probe.rs.txt](scripts/unit_probe.rs.txt),
a throwaway test kept as text and never a cargo target, analyses them in
release with `cycle19-resource-contracts.json`, the manifest the user's run
read). First run, `analyze_legacy_c_translation_unit` (C11):
[evidence/unit-probe-1.txt](evidence/unit-probe-1.txt), 10 units; second run,
`analyze_unrestricted_c11` under the units' own `-std=gnu17` for the four units
where the first harness added K3's CG30 limit:
[evidence/unit-probe-2.txt](evidence/unit-probe-2.txt).

| Row | Outcome |
|---|---|
| `rd_buf_init`, `rd_kafka_assignment_init`, `rd_kafkap_Fetch_reply_tags_set_topic_cnt` (also rule 37's `asserted_allocation`), `rd_kafka_snappy_init_env` | **Safe**, no diagnostic, premise `mes_caller_slot` listed (run 1) |
| the six `create_*_instance`, `mk_vhost_set_property` | **Safe**, no diagnostic, premise listed (run 2, gnu17) |
| `otel_span_set_attributes`, `otel_span_set_events` | Unknown, the store's leaf only (run 2): refused (the in-unit callees give no `Owned` value here; the reason was not isolated further) |
| `rd_list_grow`, the SCRAM pair, `rd_kafkap_Fetch_reply_tags_set_topic`, `mk_buffer_cat`, `unpack_opts_unit` | Unknown at the store, as predicted |

`rd_kafka_topic_partition_list_grow` and
`rd_kafka_topic_partition_set_metadata_from_rktp_stored` (`rdkafka_partition.c`,
a large unit) were not reproduced; their stores are refused by name in
section 5's reading. **11 of the 11 predicted rows reproduce Safe**; the two
conditional rows stay Unknown.

## 6. Tests

- [`tests/member_effect_stores.rs`](../../../crates/qcore-c-gateway/tests/member_effect_stores.rs),
  6 tests, every `Safe` certificate rechecked: admitted (an owned call
  result, a nullable one, an embedded place, two sibling places, null tests of
  the stored value after the store, a tested local acquisition, a cast of an
  initialised local acquisition), each row listing the premise; adversarial
  near-misses (the local used, released or stored again after the store; a
  copy of the local released after; realloc self-assignment; a rebound
  formal; a store through a local alias of the formal; a store in a loop; a
  pointer hop; a failed acquisition storing NULL then dereferenced through the
  place or the local; a second store into the place; the formal handed on
  after; a release or a write through another pointer after; a value read
  from memory; an integer cast); a caller of an admitted body stays non-`Safe`
  (a formal parent and a tracked local parent); a member release beside a
  hand-off publishes no effect its caller could read; a verdict lacking the
  premise is refused; the premise is the formula.
- [`tests/unit/member_handoff.rs`](../../../crates/qcore-c-gateway/tests/unit/member_handoff.rs),
  12 tests: every formula validates; prose is refused; the decision vectors
  and the formula table are the Lean ones; the MES block is Lean's; the
  lexical readings (loops and jumps, null tests, the place, the local, later
  indirect accesses).

**Re-pins, each named in its test** (all the shape this rule admits, no
status worsened):

- `member_effect_summary.rs`: `setnew` (`c->a = malloc(4)`, non-Windows) and
  `setnew2` (`c->a = obj_create(4)`) `Safe`; a new `setnew3` (two stores into
  the place) keeps rule 38i's name.
- `member_effect_component_scope.rs`: `cmt_opts_init` `Safe` (its
  `opts->F = cfl_sds_create(..)` hand-offs with null tests after).
- `cg11_pointer_stores.rs`: `void *q = q_alloc(); s->p = q;` `Safe`; `q_free(q)`
  after it refused.
- `member_store_retention.rs`: the refused local now carries `q_free(q)` after
  the store.
- `sub_object_stores.rs`: `h->name = x_dup(s)` and `char *n = x_dup(s); h->name = n;`
  `Safe` (W45 still decides the identity-free name, `char *t = 0; h->name = t;`).
- `ag1_indirect_member_access.rs`: `n->host = strdup(host)` `Safe`; with
  `free(n->host)` after it keeps rule 38i's name.
- `clang_gap_phase2.rs` (M03), `o04_b08.rs` (`k_fill`; `k_untracked(b)` after
  keeps B08's refusal), `leak_pin_audit.rs` (the member form; a compared place
  keeps the refusal and the leak withdrawn), `scalar_precedence.rs`
  (`p->next = q`; `q_free(q)` after stays refused).
- `monkey_vhost_property_custody.rs`: `set_root` and the whole
  `mk_vhost_set_property` `Safe` under the premise; a read of the member after
  the store keeps the refusal. **Concern recorded**: MONKEY-VHOST-PROPERTY-CUSTODY
  refused the DocumentRoot store as a leak candidate
  (MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE: a second DocumentRoot for one host
  loses the first copy). Under the user's option 1 that old value is the
  host's, the caller's, which is what `mes_caller_slot` states; the body
  publishes no summary, so `mk_vhost_set` stays refused at the call and the
  candidate stays open there. The integrator should confirm this reading; the
  alternative is to keep that one body refused by an explicit premise
  withdrawal, which no rule here provides.
- `qcore-contract/tests/plugin_guarded_symbols.rs`: a rendered-term block may
  follow SP10 (the MES block).

## 7. Lean

`QCore.MemberHandoff`, 19 theorems, audited in `proof/AuditMemberHandoff.lean`
(axioms `propext`, `Classical.choice`, `Quot.sound` only; no `sorry`, no
project axiom): `admitted_exactly_when`, `a_refusal_negates_its_condition`,
`discharged_only_by_the_handoff`, `a_refusal_is_the_gates`,
`a_used_local_keeps_the_refusal`, `a_released_local_keeps_the_refusal`,
`a_second_store_keeps_the_refusal`, `a_realloc_self_assignment_is_refused`,
`a_loop_is_refused`, `a_rebound_formal_is_refused`,
`a_local_alias_of_the_formal_is_refused`, `a_borrowed_use_after_is_refused`,
`a_row_lacking_the_premise_is_refused`, `a_carried_row_lists_the_premise`,
`the_old_value_is_custody_free_or_premised` (an admitted store's old value is
unbound in the projection, and a `Safe` row holding it lists the premise),
`no_summary_with_a_handoff`, `a_callers_decision_is_unchanged`,
`decision_vectors_hold`, `the_tables_are_counted`. `formulaTable` and
`mesTerms` are the Rust tables, row for row.

Stated, not proved: `mes_caller_slot`; faithful Clang extraction (trust
boundary 28); the lexical readings (`#mes_after` as lexical order in a body
without loops and jumps over-approximates may-execute-after).

## 8. Shared-file hook points

Each delimited `MEMBER-EFFECT-STORES hook (rule 38k): begin` / `end` where it
is more than one line:

- `crates/qcore-c-gateway/src/legacy_c.rs`: `#[path = "legacy_member_handoff.rs"] mod member_handoff_route;`
  after rule 72's module; the statement hook after rule 72's
  `owned_out_member_statement` in `expression_statement_lowered`; the
  `carry_premise` call after `legacy_conditional_embedded_storage::diagnose`
  in `analyze_translation_unit_under_holds`.
- `crates/qcore-c-gateway/src/legacy_transfer.rs`: one arm in
  `transfer_dispatch`, before rule 10v's `PointerStore` arm (the hand-off's
  flow, with the full dispatch for its fallback).
- `crates/qcore-c-gateway/src/pointer_store.rs`: `ValueClass::MemberHandoff`,
  its code, its `Deserialize` entry.
- `crates/qcore-c-gateway/src/member_store_retention.rs`: a hand-off states no
  retention (`statement` returns `None`).
- `crates/qcore-c-gateway/src/lib.rs`: `pub mod member_handoff;` after
  `mod owned_out_member;`.
- `crates/qcore-c-gateway/Cargo.toml`: the `qcore-frontend` path dependency
  (already in the workspace; `Cargo.lock` gains the edge).
- `crates/qcore-frontend/src/premise_formula.rs`: the MES block appended to
  `RENDERED_VALUE_SORTS` and `RENDERED_TERMS`;
  `crates/qcore-contract/tests/plugin_guarded_symbols.rs` admits a block after
  SP10.
- `docs/agents/rules/38i.md` (one sentence), `docs/agents/rules/38k.md`;
  generated `docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean`.

None of `legacy_cfg_builder.rs`, `legacy_classify.rs`,
`typed_access_path.rs`, `typed_field_access.rs` is edited.

## 9. Checks

No card gate (the brief: one gate on the merged tip). Targeted checks, logs
under `private/mes/` (worktree-private):

| Command | Exit | Count |
|---|---:|---|
| `cargo test -p qcore-c-gateway --test member_effect_stores` | 0 | 6 passed |
| `cargo test -p qcore-c-gateway --lib member_handoff` | 0 | 12 passed |
| `cargo test -p qcore-c-gateway --lib` | 0 | 1,431 passed, 2 ignored |
| the re-pinned and related files (38i/38j/13a/CG11/rule 70/rule 72/K3/W45/AG1/B08 and the 17 listed in `private/mes/related-1.log`, `related-2.log`) | 0 each after the re-pins | listed in the logs |
| `gate_slot.py audit -- cargo test -p qcore-c-gateway --no-fail-fast` (one crate run, `private/mes/gateway-tests-1.log`) | 101 | 306 targets, 4,229 passed, 9 failed, 11 ignored |
| the same 9, rerun | | 3 were this card's shape (`monkey_vhost_property_custody` 2, `scalar_precedence` 1), re-pinned and passing; **6 fail identically at the base `c5cd10688`** in a base worktree (`private/mes/basecheck-2.log`): `access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch`, `detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor`, `lexical_closure_nested_assignment::a_never_bound_root_still_certifies`, `scalar_read_identity::pointer_destination_retains_opaque_identity_refusal`, `typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read`, `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local` (the K batch's, not this card's) |
| `cargo test -p qcore-frontend` | 0 | all passed |
| `cargo test -p qcore-contract --test plugin_guarded_symbols --test cjson_guarded_symbols --test formal_binding_descriptions` | 0 | 4 + 8 + 9 passed |
| `gate_slot.py build -- cargo clippy -p qcore-c-gateway --all-targets -- -D warnings` | 0 | |
| `gate_slot.py build -- cargo clippy -p qcore-frontend -p qcore-contract --all-targets -- -D warnings` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `gate_slot.py lean -- gate_lean.py --module QCore.MemberHandoff --audit AuditMemberHandoff.lean` | 0 | 1 target built, 1 audit passed (`private/mes/lean-2.log`) |
| `python3 scripts/gen_rule_docs.py --check` | 0 | 272 rule files |
| `python3 scripts/apply_amendments.py docs/reports/member-effect-stores/integration-amendments.json --check` | 0 | 2 amended, 4 registered |

An earlier Lean run (`lean-1.log`) of the first proof draft was stopped by
this card (its PID only) when the exhaustive case splits proved too slow; the
proofs were rewritten as linear case chains and rerun.

## 10. Follow-ups

- `MEMBER-EFFECT-STORES-CALLER-HALF`: the caller half (CG11 field links on a
  tracked parent; the old value established at the call site; then a summary
  stating the hand-off).
- `MEMBER-EFFECT-STORES-BORROWED-USE-AFTER` (option 3): the place used after
  the store (SCRAM pair, `set_metadata_from_rktp_stored`, `mk_buffer_cat`).
- `MEMBER-EFFECT-STORES-REALLOC-SELF`: `rd_list_grow`,
  `rd_kafka_topic_partition_list_grow` need a rule-37 checked non-null
  `rd_realloc` specification (a provider change; LIBC-REALLOC-SIZE-ZERO).
- `MEMBER-EFFECT-STORES-HOP-AND-ALIAS-ROOTS`: `Topics[i].Partitions` past a
  hop below the formal, and `unpack_opts_unit`'s local alias of a `void *`
  formal.
