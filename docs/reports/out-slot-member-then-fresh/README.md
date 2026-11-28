# OUT-SLOT-MEMBER-THEN-FRESH: option (a), the op's member custody

Card OUT-SLOT-MEMBER-THEN-FRESH. USER DECISION 2026-11-28: option **(a)**,
declare the op's member custody: the member list is moved out of the op (and
nulled) before `rd_kafka_op_destroy`, so the out slot receives either the moved
member or the fresh list. Base `287dce8ed` (main, r86). Probe pins
`6aa742484`, [PREDICTION.md](PREDICTION.md) `90d1d41b4`. No rule id, no Lean
module, no contract change.

**Outcome: a stop with options.** The custody claim is true in both bodies
(section 2), but the contract vocabulary cannot state it and route G gives the
moved member no identity (section 3). Even a stated custody would move neither
row, because the early exits leave the slot unwritten under a status that
does not partition it (section 4). The card lands four pins of exactly what
is missing and the measurement. No row moved (section 5).

## 1. Probes at base

**Current route.** Opt-in `current --monkey-linked` over Fluent Bit
`046accf6` at base (section 5). Both rows are `Unknown` with the leaves
OUT-SLOT-CONDITIONAL-DELIVERY recorded:

| row | leaves |
|---|---|
| `rdkafka_subscription.c::rd_kafka_assignment` | 3 indirect accesses; `owned result of resource API rd_kafka_topic_partition_list_new stored through *partitions without an out-parameter contract` (rule 74l); unresolved `partitions`, `rkcg`, `rko` ×2 |
| `rdkafka_subscription.c::rd_kafka_subscription` | the same with `*topics`/`topics` |

The rows' only in-tree caller, `rd_kafka_share_subscription`, forwards its
own slot. No Fluent Bit plugin or core source calls either function, and the
two calls in `examples/transactions.c` are not in the compile database.

**Fixtures** ([evidence/probe-fixtures-base.txt](evidence/probe-fixtures-base.txt),
over the synthetic `c_*` op of `op_constructors.qv`. `c_op_new(1)` decides
the arm that frees `payload`):

| body | base |
|---|---|
| the shape: early `return -1`, `*out = rko->payload; rko->payload = 0; c_op_destroy(rko);`, fresh `c_item_new(0)` when null and `!err` | `Unknown`: rule 74l's slot refusal (plus the op's open `op_cb` closure and an indirect access); caller `call to uncontracted function` |
| `p = rko->payload; rko->payload = 0; c_op_destroy(rko); c_free(p);` | `Unknown`, `UnknownAlias p`: the moved member has no identity |
| the same without `c_free(p)` | `Safe`: nothing was owed |
| the same without the null store (a real double free in C) | `Unknown`, `UnknownAlias p`, never a reported double release |
| the frame links `it` at `payload`, reads it back, nulls, destroys, `c_free(p)` | `Unknown`, `UnknownAlias p` |
| the same without `c_free(p)` | `Unsafe` `Leak` (true: `it` was unlinked and dropped) |

## 2. The custody claim is true

* `rd_kafka_assignment` (`rdkafka_subscription.c:316-340`): `*partitions =
  rko->rko_u.assign.partitions; rko->rko_u.assign.partitions = NULL;
  rd_kafka_op_destroy(rko);`. `rd_kafka_subscription` (`:344-368`) does the
  same with `rko_u.subscribe.topics`.
* The destroyer frees the member: `case RD_KAFKA_OP_ASSIGN: case
  RD_KAFKA_OP_GET_ASSIGNMENT: RD_IF_FREE(rko->rko_u.assign.partitions,
  rd_kafka_topic_partition_list_destroy)` (`rdkafka_op.c:386-390`; `:380-384`
  for `subscribe.topics`). The provider's `rd_kafka_op_destroy` arms state it
  (`resource_arm_released(rko, rko_u.assign.partitions) when rko_type == 24`,
  `… subscribe.topics … == 23`).
