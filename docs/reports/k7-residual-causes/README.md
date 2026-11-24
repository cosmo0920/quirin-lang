# K7-RESIDUAL-CAUSES: the residual refusal causes of the access census, re-censused; rule 78

Card K7 of round 34 (2026-11-24; USER DECISION 2026-11-24, verbatim, through the
integrator's brief: "Let's get started out of four."), base `3ca85c7a6`. Bundle
K7 of [CURRENT-ACCESS-ALIAS-CENSUS](../current-access-alias-census/README.md)
(§3–§5, [predictions/K7.md](../current-access-alias-census/predictions/K7.md)):
the typed-path refusal causes other than the pointer-valued store (`union`,
`aggregate-value`, `call`, `indirect-call`, `cast`, `pointer-arithmetic`,
`static-local`, `statement-expression` and smaller tags). Gateway
[rule 78](../../agents/gateway.md#constraint-78), Lean `QCore.PointerOffsetPlace`
(16 audited theorems, `proof/AuditPointerOffsetPlace.lean`).
[PREDICTION.md](PREDICTION.md) was committed (`68aa61220`) before the
implementation commit (`51af41b6e`).

## 1. Re-census (11-07 vs 11-24)

**Input.** The user's `current` run written 2026-11-24 09:58 (Fluent Bit
`046accf6`, read only, never written): 4,934 function rows, 1,173 Safe / 3,750
Unknown / 11 Unsafe. **Method** ([scripts/recensus.py](scripts/recensus.py),
output [artifacts/recensus.json](artifacts/recensus.json)): the 11-07 census
tagged every IND/COND/RET leaf group by the typed path's refusal causes and
every alias leaf by its W47 discharge group; both runs read the same prepared
texts, so a leaf's `(file, function, line, family)` identity is shared (checked
on the frozen list: every line agrees). Each 11-24 leaf takes the 11-07 tag of
its identity; a leaf the 11-07 run did not have is tagged `new:<family>` and
never counts as removable (661 alias, 532 IND, 162 COND, 7 RET leaves on
Unknown rows). A row is freed by a tag set when every item is an
IND/COND/RET/ALIAS item whose tags are all in the set (the census's `rank.py`
rule). Figures are upper bounds.

| Measure | 11-07 (base `7b726587b`) | 11-24 (user run) |
|---|---:|---:|
| Unknown rows | 3,832 | 3,750 |
| K7 leaves | 1,849 | 1,798 |
| Unknown rows touched | 739 | 701 |
| Rows freed alone by the bundle | 40 | **47** (48 if the one row with new leaves had them removable) |
| `union` leaves / rows / alone | 967 / 323 / 8 | 965 / 322 / 15 |
| `aggregate-value` | 252 / 169 / 9 | 246 / 164 / 7 |
| `call` | 137 / 98 / 8 | 130 / 91 / 9 |
| `indirect-call` | 185 / 142 / 6 | 155 / 118 / 6 |
| `cast` | 71 / 38 / 0 | 69 / 37 / 0 |
| `statement-expression` | 118 / 72 / 0 | 117 / 71 / 0 |
| `static-local` | 64 / 18 / 2 | 64 / 18 / 2 |
| `pointer-arithmetic` | 33 / 15 / 2 | 33 / 15 / 2 |

