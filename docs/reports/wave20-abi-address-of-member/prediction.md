# Pre-registered prediction — card ABI-TYPEDEF-ADDRESS-OF-MEMBER (wave 20)

Written **before** the AFTER corpus run, after the single-unit probe and the
candidate census, so the measurement is allowed to contradict it.

## What the diff can reach at all

The widening changes exactly one predicate, `TypedAccessModel::addressable_field`
(through the new member set `non_pointer_fields`), whose only two consumers are
`aggregate_store::member_address_actuals` (the O04-B09B10 admission) and
`aggregate_store::insulated_bases` (its premise F). The classifier
`classify_specifiers` answers exactly what it answered before, so every other
premise keyed on it is untouched. Landing `qcore/msgpack/packer.qcore` adds one
contracted callee.

So the only statements that can move are:

1. `CALLEE(… & BASE -> FIELD …)` at a declared `Borrowed` position where
   `FIELD`'s declared type is a `union TAG`, or a typedef that *defines* its own
   `struct`/`union`/`enum` (or a chain of plain typedefs to one) — previously
   refused, now admitted;
2. `msgpack_packer_init(…)` statements, newly contracted;
3. whatever the removal of those `Unknown` instructions **unpoisons**
   downstream, which is where a status move can come from.

## The candidate census (`measure/candidates-base.txt`)

Over the pinned Fluent Bit and Monkey sources, the contracted callees carrying
an `& BASE -> FIELD` actual are:

| callee | statements |
|---|---|
| `mk_list_add` | 444 — member is `struct mk_list`, **already** admitted before this card; no movement expected |
| `pthread_mutex_lock` | 149 — member is `pthread_mutex_t`, **the new shape** |
| `pthread_mutex_unlock` | 218 — likewise |
| `malloc` | 1 — not a `Borrowed` position |

Only the subset inside the 185 audited units and inside a *projected* function
can move.

## The prediction

* **Diagnostic leaves removed**, of category `unsupported`, description
  `indirect memory access is outside the resource projection`, at
  `pthread_mutex_lock`/`pthread_mutex_unlock` statements whose actual is
  `& BASE -> FIELD` on a complete non-`union` `struct` base, plus the two
  `msgpack_packer_init` statements of `flb_input_chunk.c` (the production and
  the Monkey database's copy of the same unit).
* **Leaves added** are expected only in the *accusing* direction and only from
  unpoisoning: an `unknown_alias` or `leak` that the removed `Unknown`
  instruction had been hiding. Every one of those must be named.
* **The headline event is `flb_input_chunk_create`.** The single-unit probe
  (`measure/probe-step2.txt`) already measures it moving `Unknown` → **`Safe`**
  at the function level, 6 diagnostics → **0**, all six exits `Discharged`, with
  the success exit's three sites moving `Mixed` → `Escaped`. That is an
  admitting-direction move and is argued by name in the report.
* **The unit** `fluent-bit-chunk/flb_input_chunk.c` is provisional `Unsafe` at
  the baseline and is **not** predicted to move: its `Unsafe` is carried by other
  functions, not by the constructor, which was `Unknown`.
* **The leak pin (constraint 7)** is 272 at the wave-17 baseline. It is *not*
  predicted to hold: unpoisoning a statement can surface a leak the `Unknown`
  had hidden, in the accusing direction. Whatever it does, every moved leak leaf
  is attributed individually and none is absorbed.
* **Unit status moves** are possible in both directions and each must be argued.
  No unit is *promoted* to `Safe` by this card's own claim: a `Safe` unit
  requires full analysed coverage at the certificate gate, which none of these
  units has.
