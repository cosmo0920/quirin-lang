# AG1-INDIRECT-MEMBER-ACCESS: the access refusal, by sub-shape; rule 61

Card AG1-INDIRECT-MEMBER-ACCESS (round 33, 2026-11-18). USER DECISION
(2026-11-18, verbatim, through the integrator's brief): "We need to launch
those of the implementation cards to resolve the biggest gaps of analyzing
works of the corpus of fluent-bit source tree." Base `a9c6dcf44`
(integrate/cg25-thread-key). Gateway [rule 61](../../agents/gateway.md#constraint-61),
Lean `QCore.TypedefAggregateBase` (9 audited theorems,
`proof/AuditTypedefAggregateBase.lean`). [PREDICTION.md](PREDICTION.md) was
committed before the implementation commit.

## 1. Census (method step 1)

**Input.** The user's `current --monkey-linked` run at Fluent Bit 046accf6
(`build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`,
read only): 1,100 Safe / 3,825 Unknown / 9 Unsafe function rows; 2,285
Unknown rows carry `indirect memory access is outside the resource
projection` (the integrator's 2,290 counts five more rows; this card's
`extract.py` counts Unknown rows only), **232** carry nothing else.

**Method.** MEMBER-ACCESS-PROJECTION's AST census, reused with recorded
changes (`scripts/`): `prep_units.py` rebuilt every analysed text with the
gateway's own preprocessing and accepted it only on the report's
`preprocessed_source_sha256` (**186 of 186** match; one Monkey unit has no
compile identity and is skipped); `classify.py` binds each access leaf to
its function's Clang constructs at its line (9,560 leaf groups, 7 unbound);
`premise.py` gives each leaf one premise; `refine.py` gives each sole-blocker
function a concrete sub-shape and names its owner. The census tables are
[census/premise-table.md](census/premise-table.md) (all rows),
[census/path-table.md](census/path-table.md),
[census/sole-shapes.md](census/sole-shapes.md),
[census/refined-sole-shapes.md](census/refined-sole-shapes.md) and every
sole-blocker site with its analysed statement,
[census/sole-sites.txt](census/sole-sites.txt).

**The analyzer's own reading.** A unit-scoped `current` run of the card base
with the `indirect-census` feature over the 63 non-Monkey units holding a
sole-blocker row (`scripts/census-run.sh census-sole`, `--selection`, binary
`400317f6…`, 92 minutes, exit 0) reproduces the user's rows for those units:
3,393 of 3,394 rows identical in status and leaves (`scripts/compare.py`);
the one difference, `ut_create_fetch_op_with_offset`, is a run-wide fold
(`the run's stores into rd_kafka_op_s.rko_op_cb`) a selection cannot see. Its
gate stream ([census/store-base-causes.md](census/store-base-causes.md))
names why the declaration reader did not type each refused member store's
base: on an automatic local, 869 refusals in 267 functions answer
`TypedefDefinesAggregate` and 72 in 14 functions `TypedefDisagreeing` (depth 1), beside
703 on a typed (`Committed`) local; on a formal, 565 answer
`TypedefDefinesAggregate`. Most of those are scalar stores a later route
(rule 10t) reads; the owned-result stores among them are rule 61's shape.

### 1.1 All rows, by premise (rows mentioning; rows blocked by that premise alone)

| premise | leaf groups | Unknown rows mentioning | sole-blocker rows (premise alone) |
|---|---:|---:|---:|
| `read:pointer-member` | 2,174 | 906 | 42 |
| `call:uncontracted-callee` | 1,893 | 772 | 31 |
| `address:sub-object` | 1,778 | 685 | 35 |
| `form:union` | 740 | 275 | 10 |
| `store:pointer-name` | 465 | 289 | 14 |
| `store:pointer-call-result` | 395 | 240 | 19 |
| `store:pointer-other` | 365 | 239 | 10 |
| `store:pointer-identity-free` / `-null` | 275 / 233 | 83 / 153 | 0 |
| `read:custody-free` | 184 | 112 | 7 |
| every other premise | 1,058 | — | 23 |
| several premises in one row | | | 41 |

### 1.2 The 232 sole-blocker rows, by concrete sub-shape

Ranked by rows the sub-shape alone blocks ([refined-sole-shapes.md](census/refined-sole-shapes.md)
has samples):

| sub-shape | alone | mentioning | owner / why it stays |
|---|---:|---:|---|
| several sub-shapes in one row | 41 | | |
| uncontracted callee whose actual neither reader reads (`rd_kafka_log0`, `rd_kafka_timer_start0`, `rd_list_copy`, `rd_kafka_topic_partition_list_copy`, `mk_list_init`) | 31 | 41 | contracts; rule 12k cannot name the callee until the actual is read |
| an embedded list's address handed to a list helper (`rd_list_add(&c->config, e)`, `rd_list_destroy(&conf->interceptors.on_new)`, `rd_kafka_interceptor_method_add(&rk->…)`) | 22 | 22 | container custody: decision class 1 / rule 46 |
| a pointer member read at an actual whose callee's mode or another premise decides (`strlen(a->p) + strlen(b->q)`, `open(cf->path, …)`) | 17 | 20 | rule 14a admits one call only (below); descriptor cells (rule 38) |
| a formal's or local's pointer stored into a member (`pmi->member = member`, `map->parent = (void *) counter`) | 14 | 21 | decision class 1 (`retained_by`), rule 10v (Q36 example C) |
| a sub-object address at another position (`memset(cctx->hashTable, …)`, `memcpy(copy->base64str, …)`) | 12 | 18 | the callee's mode / interior bounds |
| a formal array's elements released (`*_destroy_array`: `for (…) X_destroy(a[i]);`) | 12 | 12 | rule 5k / W45-MEMBER-RELEASE-PREMISES, by design |
| an owned result stored into a **formal's** member (`rbuf->rbuf_extra = rd_malloc(…)`, `span->attributes = …`) | 11 | 15 | MEMBER-EFFECT-STORES (rule 38i names it where its premises hold) |
| a call through a member or function pointer | 9 | 11 | rule 11p |
| `return rd_kafka_op_err_destroy(rd_kafka_op_req(mcluster->ops, rko, -1));` (the mock API) | 8 | 18 | `rd_kafka_op_err_destroy` has no contract (cross-unit body) |
| an owned result stored into a **local** aggregate's member (`ret->host = rd_strdup(host)`) | 7 | 10 | **rule 61 where the base is alias-spelled** (section 2) |
| a custody-free read beside `?:`, a cast or a descriptor (`close(cf->fd)`) | 7 | 8 | rules 10z, 38 |
| pointer arithmetic stored into a member (`w->op_limit = w->op + len`) | 7 | 8 | rule 10v (an identity-carrying value) |
| a union member path (`LZ4_stream_t`'s `internal_donotuse`, `rko->rko_u.mock.name = rd_strdup(…)`) | 9 | 28 | rule 10w (a pointer store through a union refuses) |
| 12 smaller sub-shapes (an aggregate value, another pointer store, a member released with the parent kept, a log macro, file-scope and static roots, …) | 25 | | each at most 5 |

**What earlier cards refused on purpose, not re-admitted here.** Rule 14a's
"**Do not** admit two calls" (`QCore.ResidualAccessReadings.two_calls_are_never_read`):
a statement holding two `Borrowed`-only calls stays refused; extending it is
a follow-up below, not done here. Rule 10v's identity-carrying values (a
formal's or local's pointer, arithmetic, a member read stored into a member;
Q36's missing owner premise). Rule 10w: a pointer store through a union.
Rule 5k P4 / W45-MEMBER-RELEASE-PREMISES: a member or element of a formal
released while the formal is kept (rule 38j admits the member half under
its own checks; elements in a loop stay refused). Decision class 1
(`retained_by`, the user's 2026-10-05 split) for pointers retained in a
member or a container, class 3 for reference-count keeps
(`rd_kafka_toppar_keep0` stored into `rko->rko_rktp`), MEMBER-EFFECT-STORES
for owned values stored into a formal's member. CG10's "yield about one per
lift" stopping rule still describes the sole set: every remaining sub-shape
above three rows is a design refusal, a decision class or a contract
question.

## 2. Rule 61: a local spelled through a struct alias (method steps 2 and 4)

**The shape.** librdkafka's constructors allocate a record through a typedef
and store owned results into it:

```c
rd_kafka_topic_partition_t *rktpar = rd_calloc(1, sizeof(*rktpar));
rktpar->topic = rd_strdup(topic);       /* refused: indirect memory access */
return rktpar;
```

The keyword spelling (`struct rd_kafka_topic_partition_s *rktpar`) is
admitted by O04-B08's aggregate store (a field link from a live owner this
body acquired; probe s1 at the base, `Safe`). The alias spelling is refused
because the lexical declaration reader answers `Opaque` for a typedef that
**defines** its struct (`typedef struct TAG { … } T;`, kept byte for byte by
ABI-TYPEDEF-ADDRESS-OF-MEMBER) and `None` for a name declared by a forward
and a defining typedef it cannot compare (`rdkafka.h`'s
`typedef struct rd_kafka_Node_s rd_kafka_Node_t;` beside
`rdkafka_aux.h`'s defining one), so B08's premise 2 ("the base is exactly
`struct TAG *`") never held. Probes at the base: s2 (a forward typedef only)
`Safe`; s3 (defining) and s10 (forward and defining) refused at the store.

**Why reading it is sound.** C11 6.7.8p3: a typedef name is a synonym for the
type it names. When every typedef declaration of `A` in the unit names one
`struct TAG`, a local declared exactly `A * NAME` has type `struct TAG *`; B08's
premise is then a true fact, and every other premise (one acquired binding,
no escape, no alias, the member classes of `struct TAG`, the flow-sensitive
owner check, P-B08-SHALLOW) still decides exactly as for the keyword
spelling (`QCore.TypedefAggregateBase.the_store_decides_alike_over_either_spelling`).
The reading refuses a union, anonymous, pointer, disagreeing or incomplete
alias, a qualified or multi-declarator declaration, a formal, a name declared
twice and a name the shared reader typed otherwise; the shared classifier's
answer is unchanged for every other consumer. The forward-and-defining pair
also makes `sizeof(*NAME)` an unevaluated operand (a struct type is never
variably modified, C11 6.7.6.2p2, 6.5.3.4p2), which the base scan needs
(probe s10 was refused at that `*` before the second hook).

**Re-attribution, not admission.** `rd_kafka_Node_new` stores `rack` on one
arm only (`if (rack != NULL) ret->rack = rd_strdup(rack);`); the store is now
read and the join's partial link meets rule 11c's escape refusal (`field-link
escape: a linked child may still be live on a path the escape does not carry
it`), unchanged and honest.

**Fixtures** (`crates/qcore-c-gateway/tests/ag1_indirect_member_access.rs`,
6 tests; every `Safe` result's certificate rechecked): admitted, a defining
alias and a forward-and-defining alias constructor (both declaration forms);
kept non-`Safe`, a released owner leaving its child live (P-B08-SHALLOW), an
aliased base, a release before the store, an overwrite in a loop, a store
through a second path (`slot = &ret->host; *slot = …`), a union alias, a
pointer alias, two declarators, a formal base (rule 38i's name, never the
gate's text), and the one-arm store (rule 11c). Unit tests in
`typedef_aggregate_base.rs` (5) bind the decision to the Lean vectors and
cover the alias table's refusals.

**Lean** `QCore.TypedefAggregateBase`: `an_agreeing_alias_names_its_struct_everywhere`,
`a_disagreeing_alias_names_nothing`, `an_other_declaration_names_nothing`,
`a_missing_fact_never_admits`, `decision_vectors_hold`,
`an_admitted_local_is_the_struct_pointer`,
`the_store_decides_alike_over_either_spelling` (over
`QCore.AggregateStore.admitsStore`), `a_refused_alias_keeps_the_refusal`,
`a_struct_alias_is_never_variably_modified`; `propext` only, no
placeholders.

## 3. Shared-file hook points

Every edit outside the new module is a delimited hook
(`AG1-INDIRECT-MEMBER-ACCESS (rule 61) hook`):

- `crates/qcore-c-gateway/src/typed_field_access.rs`: the field
  `TypedAccessModel::struct_alias_heads`; the free function `struct_head`
  (above `read_aggregate_definition`); the accessor
  `TypedAccessModel::struct_alias_tag` (after `defining_struct_alias`); one
  call in `read_typedefs` before the `typedefs` merge; one match arm
  (`Some(None)`) in `never_variably_modified_at`.
- `crates/qcore-c-gateway/src/aggregate_store.rs` (not on the brief's shared
  list): `AggregateStoreBases::alias_tags` and `base_tag`; the candidate test
  of premise 2 in `store_bases`; `insulated_bases`' extra parameter and its
  `&BASE->F` arm; `admitted_field`'s tag; `member_address_actuals`' tag.
- `crates/qcore-c-gateway/src/lib.rs`: `mod typedef_aggregate_base;`.
- None of `legacy_c.rs`, `legacy_cfg_builder.rs`, `legacy_transfer.rs` or
  `typed_access_path.rs` is touched.

## 4. Measurement

MEASUREMENT

## 5. Gate

GATE

## 6. Follow-ups (registered in [integration-amendments.json](integration-amendments.json))

FOLLOWUPS
