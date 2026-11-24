# K5-LOCAL-ORIGIN-ALIASES: locals holding only loads from untracked objects

Card K5 of round 34 (2026-11-24), base `3ca85c7a6` (origin/main). Bundle K5 of
[CURRENT-ACCESS-ALIAS-CENSUS](../current-access-alias-census/README.md): the
ten local-origin alias groups. Owner follow-ups amended:
`W47-ALIAS-FLOW-RESIDUALS` (primary), `W47-ALIAS-TYPED-SCALAR-NAMES`,
`W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`. New gateway rule 77.

## 1. Re-census against the user's 2026-11-24 run

**Method.** The census was built on `7b726587b` (2026-11-07). The user's newest
`current` run (Fluent Bit `046accf6`, written 2026-11-24 09:58: 1,173 Safe /
3,750 Unknown / 11 Unsafe function rows) was read once, read-only, into
function rows ([scripts/extract.py](scripts/extract.py)).
[scripts/recensus.py](scripts/recensus.py) matches every `unknown_alias` leaf
of an Unknown row to the census's classified leaf of the same
`(file, function, variable, origin)` (the census's
`alias-leaves.jsonl.gz`; both runs read the same prepared tree, so origins
agree) and takes its W47 group; a leaf with no match is `new`. A row is freed
by K5 alone when every diagnostic it carries is a leaf of a K5 group.
Output: [artifacts/recensus.json](artifacts/recensus.json) (the 178 `new`
leaves in [artifacts/recensus-new-leaves.json](artifacts/recensus-new-leaves.json)).
[scripts/show.py](scripts/show.py) and [scripts/bodies.py](scripts/bodies.py)
print a row's leaves with their source lines and its body.

| K5 group | Leaves 11-07 | Rows 11-07 | Leaves 11-24 | Rows 11-24 | Freed by this group alone 11-07 / 11-24 |
|---|---:|---:|---:|---:|---:|
| `poisoned-by-refusal` | 1,281 | 462 | 1,114 | 423 | 0 / 1 |
| `non-pointer-name` | 1,072 | 347 | 518 | 200 | 7 / 6 |
| `derived-local-unguarded` | 1,037 | 319 | 1,027 | 312 | 2 / 2 |
| `borrowed-path-unguarded` | 875 | 354 | 860 | 350 | 9 / 10 |
| `uncontracted-result-unguarded` | 859 | 368 | 768 | 328 | 1 / 1 |
| `guarded-other` | 674 | 390 | 624 | 344 | 16 / 1 |
| `contracted-result-unguarded` | 587 | 206 | 482 | 167 | 1 / 1 |
| `address-taken` | 284 | 112 | 275 | 110 | 0 / 0 |
| `statement-expression-unguarded` | 98 | 59 | 38 | 26 | 2 / 3 |
| `field-store-owner` | 56 | 36 | 48 | 33 | 0 / 0 |
| **K5 total** | **6,823** | **1,654** | **5,754** | **1,452** | **43 / 29** (bundle) |

(The bundle column counts rows whose every blocker is in some K5 group; rows
are unit configurations, as the report counts them. 178 leaves of the 11-24
run match no census leaf; one row, `context_get_head`, is alias-only with
them: a loop cursor over a call result's member.)

**The 43 frozen functions.** 17 are now `Safe`, all through
AG2-UNRESOLVED-ALIAS (rule 62): the 15 `cmt_decode_msgpack.c` `unpack_*`
callbacks (S1, a formal copied into a local), `append_summary_metric_value`
(S3, a literal) and `LZ4F_optimalBSID` (S2, an enumerator). The other 26 are
still blocked by K5 leaves alone, with the same blocker as on 11-07; three
rows joined them (`LZ4F_getBlockSize`, `LZ4F_headerSize`,
`rd_kafka_anyconf_warn_deprecated`), for **29 rows**.

**What earlier cards refused on purpose** (nothing below is re-admitted):

- AG2-UNRESOLVED-ALIAS (rule 62) reads copies into automatic locals of a
  literal, an enumerator or a read-only unbound formal. It refused a
  file-scope object (S7: `r = rd_kafka_global_cnt;`, and so the decay of
  `rd_kafka_properties`) on rule 54's boundary (`AG2-STATIC-OBJECT-COPY`), an
  enumerator or literal into a *formal* (`LZ4F_getBlockSize`,
  `AG2-LITERAL-OR-CONSTANT-INTO-FORMAL`), a copy of a formal that is returned,
  stored, stepped or handed on (W4's binding signal), and a release of a
  `return_none` value (`__qcore_nested_value_N`). It filed the S4/S5 locals
  bound from a member read, a call or a loop cursor under
  `W47-ALIAS-FLOW-RESIDUALS` without a soundness objection: rule 62 simply
  does not read a value from a member. That is this card's shape.
- AG3-CONDITION-RETURN-CALLS kept W45's decision that an **interior address**
  (`&p->f`, an array member's decay) returned needs a derived result
  (`W45-RETURN-POINTER-RESULT-READS`, `AG3-RETURN-MEMBER-ADDRESS-DERIVED`);
  W45's own discharge says "a load through an untracked base [lowers] to an
  exit that escapes nothing". This card returns loads only.
- Rule 10b keeps the identity refusal of a null test on an unknown binding;
  rule 10a/11d's path premise reads a refused path as possibly handing a root
  to a custody position; rule 5j refuses an interior address returned; rule
  5l reads the intrusive-list `container_of`; rule 37 already gives
  `rd_calloc` and `rd_strdup` checked non-null results (W47-ALIAS-NONNULL's
  largest callees). This card narrows 10b for one binding shape (§3) and
  touches none of the others.

**Yield.** 29 rows remain, so the bundle is worth a card. By shape (the 29
rows and `context_get_head`, each row's leaves read against its body at
`046accf6`):

| Shape | Rows |
|---|---:|
| A pointer local loaded from a formal's member or element, null-tested or returned | 11 |
| ... of which the root is a call result (`fetch_metadata_kvlist_key`, `context_get_head`) or mentioned inside `likely()` (`rd_kafka_timers_next`) | 3 |
| A loaded dereference in a loop with no test (`pack_header`) | 1 |
| An interior address (`&ht->chain[i]`, `&s->iov[n]` then stepped) | 2 |
| `container_of` statement expressions (two release the element) | 3 |
| Non-pointer names: a static `int`, an enumerator into a formal, an automatic `unsigned char[16]` store, a `static const size_t[]` read, a `struct stat` copy, a descriptor formal closed | 6 |
| A file-scope array's decay walked by a cursor | 2 |
| Call results released, published or dereferenced (`mk_fifo_queue_get`, `mk_sched_get_thread_conf`, `mk_mem_alloc`, `rd_kafka_topic_partition_get_private` stored through), a formal handed on (`LZ4F_headerSize`) | 5 |