**The 11-07 frozen list (40 functions).** 1 is Safe (`get_member_id`); 3 changed
blocker (each now carries leaves the 11-07 run did not have: `rd_map_init`,
Monkey's `channel_write_in_file`, `mk_utils_hex2int`); 36 are still freed alone
by the bundle. 11 rows are newly freed alone (their other leaves left since
11-07): `LZ4_createHC`, `rd_kafka_DescribeConsumerGroups_result_groups`,
`rd_kafka_DescribeTopics_result_topics`, `ut_make_ListConsumerGroups_fanout`,
`ut_make_ListConsumerGroups_partial_error`, `rd_kafka_confval_init_ptr`,
`rd_kafka_message_produce_errstr`, `unittest_msg_seq_wrap`, `count_attributes`,
`rd_hexdump`, `ut_strcasestr`.

**The census tag is not the analyzer's reason.** The tags are W46's Python
path walker's (`access_census.py`): it tags `union` for any union step, though
rule 10w reads a union scalar read neutrally. Unit probes on the card's
harness (real Clang, `tests/k7_residual_causes.rs`) found the reasons below;
`return p->v.a` through a union is `Safe` on the base, so a `union`-tagged
row is held by something else (a pointer-valued return, an address actual, a
call beside the read).

### 1.1 The 47 rows, by concrete shape and owner

| Shape (sample) | Rows | Decision |
|---|---:|---|
| pointer-valued return read, union member or call result (`return entry_variant->data.as_kvlist;`, `return confval->u.PTR;`, `return f(r)->cluster_id;`, `return (T **)rko->rko_u.admin_result.results.rl_elems;`, `LZ4_slideInputBuffer`'s punned pointer, `ElectLeaders`' `rd_list_elem`) | 12 | **refused on purpose**: W45's derivation decision (AG3 §1.1, `W45-RETURN-POINTER-RESULT-READS`) |
| call through a function pointer (cJSON's `global_hooks.allocate`, `hooks->allocate`, `rk_conf.sasl.scram_H`, `configs[i].getValue`, `rmap->rmap_cmp`, `rko_u.broker_monitor.cb`) | 7 | **refused on purpose**: open candidate sets, rule 11p (`CG22-OPEN-INDIRECT-CALLS`) |
| record with pointer members passed by value (`LZ4F_free(p, cdict->cmem)`, `LZ4F_CustomMem`) | 3 | **refused on purpose**: CG14's `pointer-members` (rule 11f) |
| cast view of a tracked root (`&((LZ4_streamHC_t*)p)->internal_donotuse`) | 2 | **refused on purpose**: CAST-FORMALIZATION (a) |
| embedded list's address to a list helper (`rd_list_init(&rko->rko_u.admin_result.results, …)`) | 2 | **refused on purpose**: decision class 1 / rule 46 (AG1 §1.2) |
| pointer store through a union (`confval->u.PTR = NULL;`) | 1 | **refused on purpose**: rule 10w (AG1 §1.2) |
| **pointer offset under `*` or `[]`** (`*(str + 1) == 's'`, `(b->content + b->offset)[0] <= 32`) | **2** | **rule 78** (this card) |
| `<ctype.h>` table read (`isspace(c)` = `(*__ctype_b_loc())[c] & …`) | 4 | follow-up `K7-CTYPE-TABLE-READS` (needs a libc declaration of `__ctype_b_loc` and a read through its result) |
| array-element place of the function's own array (`static const struct {…} exp[]`, `exp[i].in`; an automatic one too) | 4 | follow-up `K7-OWN-ARRAY-ELEMENT-PLACES` (rule 12j calls a subscript a hop; two rows also need RD_UT_ASSERT's other leaves, one is a thread-local buffer) |
| custody-free record value (`return rktp->rktp_next_fetch_start;`, `rd_kafka_Uuid_cmp(a->id, b->id)`, a `TAILQ_FOREACH`) | 4 | `CG14-AGGREGATE-VALUE-POSITIONS` (amended); the two `rd_kafka_Uuid_cmp` rows reach a cross-unit callee |
| `errno` returned or as an actual (`return errno;`, `cmt_errno()`) | 2 | `AG3-ERRNO-RETURN` (amended); `cmt_errno_print` is cross-unit |
| union member address to a same-unit callee (`LZ4_prepareTable(&ctx->internal_donotuse, …)`) | 2 | follow-up `K7-UNION-MEMBER-ADDRESS-ACTUAL` (the callee's mode; `LZ4HC_init_internal` is itself Unknown) |
| call in a subscript index (`return hdr->counts[rd_hdr_countsIndex(…)];`) | 1 | follow-up `K7-CALL-IN-SUBSCRIPT-INDEX` |
| call beside a cast and a read in a return (`return rd_clock() - rkm->rkm_ts_enq;`) | 1 | `W43-RETURN-COMPOUND-CALL` (amended; AG3 found the cast case) |

27 of the 47 are refused on purpose and none of them is re-admitted here.

## 2. Rule 78: a pointer offset read as its subscript

**Probes on the base** (`tests/k7_residual_causes.rs` harness):
`if (str[1] == 's')` is `Safe`, `if (*(str + 1) == 's')` and
`if (*(1 + str) == 's')` are refused (branch condition);
`while (b->content[b->offset] <= 32)` is `Safe`,
`while ((b->content + b->offset)[0] <= 32)` refused (loop condition);
`int c = *(s + 2);` refused (indirect access) while `return *(s + 2);` is
`Safe` by an older route.

**Why it is sound.** C11 6.5.2.1p2 defines `E1[E2]` as `(*((E1)+(E2)))`;
6.5.6p8 makes `P + I` (either order) and `P - I` the pointers `I` elements
past and before `P`. So `*(P + I)` designates exactly the object `P[I]`
designates, `*(P - I)` the object `P[-I]`, and `(P + I)[K]` the object
`P[I + K]`. The reader (`Reader::path`) makes, for such a form, the very step
it makes for the subscript: the integer operand is read as an index
expression (refused where a subscript index is refused), the walk continues at
`P`, and whether the step reads a pointer from memory is decided on `P` as for
a subscript's base. The root, the steps, every per-hop obligation and every
later decision (rules 10t, 10w, 11j, 12h, 12j) are the subscript spelling's,
so nothing is decided that the subscript spelling would not decide
(`QCore.PointerOffsetPlace.the_decision_is_the_subscripts`), and every older
reading stands (`every_older_reading_stands`). The obligation travels with
the place: a read through a released pointer spelled as an offset is the same
`use_after_release` candidate, an unchecked allocation the same
`unknown_alias` (fixtures). No bound is claimed, as for every subscript.

**Refused** (the reader's older `None`; fixtures in
`near_misses_keep_their_refusal`): a difference of two pointers, a sum that is
not an object pointer, a function pointer or a `union`/`volatile`/`_Atomic`
pointer spelling, a non-integer operand, `I - P`, an index the reader does
not read (`*(s + (e - s))`), a pointer operand that is itself an offset
(`*((s + i) + j)`), an offset under `->` (`(b + 1)->n`), and the
compound-assignment route (`s += *(a + i)`), which does not ask this reader
(follow-up `K7-OFFSET-RESIDUAL-FORMS`).

**Fixtures** (`crates/qcore-c-gateway/tests/k7_residual_causes.rs`, 3 tests,
every row the status and the complete diagnostic set, every `Safe` result's
certificate rechecked): 11 offset/subscript pairs (conditions, loop, switch
arm, statement read, store, nested `*(*(v + i) + 1)`, member base, cast on the
pointer operand, the cJSON loop shape, a connective) each equal to its
subscript spelling and `Safe`, plus cJSON's exact loop condition over the
unit's `parse_buffer` typedef; three obligation pairs (released, unchecked,
live); four near misses. Unit tests in `pointer_offset_place.rs` (4) bind the
operand table to the Lean `offsetVectors`.

**Lean** `QCore.PointerOffsetPlace` (`propext`/`Quot.sound` only, no
placeholders): `deref_offset_is_subscript`, `deref_difference_is_subscript`,
`subscript_of_offset`, `offset_vectors_hold`,
`a_pointer_difference_is_refused`, `an_integer_minus_a_pointer_is_refused`,
`a_non_pointer_result_is_refused`, `an_other_operand_is_refused`,
`a_deref_offset_reads_as_the_subscript`,
`an_offset_subscript_reads_as_the_subscript`, `a_nested_offset_is_refused`,
`the_decision_is_the_subscripts` (over `QCore.TypedPlace.decidePlace`),
`a_refused_offset_keeps_the_refusal`, `a_bare_offset_is_not_a_place`,
`every_older_reading_stands`, `the_census_shapes_read`.

## 3. Shapes left, with their reasons

| Shape | Rows alone (11-24) | Why not here | Follow-up |
|---|---:|---|---|
| `<ctype.h>` classification | 4 (`line_protocol_escape`, `rtrim`, `rd_hexdump`, Monkey's `check_indent`) | `__ctype_b_loc` has no libc declaration (probe: `call to uncontracted function `__ctype_b_loc``); a read through its result needs a static-table result kind, a provider edit and its own reading | `K7-CTYPE-TABLE-READS` (new) |
| own array-element places | 4 | rule 12j's text calls a subscript a hop and refuses static roots; probes: `exp[i].in` on a block-scope array of records is refused, automatic or static, while a file-scope one is `Safe` | `K7-OWN-ARRAY-ELEMENT-PLACES` (new) |
| custody-free record value | 4 | CG14's discharge (a record return, a record actual at a borrowed formal) | `CG14-AGGREGATE-VALUE-POSITIONS` |
| union member address actual | 2 | the callee's mode for an interior address | `K7-UNION-MEMBER-ADDRESS-ACTUAL` (new) |
| `errno` return / actual | 2 | AG3's open shape; the actual's callee is cross-unit | `AG3-ERRNO-RETURN` |
| call in a subscript index | 1 | rules 14a/67 lower a call the caller owns; a return route does not | `K7-CALL-IN-SUBSCRIPT-INDEX` (new) |
| call, cast and read in a return | 1 | W43's compound call | `W43-RETURN-COMPOUND-CALL` |
| offset under `->`, compound assignment | 0 alone | not a subscript spelling / another reader | `K7-OFFSET-RESIDUAL-FORMS` (new) |

## 4. Measurement

[PREDICTION.md](PREDICTION.md): `cmt_get_statsd_type` and
`buffer_skip_whitespace` Unknown -> Safe on `current`, corpus and twelve-home;
no system or slice move. The card's unit-scoped reproduction of the 26
non-Monkey units holding a freed row ([scripts/census-run.sh](scripts/census-run.sh),
[artifacts/selection.json](artifacts/selection.json), base and tip binaries
built with `scripts/build.sh`) waited for an audit slot behind other cards'
measurements; its outcome is recorded in §6 when it ran. The integrator
measures every route with base `rev:3ca85c7a6`.

## 5. Shared-file hook points

- `crates/qcore-c-gateway/src/typed_access_path.rs`, `Reader::path`: in the
  `ArraySubscriptExpr` arm, the base is taken through
  `pointer_offset_place::offset_operands` (between
  `// K7-RESIDUAL-CAUSES (rule 78) hook` and `// End K7-RESIDUAL-CAUSES (rule
  78) hook`); and one new match arm before the `UnaryOperator "*"` arm,
  between the same two markers.
- `crates/qcore-c-gateway/src/lib.rs`: one `mod pointer_offset_place;` with
  its comment, after `mod union_bitfield;`.
- No edit to `legacy_c.rs`, `legacy_cfg_builder.rs`, `legacy_transfer.rs`,
  `legacy_classify.rs` or `typed_field_access.rs`.
- New files: `pointer_offset_place.rs`, `tests/k7_residual_causes.rs`,
  `docs/agents/rules/78.md`, `proof/QCore/PointerOffsetPlace.lean`,
  `proof/AuditPointerOffsetPlace.lean`; generated: `docs/agents/gateway.md`,
  `docs/agents/README.md`, `proof/QCore.lean`.

## 6. Gate and runs

Recorded in [gate.md](gate.md).
