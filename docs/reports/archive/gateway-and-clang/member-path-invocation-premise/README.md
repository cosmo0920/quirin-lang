# MEMBER-PATH-INVOCATION-PREMISE: `gone` only under an established invocation

* **Base:** main `98c51878f`. **Card:** the acceptance matrix's follow-up
  registered by CALLBACK-TABLE-MEMBER-PATH-STATES. USER DECISION
  2026-11-26: "launch the MEMBER-PATH-INVOCATION-PREMISE card".
* **Commits:** census and probe `3485bdb5b` (3 of 4 probe tests fail at
  base), [PREDICTION](PREDICTION.md) `a6dcccea4`, the semantic commit
  `a8a93c0f0` (tree `fbe41c8b7`, the measured tree), then these records.

## 1. Finding at base

Rule 30's table (C04.19.6, user decision 2026-10-15) composed a
candidate's `#released` or `#moved` state at a linked path as `gone` under
every arm join, `some` and `none` included. Every invocation clause calls
its callback zero or more times, so under `some` or `none` that reading
rests on a candidate running; with no call the object stays live while the
caller reads it released (a missed leak; a later free reads as a double
release). The probe target `crates/qcore-c-gateway/tests/member_path_invocation_premise.rs`
shows it at base ([`evidence/probe-base.log`](evidence/probe-base.log),
run by [`scripts/probe-base.sh`](scripts/probe-base.sh): exit 101, 1 passed
and 3 failed): through invokers with an early return, a call under a
condition, a loop that may run zero times, `?:`, `&&`, a `goto` past the
call, a `switch` case without it, a call handing another object, or no body
at all, the synchronous, member and table forms all read the payload taken
and the row `Safe`.

## 2. Census

[`scripts/census.py`](scripts/census.py), output
[`evidence/census.txt`](evidence/census.txt), reads every shipped `.qv`
and the gateway fixture module `member_paths.qv`.

| Invoking declaration | Form | Composed position | Candidates stating `#released` / `#moved` there | C body calls a candidate on every path? |
|---|---|---|---|---|
| `rd_kafka_op_destroy` (librdkafka `client.qv`) | member (`rd_kafka_op_s.rko_op_cb`, released `rko`, with arms) | 2 (`rko`) | `rd_kafka_admin_worker`, `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` (hand-off candidates), `rd_kafka_admin_fanout_worker`, `rd_kafka_share_commit_sync_fanout_op` (plain); also `{V, #kept}` sets on `rd_kafka_share_commit_async_fanout_op`, `rd_kafka_share_fetch_fanout_op` | **No**: `rdkafka_op.c:349`, `if ((rko->rko_type & RD_KAFKA_OP_CB) && rko->rko_op_cb)` |
| `rd_kafka_poll_cb` (librdkafka `client.qv`) | member (`rd_kafka_op_s.rko_op_cb`, owned `rko`) | 2 (`rko`) | the same | **No**: `rdkafka.c:5822` returns `PASS` first; the call is inside `rd_kafka_op_call` (`:6030`, `:6044`, `:6083`), reached only from some `switch` cases |
| `c_op_destroy` (fixture `member_paths.qv`) | member (`op.op_cb`, with arms) | 2 | `c_cb_releases`, `c_cb_moves`, `c_cb_other` (`aux`), `c_cb_deep` | fixture, no shipped body |

Every other shipped invocation clause handing a caller parameter (44 in
all: the librdkafka `rk_conf` member invocations, `rd_kafka_q_serve*`'s
forwarded `cb_type` / `opaque`, the cmetrics / ctraces unpackers and
tables, chunkio / cmetrics logging, `qsort`/`bsearch`, onigmo, OpenSSL PEM)
hands an object no member-path declaration binds: the stating declarations
bind `rd_kafka_op_s` at their third formal, and the 96 Fluent Bit
`teardown_destroy.qv` states belong to direct destroyers (a direct call
runs its body once; not an invocation form). No shipped declaration's body
establishes the invocation, so a stated premise on one would be false, and
none was added.

