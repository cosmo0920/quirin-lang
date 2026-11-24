# K3-POINTER-VALUED-STORES: prediction

Committed before the implementation commit. Base `3ca85c7a6` (origin/main at
the brief). Round 34 card K3 (USER DECISION 2026-11-24, "Let's get started out
of four"); owner `CG11-UNRESOLVED-OWNER-POINTER-STORES` (+
`CG11-UNREAD-POINTER-PLACES`), gateway rule 10v.

## What was known when this was written

From the re-census ([README.md](README.md) section 1, `evidence/`), on the
user's `current` report of 2026-11-24 (1,173 Safe / 3,750 Unknown / 11 Unsafe):

* the bundle still carries 2,167 `pointer-valued-store` leaves on 845 Unknown
  rows and frees 49 rows alone (53 at 11-07);
* the 49 are, by statement shape, mostly owned results stored into a formal's
  member (`MEMBER-EFFECT-STORES`, rule 38i names it), formal or local pointers
  stored into a member (Q36 example C, decision class 1), and calls whose
  result custody the gateway does not read; those are refused on purpose or
  owned elsewhere and are not read by this card;
* the one sub-shape this card reads, pointer arithmetic over a pointer the
  destination's own object already holds (`P = P ± E`, `R->a = R->b ± E`),
  is 18 leaves on 11 Unknown rows (lexical, `evidence/derived-yield-1124.json`),
  and is the whole blocker of 3 rows: nanopb `buf_read`
  (`stream->state = (pb_byte_t *) stream->state + count`), snappy
  `writer_set_expected_length` (`w->op_limit = w->op + len`) and Monkey
  `mk_iov_consume` (`mk_io->io[i].iov_base = (uint8_t *) mk_io->io[i].iov_base + bytes`).

## Planned change

Rule 10v reads `P = Q ± E` (casts pointer-to-pointer only, `E` an integer) as
the new neutral value class `same-object-derived` when `Q` is a pointer read of
the same place as `P` (same side-effect-free spelling) or of a pointer member
of `P`'s own object (both CG10 places root-only over one root). The owner then
decides exactly as for every other neutral value (untracked admits; tracked or
unresolved refuses), the read and the offset carry their own per-hop
obligations, and the body still publishes no summary.

## Predicted movements

* **Unknown -> Safe**: `buf_read`, `writer_set_expected_length`,
  `mk_iov_consume`, each where its only leaf is the derived store and its
  owner is an untracked formal. Each may stay Unknown if its owner chain does
  not resolve (an unresolved owner keeps the refusal) or the read obligation
  is refused.
* **Leaf-only**: the other 8 touched rows (`LZ4F_compressUpdateImpl`,
  `LZ4F_decompress`, `LZ4F_flush`, `LZ4_saveDictHC`, `load_descriptor_values`,
  `decode_static_field`, `encode_array`, `refill_tag`) lose at most their
  derived-store leaves and stay Unknown.
* **No** Safe -> non-Safe move, no Unknown -> Unsafe move, no leak or
  violation added: the reading only replaces a refusal with an access whose
  value carries no identity the projection tracks.
* **Callers**: no summary is published, so no caller moves.

```measure-prediction
route corpus
move buf_read Unknown -> Safe
move writer_set_expected_length Unknown -> Safe
move mk_iov_consume Unknown -> Safe
delta Safe +3
route system
none
route slice
none
```

The corpus rows are expected only where the pinned corpus (Fluent Bit
`66910c10`, its librdkafka and Monkey) holds the same bodies; this card did not
read the pinned trees, so a missing corpus move with the same function Safe on
`current` is a corpus-source difference, not a miss of the rule. Twelve-home:
the same three where its homes include those units, nothing else. The
integrator's `current` rerun: at most 3 rows Unknown -> Safe, about 18
`pointer-valued-store` leaves removed, every unit status unchanged.
