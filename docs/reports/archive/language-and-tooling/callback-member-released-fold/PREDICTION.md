# CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD: prediction

Committed before the implementation commit. Base `2c60986b0` (main). USER
DECISION 2026-11-23: picked in the "soundness first" set of four cards.

## What was known when this was written

No implementation was measured. Known from reading the base:

* `legacy_callback_route::callback_operands` collects a candidate's
  `resource_owned(n)` requirement on a parameter actual (`require`) for three
  clause forms: the synchronous invocation (`resource_callback_invoked`,
  `_or_default`), the table dispatch and the member invocation (plain and
  slot). The registry invocation and a member invocation deferred to a
  constructed state refuse any requirement already (`callback-effectful`).
* `legacy_retained_by.rs` folds that requirement only at a `Borrowed`
  position (into `resource_owned(n)`) and at a released slot object (r60).
  Everywhere else it is dropped. The loader restricts no actual's mode, so a
  nullable owned, consumed or released (plain member form) position, or a
  stored, removed or out one, drops a non-null requirement.
* The fixture (`crates/qcore-c-gateway/tests/callback_requirement_fold.rs`,
  base run `evidence/base-fixture.log`) shows it: at base a member invocation
  through `resource_released(o, nullable)`, a member actual at
  `resource_owned(o, nullable)` and a synchronous invocation at
  `resource_owned(o, nullable)` are `Safe` with a possibly null `o` and a
  candidate requiring it non-null.
* Census of the shipped declarations (`evidence/census.py`, 624 invocation
  clauses in 49 declarations): parameter actuals that are not borrowed are
  `rd_kafka_op_destroy`'s `rko` (released nonnull), `rd_kafka_background_queue_serve`'s
  `rko` (consumed nonnull), `rd_kafka_poll_cb`'s `rko` (owned nonnull) and
  `EVP_MD_CTX_free`'s `ctx` (released nullable, a slot object: r60's fold).
  None is at a nullable position outside r60's fold, and none is at an out,
  stored or removed position.

## Planned change

Fold the requirement at every position whose mode is a held object or null
(owned, consumed, released, stored: the nullable flag strengthened by `&&`)
for every clause form, and refuse the call (`callback-requirement-unfolded`)
when a requirement lands on an out or removed position, which no fold
states. Borrowed keeps its reading; r60's slot fold becomes the general one.

## Predicted movements

No shipped declaration puts a requirement where the fold changes anything:
every non-borrowed parameter actual is non-null or r60's slot object, so the
fold is the identity on shipped modes and the refusal is never reached.

* **Opt-in `current --monkey-linked`** over Fluent Bit 046accf6, base
  `2c60986b0` vs tip: **0 status moves, 0 content changes**; totals equal.
  Rows differ at most in identity digests (the revision in
  `evidence_version`).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); predicted none on all three, since
no pinned module carries a requirement at a position the fold changes. The
riskiest move would be a `Safe` → `Unknown` row through a candidate the
census cannot see (a run candidate requiring an actual non-null at a nullable
position of a contract this census does not list); any such move is
unpredicted by name and is reported.
