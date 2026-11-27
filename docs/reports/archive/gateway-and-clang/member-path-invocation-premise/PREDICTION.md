# MEMBER-PATH-INVOCATION-PREMISE: prediction

Committed after the census and the probe fixture, and before the semantic
commit. Base `98c51878f` (main). USER DECISION 2026-11-26: "launch the
MEMBER-PATH-INVOCATION-PREMISE card".

## What was known when this was written

* Rule 30's table (C04.19.6, user decision 2026-10-15) composes a
  candidate's `#released` or `#moved` state at a linked path as `gone` under
  every arm join, `some` and `none` included. Every invocation clause calls
  its callback zero or more times, so under `some` or `none` the reading
  rests on a candidate running
  (`QCore.CallbackTableMemberPathStates.the_gone_reading_assumes_an_invocation`).
* The probe (`crates/qcore-c-gateway/tests/member_path_invocation_premise.rs`,
  run at base by [`scripts/probe-base.sh`](scripts/probe-base.sh)): over
  invokers that may call no candidate (an early return, a call under a
  condition, a loop that may run zero times, `?:`, `&&`, a `goto` past the
  call, a `switch` case without the call, no body at all), all three forms
  read the payload taken and the row `Safe` at base, although on the
  path with no call the item leaks.
* The census ([`scripts/census.py`](scripts/census.py),
  [`evidence/census.txt`](evidence/census.txt)): the shipped declarations
  stating `#released` or `#moved` are six librdkafka `rko_op_cb` candidates
  (binding `rd_kafka_op_s` at their third formal) and the Fluent Bit
  teardown destroyers (direct calls, which run once and are not an
  invocation form). The only shipped invocations handing a caller parameter
  at a position such a candidate states are `rd_kafka_op_destroy`'s and
  `rd_kafka_poll_cb`'s `rko_op_cb` member invocations (position 2, `rko`).
  Both C bodies call the member only under a condition
  (`rdkafka_op.c:349`: `if ((rko->rko_type & RD_KAFKA_OP_CB) &&
  rko->rko_op_cb)`; `rdkafka.c:5822` returns before any call, and the
  `rd_kafka_op_call` sites at `:6030`, `:6044`, `:6083` are switch cases),
  and the call is inside `rd_kafka_op_call` for the latter (a helper the
  check does not follow). So neither is established, in any unit.

## Planned change

A new module `member_path_invocation.rs`: a must-call reading of the
invoking declaration's Clang body (rule 30's control-flow graph, a forward
data flow with `and` as the meet). Where it is not established, each
candidate state `V` reads `{V, #kept}`; a reading that decided only by the
invocation having run refuses `member-path-invocation-unestablished`. Under
an arm releasing the path on every arm nothing changes. Rules 30 and 13v
are amended, C04.19.6 too; `QCore.MemberPathInvocationPremise` proves the
reading.

## Predicted movements

* **Opt-in `current --monkey-linked`** over Fluent Bit 046accf6, base
  `98c51878f` against the tip: **0 status moves**, with equal totals. A
  `gone` reading at `rd_kafka_op_destroy` or `rd_kafka_poll_cb` needs every
  candidate of `rko_op_cb` in the run's closed set (or a decided constructor
  registration) to state `#released` / `#moved` at the linked path, and the
  caller to link a tracked object there under no arm; the run's set mixes
  candidates that state nothing at those paths (kept), whose join already
  refuses `member-path-undecided`. A few already-`Unknown` librdkafka rows
  may change content (a new refusal leaf); any status move would be `Safe`
  or `Unsafe` to `Unknown`, never towards `Safe`, and would stop the card
  for the user (step 3) before the semantic commit.
* Controls: the rows naming `rd_kafka_op_destroy` and `rd_kafka_poll_cb`
  keep their statuses.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); none is predicted on any of the
three. The riskiest move would be a librdkafka op-construction row
(`rd_kafka_admin_request_op_new`-like: an op built with one decided
callback, a member linked, then destroyed on an error path) going `Safe` to
`Unknown`.
