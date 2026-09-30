# CG15-STORAGE-DURATION-AND-VOLATILE: frozen prediction

Frozen on base `c374de96` (= local `main` = `origin/main`) after the BEFORE
corpus arbiter finished and **before any AFTER measurement**. It was not
frozen before the first line of code: the card's route-G hooks were written
and exercised on synthetic fixtures (never on the corpus) while the BEFORE run
was in flight, to learn which refusal each corpus shape actually meets. No
corpus or slice figure of the change had been produced when this file was
committed.

BEFORE (`private/cg15/before`, exit 0) reproduces the round's baseline byte
for byte: `resource-diagnostics.json` `3a25c4e9…`, 608 Safe / 10 provisional
Unsafe / 4,216 Unknown. System and tail-pause slice BEFORE runs exited 0.

## The inherited list, re-derived

CG00's list is [`../cg00-clang-gap-baseline/predictions/CG15.md`](../cg00-clang-gap-baseline/predictions/CG15.md),
measured on `ae17152b…`. [`scripts/population.py`](scripts/population.py)
(CG13's method: a current leaf with CG00's unit, function, internal line and
normalised description keeps CG00's attribution) re-derives it on this base:

| Measure | CG00 (`ae17152b`) | Re-derived (`3a25c4e9`) |
|---|---:|---:|
| M24 leaves / Unknown functions | 132 / 67 | **132 / 67** (65 declarations, 67 path leaves) |
| M23 leaves / Unknown functions | 1 / 1 | **1 / 1** |
| freed alone, alias allowed / strict | 7 / 7 | **8 / 8** |
| freed with one other card | 6 | 8 (CG10 2, CG07 2, callee closure 2, CG13 1, CG02 1) |

All seven of CG00's functions are still in the list. The eighth,
`rd_kafka_features2str`, lost its other blocker to an earlier card.

## Amendments, with reasons

Reading the eight functions' leaves (`scripts/leaves_of.py`):

1. **The five `rd_kafka_*_name` table readers** (`static const char *names[] =
   {…}; … return names[x];`) need the declaration to be no event and the
   table's constant initializer not to count as a binding in either
   never-bound closure. Both are in scope. Predicted **Safe**.
2. **`rd_kafka_broker_name`** returns and formats into a thread-local
   `char ret[4][N]` element (`ret[reti]`): the address of a thread-duration
   object, no identity. Predicted **Safe** if `rd_snprintf`'s position
   admits a borrowed actual (its other actuals are already read).
3. **`rd_kafka_fetch_pos2str`**: the same shape, but its `rd_snprintf`
   also takes `rd_kafka_offset2str(…)` as an actual; a nested call the
   M24 leaf hid may keep its own refusal (CG05). **Uncertain.**
4. **`rd_kafka_features2str`**: `ret[reti] + of` is pointer arithmetic on
   the address (CG16's, refused by the brief's scope) and `*ret[reti] =
   '\0'` a store through the decayed address. Predicted **stays Unknown**.
5. **Soundness repairs this card makes** (the fixtures were wrong `Safe` on
   the base): a block-scope `static` pointer's initializer was re-applied on
   every call (`static void *cache = 0; if (cache) free(cache);` was `Safe`
   by pruning), and a `volatile` object carried bindings and null facts
   (`void * volatile p = alloc(); if (p) free(p);` was `Safe`). Any corpus
   function whose `Safe` rested on either moves to non-Safe; each is a named
   correction.

## Predicted outcome

| Measure | Prediction |
|---|---|
| BEFORE reproduces `3a25c4e9…` byte-identically | **held** (measured before freezing) |
| Unknown → Safe, corpus | **6** (band 3–10); upper bound 8 |
| of those, in the re-derived list of 8 | ≥ 5 |
| expected among them | the five `rd_kafka_*_name`, `rd_kafka_broker_name` |
| Safe → non-Safe, corpus | **0** (band 0–3); any is a named correction (items 5) |
| provisional Unsafe added | 0 (band 0–3): a static declaration that no longer poisons the path can expose a leak the refusal hid; each classified by reading |
| M24 leaves removed | 70–110 of 132 (all 65 declarations that are wholly static-duration; the path leaves whose only cause was the static root) |
| M23 leaf | the one leaf (`rd_kafka_desensitize_str`, a `volatile char *` cursor) stays: its function has four other blockers |
| system slice (`current`, 41 / 2 / 191) | 0–3 functions move, none Safe → non-Safe unless a correction |
| tail-pause slice (28 / 1 / 240 / 1 Pending) | 0–2 functions move |

Confidence in the point estimate is moderate: the table readers are a single
shape the fixtures already decide, and the thread-local formatters depend on
contracts outside this card.