* The op owns what it holds there. The cgrp handler stores a copy
  (`rd_kafka_topic_partition_list_copy`, `rdkafka_cgrp.c:7519-7541`) into the
  request op itself and replies with it (`rd_kafka_op_reply` re-enqueues the
  same `rko`, OR-ing `RD_KAFKA_OP_REPLY`/`_CB`, which the destroy's mask
  clears: `rdkafka_op.c:804-816`).

So the member is moved out and nulled before the destroy whose arm would free
it. The claim holds and the brief's stop condition (member not nulled) does
not apply.

## 3. What the vocabulary cannot state (the stop)

Option (a) needs the reply op's member to carry custody and the read-and-null
to hand it to the reader. Every piece is refused today, and each refusal is
pinned (`crates/qcore-c-gateway/tests/op_constructor_declarations.rs`,
`the_vocabulary_cannot_state_the_members_custody`,
`an_ops_member_read_carries_no_custody`,
`a_moved_out_linked_member_keeps_no_identity`,
`the_member_then_fresh_shape_stays_refused`):

1. **No result state on a nullable result.** `rd_kafka_op_req2` is
   `resource_return_owned(nullable)` (it returns NULL on timeout). C04.19.4
   admits a result state, a decided discriminant included, only beside
   `resource_return_owned(nonnull)`: `quant-live-unbacked`.
2. **No member of a result holding an object.** C04.19.4's member states map
   a member to `#absent` or a function designator only. `#moved`, `#kept` and
   `#released` on `#result` are `quant-live-value`. C04.19.6's `#moved` is a
   state of a *formal*, and it hands the object to a holder "never the
   caller".
3. **No identity for a member read.** Route G reads `p = rko->M` as a
   borrowed alias with no identity (rule 10t), even when the frame itself
   linked `M`. A release of `p` is `UnknownAlias`. The frame-linked case is
   the gap CARRIED-CHILD-TEARDOWN-AFTER-UNLINK names (O04-READ-BOUND: a bound
   read of a member whose link holds, admitted at a release position).
