# CG12-VOID-VIEW-READBACK: frozen prediction

Re-frozen from CG00's [`predictions/CG12.md`](../cg00-clang-gap-baseline/predictions/CG12.md)
(corpus `ae17152b…`, upper bounds) before any AFTER measurement of this card.

**Disclosure of order.** The rule was drafted and exercised on synthetic
fixtures (`q_*` contracts) while the BEFORE arbiter ran, so this file was
written after the first code existed. No AFTER measurement of any route had
been taken when it was committed (§1), and §2 was added from the BEFORE run
alone, again before any AFTER run.

## 1. Frozen before BEFORE finished

**Reading of CG00's input.** CG00 attributed a leaf to M05 when any path
of the leaf carries a cast whose operand is `void *` (`attribute.py`,
cause `cast-from-void`). Read line by line, the 76 leaves are almost all
**views of an untracked `void *`** (a formal `const void *` in a
comparator, a `void *` member of a formal record such as nanopb's
`field->pData`), not readbacks of a tracked member. So CG12 is two rules
(the report's §2):

1. **The view** (the corpus's leaves): an explicit object-pointer `BitCast`
   from `void *` inside a typed path, over the root or over a pointer
   loaded from a never-bound root.
2. **The readback** (the entry's text and CG00's control): a tracked
   identity read back out of a `void *` member of an admitted store base.

**Function-level prediction (upper bounds, as CG00's).**

| Measure | CG00 | This card |
|---|---:|---:|
| Unknown → Safe, CG12 alone | 1 (`rd_strcmp3`) | **1–4**: `rd_strcmp3`, plus up to 3 whose CG00 partner has since landed (`XXH_readLE32_align`, `XXH_readLE64_align` with CG06; `rd_kafka_assignor_topic_cmp` with CG10) |
| Unknown → Safe after CG07 merges | – | + `rd_list_get_int32` if its partner was CG07 |
| Safe → non-Safe | – | **0** expected; a pun refusal (premise 2) could move a Safe function whose `void *` member is stored as one type and read as another: none expected in the corpus |
| Unsafe added | – | **0** |

Leaves that stay by construction:

- `_RK_PTR(T, base, offset)` (`rd_kafka_anyconf_copy`, `_get0`): the view's
  operand is `char *` arithmetic, not `void *` (M15 and M22), 9 leaves;
- a view over a call result (`rd_list_elem(…)`, `rd_kafka_aborted_txns…`,
  the sticky assignor): no declared root, 6 leaves;
- `clamped = *(T *)field->pData = value;` (`pb_dec_varint`, 8 leaves): an
  assignment used as a value, not a statement form the typed reader lists;
- a view over a pointer loaded from a root the closure does not prove never
  bound (`view-over-link`).

**Slices.** System and tail-pause: 0 to +2 Safe; a view inside Fluent Bit
proper (`flb_input_chunk_set_limits`, `in->storage` viewed as
`struct flb_storage_input *`) is the only known M05 site there.

**Loop-owner census.** The 80 `W47-FIELDS-BY-MEMBER-VOID-ESCAPES` conversions
are followed by no readback of this card (they are not in one body's store
base); the prediction is 0 followed, with the reason per class measured.

## 2. From the BEFORE run

Added from the BEFORE run on the base `c374de96` (corpus `3a25c4e9…`,
608 / 10 / 4,216, reproducing the brief's baseline), before any AFTER run.
`scripts/rederive.py` re-derives CG00's list: all 33 functions are still
Unknown and every one of CG00's 76 M05 origins still carries its leaf.

| Class on the base | Functions | Prediction |
|---|---|---|
| M05 leaves only | `rd_strcmp3`, `XXH_readLE32_align`, `XXH_readLE64_align` | **Safe**: `rd_strcmp3` 85% (two views over formals as call actuals of a `return`); the two `XXH_*` 55% (the views are operands of a `return ?:`, CG06's lowering, and one is an actual of `XXH_swap32`) |
| one other leaf | `rd_kafka_assignor_topic_cmp` (IND at `:256`), `rd_list_cmp_trampoline` (uncontracted `rd_list_cmp_curr`), `rd_list_get_int32` (`rd_assert`, CG07), `rd_kafka_partition_internal_find_rack` (`bsearch`) | **Unknown**, M05 leaves lifted in the first three; `topic_cmp` keeps `:256` (its pointer is the borrowed alias, 80%) |
| several other leaves | the other 26 | **Unknown** (95%); M05 leaves lifted where the view is over a formal or a never-bound load |

Leaf-level (upper bound): of the 76 origins, 9 (`_RK_PTR`), 8
(`pb_dec_varint`'s chained assignment) and 6 (a view over a call result)
stay by construction, so **at most 53** lift; some lifted leaves are
replaced by the next statement's leaf (a borrowed alias with no identity is
unknown at a custody position). Corpus totals: **+1 to +3 Safe**, no
Safe → non-Safe, no Unsafe added (90%). System slice: **+0 to +1**; the
tail-pause slice: **+0** (85%): the one Fluent Bit M05 site
(`flb_input_chunk_set_limits`) is a view over a loaded `in->storage` whose
root must be proved never bound, inside a function with 9 other leaves.
