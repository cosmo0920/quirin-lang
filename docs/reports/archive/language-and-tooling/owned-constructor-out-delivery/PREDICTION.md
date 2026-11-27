# OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY: prediction

Committed before the implementation commit. Base `2c60986b0` (main). USER
DECISION 2026-11-23: picked as one of the "soundness first" set of four
implementation cards.

## What was known when this was written

No implementation was measured. Known from reading the base and from probe
fixtures run against it (`private/ocod/probe1.log`, not committed):

* **The out-parameter delivery summary already exists.** W4
  (`legacy_summary.rs`, `infer_cfg_summary`) marks every formal the body
  stores through (`out_parameter_candidates`), probes with that slot
  sanctioned, and reads `ResourceParameterMode::Out { nullable }` off the exit
  observations (`out_mode`): every reachable exit must hold a fresh, live,
  non-null heap site in the slot (or the null constant, which makes it
  `nullable`); an exit that leaves it `Unwritten` or `Ambiguous`, holds a
  possibly-null value, or a released or escaped site, yields no summary. The
  validated `Out` summary is published (`InferredValidated`), and a caller's
  `fill(&p)` binds the delivered site to `p`: the probe `void fill(void **out)
  { *out = q_alloc(); }` is `Safe`, its caller that releases `p` is `Safe`,
  and the caller that ignores `p` is `Unsafe` (a leak in the caller). Lean:
  `QCore.CustodySummary` (`outSlotMatches`,
  `validated_summary_out_slots_match`).
* **What the four rows hit is the refusal of a body with no such summary.**
  When no validated summary sanctions `*out`, the row's own analysis runs with
  no out slot, and `legacy_transfer.rs` reports a call storing an owned result
  through the unsanctioned `*NAME` as `discarded owned result of resource API
  \`F\``, the text it shares with a real discard (`f();`). The result was not
  discarded; it was stored through a slot no out-parameter contract describes.
  The sibling store of a variable (`*out = p;`) already says so: `store
  through \`*out\` without an out-parameter contract`.
* **None of the four rows is an every-path delivery.**
  `rd_kafka_assignment` (rdkafka_subscription.c:337) and
  `rd_kafka_subscription` (:365) first store `rko->rko_u.….partitions`
  (another object's member) through the slot on every path that reaches the
  op, then store the fresh list only when that is NULL and `err` is 0, and
  leave the slot unwritten on the two early returns.
  `rd_kafka_handle_OffsetFetch` (rdkafka_request.c:1223) is an in-out slot:
  `if (!*offsets) *offsets = rd_kafka_topic_partition_list_new(16);` keeps the
  caller's list when it has one. `ut_create_topic_partition_lists`
  (rdkafka_sticky_assignor.c:2235) stores into an element of a frame-allocated
  array (`lists[i] = …`), not through a formal. No sound `Out` summary exists
  for any of them, so each stays `Unknown`.

## Planned change

No new effect mode. The `Out` summary is the delivery reading the discharge
asks for, and it stays exactly as it is. The change names the refusal: a call
whose owned result is stored through a `*NAME` slot that no out-parameter
contract or validated summary sanctions is refused as `owned result of
resource API \`F\` stored through \`*NAME\` without an out-parameter
contract`, still `unsupported`, so still `Unknown`. A real discard keeps its
text, and so does a store into an unnamed destination (`lists[i] = …`).

## Predicted movements (opt-in route, base vs tip)

* **No status move.** The changed arm is already a refusal; its kind and its
  state poisoning are unchanged. Safe, Pending, Unknown and Unsafe totals
  equal to the base's.
* **Leaf text only.** Every `discarded owned result` leaf at a `*NAME =
  CALL` store becomes the new text. Of the four rows named by the entry:
  * `rdkafka_subscription.c::rd_kafka_assignment`: leaf text changes, stays
    `Unknown`;
  * `rdkafka_subscription.c::rd_kafka_subscription`: leaf text changes, stays
    `Unknown`;
  * `rdkafka_request.c::rd_kafka_handle_OffsetFetch`: leaf text changes,
    stays `Unknown`;
  * `rdkafka_sticky_assignor.c::ut_create_topic_partition_lists`: unchanged
    (`lists[i] = …` is not a `*NAME` store), stays `Unknown`.
* Other rows with the same shape (the Fluent Bit tree spells `*NAME = CALL (`
  1,253 times by a rough `grep`, most with an uncontracted or scalar callee):
  a few to a few tens of rows change one or more leaves' text, none moves.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as on the opt-in
route: no status move, leaf text only where a `*NAME = CALL` store of an
owned result is refused.