4. **No disposition for the function.** Return 0 does partition the slot:
   both early codes (`__UNKNOWN_GROUP` -179, `__TIMED_OUT` -185) are non-zero,
   so a 0 return means `err == 0` past the op, where the slot holds the moved
   list or the fresh one (`rd_calloc` aborts rather than return NULL). A
   non-zero return leaves the slot either unwritten (early exits) or holding
   the moved member, possibly null (`err != 0`). C04.13 needs a mode in every
   class, and none says "unwritten or an owned, possibly null object". A
   declaration with only the 0 class is refused ("the return-code classes do
   not cover every returned value").

Stating (a) is therefore a new piece of contract vocabulary and a new route G
reading, which is a semantic decision for the user (section 6). Under the
brief's step 3 this is a stop with options, not a workaround.

## 4. The early exits and the callers

The early returns leave the slot as the caller held it. Even with (a) stated,
the non-zero codes do not partition delivery, so no summary may claim one
there and the caller-side reading stays refused. The callee rows also stay
`Unknown`. Their fresh store and their moved store go through a slot that
nothing sanctions (rule 74l), and their other leaves remain (indirect
accesses, unresolved `rko`/`rkcg`). **No caller becomes `Safe`**:
`rd_kafka_share_subscription` forwards its slot and inherits the same
unpartitioned status, and nothing else in the compiled tree calls either
function.

## 5. Measurement

[evidence/runs.md](evidence/runs.md): opt-in `current --monkey-linked` over
Fluent Bit `046accf6` (`private/fb-046accf6`, read-only), base and tip each
from its own `git archive` tree and its own release binary, with
`docs/reports/fb-retry-or6-element-identity/scripts/` (paths changed).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `287dce8ed` | 1,209 / 2 / 3,586 / 7 | 2,091 / 201 / 8,909 |
| tip `6aa742484` | **1,209 / 2 / 3,586 / 7** | 2,091 / 201 / 8,909 |

**No row moved and no leaf changed**
([moved-rows-base-tip.jsonl](evidence/moved-rows-base-tip.jsonl) is
`{"moves": {}}`, [changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)
is empty), as predicted. There is no `Unknown -> Safe` move to justify. Both
named rows and `rd_kafka_share_subscription` keep their base leaves
([evidence/named-rows.txt](evidence/named-rows.txt)). The later commits change
only this report.

**System-slice route: not run.** Neither row is in
`contract/profiles/fluent-bit/system-selection.json`, and the tip changes no
analysed source. **Pinned routes** (`quirin-audit measure --declared`) cannot
run on this host (no `build/cycle19` inputs or baselines). Not run, and no
result is claimed.

## 6. Options for the user

* **(a1) New vocabulary for a received object's member custody, plus the
  move-out reading.** In the contract specification (C04.19.4): a result state
  admitted on a nullable result, holding on the non-null branch; a member
  value meaning "holds an object this result owns, possibly null" under the
  decided discriminant (for `rd_kafka_op_req2`, the reply is the request op,
  type `type | REPLY` masked to `type`, a declared assumption about the
  replier). In route G (a rule, 96): a read `X = P->M` of such a member, or of
  a member the frame linked, followed by `P->M = NULL` before any release of
  `P`, binds `X` to that object (one live identity, the member unlinked). Lean
  parity is needed for both. This is the stated form of (a). Alone it moves
  neither row (section 4).
* **(a1)+(b)** The above plus OUT-SLOT-CONDITIONAL-DELIVERY's option (b): a
  "delivers or leaves unwritten" mode read only by a caller whose slot is
  known null, which amends rule 74l. Only this combination gives a caller a
  reading. It would still move no measured row, because no compiled caller
  passes a known-null `&NAME`.
* **(a2) Only the route G move-out of a frame-linked member** (gap 3), with no
  new vocabulary. It is sound alone and makes
  `a_moved_out_linked_member_keeps_no_identity`'s release `Safe`. It touches
  rule 10t/O04 territory shared with CARRIED-CHILD-TEARDOWN-AFTER-UNLINK, and
  moves neither row.
* **(c) Keep the refusal** with (a) recorded as true but unstatable (this
  card's state).

## 7. Changes

Tests only: four pins in `crates/qcore-c-gateway/tests/op_constructor_declarations.rs`
(a new section after the last test; no root test binary added). Records: this
report, [PREDICTION.md](PREDICTION.md), [evidence/](evidence/),
[integration-amendments.json](integration-amendments.json). No code, provider,
contract, rule or Lean change, so the provider gate is not selected. Rule
85(4) and 74l stand as written. `gen_rule_docs.py` and `gen_lean_roots.py`
write nothing.

## 8. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base 287dce8ed --out
private/gate` through `private/toolchain/env.sh` (Clang 21, Lean 4.35), head
`c7ece88bf` (tests as at `6aa742484`). It ran in scoped mode: no affected
crate and 12 test targets reached through the changed test file. Lean was not
selected and the provider gate was not selected. The record is
[gate/gate-1.json](gate/gate-1.json), with logs in the worktree's
`private/gate/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy` (5 packages, the selected targets) `-D warnings` | 0 | |
| `run_test_binaries.py --targets …` (12) | 0 | 74 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | **1** | 814 tests, 1 failure: `test_report_hygiene…test_real_repository_passes_hygiene_and_index_checks`, `docs/reports/INDEX.md` stale after this report was added |

The one failure was this card's own: `python3 scripts/build_docs_index.py`
had not been rerun. It was fixed in the next commit (INDEX.md and README.md
regenerated). The Python suite was then rerun standalone: exit 0, 814 tests
(`private/m/python-rerun.log`). `python3 scripts/check_lean_audits.py` was run
standalone: exit 0, 536 audits passed (`private/m/lean-audits.log`). The gate was not rerun after the fix, which touched
only generated report indexes and this report. No base failure was seen.

LLP64 fixture scan over the changed test paths
(`crates/qcore-c-gateway/tests/op_constructor_declarations.rs`): exit 0.
`check_report_hygiene.py`: exit 0. `apply_amendments.py … --check
--profiles-base 287dce8ed`: exit 0.

## 9. Follow-ups

* **OUT-SLOT-MEMBER-THEN-FRESH** amended (not retired). Option (a) is
  verified true but not statable. The four gaps of section 3 and the options
  of section 6 await the user.