## 3. Change

**New module `crates/qcore-c-gateway/src/member_path_invocation.rs`:**
`Form` (`Formal`, `Member`, `Table`), `Site` (form, arity, composed
position, handed formal), `established` / `body_establishes`,
`unestablished`, `INVOCATION = "member-path-invocation-unestablished"`.
`body_establishes` lowers the invoking `FunctionDecl`'s body to rule 30's
`member_path_reading::Graph` and runs a forward must-call data flow (`and`
as the meet). It requires, before every `return` and the end of the body,
an unconditionally evaluated call of the formal (`F(…)`, `(*F)(…)`), of
`P->PATH(…)` or of a table entry (`T[i].f(…)`, `T->f(…)`). That call must
have the clause's arity and hand the composed formal at the composed
position, and neither formal may be rebound. Not established: each state
`V` reads `{V, #kept}`. Where that changes a decided reading, the call
refuses with the new diagnostic and the object is undecided.

**Hook points (small):**

* `legacy_member_path_route.rs`: the `MemberPathOperands::invocation` field,
  `operands`' `form` argument, and in `apply` the weakened join with its
  refusal, decided once per call.
* `legacy_callback_route.rs`: `Form` passed at the `Invoked`, `Table` and
  `MemberInvoked` branches of `callback_operands`, plus `None` at rule 50's
  candidate composition. Also `member_form`, `MemberUse::form` set at the
  deferred member use, and the `Site` built in `apply`'s constructed-route
  loop.
* `legacy_constructed_route.rs`: the `MemberUse::form` field.
* `member_path_reading.rs`: `Graph`, `Node` and `Graph::of` are now
  `pub(crate)`.
* `lib.rs`: the module.

**Rules:** [rule 30](../../../../agents/rules/30.md) amended (text and summary;
the one authority). In [rule 13v](../../../../agents/rules/13v.md), the "named,
not proved" sentence now points to rule 30's amendment. C04.19.6's table
row, a new paragraph "The invocation must be established", the table-dispatch
paragraph, and provenance were all updated. No new rule id was taken.
`docs/agents/gateway.md`, `docs/agents/README.md` and `proof/QCore.lean`
were regenerated.

**Pins:** `tests/member_path_invocation_premise.rs` (4 tests): every-path
invokers in all three forms are unchanged (`Safe`, and a second free is never
`Safe`); invokers that may call no candidate refuse with the new diagnostic
and read `Unknown` in all three forms; another object or a rebound formal
establishes nothing; an arm releasing the path on every arm needs no
invocation. Three older fixtures whose bodiless invokers rested on the
premise (`member_path_release.rs`, `callback_member_released_effect_order.rs`,
`callback_table_member_path_states.rs`) now define those invokers with
bodies that call on every path (`INVOKER_BODIES`), so their pinned readings
stand. The LLP64 scan over the changed tests exits 0.

## 4. Lean

`QCore.MemberPathInvocationPremise` with `proof/AuditMemberPathInvocationPremise.lean`:
15 audited theorems, axioms `propext` / `Quot.sound` or none, no `sorry`,
no project axiom (`private/logs/lean-1.log`: build 0, audit 0).

* `reading`, `weaken`, and `an_established_reading_is_the_stated_one`.
* `an_unestablished_gone_needs_every_arm`.
* `an_unestablished_gone_leaves_exactly_once`, which holds for every sequence
  of invocations, the empty one included.
* `an_unestablished_stays_never_leaves`.
* `a_taking_candidate_without_an_arm_refuses_unestablished`.
* `the_weakening_only_refuses_more`.
* The body model `Stmt` / `Exec` / `must` / `established`, with `exec_mono`,
  `must_sound` and `an_established_body_calls_on_every_exit`. The vectors
  `may_not_call_vectors_hold` and `each_shape_has_an_execution_with_no_call`.
* `the_premise_is_discharged`: under an established body, a `gone` reading
  over the calls an execution made leaves exactly once.

