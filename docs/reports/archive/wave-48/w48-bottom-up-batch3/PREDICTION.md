# Prediction — frozen before semantic implementation

Base: `198f6fc4349d3a47591a2378835ae9286859d918`. Preparation commit
`5885a9e3d` changes documentation only. The exact-base release build and a fresh
configured corpus reproduction are required; frozen detach-lookup-membership
evidence supplies the initial census, not a substitute for that reproduction.

## Selection and dependency readiness

The supplied after-plan has 2,222 candidate bodies, 362 hypothetical all-body
ceiling callers and 256 single-body waits. Ranked by score, `rd_kafka_log0`
is first (3/1), followed by `msg_write` and `sched_thread_cleanup` (2/1).
`rd_kafka_log0` itself has just the uncontracted `rd_kafka_log_buf` call, but the
callee has eight leaves: three indirect stores, two unresolved `rko` uses,
uncontracted `rd_kafka_q_enq`, callback-member condition and callback dispatch.
The source creates an op, allocates its log string, transfers the op to a queue,
or invokes `conf->log_cb`. The existing typed queue declaration is not a
resource-projection summary of this operation. Closing it requires the queue's
ownership/effect and callback decisions (protected class 1), not a neutral
logging declaration. Its three cross-unit callers also need an explicitly
selected external home under rule 4c's existing actual-link/type/calling-
convention/interposition premise. This card adds no such home or assumption.

`msg_write` is the highest-yield bounded ready candidate: its sole leaf is
`errno == EAGAIN`, preprocessed to `(*__errno_location()) == 11`. The existing
libc contract supplies no custody result and has no parameters; this unit's
own declaration supplies exactly one pointer level over a scalar. The existing
CONSTRUCTOR-RESIDUALS rule already reads that scalar as a nested actual. Extend
only the reading position, keeping the ordinary call node and all its existing
checks, followed by a two-way unknown scalar decision. Add no neutrality
contract or opaque body-shape assumption. The explanatory `nested_call_admission`
premise must explicitly describe the new checked position; its result/effects
remain declared assumptions, never source-validated.

`sched_thread_cleanup` instead waits on `mk_http_thread_destroy`, whose five
leaves include TLS get/set, unresolved `libco_params`, unpublished list unlink
and `mk_thread_destroy`. It is not the equally cheap closure its score suggests.

## Bounded implementation and predicted movement

Admit only a direct external zero-argument call whose in-force contract has
no parameters, result None and no container effect, whose complete unit's
unambiguous declaration returns a scalar pointer. Read exactly `*F()` or
`(*F())` compared by `==` or `!=` with one unsuffixed decimal integer literal.
No pointer-valued or aggregate read, derived/owned result, actual, local body,
write, address, nested read, richer residual, or value refinement is added.
The call still uses the ordinary instantiator and CFG; short-circuit and loop
edges retain its evaluation frequency. A new audited Lean model must match the
admission and scalar observation preserving custody, rather than assume the
whole caller Safe.

Expected corpus: `msg_write` and same-unit `mk_fifo_send` become Safe, with
`mk_mq_send` conditional on a new `mk_fifo_send` home in this card's copy of the
existing link map. At most these three are targeted; incidental exact-shape
errno reads may remove leaves elsewhere. Record every actual movement, including
newly exposed alias/effect leaves. Prediction: 0–3 targeted Safe gains, no Safe
regression, no new validated Unsafe or authority promotion. Baseline is expected
963 Safe / 9 provisional Unsafe / 3,862 Unknown (4,834 rows). System and slice
statuses, null and runtime projections are expected unchanged except explicit
contract-text/digest dependencies; an incidental exact-shape scalar condition
may remove an unsupported leaf. Execute and attribute every difference.

Full standing and provider gates, before/after corpus/system/slice comparisons,
null/runtime projections, source/replay and negative controls, refreshed ranking,
one increment and current ledger/residual updates close the card. Frozen prior
reports, source pins, alias/lifetime obligations and semantic boundary28 remain.
