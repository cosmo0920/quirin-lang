# LATER-DECLARATOR-INITIALISER-UNREAD: prediction

Committed before the implementation commit. Base `2c60986b0` (main). USER
DECISION 2026-11-23: one of the "soundness first" four implementation cards.

## What was known when this was written

No implementation was measured on the corpus. Known from reading the base and
from the base's own opt-in run (`private/ldi/run-base`, 1,172 / 2 / 3,622 / 8;
access rows 2,118, alone 215, leaves 8,992):

* The statement classifier reads one assignment per statement, its first
  top-level `=`. In `T *a = X, *b = Y;` the value `X , * b = Y` holds `* b`,
  so the lexical gate refuses the whole declaration as `indirect memory access
  is outside the resource projection`, and no name it declares is bound.
* [scripts/candidates.py](scripts/candidates.py) over the base report (a rough
  regex over the unpreprocessed source: an Unknown row with an access leaf
  whose body holds `T *a = X, *b = Y;`-shaped declarations) finds 29 rows; in
  six the access leaves are the row's only leaves.

## Planned change

Read a declaration of several init-declarators, with at least one pointer
declarator, as its declarations written one per statement, when the lexical
splitter reads every declarator exactly, Clang shows one `DeclStmt` of
automatic objects with those names, and each declarator is admitted:
uninitialised and evaluating nothing (no event), a pointer initialiser that
spells no cast other than the null constant, or a scalar numeric literal.
Otherwise the old reading (and its refusal) stays.

## Predicted movements (opt-in route, base vs tip)

* **Unknown -> Safe**, where the declaration was the only obstacle and its
  initialisers are plain formals or member reads:
  `rd_kafka_metadata_cache_entry_by_id_cmp`, `rd_kafka_topic_partition_list_cmp`,
  `ConsumerPair_cmp`, `sort_by_map_elem_val_toppar_list_cnt`. Each needs the
  rest of its body admitted once the names are bound, which the base did not
  test (the refused declaration stood first), so any of them may stay Unknown
  on a newly visible leaf.
* **No move** for `rd_kafka_topic_cmp_rkt` (its initialisers are `(void *)_a`,
  a cast that is not the null constant: the decision keeps the old reading)
  and `rd_kafka_sw_str_sanitize_inplace` (pointer increments and stores
  through `d` follow).
* **ConsumerGroupHeartbeat** (`rd_kafka_mock_handle_ConsumerGroupHeartbeat`,
  99 leaves) stays Unknown; its declaration's access leaf and the
  `existing_assignment` nullable and identity leaves go (the `RD_IF_FREE`
  guard is then decided).
* Leaf-only changes in the other candidate rows: fewer access leaves at their
  declarations, possibly new leaves later in the body where a bound name now
  reaches a statement the refusal used to hide.
* Possible, not expected by name: **Unknown -> Unsafe** where a refused
  declaration hid a leak or a use after release (the card's own fixtures show
  the one-per-statement spelling's verdict can be Unsafe, provisional), and
  further Unknown -> Safe rows the rough census missed (macro-expanded
  declarations).

No Unknown -> Safe move may rest on anything but the one-per-statement
reading; each is checked against its contract dependencies.

```measure-prediction
route corpus
move rd_kafka_metadata_cache_entry_by_id_cmp Unknown -> Safe
move rd_kafka_topic_partition_list_cmp Unknown -> Safe
move ConsumerPair_cmp Unknown -> Safe
move sort_by_map_elem_val_toppar_list_cnt Unknown -> Safe
delta Safe +4
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as on the opt-in
route for the librdkafka rows; the system and tail-pause slice routes are not
expected to move. Any move is reported, named or not.