`CallbackTableMemberPathStates.the_gone_reading_assumes_an_invocation` is
kept, and its docstring now points to the discharge. `check_lean_audits.py`
passes 531 audits (`private/logs/lean-audits-1.log`).

## 5. Measurement against the prediction

The opt-in run was `quirin-audit current --monkey-linked` over Fluent Bit
046accf6 (`private/fb-046accf6`, read-only), driven by
[`scripts/run.sh`](scripts/run.sh) and `opt-in-current.sh`, which are the
previous card's scripts with this worktree's paths. Each side was a
`git archive` tree with its own release binary, run in a
`gate_slot.py audit` slot. The two runs overlapped in time. Outputs are in
`private/ctmp/run-{base,tip}` and are not committed.

| Run | Tree | Binary sha256 | Exit (build / checkers / run) | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---|
| base (12:16-13:25Z) | `98c51878f` | `9f31e827...24bafdab` | 0 / 0 / 0 | 1205 / 2 / 3589 / 8 |
| tip (12:37-13:43Z) | tree `fbe41c8b7` (= `a8a93c0f0`) | `f34f49f3...ef900dee58` | 0 / 0 / 0 | 1205 / 2 / 3589 / 8 |

The totals are equal ([base](evidence/totals-base.json),
[tip](evidence/totals-tip.json)), and so are the indirect-access figures.
[`moved-rows-base-tip.jsonl`](evidence/moved-rows-base-tip.jsonl) is
`{"moves": {}}`, and [`changed-rows-base-tip.txt`](evidence/changed-rows-base-tip.txt)
is empty. No tip row carries `member-path-invocation-unestablished`.
**Controls:** the 264 rows naming `rd_kafka_op_destroy` or `rd_kafka_poll_cb`
are 43 `Safe` and 221 `Unknown` on both sides.

**Against the prediction:** 0 status moves, 0 content changes, and equal
totals. The prediction allowed a few content changes, and none occurred. No
landed result that rests on the 2026-10-15 reading moved, so step 3 did not
stop the card.

**Pinned routes:** not run. This host has no `build/cycle19` inputs, so
`quirin-audit measure --declared` cannot run here, and nothing was
fabricated. The `measure-prediction` block (none on corpus, system and
slice) is for the integrator.

## 6. Gate, conflicts and follow-ups

See §7 for the gate record.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_member_path_route.rs`: the struct field,
  the `operands` argument and the `apply` block.
* `crates/qcore-c-gateway/src/legacy_callback_route.rs`: four `operands`
  calls, `member_form`, `MemberUse::form`, and the `Site` in `apply`'s
  constructed loop.
* `legacy_constructed_route.rs` (one field) and `member_path_reading.rs`
  (three visibilities).
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.6) and rules `30.md` / `13v.md`.
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`: rerun their generators to resolve.

**Follow-ups:** this card retires MEMBER-PATH-INVOCATION-PREMISE and
registers none. The body check does not follow a helper call such as
`rd_kafka_op_call`, which keeps it fail-closed. A stated premise
vocabulary was not needed, because no shipped body could make the
statement true.

## 7. Gate

**Record:** [`gate/gate-1.json`](gate/gate-1.json). **Command:** `sh
private/toolchain/env.sh python3 scripts/gate_scope.py --base 98c51878f
--out private/gate` (Clang 21, Lean 4.35); log `private/logs/gate-1.log`.
The gated tree is `74708f332`; this section and its record are docs only.
**Exit 0, 8 steps passed.** This was a scoped run with Lean `all` and no
provider gate, since `providers/` is unchanged.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy -D warnings` | 0, 0, 0 | `third_party/libevent` checked out |
| Rust suite (scoped) | 0 | 722 units; 10344 passed, 0 failed, 24 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 531 audits |
| Python suite | 0 | 807 tests |

No base failures were seen. **LLP64 fixture scan** over the four changed
test paths: exit 0 (0 counted).
