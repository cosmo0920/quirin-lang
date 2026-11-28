# OUT-SLOT-MEMBER-THEN-FRESH: prediction

The first prediction was committed after the probes (`6aa742484`) and before
any semantic change, for the stop with options: no movement. It is kept as
section 1. Section 2 is the amendment for the implementation and is committed
before the semantic commit. Base `287dce8ed` (main, r86).

## 1. The stop (USER DECISION 2026-11-28, option (a))

The custody claim held in the bodies, but the vocabulary could not state it,
so only pins and records landed. Prediction: **none** on every route. This
was measured and held (base and tip `6aa742484` both 1,209 / 2 / 3,586 / 7,
no row moved, no leaf changed).

## 2. The implementation (USER DECISION 2026-11-28, option (a1)+(b))

### Planned change

* **(a1)** `resource_member_owned(result, TAG.PATH) when FIELD == VALUE`
  (C04.19.4.1): an owned member of a result under its decided discriminant.
  A result state is admitted on a nullable result beside an owned member,
  never with the liveness part. Declared for `rd_kafka_op_req2`:
  `resource_discriminant_set(result, rd_kafka_op_s.rko_type, type,
  1073741824)` and owned `rko_u.subscribe.topics` / `rko_u.assign.partitions`
  under 23 / 24 with the reply flag.
* **Rule 96**: the read `D = B->M;` that the next statement nulls moves the
  member's object to `D`. A link the body holds alone gives an alias. An
  unspent owned-member state gives a fresh possibly-null site. Everything
  else is the read's own lowering. A store over an unspent owned member is
  refused.
* **Rule 74l (b)**: rule 85's null entry alone publishes a `null_only`
  summary, read only into a slot the caller knows null.

### What was known when this was written

From the fixtures over the synthetic `c_*` op, at the uncommitted tip:

* a stated move-out released is `Safe` and dropped leaks;
* a frame-linked move-out released is `Safe` (it was `Unknown` at base);
* a mode-(b) callee's known-null caller is `Safe`, and every other caller is
  refused by name.

The upstream bodies store the member into `*partitions` / `*topics`
**before** `rd_kafka_op_destroy(rko)`. The op actual may designate caller
memory, so rule 11r(8) poisons the bound slot at the destroy and no summary
validates (pinned: `a_slot_bound_before_the_destroy_stays_refused`). Their
`!*out && !err` test and the indirect accesses (`rkcg->rkcg_ops`,
`rko->rko_err`) block them besides.

### Predicted movements (opt-in route, base vs tip)

* **The two named rows stay `Unknown`.** Their leaves may gain the rule 96
  overwrite refusal at the null store (`store into
  `rko->rko_u.assign.partitions`, which still holds an owned member the
  store would lose`): the move-out into the unsanctioned slot reads the
  fallback, so the owned state is unspent when the null store comes. Their
  base leaves otherwise stay.
* **`rd_kafka_share_subscription` stays `Unknown`.** It forwards its slot,
  and no compiled caller passes a known-null slot to either function.
* **Elsewhere: zero to a few rows move.**
  * Every `rd_kafka_op_req2(…, CONSTANT)` result now carries a decided
    discriminant. A later `rd_kafka_op_destroy` of it reads the decided arm
    where the run's stores keep `rko_type` (rule 12o). An `Unknown` row whose
    only blocker was `arm-undecided` could move, either way.
  * A frame-linked read-and-null (`x = o->f; o->f = NULL;` after
    `o->f = y;`) elsewhere could move `Unknown -> Safe`, or `-> Unsafe` where
    the moved child really leaks.

  Expected: none, since both shapes are rare in the corpus and their rows
  carry other blockers.
* Leaf text: the overwrite refusal on any row that nulls an owned member of a
  req2 reply without moving it out.

The block states the expectation (none). Any move the measurement finds is
reported against it and justified row by row.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there: at most the moves
above.
