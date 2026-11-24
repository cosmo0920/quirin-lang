# K7-RESIDUAL-CAUSES: prediction

Frozen before the rule-78 implementation commit and before any measurement of
an implementation build. The integrator measures with base `rev:3ca85c7a6`.
The forecast reads the re-census of the user's 2026-11-24 `current` run
([README](README.md) §1) and unit probes on the card's test harness
(`crates/qcore-c-gateway/tests/k7_residual_causes.rs`, real Clang, the
synthetic `k_*`/`d_*` contracts). The unit-scoped reproduction of the
production rows (`scripts/census-run.sh`) was still queued for an audit slot
when this file was committed.

## Mechanism and what it can move

**Rule 78** reads `*(P + I)`, `*(I + P)`, `*(P - I)` and `(P + I)[K]` in the
typed access-path reader as the subscripts C11 6.5.2.1p2 defines them to be
(`P[I]`, `P[-I]`, `P[I + K]`): the same root, steps, per-hop obligations and
decisions as the subscript spelling. It only adds readings where the reader
answered no place; every older reading stands
(`QCore.PointerOffsetPlace.every_older_reading_stands`). A row moves only when
every remaining leaf of it is such a place.

On `current` the census tag `pointer-arithmetic` (any `+`/`-` on a pointer in
an access path, a superset of what rule 78 reads) holds 33 leaves on 15
Unknown rows; it frees two rows alone:

* `cmt_get_statsd_type` (cmetrics `cmt_decode_statsd.c:505`,
  `if (*(str + 1) == 's')`, its only leaf). The probe replica of the whole
  function is `Safe` with the rule and refused without it.
* `buffer_skip_whitespace` (librdkafka `cJSON.c:1097`, the loop condition
  `... && (((buffer)->content + (buffer)->offset)[0] <= 32)`, its only leaf).
  The loop condition alone, over the unit's own `parse_buffer` typedef, is
  `Safe` with the rule and refused without it. The harness refuses the whole
  function also in its subscript spelling (a branch the production row does
  not refuse), so the forecast for this row rests on the production row
  having no other leaf.

## Predicted moves

| Route | Function | Today | Predicted |
|---|---|---|---|
| `current` (user run) | `cmt_get_statsd_type` | Unknown | Safe |
| `current` (user run) | `buffer_skip_whitespace` | Unknown | Safe |
| corpus (eight-home) | `cmt_get_statsd_type` | Unknown | Safe |
| corpus (eight-home) | `buffer_skip_whitespace` | Unknown | Safe |
| twelve-home | the same two | Unknown | Safe (the corpus inputs) |
| system, slice | — | — | no move |

No `Safe -> non-Safe` move on any route: rule 78 adds readings only. No new
`Unsafe` row is forecast; a read through a released pointer spelled as an
offset is now the same `use_after_release` candidate as its subscript
spelling, which is the only way an `Unsafe` could appear. A further corpus
move of the same class (a function whose only remaining leaves are offset
places) is possible but not forecast by name.

## Predicted leaf changes

* Corpus and `current`: 10 to 33 IND/COND/RET leaves removed where the
  refused place is an offset under `*` or `[]` (the census tag's 33 leaves on
  `current` is the ceiling there), each possibly exchanged for an
  `unknown_alias` at the read (the subscript's base obligation on a bound
  base: a refusal, never an accusation) or for another refusal of the same
  statement.
* Zero change of `leak`, `double_release`, `summary_conflict` counts.

```measure-prediction
route corpus
move cmt_get_statsd_type Unknown -> Safe
move buffer_skip_whitespace Unknown -> Safe
delta Safe +2
route twelve-home
move cmt_get_statsd_type Unknown -> Safe
move buffer_skip_whitespace Unknown -> Safe
delta Safe +2
route system
none
route slice
none
```

## Expected effect on `current`

Two rows Unknown -> Safe (`cmt_get_statsd_type`, `buffer_skip_whitespace`),
1,173 -> 1,175 Safe. The census's band for the whole bundle (10 to 20 Safe)
is not reached: of the 47 rows the bundle frees alone on 11-24, 27 are
refused on purpose by earlier cards (pointer-valued returns, open indirect
call sets, list-helper custody, a pointer store through a union, records
with pointer members, cast views of a tracked root), and 18 of the other 20
need readings this card leaves as named follow-ups ([README](README.md) §3).
